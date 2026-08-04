use homomorphic_hash_rs::algebra::galois_256::GaloisSignature256;
use homomorphic_hash_rs::domains::chemistry::smiles_parser::SmilesParser;
use homomorphic_hash_rs::engine::canonizer::CellularGaloisCanonizer;
use std::collections::HashSet;

// =========================================================================
// CORE DIAGNOSTIC HELPERS
// =========================================================================

/// Computes the exact 256-bit topological matrix for a given SMILES string.
/// If `permute` is true, it shuffles the memory layout before canonization.
fn extract_topology(smiles: &str, permute: bool) -> Vec<[u64; 4]> {
    let base_complex = SmilesParser::parse_to_complex(smiles);

    let target_graph = if permute {
        base_complex.generate_isomorphic_permutation()
    } else {
        base_complex
    };

    let nodes = CellularGaloisCanonizer::canonize(&target_graph, target_graph.var_count);

    let mut sigs: Vec<GaloisSignature256> = nodes.into_iter().map(|n| n.signature).collect();
    // Sort to guarantee canonical deterministic ordering
    sigs.sort_by_key(|signature| signature.0);

    // Extract the raw 256-bit arrays
    sigs.into_iter().map(|s| s.0).collect()
}

/// Visual helper: Dumps a Hex representation of the first and last block of the tensor.
/// NOTE: This is STRICTLY for terminal logging. The actual math uses the full Vec<[u64; 4]>.
fn format_tensor_fingerprint(topology: &[[u64; 4]]) -> String {
    if topology.is_empty() {
        return "EMPTY".to_string();
    }
    let first_node = topology[0];
    let last_node = topology[topology.len() - 1];
    // We print the highest 32 bits of the first u64 of the Head and Tail nodes
    format!(
        "Head:[{:08X}...] Tail:[{:08X}...]",
        first_node[0] >> 32,
        last_node[3] >> 32
    )
}

/// Asserts that a molecule yields the EXACT same topology across N permutations.
fn run_invariance_test(test_name: &str, target_smiles: &[&str], permutations: usize) -> bool {
    println!("\n>>> RUNNING: {}", test_name);
    let mut all_passed = true;

    for smiles in target_smiles {
        println!("    Target Molecule: {}", smiles);
        let mut expected_topology: Option<Vec<[u64; 4]>> = None;

        for i in 0..permutations {
            let current_topology = extract_topology(smiles, true);
            let current_fp = format_tensor_fingerprint(&current_topology);

            match &expected_topology {
                None => {
                    println!("      -> Base Tensor (Perm 0):  {}", current_fp);
                    expected_topology = Some(current_topology);
                }
                Some(expected) => {
                    // STRICT MATH: Comparing the full memory matrix, not the string fingerprint.
                    if &current_topology != expected {
                        println!("      -> ❌ Perm {} FAILED: {}", i, current_fp);
                        all_passed = false;
                        break;
                    } else if i == permutations - 1 {
                        // Just print the last one to show the loop finished successfully
                        println!("      -> Perm {}: {} (PERFECT MATCH)", i, current_fp);
                    }
                }
            }
        }
    }

    if all_passed {
        println!("    ✅ PASSED: Mathematical Invariance holds under extreme permutations.");
    }
    all_passed
}

/// Asserts that a list of DISTINCT isomers yield STRICTLY UNIQUE topologies (0 collisions).
fn run_discrimination_test(test_name: &str, isomer_smiles: &[&str]) -> bool {
    println!("\n>>> RUNNING: {}", test_name);

    let mut unique_registry: HashSet<Vec<[u64; 4]>> = HashSet::new();
    let mut no_collisions = true;

    for smiles in isomer_smiles {
        let topology = extract_topology(smiles, false);
        let fp = format_tensor_fingerprint(&topology);

        // Padding for visual alignment in console
        let padded_smiles = format!("{:width$}", smiles, width = 20);
        println!("    Isomer [ {} ] -> Tensor: {}", padded_smiles, fp);

        // STRICT MATH: HashSet evaluates the raw Vec<[u64; 4]> memory structure
        if !unique_registry.insert(topology) {
            println!("    ❌ FAILED: Collision detected! Engine collapsed structural uniqueness.");
            no_collisions = false;
        }
    }

    if no_collisions {
        println!("    ✅ PASSED: 1-WL Defeated. All isomers cleanly resolved in Galois Field.");
    }
    no_collisions
}

