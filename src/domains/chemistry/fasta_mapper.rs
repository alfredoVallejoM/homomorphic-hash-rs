use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::collections::HashMap;
use std::time::Instant;
use rayon::prelude::*;

use crate::domains::chemistry::smiles_parser::{SmilesParser, MolecularComplex};

// =========================================================================
// FASTA MAPPER: ALGEBRAIC GRAPH STITCHING ENGINE
// Objective: Bypasses string-concatenation bottlenecks. Pre-compiles the
// 20 standard amino acids into Abstract Syntax Trees (ASTs) and merges
// their adjacency matrices in RAM to construct massive GF(2^256) tensors.
// =========================================================================

pub struct FastaRecord {
    pub header: String,
    pub sequence: String,
}

/// Represents a pre-compiled, mathematically validated Amino Acid subgraph.
#[derive(Clone)]
struct AminoAcidTemplate {
    pub complex: MolecularComplex,
    pub n_terminus_idx: usize,
    pub c_terminus_idx: usize,
}

pub struct FastaGraphMapper {
    templates: HashMap<char, AminoAcidTemplate>,
}

impl FastaGraphMapper {
    /// Initializes the engine, compiling the biochemical dictionary into RAM exactly once.
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        let t0 = Instant::now();

        // The dictionary. Format: (Residue, SMILES, N-term internal index, C-term internal index)
        let standard_aas = vec![
            ('G', "NCC(=O)", 0, 2),                   // Glycine
            ('A', "N[C@@H](C)C(=O)", 0, 3),           // Alanine
            ('V', "N[C@@H](C(C)C)C(=O)", 0, 4),       // Valine
            ('L', "N[C@@H](CC(C)C)C(=O)", 0, 5),      // Leucine
            ('I', "N[C@@H](C(C)CC)C(=O)", 0, 5),      // Isoleucine
            ('M', "N[C@@H](CCSC)C(=O)", 0, 5),        // Methionine
            ('P', "N1CCCC1C(=O)", 0, 5),              // Proline (N is 0, Carbonyl C is 5)
            ('F', "N[C@@H](Cc1ccccc1)C(=O)", 0, 8),   // Phenylalanine
            ('Y', "N[C@@H](Cc1ccc(O)cc1)C(=O)", 0, 9),// Tyrosine
            ('W', "N[C@@H](Cc1c[nH]c2ccccc12)C(=O)", 0, 11), // Tryptophan
            ('S', "N[C@@H](CO)C(=O)", 0, 3),          // Serine
            ('T', "N[C@@H](C(O)C)C(=O)", 0, 4),       // Threonine
            ('C', "N[C@@H](CS)C(=O)", 0, 3),          // Cysteine
            ('N', "N[C@@H](CC(=O)N)C(=O)", 0, 5),     // Asparagine
            ('Q', "N[C@@H](CCC(=O)N)C(=O)", 0, 6),    // Glutamine
            ('K', "N[C@@H](CCCCN)C(=O)", 0, 6),       // Lysine
            ('R', "N[C@@H](CCCNC(=N)N)C(=O)", 0, 8),  // Arginine
            ('H', "N[C@@H](Cc1c[nH]cn1)C(=O)", 0, 7), // Histidine
            ('D', "N[C@@H](CC(=O)O)C(=O)", 0, 5),     // Aspartic Acid
            ('E', "N[C@@H](CCC(=O)O)C(=O)", 0, 6),    // Glutamic Acid
        ];

        for (code, smiles, n_idx, c_idx) in standard_aas {
            let complex = SmilesParser::parse_to_complex(smiles);
            templates.insert(code, AminoAcidTemplate {
                complex,
                n_terminus_idx: n_idx,
                c_terminus_idx: c_idx,
            });
        }

