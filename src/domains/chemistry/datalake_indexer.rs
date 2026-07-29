use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, BufReader};
use std::path::Path;
use std::time::Instant;
use rayon::prelude::*;
use serde::{Serialize, Deserialize};

use crate::algebra::galois_256::GaloisSignature256;
use crate::engine::canonizer::{CellularGaloisCanonizer, CanonicalNode};
use crate::domains::chemistry::fasta_mapper::{FastaGraphMapper, FastaRecord};
use crate::domains::chemistry::pdb_mapper::PdbGraphMapper;
use crate::domains::chemistry::smiles_parser::MolecularComplex;

// =========================================================================
// DATALAKE INDEXER: STRATIFIED BINARY CACHE ENGINE
// Objective: Ingest raw complexes, execute the O(V+E) Galois Canonization
// once, and persist the 256-bit signatures into Isosteric Buckets (V, E)
// using ultra-fast binary serialization (bincode). This annihilates the
// N^2 cross-comparison bottleneck by isolating mathematical equivalence classes.
// =========================================================================

/// The unique macro-state key for topological bucketing.
/// Two graphs can ONLY be isomorphic if they share the exact same V, E, and Betti number.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsostericKey {
    pub v_count: usize,
    pub e_count: usize,
    pub betti_number: usize,
}

/// The atomic unit of our stored cache.
#[derive(Serialize, Deserialize)]
pub struct CachedMacromolecule {
    pub header_id: String,
    pub canonical_nodes: Vec<CanonicalNode>,
}

/// The Stratified Database format that will be mapped into RAM (Zero-Copy ready)
#[derive(Serialize, Deserialize)]
pub struct StratifiedDataLake {
    pub source_name: String,
    pub total_records: usize,
    pub buckets: HashMap<IsostericKey, Vec<CachedMacromolecule>>,
}

pub struct DataLakeIndexer;

impl DataLakeIndexer {
    /// Helper: Calculate Macro-Properties
    fn compute_key(complex: &MolecularComplex) -> IsostericKey {
        let v_count = complex.var_count;
        let e_count = complex.clauses.len();
        // Betti Number (Cyclomatic complexity): E - V + C (Assuming 1 connected component)
        let betti_number = e_count.saturating_sub(v_count).saturating_add(1);

        IsostericKey { v_count, e_count, betti_number }
    }

    /// PILELINE 1: FASTA/Proteome to Stratified Binary Cache
    /// Takes the massive AST-assembled complexes from the FastaMapper, canonizes them
    /// in parallel, and clusters them by (V, E, Betti).
    pub fn build_proteome_cache<P: AsRef<Path>>(
        output_bin_path: P,
        complexes: Vec<(String, MolecularComplex)>
    ) {
        println!("    [INDEXER] Initiating Massive Proteome Canonization and Stratification...");
        let t0 = Instant::now();
        let total = complexes.len();

        // 1. Parallel Canonization (Heavy Math happens here, leveraging all CPU cores)
        let canonized_entries: Vec<(IsostericKey, CachedMacromolecule)> = complexes.into_par_iter()
            .map(|(header, complex)| {
                let key = Self::compute_key(&complex);
                let nodes = CellularGaloisCanonizer::canonize(&complex, complex.var_count);
                let record = CachedMacromolecule { header_id: header, canonical_nodes: nodes };
                (key, record)
            })
            .collect();

        // 2. Sequential Bucketing (Fast memory pointer reorganization)
        let mut lake = StratifiedDataLake {
            source_name: "UniProt_SwissProt".to_string(),
            total_records: total,
            buckets: HashMap::new(),
        };

        for (key, record) in canonized_entries {
            lake.buckets.entry(key).or_insert_with(Vec::new).push(record);
        }

        // 3. Binary Serialization to Disk (Bincode)
        let file = File::create(&output_bin_path).expect("CRITICAL: Failed to create binary cache file");
        let mut writer = BufWriter::new(file);
        bincode::serialize_into(&mut writer, &lake).expect("CRITICAL: Bincode serialization failed");

        let elapsed = t0.elapsed().as_secs();
        println!("    [OK] Indexing Complete. Stratified {} macromolecules into {} Isosteric Buckets.", total, lake.buckets.len());
        println!("    [IO] Binary lake persisted to {:?} in {} seconds.", output_bin_path.as_ref(), elapsed);
    }

    /// PIPELINE 2: PDB/3D Structures to Stratified Binary Cache
    /// Consumes physical coordinate files, maps their adjacency via Cell-Lists,
    /// and persists them to evaluate Matrix Bifurcation dynamically.
    pub fn build_structural_cache<P: AsRef<Path>>(
        output_bin_path: P,
        pdb_file_paths: Vec<String>
    ) {
        println!("    [INDEXER] Initiating Structural 3D (PDB) Canonization...");
        let t0 = Instant::now();
        let total_files = pdb_file_paths.len();

        let canonized_entries: Vec<(IsostericKey, CachedMacromolecule)> = pdb_file_paths.into_par_iter()
            .filter_map(|path| {
                // Read PDB and infer dense graph via Spatial Hashing
                if let Some(complex) = PdbGraphMapper::parse_pdb_to_complex(&path) {
                    let key = Self::compute_key(&complex);
                    let nodes = CellularGaloisCanonizer::canonize(&complex, complex.var_count);
                    let record = CachedMacromolecule { header_id: path, canonical_nodes: nodes };
                    Some((key, record))
                } else {
                    None
                }
            })
            .collect();

        let mut lake = StratifiedDataLake {
            source_name: "RCSB_PDB_Structural".to_string(),
            total_records: canonized_entries.len(),
            buckets: HashMap::new(),
        };

        for (key, record) in canonized_entries {
            lake.buckets.entry(key).or_insert_with(Vec::new).push(record);
        }

        let file = File::create(&output_bin_path).expect("CRITICAL: Failed to create binary cache file");
        let mut writer = BufWriter::new(file);
        bincode::serialize_into(&mut writer, &lake).expect("CRITICAL: Bincode serialization failed");

        println!("    [OK] 3D Structural Indexing Complete. Processed {}/{} valid PDBs in {} seconds.",
                 lake.total_records, total_files, t0.elapsed().as_secs());
    }

    /// ZERO-COPY LOADING: Maps the binary lake directly into RAM for instant Demo Execution
    pub fn load_stratified_lake<P: AsRef<Path>>(bin_path: P) -> StratifiedDataLake {
        let t0 = Instant::now();
        println!("    [IO] Hydrating Stratified Data Lake from disk...");

        let file = File::open(bin_path).expect("CRITICAL: Binary Cache not found. Run Indexer first.");
        let reader = BufReader::new(file);

        // Deserializing via bincode is orders of magnitude faster than parsing strings
        let lake: StratifiedDataLake = bincode::deserialize_from(reader)
            .expect("CRITICAL: Corrupt binary cache. Needs re-indexing.");

        println!("    [OK] Lake Hydrated. Loaded {} records across {} Isosteric Buckets in {} ms.",
                 lake.total_records, lake.buckets.len(), t0.elapsed().as_millis());

        lake
    }
}
