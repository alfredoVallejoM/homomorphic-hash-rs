use std::time::Instant;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::prelude::*;

use crate::harness::experiment::{ScientificExperiment, ExperimentOutcome};
use crate::harness::telemetry::TelemetryRecord;
use crate::domains::chemistry::smiles_parser::{SmilesParser, MolecularComplex};
use crate::engine::canonizer::CellularGaloisCanonizer;

// =========================================================================
// DEMONSTRATION 1: UNIVERSAL INVARIANCE (The SMILES Shuffle)
// =========================================================================

pub struct Demo1Level1Positional {
    smiles_targets: Vec<(&'static str, &'static str)>,
    setup_time_ns: u128,
}

impl Demo1Level1Positional {
    pub fn new() -> Self {
        Self {
            smiles_targets: vec![
                ("Ibuprofen", "CC(C)CC1=CC=C(C=C1)C(C)C(=O)O"),
                ("Aspirin", "CC(=O)OC1=CC=CC=C1C(=O)O"),
            ],
            setup_time_ns: 0,
        }
    }
}

impl ScientificExperiment for Demo1Level1Positional {
    fn setup(&mut self) {
        self.setup_time_ns = Instant::now().elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start_global = Instant::now();
        let mut all_passed = true;

        for (name, smiles) in &self.smiles_targets {
            let base_complex = SmilesParser::parse_to_complex(smiles);
            let mut expected: Option<Vec<[u64; 4]>> = None;
            let mut latencies_us = Vec::new();

            for _ in 0..50 {
                let perm = base_complex.generate_isomorphic_permutation();

                let t0 = Instant::now();
                let nodes = CellularGaloisCanonizer::canonize(&perm, perm.var_count);
                latencies_us.push(t0.elapsed().as_micros());

                let mut sigs: Vec<_> = nodes.into_iter().map(|n| n.signature).collect();
                sigs.sort_by(|a, b| a.0.cmp(&b.0));
                let current: Vec<[u64; 4]> = sigs.into_iter().map(|s| s.0).collect();

                match &expected {
                    None => expected = Some(current.clone()),
                    Some(exp) => if &current != exp { all_passed = false; break; }
                }
            }

            // Calculate Latency Variance to prove memory fragmentation does not degrade performance
            let mean = latencies_us.iter().sum::<u128>() as f64 / 50.0;
            let variance = latencies_us.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / 50.0;
            println!("    [METRIC] {}: Mean Latency = {:.2}us, StdDev = {:.2}us", name, mean, variance.sqrt());
        }
        (ExperimentOutcome::IsomorphismMatch(all_passed), 0, start_global.elapsed().as_nanos())
    }

    fn verify(&self, outcome: &ExperimentOutcome) -> bool { match outcome { ExperimentOutcome::IsomorphismMatch(res) => *res, _ => false } }
    fn get_base_telemetry(&self) -> TelemetryRecord { TelemetryRecord { domain: "Chem".to_string(), experiment_name: "Demo1_L1".to_string(), vertices: 100, edges: 0, density: 0.0, parse_time_ns: self.setup_time_ns, l1_shield_time_ns: 0, galois_engine_time_ns: 0, l1_rejection_rate: 0.0, threads_utilized: 1, peak_memory_mb: 0.0, isomorphism_verified: true, false_positives_detected: 0 } }
}

pub struct Demo1Level2Symmetry {
    smiles_targets: Vec<(&'static str, &'static str)>,
    setup_time_ns: u128,
}

impl Demo1Level2Symmetry {
    pub fn new() -> Self {
        Self {
            smiles_targets: vec![
                ("Benzene", "C1=CC=CC=C1"),
                ("Coronene", "C1=CC2=C3C4=C1C=CC5=C4C6=C(C=C5)C=CC7=C6C3=C(C=C2)C=C7"),
            ],
            setup_time_ns: 0,
        }
    }
}

impl ScientificExperiment for Demo1Level2Symmetry {
    fn setup(&mut self) {
        self.setup_time_ns = Instant::now().elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start_global = Instant::now();
        let mut all_passed = true;

        for (name, smiles) in &self.smiles_targets {
            let base_complex = SmilesParser::parse_to_complex(smiles);
            let mut expected: Option<Vec<[u64; 4]>> = None;
            let mut latencies_us = Vec::new();

            for _ in 0..50 {
                let perm = base_complex.generate_isomorphic_permutation();

                let t0 = Instant::now();
                let nodes = CellularGaloisCanonizer::canonize(&perm, perm.var_count);
                latencies_us.push(t0.elapsed().as_micros());

                let mut sigs: Vec<_> = nodes.into_iter().map(|n| n.signature).collect();
                sigs.sort_by(|a, b| a.0.cmp(&b.0));
                let current: Vec<[u64; 4]> = sigs.into_iter().map(|s| s.0).collect();

                match &expected {
                    None => expected = Some(current.clone()),
                    Some(exp) => if &current != exp { all_passed = false; break; }
                }
            }

            let mean = latencies_us.iter().sum::<u128>() as f64 / 50.0;
            println!("    [METRIC] {} (Automorphic Stress): Mean Latency = {:.2}us", name, mean);
        }
        (ExperimentOutcome::IsomorphismMatch(all_passed), 0, start_global.elapsed().as_nanos())
    }

