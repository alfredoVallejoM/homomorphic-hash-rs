use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::time::Instant;

use crate::algebra::galois_256::GaloisSignature256;
use crate::domains::chemistry::smiles_parser::{MolecularComplex, SmilesParser};
use crate::engine::canonizer::CellularGaloisCanonizer;
use crate::harness::experiment::{ExperimentOutcome, ScientificExperiment};
use crate::harness::telemetry::TelemetryRecord;

// =========================================================================
// DEMONSTRATION 2: 1-WL DEFEAT (Isomer Resolution & Tensor Extraction)
// =========================================================================

/// Computes bitwise Hamming distance between two F256 topological tensors.
fn compute_algebraic_distance(tensor_a: &[[u64; 4]], tensor_b: &[[u64; 4]]) -> usize {
    let mut diff = 0;
    let len = tensor_a.len().min(tensor_b.len());
    for i in 0..len {
        for j in 0..4 {
            diff += (tensor_a[i][j] ^ tensor_b[i][j]).count_ones() as usize;
        }
    }
    diff += tensor_a.len().abs_diff(tensor_b.len()) * 256;
    diff
}

/// Serializes the raw mathematical tensor from RAM into a deterministic Hexadecimal string
/// so it can be exported and verified by external peers.
fn serialize_tensor_to_hex(tensor: &[[u64; 4]]) -> String {
    let mut hex_string = String::with_capacity(tensor.len() * 64);
    for row in tensor {
        hex_string.push_str(&format!(
            "{:016X}{:016X}{:016X}{:016X}",
            row[0], row[1], row[2], row[3]
        ));
    }
    hex_string
}

/// Extracts the leading and trailing 64 bits for quick visual inspection.
fn extract_head_tail(tensor: &[[u64; 4]]) -> (String, String) {
    if tensor.is_empty() {
        return (String::new(), String::new());
    }
    let head = format!("{:016X}", tensor[0][0]);
    let tail = format!("{:016X}", tensor.last().unwrap()[3]);
    (head, tail)
}

fn ensure_results_dir() {
    std::fs::create_dir_all("data/chemistry/results").unwrap_or_default();
}

// =========================================================================

pub struct Demo2Level1Alkanes {
    smiles_targets: Vec<&'static str>,
    setup_time_ns: u128,
}

impl Demo2Level1Alkanes {
    pub fn new() -> Self {
        Self {
            // The 5 structural isomers of Hexane
            smiles_targets: vec!["CCCCCC", "CCCC(C)C", "CCC(C)CC", "CCC(C)(C)C", "CC(C)C(C)C"],
            setup_time_ns: 0,
        }
    }
}

impl ScientificExperiment for Demo2Level1Alkanes {
    fn setup(&mut self) {
        self.setup_time_ns = Instant::now().elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start = Instant::now();
        let mut registry = HashSet::new();
        let mut ok = true;

        ensure_results_dir();
        let export_path = "data/chemistry/results/demo2_L1_alkanes_hashes.csv";
        let mut file = File::create(export_path).expect("Failed to create Alkane hash file");
        writeln!(
            file,
            "Isomer_Name,Head_Hash_64bit,Tail_Hash_64bit,Full_Algebraic_Signature"
        )
        .unwrap();

        for s in &self.smiles_targets {
            let c = SmilesParser::parse_to_complex(s);
            let nodes = CellularGaloisCanonizer::canonize(&c, c.var_count);
            let mut sigs: Vec<_> = nodes.into_iter().map(|n| n.signature).collect();
            sigs.sort_by(|a, b| a.0.cmp(&b.0));
            let t: Vec<[u64; 4]> = sigs.into_iter().map(|sig| sig.0).collect();

            let (head, tail) = extract_head_tail(&t);
            let full_hex = serialize_tensor_to_hex(&t);
            writeln!(file, "{},{},{},{}", s, head, tail, full_hex).unwrap();

            if !registry.insert(t) {
                println!("    [ERROR] Collision detected in Alkane Isomers!");
                ok = false;
            }
        }
        println!(
            "    [METRIC] Hexane Isomer hashes exported to: {}",
            export_path
        );

        (
            ExperimentOutcome::IsomorphismMatch(ok),
            0,
            start.elapsed().as_nanos(),
        )
    }
    fn verify(&self, outcome: &ExperimentOutcome) -> bool {
        match outcome {
            ExperimentOutcome::IsomorphismMatch(res) => *res,
            _ => false,
        }
    }
    fn get_base_telemetry(&self) -> TelemetryRecord {
        TelemetryRecord {
            domain: "Chem".to_string(),
            experiment_name: "Demo2_L1".to_string(),
            vertices: 5,
            edges: 0,
            density: 0.0,
            parse_time_ns: self.setup_time_ns,
            l1_shield_time_ns: 0,
            galois_engine_time_ns: 0,
            l1_rejection_rate: 0.0,
            threads_utilized: 1,
            peak_memory_mb: 0.0,
            isomorphism_verified: true,
            false_positives_detected: 0,
        }
    }
}

// =========================================================================