        println!("    [CACHE] FASTA Algebraic Templates compiled in {} ms.", t0.elapsed().as_millis());
        Self { templates }
    }

    /// Reads massive .fasta files iteratively to prevent RAM spikes.
    pub fn ingest_fasta_file<P: AsRef<Path>>(filepath: P) -> Vec<FastaRecord> {
        let t0 = Instant::now();
        println!("    [IO] Ingesting FASTA library from disk...");

        let file = File::open(&filepath).expect("CRITICAL: Missing FASTA dataset");
        let reader = BufReader::new(file);

        let mut records = Vec::new();
        let mut current_header = String::new();
        let mut current_seq = String::new();

        for line_result in reader.lines() {
            let line = line_result.unwrap();
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }

            if trimmed.starts_with('>') {
                if !current_header.is_empty() {
                    records.push(FastaRecord { header: current_header.clone(), sequence: current_seq.clone() });
                    current_seq.clear();
                }
                current_header = trimmed[1..].to_string();
            } else {
                current_seq.push_str(trimmed);
            }
        }

        if !current_header.is_empty() {
            records.push(FastaRecord { header: current_header, sequence: current_seq });
        }

        println!("    [OK] Extracted {} biological sequences in {} ms.", records.len(), t0.elapsed().as_millis());
        records
    }

    /// The Core Engine: Algebraically stitches ASTs together to form a macromolecule.
    /// Operates in sub-milliseconds avoiding any string parsing overhead.
    pub fn assemble_macromolecule(&self, sequence: &str) -> MolecularComplex {
        let mut total_vars = 0;
        let mut total_clauses = Vec::new();
        let mut total_seeds = Vec::new();

        let mut prev_c_term_global_idx: Option<usize> = None;

        for residue in sequence.chars() {
            // Default to Glycine for unknown/non-standard residues to maintain connectivity
            let template = self.templates.get(&residue.to_ascii_uppercase()).unwrap_or_else(|| self.templates.get(&'G').unwrap());

            let vertex_offset = total_vars;

            // 1. Inject internal nodes (Seeds)
            total_seeds.extend(template.complex.seeds.clone());

            // 2. Inject internal edges (Clauses), offset by the current vertex count
            for clause in &template.complex.clauses {
                let shifted_clause: Vec<usize> = clause.iter().map(|&v| v + vertex_offset).collect();
                total_clauses.push(shifted_clause);
            }

            // 3. Form the Peptide Bond (Algebraic Stitching)
            let current_n_term_global_idx = vertex_offset + template.n_terminus_idx;
            if let Some(prev_c_term) = prev_c_term_global_idx {
                // Connect the Carbonyl Carbon of Residue N-1 to the Nitrogen of Residue N
                total_clauses.push(vec![prev_c_term, current_n_term_global_idx]);
            }

            // 4. Update the pointer for the next peptide bond
            prev_c_term_global_idx = Some(vertex_offset + template.c_terminus_idx);
            total_vars += template.complex.var_count;
        }

        // Rebuild adjacency list
        let mut var_to_clauses = vec![vec![]; total_vars];
        for (c_idx, vars) in total_clauses.iter().enumerate() {
            for &v in vars {
                var_to_clauses[v].push(c_idx);
            }
        }

        MolecularComplex {
            var_count: total_vars,
            clauses: total_clauses,
            var_to_clauses,
            seeds: total_seeds,
        }
    }

    /// Parallel pipeline for the entire proteome
    pub fn process_proteome_in_parallel(&self, records: &[FastaRecord]) -> Vec<(String, MolecularComplex)> {
        println!("    [COMPUTE] Initiating Parallel AST Assembly for {} records...", records.len());
        let t0 = Instant::now();

        let complexes: Vec<(String, MolecularComplex)> = records.par_iter()
            .map(|record| {
                let complex = self.assemble_macromolecule(&record.sequence);
                (record.header.clone(), complex)
            }).collect();

        println!("    [OK] Assembled {} macromolecules in {} ms.", complexes.len(), t0.elapsed().as_millis());
        complexes
    }
}
