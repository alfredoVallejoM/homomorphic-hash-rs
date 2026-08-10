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

    #[test]
    fn comprehensive_smoke_covers_every_signature_law_and_graph_scale() {
        let manifest = load_manifest(std::path::Path::new(
            "../../validation/benchmarks/manifests/comprehensive-smoke-v1.json",
        ))
        .expect("comprehensive smoke manifest");
        assert_eq!(manifest.profile, CampaignProfile::Smoke);
        assert_eq!(manifest.cells.len(), 65);
        for operation in [
            "signature.additive.merge-total",
            "signature.sequence.concatenate-total",
            "signature.bidirectional.concatenate-total",
            "signature.multiset.merge-total",
            "signature.multi-multiset-k2.merge-total",
            "signature.multi-sequence-k2.concatenate-total",
        ] {
            assert!(manifest
                .cells
                .iter()
                .any(|cell| cell.operation == operation));
        }
        assert!(manifest
            .cells
            .iter()
            .any(|cell| cell.operation == "graph.fast-prepared" && cell.scale == 131_072));
        assert!(manifest
            .cells
            .iter()
            .any(|cell| cell.operation == "graph.exact" && cell.scale == 14));
    }

    #[test]
    fn scaling_manifests_share_the_same_complete_cell_inventory() {
        let pilot = load_manifest(std::path::Path::new(
            "../../validation/benchmarks/manifests/pilot-scaling-v1.json",
        ))
        .expect("pilot scaling manifest");
        let publication = load_manifest(std::path::Path::new(
            "../../validation/benchmarks/manifests/publication-informative-v1.json",
        ))
        .expect("publication scaling manifest");
        let controlled = load_manifest(std::path::Path::new(
            "../../validation/benchmarks/manifests/publication-controlled-v1.json",
        ))
        .expect("controlled scaling manifest");
        assert_eq!(pilot.profile, CampaignProfile::Pilot);
        assert_eq!(publication.profile, CampaignProfile::Publication);
        assert_eq!(controlled.profile, CampaignProfile::Publication);
        assert_eq!(controlled.host_mode, HostMode::Dedicated);
        assert_eq!(pilot.cells.len(), 44);
        assert_eq!(publication.cells.len(), pilot.cells.len());
        assert_eq!(
            serde_json::to_value(&controlled.cells).expect("serialize controlled cells"),
            serde_json::to_value(&pilot.cells).expect("serialize pilot cells")
        );
    }

    #[test]
    fn bulk_scaling_manifest_covers_both_structural_adapters() {
        let manifest = load_manifest(std::path::Path::new(
            "../../validation/benchmarks/manifests/pilot-bulk-scaling-v1.json",
        ))
        .expect("bulk scaling manifest");
        assert_eq!(manifest.profile, CampaignProfile::Pilot);
        assert_eq!(manifest.cells.len(), 57);
        assert!(manifest
            .cells
            .iter()
            .any(|cell| cell.operation == "summary-tree.bulk-batch-total"));
        assert!(manifest
            .cells
            .iter()
            .any(|cell| cell.operation == "database.adaptive-transaction-total"));

        let confirmation = load_manifest(std::path::Path::new(
            "../../validation/benchmarks/manifests/pilot-bulk-scaling-v2.json",
        ))
        .expect("bulk scaling confirmation manifest");
        assert_eq!(confirmation.cells.len(), manifest.cells.len());
        let frontier = load_manifest(std::path::Path::new(
            "../../validation/benchmarks/manifests/pilot-bulk-density-frontier-v1.json",
        ))
        .expect("bulk density frontier manifest");
        assert_eq!(frontier.cells.len(), 18);
        let streaming = load_manifest(std::path::Path::new(
            "../../validation/benchmarks/manifests/pilot-db-streaming-v1.json",
        ))
        .expect("database streaming manifest");
        assert_eq!(streaming.cells.len(), 6);
    }
}
