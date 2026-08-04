#![allow(clippy::needless_range_loop)]

use crate::engine::canonizer::TopologyProvider;
use microfield::{CanonicalEncoding, Field, Fp251V1};

/// Spectral Engine operating strictly over the Prime Field GF(251).
/// Resolves local Betti numbers and closed-walk topologies to break symmetry
/// in strongly regular graphs before Galois message passing begins.
pub struct SpectralEngineF251;

impl SpectralEngineF251 {
    #[inline]
    fn canonical_u64(value: Fp251V1) -> u64 {
        u64::from(value.to_canonical()[0])
    }

    // =========================================================================
    // BRANCH A: SPARSE REGIME (Contiguous Vector Dynamic Programming)
    // Complexity: O(V * E * L) Time | O(V) Space
    // =========================================================================
    pub fn compute_sparse<T: TopologyProvider + ?Sized>(
        provider: &T,
        v_count: usize,
        l_max: usize,
    ) -> Vec<Vec<u64>> {
        let mut spectra = vec![vec![0u64; l_max]; v_count];

        // BINARY SQUASH: Precompute a unique adjacency list to prevent multigraphs.
        // This ensures the geometric skeleton is evaluated without physical multiplicity.
        let mut unique_neighbors = vec![Vec::new(); v_count];
        for c_idx in 0..provider.num_clauses() {
            let vars = provider.variables_in_clause(c_idx);
            for &u in &vars {
                for &v in &vars {
                    if u != v && !unique_neighbors[u].contains(&v) {
                        unique_neighbors[u].push(v);
                    }
                }
            }
        }

        let mut current_state = vec![Fp251V1::ZERO; v_count];
        let mut next_state = vec![Fp251V1::ZERO; v_count];

        for source_node in 0..v_count {
            current_state.fill(Fp251V1::ZERO);
            current_state[source_node] = Fp251V1::ONE;

            for step in 0..l_max {
                next_state.fill(Fp251V1::ZERO);

                // Pass mass through the simple geometric edges
                for i in 0..v_count {
                    if !current_state[i].is_zero() {
                        for &neighbor in &unique_neighbors[i] {
                            next_state[neighbor] = next_state[neighbor].add(current_state[i]);
                        }
                    }
                }

                // FIX: Removed the vestigial `trivial_bounce` subtraction.
                // Since `unique_neighbors` enforces `u != v` (no self-loops),
                // the topological walk count is natively correct without artificial corrections.

                spectra[source_node][step] = Self::canonical_u64(next_state[source_node]);
                current_state.copy_from_slice(&next_state);
            }
        }

        spectra
    }

    // =========================================================================
    // BRANCH B: DENSE REGIME (Adjacency Matrix Multiplication over GF(251))
    // Complexity: O(L * V^3) Time (Cache-friendly loop order) | O(V^2) Space
    // =========================================================================
    pub fn compute_dense<T: TopologyProvider + ?Sized>(
        provider: &T,
        v_count: usize,
        l_max: usize,
    ) -> Vec<Vec<u64>> {
        let mut spectra = vec![vec![0u64; l_max]; v_count];

        // 1. Build the Adjacency Matrix A over GF(251)
        let mut adj_matrix = vec![Fp251V1::ZERO; v_count * v_count];

        for c_idx in 0..provider.num_clauses() {
            let vars = provider.variables_in_clause(c_idx);
            for &u in &vars {
                for &v in &vars {
                    if u != v {
                        let idx = u * v_count + v;
                        // BINARY SQUASH FIX: Pure topological boolean mask.
                        // Whether they share 1 clause or 3 (e.g. triple bond), structurally they are adjacent.
                        // This prevents combinatorial explosion in the matrix trace.
                        adj_matrix[idx] = Fp251V1::ONE;
                    }
                }
            }
        }

        // 2. Base case: M_1 = A
        let mut current_matrix = adj_matrix.clone();
        let mut next_matrix = vec![Fp251V1::ZERO; v_count * v_count];

        for step in 0..l_max {
            // Extract the diagonal (Traces of M_k) which represent closed walks
            for i in 0..v_count {
                spectra[i][step] = Self::canonical_u64(current_matrix[i * v_count + i]);
            }

            if step == l_max - 1 {
                break;
            }

            // Matrix Multiplication: next_matrix = current_matrix * adj_matrix mod 251
            // Optimized loop order (i, k, j) for hardware cache prefetching
            next_matrix.fill(Fp251V1::ZERO);
            for i in 0..v_count {
                for k in 0..v_count {
                    let m_ik = current_matrix[i * v_count + k];
                    if m_ik.is_zero() {
                        continue;
                    }

                    for j in 0..v_count {
                        let a_kj = adj_matrix[k * v_count + j];
                        if a_kj.is_zero() {
                            continue;
                        }

                        let product = m_ik.mul(a_kj);
                        let dest_idx = i * v_count + j;
                        next_matrix[dest_idx] = next_matrix[dest_idx].add(product);
                    }
                }
            }

            current_matrix.copy_from_slice(&next_matrix);
        }

        spectra
    }
}
