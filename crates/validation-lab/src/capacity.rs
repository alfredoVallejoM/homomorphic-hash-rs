//! Host-specific RC.8 end-to-end capacity and SLO campaign.

use std::{
    collections::BTreeMap,
    fs,
    hint::black_box,
    path::Path,
    time::{Duration, Instant},
};

use algesum::{
    AdditiveDelta, AdditiveSignature, ApplicationNamespace, BinaryPolynomialEncoder,
    BoundedSetReconciler, CanonicalGraphDag, CanonicalSearchBudget, DatabaseApplyPath,
    DatabaseApplyPolicy, DatabaseColumn, DatabaseColumnType, DatabaseRow, DatabaseSchema,
    DatabaseTransactionLimits, DatabaseTransactionLog, DatabaseValue, DeltaJournal,
    FastGraphLabeler, FileChunkProfile, GraphExecution, GraphSchemaId, GraphWorkspace,
    HomomorphicSummaryTree, IncidenceGraph, IncidenceGraphBuilder, Microcanon, MicrocanonOutcome,
    MultisetDelta, MultisetSignature, PartitionedDatabase, PrimeIntegerEncoder,
    ReconciliationLimits, RevisionedSignature, RowMutation, SequenceAppend, SequenceSignature,
    SequenceTrim, SignatureBuilder, SignatureDelta, SummaryEditPath, SummaryEditPolicy,
    TrackedMultiset, TrackedSequence, TransactionDelta,
};
use allocation_counter::measure;
use microfield::{
    BinaryPolynomialField, CanonicalEncoding, Engine, Field, Fp251V1, Gf2_128V1, PackedBatch,
};
use serde::{Deserialize, Serialize};

use crate::{model::EnvironmentReport, performance::environment};

type BinaryEncoder = BinaryPolynomialEncoder;
type Database = PartitionedDatabase<Gf2_128V1, BinaryEncoder>;
type SummaryTree = HomomorphicSummaryTree<Gf2_128V1, BinaryEncoder>;

#[derive(Clone, Debug, Deserialize)]
pub struct CapacityManifest {
    schema: String,
    campaign_id: String,
    maximum_regression_percent: f64,
    warmup_iterations: usize,
    measured_iterations: usize,
    field_elements: usize,
    signature_items: usize,
    file_bytes: usize,
    database_rows: usize,
    graph_vertices: usize,
    file_edit_sizes: Vec<usize>,
    database_mutation_counts: Vec<usize>,
    file_max_incremental_edit_bytes: usize,
    database_max_incremental_mutations: usize,
    slos: Vec<WorkloadSlo>,
}

#[derive(Clone, Debug, Deserialize)]
struct WorkloadSlo {
    workload: String,
    max_p95_ns: u128,
    minimum_throughput: f64,
    max_allocated_bytes: u64,
    #[serde(default)]
    relative_to: Option<String>,
    #[serde(default)]
    max_p50_ratio: Option<f64>,
    #[serde(default)]
    action_when_relative_limit_exceeded: Option<String>,
}

impl CapacityManifest {
    fn validate(&self) -> Result<(), String> {
        if self.schema != "microfield-rc8-capacity-manifest-v1"
            || self.campaign_id.is_empty()
            || !self.maximum_regression_percent.is_finite()
            || !(0.0..=100.0).contains(&self.maximum_regression_percent)
            || self.measured_iterations < 5
            || self.field_elements == 0
            || self.signature_items == 0
            || self.file_bytes < 4_096
            || self.database_rows == 0
            || self.graph_vertices < 4
            || !valid_scale_points(&self.file_edit_sizes, self.file_bytes)
            || !valid_scale_points(&self.database_mutation_counts, self.database_rows)
            || !self
                .file_edit_sizes
                .contains(&self.file_max_incremental_edit_bytes)
            || !self
                .database_mutation_counts
                .contains(&self.database_max_incremental_mutations)
            || !self
                .file_edit_sizes
                .iter()
                .any(|scale| *scale > self.file_max_incremental_edit_bytes)
            || !self
                .database_mutation_counts
                .iter()
                .any(|scale| *scale > self.database_max_incremental_mutations)
            || self.slos.is_empty()
        {
            return Err("invalid RC.8 capacity manifest".into());
        }
        let mut names = std::collections::BTreeSet::new();
        for slo in &self.slos {
            if !names.insert(slo.workload.as_str())
                || slo.max_p95_ns == 0
                || !slo.minimum_throughput.is_finite()
                || slo.minimum_throughput < 0.0
                || slo
                    .max_p50_ratio
                    .is_some_and(|ratio| !ratio.is_finite() || ratio <= 0.0)
                || slo.relative_to.is_some() != slo.max_p50_ratio.is_some()
                || slo.relative_to.is_some() != slo.action_when_relative_limit_exceeded.is_some()
                || slo
                    .action_when_relative_limit_exceeded
                    .as_ref()
                    .is_some_and(String::is_empty)
            {
                return Err(format!("invalid RC.8 SLO for {}", slo.workload));
            }
        }
        for slo in &self.slos {
            if slo
                .relative_to
                .as_ref()
                .is_some_and(|baseline| !names.contains(baseline.as_str()))
            {
                return Err(format!(
                    "RC.8 SLO {} references an unknown baseline",
                    slo.workload
                ));
            }
        }
        Ok(())
    }

