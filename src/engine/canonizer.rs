use crate::algebra::traits::FiniteField;
use crate::algebra::galois_256::GaloisSignature256;
use crate::topology::traits::HomomorphicAggregator;
use crate::topology::multiset::MultisetAggregator;
use crate::topology::bloom_l1::{TopologicalMask, TopoBloomMask};
use crate::engine::spectral_f251::SpectralEngineF251;

/// Bipartite Interface for Cellular Complexes.
pub trait TopologyProvider {
    fn num_variables(&self) -> usize;
    fn num_clauses(&self) -> usize;
    fn variables_in_clause(&self, clause_index: usize) -> Vec<usize>;
    fn clauses_for_variable(&self, variable_index: usize) -> Vec<usize>;

    /// NEW: Provides the physical mass/seed of the variable (e.g., Atom type, Logic Gate polarity).
    /// Default implementation returns None, ensuring 100% backward compatibility
    /// with existing pure-logic tests that only care about unweighted topology.
    fn initial_state(&self, _variable_index: usize) -> Option<GaloisSignature256> {
        None
    }
}

/// The final crystallized topological DNA of a node.
#[derive(Clone, Debug)]
pub struct CanonicalNode {
    pub original_index: usize,
    pub signature: GaloisSignature256,
    pub bloom_mask: TopoBloomMask,
}

pub struct CellularGaloisCanonizer;

impl CellularGaloisCanonizer {
    /// Executes the full automorphic canonization process.
    pub fn canonize<T: TopologyProvider + ?Sized>(provider: &T, max_iterations: usize) -> Vec<CanonicalNode> {
        let v_count = provider.num_variables();
        let c_count = provider.num_clauses();

        if v_count == 0 { return vec![]; }

        // Phase 1: Hybrid Spectral Initialization (GF_251 Betti Numbers)
        let mut var_signatures = Self::hybrid_spectral_initialization(provider, v_count, c_count);

        // NEW: Atomic Seed Injection (Curing Atomic Blindness)
        for i in 0..v_count {
            if let Some(seed) = provider.initial_state(i) {
                // Homomorphically add the chemical/physical identity to the topological spectrum.
                // This guarantees both geometric and material uniqueness in the baseline signature.
                var_signatures[i] = var_signatures[i].add(&seed);
            }
        }

        let mut clause_signatures = vec![GaloisSignature256::zero(); c_count];

        // L1 Shield Pre-computation
        let mut var_masks = vec![TopoBloomMask::empty(); v_count];
        for i in 0..v_count {
            var_masks[i] = TopoBloomMask::from_variable_index(i);
        }

        // Phase 2: Cohomological Refinement (Message Passing)
        for _ in 0..max_iterations {
            // Step 2A: Volume Update (Clauses)
            let mut next_clause_signatures = vec![GaloisSignature256::zero(); c_count];
            for j in 0..c_count {
                let mut clause_state = MultisetAggregator::empty_state();
                for &v_idx in &provider.variables_in_clause(j) {
                    clause_state = MultisetAggregator::aggregate(
                        &clause_state,
                        &var_signatures[v_idx],
                        0
                    );
                }

                // Asymmetric Inertia: S^2 * Phi
                let inertia = clause_signatures[j].mul(&clause_signatures[j]).shift_phase();
                next_clause_signatures[j] = inertia.add(&clause_state);
            }
            clause_signatures = next_clause_signatures;

            // Step 2B: Boundary Update (Variables)
            let mut next_var_signatures = vec![GaloisSignature256::zero(); v_count];
            for i in 0..v_count {
                let mut var_state = MultisetAggregator::empty_state();
                let mut var_mask = var_masks[i].clone();

                for &c_idx in &provider.clauses_for_variable(i) {
                    var_state = MultisetAggregator::aggregate(
                        &var_state,
                        &clause_signatures[c_idx],
                        0
                    );

                    // Accumulate L1 entropy
                    for &neighbor_v in &provider.variables_in_clause(c_idx) {
                        var_mask = var_mask.union(&var_masks[neighbor_v]);
                    }
                }

                let inertia = var_signatures[i].mul(&var_signatures[i]).shift_phase();
                next_var_signatures[i] = inertia.add(&var_state);
                var_masks[i] = var_mask;
            }
            var_signatures = next_var_signatures;
        }

        // Output Generation
        var_signatures.into_iter().enumerate().zip(var_masks.into_iter()).map(|((i, sig), mask)| {
            CanonicalNode {
                original_index: i,
                signature: sig,
                bloom_mask: mask,
            }
        }).collect()
    }

    /// Implements the Context-Aware Hybrid Engine to prevent thermodynamic collapse.
    fn hybrid_spectral_initialization<T: TopologyProvider + ?Sized>(
        provider: &T,
        v_count: usize,
        c_count: usize
    ) -> Vec<GaloisSignature256> {
        let density = if v_count > 0 { c_count as f64 / v_count as f64 } else { 0.0 };
        let split_threshold = (v_count as f64).sqrt().max(10.0);

        let estimated_diameter = if density > 10.0 { 2 } else { 4 };
        let l_max = 9.min(2 * estimated_diameter + 1);

        let spectra = if density >= split_threshold {
            SpectralEngineF251::compute_dense(provider, v_count, l_max)
        } else {
            SpectralEngineF251::compute_sparse(provider, v_count, l_max)
        };

        spectra.into_iter().map(|spectrum| {
            let mut buffer = [0u8; 32];
            for (i, &walk_count) in spectrum.iter().enumerate().take(8) {
                buffer[i] = walk_count as u8;
            }
            GaloisSignature256::from_bytes_canonical(&buffer)
        }).collect()
    }
}