    fn verify(&self, outcome: &ExperimentOutcome) -> bool { match outcome { ExperimentOutcome::IsomorphismMatch(res) => *res, _ => false } }
    fn get_base_telemetry(&self) -> TelemetryRecord { TelemetryRecord { domain: "Chem".to_string(), experiment_name: "Demo1_L2".to_string(), vertices: 100, edges: 0, density: 0.0, parse_time_ns: self.setup_time_ns, l1_shield_time_ns: 0, galois_engine_time_ns: 0, l1_rejection_rate: 0.0, threads_utilized: 1, peak_memory_mb: 0.0, isomorphism_verified: true, false_positives_detected: 0 } }
}

pub struct Demo1Level3Massive {
    csv_path: String,
    parsed: Vec<(String, MolecularComplex)>,
    setup_time_ns: u128,
    limit: usize,
}

impl Demo1Level3Massive {
    pub fn new(csv_path: &str, limit: usize) -> Self {
        Self { csv_path: csv_path.to_string(), parsed: vec![], setup_time_ns: 0, limit }
    }
}

impl ScientificExperiment for Demo1Level3Massive {
    fn setup(&mut self) {
        let start = Instant::now();
        let file = File::open(&self.csv_path).expect("CRITICAL: Missing massive dataset");
        let mut rdr = csv::Reader::from_reader(file);
        let mut seen = HashSet::new();

        for result in rdr.records().take(self.limit) {
            let s = result.unwrap().get(0).unwrap().replace("@", "");
            if !s.is_empty() && seen.insert(s.clone()) {
                if let Some(c) = SmilesParser::try_parse_to_complex(&s) {
                    self.parsed.push((s, c));
                }
            }
        }
        self.setup_time_ns = start.elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start_global = Instant::now();
        let counter = AtomicUsize::new(0);
        let total = self.parsed.len();

        // Collect O(V+E) metrics for thermodynamic plotting
        let metrics: Vec<(usize, u128, bool)> = self.parsed.par_iter().map(|(_, complex)| {
            let mut expected: Option<Vec<[u64; 4]>> = None;
            let mut invariant = true;
            let mut total_latency_us = 0;

            for _ in 0..5 {
                let perm = complex.generate_isomorphic_permutation();
                let t0 = Instant::now();
                let nodes = CellularGaloisCanonizer::canonize(&perm, perm.var_count);
                total_latency_us += t0.elapsed().as_micros();

                let mut sigs: Vec<_> = nodes.into_iter().map(|n| n.signature).collect();
                sigs.sort_by(|a, b| a.0.cmp(&b.0));

                // CORRECTED VARIABLE NAME: curr -> current
                let current: Vec<[u64; 4]> = sigs.into_iter().map(|s| s.0).collect();

                match &expected {
                    None => expected = Some(current.clone()),
                    Some(exp) => if &current != exp { invariant = false; break; }
                }
            }

            let c = counter.fetch_add(1, Ordering::Relaxed);
            if c > 0 && c % 5000 == 0 { println!("    [TRACE] Demo 1: Processed {}/{}", c, total); }

            let v_plus_e = complex.var_count + complex.clauses.len();
            let avg_latency = total_latency_us / 5;

            (v_plus_e, avg_latency, invariant)
        }).collect();

        // Ensure the results directory exists
        std::fs::create_dir_all("data/chemistry/results").unwrap_or_default();

        // Export O(V+E) data to CSV
        let export_path = "data/chemistry/results/telemetry_demo1_O_V_E.csv";
        let mut file = File::create(export_path).expect("Failed to create O(V+E) export file");
        writeln!(file, "v_plus_e,latency_us").unwrap();

        let mut total_invariance = true;
        for (v_e, lat, inv) in metrics {
            writeln!(file, "{},{}", v_e, lat).unwrap();
            if !inv { total_invariance = false; }
        }
        println!("    [METRIC] O(V+E) Complexity data exported to {}", export_path);

        (ExperimentOutcome::IsomorphismMatch(total_invariance), 0, start_global.elapsed().as_nanos())
    }

    fn verify(&self, outcome: &ExperimentOutcome) -> bool { match outcome { ExperimentOutcome::IsomorphismMatch(res) => *res, _ => false } }
    fn get_base_telemetry(&self) -> TelemetryRecord { TelemetryRecord { domain: "Chem".to_string(), experiment_name: "Demo1_L3".to_string(), vertices: self.parsed.len(), edges: 0, density: 0.0, parse_time_ns: self.setup_time_ns, l1_shield_time_ns: 0, galois_engine_time_ns: 0, l1_rejection_rate: 0.0, threads_utilized: rayon::current_num_threads(), peak_memory_mb: 0.0, isomorphism_verified: true, false_positives_detected: 0 } }
}