// =========================================================================
// MAIN DIAGNOSTIC SUITE
// =========================================================================
fn main() {
    println!("======================================================================");
    println!("🔍 RUNNING ISOLATED DIAGNOSTICS: MEMORY TENSOR VISUALIZATION");
    println!("======================================================================");

    let mut total_tests = 0;
    let mut passed_tests = 0;

    // -------------------------------------------------------------------------
    // DEMONSTRATION 1: UNIVERSAL INVARIANCE (0% False Negatives)
    // -------------------------------------------------------------------------

    let d1_l1 = run_invariance_test(
        "Demo 1 Level 1 (Positional Isomerism: Ibuprofen & Aspirin)",
        &[
            "CC(C)CC1=CC=C(C=C1)C(C)C(=O)O", // Ibuprofen
            "CC(=O)OC1=CC=CC=C1C(=O)O",      // Aspirin
        ],
        50,
    );
    total_tests += 1;
    if d1_l1 {
        passed_tests += 1;
    }

    let d1_l2 = run_invariance_test(
        "Demo 1 Level 2 (Symmetric Stress: Benzene & Coronene)",
        &[
            "C1=CC=CC=C1",                                            // Benzene
            "C1=CC2=C3C4=C1C=CC5=C4C6=C(C=C5)C=CC7=C6C3=C(C=C2)C=C7", // Coronene
        ],
        50,
    );
    total_tests += 1;
    if d1_l2 {
        passed_tests += 1;
    }

    let d1_l3 = run_invariance_test(
        "Demo 1 Level 3 (Micro-ChEMBL Proxy: 5 Diverse Targets)",
        &[
            "CN1C=NC2=C1C(=O)N(C(=O)N2C)C", // Caffeine
            "C1=CC=C(C=C1)C=O",             // Benzaldehyde
            "C1CCCCC1",                     // Cyclohexane
            "CC(C)(C)O",                    // tert-Butanol
            "C1=CC=NC=C1",                  // Pyridine
        ],
        20,
    );
    total_tests += 1;
    if d1_l3 {
        passed_tests += 1;
    }

    println!("\n----------------------------------------------------------------------");

    // -------------------------------------------------------------------------
    // DEMONSTRATION 2: ISOMER RESOLUTION (0% False Positives / 1-WL Defeat)
    // -------------------------------------------------------------------------

    let d2_l1 = run_discrimination_test(
        "Demo 2 Level 1 (Alkane Resolution: 5 Isomers of Hexane)",
        &[
            "CCCCCC",     // Hexane
            "CCCC(C)C",   // 2-Methylpentane
            "CCC(C)CC",   // 3-Methylpentane
            "CCC(C)(C)C", // 2,2-Dimethylbutane
            "CC(C)C(C)C", // 2,3-Dimethylbutane
        ],
    );
    total_tests += 1;
    if d2_l1 {
        passed_tests += 1;
    }

    let d2_l2 = run_discrimination_test(
        "Demo 2 Level 2 (Aromatic Substitution: Xylene Isomers)",
        &[
            "CC1=C(C)C=CC=C1", // Ortho-Xylene
            "CC1=CC(C)=CC=C1", // Meta-Xylene
            "CC1=CC=C(C)C=C1", // Para-Xylene
        ],
    );
    total_tests += 1;
    if d2_l2 {
        passed_tests += 1;
    }

    let d2_l3 = run_discrimination_test(
        "Demo 2 Level 3 (1-WL Paradox: Decalin vs Bicyclopentyl)",
        &[
            "C1CCC2CCCCC2C1",   // Decalin
            "C1CCC(C1)C2CCCC2", // Bicyclopentyl
        ],
    );
    total_tests += 1;
    if d2_l3 {
        passed_tests += 1;
    }

    println!("\n======================================================================");
    if passed_tests == total_tests {
        println!(
            "🏆 ALL DIAGNOSTICS PASSED! ({}/{})",
            passed_tests, total_tests
        );
        println!("The mathematical core flawlessly breaks graph automorphisms while retaining strict 256-bit invariance.");
    } else {
        println!("💥 DIAGNOSTICS FAILED! ({}/{})", passed_tests, total_tests);
    }
}
