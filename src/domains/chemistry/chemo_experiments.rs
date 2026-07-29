use std::time::Instant;
use std::collections::HashSet;
use std::fs::File;
use rayon::prelude::*;

use crate::algebra::galois_256::GaloisSignature256;
use crate::harness::experiment::{ScientificExperiment, ExperimentOutcome};
use crate::harness::telemetry::TelemetryRecord;
use crate::domains::chemistry::smiles_parser::{SmilesParser, MolecularComplex};
use crate::engine::canonizer::CellularGaloisCanonizer;

// =========================================================================
// DEMONSTRATION 1: UNIVERSAL INVARIANCE (The SMILES Shuffle)
// =========================================================================

/// LEVEL 1 (TRIVIAL): Positional Isomerism.
/// Target: Common molecules like Ibuprofen and Aspirin.
pub struct Demo1Level1PositionalIsomerism {
    smiles_targets: Vec<(&'static str, &'static str)>,
    setup_time_ns: u128,
}

impl Demo1Level1PositionalIsomerism {
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

impl ScientificExperiment for Demo1Level1PositionalIsomerism {
    fn setup(&mut self) {
        self.setup_time_ns = Instant::now().elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start_engine = Instant::now();
        let mut all_passed = true;

        for (_name, smiles) in &self.smiles_targets {
            let base_complex = SmilesParser::parse_to_complex(smiles);

            let mut expected_topology: Option<Vec<[u64; 4]>> = None;

            for _ in 0..50 {
                let perm = base_complex.generate_isomorphic_permutation();
                let nodes = CellularGaloisCanonizer::canonize(&perm, perm.var_count);
                let mut sigs: Vec<GaloisSignature256> = nodes.into_iter().map(|n| n.signature).collect();
                sigs.sort_by(|a, b| a.0.cmp(&b.0));

                let current_topology: Vec<[u64; 4]> = sigs.into_iter().map(|s| s.0).collect();

                match &expected_topology {
                    None => expected_topology = Some(current_topology),
                    Some(expected) => {
                        if &current_topology != expected {
                            all_passed = false;
                        }
                    }
                }
            }
        }
        (ExperimentOutcome::IsomorphismMatch(all_passed), 0, start_engine.elapsed().as_nanos())
    }

    fn verify(&self, outcome: &ExperimentOutcome) -> bool {
        match outcome { ExperimentOutcome::IsomorphismMatch(res) => *res, _ => false }
    }

    fn get_base_telemetry(&self) -> TelemetryRecord {
        TelemetryRecord { domain: "Chemistry".to_string(), experiment_name: "Demo1_L1_Positional".to_string(), vertices: 100, edges: 0, density: 0.0, parse_time_ns: self.setup_time_ns, l1_shield_time_ns: 0, galois_engine_time_ns: 0, l1_rejection_rate: 0.0, threads_utilized: 1, peak_memory_mb: 0.0, isomorphism_verified: true, false_positives_detected: 0 }
    }
}

/// LEVEL 2 (SYMMETRIC STRESS): Automorphic nightmare.
/// Target: Highly symmetrical molecules.
/// NOTE: Coronene ("Superbenzene") is used to test dense cyclic symmetry without parser constraints.
pub struct Demo1Level2SymmetricStress {
    smiles_targets: Vec<(&'static str, &'static str)>,
    setup_time_ns: u128,
}

impl Demo1Level2SymmetricStress {
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

impl ScientificExperiment for Demo1Level2SymmetricStress {
    fn setup(&mut self) {
        self.setup_time_ns = Instant::now().elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start_engine = Instant::now();
        let mut all_passed = true;

        for (_name, smiles) in &self.smiles_targets {
            let base_complex = SmilesParser::parse_to_complex(smiles);
            let mut expected_topology: Option<Vec<[u64; 4]>> = None;

            for _ in 0..50 {
                let perm = base_complex.generate_isomorphic_permutation();
                let nodes = CellularGaloisCanonizer::canonize(&perm, perm.var_count);
                let mut sigs: Vec<GaloisSignature256> = nodes.into_iter().map(|n| n.signature).collect();
                sigs.sort_by(|a, b| a.0.cmp(&b.0));

                let current_topology: Vec<[u64; 4]> = sigs.into_iter().map(|s| s.0).collect();

                match &expected_topology {
                    None => expected_topology = Some(current_topology),
                    Some(expected) => {
                        if &current_topology != expected {
                            all_passed = false;
                        }
                    }
                }
            }
        }
        (ExperimentOutcome::IsomorphismMatch(all_passed), 0, start_engine.elapsed().as_nanos())
    }

