use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use allocation_counter::measure;
use sha2::{Digest, Sha256};

use crate::write_json;

use super::{
    environment,
    model::{
        AggregateCell, AggregateReport, BenchmarkCell, BenchmarkManifest, CampaignProfile,
        EnvironmentClassification, EnvironmentReport, ExecutionOrder, ExecutionTask,
        GraphExactOutcome, PairedComparison, RawObservation, ScalingCurve, WorkerReport,
    },
    stats::{
        bootstrap_quantile_ci, bootstrap_statistic_ci, derive_seed, log_log_slope, mad, median,
        quantile, relative_half_width, SplitMix64,
    },
    workloads::{self, PreparedOperation},
};

pub fn load_manifest(path: &Path) -> Result<BenchmarkManifest, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut manifest: BenchmarkManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    if manifest.cells.is_empty() {
        let reference = manifest
            .cells_from
            .as_deref()
            .ok_or("benchmark manifest has neither cells nor cells_from")?;
        if !valid_manifest_reference(reference) {
            return Err("cells_from must be a sibling JSON filename".into());
        }
        let referenced_path = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(reference);
        let referenced_bytes = fs::read(&referenced_path)
            .map_err(|error| format!("read {}: {error}", referenced_path.display()))?;
        let referenced: BenchmarkManifest = serde_json::from_slice(&referenced_bytes)
            .map_err(|error| format!("parse {}: {error}", referenced_path.display()))?;
        if referenced.cells.is_empty() {
            return Err("cells_from may not reference another empty manifest".into());
        }
        manifest.cells = referenced.cells;
    }
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn run_worker(
    manifest_path: &Path,
    cell_id: &str,
    process_index: usize,
    output: &Path,
) -> Result<WorkerReport, String> {
    let manifest = load_manifest(manifest_path)?;
    let cell = manifest
        .cells
        .iter()
        .find(|cell| cell.id == cell_id)
        .ok_or_else(|| format!("unknown benchmark cell {cell_id:?}"))?
        .clone();
    let seed = worker_seed(&manifest, &cell, process_index);
    let setup_start = Instant::now();
    let mut prepared = workloads::prepare(&cell, seed)?;
    let graph_exact = prepared.graph_exact.clone();
    let setup_ns = duration_ns(setup_start.elapsed().as_nanos());
    let batch_iterations = calibrate(&manifest, &mut prepared);
    for _ in 0..manifest.warmup_observations {
        black_box(run_batch(&mut prepared, batch_iterations));
    }
    let allocations = measure(|| {
        black_box(run_batch(&mut prepared, batch_iterations));
    });
    let mut observations = Vec::with_capacity(manifest.measured_observations);
    for observation_index in 0..manifest.measured_observations {
        let start = Instant::now();
        let checksum = black_box(run_batch(&mut prepared, batch_iterations));
        let elapsed_ns = duration_ns(start.elapsed().as_nanos());
        let logical_units = prepared
            .logical_units_per_action
            .checked_mul(batch_iterations)
            .ok_or("logical unit overflow")?;
        observations.push(RawObservation {
            observation_index,
            elapsed_ns,
            batch_iterations,
            logical_units,
            ns_per_logical_unit: elapsed_ns as f64 / logical_units.max(1) as f64,
            checksum: format!("{checksum:016x}"),
        });
    }
    let report = WorkerReport {
        schema: "microfield-publication-worker-v2".into(),
        campaign_id: manifest.campaign_id,
        cell,
        process_index,
        seed,
        pid: std::process::id(),
        setup_ns,
        batch_iterations,
        allocation_count: allocations.count_total,
        allocated_bytes: allocations.bytes_total,
        peak_allocated_bytes: allocations.bytes_max,
        graph_exact,
        observations,
    };
    write_json(output, &report)?;
    Ok(report)
}

