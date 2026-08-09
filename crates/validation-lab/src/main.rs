use std::{env, path::PathBuf, process::ExitCode};

use microfield_validation_lab::{
    capacity, decision, g11, g12, g13_g14, load_manifest, performance, publication, run_semantic,
    write_json, write_semantic_csv,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("f6-validation: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    let mut manifest = match command.as_str() {
        "rc8-capacity" | "rc8-compare" => PathBuf::from("validation/rc/capacity-manifest-v1.json"),
        "rc10-decision" => PathBuf::from("validation/rc/decision-manifest-v1.json"),
        "publication-campaign" | "publication-worker" | "publication-analyse" => {
            PathBuf::from("validation/benchmarks/manifests/smoke-v1.json")
        }
        _ => PathBuf::from("validation/f6/manifest.json"),
    };
    let mut output = None;
    let mut baseline = None;
    let mut candidate = None;
    let mut capacity_reports = Vec::new();
    let mut consumer_reports = Vec::new();
    let mut required_ci_gates_passed = false;
    let mut cell = None;
    let mut process_index = None;
    let mut run_directory = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--manifest" => {
                manifest = PathBuf::from(args.next().ok_or("--manifest requires a path")?)
            }
            "--out" => output = Some(PathBuf::from(args.next().ok_or("--out requires a path")?)),
            "--baseline" => {
                baseline = Some(PathBuf::from(
                    args.next().ok_or("--baseline requires a path")?,
                ))
            }
            "--candidate" => {
                candidate = Some(PathBuf::from(
                    args.next().ok_or("--candidate requires a path")?,
                ))
            }
            "--capacity-report" => capacity_reports.push(PathBuf::from(
                args.next().ok_or("--capacity-report requires a path")?,
            )),
            "--consumer-report" => consumer_reports.push(PathBuf::from(
                args.next().ok_or("--consumer-report requires a path")?,
            )),
            "--required-ci-gates-passed" => required_ci_gates_passed = true,
            "--cell" => cell = Some(args.next().ok_or("--cell requires an id")?),
            "--process-index" => {
                process_index = Some(
                    args.next()
                        .ok_or("--process-index requires an integer")?
                        .parse::<usize>()
                        .map_err(|error| format!("invalid --process-index: {error}"))?,
                )
            }
            "--run-dir" => {
                run_directory = Some(PathBuf::from(
                    args.next().ok_or("--run-dir requires a path")?,
                ))
            }
            _ => return Err(format!("unknown argument {argument:?}\n{}", usage())),
        }
    }
    if command == "publication-worker" {
        let destination = output.ok_or("publication-worker requires --out")?;
        publication::run_worker(
            &manifest,
            &cell.ok_or("publication-worker requires --cell")?,
            process_index.ok_or("publication-worker requires --process-index")?,
            &destination,
        )?;
        return Ok(());
    }
    if command == "publication-campaign" {
        let destination = run_directory.ok_or("publication-campaign requires --run-dir")?;
        let report = publication::run_campaign(&manifest, &destination)?;
        println!(
            "wrote {} benchmark cells to {} ({:?}, claims_allowed={})",
            report.cells.len(),
            destination.display(),
            report.environment_classification,
            report.claims_allowed,
        );
        return Ok(());
    }
    if command == "publication-analyse" {
        let destination = run_directory.ok_or("publication-analyse requires --run-dir")?;
        let report = publication::analyse_run(&manifest, &destination)?;
        println!(
            "regenerated {} benchmark cells below {}",
            report.cells.len(),
            destination.display(),
        );
        return Ok(());
    }
    if command == "rc8-capacity" {
        let destination = output.ok_or("rc8-capacity requires --out; results are host-specific")?;
        let capacity_manifest = capacity::load_manifest(&manifest)?;
        let report = capacity::run_campaign(&capacity_manifest)?;
        write_json(&destination, &report)?;
        println!(
            "wrote host-specific RC.8 capacity report to {}",
            destination.display()
        );
        if !report.passed {
            return Err("one or more RC.8 capacity SLOs failed".into());
        }
        return Ok(());
    }
    if command == "rc8-compare" {
        let destination = output.ok_or("rc8-compare requires --out")?;
        let baseline = baseline.ok_or("rc8-compare requires --baseline")?;
        let candidate = candidate.ok_or("rc8-compare requires --candidate")?;
        let capacity_manifest = capacity::load_manifest(&manifest)?;
        let report = capacity::compare_reports(
            &baseline,
            &candidate,
            capacity_manifest.maximum_regression_percent(),
        )?;
        write_json(&destination, &report)?;
        println!("wrote RC.8 regression report to {}", destination.display());
        if !report.passed {
            return Err("one or more frozen RC.8 routes regressed beyond the threshold".into());
        }
        return Ok(());
    }
    if command == "rc10-decision" {
        let destination = output.ok_or("rc10-decision requires --out")?;
        let decision_manifest = decision::load_manifest(&manifest)?;
        let report = decision::build_report(
            &decision_manifest,
            &capacity_reports,
            &consumer_reports,
            required_ci_gates_passed,
        )?;
        write_json(&destination, &report)?;
        println!(
            "wrote RC.10 {} decision to {}",
            report.final_decision,
            destination.display()
        );
        if report.final_decision == "NotReady" {
            return Err("RC.10 emitted NotReady because one or more gates failed".into());
        }
        return Ok(());
    }
    let manifest_data = load_manifest(&manifest)?;
    let root = manifest
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .ok_or("manifest must live below the repository root")?;
    match command.as_str() {
        "semantic" => {
            let destination =
                output.unwrap_or_else(|| PathBuf::from("validation/f6/results/semantic-v1.json"));
            let report = run_semantic(&manifest_data, root)?;
            write_json(&destination, &report)?;
            let csv_destination = destination.with_extension("csv");
            write_semantic_csv(&csv_destination, &report)?;
            println!(
                "wrote deterministic semantic reports to {} and {}",
                destination.display(),
                csv_destination.display()
            );
        }
        "performance" => {
            let destination =
                output.ok_or("performance requires --out; results are host-specific")?;
            let report = performance::run_campaign(&manifest_data)?;
            write_json(&destination, &report)?;
            println!(
                "wrote host-specific performance report to {}",
                destination.display()
            );
        }
        "g11" => {
            let destination =
                output.unwrap_or_else(|| PathBuf::from("validation/f6/results/g11-v1.json"));
            let report = g11::run_campaign(&manifest_data, root)?;
            write_json(&destination, &report)?;
            println!(
                "wrote deterministic G11 report to {}",
                destination.display()
            );
        }
        "g12" => {
            let destination =
                output.unwrap_or_else(|| PathBuf::from("validation/f6/results/g12-v1.json"));
            let report = g12::run_campaign(&manifest_data)?;
            write_json(&destination, &report)?;
            println!(
                "wrote deterministic G12 report to {}",
                destination.display()
            );
        }
        "g13-g14" => {
            let destination =
                output.unwrap_or_else(|| PathBuf::from("validation/f6/results/g13-g14-v1.json"));
            let report = g13_g14::run_campaign(&manifest_data)?;
            write_json(&destination, &report)?;
            println!(
                "wrote deterministic G13/G14 report to {}",
                destination.display()
            );
        }
        _ => return Err(usage()),
    }
    Ok(())
}

fn usage() -> String {
    "usage: f6-validation <semantic|performance|g11|g12|g13-g14|rc8-capacity|rc8-compare|rc10-decision|publication-campaign|publication-worker|publication-analyse> [--manifest PATH] [--out PATH] [--run-dir PATH] [--cell ID] [--process-index N] [--baseline PATH] [--candidate PATH] [--capacity-report PATH]... [--consumer-report PATH]... [--required-ci-gates-passed]".into()
}
