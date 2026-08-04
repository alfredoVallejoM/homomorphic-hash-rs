use serde::Serialize;
use std::fs::OpenOptions;

#[derive(Clone, Debug, Serialize)]
pub struct TelemetryRecord {
    // Context
    pub domain: String,          // e.g., "Chemistry", "Logic", "Network"
    pub experiment_name: String, // e.g., "ZINC15_L1_Screening"

    // Graph Thermodynamics
    pub vertices: usize,
    pub edges: usize,
    pub density: f64,

    // High-Precision Chronometry (Nanoseconds)
    pub parse_time_ns: u128,
    pub l1_shield_time_ns: u128,
    pub galois_engine_time_ns: u128,

    // Hardware & Efficiency Metrics
    pub l1_rejection_rate: f64,
    pub threads_utilized: usize,
    pub peak_memory_mb: f64,

    // Mathematical Verdict
    pub isomorphism_verified: bool,
    pub false_positives_detected: usize,
}

pub struct TelemetryRecorder {
    pub records: Vec<TelemetryRecord>,
}

impl TelemetryRecorder {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn record(&mut self, data: TelemetryRecord) {
        self.records.push(data);
    }

    /// Dumps the entire telemetry session to a CSV file for Python visualization.
    pub fn export_to_csv(&self, filepath: &str) {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filepath)
            .expect("Failed to open telemetry CSV file");

        let mut wtr = csv::Writer::from_writer(file);
        for record in &self.records {
            wtr.serialize(record)
                .expect("Failed to serialize telemetry record");
        }
        wtr.flush().expect("Failed to flush CSV writer");
    }
}

impl Default for TelemetryRecorder {
    fn default() -> Self {
        Self::new()
    }
}