pub fn run_campaign(manifest_path: &Path, run_directory: &Path) -> Result<AggregateReport, String> {
    let manifest = load_manifest(manifest_path)?;
    ensure_new_run_directory(run_directory)?;
    let captured_environment = environment::capture(&manifest);
    fs::create_dir_all(run_directory.join("raw/workers"))
        .map_err(|error| format!("create {}: {error}", run_directory.display()))?;
    write_json(&run_directory.join("manifest.json"), &manifest)?;
    write_json(
        &run_directory.join("environment.json"),
        &captured_environment,
    )?;

    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut reports = Vec::new();
    let mut tasks = Vec::new();
    let mut active = manifest
        .cells
        .iter()
        .map(|cell| cell.id.clone())
        .collect::<BTreeSet<_>>();
    let mut sequence = 0_usize;

    for process_index in 0..manifest.maximum_processes {
        if active.is_empty() {
            break;
        }
        let mut round = active.iter().cloned().collect::<Vec<_>>();
        SplitMix64::new(derive_seed(manifest.seed, "execution-order", process_index))
            .shuffle(&mut round);
        for cell_id in round {
            let cell = manifest
                .cells
                .iter()
                .find(|cell| cell.id == cell_id)
                .expect("active cell came from the validated manifest");
            let seed = worker_seed(&manifest, cell, process_index);
            let output = worker_path(run_directory, &cell_id, process_index);
            let status = Command::new(&executable)
                .arg("publication-worker")
                .arg("--manifest")
                .arg(manifest_path)
                .arg("--cell")
                .arg(&cell_id)
                .arg("--process-index")
                .arg(process_index.to_string())
                .arg("--out")
                .arg(&output)
                .status()
                .map_err(|error| format!("spawn worker {cell_id}: {error}"))?;
            tasks.push(ExecutionTask {
                sequence,
                round: process_index,
                cell: cell_id.clone(),
                process_index,
                seed,
                status: if status.success() {
                    "success"
                } else {
                    "failed"
                }
                .into(),
            });
            sequence += 1;
            if !status.success() {
                write_json(
                    &run_directory.join("execution-order.json"),
                    &ExecutionOrder {
                        schema: "microfield-publication-execution-order-v1".into(),
                        campaign_id: manifest.campaign_id.clone(),
                        tasks,
                    },
                )?;
                return Err(format!("publication worker {cell_id} failed with {status}"));
            }
            reports.push(read_worker(&output)?);
        }

        let process_count = process_index + 1;
        if process_count >= manifest.minimum_processes
            && (process_count == manifest.maximum_processes
                || (process_count - manifest.minimum_processes) % manifest.precision_check_every
                    == 0)
        {
            let aggregate = aggregate(&manifest, &captured_environment, &reports)?;
            active.retain(|cell| {
                !aggregate
                    .cells
                    .iter()
                    .any(|summary| summary.id == *cell && summary.precision_met)
            });
        }
    }

    let execution_order = ExecutionOrder {
        schema: "microfield-publication-execution-order-v1".into(),
        campaign_id: manifest.campaign_id.clone(),
        tasks,
    };
    write_json(
        &run_directory.join("execution-order.json"),
        &execution_order,
    )?;
    write_raw_jsonl(&run_directory.join("raw/workers.jsonl"), &reports)?;
    fs::remove_dir_all(run_directory.join("raw/workers"))
        .map_err(|error| format!("remove consolidated worker files: {error}"))?;
    write_outputs(&manifest, &captured_environment, &reports, run_directory)
}

pub fn analyse_run(manifest_path: &Path, run_directory: &Path) -> Result<AggregateReport, String> {
    let manifest = load_manifest(manifest_path)?;
    let environment: EnvironmentReport = read_json(&run_directory.join("environment.json"))?;
    let reports = read_raw_jsonl(&run_directory.join("raw/workers.jsonl"))?;
    write_outputs(&manifest, &environment, &reports, run_directory)
}

fn write_outputs(
    manifest: &BenchmarkManifest,
    environment: &EnvironmentReport,
    reports: &[WorkerReport],
    run_directory: &Path,
) -> Result<AggregateReport, String> {
    let report = aggregate(manifest, environment, reports)?;
    write_json(&run_directory.join("aggregate.json"), &report)?;
    write_aggregate_csv(&run_directory.join("aggregate.csv"), &report)?;
    write_comparison_csv(&run_directory.join("comparisons.csv"), &report)?;
    write_markdown_report(&run_directory.join("report.md"), environment, &report)?;
    write_checksums(run_directory)?;
    Ok(report)
}