pub struct Demo2Level2Aromatic {
    smiles_targets: Vec<&'static str>,
    setup_time_ns: u128,
}

impl Demo2Level2Aromatic {
    pub fn new() -> Self {
        Self {
            // Ortho, Meta, Para substitution
            smiles_targets: vec!["CC1=C(C)C=CC=C1", "CC1=CC(C)=CC=C1", "CC1=CC=C(C)C=C1"],
            setup_time_ns: 0,
        }
    }
}

impl ScientificExperiment for Demo2Level2Aromatic {
    fn setup(&mut self) {
        self.setup_time_ns = Instant::now().elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start = Instant::now();
        let mut registry: HashMap<String, Vec<[u64; 4]>> = HashMap::new();
        let mut ok = true;

        ensure_results_dir();
        let hashes_path = "data/chemistry/results/demo2_L2_aromatic_hashes.csv";
        let mut hashes_file =
            File::create(hashes_path).expect("Failed to create Aromatic hash file");
        writeln!(hashes_file, "Isomer_Name,Full_Algebraic_Signature").unwrap();

        for s in &self.smiles_targets {
            let c = SmilesParser::parse_to_complex(s);
            let nodes = CellularGaloisCanonizer::canonize(&c, c.var_count);
            let mut sigs: Vec<_> = nodes.into_iter().map(|n| n.signature).collect();
            sigs.sort_by(|a, b| a.0.cmp(&b.0));
            let t: Vec<[u64; 4]> = sigs.into_iter().map(|sig| sig.0).collect();

            writeln!(hashes_file, "{},{}", s, serialize_tensor_to_hex(&t)).unwrap();

            let unique_sig: HashSet<Vec<[u64; 4]>> = registry.values().cloned().collect();
            if unique_sig.contains(&t) {
                ok = false;
            }
            registry.insert(s.to_string(), t);
        }

        let dist_path = "data/chemistry/results/demo2_L2_aromatic_hamming_distances.csv";
        let mut dist_file =
            File::create(dist_path).expect("Failed to create Hamming distance file");
        writeln!(dist_file, "Isomer_A,Isomer_B,Hamming_Distance_Bits").unwrap();

        let keys: Vec<String> = registry.keys().cloned().collect();
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                let dist = compute_algebraic_distance(&registry[&keys[i]], &registry[&keys[j]]);
                writeln!(dist_file, "{},{},{}", keys[i], keys[j], dist).unwrap();
            }
        }
        println!("    [METRIC] Aromatic distances & hashes exported to data/chemistry/results/");

        (
            ExperimentOutcome::IsomorphismMatch(ok),
            0,
            start.elapsed().as_nanos(),
        )
    }
    fn verify(&self, outcome: &ExperimentOutcome) -> bool {
        match outcome {
            ExperimentOutcome::IsomorphismMatch(res) => *res,
            _ => false,
        }
    }
    fn get_base_telemetry(&self) -> TelemetryRecord {
        TelemetryRecord {
            domain: "Chem".to_string(),
            experiment_name: "Demo2_L2".to_string(),
            vertices: 3,
            edges: 0,
            density: 0.0,
            parse_time_ns: self.setup_time_ns,
            l1_shield_time_ns: 0,
            galois_engine_time_ns: 0,
            l1_rejection_rate: 0.0,
            threads_utilized: 1,
            peak_memory_mb: 0.0,
            isomorphism_verified: true,
            false_positives_detected: 0,
        }
    }
}

// =========================================================================

pub struct Demo2Level3WLParadox {
    smiles_targets: Vec<&'static str>,
    setup_time_ns: u128,
}

impl Demo2Level3WLParadox {
    pub fn new() -> Self {
        Self {
            // Decalin vs Bicyclopentyl
            smiles_targets: vec!["C1CCC2CCCCC2C1", "C1CCC(C1)C2CCCC2"],
            setup_time_ns: 0,
        }
    }
}

impl ScientificExperiment for Demo2Level3WLParadox {
    fn setup(&mut self) {
        self.setup_time_ns = Instant::now().elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start = Instant::now();
        let mut registry = HashSet::new();
        let mut ok = true;

        ensure_results_dir();
        let export_path = "data/chemistry/results/demo2_L3_WL_paradox_hashes.csv";
        let mut file = File::create(export_path).expect("Failed to create WL Paradox hash file");
        writeln!(
            file,
            "Molecule_Name,Head_Hash_64bit,Tail_Hash_64bit,Full_Algebraic_Signature"
        )
        .unwrap();

        for s in &self.smiles_targets {
            let c = SmilesParser::parse_to_complex(s);
            let nodes = CellularGaloisCanonizer::canonize(&c, c.var_count);
            let mut sigs: Vec<_> = nodes.into_iter().map(|n| n.signature).collect();
            sigs.sort_by(|a, b| a.0.cmp(&b.0));
            let t: Vec<[u64; 4]> = sigs.into_iter().map(|sig| sig.0).collect();

            let (head, tail) = extract_head_tail(&t);
            let full_hex = serialize_tensor_to_hex(&t);
            writeln!(file, "{},{},{},{}", s, head, tail, full_hex).unwrap();

            if !registry.insert(t) {
                ok = false;
            }
        }
        println!(
            "    [METRIC] 1-WL Paradox hashes exported to: {}",
            export_path
        );

        (
            ExperimentOutcome::IsomorphismMatch(ok),
            0,
            start.elapsed().as_nanos(),
        )
    }
    fn verify(&self, outcome: &ExperimentOutcome) -> bool {
        match outcome {
            ExperimentOutcome::IsomorphismMatch(res) => *res,
            _ => false,
        }
    }
    fn get_base_telemetry(&self) -> TelemetryRecord {
        TelemetryRecord {
            domain: "Chem".to_string(),
            experiment_name: "Demo2_L3".to_string(),
            vertices: 2,
            edges: 0,
            density: 0.0,
            parse_time_ns: self.setup_time_ns,
            l1_shield_time_ns: 0,
            galois_engine_time_ns: 0,
            l1_rejection_rate: 0.0,
            threads_utilized: 1,
            peak_memory_mb: 0.0,
            isomorphism_verified: true,
            false_positives_detected: 0,
        }
    }
}

