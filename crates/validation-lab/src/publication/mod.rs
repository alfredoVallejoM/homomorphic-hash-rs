//! Reproducible pre-RC benchmark campaign and statistical analysis.

mod environment;
mod model;
mod runner;
mod stats;
mod workloads;

pub use model::{
    AggregateReport, BenchmarkManifest, CampaignProfile, EnvironmentClassification,
    EnvironmentReport, ExecutionOrder, HostMode, WorkerReport,
};
pub use runner::{analyse_run, load_manifest, run_campaign, run_worker};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_manifest_is_valid() {
        let manifest = load_manifest(std::path::Path::new(
            "../../validation/benchmarks/manifests/smoke-v1.json",
        ))
        .expect("workspace smoke manifest");
        assert_eq!(manifest.profile, CampaignProfile::Smoke);
        assert!(manifest.cells.len() >= 12);
    }
}