fn validate_manifest(manifest: &BenchmarkManifest) -> Result<(), String> {
    if manifest.schema != "microfield-publication-benchmark-manifest-v1"
        || manifest.campaign_id.is_empty()
        || manifest.minimum_processes < 2
        || manifest.maximum_processes < manifest.minimum_processes
        || manifest.maximum_processes > 100
        || manifest.precision_check_every == 0
        || manifest.warmup_observations == 0
        || manifest.measured_observations < 3
        || manifest.target_observation_ns == 0
        || manifest.maximum_batch_iterations == 0
        || manifest.bootstrap_resamples < 100
        || !manifest.maximum_relative_ci_half_width.is_finite()
        || !(0.0..=0.5).contains(&manifest.maximum_relative_ci_half_width)
        || manifest.cells.is_empty()
    {
        return Err("invalid publication benchmark manifest".into());
    }
    match manifest.profile {
        CampaignProfile::Smoke if manifest.maximum_processes > 3 => {
            return Err("smoke profile may not exceed three processes".into());
        }
        CampaignProfile::Pilot if manifest.minimum_processes < 5 => {
            return Err("pilot profile requires at least five processes".into());
        }
        CampaignProfile::Publication if manifest.minimum_processes < 30 => {
            return Err("publication profile requires at least 30 processes".into());
        }
        _ => {}
    }
    let mut ids = BTreeSet::new();
    for cell in &manifest.cells {
        if !ids.insert(cell.id.as_str())
            || !valid_id(&cell.id)
            || cell.family.is_empty()
            || !workloads::SUPPORTED_OPERATIONS.contains(&cell.operation.as_str())
            || cell.scale == 0
            || cell.scale_unit.is_empty()
            || cell.payload_bytes == 0
            || cell.payload_bytes > 1_048_576
            || cell.dataset_size.is_some_and(|size| size < cell.scale)
            || cell.curve.is_some() != cell.strategy.is_some()
        {
            return Err(format!("invalid publication benchmark cell {}", cell.id));
        }
    }
    for cell in &manifest.cells {
        if let Some(baseline) = &cell.baseline_cell {
            if baseline == &cell.id || !ids.contains(baseline.as_str()) {
                return Err(format!("invalid baseline for benchmark cell {}", cell.id));
            }
        }
    }
    Ok(())
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 160
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn valid_manifest_reference(reference: &str) -> bool {
    reference.ends_with(".json")
        && !reference.is_empty()
        && reference
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn worker_seed(manifest: &BenchmarkManifest, cell: &BenchmarkCell, process_index: usize) -> u64 {
    let input_identity = cell.baseline_cell.as_deref().unwrap_or(&cell.id);
    derive_seed(manifest.seed, input_identity, process_index)
}

fn calibrate(manifest: &BenchmarkManifest, prepared: &mut PreparedOperation) -> u64 {
    let maximum = prepared
        .maximum_batch_iterations
        .unwrap_or(manifest.maximum_batch_iterations)
        .min(manifest.maximum_batch_iterations);
    let mut batch = 1_u64;
    loop {
        let start = Instant::now();
        black_box(run_batch(prepared, batch));
        if duration_ns(start.elapsed().as_nanos()) >= manifest.target_observation_ns
            || batch >= maximum
        {
            return batch;
        }
        batch = batch.saturating_mul(2).min(maximum);
    }
}

fn run_batch(prepared: &mut PreparedOperation, iterations: u64) -> u64 {
    let mut checksum = 0xcbf2_9ce4_8422_2325_u64;
    for index in 0..iterations {
        let value = black_box((prepared.run)());
        checksum = (checksum ^ value ^ index).wrapping_mul(0x0000_0100_0000_01b3);
    }
    checksum
}

fn aggregate(
    manifest: &BenchmarkManifest,
    environment: &EnvironmentReport,
    reports: &[WorkerReport],
) -> Result<AggregateReport, String> {
    validate_worker_reports(manifest, reports)?;
    let mut by_cell = BTreeMap::<&str, Vec<&WorkerReport>>::new();
    for report in reports {
        by_cell.entry(&report.cell.id).or_default().push(report);
    }
    let mut cells = Vec::with_capacity(manifest.cells.len());
    for cell in &manifest.cells {
        let workers = by_cell.get(cell.id.as_str()).cloned().unwrap_or_default();
        if workers.is_empty() {
            return Err(format!("no worker reports for {}", cell.id));
        }
        cells.push(aggregate_cell(manifest, cell, &workers));
    }
    let comparisons = paired_comparisons(manifest, &by_cell);
    let curves = scaling_curves(manifest, &cells);
    let all_cells_precise = cells.iter().all(|cell| cell.precision_met);
    let claims_allowed = manifest.profile == CampaignProfile::Publication
        && environment.classification == EnvironmentClassification::Controlled
        && all_cells_precise;
    let schema = if reports
        .iter()
        .all(|report| report.schema == "microfield-publication-worker-v2")
    {
        "microfield-publication-aggregate-v2"
    } else {
        "microfield-publication-aggregate-v1"
    };
    Ok(AggregateReport {
        schema: schema.into(),
        campaign_id: manifest.campaign_id.clone(),
        profile: manifest.profile,
        environment_classification: environment.classification,
        cells,
        comparisons,
        curves,
        all_cells_precise,
        claims_allowed,
    })
}

fn aggregate_cell(
    manifest: &BenchmarkManifest,
    cell: &BenchmarkCell,
    workers: &[&WorkerReport],
) -> AggregateCell {
    let clusters = workers
        .iter()
        .map(|worker| {
            worker
                .observations
                .iter()
                .map(|sample| sample.ns_per_logical_unit)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let process_medians = clusters
        .iter()
        .map(|cluster| median(cluster))
        .collect::<Vec<_>>();
    let pooled = clusters.iter().flatten().copied().collect::<Vec<_>>();
    let center = median(&process_medians);
    let median_ci95 = bootstrap_statistic_ci(
        &process_medians,
        manifest.bootstrap_resamples,
        derive_seed(manifest.seed, &format!("{}-median", cell.id), workers.len()),
        median,
    );
    let p95_ci95 = bootstrap_quantile_ci(
        &clusters,
        0.95,
        manifest.bootstrap_resamples,
        derive_seed(manifest.seed, &format!("{}-p95", cell.id), workers.len()),
    );
    let p99_ci95 = bootstrap_quantile_ci(
        &clusters,
        0.99,
        manifest.bootstrap_resamples,
        derive_seed(manifest.seed, &format!("{}-p99", cell.id), workers.len()),
    );
    let relative = relative_half_width(center, median_ci95);
    let precision_met = workers.len() >= manifest.minimum_processes
        && relative <= manifest.maximum_relative_ci_half_width;
    let allocations = workers
        .iter()
        .map(|worker| worker.allocation_count as f64)
        .collect::<Vec<_>>();
    let allocated_bytes = workers
        .iter()
        .map(|worker| worker.allocated_bytes as f64)
        .collect::<Vec<_>>();
    let peak_bytes = workers
        .iter()
        .map(|worker| worker.peak_allocated_bytes as f64)
        .collect::<Vec<_>>();
    let graph_exact = workers[0].graph_exact.clone();
    AggregateCell {
        id: cell.id.clone(),
        family: cell.family.clone(),
        operation: cell.operation.clone(),
        scale: cell.scale,
        scale_unit: cell.scale_unit.clone(),
        process_count: workers.len(),
        observation_count: pooled.len(),
        median_ns_per_unit: center,
        p95_ns_per_unit: quantile(&pooled, 0.95),
        p99_ns_per_unit: quantile(&pooled, 0.99),
        mad_ns_per_unit: mad(&pooled),
        median_ci95,
        p95_ci95,
        p99_ci95,
        relative_median_ci_half_width: relative,
        precision_met,
        allocation_count_median: median(&allocations),
        allocated_bytes_median: median(&allocated_bytes),
        peak_allocated_bytes_median: median(&peak_bytes),
        graph_exact,
        status: if precision_met {
            "Precise"
        } else {
            "Inconclusive"
        }
        .into(),
    }
}

fn paired_comparisons(
    manifest: &BenchmarkManifest,
    by_cell: &BTreeMap<&str, Vec<&WorkerReport>>,
) -> Vec<PairedComparison> {
    let mut comparisons = Vec::new();
    for cell in &manifest.cells {
        let Some(baseline_id) = cell.baseline_cell.as_deref() else {
            continue;
        };
        let candidate = by_process(by_cell.get(cell.id.as_str()).map_or(&[], Vec::as_slice));
        let baseline = by_process(by_cell.get(baseline_id).map_or(&[], Vec::as_slice));
        let ratios = candidate
            .iter()
            .filter_map(|(index, report)| {
                let baseline = baseline.get(index)?;
                let left = process_median(report);
                let right = process_median(baseline);
                (right > 0.0).then_some(left / right)
            })
            .collect::<Vec<_>>();
        if ratios.is_empty() {
            continue;
        }
        let ratio_ci95 = bootstrap_statistic_ci(
            &ratios,
            manifest.bootstrap_resamples,
            derive_seed(manifest.seed, &format!("{}-ratio", cell.id), ratios.len()),
            median,
        );
        comparisons.push(PairedComparison {
            cell: cell.id.clone(),
            baseline_cell: baseline_id.into(),
            paired_processes: ratios.len(),
            median_ratio: median(&ratios),
            ratio_ci95,
            status: if ratios.len() >= manifest.minimum_processes {
                "Paired"
            } else {
                "Inconclusive"
            }
            .into(),
        });
    }
    comparisons
}

fn scaling_curves(manifest: &BenchmarkManifest, cells: &[AggregateCell]) -> Vec<ScalingCurve> {
    let summaries = cells
        .iter()
        .map(|cell| (cell.id.as_str(), cell))
        .collect::<BTreeMap<_, _>>();
    let mut groups = BTreeMap::<(String, String), Vec<(f64, f64)>>::new();
    for cell in &manifest.cells {
        if let (Some(curve), Some(strategy)) = (&cell.curve, &cell.strategy) {
            groups
                .entry((curve.clone(), strategy.clone()))
                .or_default()
                .push((
                    cell.scale as f64,
                    summaries[&cell.id.as_str()].median_ns_per_unit,
                ));
        }
    }
    groups
        .into_iter()
        .map(|((curve, strategy), mut points)| {
            points.sort_by(|left, right| left.0.total_cmp(&right.0));
            let slope = (points.len() >= 5)
                .then(|| log_log_slope(&points))
                .flatten();
            ScalingCurve {
                curve,
                strategy,
                points: points.len(),
                log_log_slope: slope,
                status: if slope.is_some() {
                    "Estimated"
                } else {
                    "InsufficientScalePoints"
                }
                .into(),
            }
        })
        .collect()
}

fn validate_worker_reports(
    manifest: &BenchmarkManifest,
    reports: &[WorkerReport],
) -> Result<(), String> {
    let cells = manifest
        .cells
        .iter()
        .map(|cell| (cell.id.as_str(), cell))
        .collect::<BTreeMap<_, _>>();
    let mut keys = BTreeSet::new();
    let mut graph_exact_by_cell = BTreeMap::new();
    for report in reports {
        let exact_telemetry_valid = if report.schema == "microfield-publication-worker-v1" {
            report.graph_exact.is_none()
        } else if report.cell.operation == "graph.exact" {
            report.graph_exact.as_ref().is_some_and(|telemetry| {
                telemetry.node_budget > 0
                    && telemetry.explored_nodes <= telemetry.node_budget
                    && (telemetry.outcome == GraphExactOutcome::Exact)
                        == telemetry.exhausted_limit.is_none()
            })
        } else {
            report.graph_exact.is_none()
        };
        let telemetry_consistent = match graph_exact_by_cell.get(report.cell.id.as_str()) {
            Some(previous) => *previous == report.graph_exact.as_ref(),
            None => {
                graph_exact_by_cell.insert(report.cell.id.as_str(), report.graph_exact.as_ref());
                true
            }
        };
        if !matches!(
            report.schema.as_str(),
            "microfield-publication-worker-v1" | "microfield-publication-worker-v2"
        ) || reports
            .first()
            .is_some_and(|first| first.schema != report.schema)
            || report.campaign_id != manifest.campaign_id
            || cells
                .get(report.cell.id.as_str())
                .map(|cell| cell.operation.as_str())
                != Some(report.cell.operation.as_str())
            || report.observations.len() != manifest.measured_observations
            || report.observations.iter().any(|sample| {
                !sample.ns_per_logical_unit.is_finite()
                    || sample.ns_per_logical_unit < 0.0
                    || sample.logical_units == 0
            })
            || !exact_telemetry_valid
            || !telemetry_consistent
            || !keys.insert((report.cell.id.as_str(), report.process_index))
        {
            return Err(format!(
                "invalid or duplicate worker report for {} process {}",
                report.cell.id, report.process_index
            ));
        }
    }
    Ok(())
}

fn by_process<'a>(reports: &[&'a WorkerReport]) -> BTreeMap<usize, &'a WorkerReport> {
    reports
        .iter()
        .map(|report| (report.process_index, *report))
        .collect()
}

fn process_median(report: &WorkerReport) -> f64 {
    median(
        &report
            .observations
            .iter()
            .map(|sample| sample.ns_per_logical_unit)
            .collect::<Vec<_>>(),
    )
}

fn ensure_new_run_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        let mut entries =
            fs::read_dir(path).map_err(|error| format!("read {}: {error}", path.display()))?;
        if entries.next().is_some() {
            return Err(format!(
                "refusing to overwrite non-empty benchmark run {}",
                path.display()
            ));
        }
    }
    fs::create_dir_all(path).map_err(|error| format!("create {}: {error}", path.display()))
}