    #[must_use]
    pub const fn maximum_regression_percent(&self) -> f64 {
        self.maximum_regression_percent
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CapacityReport {
    pub schema: &'static str,
    pub campaign_id: String,
    pub environment: EnvironmentReport,
    pub samples: Vec<CapacitySample>,
    pub gates: Vec<CapacityGate>,
    pub break_even_curves: Vec<BreakEvenCurve>,
    pub passed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct BreakEvenCurve {
    pub workload: String,
    pub scale_unit: String,
    pub points: Vec<BreakEvenPoint>,
    pub first_unprofitable_scale: Option<usize>,
    pub configured_incremental_ceiling: usize,
    pub fallback_when_unprofitable: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct BreakEvenPoint {
    pub scale: usize,
    pub incremental_p50_ns: u128,
    pub rebuild_p50_ns: u128,
    pub incremental_to_rebuild_ratio: f64,
    pub incremental_profitable: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CapacitySample {
    pub workload: String,
    pub scale: usize,
    pub scale_unit: String,
    pub iterations: usize,
    pub p50_ns: u128,
    pub p95_ns: u128,
    pub p99_ns: u128,
    pub throughput_per_second: f64,
    pub allocation_count: u64,
    pub allocated_bytes: u64,
    pub peak_allocated_bytes: u64,
    pub wire_bytes: usize,
    pub persistent_bytes: usize,
    pub io_avoided_bytes: usize,
    pub checksum: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CapacityGate {
    pub workload: String,
    pub p95_within_limit: bool,
    pub throughput_within_limit: bool,
    pub allocations_within_limit: bool,
    pub peak_allocation_within_limit: bool,
    pub relative_p50_ratio: Option<f64>,
    pub relative_p50_within_limit: Option<bool>,
    pub action_when_relative_limit_exceeded: Option<String>,
    pub passed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CapacityRegressionReport {
    pub schema: &'static str,
    pub campaign_id: String,
    pub baseline_environment: ComparableEnvironment,
    pub candidate_environment: ComparableEnvironment,
    pub maximum_regression_percent: f64,
    pub gates: Vec<CapacityRegressionGate>,
    pub passed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComparableEnvironment {
    pub architecture: String,
    pub operating_system: String,
    pub rustc: String,
    pub logical_threads: usize,
    pub detected_features: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CapacityRegressionGate {
    pub workload: String,
    pub p50_regression_percent: f64,
    pub p95_regression_percent: f64,
    pub allocated_bytes_regression_percent: Option<f64>,
    pub introduced_allocated_bytes: u64,
    pub peak_allocated_bytes_regression_percent: Option<f64>,
    pub introduced_peak_allocated_bytes: u64,
    pub passed: bool,
}

#[derive(Debug, Deserialize)]
struct ComparableCapacityReport {
    schema: String,
    campaign_id: String,
    environment: ComparableEnvironment,
    samples: Vec<ComparableCapacitySample>,
}

#[derive(Debug, Deserialize)]
struct ComparableCapacitySample {
    workload: String,
    scale: usize,
    scale_unit: String,
    iterations: usize,
    p50_ns: u128,
    p95_ns: u128,
    allocated_bytes: u64,
    peak_allocated_bytes: u64,
    checksum: String,
}

#[derive(Clone, Copy)]
struct Footprint {
    wire_bytes: usize,
    persistent_bytes: usize,
    io_avoided_bytes: usize,
}

pub fn load_manifest(path: &Path) -> Result<CapacityManifest, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let manifest: CapacityManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    manifest.validate()?;
    Ok(manifest)
}

pub fn run_campaign(manifest: &CapacityManifest) -> Result<CapacityReport, String> {
    let mut samples = Vec::new();
    field_samples(manifest, &mut samples)?;
    signature_samples(manifest, &mut samples)?;
    delta_samples(manifest, &mut samples)?;
    file_samples(manifest, &mut samples)?;
    database_samples(manifest, &mut samples)?;
    graph_samples(manifest, &mut samples)?;
    let break_even_curves = break_even_curves(manifest)?;

    let by_name = samples
        .iter()
        .map(|sample| (sample.workload.as_str(), sample))
        .collect::<BTreeMap<_, _>>();
    let declared = manifest
        .slos
        .iter()
        .map(|slo| slo.workload.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let observed = by_name
        .keys()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if declared != observed {
        return Err(format!(
            "capacity workload/SLO drift: declared={declared:?} observed={observed:?}"
        ));
    }

    let gates = manifest
        .slos
        .iter()
        .map(|slo| {
            let sample = by_name[slo.workload.as_str()];
            let relative_p50_ratio = slo
                .relative_to
                .as_deref()
                .map(|baseline| sample.p50_ns as f64 / by_name[baseline].p50_ns.max(1) as f64);
            let relative_p50_within_limit = relative_p50_ratio
                .zip(slo.max_p50_ratio)
                .map(|(ratio, maximum)| ratio <= maximum);
            let p95_within_limit = sample.p95_ns <= slo.max_p95_ns;
            let throughput_within_limit = sample.throughput_per_second >= slo.minimum_throughput;
            let allocations_within_limit = sample.allocated_bytes <= slo.max_allocated_bytes;
            let peak_allocation_within_limit =
                sample.peak_allocated_bytes <= slo.max_allocated_bytes;
            let passed = p95_within_limit
                && throughput_within_limit
                && allocations_within_limit
                && peak_allocation_within_limit
                && relative_p50_within_limit.unwrap_or(true);
            CapacityGate {
                workload: slo.workload.clone(),
                p95_within_limit,
                throughput_within_limit,
                allocations_within_limit,
                peak_allocation_within_limit,
                relative_p50_ratio,
                relative_p50_within_limit,
                action_when_relative_limit_exceeded: slo
                    .action_when_relative_limit_exceeded
                    .clone(),
                passed,
            }
        })
        .collect::<Vec<_>>();
    let passed = gates.iter().all(|gate| gate.passed);
    Ok(CapacityReport {
        schema: "microfield-rc8-capacity-report-v1",
        campaign_id: manifest.campaign_id.clone(),
        environment: environment(),
        samples,
        gates,
        break_even_curves,
        passed,
    })
}

fn valid_scale_points(points: &[usize], maximum: usize) -> bool {
    !points.is_empty()
        && points.iter().all(|point| *point > 0 && *point <= maximum)
        && points.windows(2).all(|window| window[0] < window[1])
}

pub fn compare_reports(
    baseline_path: &Path,
    candidate_path: &Path,
    maximum_regression_percent: f64,
) -> Result<CapacityRegressionReport, String> {
    if !maximum_regression_percent.is_finite()
        || !(0.0..=100.0).contains(&maximum_regression_percent)
    {
        return Err("invalid RC.8 regression threshold".into());
    }
    let baseline = read_comparable_report(baseline_path)?;
    let candidate = read_comparable_report(candidate_path)?;
    compare_report_values(baseline, candidate, maximum_regression_percent)
}

fn read_comparable_report(path: &Path) -> Result<ComparableCapacityReport, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn compare_report_values(
    baseline: ComparableCapacityReport,
    candidate: ComparableCapacityReport,
    maximum_regression_percent: f64,
) -> Result<CapacityRegressionReport, String> {
    if baseline.schema != "microfield-rc8-capacity-report-v1"
        || candidate.schema != "microfield-rc8-capacity-report-v1"
        || baseline.campaign_id != candidate.campaign_id
    {
        return Err("incompatible RC.8 report schema or campaign".into());
    }
    if baseline.environment != candidate.environment {
        return Err(format!(
            "RC.8 reports came from different environments: baseline={:?} candidate={:?}",
            baseline.environment, candidate.environment
        ));
    }
    let baseline_by_name = baseline
        .samples
        .iter()
        .map(|sample| (sample.workload.as_str(), sample))
        .collect::<BTreeMap<_, _>>();
    let candidate_by_name = candidate
        .samples
        .iter()
        .map(|sample| (sample.workload.as_str(), sample))
        .collect::<BTreeMap<_, _>>();
    if baseline_by_name.len() != baseline.samples.len()
        || candidate_by_name.len() != candidate.samples.len()
        || baseline_by_name.keys().ne(candidate_by_name.keys())
    {
        return Err("RC.8 reports contain duplicate or different workloads".into());
    }

    let mut gates = Vec::with_capacity(baseline.samples.len());
    for (workload, baseline_sample) in baseline_by_name {
        let candidate_sample = candidate_by_name[workload];
        if baseline_sample.scale != candidate_sample.scale
            || baseline_sample.scale_unit != candidate_sample.scale_unit
            || baseline_sample.iterations != candidate_sample.iterations
            || baseline_sample.checksum != candidate_sample.checksum
        {
            return Err(format!(
                "RC.8 workload {workload} changed scale, iterations or semantic checksum"
            ));
        }
        let p50_regression_percent = regression_percent(
            baseline_sample.p50_ns.max(1),
            candidate_sample.p50_ns.max(1),
        );
        let p95_regression_percent = regression_percent(
            baseline_sample.p95_ns.max(1),
            candidate_sample.p95_ns.max(1),
        );
        let (allocated_bytes_regression_percent, introduced_allocated_bytes) =
            if baseline_sample.allocated_bytes == 0 {
                (None, candidate_sample.allocated_bytes)
            } else {
                (
                    Some(regression_percent(
                        baseline_sample.allocated_bytes,
                        candidate_sample.allocated_bytes,
                    )),
                    0,
                )
            };
        let (peak_allocated_bytes_regression_percent, introduced_peak_allocated_bytes) =
            if baseline_sample.peak_allocated_bytes == 0 {
                (None, candidate_sample.peak_allocated_bytes)
            } else {
                (
                    Some(regression_percent(
                        baseline_sample.peak_allocated_bytes,
                        candidate_sample.peak_allocated_bytes,
                    )),
                    0,
                )
            };
        let allocation_passed = introduced_allocated_bytes == 0
            && allocated_bytes_regression_percent
                .is_none_or(|regression| regression <= maximum_regression_percent)
            && introduced_peak_allocated_bytes == 0
            && peak_allocated_bytes_regression_percent
                .is_none_or(|regression| regression <= maximum_regression_percent);
        let passed = p50_regression_percent <= maximum_regression_percent
            && p95_regression_percent <= maximum_regression_percent
            && allocation_passed;
        gates.push(CapacityRegressionGate {
            workload: workload.into(),
            p50_regression_percent,
            p95_regression_percent,
            allocated_bytes_regression_percent,
            introduced_allocated_bytes,
            peak_allocated_bytes_regression_percent,
            introduced_peak_allocated_bytes,
            passed,
        });
    }
    let passed = gates.iter().all(|gate| gate.passed);
    Ok(CapacityRegressionReport {
        schema: "microfield-rc8-capacity-regression-report-v1",
        campaign_id: baseline.campaign_id,
        baseline_environment: baseline.environment,
        candidate_environment: candidate.environment,
        maximum_regression_percent,
        gates,
        passed,
    })
}

fn regression_percent<T>(baseline: T, candidate: T) -> f64
where
    T: Copy + Into<u128>,
{
    let baseline = baseline.into() as f64;
    let candidate = candidate.into() as f64;
    (candidate / baseline - 1.0) * 100.0
}

fn break_even_curves(manifest: &CapacityManifest) -> Result<Vec<BreakEvenCurve>, String> {
    Ok(vec![
        file_edit_break_even(manifest)?,
        database_mutation_break_even(manifest)?,
    ])
}

fn file_edit_break_even(manifest: &CapacityManifest) -> Result<BreakEvenCurve, String> {
    let profile = FileChunkProfile::fixed(4_096).map_err(debug_error)?;
    let original = deterministic_bytes(manifest.file_bytes);
    let mut points = Vec::with_capacity(manifest.file_edit_sizes.len());
    for &edit_bytes in &manifest.file_edit_sizes {
        let range_start = (original.len() - edit_bytes) / 2;
        let range_end = range_start + edit_bytes;
        let mut incremental_tree = build_tree(profile, &original)?;
        let mut replacement = vec![0xa5; edit_bytes];
        let mut replacement_byte = 0xa5_u8;
        let incremental_p50_ns = timed_p50(manifest, || {
            replacement_byte ^= 0xff;
            replacement.fill(replacement_byte);
            incremental_tree
                .replace_range(range_start..range_end, &replacement)
                .unwrap();
            checksum_field(incremental_tree.root().evaluation())
        });

        let mut rebuilt_bytes = original.clone();
        let mut rebuild_byte = 0xa5_u8;
        let rebuild_p50_ns = timed_p50(manifest, || {
            rebuild_byte ^= 0xff;
            rebuilt_bytes[range_start..range_end].fill(rebuild_byte);
            checksum_field(
                build_tree(profile, &rebuilt_bytes)
                    .unwrap()
                    .root()
                    .evaluation(),
            )
        });
        points.push(break_even_point(
            edit_bytes,
            incremental_p50_ns,
            rebuild_p50_ns,
        ));
    }
    let first_unprofitable_scale = points
        .iter()
        .find(|point| !point.incremental_profitable)
        .map(|point| point.scale);
    Ok(BreakEvenCurve {
        workload: "file.summary-tree.edit-vs-rebuild".into(),
        scale_unit: "edited-bytes".into(),
        points,
        first_unprofitable_scale,
        configured_incremental_ceiling: manifest.file_max_incremental_edit_bytes,
        fallback_when_unprofitable: "rebuild from authoritative file bytes".into(),
    })
}

fn database_mutation_break_even(manifest: &CapacityManifest) -> Result<BreakEvenCurve, String> {
    let schema = database_schema()?;
    let namespace = database_namespace();
    let limits = DatabaseTransactionLimits::default();
    let rows = (0..manifest.database_rows)
        .map(|id| database_row(id as u64, 1))
        .collect::<Vec<_>>();
    let total_iterations = manifest.warmup_iterations + manifest.measured_iterations;
    let mut points = Vec::with_capacity(manifest.database_mutation_counts.len());

    for &mutation_count in &manifest.database_mutation_counts {
        let mut database = Database::from_rows(
            namespace,
            schema.clone(),
            16,
            binary_encoder(),
            Gf2_128V1::ONE,
            rows.clone(),
        )
        .map_err(debug_error)?;
        let transactions = (0..total_iterations)
            .map(|revision| {
                let mutations = (0..mutation_count)
                    .map(|id| RowMutation::Update {
                        before: database_row(id as u64, revision as u64 + 1),
                        after: database_row(id as u64, revision as u64 + 2),
                    })
                    .collect();
                TransactionDelta::new(namespace, &schema, revision as u64, mutations).unwrap()
            })
            .collect::<Vec<_>>();
        let mut transaction_index = 0_usize;
        let incremental_p50_ns = timed_p50(manifest, || {
            database
                .apply_transaction(&transactions[transaction_index], limits)
                .unwrap();
            transaction_index += 1;
            checksum_field(database.summary().unwrap().evaluation())
        });

        let mut rebuild_version = 1_u64;
        let rebuild_p50_ns = timed_p50(manifest, || {
            rebuild_version += 1;
            let mut candidate_rows = rows.clone();
            for (id, row) in candidate_rows.iter_mut().enumerate().take(mutation_count) {
                *row = database_row(id as u64, rebuild_version);
            }
            let rebuilt = Database::from_rows(
                namespace,
                schema.clone(),
                16,
                binary_encoder(),
                Gf2_128V1::ONE,
                candidate_rows,
            )
            .unwrap();
            checksum_field(rebuilt.summary().unwrap().evaluation())
        });
        points.push(break_even_point(
            mutation_count,
            incremental_p50_ns,
            rebuild_p50_ns,
        ));
    }
    let first_unprofitable_scale = points
        .iter()
        .find(|point| !point.incremental_profitable)
        .map(|point| point.scale);
    Ok(BreakEvenCurve {
        workload: "database.transaction-vs-rebuild".into(),
        scale_unit: "row-mutations".into(),
        points,
        first_unprofitable_scale,
        configured_incremental_ceiling: manifest.database_max_incremental_mutations,
        fallback_when_unprofitable: "rebuild from the authoritative table".into(),
    })
}

fn break_even_point(
    scale: usize,
    incremental_p50_ns: u128,
    rebuild_p50_ns: u128,
) -> BreakEvenPoint {
    let incremental_to_rebuild_ratio = incremental_p50_ns as f64 / rebuild_p50_ns.max(1) as f64;
    BreakEvenPoint {
        scale,
        incremental_p50_ns,
        rebuild_p50_ns,
        incremental_to_rebuild_ratio,
        incremental_profitable: incremental_to_rebuild_ratio <= 1.0,
    }
}

fn timed_p50(manifest: &CapacityManifest, mut action: impl FnMut() -> u64) -> u128 {
    for _ in 0..manifest.warmup_iterations {
        black_box(action());
    }
    let mut durations = Vec::with_capacity(manifest.measured_iterations);
    for _ in 0..manifest.measured_iterations {
        let start = Instant::now();
        black_box(action());
        durations.push(start.elapsed());
    }
    durations.sort_unstable();
    percentile(&durations, 50).as_nanos()
}

fn field_samples(
    manifest: &CapacityManifest,
    samples: &mut Vec<CapacitySample>,
) -> Result<(), String> {
    let count = manifest.field_elements;
    let left = Gf2_128V1::from_polynomial_bytes_mod(&[0xa5; 16]);
    let right = Gf2_128V1::from_polynomial_bytes_mod(&[0x3c; 16]);
    samples.push(benchmark(
        "field.gf2-128.scalar-mul",
        count,
        "field-operations",
        manifest,
        Footprint::zero(),
        || {
            let mut state = left;
            for _ in 0..count {
                state *= right;
            }
            checksum_field(state)
        },
    ));

    let lhs = vec![left; count];
    let rhs = vec![right; count];
    let mut output = vec![Gf2_128V1::ZERO; count];
    let engine = Engine::<Gf2_128V1>::builder()
        .expected_batch(count)
        .detect()
        .map_err(debug_error)?;
    samples.push(benchmark(
        "field.gf2-128.detected-batch-mul",
        count,
        "field-elements",
        manifest,
        Footprint::zero(),
        || {
            engine.mul_into(&mut output, &lhs, &rhs).unwrap();
            checksum_field(output[count - 1])
        },
    ));

    let packed_lhs = PackedBatch::from_aos(&engine, &lhs).map_err(debug_error)?;
    let packed_rhs = PackedBatch::from_aos(&engine, &rhs).map_err(debug_error)?;
    let mut packed_output = PackedBatch::new(&engine, count).map_err(debug_error)?;
    samples.push(benchmark(
        "field.gf2-128.packed-mul-kernel",
        count,
        "field-elements",
        manifest,
        Footprint::zero(),
        || {
            engine
                .mul_packed_into(&mut packed_output, &packed_lhs, &packed_rhs)
                .unwrap();
            black_box(packed_output.as_view());
            packed_output.len() as u64
        },
    ));

    let mut packed_pipeline_lhs = PackedBatch::new(&engine, count).map_err(debug_error)?;
    let mut packed_pipeline_rhs = PackedBatch::new(&engine, count).map_err(debug_error)?;
    let mut packed_pipeline_output = PackedBatch::new(&engine, count).map_err(debug_error)?;
    let mut unpacked = vec![Gf2_128V1::ZERO; count];
    samples.push(benchmark(
        "field.gf2-128.packed-mul-pipeline",
        count,
        "field-elements",
        manifest,
        Footprint::zero(),
        || {
            packed_pipeline_lhs.pack_from(&lhs).unwrap();
            packed_pipeline_rhs.pack_from(&rhs).unwrap();
            engine
                .mul_packed_into(
                    &mut packed_pipeline_output,
                    &packed_pipeline_lhs,
                    &packed_pipeline_rhs,
                )
                .unwrap();
            packed_pipeline_output.unpack_into(&mut unpacked).unwrap();
            checksum_field(unpacked[count - 1])
        },
    ));
    Ok(())
}

fn signature_samples(
    manifest: &CapacityManifest,
    samples: &mut Vec<CapacitySample>,
) -> Result<(), String> {
    let items = payloads(manifest.signature_items);
    let refs = || items.iter().map(Vec::as_slice);
    let half = items.len() / 2;
    let left_additive = build_additive(&items[..half]);
    let right_additive = build_additive(&items[half..]);
    let left_sequence = build_sequence(&items[..half]);
    let right_sequence = build_sequence(&items[half..]);

    samples.push(benchmark(
        "signature.additive.ingest",
        items.len(),
        "items",
        manifest,
        Footprint::zero(),
        || {
            let mut signature = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
            signature.absorb_many(refs()).unwrap();
            checksum_field(signature.state())
        },
    ));
    samples.push(benchmark(
        "signature.sequence.ingest",
        items.len(),
        "items",
        manifest,
        Footprint::zero(),
        || {
            let mut signature =
                SequenceSignature::<Fp251V1, _>::new(prime_encoder(), sequence_base()).unwrap();
            signature.push_many(refs()).unwrap();
            checksum_field(signature.state())
        },
    ));
    samples.push(benchmark(
        "signature.bidirectional-sequence.ingest",
        items.len(),
        "items",
        manifest,
        Footprint::zero(),
        || {
            let builder = SignatureBuilder::<Fp251V1, _>::new(prime_encoder());
            let mut signature = builder.bidirectional_sequence(sequence_base()).unwrap();
            signature.push_many(refs()).unwrap();
            checksum_bytes(&signature.to_canonical_bytes())
        },
    ));
    samples.push(benchmark(
        "signature.multiset.ingest",
        items.len(),
        "items",
        manifest,
        Footprint::zero(),
        || {
            let mut signature =
                MultisetSignature::<Fp251V1, _>::new(prime_encoder(), Fp251V1::from_u64_mod(11));
            signature.insert_many(refs()).unwrap();
            checksum_field(signature.nonzero_product())
        },
    ));
    samples.push(benchmark(
        "signature.multi-evaluation-multiset-k2.ingest",
        items.len(),
        "items",
        manifest,
        Footprint::zero(),
        || {
            let builder = SignatureBuilder::<Fp251V1, _>::new(prime_encoder());
            let mut signature = builder
                .multi_evaluation_multiset([Fp251V1::from_u64_mod(11), Fp251V1::from_u64_mod(13)])
                .unwrap();
            signature.insert_many(refs()).unwrap();
            checksum_bytes(&signature.to_canonical_bytes())
        },
    ));
    samples.push(benchmark(
        "signature.multi-evaluation-sequence-k2.ingest",
        items.len(),
        "items",
        manifest,
        Footprint::zero(),
        || {
            let builder = SignatureBuilder::<Fp251V1, _>::new(prime_encoder());
            let mut signature = builder
                .multi_evaluation_sequence([sequence_base(), Fp251V1::from_u64_mod(17)])
                .unwrap();
            signature.push_many(refs()).unwrap();
            checksum_bytes(&signature.to_canonical_bytes())
        },
    ));
    samples.push(benchmark(
        "signature.additive.direct-field-merge",
        items.len(),
        "items",
        manifest,
        Footprint::zero(),
        || checksum_field(left_additive.state() + right_additive.state()),
    ));
    samples.push(benchmark(
        "signature.additive.merge",
        items.len(),
        "items",
        manifest,
        Footprint {
            wire_bytes: left_additive.to_canonical_bytes().len()
                + right_additive.to_canonical_bytes().len(),
            persistent_bytes: left_additive.to_canonical_bytes().len(),
            io_avoided_bytes: items.len() * 16,
        },
        || checksum_field(left_additive.combine(&right_additive).unwrap().state()),
    ));
    samples.push(benchmark(
        "signature.sequence.concatenate",
        items.len(),
        "items",
        manifest,
        Footprint {
            wire_bytes: right_sequence.to_canonical_bytes().len(),
            persistent_bytes: left_sequence.to_canonical_bytes().len(),
            io_avoided_bytes: items.len() * 16,
        },
        || checksum_field(left_sequence.concatenate(&right_sequence).unwrap().state()),
    ));

    let mut tracked_multiset = TrackedMultiset::new(prime_encoder(), Fp251V1::ONE);
    for item in &items {
        tracked_multiset.insert(item).unwrap();
    }
    let multiset_snapshot = tracked_multiset.to_snapshot_bytes().unwrap();
    let removal = items[items.len() / 3].clone();
    samples.push(benchmark(
        "signature.tracked-multiset.remove",
        items.len(),
        "tracked-items",
        manifest,
        Footprint::zero(),
        || {
            let mut candidate = tracked_multiset.clone();
            candidate.remove(&removal).unwrap();
            checksum_field(candidate.signature().nonzero_product())
        },
    ));
    samples.push(benchmark(
        "signature.tracked-multiset.restore",
        multiset_snapshot.len(),
        "snapshot-bytes",
        manifest,
        Footprint {
            wire_bytes: multiset_snapshot.len(),
            persistent_bytes: multiset_snapshot.len(),
            io_avoided_bytes: 0,
        },
        || {
            let restored = TrackedMultiset::<Fp251V1, _>::from_snapshot_bytes(
                prime_encoder(),
                Fp251V1::ONE,
                &multiset_snapshot,
            )
            .unwrap();
            checksum_field(restored.signature().nonzero_product())
        },
    ));

    let mut tracked_sequence = TrackedSequence::new(prime_encoder(), sequence_base()).unwrap();
    for item in &items {
        tracked_sequence.push(item).unwrap();
    }
    let snapshot = tracked_sequence.to_snapshot_bytes().unwrap();
    samples.push(benchmark(
        "signature.tracked-sequence.restore",
        snapshot.len(),
        "snapshot-bytes",
        manifest,
        Footprint {
            wire_bytes: snapshot.len(),
            persistent_bytes: snapshot.len(),
            io_avoided_bytes: 0,
        },
        || {
            let restored = TrackedSequence::<Fp251V1, _>::from_snapshot_bytes(
                prime_encoder(),
                sequence_base(),
                &snapshot,
            )
            .unwrap();
            checksum_field(restored.signature().state())
        },
    ));
    Ok(())
}

fn delta_samples(
    manifest: &CapacityManifest,
    samples: &mut Vec<CapacitySample>,
) -> Result<(), String> {
    let namespace = namespace();
    let initial = payloads(manifest.signature_items);
    let state = RevisionedSignature::new(namespace, build_additive(&initial));
    let removed = build_additive(&[]);
    let added = build_additive(&[b"one-new-item".to_vec()]);
    let delta = AdditiveDelta::new(namespace, 0, removed.clone(), added.clone()).unwrap();
    let delta_wire = delta.to_canonical_bytes();
    samples.push(benchmark(
        "delta.additive.generate",
        1,
        "operations",
        manifest,
        Footprint {
            wire_bytes: delta_wire.len(),
            persistent_bytes: delta_wire.len(),
            io_avoided_bytes: 0,
        },
        || {
            checksum_bytes(
                &AdditiveDelta::new(namespace, 0, removed.clone(), added.clone())
                    .unwrap()
                    .to_canonical_bytes(),
            )
        },
    ));
    samples.push(benchmark(
        "delta.additive.apply",
        initial.len(),
        "base-items",
        manifest,
        Footprint {
            wire_bytes: delta_wire.len(),
            persistent_bytes: delta_wire.len(),
            io_avoided_bytes: initial.len() * 16,
        },
        || {
            let mut candidate = state.clone();
            candidate.apply(&delta).unwrap();
            checksum_field(candidate.state().state())
        },
    ));

    let mut applied_state = state.clone();
    applied_state.apply(&delta).unwrap();
    let rollback = AdditiveDelta::new(namespace, 1, added, removed).unwrap();
    samples.push(benchmark(
        "delta.additive.rollback",
        initial.len(),
        "base-items",
        manifest,
        Footprint {
            wire_bytes: rollback.to_canonical_bytes().len(),
            persistent_bytes: rollback.to_canonical_bytes().len(),
            io_avoided_bytes: initial.len() * 16,
        },
        || {
            let mut candidate = applied_state.clone();
            candidate.apply(&rollback).unwrap();
            checksum_field(candidate.state().state())
        },
    ));

    let multiset_state = RevisionedSignature::new(namespace, build_multiset(&initial));
    let multiset_delta = MultisetDelta::new(
        namespace,
        0,
        build_multiset(&[initial[0].clone()]),
        build_multiset(&[b"multiset-new-item".to_vec()]),
    )
    .unwrap();
    samples.push(benchmark(
        "delta.multiset.apply",
        initial.len(),
        "base-items",
        manifest,
        Footprint {
            wire_bytes: multiset_delta.to_canonical_bytes().len(),
            persistent_bytes: multiset_delta.to_canonical_bytes().len(),
            io_avoided_bytes: initial.len() * 16,
        },
        || {
            let mut candidate = multiset_state.clone();
            candidate.apply(&multiset_delta).unwrap();
            checksum_field(candidate.state().nonzero_product())
        },
    ));

    let journal_len = 64_usize;
    let mut journal = DeltaJournal::new();
    for revision in 0..journal_len {
        journal
            .append(
                AdditiveDelta::new(
                    namespace,
                    revision as u64,
                    build_additive(&[]),
                    build_additive(&[revision.to_le_bytes().to_vec()]),
                )
                .unwrap(),
            )
            .unwrap();
    }
    let journal_wire = journal.to_canonical_bytes().unwrap();
    samples.push(benchmark(
        "delta.journal.replay",
        journal_len,
        "deltas",
        manifest,
        Footprint {
            wire_bytes: journal_wire.len(),
            persistent_bytes: journal_wire.len(),
            io_avoided_bytes: 0,
        },
        || {
            let mut candidate = RevisionedSignature::new(namespace, build_additive(&[]));
            journal.replay(&mut candidate).unwrap();
            checksum_field(candidate.state().state())
        },
    ));

    let suffix = build_sequence(&initial[initial.len().saturating_sub(64)..]);
    let append = SequenceAppend::new(namespace, 0, suffix.clone()).unwrap();
    let append_state = RevisionedSignature::new(
        namespace,
        build_sequence(&initial[..initial.len().saturating_sub(64)]),
    );
    samples.push(benchmark(
        "delta.sequence.append",
        initial.len(),
        "base-items",
        manifest,
        Footprint {
            wire_bytes: append.to_canonical_bytes().len(),
            persistent_bytes: append.to_canonical_bytes().len(),
            io_avoided_bytes: initial.len() * 16,
        },
        || {
            let mut candidate = append_state.clone();
            candidate.apply(&append).unwrap();
            checksum_field(candidate.state().state())
        },
    ));
    let sequence_state = RevisionedSignature::new(namespace, build_sequence(&initial));
    let trim = SequenceTrim::new(namespace, 0, suffix).unwrap();
    samples.push(benchmark(
        "delta.sequence.trim",
        initial.len(),
        "base-items",
        manifest,
        Footprint {
            wire_bytes: trim.to_canonical_bytes().len(),
            persistent_bytes: trim.to_canonical_bytes().len(),
            io_avoided_bytes: initial.len() * 16,
        },
        || {
            let mut candidate = sequence_state.clone();
            candidate.apply(&trim).unwrap();
            checksum_field(candidate.state().state())
        },
    ));
    Ok(())
}

fn file_samples(
    manifest: &CapacityManifest,
    samples: &mut Vec<CapacitySample>,
) -> Result<(), String> {
    let profile = FileChunkProfile::fixed(4_096).map_err(debug_error)?;
    let mut bytes = deterministic_bytes(manifest.file_bytes);
    let mut tree = build_tree(profile, &bytes)?;
    let checkpoint_bytes = tree.to_checkpoint_bytes().map_err(debug_error)?.len();
    let mut edit_byte = 0_u8;
    samples.push(benchmark(
        "file.summary-tree.local-edit",
        bytes.len(),
        "file-bytes",
        manifest,
        Footprint {
            wire_bytes: 1,
            persistent_bytes: checkpoint_bytes,
            io_avoided_bytes: bytes.len().saturating_sub(1),
        },
        || {
            edit_byte ^= 1;
            tree.replace_range(17..18, &[edit_byte]).unwrap();
            checksum_field(tree.root().evaluation())
        },
    ));

    let mut append_tree = build_tree(profile, &bytes)?;
    let append_bytes = deterministic_bytes(64);
    samples.push(benchmark(
        "file.summary-tree.append",
        bytes.len(),
        "file-bytes",
        manifest,
        Footprint {
            wire_bytes: append_bytes.len(),
            persistent_bytes: checkpoint_bytes,
            io_avoided_bytes: bytes.len(),
        },
        || {
            append_tree.append(&append_bytes).unwrap();
            checksum_field(append_tree.root().evaluation())
        },
    ));

    let mut boundary_tree = build_tree(profile, &bytes)?;
    let inserted = deterministic_bytes(31);
    let mut insertion_offset = bytes.len() / 2;
    samples.push(benchmark(
        "file.summary-tree.boundary-insert",
        bytes.len(),
        "file-bytes",
        manifest,
        Footprint {
            wire_bytes: inserted.len(),
            persistent_bytes: checkpoint_bytes,
            io_avoided_bytes: 0,
        },
        || {
            boundary_tree
                .insert_range(insertion_offset, &inserted)
                .unwrap();
            insertion_offset += inserted.len();
            checksum_field(boundary_tree.root().evaluation())
        },
    ));

    let mut policy_tree = build_tree(profile, &bytes)?;
    let policy = SummaryEditPolicy::new(manifest.file_max_incremental_edit_bytes);
    let mut policy_replacement = vec![0x3c; bytes.len()];
    let mut policy_byte = 0x3c_u8;
    samples.push(benchmark(
        "file.summary-tree.policy-fallback",
        bytes.len(),
        "edited-bytes",
        manifest,
        Footprint {
            wire_bytes: bytes.len(),
            persistent_bytes: checkpoint_bytes,
            io_avoided_bytes: 0,
        },
        || {
            policy_byte ^= 0xff;
            policy_replacement.fill(policy_byte);
            let report = policy_tree
                .replace_range_with_policy(0..bytes.len(), &policy_replacement, policy)
                .unwrap();
            assert_eq!(report.path(), SummaryEditPath::BoundaryRebuild);
            checksum_field(policy_tree.root().evaluation())
        },
    ));

    let mut rebuild_byte = 0_u8;
    samples.push(benchmark(
        "file.summary-tree.full-rebuild",
        bytes.len(),
        "file-bytes",
        manifest,
        Footprint {
            wire_bytes: bytes.len(),
            persistent_bytes: checkpoint_bytes,
            io_avoided_bytes: 0,
        },
        || {
            rebuild_byte ^= 1;
            bytes[17] = rebuild_byte;
            checksum_field(build_tree(profile, &bytes).unwrap().root().evaluation())
        },
    ));
    Ok(())
}

fn database_samples(
    manifest: &CapacityManifest,
    samples: &mut Vec<CapacitySample>,
) -> Result<(), String> {
    let schema = database_schema()?;
    let namespace = database_namespace();
    let limits = DatabaseTransactionLimits::default();
    let rows = (0..manifest.database_rows)
        .map(|id| database_row(id as u64, 1))
        .collect::<Vec<_>>();
    let mut database = Database::from_rows(
        namespace,
        schema.clone(),
        16,
        binary_encoder(),
        Gf2_128V1::ONE,
        rows.clone(),
    )
    .map_err(debug_error)?;
    let iterations = manifest.warmup_iterations + manifest.measured_iterations + 1;
    let transactions = (0..iterations)
        .map(|revision| {
            let before = database_row(0, revision as u64 + 1);
            let after = database_row(0, revision as u64 + 2);
            TransactionDelta::new(
                namespace,
                &schema,
                revision as u64,
                vec![RowMutation::Update { before, after }],
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let transaction_wire = transactions[0].to_canonical_bytes().len();
    let generation_before = rows[0].clone();
    let generation_after = database_row(0, generation_before.version() + 1);
    samples.push(benchmark(
        "database.transaction.generate",
        1,
        "row-mutations",
        manifest,
        Footprint {
            wire_bytes: transaction_wire,
            persistent_bytes: transaction_wire,
            io_avoided_bytes: 0,
        },
        || {
            checksum_bytes(
                &TransactionDelta::new(
                    namespace,
                    &schema,
                    0,
                    vec![RowMutation::Update {
                        before: generation_before.clone(),
                        after: generation_after.clone(),
                    }],
                )
                .unwrap()
                .to_canonical_bytes(),
            )
        },
    ));
    let mut transaction_index = 0;
    samples.push(benchmark(
        "database.transaction.apply",
        rows.len(),
        "table-rows",
        manifest,
        Footprint {
            wire_bytes: transaction_wire,
            persistent_bytes: transaction_wire,
            io_avoided_bytes: rows.len().saturating_sub(1) * 32,
        },
        || {
            database
                .apply_transaction(&transactions[transaction_index], limits)
                .unwrap();
            transaction_index += 1;
            checksum_field(database.summary().unwrap().evaluation())
        },
    ));

    let fallback_mutations = manifest
        .database_mutation_counts
        .iter()
        .copied()
        .find(|count| *count > manifest.database_max_incremental_mutations)
        .expect("validated RC.8 database fallback point");
    let mut policy_database = Database::from_rows(
        namespace,
        schema.clone(),
        16,
        binary_encoder(),
        Gf2_128V1::ONE,
        rows.clone(),
    )
    .map_err(debug_error)?;
    let mut policy_targets = Vec::with_capacity(iterations);
    let policy_transactions = (0..iterations)
        .map(|revision| {
            let mutations = (0..fallback_mutations)
                .map(|id| RowMutation::Update {
                    before: database_row(id as u64, revision as u64 + 1),
                    after: database_row(id as u64, revision as u64 + 2),
                })
                .collect::<Vec<_>>();
            let mut target = rows.clone();
            for (id, row) in target.iter_mut().enumerate().take(fallback_mutations) {
                *row = database_row(id as u64, revision as u64 + 2);
            }
            policy_targets.push(target);
            TransactionDelta::new(namespace, &schema, revision as u64, mutations).unwrap()
        })
        .collect::<Vec<_>>();
    let policy_wire = policy_transactions[0].to_canonical_bytes().len();
    let policy = DatabaseApplyPolicy::new(manifest.database_max_incremental_mutations);
    let mut policy_index = 0_usize;
    samples.push(benchmark(
        "database.transaction.policy-fallback",
        fallback_mutations,
        "row-mutations",
        manifest,
        Footprint {
            wire_bytes: policy_wire,
            persistent_bytes: policy_wire,
            io_avoided_bytes: 0,
        },
        || {
            let target = &policy_targets[policy_index];
            let report = policy_database
                .apply_transaction_with_policy(
                    &policy_transactions[policy_index],
                    limits,
                    policy,
                    || target.clone(),
                )
                .unwrap();
            assert_eq!(report.path(), DatabaseApplyPath::AuthoritativeRebuild);
            policy_index += 1;
            checksum_field(policy_database.summary().unwrap().evaluation())
        },
    ));

    let mut rebuild_version = 1_u64;
    samples.push(benchmark(
        "database.table.rebuild",
        rows.len(),
        "table-rows",
        manifest,
        Footprint::zero(),
        || {
            rebuild_version += 1;
            let mut candidate_rows = rows.clone();
            candidate_rows[0] = database_row(0, rebuild_version);
            let rebuilt = Database::from_rows(
                namespace,
                schema.clone(),
                16,
                binary_encoder(),
                Gf2_128V1::ONE,
                candidate_rows,
            )
            .unwrap();
            checksum_field(rebuilt.summary().unwrap().evaluation())
        },
    ));

    let replay_len = 64_usize;
    let mut transaction_log = DatabaseTransactionLog::new();
    for revision in 0..replay_len {
        transaction_log
            .append(
                TransactionDelta::new(
                    namespace,
                    &schema,
                    revision as u64,
                    vec![RowMutation::Insert(database_row(
                        10_000 + revision as u64,
                        1,
                    ))],
                )
                .unwrap(),
            )
            .unwrap();
    }
    let transaction_log_bytes = transaction_log.to_canonical_bytes().map_err(debug_error)?;
    let empty_database = Database::from_rows(
        namespace,
        schema.clone(),
        16,
        binary_encoder(),
        Gf2_128V1::ONE,
        Vec::<DatabaseRow>::new(),
    )
    .map_err(debug_error)?;
    samples.push(benchmark(
        "database.transaction-log.replay",
        replay_len,
        "transactions",
        manifest,
        Footprint {
            wire_bytes: transaction_log_bytes.len(),
            persistent_bytes: transaction_log_bytes.len(),
            io_avoided_bytes: 0,
        },
        || {
            let mut candidate = empty_database.clone();
            transaction_log.replay(&mut candidate, limits).unwrap();
            checksum_field(candidate.summary().unwrap().evaluation())
        },
    ));

    let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(128, 8, 8, 4_096))
        .map_err(debug_error)?;
    let left = (0..64_u16).collect::<Vec<_>>();
    let mut right = left.clone();
    right.splice(10..14, [90, 91, 92, 93]);
    right.sort_unstable();
    let left_sketch = reconciler.sketch(&left).map_err(debug_error)?;
    let right_sketch = reconciler.sketch(&right).map_err(debug_error)?;
    let wire_bytes = left_sketch.to_canonical_bytes().len();
    samples.push(benchmark(
        "database.reconciliation.decode",
        8,
        "symmetric-difference",
        manifest,
        Footprint {
            wire_bytes,
            persistent_bytes: wire_bytes,
            io_avoided_bytes: left.len() * 2,
        },
        || {
            let recovered = reconciler
                .reconcile(&left_sketch, &right_sketch, &right)
                .unwrap();
            (recovered.only_left().len() + recovered.only_right().len()) as u64
        },
    ));
    Ok(())
}

fn graph_samples(
    manifest: &CapacityManifest,
    samples: &mut Vec<CapacitySample>,
) -> Result<(), String> {
    let graph = sparse_cycle(manifest.graph_vertices)?;
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 2>::new(prime_encoder(), algesum::RefinementProfile::fast())
            .map_err(debug_error)?;
    let prepared = labeler.prepare(&graph).map_err(debug_error)?;
    let mut workspace = GraphWorkspace::new();
    workspace.reserve_for(graph.vertex_count(), 4);
    samples.push(benchmark(
        "graph.fast-filter.prepared",
        graph.vertex_count(),
        "vertices",
        manifest,
        Footprint::zero(),
        || {
            let analysis = labeler
                .analyze_prepared_with_workspace(
                    &prepared,
                    &mut workspace,
                    GraphExecution::Sequential,
                )
                .unwrap();
            checksum_field(analysis.signature().lanes()[0])
        },
    ));

    let exact_graph = distinct_path(10)?;
    let schema = GraphSchemaId::derive(b"rc8-capacity-graph-v1");
    let canonizer = Microcanon::new(schema);
    let budget = CanonicalSearchBudget::new(1_000_000);
    samples.push(benchmark(
        "graph.microcanon.exact",
        exact_graph.vertex_count(),
        "vertices",
        manifest,
        Footprint::zero(),
        || match canonizer.canonicalize(&exact_graph, budget).unwrap() {
            MicrocanonOutcome::Exact { form, .. } => checksum_bytes(form.bytes()),
            MicrocanonOutcome::Inconclusive { report } => panic!("capacity graph: {report:?}"),
        },
    ));

    let mut dag = CanonicalGraphDag::new(schema);
    dag.resolve(&exact_graph, &canonizer, budget, &[], None)
        .map_err(debug_error)?;
    let dag_bytes = dag.to_canonical_bytes().len();
    samples.push(benchmark(
        "graph.canonical-dag.reuse",
        exact_graph.vertex_count(),
        "vertices",
        manifest,
        Footprint {
            wire_bytes: dag_bytes,
            persistent_bytes: dag_bytes,
            io_avoided_bytes: 0,
        },
        || {
            let outcome = dag
                .resolve(&exact_graph, &canonizer, budget, &[], Some(dag.revision()))
                .unwrap();
            match outcome {
                algesum::GraphDagResolveOutcome::Reused { node, .. } => node.as_u64(),
                other => panic!("expected DAG reuse, got {other:?}"),
            }
        },
    ));
    Ok(())
}

fn benchmark(
    workload: &str,
    scale: usize,
    scale_unit: &str,
    manifest: &CapacityManifest,
    footprint: Footprint,
    mut action: impl FnMut() -> u64,
) -> CapacitySample {
    for _ in 0..manifest.warmup_iterations {
        black_box(action());
    }
    let mut durations = Vec::with_capacity(manifest.measured_iterations);
    let mut checksum = 0_u64;
    for _ in 0..manifest.measured_iterations {
        let start = Instant::now();
        checksum ^= black_box(action());
        durations.push(start.elapsed());
    }
    let allocations = measure(|| {
        black_box(action());
    });
    durations.sort_unstable();
    let p50_ns = percentile(&durations, 50).as_nanos();
    let p95_ns = percentile(&durations, 95).as_nanos();
    let p99_ns = percentile(&durations, 99).as_nanos();
    let throughput_per_second = scale as f64 * 1_000_000_000.0 / p50_ns.max(1) as f64;
    CapacitySample {
        workload: workload.into(),
        scale,
        scale_unit: scale_unit.into(),
        iterations: durations.len(),
        p50_ns,
        p95_ns,
        p99_ns,
        throughput_per_second,
        allocation_count: allocations.count_total,
        allocated_bytes: allocations.bytes_total,
        peak_allocated_bytes: allocations.bytes_max,
        wire_bytes: footprint.wire_bytes,
        persistent_bytes: footprint.persistent_bytes,
        io_avoided_bytes: footprint.io_avoided_bytes,
        checksum: format!("{checksum:016x}"),
    }
}

fn percentile(durations: &[Duration], percentile: usize) -> Duration {
    durations[((durations.len() - 1) * percentile).div_ceil(100)]
}

impl Footprint {
    const fn zero() -> Self {
        Self {
            wire_bytes: 0,
            persistent_bytes: 0,
            io_avoided_bytes: 0,
        }
    }
}

fn payloads(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|index| {
            let mut bytes = vec![0_u8; 16];
            bytes[..8].copy_from_slice(&(index as u64).to_le_bytes());
            bytes[8..].copy_from_slice(&(index as u64).rotate_left(17).to_le_bytes());
            bytes
        })
        .collect()
}

fn deterministic_bytes(count: usize) -> Vec<u8> {
    (0..count)
        .map(|index| (index as u64).wrapping_mul(131).wrapping_add(17) as u8)
        .collect()
}

fn prime_encoder() -> PrimeIntegerEncoder {
    PrimeIntegerEncoder::new(0x5243_8001)
}

fn binary_encoder() -> BinaryEncoder {
    BinaryEncoder::new(0x5243_8002)
}

fn sequence_base() -> Fp251V1 {
    Fp251V1::from_u64_mod(7)
}

fn namespace() -> ApplicationNamespace {
    ApplicationNamespace::derive(b"rc8-capacity-signatures-v1")
}

fn build_additive(items: &[Vec<u8>]) -> AdditiveSignature<Fp251V1, PrimeIntegerEncoder> {
    let mut signature = AdditiveSignature::new(prime_encoder());
    signature
        .absorb_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

fn build_sequence(items: &[Vec<u8>]) -> SequenceSignature<Fp251V1, PrimeIntegerEncoder> {
    let mut signature = SequenceSignature::new(prime_encoder(), sequence_base()).unwrap();
    signature
        .push_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

fn build_multiset(items: &[Vec<u8>]) -> MultisetSignature<Fp251V1, PrimeIntegerEncoder> {
    let mut signature = MultisetSignature::new(prime_encoder(), Fp251V1::ONE);
    signature
        .insert_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

fn build_tree(profile: FileChunkProfile, bytes: &[u8]) -> Result<SummaryTree, String> {
    SummaryTree::from_bytes(
        profile,
        binary_encoder(),
        Gf2_128V1::from_polynomial_bytes_mod(&[2]),
        bytes,
    )
    .map_err(debug_error)
}

fn database_namespace() -> ApplicationNamespace {
    ApplicationNamespace::derive(b"rc8-capacity-database-v1")
}

fn database_schema() -> Result<DatabaseSchema, String> {
    DatabaseSchema::new(
        1,
        vec![
            DatabaseColumn::new("id", DatabaseColumnType::U64, false),
            DatabaseColumn::new("payload", DatabaseColumnType::Bytes, false),
        ],
        vec![0],
    )
    .map_err(debug_error)
}

fn database_row(id: u64, version: u64) -> DatabaseRow {
    DatabaseRow::new(
        version,
        vec![
            DatabaseValue::U64(id),
            DatabaseValue::Bytes(id.rotate_left(13).to_le_bytes().to_vec()),
        ],
    )
}

fn sparse_cycle(vertices: usize) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect::<Vec<_>>();
    for index in 0..vertices {
        builder
            .add_undirected_relation(
                ids[index],
                ids[(index + 1) % vertices],
                b"edge",
                b"cycle",
                1,
            )
            .map_err(debug_error)?;
    }
    builder.build().map_err(debug_error)
}

fn distinct_path(vertices: usize) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|index| builder.add_vertex((index as u64).to_le_bytes().to_vec()))
        .collect::<Vec<_>>();
    for pair in ids.windows(2) {
        builder
            .add_undirected_relation(pair[0], pair[1], b"edge", b"path", 1)
            .map_err(debug_error)?;
    }
    builder.build().map_err(debug_error)
}

fn checksum_field<F: CanonicalEncoding>(value: F) -> u64 {
    checksum_bytes(value.to_canonical().as_ref())
}

fn checksum_bytes(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .take(8)
        .enumerate()
        .fold(0_u64, |state, (index, byte)| {
            state | (u64::from(*byte) << (index * 8))
        })
}

fn debug_error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_uses_nearest_rank_without_interpolation() {
        let samples = (1..=21).map(Duration::from_nanos).collect::<Vec<_>>();
        assert_eq!(percentile(&samples, 50), Duration::from_nanos(11));
        assert_eq!(percentile(&samples, 95), Duration::from_nanos(20));
        assert_eq!(percentile(&samples, 99), Duration::from_nanos(21));
    }

    #[test]
    fn comparison_rejects_time_regression_over_three_percent() {
        let baseline = comparable_report(100, 200, 100);
        let candidate = comparable_report(104, 205, 102);
        let report = compare_report_values(baseline, candidate, 3.0).unwrap();
        assert!(!report.passed);
        assert!(!report.gates[0].passed);
    }

    #[test]
    fn comparison_accepts_improvements_and_rejects_new_allocations() {
        let baseline = comparable_report(100, 200, 0);
        let improved = comparable_report(90, 190, 0);
        assert!(
            compare_report_values(baseline, improved, 3.0)
                .unwrap()
                .passed
        );

        let baseline = comparable_report(100, 200, 0);
        let allocated = comparable_report(100, 200, 1);
        let report = compare_report_values(baseline, allocated, 3.0).unwrap();
        assert!(!report.passed);
        assert_eq!(report.gates[0].introduced_allocated_bytes, 1);
    }

    #[test]
    fn comparison_rejects_semantically_different_workloads() {
        let baseline = comparable_report(100, 200, 0);
        let mut candidate = comparable_report(100, 200, 0);
        candidate.samples[0].checksum = "different".into();
        assert!(compare_report_values(baseline, candidate, 3.0).is_err());
    }

    fn comparable_report(
        p50_ns: u128,
        p95_ns: u128,
        allocated_bytes: u64,
    ) -> ComparableCapacityReport {
        ComparableCapacityReport {
            schema: "microfield-rc8-capacity-report-v1".into(),
            campaign_id: "test".into(),
            environment: ComparableEnvironment {
                architecture: "test-arch".into(),
                operating_system: "test-os".into(),
                rustc: "test-rustc".into(),
                logical_threads: 1,
                detected_features: Vec::new(),
            },
            samples: vec![ComparableCapacitySample {
                workload: "route".into(),
                scale: 64,
                scale_unit: "items".into(),
                iterations: 21,
                p50_ns,
                p95_ns,
                allocated_bytes,
                peak_allocated_bytes: allocated_bytes,
                checksum: "same".into(),
            }],
        }
    }
}
