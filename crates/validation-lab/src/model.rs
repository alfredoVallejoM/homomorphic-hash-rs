use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct ValidationManifest {
    pub schema_version: u32,
    pub campaign_id: String,
    pub seed: u64,
    pub signature: SignatureManifest,
    pub graph: GraphManifest,
    pub performance: PerformanceManifest,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SignatureManifest {
    pub alphabet_size: u8,
    pub exhaustive_max_length: usize,
    pub collision_max_length: usize,
    pub reconciliation_universe: u8,
    pub reconciliation_max_difference: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GraphManifest {
    pub exhaustive_max_vertices_ci: usize,
    pub exhaustive_max_vertices_full: usize,
    pub rounds: usize,
    pub exact_node_budget: u64,
    pub exact_retained_state_cells: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PerformanceManifest {
    pub warmup_iterations: usize,
    pub measured_iterations: usize,
    pub sparse_graph_vertices: Vec<usize>,
}

impl ValidationManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported manifest schema {}",
                self.schema_version
            ));
        }
        if self.campaign_id.is_empty()
            || !(2..=16).contains(&self.signature.alphabet_size)
            || self.signature.exhaustive_max_length < self.signature.collision_max_length
            || self.signature.reconciliation_universe < 8
            || self.signature.reconciliation_max_difference == 0
            || self.graph.exhaustive_max_vertices_ci > self.graph.exhaustive_max_vertices_full
            || self.graph.exhaustive_max_vertices_full != 8
            || self.graph.rounds == 0
            || self.graph.exact_node_budget == 0
            || self.graph.exact_retained_state_cells == 0
            || self.performance.measured_iterations == 0
            || self.performance.sparse_graph_vertices.is_empty()
        {
            return Err("invalid or internally inconsistent F6.V manifest".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SemanticReport {
    pub schema_version: u32,
    pub campaign_id: String,
    pub seed: u64,
    pub signatures: SignatureCampaignReport,
    pub reconciliation: ReconciliationReport,
    pub graphs: GraphCampaignReport,
}

#[derive(Clone, Debug, Serialize)]
pub struct SignatureCampaignReport {
    pub enumerated_words: u64,
    pub metamorphic_checks: u64,
    pub collision_profiles: Vec<CollisionProfile>,
    pub minimum_examples: Vec<CollisionExample>,
    pub residual_membership_control: String,
    pub applications: Vec<SignatureApplicationResult>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SignatureApplicationResult {
    pub application: String,
    pub classification: String,
    pub evidence: String,
    pub required_confirmation: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CollisionProfile {
    pub signature: String,
    pub semantic_inputs: u64,
    pub distinct_outputs: u64,
    pub collision_buckets: u64,
    pub colliding_inputs: u64,
    pub minimum_colliding_size: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CollisionExample {
    pub signature: String,
    pub left: Vec<u8>,
    pub right: Vec<u8>,
    pub classification: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReconciliationReport {
    pub exhaustive_pairs: u64,
    pub recovered_pairs: u64,
    pub rejected_over_bound: u64,
    pub maximum_symmetric_difference: usize,
    pub classification: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphCampaignReport {
    pub oracle: String,
    pub corpus_sha256: String,
    pub graph_count: u64,
    pub relabeling_checks: u64,
    pub collision_profiles: Vec<GraphCollisionProfile>,
    pub minimum_fast_collision: Option<GraphCollisionExample>,
    pub adversarial_families: Vec<AdversarialFamilyResult>,
    pub applied_verticals: Vec<AppliedVerticalResult>,
    pub incremental_work_curve: Vec<IncrementalCurvePoint>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphCollisionProfile {
    pub tier: String,
    pub distinct_outputs: u64,
    pub collision_buckets: u64,
    pub colliding_graphs: u64,
    pub colliding_pairs: u64,
    pub maximum_bucket_size: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphCollisionExample {
    pub left_graph6: String,
    pub right_graph6: String,
    pub escalated_hybrid_distinguishes: bool,
    pub escalated_global_v2_distinguishes: bool,
    pub escalated_adaptive_v2_distinguishes: bool,
    pub escalated_multi_field_distinguishes: bool,
    pub exact_distinguishes: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdversarialFamilyResult {
    pub family: String,
    pub non_isomorphic: bool,
    pub fast_distinguishes: bool,
    pub hybrid_distinguishes: bool,
    pub exact_distinguishes: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AppliedVerticalResult {
    pub vertical: String,
    pub relabeling_invariant: bool,
    pub typed_perturbation_detected: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct IncrementalCurvePoint {
    pub vertex_count: usize,
    pub edited_vertices: usize,
    pub recomputed_vertex_rounds: usize,
    pub full_vertex_rounds: usize,
    pub work_ratio: f64,
    pub matches_full_recomputation: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PerformanceReport {
    pub schema_version: u32,
    pub campaign_id: String,
    pub environment: EnvironmentReport,
    pub samples: Vec<PerformanceSample>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EnvironmentReport {
    pub architecture: String,
    pub operating_system: String,
    pub rustc: String,
    pub logical_threads: usize,
    pub detected_features: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PerformanceSample {
    pub operation: String,
    pub input_size: usize,
    pub iterations: usize,
    pub median_ns: u128,
    pub p95_ns: u128,
    pub checksum: String,
}