fn worker_path(run_directory: &Path, cell_id: &str, process_index: usize) -> PathBuf {
    run_directory
        .join("raw/workers")
        .join(format!("{cell_id}-{process_index:03}.json"))
}

fn read_worker(path: &Path) -> Result<WorkerReport, String> {
    read_json(path)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn write_raw_jsonl(path: &Path, reports: &[WorkerReport]) -> Result<(), String> {
    let mut contents = Vec::new();
    for report in reports {
        serde_json::to_writer(&mut contents, report).map_err(|error| error.to_string())?;
        contents.push(b'\n');
    }
    fs::write(path, contents).map_err(|error| format!("write {}: {error}", path.display()))
}

fn read_raw_jsonl(path: &Path) -> Result<Vec<WorkerReport>, String> {
    let contents =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    contents
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line)
                .map_err(|error| format!("parse {} line {}: {error}", path.display(), index + 1))
        })
        .collect()
}

fn write_aggregate_csv(path: &Path, report: &AggregateReport) -> Result<(), String> {
    let mut contents = String::from(
        "cell,family,operation,scale,scale_unit,processes,observations,median_ns_per_unit,p95_ns_per_unit,p99_ns_per_unit,mad_ns_per_unit,median_ci95_lower,median_ci95_upper,relative_ci_half_width,precision,status,exact_outcome,exact_node_budget,exact_explored_nodes,exact_leaf_count,exact_maximum_depth,exact_path,exact_exhausted_limit\n",
    );
    for cell in &report.cells {
        let telemetry = cell.graph_exact.as_ref();
        contents.push_str(&format!(
            "{},{},{},{},{},{},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{},{},{},{},{},{},{},{},{}\n",
            cell.id,
            cell.family,
            cell.operation,
            cell.scale,
            cell.scale_unit,
            cell.process_count,
            cell.observation_count,
            cell.median_ns_per_unit,
            cell.p95_ns_per_unit,
            cell.p99_ns_per_unit,
            cell.mad_ns_per_unit,
            cell.median_ci95.lower,
            cell.median_ci95.upper,
            cell.relative_median_ci_half_width,
            cell.precision_met,
            cell.status,
            telemetry.map_or("", |value| value.outcome.as_str()),
            telemetry.map_or(String::new(), |value| value.node_budget.to_string()),
            telemetry.map_or(String::new(), |value| value.explored_nodes.to_string()),
            telemetry.map_or(String::new(), |value| value.leaf_count.to_string()),
            telemetry.map_or(String::new(), |value| value.maximum_depth.to_string()),
            telemetry.map_or("", |value| value.path.as_str()),
            telemetry
                .and_then(|value| value.exhausted_limit.map(|limit| limit.as_str()))
                .unwrap_or(""),
        ));
    }
    fs::write(path, contents).map_err(|error| format!("write {}: {error}", path.display()))
}

