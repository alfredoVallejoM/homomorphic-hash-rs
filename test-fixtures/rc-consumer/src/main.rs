use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: homomorphic-hash-rc-consumer ARTIFACT_DIRECTORY");
        return ExitCode::FAILURE;
    };
    match homomorphic_hash_rc_consumer::run_scenario(&path) {
        Ok(report) => {
            let report_path = path.join("rc9-report.json");
            let bytes = serde_json::to_vec_pretty(&report).expect("serializable RC.9 report");
            if let Err(error) = std::fs::write(&report_path, bytes) {
                eprintln!("RC.9 consumer: write {}: {error}", report_path.display());
                return ExitCode::FAILURE;
            }
            println!("{report:#?}\nwrote {}", report_path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("RC.9 consumer: {error}");
            ExitCode::FAILURE
        }
    }
}
