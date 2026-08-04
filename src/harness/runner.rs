use super::experiment::ScientificExperiment;
use super::telemetry::{TelemetryRecord, TelemetryRecorder};
use rayon::prelude::*;
use std::time::Instant;

pub struct BenchmarkRunner {
    experiments: Vec<Box<dyn ScientificExperiment>>,
    telemetry: TelemetryRecorder,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            experiments: Vec::new(),
            telemetry: TelemetryRecorder::new(),
        }
    }

    pub fn add_experiment(&mut self, exp: Box<dyn ScientificExperiment>) {
        self.experiments.push(exp);
    }

    /// Ignites the test suite, enforcing strict chronometry and Thread-Safety.
    pub fn ignite(&mut self, output_csv_path: &str) {
        println!("🚀 Igniting Universal Benchmark Harness...");

        // Phase 1: Setup (Untimed - I/O Heavy)
        println!(
            "⏳ Running setup phase for {} experiments...",
            self.experiments.len()
        );
        self.experiments.par_iter_mut().for_each(|exp| exp.setup());

        // Phase 2: Execution (Strictly Timed)
        println!("⚡ Executing core mathematical engines...");
        let results: Vec<TelemetryRecord> = self
            .experiments
            .par_iter()
            .map(|exp| {
                let _start = Instant::now(); // Outer timing if needed
                let (outcome, l1_time, galois_time) = exp.execute();

                // Verification Trap
                assert!(
                    exp.verify(&outcome),
                    "CRITICAL: Mathematical Verification Failed!"
                );

                // Build the record
                let mut record = exp.get_base_telemetry();
                record.l1_shield_time_ns = l1_time;
                record.galois_engine_time_ns = galois_time;

                record
            })
            .collect();

        // Phase 3: Telemetry Collection and Export
        for record in results {
            self.telemetry.record(record);
        }

        self.telemetry.export_to_csv(output_csv_path);
        println!("✅ Telemetry successfully exported to: {}", output_csv_path);
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}