fn write_comparison_csv(path: &Path, report: &AggregateReport) -> Result<(), String> {
    let mut contents = String::from(
        "cell,baseline_cell,paired_processes,median_ratio,ratio_ci95_lower,ratio_ci95_upper,status\n",
    );
    for comparison in &report.comparisons {
        contents.push_str(&format!(
            "{},{},{},{:.9},{:.9},{:.9},{}\n",
            comparison.cell,
            comparison.baseline_cell,
            comparison.paired_processes,
            comparison.median_ratio,
            comparison.ratio_ci95.lower,
            comparison.ratio_ci95.upper,
            comparison.status,
        ));
    }
    fs::write(path, contents).map_err(|error| format!("write {}: {error}", path.display()))
}

fn write_markdown_report(
    path: &Path,
    environment: &EnvironmentReport,
    report: &AggregateReport,
) -> Result<(), String> {
    let mut contents = format!(
        "# Benchmark {}\n\nClasificación: `{:?}`. Claims publicables: `{}`. Las firmas medidas son resúmenes algebraicos no criptográficos.\n\n| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |\n|---|---:|---:|---:|---:|---|\n",
        report.campaign_id, environment.classification, report.claims_allowed
    );
    for cell in &report.cells {
        contents.push_str(&format!(
            "| `{}` | {} {} | {:.3} | [{:.3}, {:.3}] | {:.3} | {} |\n",
            cell.id,
            cell.scale,
            cell.scale_unit,
            cell.median_ns_per_unit,
            cell.median_ci95.lower,
            cell.median_ci95.upper,
            cell.p95_ns_per_unit,
            cell.status,
        ));
    }
    let exact_cells = report
        .cells
        .iter()
        .filter_map(|cell| cell.graph_exact.as_ref().map(|telemetry| (cell, telemetry)))
        .collect::<Vec<_>>();
    if !exact_cells.is_empty() {
        contents.push_str("\n## Telemetría exacta de grafos\n\n| Celda | Outcome | Presupuesto | Explorados | Hojas | Profundidad | Ruta | Límite agotado |\n|---|---|---:|---:|---:|---:|---|---|\n");
        for (cell, telemetry) in exact_cells {
            contents.push_str(&format!(
                "| `{}` | {} | {} | {} | {} | {} | {} | {} |\n",
                cell.id,
                telemetry.outcome.as_str(),
                telemetry.node_budget,
                telemetry.explored_nodes,
                telemetry.leaf_count,
                telemetry.maximum_depth,
                telemetry.path.as_str(),
                telemetry
                    .exhausted_limit
                    .map_or("—", |limit| limit.as_str()),
            ));
        }
    }
    if !report.comparisons.is_empty() {
        contents.push_str("\n## Comparaciones pareadas\n\n| Celda | Baseline | Ratio mediano | IC 95 % |\n|---|---|---:|---:|\n");
        for comparison in &report.comparisons {
            contents.push_str(&format!(
                "| `{}` | `{}` | {:.4} | [{:.4}, {:.4}] |\n",
                comparison.cell,
                comparison.baseline_cell,
                comparison.median_ratio,
                comparison.ratio_ci95.lower,
                comparison.ratio_ci95.upper,
            ));
        }
    }
    contents.push_str("\nLos datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.\n");
    fs::write(path, contents).map_err(|error| format!("write {}: {error}", path.display()))
}