    fn verify(&self, outcome: &ExperimentOutcome) -> bool {
        match outcome { ExperimentOutcome::IsomorphismMatch(res) => *res, _ => false }
    }

    fn get_base_telemetry(&self) -> TelemetryRecord {
        TelemetryRecord { domain: "Chemistry".to_string(), experiment_name: "Demo1_L2_Symmetry".to_string(), vertices: 100, edges: 0, density: 0.0, parse_time_ns: self.setup_time_ns, l1_shield_time_ns: 0, galois_engine_time_ns: 0, l1_rejection_rate: 0.0, threads_utilized: 1, peak_memory_mb: 0.0, isomorphism_verified: true, false_positives_detected: 0 }
    }
}

/// LEVEL 3 (CHEMBL SCALE): Statistical Brute Force.
/// Target: Massive chemical space evaluation to prove 0% collision rate.
pub struct Demo1Level3ChemblScale {
    csv_path: String,
    parsed_base_graphs: Vec<MolecularComplex>,
    setup_time_ns: u128,
}

impl Demo1Level3ChemblScale {
    pub fn new(csv_path: &str) -> Self {
        Self {
            csv_path: csv_path.to_string(),
            parsed_base_graphs: Vec::new(),
            setup_time_ns: 0
        }
    }
}

impl ScientificExperiment for Demo1Level3ChemblScale {
    fn setup(&mut self) {
        let start = Instant::now();
        let file = File::open(&self.csv_path).expect("CRITICAL: Missing Level 3 dataset");
        let mut rdr = csv::Reader::from_reader(file);

        for result in rdr.records().take(100_000) {
            if let Ok(record) = result {
                let s = if record.len() > 1 { record.get(1).unwrap_or("").trim() } else { record.get(0).unwrap_or("").trim() };
                if !s.is_empty() && s != "smiles" {
                    if let Some(complex) = SmilesParser::try_parse_to_complex(s) {
                        self.parsed_base_graphs.push(complex);
                    }
                }
            }
        }
        self.setup_time_ns = start.elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start_engine = Instant::now();

        let results: Vec<(bool, Vec<[u64; 4]>)> = self.parsed_base_graphs.par_iter().map(|base_complex| {
            let mut expected_topology: Option<Vec<[u64; 4]>> = None;
            let mut invariant = true;

            for _ in 0..5 {
                let perm = base_complex.generate_isomorphic_permutation();
                let nodes = CellularGaloisCanonizer::canonize(&perm, perm.var_count);
                let mut sigs: Vec<GaloisSignature256> = nodes.into_iter().map(|n| n.signature).collect();
                sigs.sort_by(|a, b| a.0.cmp(&b.0));

                let current_topology: Vec<[u64; 4]> = sigs.into_iter().map(|s| s.0).collect();

                match &expected_topology {
                    None => expected_topology = Some(current_topology),
                    Some(expected) => {
                        if &current_topology != expected {
                            invariant = false;
                        }
                    }
                }
            }
            (invariant, expected_topology.unwrap())
        }).collect();

        let zero_false_negatives = results.iter().all(|(invariant, _)| *invariant);

        let mut unique_registry: HashSet<&Vec<[u64; 4]>> = HashSet::new();
        for (_, topology) in &results {
            unique_registry.insert(topology);
        }
        let zero_collisions = unique_registry.len() == self.parsed_base_graphs.len();

        (ExperimentOutcome::IsomorphismMatch(zero_false_negatives && zero_collisions), 0, start_engine.elapsed().as_nanos())
    }

    fn verify(&self, outcome: &ExperimentOutcome) -> bool {
        match outcome { ExperimentOutcome::IsomorphismMatch(res) => *res, _ => false }
    }

