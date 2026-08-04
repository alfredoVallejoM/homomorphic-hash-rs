use super::telemetry::TelemetryRecord;

pub enum ExperimentOutcome {
    IsomorphismMatch(bool),
    L1Screening {
        rejected: bool,
        false_positive: bool,
    },
    StateReconciliation {
        target_hash_found: bool,
    },
}

/// The blueprint for a reproducible scientific demonstration.
pub trait ScientificExperiment: Send + Sync {
    /// Loads datasets, parses strings into graphs, and prepares memory.
    /// This phase is NOT timed by the benchmark runner.
    fn setup(&mut self);

    /// Executes the core Galois and Betti engine.
    /// Returns: (Outcome, L1_Shield_Time_ns, Galois_Engine_Time_ns)
    fn execute(&self) -> (ExperimentOutcome, u128, u128);

    /// Applies the experiment's declared acceptance predicate.
    ///
    /// A successful demo is observational evidence only; it does not establish
    /// collision freedom or exact graph canonization.
    fn verify(&self, outcome: &ExperimentOutcome) -> bool;

    /// Provides the base telemetry data to the orchestrator.
    fn get_base_telemetry(&self) -> TelemetryRecord;
}