fn write_checksums(run_directory: &Path) -> Result<(), String> {
    let names = [
        "manifest.json",
        "environment.json",
        "execution-order.json",
        "raw/workers.jsonl",
        "aggregate.json",
        "aggregate.csv",
        "comparisons.csv",
        "report.md",
    ];
    let mut lines = Vec::new();
    for name in names {
        let bytes = fs::read(run_directory.join(name))
            .map_err(|error| format!("read checksum input {name}: {error}"))?;
        lines.push(format!("{:x}  {name}", Sha256::digest(bytes)));
    }
    let mut contents = lines.join("\n");
    contents.push('\n');
    fs::write(run_directory.join("checksums.txt"), contents)
        .map_err(|error| format!("write checksums: {error}"))
}

fn duration_ns(nanos: u128) -> u64 {
    u64::try_from(nanos).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publication::model::{BenchmarkCell, HostMode};

    fn manifest() -> BenchmarkManifest {
        BenchmarkManifest {
            schema: "microfield-publication-benchmark-manifest-v1".into(),
            campaign_id: "test".into(),
            profile: CampaignProfile::Smoke,
            seed: 7,
            host_mode: HostMode::CiShared,
            minimum_processes: 2,
            maximum_processes: 2,
            precision_check_every: 1,
            warmup_observations: 1,
            measured_observations: 3,
            target_observation_ns: 1_000,
            maximum_batch_iterations: 1_024,
            bootstrap_resamples: 100,
            maximum_relative_ci_half_width: 0.5,
            cells_from: None,
            cells: vec![BenchmarkCell {
                id: "field-gf2".into(),
                family: "field".into(),
                operation: "field.gf2-128.mul".into(),
                scale: 1,
                scale_unit: "operations".into(),
                payload_bytes: 16,
                dataset_size: None,
                baseline_cell: None,
                curve: None,
                strategy: None,
            }],
        }
    }

    #[test]
    fn manifest_validation_rejects_duplicates_and_unknown_operations() {
        let mut value = manifest();
        assert!(validate_manifest(&value).is_ok());
        value.cells.push(value.cells[0].clone());
        assert!(validate_manifest(&value).is_err());
        value.cells.pop();
        value.cells[0].operation = "unknown".into();
        assert!(validate_manifest(&value).is_err());
    }

    #[test]
    fn paired_cells_share_the_same_process_input_seed() {
        let mut value = manifest();
        let mut candidate = value.cells[0].clone();
        candidate.id = "field-gf2-batch".into();
        candidate.baseline_cell = Some(value.cells[0].id.clone());
        assert_eq!(
            worker_seed(&value, &value.cells[0], 9),
            worker_seed(&value, &candidate, 9)
        );
        value.cells.push(candidate);
        assert!(validate_manifest(&value).is_ok());
    }

    #[test]
    fn worker_aggregation_preserves_process_independence() {
        let value = manifest();
        let environment = EnvironmentReport {
            schema: "environment".into(),
            classification: EnvironmentClassification::Smoke,
            host_mode: HostMode::CiShared,
            commit: "test".into(),
            tree_clean: true,
            binary_sha256: "test".into(),
            profile: "release".into(),
            architecture: "test".into(),
            operating_system: "test".into(),
            kernel: "test".into(),
            rustc: "test".into(),
            cargo: "test".into(),
            rustflags: String::new(),
            cpu: BTreeMap::new(),
            memory: BTreeMap::new(),
            frequency: BTreeMap::new(),
            temperature: BTreeMap::new(),
            filesystem: BTreeMap::new(),
            affinity: "0".into(),
            container: "none".into(),
            utc_started: "test".into(),
            disclosed_unknowns: Vec::new(),
        };
        let reports = [10.0, 20.0]
            .into_iter()
            .enumerate()
            .map(|(process_index, duration)| WorkerReport {
                schema: "microfield-publication-worker-v1".into(),
                campaign_id: "test".into(),
                cell: value.cells[0].clone(),
                process_index,
                seed: process_index as u64,
                pid: 1,
                setup_ns: 1,
                batch_iterations: 1,
                allocation_count: 0,
                allocated_bytes: 0,
                peak_allocated_bytes: 0,
                graph_exact: None,
                observations: (0..3)
                    .map(|observation_index| RawObservation {
                        observation_index,
                        elapsed_ns: duration as u64,
                        batch_iterations: 1,
                        logical_units: 1,
                        ns_per_logical_unit: duration,
                        checksum: "00".into(),
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        let report = aggregate(&value, &environment, &reports).unwrap();
        assert_eq!(report.cells[0].process_count, 2);
        assert_eq!(report.cells[0].observation_count, 6);
        assert_eq!(report.cells[0].median_ns_per_unit, 15.0);
        assert!(!report.claims_allowed);

        let mut legacy_manifest = value.clone();
        legacy_manifest.cells[0].operation = "graph.exact".into();
        legacy_manifest.cells[0].family = "graph".into();
        let mut legacy_reports = reports.clone();
        for worker in &mut legacy_reports {
            worker.cell = legacy_manifest.cells[0].clone();
        }
        let legacy = aggregate(&legacy_manifest, &environment, &legacy_reports).unwrap();
        assert_eq!(legacy.schema, "microfield-publication-aggregate-v1");
        assert!(legacy.cells[0].graph_exact.is_none());

        for worker in &mut legacy_reports {
            worker.schema = "microfield-publication-worker-v2".into();
        }
        assert!(aggregate(&legacy_manifest, &environment, &legacy_reports).is_err());
    }
}