    fn get_base_telemetry(&self) -> TelemetryRecord {
        TelemetryRecord { domain: "Chemistry".to_string(), experiment_name: "Demo1_L3_Massive".to_string(), vertices: self.parsed_base_graphs.len() * 5, edges: 0, density: 0.0, parse_time_ns: self.setup_time_ns, l1_shield_time_ns: 0, galois_engine_time_ns: 0, l1_rejection_rate: 0.0, threads_utilized: rayon::current_num_threads(), peak_memory_mb: 0.0, isomorphism_verified: true, false_positives_detected: 0 }
    }
}

// =========================================================================
// DEMONSTRATION 2: 1-WL DEFEAT (ISOMER RESOLUTION)
// =========================================================================

/// LEVEL 1: Massive Alkanes.
/// Proves that the algorithm can distinguish thousands of structural isomers.
pub struct Demo2Level1MassiveAlkanes {
    csv_path: String,
    parsed_graphs: Vec<MolecularComplex>,
    setup_time_ns: u128,
}

impl Demo2Level1MassiveAlkanes {
    pub fn new(csv_path: &str) -> Self {
        Self {
            csv_path: csv_path.to_string(),
            parsed_graphs: Vec::new(),
            setup_time_ns: 0
        }
    }
}

impl ScientificExperiment for Demo2Level1MassiveAlkanes {
    fn setup(&mut self) {
        let start = Instant::now();
        let file = File::open(&self.csv_path).expect("CRITICAL: Missing Level 1 Isomer dataset");
        let mut rdr = csv::Reader::from_reader(file);

        for result in rdr.records() {
            if let Ok(record) = result {
                let s = record.get(0).unwrap_or("").trim();
                if !s.is_empty() && s != "smiles" {
                    if let Some(complex) = SmilesParser::try_parse_to_complex(s) {
                        self.parsed_graphs.push(complex);
                    }
                }
            }
        }
        self.setup_time_ns = start.elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start_engine = Instant::now();

        let generated_topologies: Vec<Vec<[u64; 4]>> = self.parsed_graphs.par_iter().map(|graph| {
            let nodes = CellularGaloisCanonizer::canonize(graph, graph.var_count);
            let mut sigs: Vec<GaloisSignature256> = nodes.into_iter().map(|n| n.signature).collect();
            sigs.sort_by(|a, b| a.0.cmp(&b.0));

            sigs.into_iter().map(|s| s.0).collect()
        }).collect();

        let mut unique_registry: HashSet<Vec<[u64; 4]>> = HashSet::new();
        let mut no_collisions = true;

        for topology in generated_topologies {
            if !unique_registry.insert(topology) {
                no_collisions = false;
                break;
            }
        }

        (ExperimentOutcome::IsomorphismMatch(no_collisions), 0, start_engine.elapsed().as_nanos())
    }

    fn verify(&self, outcome: &ExperimentOutcome) -> bool {
        match outcome { ExperimentOutcome::IsomorphismMatch(res) => *res, _ => false }
    }

    fn get_base_telemetry(&self) -> TelemetryRecord {
        TelemetryRecord { domain: "Chemistry".to_string(), experiment_name: "Demo2_L1_Alkanes".to_string(), vertices: self.parsed_graphs.len(), edges: 0, density: 0.0, parse_time_ns: self.setup_time_ns, l1_shield_time_ns: 0, galois_engine_time_ns: 0, l1_rejection_rate: 0.0, threads_utilized: rayon::current_num_threads(), peak_memory_mb: 0.0, isomorphism_verified: true, false_positives_detected: 0 }
    }
}

/// LEVEL 2: Aromatic Substitution.
/// Verifies precise positional resolution within rings (Ortho, Meta, Para).
pub struct Demo2Level2AromaticSubstitution {
    smiles_targets: Vec<(&'static str, &'static str)>,
    setup_time_ns: u128,
}

impl Demo2Level2AromaticSubstitution {
    pub fn new() -> Self {
        Self {
            smiles_targets: vec![
                ("Ortho-Xylene", "CC1=C(C)C=CC=C1"),
                ("Meta-Xylene", "CC1=CC(C)=CC=C1"),
                ("Para-Xylene", "CC1=CC=C(C)C=C1"),
            ],
            setup_time_ns: 0,
        }
    }
}

impl ScientificExperiment for Demo2Level2AromaticSubstitution {
    fn setup(&mut self) {
        self.setup_time_ns = Instant::now().elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start_engine = Instant::now();
        let mut unique_registry: HashSet<Vec<[u64; 4]>> = HashSet::new();
        let mut no_collisions = true;

        for (_name, smiles) in &self.smiles_targets {
            let graph = SmilesParser::parse_to_complex(smiles);
            let nodes = CellularGaloisCanonizer::canonize(&graph, graph.var_count);
            let mut sigs: Vec<GaloisSignature256> = nodes.into_iter().map(|n| n.signature).collect();
            sigs.sort_by(|a, b| a.0.cmp(&b.0));

            let topology: Vec<[u64; 4]> = sigs.into_iter().map(|s| s.0).collect();
            if !unique_registry.insert(topology) {
                no_collisions = false;
            }
        }
        (ExperimentOutcome::IsomorphismMatch(no_collisions), 0, start_engine.elapsed().as_nanos())
    }