// =========================================================================

pub struct Demo2Level4MassivePubChem {
    csv_path: String,
    parsed_graphs: Vec<(String, MolecularComplex)>,
    setup_time_ns: u128,
}

impl Demo2Level4MassivePubChem {
    pub fn new(csv_path: &str) -> Self {
        Self {
            csv_path: csv_path.to_string(),
            parsed_graphs: Vec::new(),
            setup_time_ns: 0,
        }
    }
}

impl ScientificExperiment for Demo2Level4MassivePubChem {
    fn setup(&mut self) {
        let start = Instant::now();
        let file = File::open(&self.csv_path).expect("CRITICAL: Missing isomers file");
        let mut rdr = csv::Reader::from_reader(file);
        let mut seen_2d_topologies = std::collections::HashSet::new();

        for result in rdr.records() {
            if let Ok(record) = result {
                let s = record.get(0).unwrap_or("").trim();
                let clean = s.replace("@@", "").replace("@", "");
                if !clean.is_empty() && seen_2d_topologies.insert(clean.clone()) {
                    if let Some(complex) = SmilesParser::try_parse_to_complex(&clean) {
                        self.parsed_graphs.push((clean, complex));
                    }
                }
            }
        }
        self.setup_time_ns = start.elapsed().as_nanos();
    }

    fn execute(&self) -> (ExperimentOutcome, u128, u128) {
        let start_engine = Instant::now();

        let topologies: Vec<(String, Vec<[u64; 4]>)> = self
            .parsed_graphs
            .par_iter()
            .map(|(smiles, graph)| {
                let nodes = CellularGaloisCanonizer::canonize(graph, graph.var_count);
                let mut sigs: Vec<GaloisSignature256> =
                    nodes.into_iter().map(|n| n.signature).collect();
                sigs.sort_by(|a, b| a.0.cmp(&b.0));
                (smiles.clone(), sigs.into_iter().map(|s| s.0).collect())
            })
            .collect();

        ensure_results_dir();
        let export_path = "data/chemistry/results/demo2_L4_massive_hashes.csv";
        // We write to a thread-local string first, or aggregate sequentially to avoid parallel file lock overhead
        let mut file =
            File::create(export_path).expect("Failed to create Massive Isomer hash file");
        writeln!(file, "Isomer_Name,Full_Algebraic_Signature").unwrap();

        let mut registry = HashMap::new();
        let mut no_collisions = true;
        for (smiles, topology) in topologies {
            writeln!(file, "{},{}", smiles, serialize_tensor_to_hex(&topology)).unwrap();

            if let Some(existing) = registry.insert(topology, smiles.clone()) {
                println!(
                    "🚨 [COLLISION] True 1-WL failure between: {} and {}",
                    existing, smiles
                );
                no_collisions = false;
            }
        }
        println!(
            "    [METRIC] Massive 1-WL Defeat dataset ({} hashes) exported to: {}",
            registry.len(),
            export_path
        );

        (
            ExperimentOutcome::IsomorphismMatch(no_collisions),
            0,
            start_engine.elapsed().as_nanos(),
        )
    }

    fn verify(&self, outcome: &ExperimentOutcome) -> bool {
        match outcome {
            ExperimentOutcome::IsomorphismMatch(res) => *res,
            _ => false,
        }
    }
    fn get_base_telemetry(&self) -> TelemetryRecord {
        TelemetryRecord {
            domain: "Chem".to_string(),
            experiment_name: "Demo2_L4_Massive".to_string(),
            vertices: self.parsed_graphs.len(),
            edges: 0,
            density: 0.0,
            parse_time_ns: self.setup_time_ns,
            l1_shield_time_ns: 0,
            galois_engine_time_ns: 0,
            l1_rejection_rate: 0.0,
            threads_utilized: rayon::current_num_threads(),
            peak_memory_mb: 0.0,
            isomorphism_verified: true,
            false_positives_detected: 0,
        }
    }
}
