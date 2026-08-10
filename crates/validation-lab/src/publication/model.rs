use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CampaignProfile {
    Smoke,
    Pilot,
    Publication,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostMode {
    CiShared,
    Interactive,
    Dedicated,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkManifest {
    pub schema: String,
    pub campaign_id: String,
    pub profile: CampaignProfile,
    pub seed: u64,
    pub host_mode: HostMode,
    pub minimum_processes: usize,
    pub maximum_processes: usize,
    pub precision_check_every: usize,
    pub warmup_observations: usize,
    pub measured_observations: usize,
    pub target_observation_ns: u64,
    pub maximum_batch_iterations: u64,
    pub bootstrap_resamples: usize,
    pub maximum_relative_ci_half_width: f64,
    #[serde(default)]
    pub cells_from: Option<String>,
    #[serde(default)]
    pub cells: Vec<BenchmarkCell>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkCell {
    pub id: String,
    pub family: String,
    pub operation: String,
    pub scale: usize,
    pub scale_unit: String,
    #[serde(default = "default_payload_bytes")]
    pub payload_bytes: usize,
    #[serde(default)]
    pub dataset_size: Option<usize>,
    #[serde(default)]
    pub baseline_cell: Option<String>,
    #[serde(default)]
    pub curve: Option<String>,
    #[serde(default)]
    pub strategy: Option<String>,
}

const fn default_payload_bytes() -> usize {
    16
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawObservation {
    pub observation_index: usize,
    pub elapsed_ns: u64,
    pub batch_iterations: u64,
    pub logical_units: u64,
    pub ns_per_logical_unit: f64,
    pub checksum: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerReport {
    pub schema: String,
    pub campaign_id: String,
    pub cell: BenchmarkCell,
    pub process_index: usize,
    pub seed: u64,
    pub pid: u32,
    pub setup_ns: u64,
    pub batch_iterations: u64,
    pub allocation_count: u64,
    pub allocated_bytes: u64,
    pub peak_allocated_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_exact: Option<GraphExactTelemetry>,
    pub observations: Vec<RawObservation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GraphExactTelemetry {
    pub outcome: GraphExactOutcome,
    pub node_budget: u64,
    pub explored_nodes: u64,
    pub leaf_count: u64,
    pub maximum_depth: usize,
    pub path: GraphExactPath,
    pub exhausted_limit: Option<GraphExactLimit>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphExactOutcome {
    Exact,
    Inconclusive,
}

impl GraphExactOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphExactPath {
    ExactRefinementDiscrete,
    WeakComponentDecomposition,
    IndividualizationRefinement,
}

impl GraphExactPath {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExactRefinementDiscrete => "exact-refinement-discrete",
            Self::WeakComponentDecomposition => "weak-component-decomposition",
            Self::IndividualizationRefinement => "individualization-refinement",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphExactLimit {
    SearchNodes,
    RetainedStateCells,
    RetainedBytes,
    SearchDepth,
    ElapsedTime,
}

impl GraphExactLimit {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SearchNodes => "search-nodes",
            Self::RetainedStateCells => "retained-state-cells",
            Self::RetainedBytes => "retained-bytes",
            Self::SearchDepth => "search-depth",
            Self::ElapsedTime => "elapsed-time",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    pub confidence_level: f64,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregateCell {
    pub id: String,
    pub family: String,
    pub operation: String,
    pub scale: usize,
    pub scale_unit: String,
    pub process_count: usize,
    pub observation_count: usize,
    pub median_ns_per_unit: f64,
    pub p95_ns_per_unit: f64,
    pub p99_ns_per_unit: f64,
    pub mad_ns_per_unit: f64,
    pub median_ci95: ConfidenceInterval,
    pub p95_ci95: ConfidenceInterval,
    pub p99_ci95: ConfidenceInterval,
    pub relative_median_ci_half_width: f64,
    pub precision_met: bool,
    pub allocation_count_median: f64,
    pub allocated_bytes_median: f64,
    pub peak_allocated_bytes_median: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_exact: Option<GraphExactTelemetry>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairedComparison {
    pub cell: String,
    pub baseline_cell: String,
    pub paired_processes: usize,
    pub median_ratio: f64,
    pub ratio_ci95: ConfidenceInterval,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScalingCurve {
    pub curve: String,
    pub strategy: String,
    pub points: usize,
    pub log_log_slope: Option<f64>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregateReport {
    pub schema: String,
    pub campaign_id: String,
    pub profile: CampaignProfile,
    pub environment_classification: EnvironmentClassification,
    pub cells: Vec<AggregateCell>,
    pub comparisons: Vec<PairedComparison>,
    pub curves: Vec<ScalingCurve>,
    pub all_cells_precise: bool,
    pub claims_allowed: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnvironmentClassification {
    Smoke,
    Informative,
    Controlled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvironmentReport {
    pub schema: String,
    pub classification: EnvironmentClassification,
    pub host_mode: HostMode,
    pub commit: String,
    pub tree_clean: bool,
    pub binary_sha256: String,
    pub profile: String,
    pub architecture: String,
    pub operating_system: String,
    pub kernel: String,
    pub rustc: String,
    pub cargo: String,
    pub rustflags: String,
    pub cpu: BTreeMap<String, String>,
    pub memory: BTreeMap<String, String>,
    pub frequency: BTreeMap<String, String>,
    pub temperature: BTreeMap<String, String>,
    pub filesystem: BTreeMap<String, String>,
    pub affinity: String,
    pub container: String,
    pub utc_started: String,
    pub disclosed_unknowns: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionTask {
    pub sequence: usize,
    pub round: usize,
    pub cell: String,
    pub process_index: usize,
    pub seed: u64,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionOrder {
    pub schema: String,
    pub campaign_id: String,
    pub tasks: Vec<ExecutionTask>,
}