    fn verify(&self, outcome: &ExperimentOutcome) -> bool {
        match outcome { ExperimentOutcome::IsomorphismMatch(res) => *res, _ => false }
    }

    fn get_base_telemetry(&self) -> TelemetryRecord {
        TelemetryRecord { domain: "Chemistry".to_string(), experiment_name: "Demo2_L2_Xylene".to_string(), vertices: 3, edges: 0, density: 0.0, parse_time_ns: self.setup_time_ns, l1_shield_time_ns: 0, galois_engine_time_ns: 0, l1_rejection_rate: 0.0, threads_utilized: 1, peak_memory_mb: 0.0, isomorphism_verified: true, false_positives_detected: 0 }
    }
}

/// LEVEL 3: Weisfeiler-Lehman Paradoxes.
/// Proves that the F251 Spectral Engine bypasses standard 1-WL blind spots using
/// Decalin vs Bicyclopentyl, which share identical 1-WL topologies.
pub struct Demo2Level3WLParadoxes {
    smiles_targets: Vec<(&'static str, &'static str)>,
    setup_time_ns: u128,
}

impl Demo2Level3WLParadoxes {
    pub fn new() -> Self {
        Self {
            smiles_targets: vec![
                ("Decalin", "C1CCC2CCCCC2C1"),
                ("Bicyclopentyl", "C1CCC(C1)C2CCCC2"),
            ],
            setup_time_ns: 0,
        }
    }
}

impl ScientificExperiment for Demo2Level3WLParadoxes {
    fn setup(&mut self) {
        self.setup_time_ns = Instant::now().elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start_engine = Instant::now();
        let mut unique_registry: HashSet<Vec<[u64; 4]>> = HashSet::new();
        let mut no_collisions = true;

        for (_name, smiles) in &self.smiles_targets {
            let graph = SmilesParser::parse_to_complex(smiles);
            let nodes = CellularGaloisCanonizer::canonize(&graph, graph.var_count);
            let mut sigs: Vec<GaloisSignature256> = nodes.into_iter().map(|n| n.signature).collect();
            sigs.sort_by(|a, b| a.0.cmp(&b.0));

            let topology: Vec<[u64; 4]> = sigs.into_iter().map(|s| s.0).collect();
            if !unique_registry.insert(topology) {
                no_collisions = false;
            }
        }
        (ExperimentOutcome::IsomorphismMatch(no_collisions), 0, start_engine.elapsed().as_nanos())
    }

    fn verify(&self, outcome: &ExperimentOutcome) -> bool {
        match outcome { ExperimentOutcome::IsomorphismMatch(res) => *res, _ => false }
    }

    fn get_base_telemetry(&self) -> TelemetryRecord {
        TelemetryRecord { domain: "Chemistry".to_string(), experiment_name: "Demo2_L3_1WL_Paradox".to_string(), vertices: 2, edges: 0, density: 0.0, parse_time_ns: self.setup_time_ns, l1_shield_time_ns: 0, galois_engine_time_ns: 0, l1_rejection_rate: 0.0, threads_utilized: 1, peak_memory_mb: 0.0, isomorphism_verified: true, false_positives_detected: 0 }
    }
}
