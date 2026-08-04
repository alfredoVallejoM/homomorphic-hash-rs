use std::{env, path::PathBuf, process::ExitCode};

use microfield_validation_lab::{
    g11, g12, g13_g14, load_manifest, performance, run_semantic, write_json, write_semantic_csv,
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
    let mut manifest = PathBuf::from("validation/f6/manifest.json");
    let mut output = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--manifest" => {
                manifest = PathBuf::from(args.next().ok_or("--manifest requires a path")?)
            }
            "--out" => output = Some(PathBuf::from(args.next().ok_or("--out requires a path")?)),
            _ => return Err(format!("unknown argument {argument:?}\n{}", usage())),
        }
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
    "usage: f6-validation <semantic|performance|g11|g12|g13-g14> [--manifest PATH] [--out PATH]"
        .into()
}
