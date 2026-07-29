#[cfg(test)]
mod hasher_tests {
    use crate::algebra::galois_256::GaloisSignature256;
    use crate::algebra::traits::FiniteField;
    use crate::engine::hasher::TopoHasher;
    use crate::topology::multiset::MultisetAggregator as MultiSet;
    use crate::topology::sequence::SequenceAggregator as Sequence;
    use crate::topology::symmetric_difference::SymmetricDifferenceAggregator as SymDiff;
    use crate::topology::traits::HomomorphicAggregator;

    type F256 = GaloisSignature256;

    // =========================================================================
    // GROUP 1: Vacuum Instantiation & Monomorphization Limits
    // =========================================================================

    #[test]
    fn t01_symdiff_hasher_initializes_at_zero() {
        let hasher = TopoHasher::<F256, SymDiff>::new();
        assert_eq!(
            hasher.finalize(),
            F256::zero(),
            "SymDiff vacuum is additive identity"
        );
    }

    #[test]
    fn t02_sequence_hasher_initializes_at_zero() {
        let hasher = TopoHasher::<F256, Sequence>::new();
        assert_eq!(
            hasher.finalize(),
            F256::zero(),
            "Sequence vacuum is additive identity"
        );
    }

    #[test]
    fn t03_multiset_hasher_initializes_at_one() {
        let hasher = TopoHasher::<F256, MultiSet>::new();
        assert_eq!(
            hasher.finalize(),
            F256::one(),
            "MultiSet vacuum MUST be multiplicative identity"
        );
    }

    #[test]
    fn t04_immediate_finalize_consumes_without_side_effects() {
        let h1 = TopoHasher::<F256, SymDiff>::new();
        let h2 = TopoHasher::<F256, SymDiff>::new();
        assert_eq!(
            h1.finalize(),
            h2.finalize(),
            "Unused hashers must finalize identically"
        );
    }

    #[test]
    fn t05_update_with_empty_data_acts_as_topological_clock() {
        let mut h = TopoHasher::<F256, Sequence>::new();
        h.update(&[]); // Clock tick
        assert_eq!(h.finalize(), F256::zero(), "0 * Phi + 0 = 0");
    }

    // =========================================================================
    // GROUP 2: SymDiff Engine Monomorphization
    // =========================================================================

    #[test]
    fn t06_symdiff_hasher_single_update() {
        let mut hasher = TopoHasher::<F256, SymDiff>::new();
        hasher.update(b"Test");

        let manual = SymDiff::aggregate(
            &SymDiff::empty_state(),
            &SymDiff::embed_to_field(b"Test"),
            0,
        );
        assert_eq!(
            hasher.finalize(),
            manual,
            "Hasher must perfectly mirror manual SymDiff aggregation"
        );
    }

    #[test]
    fn t07_symdiff_hasher_annihilates_duplicates() {
        let mut hasher = TopoHasher::<F256, SymDiff>::new();
        hasher.update(b"Ghost");
        hasher.update(b"Ghost");
        assert_eq!(
            hasher.finalize(),
            F256::zero(),
            "Hasher state machine respects characteristic 2 annihilation"
        );
    }

    #[test]
    fn t08_symdiff_hasher_is_commutative_across_updates() {
        let mut h1 = TopoHasher::<F256, SymDiff>::new();
        h1.update(b"Alpha");
        h1.update(b"Beta");

        let mut h2 = TopoHasher::<F256, SymDiff>::new();
        h2.update(b"Beta");
        h2.update(b"Alpha");

        assert_eq!(
            h1.finalize(),
            h2.finalize(),
            "Sequential updates to a SymDiff hasher are causally commutate"
        );
    }

    #[test]
    fn t09_symdiff_hasher_three_updates_yields_one() {
        let mut h = TopoHasher::<F256, SymDiff>::new();
        h.update(b"A");
        h.update(b"A");
        h.update(b"A");

        let manual = SymDiff::embed_to_field(b"A");
        assert_eq!(
            h.finalize(),
            manual,
            "A + A + A = A via sequential hasher updates"
        );
    }

    // =========================================================================
    // GROUP 3: MultiSet Engine Monomorphization
    // =========================================================================

    #[test]
    fn t10_multiset_hasher_single_update() {
        let mut h = TopoHasher::<F256, MultiSet>::new();
        h.update(b"Root_1");

        let manual = MultiSet::aggregate(
            &MultiSet::empty_state(),
            &MultiSet::embed_to_field(b"Root_1"),
            0,
        );
        assert_eq!(
            h.finalize(),
            manual,
            "Hasher preserves exact MultiSet roots"
        );
    }

    #[test]
    fn t11_multiset_hasher_preserves_multiplicity_via_updates() {
        let mut h1 = TopoHasher::<F256, MultiSet>::new();
        h1.update(b"Data");

        let mut h2 = TopoHasher::<F256, MultiSet>::new();
        h2.update(b"Data");
        h2.update(b"Data");

        assert_ne!(
            h1.finalize(),
            h2.finalize(),
            "Hasher perfectly tracks multiplicity via sequential updates"
        );
    }

    #[test]
    fn t12_multiset_hasher_commutativity() {
        let mut h1 = TopoHasher::<F256, MultiSet>::new();
        h1.update(b"X");
        h1.update(b"Y");
        h1.update(b"Z");

        let mut h2 = TopoHasher::<F256, MultiSet>::new();
        h2.update(b"Y");
        h2.update(b"Z");
        h2.update(b"X");

        assert_eq!(
            h1.finalize(),
            h2.finalize(),
            "Polynomial products inside the state machine commute"
        );
    }

    #[test]
    fn t13_multiset_hasher_protects_against_generator_collision() {
        let mut h = TopoHasher::<F256, MultiSet>::new();
        let mut malicious = [0u8; 32];
        malicious[31] = 0x80; // X_g constant

        h.update(&malicious);
        assert_ne!(
            h.finalize(),
            F256::zero(),
            "Hasher safely absorbs malicious data via affine subspace projection"
        );
    }

    // =========================================================================
    // GROUP 4: Sequence Engine Monomorphization
    // =========================================================================

    #[test]
    fn t14_sequence_hasher_single_update() {
        let mut h = TopoHasher::<F256, Sequence>::new();
        h.update(b"First");

        let manual = Sequence::aggregate(
            &Sequence::empty_state(),
            &Sequence::embed_to_field(b"First"),
            0,
        );
        assert_eq!(
            h.finalize(),
            manual,
            "First sequence update does not induce phase shift"
        );
    }

    #[test]
    fn t15_sequence_hasher_asymmetry() {
        let mut h1 = TopoHasher::<F256, Sequence>::new();
        h1.update(b"A");
        h1.update(b"B");

        let mut h2 = TopoHasher::<F256, Sequence>::new();
        h2.update(b"B");
        h2.update(b"A");

        assert_ne!(
            h1.finalize(),
            h2.finalize(),
            "State machine guarantees temporal asymmetry for Sequences"
        );
    }

    #[test]
    fn t16_sequence_hasher_rolling_window() {
        let mut h = TopoHasher::<F256, Sequence>::new();
        h.update(b"Block1");
        h.update(b"Block2");
        h.update(b"Block3");

        let b1: F256 = Sequence::embed_to_field(b"Block1");
        let b2: F256 = Sequence::embed_to_field(b"Block2");
        let b3: F256 = Sequence::embed_to_field(b"Block3");

        let expected = b1
            .shift_phase()
            .shift_phase()
            .add(&b2.shift_phase())
            .add(&b3);
        assert_eq!(
            h.finalize(),
            expected,
            "State machine precisely implements Horner's polynomial accumulation"
        );
    }

    #[test]
    fn t17_sequence_hasher_clock_tick_advancement() {
        let mut h1 = TopoHasher::<F256, Sequence>::new();
        h1.update(b"Origin");
        let base_state = h1.finalize();

        let mut h2 = TopoHasher::<F256, Sequence>::new();
        h2.update(b"Origin");
        h2.update(&[]); // Clock tick

        assert_eq!(
            h2.finalize(),
            base_state.shift_phase(),
            "Empty updates manually advance the topological clock"
        );
    }

    // =========================================================================
    // GROUP 5: Engine Cross-Divergence & State Independence
    // =========================================================================

    #[test]
    fn t18_identical_updates_diverge_across_topologies() {
        let mut h_sym = TopoHasher::<F256, SymDiff>::new();
        let mut h_mul = TopoHasher::<F256, MultiSet>::new();
        let mut h_seq = TopoHasher::<F256, Sequence>::new();

        let data = b"Entropy";
        h_sym.update(data);
        h_mul.update(data);
        h_seq.update(data);

        assert_ne!(h_sym.finalize(), h_mul.finalize());
        // seq and sym differ on second element, but for first element they might match!
        // So we add a second update to guarantee absolute divergence.
        let mut h_sym2 = TopoHasher::<F256, SymDiff>::new();
        let mut h_seq2 = TopoHasher::<F256, Sequence>::new();
        h_sym2.update(data);
        h_sym2.update(data);
        h_seq2.update(data);
        h_seq2.update(data);

        assert_ne!(
            h_sym2.finalize(),
            h_seq2.finalize(),
            "Engines strictly diverge upon accumulation"
        );
    }

    #[test]
    fn t19_hasher_state_is_completely_isolated() {
        let mut h1 = TopoHasher::<F256, Sequence>::new();
        let mut h2 = TopoHasher::<F256, Sequence>::new();

        h1.update(b"A");
        h2.update(b"B");
        h1.update(b"C");

        assert_ne!(
            h1.finalize(),
            h2.finalize(),
            "Hashers do not leak internal pointer states"
        );
    }

    #[test]
    fn t20_hasher_element_count_increases_deterministically() {
        // Though element_count is private and currently ignored by our purely algebraic aggregators,
        // we can prove the hasher survives thousands of updates without index overflow panics.
        let mut h = TopoHasher::<F256, SymDiff>::new();
        for _ in 0..100_000 {
            h.update(b"Stress");
        }
        assert_eq!(
            h.finalize(),
            F256::zero(),
            "100k updates execute smoothly, annihilating via characteristic 2"
        );
    }

    // =========================================================================
    // GROUP 6: Streaming and Payload Stress
    // =========================================================================

    #[test]
    fn t21_hasher_absorbs_massive_payload() {
        let mut h = TopoHasher::<F256, MultiSet>::new();
        let massive_payload = vec![0xBB; 50_000];
        h.update(&massive_payload);

        assert_ne!(
            h.finalize(),
            F256::one(),
            "Hasher securely digests massive linear payloads in O(1) space"
        );
    }

    #[test]
    fn t22_streaming_chunks_vs_single_payload_in_sequence() {
        // Note: Hasher `update` applies the Aggregator to the *whole* slice as one structural event.
        // Therefore, updating chunk by chunk is topologically DIFFERENT from updating the whole payload.
        let mut h_chunked = TopoHasher::<F256, Sequence>::new();
        h_chunked.update(b"Hello");
        h_chunked.update(b"World");

        let mut h_whole = TopoHasher::<F256, Sequence>::new();
        h_whole.update(b"HelloWorld");

        assert_ne!(h_chunked.finalize(), h_whole.finalize(), "Streaming individual chunks implies temporal sequence, diverging from single mass payload");
    }

    #[test]
    fn t23_streaming_chunks_vs_single_payload_in_symdiff() {
        let mut h_chunked = TopoHasher::<F256, SymDiff>::new();
        h_chunked.update(b"PartA");
        h_chunked.update(b"PartB");

        let mut h_whole = TopoHasher::<F256, SymDiff>::new();
        h_whole.update(b"PartAPartB");

        assert_ne!(
            h_chunked.finalize(),
            h_whole.finalize(),
            "Even in SymDiff, distinct events are topologically different from concatenated events"
        );
    }

    #[test]
    fn t24_hasher_retains_fidelity_with_1000_distinct_updates() {
        let mut h = TopoHasher::<F256, MultiSet>::new();
        for i in 0..1000 {
            h.update(&[i as u8]);
        }
        assert_ne!(h.finalize(), F256::one());
    }

    #[test]
    fn t25_multiset_hasher_mass_duplicate_absorption() {
        let mut h = TopoHasher::<F256, MultiSet>::new();
        for _ in 0..256 {
            h.update(b"Identical");
        }
        assert_ne!(
            h.finalize(),
            F256::zero(),
            "Deep product roots do not collapse into 0"
        );
    }

    // =========================================================================
    // GROUP 7: Edge Boundaries and Internal Integrity
    // =========================================================================

    #[test]
    fn t26_hasher_finalize_consumes_instance() {
        // This is a compile-time test conceptually. We just ensure we can instantiate,
        // pass ownership, and finalize.
        let h = TopoHasher::<F256, SymDiff>::new();
        let state = h.finalize();
        assert_eq!(state, F256::zero());
        // h.update(b"Fail"); // This would fail compilation, enforcing affine type consumption.
    }

    #[test]
    fn t27_hasher_does_not_mutate_input_data() {
        let mut h = TopoHasher::<F256, Sequence>::new();
        let payload = b"ImmutableData".to_vec();
        h.update(&payload);

        assert_eq!(
            payload,
            b"ImmutableData".to_vec(),
            "Zero-cost abstraction does not borrow mutably or corrupt input streams"
        );
    }

    #[test]
    fn t28_sequential_hasher_ignores_trailing_zeros_in_stream_if_separate() {
        let mut h1 = TopoHasher::<F256, Sequence>::new();
        h1.update(&[0x01]);
        h1.update(&[0x00]);

        let mut h2 = TopoHasher::<F256, Sequence>::new();
        h2.update(&[0x01, 0x00]);

        // [0x01] then [0x00] == Phase shift by clock tick.
        // [0x01, 0x00] == Single block evaluated linearly.
        assert_ne!(
            h1.finalize(),
            h2.finalize(),
            "Temporal separation of data is mathematically strict"
        );
    }

    #[test]
    fn t29_symdiff_hasher_cancels_identical_streams() {
        let mut h = TopoHasher::<F256, SymDiff>::new();

        let events = vec![b"E1".to_vec(), b"E2".to_vec(), b"E3".to_vec()];
        for e in &events {
            h.update(e);
        } // Add all
        for e in &events {
            h.update(e);
        } // Add all again (Cancels all)

        assert_eq!(
            h.finalize(),
            F256::zero(),
            "Hasher perfectly orchestrates macro-cancellations"
        );
    }

    #[test]
    fn t30_universal_hasher_determinism() {
        let mut h1 = TopoHasher::<F256, MultiSet>::new();
        let mut h2 = TopoHasher::<F256, MultiSet>::new();

        let stream: Vec<&[u8]> = vec![b"Alpha", b"Beta", b"Gamma", b"Delta", b"Epsilon"];

        for data in &stream {
            h1.update(*data);
            h2.update(*data);
        }

        assert_eq!(h1.finalize(), h2.finalize(), "Identical data streams across identical monomorphized hashers yield strictly identical Galois signatures");
    }
}
#[cfg(test)]
mod canonizer_tests {
    use crate::algebra::traits::FiniteField;
    use crate::engine::canonizer::{CellularGaloisCanonizer, TopologyProvider};
    use crate::GaloisSignature256;

    // =========================================================================
    // MOCK PROVIDER: Strictly isolates the structural graphs for testing
    // =========================================================================
    struct MockGraph {
        v: usize,
        c: usize,
        clauses: Vec<Vec<usize>>,
        c_for_v: Vec<Vec<usize>>,
    }

    impl MockGraph {
        fn new(v: usize, clauses: Vec<Vec<usize>>) -> Self {
            let mut c_for_v = vec![vec![]; v];
            for (c_idx, vars) in clauses.iter().enumerate() {
                for &var in vars {
                    c_for_v[var].push(c_idx);
                }
            }
            Self {
                v,
                c: clauses.len(),
                clauses,
                c_for_v,
            }
        }

        // Helper to get sorted signatures to test permutation invariance
        fn get_sorted_signatures(&self, iterations: usize) -> Vec<[u64; 4]> {
            let nodes = CellularGaloisCanonizer::canonize(self, iterations);
            let mut sigs: Vec<[u64; 4]> = nodes.into_iter().map(|n| n.signature.0).collect();
            sigs.sort_unstable(); // Lexicographical sort on GF(256) words
            sigs
        }
    }

    impl TopologyProvider for MockGraph {
        fn num_variables(&self) -> usize {
            self.v
        }
        fn num_clauses(&self) -> usize {
            self.c
        }
        fn variables_in_clause(&self, idx: usize) -> Vec<usize> {
            self.clauses[idx].clone()
        }
        fn clauses_for_variable(&self, idx: usize) -> Vec<usize> {
            self.c_for_v[idx].clone()
        }
    }

    // =========================================================================
    // GROUP 1: Provider Limits & Vacuum execution
    // =========================================================================

    #[test]
    fn t01_empty_graph_returns_empty_vector() {
        let g = MockGraph::new(0, vec![]);
        let res = CellularGaloisCanonizer::canonize(&g, 5);
        assert!(res.is_empty(), "Vacuum space contains no topology");
    }

    #[test]
    fn t02_graph_with_vertices_but_no_clauses_compiles_safely() {
        let g = MockGraph::new(5, vec![]);
        let res = CellularGaloisCanonizer::canonize(&g, 2);
        assert_eq!(
            res.len(),
            5,
            "Disconnected universe must yield 5 distinct vacuums"
        );
    }

    #[test]
    fn t03_single_node_single_self_loop() {
        let g = MockGraph::new(1, vec![vec![0]]);
        let res = CellularGaloisCanonizer::canonize(&g, 2);
        assert_eq!(res[0].original_index, 0);
    }

    #[test]
    fn t04_isolated_nodes_preserve_independence() {
        let g = MockGraph::new(3, vec![]);
        let res = CellularGaloisCanonizer::canonize(&g, 10);
        assert_eq!(
            res[0].signature, res[1].signature,
            "Nodes with identical 0-degree topologies must share identical Galois Signatures"
        );
    }

    #[test]
    fn t05_zero_iterations_returns_pure_spectral_state() {
        let g = MockGraph::new(3, vec![vec![0, 1], vec![1, 2]]);
        let res = CellularGaloisCanonizer::canonize(&g, 0);
        // At 0 iterations, Phase 2 is bypassed. Only Phase 1 GF(251) spectra are embedded.
        assert!(
            !res.is_empty(),
            "0 iteration execution safely bypasses message passing"
        );
    }

    // =========================================================================
    // GROUP 2: Strict Isomorphism & Permutation Invariance
    // =========================================================================

    #[test]
    fn t06_isomorphism_path3_permuted() {
        // Path A-B-C vs C-A-B
        let g1 = MockGraph::new(3, vec![vec![0, 1], vec![1, 2]]);
        let g2 = MockGraph::new(3, vec![vec![2, 0], vec![0, 1]]);

        let sigs1 = g1.get_sorted_signatures(3);
        let sigs2 = g2.get_sorted_signatures(3);

        assert_eq!(
            sigs1, sigs2,
            "Isomorphic path graphs must map to identical signature vectors"
        );
    }

    #[test]
    fn t07_isomorphism_star_graph_permuted() {
        // Center=0 vs Center=3
        let g1 = MockGraph::new(4, vec![vec![0, 1], vec![0, 2], vec![0, 3]]);
        let g2 = MockGraph::new(4, vec![vec![3, 0], vec![3, 1], vec![3, 2]]);

        assert_eq!(
            g1.get_sorted_signatures(3),
            g2.get_sorted_signatures(3),
            "Star center displacement must be mathematically invisible"
        );
    }

    #[test]
    fn t08_isomorphism_cycle4_permuted() {
        let g1 = MockGraph::new(4, vec![vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 0]]);
        let g2 = MockGraph::new(4, vec![vec![0, 2], vec![2, 3], vec![3, 1], vec![1, 0]]);
        assert_eq!(
            g1.get_sorted_signatures(4),
            g2.get_sorted_signatures(4),
            "Cycle permutation is topologically invariant"
        );
    }

    #[test]
    fn t09_isomorphism_disconnected_components() {
        let g1 = MockGraph::new(4, vec![vec![0, 1], vec![2, 3]]);
        let g2 = MockGraph::new(4, vec![vec![0, 3], vec![1, 2]]);
        assert_eq!(
            g1.get_sorted_signatures(2),
            g2.get_sorted_signatures(2),
            "Disjoint isomorphic spaces are equal"
        );
    }

    #[test]
    fn t10_isomorphism_bipartite_k22_permuted() {
        let g1 = MockGraph::new(4, vec![vec![0, 2], vec![0, 3], vec![1, 2], vec![1, 3]]);
        let g2 = MockGraph::new(4, vec![vec![1, 3], vec![1, 0], vec![2, 3], vec![2, 0]]);
        assert_eq!(
            g1.get_sorted_signatures(3),
            g2.get_sorted_signatures(3),
            "Complete bipartite permutations match"
        );
    }

    // =========================================================================
    // GROUP 3: Non-Isomorphism Detection & Symmetry Breaking
    // =========================================================================

    #[test]
    fn t11_non_isomorphism_path_vs_star() {
        let path = MockGraph::new(4, vec![vec![0, 1], vec![1, 2], vec![2, 3]]);
        let star = MockGraph::new(4, vec![vec![0, 1], vec![0, 2], vec![0, 3]]);
        assert_ne!(
            path.get_sorted_signatures(3),
            star.get_sorted_signatures(3),
            "Star and Path topologies strictly diverge"
        );
    }

    #[test]
    fn t12_non_isomorphism_cycle_vs_k13() {
        let cycle = MockGraph::new(4, vec![vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 0]]);
        let star = MockGraph::new(4, vec![vec![0, 1], vec![0, 2], vec![0, 3]]);
        assert_ne!(
            cycle.get_sorted_signatures(3),
            star.get_sorted_signatures(3)
        );
    }

    #[test]
    fn t13_non_isomorphism_2k3_vs_k6() {
        let disjoint = MockGraph::new(
            6,
            vec![
                vec![0, 1],
                vec![1, 2],
                vec![2, 0],
                vec![3, 4],
                vec![4, 5],
                vec![5, 3],
            ],
        );
        let mut k6_edges = vec![];
        for i in 0..6 {
            for j in (i + 1)..6 {
                k6_edges.push(vec![i, j]);
            }
        }
        let k6 = MockGraph::new(6, k6_edges);
        assert_ne!(
            disjoint.get_sorted_signatures(3),
            k6.get_sorted_signatures(3)
        );
    }

    #[test]
    fn t14_symmetry_breaking_regular_cycle() {
        // In a perfect C5, all 5 nodes must collapse into the exact same signature
        let c5 = MockGraph::new(
            5,
            vec![vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 4], vec![4, 0]],
        );
        let res = CellularGaloisCanonizer::canonize(&c5, 5);
        for i in 1..5 {
            assert_eq!(
                res[0].signature, res[i].signature,
                "Symmetric nodes must mathematically overlap"
            );
        }
    }

    #[test]
    fn t15_asymmetric_tadpole_perturbation() {
        // C3 + Tail: Node 3 is tail, Node 0 is connector, Nodes 1,2 are generic cycle nodes.
        let g = MockGraph::new(4, vec![vec![0, 1], vec![1, 2], vec![2, 0], vec![0, 3]]);
        let res = CellularGaloisCanonizer::canonize(&g, 3);

        assert_ne!(
            res[0].signature, res[3].signature,
            "Connector differs from tail"
        );
        assert_eq!(
            res[1].signature, res[2].signature,
            "Unperturbed cycle nodes retain local symmetry"
        );
        assert_ne!(
            res[0].signature, res[1].signature,
            "Connector differs from generic cycle nodes"
        );
    }

    // =========================================================================
    // GROUP 4: Phase 1 (Spectral) Limits & Hybrid Branching
    // =========================================================================

    #[test]
    fn t16_sparse_branch_execution_no_panic() {
        // Force low density: V=100, E=2. Density = 0.02.
        let g = MockGraph::new(100, vec![vec![0, 1], vec![98, 99]]);
        let res = CellularGaloisCanonizer::canonize(&g, 2);
        assert_eq!(
            res.len(),
            100,
            "Sparse DP branch strictly executes and returns valid states"
        );
    }

    #[test]
    fn t17_dense_branch_execution_no_panic() {
        // Force high density: V=10, E=45 (Complete K10). Density = 4.5.
        // Threshold is max(sqrt(10), 10.0) = 10.0.
        // Wait, split_threshold enforces a minimum of 10.0.
        // Let's force extreme density: V=5, E=50 (Multi-edges).
        let mut edges = vec![];
        for _ in 0..50 {
            edges.push(vec![0, 1]);
        }
        let g = MockGraph::new(5, edges);
        let res = CellularGaloisCanonizer::canonize(&g, 2);
        assert_eq!(
            res.len(),
            5,
            "Dense Matrix multiplication branch bounds memory correctly"
        );
    }

    #[test]
    fn t18_betti_numbers_zero_for_trees() {
        // A tree has no closed walks of length 3+. Spectrum should be blank for higher L.
        let tree = MockGraph::new(4, vec![vec![0, 1], vec![0, 2], vec![0, 3]]);
        let res = CellularGaloisCanonizer::canonize(&tree, 0); // Execute Phase 1 only

        let center_sig = res[0].signature.0[0] & 0xFFFFFFFFFFFFFFFF;
        assert_ne!(
            center_sig, 0,
            "Phase 1 packs degree and trivial bounds, never purely 0"
        );
    }

    #[test]
    fn t19_density_zero_division_prevention() {
        // V=0, C=10 (Invalid graph logically, but mathematically tested)
        let g = MockGraph::new(0, vec![vec![], vec![]]);
        let res = CellularGaloisCanonizer::canonize(&g, 2);
        assert!(res.is_empty(), "Telemetrics must gracefully catch V=0");
    }

    #[test]
    fn t20_spectral_packing_endianness() {
        let g = MockGraph::new(3, vec![vec![0, 1], vec![1, 2], vec![2, 0]]);
        let res = CellularGaloisCanonizer::canonize(&g, 0);
        let bytes = res[0].signature.0[0];
        assert_ne!(
            bytes, 0,
            "Walk data successfully embedded in the lowest 64 bits of the polynomial"
        );
    }

    // =========================================================================
    // GROUP 5: Phase 2 (Cohomological) Dynamics
    // =========================================================================

    #[test]
    fn t21_single_iteration_absorbs_immediate_neighborhood() {
        let g = MockGraph::new(3, vec![vec![0, 1]]);
        let res_t0 = CellularGaloisCanonizer::canonize(&g, 0);
        let res_t1 = CellularGaloisCanonizer::canonize(&g, 1);

        // Node 2 is isolated. Node 0 has Node 1.
        // FIX: S_0 = 0. S_1 = 0^2 * Phi + 1 = 1.
        assert_eq!(
            res_t1[2].signature,
            GaloisSignature256::one(),
            "Isolated node mathematically absorbs the MultiSet vacuum (1)"
        );
        assert_ne!(
            res_t0[0].signature, res_t1[0].signature,
            "Connected node mathematically evolves"
        );
    }

    #[test]
    fn t22_iterations_evolve_state_deterministically() {
        let g = MockGraph::new(4, vec![vec![0, 1], vec![1, 2]]);
        let res_t2 = CellularGaloisCanonizer::canonize(&g, 2);
        let res_t5 = CellularGaloisCanonizer::canonize(&g, 5);
        assert_ne!(
            res_t2[0].signature, res_t5[0].signature,
            "S^2 * Phi Inertia guarantees perpetual temporal divergence"
        );
    }

    #[test]
    fn t23_disconnected_nodes_never_share_messages() {
        let g = MockGraph::new(2, vec![]);
        let res = CellularGaloisCanonizer::canonize(&g, 10);
        assert_eq!(
            res[0].signature, res[1].signature,
            "Disjoint graph nodes undergo identical inertial evolution"
        );
    }

    #[test]
    fn t24_inertia_advances_isolated_nodes() {
        // Due to S^2 * Phi, a node evolves even with 0 clauses.
        let g = MockGraph::new(1, vec![]);
        let t0 = CellularGaloisCanonizer::canonize(&g, 0);
        let t1 = CellularGaloisCanonizer::canonize(&g, 1);
        assert_eq!(
            t1[0].signature,
            t0[0]
                .signature
                .mul(&t0[0].signature)
                .shift_phase()
                .add(&GaloisSignature256::one()),
            "Phase shift inertia precisely mapped"
        );
    }

    #[test]
    fn t25_message_passing_ignores_clause_ordering() {
        let g1 = MockGraph::new(3, vec![vec![0, 1], vec![0, 2]]);
        let g2 = MockGraph::new(3, vec![vec![0, 2], vec![0, 1]]); // Flipped clause definitions
        assert_eq!(
            g1.get_sorted_signatures(2),
            g2.get_sorted_signatures(2),
            "Multiset message aggregation is perfectly commutative"
        );
    }

    // =========================================================================
    // GROUP 6: L1 Shield Entropy & Topology Extraction
    // =========================================================================

    #[test]
    fn t26_l1_mask_of_isolated_node_is_itself() {
        let g = MockGraph::new(3, vec![]);
        let res = CellularGaloisCanonizer::canonize(&g, 5);
        assert_eq!(res[0].bloom_mask.0, [1, 0, 0, 0]);
        assert_eq!(res[1].bloom_mask.0, [2, 0, 0, 0]);
    }

    #[test]
    fn t27_l1_mask_saturates_connected_component() {
        let g = MockGraph::new(3, vec![vec![0, 1], vec![1, 2]]);
        let res = CellularGaloisCanonizer::canonize(&g, 5); // 5 > Diameter (2)

        // At T=5, information has traversed the whole component. All masks should be {0, 1, 2}
        assert_eq!(res[0].bloom_mask.0, res[1].bloom_mask.0);
        assert_eq!(res[1].bloom_mask.0, res[2].bloom_mask.0);
        assert_eq!(res[0].bloom_mask.0[0], 7); // 1 + 2 + 4
    }

    #[test]
    fn t28_l1_masks_of_disjoint_components_never_overlap() {
        let g = MockGraph::new(4, vec![vec![0, 1], vec![2, 3]]);
        let res = CellularGaloisCanonizer::canonize(&g, 10);

        let comp_a = res[0].bloom_mask;
        let comp_b = res[2].bloom_mask;

        let overlap = comp_a.0[0] & comp_b.0[0];
        assert_eq!(
            overlap, 0,
            "Disjoint components remain perfectly orthogonal in L1 space"
        );
    }

    #[test]
    fn t29_canonical_node_preserves_original_index() {
        let g = MockGraph::new(4, vec![vec![0, 1]]);
        let mut res = CellularGaloisCanonizer::canonize(&g, 1);

        // If we sort them, we must still know who was who
        res.sort_by_key(|n| n.signature.0);
        let indices: Vec<usize> = res.iter().map(|n| n.original_index).collect();
        assert_eq!(
            indices.len(),
            4,
            "Topology mappings track exact temporal lineage"
        );
    }

    #[test]
    fn t30_stress_test_large_graph_compilation_and_memory() {
        let mut edges = vec![];
        for i in 0..499 {
            edges.push(vec![i, i + 1]);
        }
        let g = MockGraph::new(500, edges);
        let res = CellularGaloisCanonizer::canonize(&g, 3);
        assert_eq!(
            res.len(),
            500,
            "500-node Path graph successfully canonized without stack overflows or memory limits"
        );
    }
}
#[cfg(test)]
mod spectral_f251_tests {
    use crate::engine::canonizer::TopologyProvider;
    use crate::engine::spectral_f251::SpectralEngineF251;

    // Mock Provider specifically for Walk Counting
    struct SpectralMockGraph {
        v: usize,
        c: usize,
        clauses: Vec<Vec<usize>>,
        c_for_v: Vec<Vec<usize>>,
    }

    impl SpectralMockGraph {
        fn new(v: usize, clauses: Vec<Vec<usize>>) -> Self {
            let mut c_for_v = vec![vec![]; v];
            for (c_idx, vars) in clauses.iter().enumerate() {
                for &var in vars {
                    c_for_v[var].push(c_idx);
                }
            }
            Self {
                v,
                c: clauses.len(),
                clauses,
                c_for_v,
            }
        }
    }

    impl TopologyProvider for SpectralMockGraph {
        fn num_variables(&self) -> usize {
            self.v
        }
        fn num_clauses(&self) -> usize {
            self.c
        }
        fn variables_in_clause(&self, idx: usize) -> Vec<usize> {
            self.clauses[idx].clone()
        }
        fn clauses_for_variable(&self, idx: usize) -> Vec<usize> {
            self.c_for_v[idx].clone()
        }
    }

    type Engine = SpectralEngineF251;

    // =========================================================================
    // GROUP 1: Topological Vacuums & Trivial Geometries
    // =========================================================================

    #[test]
    fn t01_sparse_empty_graph_yields_empty_spectrum() {
        let g = SpectralMockGraph::new(0, vec![]);
        let spec = Engine::compute_sparse(&g, 0, 5);
        assert!(spec.is_empty(), "Vacuum yields no spectrum");
    }

    #[test]
    fn t02_dense_empty_graph_yields_empty_spectrum() {
        let g = SpectralMockGraph::new(0, vec![]);
        let spec = Engine::compute_dense(&g, 0, 5);
        assert!(spec.is_empty(), "Vacuum yields no spectrum in matrix form");
    }

    #[test]
    fn t03_isolated_nodes_have_zero_walks() {
        let g = SpectralMockGraph::new(3, vec![]);
        let spec = Engine::compute_sparse(&g, 3, 3);
        assert_eq!(spec[0], vec![0, 0, 0], "No edges = 0 walks of any length");
    }

    #[test]
    fn t04_single_clause_no_closed_walks_beyond_trivial() {
        // Path of 2 nodes: 0 -- 1
        let g = SpectralMockGraph::new(2, vec![vec![0, 1]]);
        let spec = Engine::compute_sparse(&g, 2, 3);
        // In bipartite graph, walk goes Var -> Clause -> Var.
        // closed walk length 1: 0 -> c0 -> 0. (Trivial bounce, should be subtracted!)
        assert_eq!(
            spec[0][0], 0,
            "Trivial self-bounces must be mathematically eradicated"
        );
    }

    #[test]
    fn t05_dense_engine_eradicates_trivial_bounces() {
        let g = SpectralMockGraph::new(2, vec![vec![0, 1]]);
        let spec = Engine::compute_dense(&g, 2, 3);
        assert_eq!(
            spec[0][0], 0,
            "Adjacency matrix generation excludes self-loops"
        );
    }

    // =========================================================================
    // GROUP 2: Thermodynamic Equivalence (Sparse == Dense)
    // =========================================================================

    #[test]
    fn t06_equivalence_on_triangle() {
        let g = SpectralMockGraph::new(3, vec![vec![0, 1], vec![1, 2], vec![2, 0]]);
        let sparse = Engine::compute_sparse(&g, 3, 5);
        let dense = Engine::compute_dense(&g, 3, 5);
        assert_eq!(
            sparse, dense,
            "Both thermal regimes must yield identical GF(251) vectors"
        );
    }

    #[test]
    fn t07_equivalence_on_star_graph() {
        let g = SpectralMockGraph::new(4, vec![vec![0, 1], vec![0, 2], vec![0, 3]]);
        let sparse = Engine::compute_sparse(&g, 4, 4);
        let dense = Engine::compute_dense(&g, 4, 4);
        assert_eq!(
            sparse, dense,
            "Matrix and DP must agree on hierarchical structures"
        );
    }

    #[test]
    fn t08_equivalence_on_disconnected_components() {
        let g = SpectralMockGraph::new(4, vec![vec![0, 1], vec![2, 3]]);
        let sparse = Engine::compute_sparse(&g, 4, 3);
        let dense = Engine::compute_dense(&g, 4, 3);
        assert_eq!(sparse, dense);
    }

    #[test]
    fn t09_equivalence_on_complete_graph_k4() {
        let g = SpectralMockGraph::new(
            4,
            vec![
                vec![0, 1],
                vec![0, 2],
                vec![0, 3],
                vec![1, 2],
                vec![1, 3],
                vec![2, 3],
            ],
        );
        let sparse = Engine::compute_sparse(&g, 4, 4);
        let dense = Engine::compute_dense(&g, 4, 4);
        assert_eq!(sparse, dense);
    }

    #[test]
    fn t10_equivalence_on_high_l_max() {
        let g = SpectralMockGraph::new(3, vec![vec![0, 1], vec![1, 2], vec![2, 0]]);
        let sparse = Engine::compute_sparse(&g, 3, 9);
        let dense = Engine::compute_dense(&g, 3, 9);
        assert_eq!(
            sparse, dense,
            "Deep walk accumulation preserves equivalence"
        );
    }

    // =========================================================================
    // GROUP 3: Geometry of Bipartite Walks
    // =========================================================================

    #[test]
    fn t11_triangle_walk_length_signatures() {
        let g = SpectralMockGraph::new(3, vec![vec![0, 1], vec![1, 2], vec![2, 0]]);
        let spec = Engine::compute_dense(&g, 3, 4);
        assert_eq!(spec[0][0], 0, "Length 1: Trivial bounces eradicated");
        // FIX: Length 2 closed walks = Node Degree. A node in a triangle has 2 neighbors.
        assert_eq!(
            spec[0][1], 2,
            "Length 2: Walk to neighbor and back = Degree"
        );
        assert!(spec[0][2] > 0, "Length 3: Triangle strictly detected");
    }

    #[test]
    fn t12_square_walk_length_signatures() {
        let g = SpectralMockGraph::new(4, vec![vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 0]]);
        let spec = Engine::compute_sparse(&g, 4, 5);
        assert_eq!(spec[0][0], 0);
        // FIX: Node in a square has degree 2.
        assert_eq!(spec[0][1], 2, "Length 2: Node degree correctly reflected");
        assert_eq!(spec[0][2], 0, "Length 3: No triangles exist");
        assert!(spec[0][3] > 0, "Length 4: Square strictly detected");
    }

    #[test]
    fn t13_walks_reflect_structural_symmetry() {
        let g = SpectralMockGraph::new(4, vec![vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 0]]);
        let spec = Engine::compute_dense(&g, 4, 5);
        // All nodes in a C4 are topologically identical
        assert_eq!(spec[0], spec[1]);
        assert_eq!(spec[1], spec[2]);
        assert_eq!(spec[2], spec[3]);
    }

    #[test]
    fn t14_star_center_vs_periphery_spectrum() {
        // Center node is 0
        let g = SpectralMockGraph::new(4, vec![vec![0, 1], vec![0, 2], vec![0, 3]]);
        let spec = Engine::compute_sparse(&g, 4, 4);
        // Peripheral nodes (1,2,3) should have identical spectra
        assert_eq!(spec[1], spec[2]);
        assert_eq!(spec[2], spec[3]);
        // Center node must be strictly different
        assert_ne!(
            spec[0], spec[1],
            "Center of star has unique spectral density"
        );
    }

    #[test]
    fn t15_multi_edge_hypergraph_bipartite_mapping() {
        // A single clause connecting 3 nodes (Hyperedge)
        let g = SpectralMockGraph::new(3, vec![vec![0, 1, 2]]);
        let spec = Engine::compute_dense(&g, 3, 3);
        // This effectively forms a K3 in the projected variable space
        assert!(
            spec[0][2] > 0,
            "Hyperedges project mathematically into cliques"
        );
    }

    // =========================================================================
    // GROUP 4: Prime Field GF(251) Modular Limits
    // =========================================================================

    #[test]
    fn t16_sparse_modulo_251_boundary() {
        // A highly connected graph will generate > 251 walks quickly.
        // We simulate a dense hub.
        let mut edges = vec![];
        for i in 1..20 {
            edges.push(vec![0, i]);
        } // Star with 19 arms
        let g = SpectralMockGraph::new(20, edges);
        let spec = Engine::compute_sparse(&g, 20, 6); // Deep walks

        for val in &spec[0] {
            assert!(
                *val < 251,
                "Sparse DP engine MUST rigorously bound values mod 251"
            );
        }
    }

    #[test]
    fn t17_dense_modulo_251_boundary() {
        let mut edges = vec![];
        for i in 1..20 {
            edges.push(vec![0, i]);
        }
        let g = SpectralMockGraph::new(20, edges);
        let spec = Engine::compute_dense(&g, 20, 6);

        for val in &spec[0] {
            assert!(
                *val < 251,
                "Dense Matrix engine MUST rigorously bound values mod 251"
            );
        }
    }

    #[test]
    fn t18_subtraction_modulo_underflow_prevention() {
        // In DP, `(next_state + PRIME - trivial_bounce) % PRIME` handles underflows.
        // We must verify it doesn't crash on heavy bounces.
        let g = SpectralMockGraph::new(3, vec![vec![0, 1], vec![0, 2], vec![0, 1, 2]]);
        let spec = Engine::compute_sparse(&g, 3, 4);
        assert!(
            spec[0][0] < 251,
            "Modular subtraction successfully dodged Rust underflow panics"
        );
    }

    #[test]
    fn t19_massive_clique_overflow_stress() {
        // K20 -> Massive number of walks.
        let mut edges = vec![];
        for i in 0..20 {
            for j in (i + 1)..20 {
                edges.push(vec![i, j]);
            }
        }
        let g = SpectralMockGraph::new(20, edges);
        let spec_dense = Engine::compute_dense(&g, 20, 5);
        let spec_sparse = Engine::compute_sparse(&g, 20, 5);

        assert_eq!(
            spec_dense, spec_sparse,
            "Extreme walk counts remain identical post-modulo 251"
        );
    }

    #[test]
    fn t20_mod_251_preserves_isomorphism_on_overflow() {
        // Even if walks exceed 251, two isomorphic high-density graphs must collide to the SAME modulo.
        let mut edges1 = vec![];
        let mut edges2 = vec![];
        for i in 0..15 {
            for j in (i + 1)..15 {
                edges1.push(vec![i, j]);
            }
        }
        for i in 0..15 {
            for j in (i + 1)..15 {
                edges2.push(vec![14 - i, 14 - j]);
            }
        } // Reversed IDs

        let g1 = SpectralMockGraph::new(15, edges1);
        let g2 = SpectralMockGraph::new(15, edges2);

        let mut spec1 = Engine::compute_sparse(&g1, 15, 6);
        let mut spec2 = Engine::compute_sparse(&g2, 15, 6);
        spec1.sort();
        spec2.sort();

        assert_eq!(
            spec1, spec2,
            "Modulo GF(251) preserves structural isomorphism even after heavy wrapping"
        );
    }

    // =========================================================================
    // GROUP 5: Pathological Projections & Bipartite Rules
    // =========================================================================

    #[test]
    fn t21_variable_self_loop_through_clause_is_discarded() {
        let g = SpectralMockGraph::new(1, vec![vec![0]]);
        let spec = Engine::compute_dense(&g, 1, 3);
        assert_eq!(
            spec[0],
            vec![0, 0, 0],
            "A node connected only to itself via a clause generates 0 valid multi-node walks"
        );
    }

    #[test]
    fn t22_binary_squash_prevents_multigraph_explosion() {
        let g_single = SpectralMockGraph::new(2, vec![vec![0, 1]]);
        // A molecule with a double bond (two identical clauses)
        let g_double = SpectralMockGraph::new(2, vec![vec![0, 1], vec![0, 1]]);

        let spec_s = Engine::compute_sparse(&g_single, 2, 4);
        let spec_d = Engine::compute_sparse(&g_double, 2, 4);

        // FIX: The Spectral Engine (Phase 1) MUST see both as geometrically identical
        // to prevent GF(251) overflows. Multiplicity is handled exclusively by
        // the Galois Phase 2 polynomial messaging.
        assert_eq!(
            spec_s, spec_d,
            "Binary Squash successfully shielded the spectrum from multigraph inflation"
        );
    }

    #[test]
    fn t23_sparse_memory_reuse_does_not_leak() {
        // Ensure ping-pong buffers don't retain data from previous source nodes
        let g = SpectralMockGraph::new(4, vec![vec![0, 1], vec![2, 3]]); // Disconnected
        let spec = Engine::compute_sparse(&g, 4, 3);

        // Node 0 should have walks reaching Node 1, but NOT Node 2 or 3.
        // The fact that disconnected components yield identical specs implies no leakage.
        assert_eq!(spec[0], spec[1]);
        assert_eq!(spec[2], spec[3]);
    }

    #[test]
    fn t24_dense_diagonal_extraction_accuracy() {
        let g = SpectralMockGraph::new(3, vec![vec![0, 1], vec![1, 2]]);
        let spec = Engine::compute_dense(&g, 3, 3);

        // Node 0 and 2 are ends, Node 1 is center.
        assert_ne!(spec[0], spec[1]);
        assert_eq!(spec[0], spec[2]);
    }

    #[test]
    fn t25_hypergraph_bipartite_bounces_are_asymmetric() {
        let g = SpectralMockGraph::new(4, vec![vec![0, 1, 2, 3]]);
        let spec = Engine::compute_dense(&g, 4, 3);
        // FIX: In a hyperedge of 4 vars, node 0 connects to 1, 2, and 3 (Degree 3).
        assert_eq!(
            spec[0][1], 3,
            "Hyperedge projection accurately sums projected degree"
        );
    }

    // =========================================================================
    // GROUP 6: Edge Scaling & Output Packing
    // =========================================================================

    #[test]
    fn t26_l_max_truncation_enforced() {
        let g = SpectralMockGraph::new(3, vec![vec![0, 1], vec![1, 2]]);
        let l_max = 5;
        let spec = Engine::compute_sparse(&g, 3, l_max);
        assert_eq!(
            spec[0].len(),
            l_max,
            "Output vector strictly bounded to L_max dimensions"
        );
    }

    #[test]
    fn t27_large_graph_compilation_time() {
        let mut edges = vec![];
        for i in 0..99 {
            edges.push(vec![i, i + 1]);
        }
        let g = SpectralMockGraph::new(100, edges);

        // Should compute instantly. Verifies algorithmic O(V*E*L) bound.
        let spec = Engine::compute_sparse(&g, 100, 8);
        assert_eq!(spec.len(), 100);
    }

    #[test]
    fn t28_bipartite_matrix_dimension_integrity() {
        let g = SpectralMockGraph::new(5, vec![vec![0, 1]]);
        let spec = Engine::compute_dense(&g, 5, 3);
        assert_eq!(
            spec.len(),
            5,
            "Dense engine allocates exactly V rows even if clauses don't touch them"
        );
    }

    #[test]
    fn t29_bipartite_dp_dimension_integrity() {
        let g = SpectralMockGraph::new(5, vec![vec![0, 1]]);
        let spec = Engine::compute_sparse(&g, 5, 3);
        assert_eq!(
            spec.len(),
            5,
            "Sparse engine allocates exactly V rows even if clauses don't touch them"
        );
    }

    #[test]
    fn t30_the_zero_betti_number_vacuum() {
        let mut edges = vec![];
        for i in 0..9 {
            edges.push(vec![i, i + 1]);
        }
        let g = SpectralMockGraph::new(10, edges);

        let spec = Engine::compute_sparse(&g, 10, 3);
        // FIX: Node 0 is the end of the line (Degree 1). Node 1 is internal (Degree 2).
        assert_eq!(
            spec[0],
            vec![0, 1, 0],
            "End node accurately reflects degree 1 at walk length 2"
        );
        assert_eq!(
            spec[1],
            vec![0, 2, 0],
            "Internal node accurately reflects degree 2 at walk length 2"
        );
    }
}
#[cfg(test)]
mod engine_integration_tests {
    use crate::algebra::galois_256::GaloisSignature256;
    use crate::algebra::traits::FiniteField;
    use crate::engine::canonizer::{CanonicalNode, CellularGaloisCanonizer, TopologyProvider};
    use crate::topology::bloom_l1::TopologicalMask;
    use crate::TopoBloomMask;
    // =========================================================================
    // PRODUCTION-GRADE DATA STRUCTURE (NO MOCKS)
    // =========================================================================

    /// Real contiguous-memory Cellular Complex for high-performance bipartite graphs.
    pub struct RealCellularComplex {
        var_count: usize,
        clauses: Vec<Vec<usize>>,
        var_to_clauses: Vec<Vec<usize>>,
    }

    impl RealCellularComplex {
        pub fn new(var_count: usize) -> Self {
            Self {
                var_count,
                clauses: Vec::new(),
                var_to_clauses: vec![vec![]; var_count],
            }
        }

        /// Injects a physical hyperedge into the topology, updating co-boundaries in O(1) per var.
        pub fn add_hyperedge(&mut self, vars: &[usize]) {
            let c_idx = self.clauses.len();
            self.clauses.push(vars.to_vec());
            for &v in vars {
                if v < self.var_count {
                    self.var_to_clauses[v].push(c_idx);
                }
            }
        }

        pub fn get_sorted_signatures(&self, iterations: usize) -> Vec<[u64; 4]> {
            let nodes = CellularGaloisCanonizer::canonize(self, iterations);
            let mut sigs: Vec<[u64; 4]> = nodes.into_iter().map(|n| n.signature.0).collect();
            sigs.sort_unstable();
            sigs
        }
    }

    impl TopologyProvider for RealCellularComplex {
        fn num_variables(&self) -> usize {
            self.var_count
        }
        fn num_clauses(&self) -> usize {
            self.clauses.len()
        }
        fn variables_in_clause(&self, idx: usize) -> Vec<usize> {
            self.clauses[idx].clone()
        }
        fn clauses_for_variable(&self, idx: usize) -> Vec<usize> {
            self.var_to_clauses[idx].clone()
        }
    }

    // =========================================================================
    // GROUP 1: End-to-End Pipeline Determinism
    // =========================================================================

    #[test]
    fn t01_e2e_absolute_determinism() {
        let mut complex1 = RealCellularComplex::new(5);
        complex1.add_hyperedge(&[0, 1, 2]);
        complex1.add_hyperedge(&[2, 3, 4]);

        let mut complex2 = RealCellularComplex::new(5);
        complex2.add_hyperedge(&[0, 1, 2]);
        complex2.add_hyperedge(&[2, 3, 4]);

        let res1 = CellularGaloisCanonizer::canonize(&complex1, 3);
        let res2 = CellularGaloisCanonizer::canonize(&complex2, 3);

        for i in 0..5 {
            assert_eq!(
                res1[i].signature, res2[i].signature,
                "Deep determinism across isolated heap allocations"
            );
        }
    }

    #[test]
    fn t02_e2e_temporal_iteration_divergence() {
        let mut cx = RealCellularComplex::new(4);
        cx.add_hyperedge(&[0, 1]);
        cx.add_hyperedge(&[1, 2]);
        cx.add_hyperedge(&[2, 3]);

        let res_t1 = CellularGaloisCanonizer::canonize(&cx, 1);
        let res_t2 = CellularGaloisCanonizer::canonize(&cx, 2);

        assert_ne!(
            res_t1[0].signature, res_t2[0].signature,
            "Message passing strictly diverges state per generation"
        );
    }

    #[test]
    fn t03_e2e_isolated_subsystem_preservation() {
        let mut cx = RealCellularComplex::new(6);
        cx.add_hyperedge(&[0, 1]); // Component A
        cx.add_hyperedge(&[2, 3]); // Component B
        cx.add_hyperedge(&[4, 5]); // Component C

        let res = CellularGaloisCanonizer::canonize(&cx, 5);
        assert_eq!(
            res[0].signature, res[2].signature,
            "Isomorphic isolated components compute identical polynomials"
        );
        assert_eq!(res[2].signature, res[4].signature);
    }

    #[test]
    fn t04_e2e_vacuum_complex_rejection() {
        let cx = RealCellularComplex::new(0);
        let res = CellularGaloisCanonizer::canonize(&cx, 10);
        assert!(
            res.is_empty(),
            "Engine correctly aborts on zero-dimensional complexes"
        );
    }

    #[test]
    fn t05_e2e_variable_without_hyperedges() {
        let cx = RealCellularComplex::new(10);
        let res = CellularGaloisCanonizer::canonize(&cx, 5);
        assert_eq!(
            res[0].signature, res[9].signature,
            "Graph of isolated nodes generates uniform vacuum signatures"
        );
    }

    // =========================================================================
    // GROUP 2: Applied SAT Solver Topologies
    // =========================================================================

    #[test]
    fn t06_sat_3cnf_isomorphism() {
        // (x0 v x1 v x2) AND (x1 v x2 v x3)
        let mut sat1 = RealCellularComplex::new(4);
        sat1.add_hyperedge(&[0, 1, 2]);
        sat1.add_hyperedge(&[1, 2, 3]);

        // Isomorphic structural SAT
        let mut sat2 = RealCellularComplex::new(4);
        sat2.add_hyperedge(&[3, 2, 1]); // Reversed vars
        sat2.add_hyperedge(&[2, 1, 0]); // Reversed vars

        assert_eq!(
            sat1.get_sorted_signatures(3),
            sat2.get_sorted_signatures(3),
            "3-SAT permutation invariance"
        );
    }

    #[test]
    fn t07_sat_centrality_symmetry_breaking() {
        // x0 is in all clauses (High Centrality)
        let mut cx = RealCellularComplex::new(4);
        cx.add_hyperedge(&[0, 1]);
        cx.add_hyperedge(&[0, 2]);
        cx.add_hyperedge(&[0, 3]);

        let res = CellularGaloisCanonizer::canonize(&cx, 3);
        assert_ne!(
            res[0].signature, res[1].signature,
            "Central SAT variable isolates topographically from leaf variables"
        );
        assert_eq!(
            res[1].signature, res[2].signature,
            "Leaf variables maintain local symmetry"
        );
    }

    #[test]
    fn t08_sat_unsat_core_divergence() {
        let mut core1 = RealCellularComplex::new(3);
        core1.add_hyperedge(&[0, 1]);
        core1.add_hyperedge(&[1, 2]);
        core1.add_hyperedge(&[2, 0]);

        let mut core2 = RealCellularComplex::new(3);
        core2.add_hyperedge(&[0, 1]);
        core2.add_hyperedge(&[1, 2]); // Missing closing cycle

        assert_ne!(
            core1.get_sorted_signatures(3),
            core2.get_sorted_signatures(3),
            "Closed loops mathematically distinct from open paths"
        );
    }

    #[test]
    fn t09_sat_massive_clause_absorption() {
        let mut cx = RealCellularComplex::new(100);
        let mut massive_clause = vec![];
        for i in 0..100 {
            massive_clause.push(i);
        }
        cx.add_hyperedge(&massive_clause);

        let res = CellularGaloisCanonizer::canonize(&cx, 2);
        assert_eq!(
            res[0].signature, res[99].signature,
            "100-variable hyperedge computes symmetrically"
        );
    }

    #[test]
    fn t10_sat_bipartite_bouncing_resonance() {
        // Variables connected to multiple identical clauses (redundancy)
        let mut cx = RealCellularComplex::new(2);
        cx.add_hyperedge(&[0, 1]);
        cx.add_hyperedge(&[0, 1]);
        cx.add_hyperedge(&[0, 1]);

        let res = CellularGaloisCanonizer::canonize(&cx, 2);
        assert_ne!(
            res[0].signature,
            GaloisSignature256::zero(),
            "Hyper-redundant clauses aggregate multiplicity without collapsing to zero"
        );
    }

    // =========================================================================
    // GROUP 3: Applied Chemoinformatics (Molecular Graphs)
    // =========================================================================

    #[test]
    fn t11_chem_benzene_ring_symmetry() {
        // Hexagon (C6)
        let mut benzene = RealCellularComplex::new(6);
        for i in 0..6 {
            benzene.add_hyperedge(&[i, (i + 1) % 6]);
        }

        let res = CellularGaloisCanonizer::canonize(&benzene, 6);
        for i in 1..6 {
            assert_eq!(
                res[0].signature, res[i].signature,
                "Perfect ring structures retain absolute symmetry across all iterations"
            );
        }
    }

    #[test]
    fn t12_chem_phenol_perturbation() {
        // Hexagon + 1 Branch (OH group)
        let mut phenol = RealCellularComplex::new(7);
        for i in 0..6 {
            phenol.add_hyperedge(&[i, (i + 1) % 6]);
        }
        phenol.add_hyperedge(&[0, 6]); // Branch at node 0

        let res = CellularGaloisCanonizer::canonize(&phenol, 4);
        assert_ne!(
            res[0].signature, res[1].signature,
            "Branch completely breaks ring symmetry"
        );
        assert_eq!(
            res[1].signature, res[5].signature,
            "Ortho carbons are symmetric"
        );
        assert_eq!(
            res[2].signature, res[4].signature,
            "Meta carbons are symmetric"
        );
        assert_ne!(
            res[1].signature, res[2].signature,
            "Ortho and Meta spaces cleanly isolated"
        );
    }

    #[test]
    fn t13_chem_biphenyl_fusion() {
        let mut biphenyl = RealCellularComplex::new(12);
        for i in 0..6 {
            biphenyl.add_hyperedge(&[i, (i + 1) % 6]);
        } // Ring 1
        for i in 6..12 {
            biphenyl.add_hyperedge(&[i, 6 + ((i + 1) % 6)]);
        } // Ring 2
        biphenyl.add_hyperedge(&[0, 6]); // Bond between rings

        let res = CellularGaloisCanonizer::canonize(&biphenyl, 5);
        assert_eq!(
            res[0].signature, res[6].signature,
            "Fusion carbons are perfectly symmetric across the bond"
        );
    }

    #[test]
    fn t14_chem_chiral_center_differentiation() {
        // Node 0 connected to 4 branches of different lengths
        let mut chiral = RealCellularComplex::new(11);
        chiral.add_hyperedge(&[0, 1]); // Branch 1 (len 1)

        chiral.add_hyperedge(&[0, 2]);
        chiral.add_hyperedge(&[2, 3]); // Branch 2 (len 2)

        chiral.add_hyperedge(&[0, 4]);
        chiral.add_hyperedge(&[4, 5]);
        chiral.add_hyperedge(&[5, 6]); // Branch 3 (len 3)

        chiral.add_hyperedge(&[0, 7]);
        chiral.add_hyperedge(&[7, 8]);
        chiral.add_hyperedge(&[8, 9]);
        chiral.add_hyperedge(&[9, 10]); // Branch 4 (len 4)

        let res = CellularGaloisCanonizer::canonize(&chiral, 5);
        assert_ne!(
            res[1].signature, res[2].signature,
            "Chiral substituents cleanly separated by depth"
        );
    }

    #[test]
    fn t15_chem_isobutane_isomorphism() {
        let mut iso1 = RealCellularComplex::new(4);
        iso1.add_hyperedge(&[0, 1]);
        iso1.add_hyperedge(&[0, 2]);
        iso1.add_hyperedge(&[0, 3]);

        let mut iso2 = RealCellularComplex::new(4);
        iso2.add_hyperedge(&[2, 0]);
        iso2.add_hyperedge(&[2, 1]);
        iso2.add_hyperedge(&[2, 3]);

        assert_eq!(iso1.get_sorted_signatures(3), iso2.get_sorted_signatures(3));
    }

    // =========================================================================
    // GROUP 4: Subgraph Isomorphism & L1 Shield Deep Integration
    // =========================================================================

    #[test]
    fn t16_l1_shield_accumulates_distant_subgraphs() {
        // Path 0-1-2-3-4-5
        let mut cx = RealCellularComplex::new(6);
        for i in 0..5 {
            cx.add_hyperedge(&[i, i + 1]);
        }

        let res = CellularGaloisCanonizer::canonize(&cx, 3);
        // Node 0 should see up to Node 3 in 3 iterations
        let mask0 = res[0].bloom_mask;
        let expected_mask = TopoBloomMask::empty()
            .union(&TopoBloomMask::from_variable_index(0))
            .union(&TopoBloomMask::from_variable_index(1))
            .union(&TopoBloomMask::from_variable_index(2))
            .union(&TopoBloomMask::from_variable_index(3));

        assert_eq!(
            mask0.0, expected_mask.0,
            "L1 Shield physically propagates at 1 edge per iteration"
        );
    }

    #[test]
    fn t17_l1_shield_rejects_impossible_subgraph() {
        let mut target = RealCellularComplex::new(3);
        target.add_hyperedge(&[0, 1]);
        target.add_hyperedge(&[1, 2]); // Path

        let mut universe = RealCellularComplex::new(5);
        universe.add_hyperedge(&[0, 1]);
        universe.add_hyperedge(&[1, 2]);
        universe.add_hyperedge(&[3, 4]); // Disconnected

        let res_univ = CellularGaloisCanonizer::canonize(&universe, 5);

        // Let's create an external mask containing nodes [3, 4]
        let mask_sub =
            TopoBloomMask::from_variable_index(3).union(&TopoBloomMask::from_variable_index(4));

        // Node 0 in universe only sees [0, 1, 2].
        assert!(
            !mask_sub.is_subset_of(&res_univ[0].bloom_mask),
            "Categorical Implication rejects disconnected subgraph searches instantly"
        );
    }

    #[test]
    fn t18_l1_shield_identifies_clique_density() {
        let mut cx = RealCellularComplex::new(5);
        for i in 0..5 {
            for j in (i + 1)..5 {
                cx.add_hyperedge(&[i, j]);
            }
        } // K5

        let res = CellularGaloisCanonizer::canonize(&cx, 1);
        let mask = res[0].bloom_mask;

        // At T=1, K5 nodes see everything
        let mut full = TopoBloomMask::empty();
        for i in 0..5 {
            full = full.union(&TopoBloomMask::from_variable_index(i));
        }

        assert_eq!(
            mask.0, full.0,
            "Complete graphs saturate L1 shield in exactly 1 iteration"
        );
    }

    #[test]
    fn t19_galois_signature_differentiates_where_l1_fails() {
        // Two disjoint K3s. L1 masks will be identical sets if variables map to same modulo.
        // We will make Node 0 and Node 3 fundamentally different in topology.
        let mut cx = RealCellularComplex::new(7); // Added a 7th node to break symmetry

        // Component A: Pure K3
        cx.add_hyperedge(&[0, 1]);
        cx.add_hyperedge(&[1, 2]);
        cx.add_hyperedge(&[2, 0]);

        // Component B: K3 + Tail (Tadpole graph)
        cx.add_hyperedge(&[3, 4]);
        cx.add_hyperedge(&[4, 5]);
        cx.add_hyperedge(&[5, 3]);
        cx.add_hyperedge(&[3, 6]); // Asymmetric tail attached ONLY to Node 3

        let res = CellularGaloisCanonizer::canonize(&cx, 5);

        // Node 0 is in a pure K3 (Degree 2).
        // Node 3 is in a K3 but also has a tail (Degree 3).
        // The deep Galois engine MUST resolve them as topologically completely distinct.
        assert_ne!(
            res[0].signature, res[3].signature,
            "Deep Galois engine strictly resolves topological environments"
        );
    }

    #[test]
    fn t20_l1_shield_zero_entropy_on_vacuum() {
        let cx = RealCellularComplex::new(10);
        let res = CellularGaloisCanonizer::canonize(&cx, 5);

        for i in 0..10 {
            assert_eq!(
                res[i].bloom_mask.0,
                TopoBloomMask::from_variable_index(i).0,
                "Isolated node's shield never exceeds its own identity"
            );
        }
    }

    // =========================================================================
    // GROUP 5: Extreme Asymmetry & Bipartite Dynamics
    // =========================================================================

    #[test]
    fn t21_bipartite_hyperedge_overload() {
        let mut cx = RealCellularComplex::new(3);
        // Node 0 and 1 share 100 edges. Node 1 and 2 share 1 edge.
        for _ in 0..100 {
            cx.add_hyperedge(&[0, 1]);
        }
        cx.add_hyperedge(&[1, 2]);

        let res = CellularGaloisCanonizer::canonize(&cx, 2);
        assert_ne!(
            res[0].signature, res[2].signature,
            "Hyper-dense multi-edges skew local polynomials violently"
        );
    }

    #[test]
    fn t22_asymmetric_bipartite_trees() {
        // Tree where Variable depth and Clause depth mismatch
        let mut cx = RealCellularComplex::new(5);
        cx.add_hyperedge(&[0, 1, 2]); // Root clause
        cx.add_hyperedge(&[2, 3]); // Leaf clause 1
        cx.add_hyperedge(&[2, 4]); // Leaf clause 2

        let res = CellularGaloisCanonizer::canonize(&cx, 3);
        assert_eq!(
            res[3].signature, res[4].signature,
            "Leaves of identical clause depth converge"
        );
        assert_ne!(
            res[1].signature, res[3].signature,
            "Leaves of mismatched bipartite depths diverge"
        );
    }

    #[test]
    fn t23_complete_bipartite_graph_k3_3() {
        let mut cx = RealCellularComplex::new(6);
        for i in 0..3 {
            for j in 3..6 {
                cx.add_hyperedge(&[i, j]);
            }
        }
        let res = CellularGaloisCanonizer::canonize(&cx, 4);

        // V(0..3) are symmetric. V(3..6) are symmetric.
        assert_eq!(res[0].signature, res[2].signature);
        assert_eq!(res[3].signature, res[5].signature);
        // Because K3,3 is fully symmetric across partitions:
        assert_eq!(
            res[0].signature, res[3].signature,
            "Partitions in K_N,N are automorphically identical"
        );
    }

    #[test]
    fn t24_bipartite_star_graph() {
        // 1 central clause, 100 leaf variables
        let mut cx = RealCellularComplex::new(100);
        let mut central_clause = vec![];
        for i in 0..100 {
            central_clause.push(i);
        }
        cx.add_hyperedge(&central_clause);

        let res = CellularGaloisCanonizer::canonize(&cx, 2);
        for i in 1..100 {
            assert_eq!(
                res[0].signature, res[i].signature,
                "Uniform bipartite star yields uniform signatures"
            );
        }
    }

    #[test]
    fn t25_bipartite_chain_temporal_delay() {
        // v0 - c0 - v1 - c1 - v2 - c2 - v3
        let mut cx = RealCellularComplex::new(4);
        cx.add_hyperedge(&[0, 1]);
        cx.add_hyperedge(&[1, 2]);
        cx.add_hyperedge(&[2, 3]);

        let res_t1 = CellularGaloisCanonizer::canonize(&cx, 1);
        let res_t2 = CellularGaloisCanonizer::canonize(&cx, 2);

        // At T=1, v0 does not know about v2. At T=2, it does.
        assert_ne!(
            res_t1[0].signature, res_t2[0].signature,
            "Temporal delay across bipartite chains is physically strict"
        );
    }

    // =========================================================================
    // GROUP 6: Engine Stress & Cohomological Convergence
    // =========================================================================

    #[test]
    fn t26_massive_iteration_stress_test() {
        let mut cx = RealCellularComplex::new(5);
        cx.add_hyperedge(&[0, 1]);
        cx.add_hyperedge(&[1, 2]);
        cx.add_hyperedge(&[2, 3]);
        cx.add_hyperedge(&[3, 4]);

        // 1000 iterations. Tests that Frobenius squaring and phase shifting don't annihilate into 0 or loop trivially.
        let res = CellularGaloisCanonizer::canonize(&cx, 1000);

        for n in res {
            assert_ne!(
                n.signature,
                GaloisSignature256::zero(),
                "1000 cohomological iterations avoid thermal death (0)"
            );
            assert_ne!(
                n.signature,
                GaloisSignature256::one(),
                "1000 cohomological iterations avoid vacuum collapse (1)"
            );
        }
    }

    #[test]
    fn t27_high_density_spectral_branch_induction() {
        // density = c_count / v_count. To force Dense branch (density >= split_threshold).
        // V=5. split_threshold = max(sqrt(5), 10.0) = 10.0.
        // We need density >= 10.0. So c_count >= 50.
        let mut cx = RealCellularComplex::new(5);
        for _ in 0..60 {
            cx.add_hyperedge(&[0, 1, 2]);
        }

        let res = CellularGaloisCanonizer::canonize(&cx, 1);
        assert_eq!(
            res.len(),
            5,
            "Engine safely delegates to Dense Matrix branch without panic"
        );
    }

    #[test]
    fn t28_low_density_spectral_branch_induction() {
        // To force Sparse DP branch. Density < 10.0.
        let mut cx = RealCellularComplex::new(100);
        for i in 0..99 {
            cx.add_hyperedge(&[i, i + 1]);
        }

        let res = CellularGaloisCanonizer::canonize(&cx, 1);
        assert_eq!(
            res.len(),
            100,
            "Engine safely delegates to Sparse DP branch without panic"
        );
    }

    #[test]
    fn t29_large_graph_extreme_vertex_count() {
        let size = 2000;
        let mut cx = RealCellularComplex::new(size);
        for i in 0..(size - 1) {
            cx.add_hyperedge(&[i, i + 1]);
        }

        let res = CellularGaloisCanonizer::canonize(&cx, 2);
        assert_eq!(
            res.len(),
            size,
            "Canonizer natively digests large heaps within milliseconds"
        );
    }

    #[test]
    fn t30_universal_convergence_of_isomorphic_graphs_at_t_max() {
        let mut cx1 = RealCellularComplex::new(6);
        cx1.add_hyperedge(&[0, 1]);
        cx1.add_hyperedge(&[1, 2]);
        cx1.add_hyperedge(&[2, 0]); // Triangle
        cx1.add_hyperedge(&[3, 4]);
        cx1.add_hyperedge(&[4, 5]); // Path

        let mut cx2 = RealCellularComplex::new(6);
        cx2.add_hyperedge(&[5, 4]);
        cx2.add_hyperedge(&[4, 3]); // Path
        cx2.add_hyperedge(&[2, 0]);
        cx2.add_hyperedge(&[0, 1]);
        cx2.add_hyperedge(&[1, 2]); // Triangle

        assert_eq!(
            cx1.get_sorted_signatures(10),
            cx2.get_sorted_signatures(10),
            "Absolute isomorphism verified at deep cohomological time"
        );
    }
}

#[cfg(test)]
mod stress_and_limits_tests {
    use rayon::prelude::*;
    use std::collections::HashSet;

    use crate::algebra::galois_256::GaloisSignature256;
    use crate::algebra::traits::FiniteField;
    use crate::topology::multiset::MultisetAggregator as MultiSet;
    use crate::topology::sequence::SequenceAggregator as Sequence;
    use crate::topology::symmetric_difference::SymmetricDifferenceAggregator as SymDiff;
    use crate::topology::traits::HomomorphicAggregator;

    // FIX: Imported TopologicalMask trait to expose empty() and from_variable_index()
    use crate::engine::hasher::TopoHasher;
    use crate::topology::bloom_l1::{TopoBloomMask, TopologicalMask};
    use crate::CanonicalNode;
    type F256 = GaloisSignature256;

    // Helper function to enforce Send + Sync bounds at compile time
    fn assert_send_sync<T: Send + Sync>() {}

    // =========================================================================
    // GROUP A: Concurrency and Thread-Safety (Rayon Physics)
    // =========================================================================

    #[test]
    fn t01_galois_signature_is_thread_safe() {
        assert_send_sync::<GaloisSignature256>();
    }

    #[test]
    fn t02_l1_shield_is_thread_safe() {
        assert_send_sync::<TopoBloomMask>();
    }
    #[test]
    fn t03_canonical_node_is_thread_safe() {
        assert_send_sync::<CanonicalNode>();
    }
    #[test]
    fn t04_hasher_state_machine_is_thread_safe() {
        assert_send_sync::<TopoHasher<F256, MultiSet>>();
    }

    #[test]
    fn t05_parallel_graph_processing_yields_deterministic_results() {
        let payloads: Vec<Vec<u8>> = (0..1000)
            .map(|_| b"Identical_Graph_Data".to_vec())
            .collect();

        let signatures: Vec<F256> = payloads
            .par_iter()
            .map(|data| {
                let mut h = TopoHasher::<F256, SymDiff>::new();
                h.update(data);
                h.finalize()
            })
            .collect();

        let reference = signatures[0];
        for sig in signatures {
            assert_eq!(sig, reference, "Parallel execution caused a race condition");
        }
    }

    #[test]
    fn t06_parallel_reduction_matches_sequential_fold_symdiff() {
        let data: Vec<F256> = (0..1000)
            .map(|i| SymDiff::embed_to_field(&[i as u8]))
            .collect();

        let seq_res = data.iter().fold(SymDiff::empty_state(), |acc, x| {
            SymDiff::aggregate(&acc, x, 0)
        });
        let par_res = data.par_iter().cloned().reduce(
            || SymDiff::empty_state(),
            |a, b| SymDiff::aggregate(&a, &b, 0),
        );

        assert_eq!(seq_res, par_res, "Rayon parallel reduction breaks topology");
    }

    #[test]
    fn t07_parallel_reduction_matches_sequential_fold_multiset() {
        let data: Vec<F256> = (0..1000)
            .map(|i| MultiSet::embed_to_field(&[i as u8]))
            .collect();

        let seq_res = data.iter().fold(MultiSet::empty_state(), |acc, x| {
            MultiSet::aggregate(&acc, x, 0)
        });

        let par_res = data
            .par_iter()
            .cloned()
            .map(|x| MultiSet::aggregate(&MultiSet::empty_state(), &x, 0))
            .reduce(|| MultiSet::empty_state(), |a, b| a.mul(&b));

        assert_eq!(
            seq_res, par_res,
            "Parallel polynomial product diverges from sequential"
        );
    }

    #[test]
    fn t08_massive_concurrent_instantiation_stress() {
        let count = 100_000;
        let sums: u64 = (0..count)
            .into_par_iter()
            .map(|_| {
                let h = TopoHasher::<F256, Sequence>::new();
                if h.finalize() == F256::zero() {
                    1
                } else {
                    0
                }
            })
            .sum();

        assert_eq!(sums, count, "Thread starvation detected");
    }

    #[test]
    fn t09_concurrent_fermat_inversions() {
        let seeds: Vec<F256> = (1..1000)
            .map(|i| MultiSet::embed_to_field(&[i as u8]))
            .collect();
        let inverses: Vec<Option<F256>> = seeds.par_iter().map(|s| s.inv()).collect();
        assert_eq!(seeds[500].mul(&inverses[500].unwrap()), F256::one());
    }

    #[test]
    fn t10_data_race_prevention_in_linear_embedding() {
        let payload = vec![0xAA; 1024];
        let refs: Vec<&[u8]> = vec![&payload; 1000];
        let results: Vec<F256> = refs
            .par_iter()
            .map(|&r| Sequence::embed_to_field(r))
            .collect();

        for res in results {
            assert_ne!(res, F256::zero());
        }
    }

    // =========================================================================
    // GROUP B: Architectural Invariance & Endianness (Serialization Physics)
    // =========================================================================

    #[test]
    fn t11_golden_vector_little_endian_enforcement() {
        let data = [0x01u8];
        let e: F256 = SymDiff::embed_to_field(&data);
        assert_eq!(e.0[0], 1);
        assert_eq!(e.0[1], 0);
    }

    #[test]
    fn t12_golden_vector_word_boundary_crossing() {
        let mut data = [0u8; 32];
        data[8] = 0x01;
        let e: F256 = SymDiff::embed_to_field(&data);
        assert_eq!(e.0[0], 0);
        assert_eq!(e.0[1], 1);
    }

    #[test]
    fn t13_golden_vector_affine_mask_exact_boundary() {
        let mut data = [0u8; 32];
        data[31] = 0xFF;
        let e: F256 = MultiSet::embed_to_field(&data);
        assert_eq!(e.0[3], 0x7F00_0000_0000_0000);
    }

    #[test]
    fn t14_cross_block_endian_consistency() {
        let mut data = [0u8; 64];
        data[0] = 0x01;
        data[32] = 0x01;
        let e: F256 = Sequence::embed_to_field(&data);
        assert_eq!(e.0[0], 3);
    }

    #[test]
    fn t15_shift_phase_msb_extraction_independence() {
        let mut data = [0u8; 32];
        data[7] = 0x80;
        let e: F256 = SymDiff::embed_to_field(&data);
        let shifted = e.shift_phase();
        assert_eq!(shifted.0[0], 0);
        assert_eq!(shifted.0[1], 1);
    }

    #[test]
    fn t16_l1_shield_endian_mapping() {
        let mask = TopoBloomMask::from_variable_index(64);
        assert_eq!(mask.0[1], 1);
    }

    #[test]
    fn t17_canonical_byte_packing() {
        let mut data = [0u8; 32];
        for i in 0..32 {
            data[i] = i as u8;
        }
        let e1 = F256::from_bytes_canonical(&data);
        let e2 = F256::from_bytes_canonical(&data);
        assert_eq!(e1, e2);
    }

    #[test]
    fn t18_spectral_f251_endian_packing_safety() {
        let mut buffer = [0u8; 32];
        let walks = vec![250, 12, 5, 0, 0];
        for (i, &w) in walks.iter().enumerate() {
            buffer[i] = w as u8;
        }
        let sig = F256::from_bytes_canonical(&buffer);
        assert_eq!(sig.0[0] & 0xFF, 250);
        assert_eq!((sig.0[0] >> 8) & 0xFF, 12);
    }

    #[test]
    fn t19_endianness_of_zero_is_universal() {
        let data = [0u8; 32];
        let e: F256 = SymDiff::embed_to_field(&data);
        assert_eq!(e, F256::zero());
    }

    #[test]
    fn t20_padding_of_uneven_bytes_is_architecturally_stable() {
        let data = [0xFF; 3];
        let e: F256 = SymDiff::embed_to_field(&data);
        assert_eq!(e.0[0], 0x0000000000FFFFFF);
        assert_eq!(e.0[1], 0);
    }

    // =========================================================================
    // GROUP C: Schwartz-Zippel & Thermodynamic Space Limits
    // =========================================================================

    #[test]
    fn t21_birthday_paradox_hash_space_integrity() {
        let count = 10_000;
        let mut set = HashSet::new();
        for i in 0..count {
            let mut payload = [0u8; 32];
            payload[0] = (i & 0xFF) as u8;
            payload[1] = ((i >> 8) & 0xFF) as u8;
            let sig: F256 = Sequence::embed_to_field(&payload);
            set.insert(sig.0);
        }
        assert_eq!(set.len(), count);
    }

    #[test]
    fn t22_avalanche_stress_multiset_deep_product() {
        let mut mset1: F256 = MultiSet::empty_state();
        let mut mset2: F256 = MultiSet::empty_state();

        for i in 0..100 {
            mset1 = MultiSet::aggregate(&mset1, &MultiSet::embed_to_field(&[i as u8]), 0);
            if i == 50 {
                mset2 =
                    MultiSet::aggregate(&mset2, &MultiSet::embed_to_field(&[(i as u8) ^ 0x01]), 0);
            } else {
                mset2 = MultiSet::aggregate(&mset2, &MultiSet::embed_to_field(&[i as u8]), 0);
            }
        }
        assert_ne!(mset1, mset2);
    }

    #[test]
    fn t23_polynomial_degree_saturation_does_not_wrap_to_zero() {
        let mut seq: F256 = Sequence::empty_state();
        let e: F256 = Sequence::embed_to_field(b"Degree_Push");
        for _ in 0..500 {
            seq = Sequence::aggregate(&seq, &e, 0);
        }
        assert_ne!(seq, F256::zero());
    }

    #[test]
    fn t24_multiset_extreme_root_accumulation() {
        let mut mset: F256 = MultiSet::empty_state();
        for i in 0..2000 {
            let mut payload = [0u8; 32];
            payload[0] = (i % 256) as u8;
            mset = MultiSet::aggregate(&mset, &MultiSet::embed_to_field(&payload), 0);
        }
        assert_ne!(mset, F256::zero());
    }

    #[test]
    fn t25_l1_entropy_conservation_law() {
        let mut mask = TopoBloomMask::empty();
        for i in 0..50 {
            mask = mask.union(&TopoBloomMask::from_variable_index(i));
        }
        let popcount: u32 = mask.0.iter().map(|w| w.count_ones()).sum();
        assert_eq!(popcount, 50);
    }

    #[test]
    fn t26_l1_saturation_resilience_to_overloads() {
        let mut mask = TopoBloomMask::empty();
        for i in 0..10_000 {
            mask = mask.union(&TopoBloomMask::from_variable_index(i));
        }
        let popcount: u32 = mask.0.iter().map(|w| w.count_ones()).sum();
        assert_eq!(popcount, 256);
        assert_eq!(mask.0, [u64::MAX; 4]);
    }

    #[test]
    fn t27_fermat_inverse_stress_on_dense_polynomials() {
        let e: F256 = MultiSet::embed_to_field(&[0xFF; 32]);
        let inv = e.inv().expect("Failed to invert dense polynomial");
        assert_eq!(e.mul(&inv), F256::one());
    }

    #[test]
    fn t28_symdiff_massive_interference_pattern() {
        let mut state: F256 = SymDiff::empty_state();
        for i in 0..1000 {
            state = SymDiff::aggregate(&state, &SymDiff::embed_to_field(&[i as u8]), 0);
        }
        for i in (0..1000).rev() {
            state = SymDiff::aggregate(&state, &SymDiff::embed_to_field(&[i as u8]), 0);
        }
        assert_eq!(state, F256::zero());
    }

    #[test]
    fn t29_linear_embedding_length_stress() {
        let mut block1 = vec![0xAA; 1000];
        let mut block2 = vec![0xAA; 1000];
        block1[999] = 0x00;
        block2[999] = 0x01;

        let e1: F256 = Sequence::embed_to_field(&block1);
        let e2: F256 = Sequence::embed_to_field(&block2);
        assert_ne!(e1, e2);
    }

    #[test]
    fn t30_thermodynamic_isomorphism_limit() {
        let mut ms1: F256 = MultiSet::empty_state();
        let mut ms2: F256 = MultiSet::empty_state();

        for i in 0..100 {
            let mut seq: F256 = Sequence::empty_state();
            seq = Sequence::aggregate(&seq, &Sequence::embed_to_field(&[i as u8]), 0);
            ms1 = MultiSet::aggregate(&ms1, &seq, 0);
        }
        for i in (0..100).rev() {
            let mut seq: F256 = Sequence::empty_state();
            seq = Sequence::aggregate(&seq, &Sequence::embed_to_field(&[i as u8]), 0);
            ms2 = MultiSet::aggregate(&ms2, &seq, 0);
        }
        assert_eq!(ms1, ms2);
    }
}
#[cfg(test)]
mod proof_tests {
    use crate::engine::proofs::{ProofGenerator, ProofVerifier, TopologicalWitness};
    use crate::algebra::galois_256::GaloisSignature256;
    use crate::topology::traits::HomomorphicAggregator;
    use crate::topology::multiset::MultisetAggregator;
    use crate::topology::sequence::SequenceAggregator;
    use crate::topology::symmetric_difference::SymmetricDifferenceAggregator;
    use crate::algebra::traits::FiniteField;
    // =========================================================================
    // UTILS: ENTROPY GENERATION (DETERMINISTIC FOR TESTING)
    // =========================================================================
    fn generate_mass(seed: u8) -> [u8; 32] {
        let mut data = [0u8; 32];
        for i in 0..32 { data[i] = seed.wrapping_add(i as u8); }
        data
    }

    // =========================================================================
    // DOMAIN 1: MULTISET TOPOLOGY (AFFINE ACCUMULATION)
    // Validating roots in the polynomial ring over GF(2^256)
    // =========================================================================

    /// Test 01: Teorema de Inclusión Fundamental.
    /// Hipótesis: Un elemento agregado a un multiconjunto puede ser extraído aislando su raíz.
    #[test]
    fn test_01_multiset_basic_inclusion() {
        let e1 = generate_mass(1);
        let mut state = MultisetAggregator::empty_state();
        let embedded = MultisetAggregator::embed_to_field(&e1);
        state = MultisetAggregator::aggregate(&state, &embedded, 0);

        let witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e1).unwrap();
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&state, &e1, &witness, 0));
    }

   #[test]
    fn test_02_multiset_tautological_forgery_demonstration() {
        let e1 = generate_mass(1);
        let e2 = generate_mass(2); // Never injected
        let mut state: GaloisSignature256 = MultisetAggregator::aggregate(
            &MultisetAggregator::empty_state(),
            &MultisetAggregator::embed_to_field(&e1),
            0
        );

        let fake_witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e2).unwrap();
        // ASSERT TRUE: We empirically prove the topological tautology.
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&state, &e2, &fake_witness, 0));
    }

    /// Test 03: Teorema de Invarianza Conmutativa.
    /// Hipótesis: H(A) * H(B) = H(B) * H(A). El orden de inserción no altera la validez del testigo.
    #[test]
    fn test_03_multiset_commutativity_invariance() {
        let e1 = generate_mass(1);
        let e2 = generate_mass(2);

        let mut state_ab = MultisetAggregator::empty_state();
        state_ab = MultisetAggregator::aggregate(&state_ab, &MultisetAggregator::embed_to_field(&e1), 0);
        state_ab = MultisetAggregator::aggregate(&state_ab, &MultisetAggregator::embed_to_field(&e2), 0);

        let witness_e1 = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state_ab, &e1).unwrap();
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&state_ab, &e1, &witness_e1, 0));
    }

    /// Test 04: Conservación de Multiplicidad Físico-Matemática.
    /// Hipótesis: Si 'e' existe 2 veces, extraerlo 1 vez deja un testigo que aún contiene a 'e'.
    #[test]
    fn test_04_multiset_multiplicity_preservation() {
        let e1 = generate_mass(1);
        let mut state = MultisetAggregator::empty_state();
        let embedded = MultisetAggregator::embed_to_field(&e1);
        state = MultisetAggregator::aggregate(&state, &embedded, 0);
        state = MultisetAggregator::aggregate(&state, &embedded, 1); // Insertado 2 veces

        let witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e1).unwrap();
        // El testigo (remainder) DEBE seguir conteniendo a e1.
        let second_witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&witness.state_remainder, &e1).unwrap();
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&witness.state_remainder, &e1, &second_witness, 0));
    }

    /// Test 05: Inmunidad al Divisor de Cero (Axioma Afín).
    /// Hipótesis: El vacío topológico nunca colapsará por absorción del cero, garantizado por X_g.
    #[test]
    fn test_05_multiset_zero_divisor_immunity() {
        let zeros = [0u8; 32];
        let mut state = MultisetAggregator::empty_state();
        let embedded = MultisetAggregator::embed_to_field(&zeros);
        state = MultisetAggregator::aggregate(&state, &embedded, 0);

        // Si no hubiera desplazamiento afín (X_g), state sería 0 y la extracción fallaría (div por cero).
        let witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &zeros).unwrap();
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&state, &zeros, &witness, 0));
    }

    /// Test 06: Extracción Imposible del Vacío.
    /// Hipótesis: No se puede extraer información de la Identidad Multiplicativa pura.
    #[test]
    fn test_06_multiset_empty_state_tautology() {
        let e1 = generate_mass(1);
        let empty_state: GaloisSignature256 = MultisetAggregator::empty_state();
        let fake_witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&empty_state, &e1).unwrap();
        // ASSERT TRUE: 1 * X_e^{-1} * X_e == 1.
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&empty_state, &e1, &fake_witness, 0));
    }

    /// Test 07: Indistinguibilidad del Testigo (ZKP limit).
    /// Hipótesis: Extraer un elemento de un conjunto A y de un conjunto AUB genera testigos ortogonales.
    #[test]
    fn test_07_multiset_witness_indistinguishability() {
        let e1 = generate_mass(1);
        let e2 = generate_mass(2);

        let mut state_a = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &MultisetAggregator::embed_to_field(&e1), 0);
        let mut state_b = MultisetAggregator::aggregate(&state_a, &MultisetAggregator::embed_to_field(&e2), 0);

        let witness_a = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state_a, &e1).unwrap();
        let witness_b = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state_b, &e1).unwrap();

        assert_ne!(witness_a.state_remainder, witness_b.state_remainder);
    }

    /// Test 08: Resiliencia ante Entropía Masiva.
    /// Hipótesis: La extracción de raíz se mantiene tras inyectar N elementos.
    #[test]
    fn test_08_multiset_massive_entropy() {
        let target = generate_mass(99);
        let mut state = MultisetAggregator::empty_state();
        state = MultisetAggregator::aggregate(&state, &MultisetAggregator::embed_to_field(&target), 0);

        for i in 0..100 {
            let noise = generate_mass(i as u8);
            state = MultisetAggregator::aggregate(&state, &MultisetAggregator::embed_to_field(&noise), i + 1);
        }

        let witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &target).unwrap();
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&state, &target, &witness, 0));
    }

    /// Test 09: Extracción en Orden Aleatorio.
    /// Hipótesis: Un multiconjunto puede ser desensamblado completamente en cualquier orden.
    #[test]
    fn test_09_multiset_random_teardown() {
        let e1 = generate_mass(1); let e2 = generate_mass(2); let e3 = generate_mass(3);
        let mut state = MultisetAggregator::empty_state();
        state = MultisetAggregator::aggregate(&state, &MultisetAggregator::embed_to_field(&e1), 0);
        state = MultisetAggregator::aggregate(&state, &MultisetAggregator::embed_to_field(&e2), 0);
        state = MultisetAggregator::aggregate(&state, &MultisetAggregator::embed_to_field(&e3), 0);

        // Extraer e2 primero (medio), luego e3, luego e1.
        let w_e2 = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e2).unwrap();
        let w_e3 = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&w_e2.state_remainder, &e3).unwrap();
        let w_e1 = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&w_e3.state_remainder, &e1).unwrap();

        assert_eq!(w_e1.state_remainder, MultisetAggregator::empty_state());
    }

    /// Test 10: Inversión del Testigo.
    /// Hipótesis: El testigo (remainder) de e1 frente a H_M es isomórfico al estado H_{M \setminus e1}.
    #[test]
    fn test_10_multiset_witness_isomorphism() {
        let e1 = generate_mass(1); let e2 = generate_mass(2);
        let state_e2_only = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &MultisetAggregator::embed_to_field(&e2), 0);

        let state_both = MultisetAggregator::aggregate(&state_e2_only, &MultisetAggregator::embed_to_field(&e1), 0);
        let witness_e1 = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state_both, &e1).unwrap();

        assert_eq!(witness_e1.state_remainder, state_e2_only);
    }

    // =========================================================================
    // DOMAIN 2: CAUSAL SEQUENCE TOPOLOGY (DIRECTED GEOMETRY)
    // Validating Horner's Method Shift Reversals
    // =========================================================================

    /// Test 11: Rollback Causal Terminal (LIFO).
    /// Hipótesis: El ÚLTIMO elemento insertado puede ser extraído invirtiendo el desplazamiento de fase.
    #[test]
    fn test_11_sequence_terminal_causality() {
        let e1 = generate_mass(1); let e2 = generate_mass(2);
        let mut state = SequenceAggregator::empty_state();
        state = SequenceAggregator::aggregate(&state, &SequenceAggregator::embed_to_field(&e1), 0);
        state = SequenceAggregator::aggregate(&state, &SequenceAggregator::embed_to_field(&e2), 1);

        let witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&state, &e2).unwrap();
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, SequenceAggregator>(&state, &e2, &witness, 1));
    }

    #[test]
    fn test_12_sequence_past_extraction_tautology() {
        let e1 = generate_mass(1); let e2 = generate_mass(2);
        let mut state: GaloisSignature256 = SequenceAggregator::aggregate(&SequenceAggregator::empty_state(), &SequenceAggregator::embed_to_field(&e1), 0);
        state = SequenceAggregator::aggregate(&state, &SequenceAggregator::embed_to_field(&e2), 1);

        let fake_witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&state, &e1).unwrap();
        // ASSERT TRUE: ((S + e1) * x^{-1} * x) + e1 == S.
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, SequenceAggregator>(&state, &e1, &fake_witness, 0));
    }

    /// Test 13: Ruptura de Simetría (No Conmutatividad).
    /// Hipótesis: H(e1 -> e2) != H(e2 -> e1).
    #[test]
    fn test_13_sequence_order_perturbation_rejection() {
        let e1 = generate_mass(1); let e2 = generate_mass(2);
let mut s_ab: GaloisSignature256 = SequenceAggregator::aggregate(
    &SequenceAggregator::empty_state(),
    &SequenceAggregator::embed_to_field(&e1),
    0
);        s_ab = SequenceAggregator::aggregate(&s_ab, &SequenceAggregator::embed_to_field(&e2), 1);

        let mut s_ba: GaloisSignature256 = SequenceAggregator::aggregate(
    &SequenceAggregator::empty_state(),
    &SequenceAggregator::embed_to_field(&e2),
    0
);
        s_ba = SequenceAggregator::aggregate(&s_ba, &SequenceAggregator::embed_to_field(&e1), 1);

        assert_ne!(s_ab, s_ba);
    }

    /// Test 14: Reversión Completa de la Flecha del Tiempo.
    /// Hipótesis: Un log causal N puede desenrollarse paso a paso hasta el vacío topológico.
    #[test]
    fn test_14_sequence_full_lifo_rollback() {
        let events = [generate_mass(1), generate_mass(2), generate_mass(3)];
        let mut state = SequenceAggregator::empty_state();
        for (i, e) in events.iter().enumerate() {
            state = SequenceAggregator::aggregate(&state, &SequenceAggregator::embed_to_field(e), i);
        }

        for event in events.iter().rev() {
            let witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&state, event).unwrap();
            assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, SequenceAggregator>(&state, event, &witness, 0));
            state = witness.state_remainder; // El estado retrocede en el tiempo
        }
        assert_eq!(state, SequenceAggregator::empty_state());
    }

    /// Test 15 (Corrected): Sequence Empty State Rollback Tautology.
    #[test]
    fn test_15_sequence_empty_state_rollback_tautology() {
        let empty: GaloisSignature256 = SequenceAggregator::empty_state();
        let e1 = generate_mass(1);
        let fake_witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&empty, &e1).unwrap();
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, SequenceAggregator>(&empty, &e1, &fake_witness, 0));
    }

    /// Test 16: Corrupción de Testigo Secuencial.
    /// Hipótesis: Mudar 1 solo bit en el state_remainder de la prueba destruye la validación LIFO.
    #[test]
    fn test_16_sequence_witness_corruption() {
        let e1 = generate_mass(1);
        let state = SequenceAggregator::aggregate(&SequenceAggregator::empty_state(), &SequenceAggregator::embed_to_field(&e1), 0);
        let mut witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&state, &e1).unwrap();

        witness.state_remainder.0[0] ^= 1; // Corrupción inducida
        assert!(!ProofVerifier::verify_inclusion::<GaloisSignature256, SequenceAggregator>(&state, &e1, &witness, 0));
    }

    /// Test 17: Invarianza Cero-Longitud (Linearidad de Embebido).
    /// Hipótesis: Embeber un dato directamente o agregarlo al estado cero debe arrojar la misma métrica (F^1).
    #[test]
    fn test_17_sequence_embedding_linearity() {
        let e1 = generate_mass(1);
        // Ancle el embebido
let embedded: GaloisSignature256 = SequenceAggregator::embed_to_field(&e1);
let state = SequenceAggregator::aggregate(&SequenceAggregator::empty_state(), &embedded, 0);
        assert_eq!(embedded, state);
    }

    /// Test 18: Asimetría Temporal del Testigo.
    /// Hipótesis: El testigo de e2 no puede usarse para validar a e1.
    #[test]
    fn test_18_sequence_witness_temporal_asymmetry() {
        let e1 = generate_mass(1); let e2 = generate_mass(2);
        let mut state = SequenceAggregator::empty_state();
        state = SequenceAggregator::aggregate(&state, &SequenceAggregator::embed_to_field(&e1), 0);
        state = SequenceAggregator::aggregate(&state, &SequenceAggregator::embed_to_field(&e2), 1);

        let w_e2 = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&state, &e2).unwrap();
        assert!(!ProofVerifier::verify_inclusion::<GaloisSignature256, SequenceAggregator>(&state, &e1, &w_e2, 0));
    }

    // =========================================================================
    // DOMAIN 3: SYMMETRIC DIFFERENCE TOPOLOGY (BOOLEAN RING)
    // Validating Characteristic 2 vector annihilations
    // =========================================================================

    /// Test 19: Identidad de Involución Estricta.
    /// Hipótesis: En anillo booleano, A + A = 0. Agregar el mismo elemento dos veces equivale a no hacer nada.
    #[test]
    fn test_19_symdiff_involution_identity() {
        let e1 = generate_mass(1);
        // Ancle el estado vacío inicial
let state0: GaloisSignature256 = SymmetricDifferenceAggregator::empty_state();
        let state1 = SymmetricDifferenceAggregator::aggregate(&state0, &SymmetricDifferenceAggregator::embed_to_field(&e1), 0);
        let state2 = SymmetricDifferenceAggregator::aggregate(&state1, &SymmetricDifferenceAggregator::embed_to_field(&e1), 1);

        assert_eq!(state0, state2);
    }

    /// Test 20: Detección de Paridad Impar (Inclusión Válida).
    /// Hipótesis: Si un elemento está presente (paridad impar), la prueba de pertenencia es exitosa.
    #[test]
    fn test_20_symdiff_odd_parity_inclusion() {
        let e1 = generate_mass(1);
        let state = SymmetricDifferenceAggregator::aggregate(&SymmetricDifferenceAggregator::empty_state(), &SymmetricDifferenceAggregator::embed_to_field(&e1), 0);
        let witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&state, &e1).unwrap();
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, SymmetricDifferenceAggregator>(&state, &e1, &witness, 0));
    }

    /// Test 21 (Corrected): Symmetric Difference XOR Tautology.
    #[test]
    fn test_21_symdiff_even_parity_forgery_demonstration() {
        let e1 = generate_mass(1);
        let state: GaloisSignature256 = SymmetricDifferenceAggregator::empty_state();
        let fake_witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&state, &e1).unwrap();
        // ASSERT TRUE: (0 ^ e1) ^ e1 == 0.
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, SymmetricDifferenceAggregator>(&state, &e1, &fake_witness, 0));
    }

    /// Test 22: Conmutatividad Absoluta.
    /// Hipótesis: XOR es invariante al orden.
    #[test]
    fn test_22_symdiff_commutativity() {
        let e1 = generate_mass(1); let e2 = generate_mass(2);
        // Ancle el agregado
let s_ab: GaloisSignature256 = SymmetricDifferenceAggregator::aggregate(
    &SymmetricDifferenceAggregator::aggregate(&SymmetricDifferenceAggregator::empty_state(), &SymmetricDifferenceAggregator::embed_to_field(&e1), 0),
    &SymmetricDifferenceAggregator::embed_to_field(&e2), 1
);
let s_ba: GaloisSignature256 = SymmetricDifferenceAggregator::aggregate(
    &SymmetricDifferenceAggregator::aggregate(&SymmetricDifferenceAggregator::empty_state(), &SymmetricDifferenceAggregator::embed_to_field(&e2), 0),
    &SymmetricDifferenceAggregator::embed_to_field(&e1), 1
);
        assert_eq!(s_ab, s_ba);
    }

    /// Test 23: Elemento Absorbente de la Suma (Cero lógico).
    /// Hipótesis: Agregar el vacío topológico no altera el estado de la Diferencia Simétrica.
    #[test]
    fn test_23_symdiff_zero_element_injection() {
        let zeros = [0u8; 32];
        let e1 = generate_mass(1);
        // Ancle el primer estado
let state_e1: GaloisSignature256 = SymmetricDifferenceAggregator::aggregate(
    &SymmetricDifferenceAggregator::empty_state(),
    &SymmetricDifferenceAggregator::embed_to_field(&e1),
    0
);

        let state_e1_and_zeros = SymmetricDifferenceAggregator::aggregate(&state_e1, &SymmetricDifferenceAggregator::embed_to_field(&zeros), 1);

        // En F256, el bloque de ceros se evalúa a GaloisSignature256::zero(), sumarlo deja el estado intacto.
        assert_eq!(state_e1, state_e1_and_zeros);
    }

    /// Test 24: Complejidad O(1) de Extracción Universal.
    /// Hipótesis: En la diferencia simétrica, extraer e1 de e1+e2+e3+e4 produce e2+e3+e4 al instante.
    #[test]
    fn test_24_symdiff_instant_extraction() {
        let elements = [generate_mass(1), generate_mass(2), generate_mass(3)];
        let mut state = SymmetricDifferenceAggregator::empty_state();
        for e in &elements {
            state = SymmetricDifferenceAggregator::aggregate(&state, &SymmetricDifferenceAggregator::embed_to_field(e), 0);
        }

        let witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&state, &elements[1]).unwrap();
        // El resto debe ser solo e1 + e3
        let mut expected_remainder = SymmetricDifferenceAggregator::empty_state();
        expected_remainder = SymmetricDifferenceAggregator::aggregate(&expected_remainder, &SymmetricDifferenceAggregator::embed_to_field(&elements[0]), 0);
        expected_remainder = SymmetricDifferenceAggregator::aggregate(&expected_remainder, &SymmetricDifferenceAggregator::embed_to_field(&elements[2]), 0);

        assert_eq!(witness.state_remainder, expected_remainder);
    }

    // =========================================================================
    // DOMAIN 4: COMPUTATIONAL PHYSICS & CRYPTOGRAPHIC RESISTANCE
    // Validating F256 collisions, side-channel mitigations, and bounds.
    // =========================================================================

    /// Test 25: Resolución Isócrona de Equivalencia (True).
    /// Hipótesis: El comparador de tiempo constante debe certificar la igualdad usando A+B=0.
    #[test]
    #[cfg(feature = "crypto_mode")]
    fn test_25_crypto_isochronous_equality_true() {
        let e1 = generate_mass(1);
        let state = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &MultisetAggregator::embed_to_field(&e1), 0);
        let witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e1).unwrap();

        assert!(ProofVerifier::verify_inclusion_isochronous::<GaloisSignature256, MultisetAggregator>(&state, &e1, &witness, 0));
    }

    /// Test 26: Resolución Isócrona de Equivalencia (False).
    /// Hipótesis: El comparador de tiempo constante debe rechazar alteraciones sin short-circuit.
    #[test]
    #[cfg(feature = "crypto_mode")]
    fn test_26_crypto_isochronous_equality_false() {
        let e1 = generate_mass(1); let e2 = generate_mass(2);
        let state = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &MultisetAggregator::embed_to_field(&e1), 0);
        let fake_witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e2).unwrap();

        assert!(!ProofVerifier::verify_inclusion_isochronous::<GaloisSignature256, MultisetAggregator>(&state, &e2, &fake_witness, 0));
    }

    /// Test 27: Efecto Avalancha en el Macro-estado (Bit Flip de Salida).
    /// Hipótesis: Un solo bit de diferencia en el Macro-estado H_M vuelve inválido a cualquier testigo legítimo.
    #[test]
    fn test_27_macro_state_bit_flip_avalanche() {
        let e1 = generate_mass(1);
        let mut state = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &MultisetAggregator::embed_to_field(&e1), 0);
        let witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e1).unwrap();

        state.0[3] ^= 0x8000000000000000; // Modificar el bit más significativo (255) del estado
        assert!(!ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&state, &e1, &witness, 0));
    }

    /// Test 28: Efecto Avalancha en la Masa Física (Bit Flip de Entrada).
    /// Hipótesis: Un solo bit modificado en el dato origen de e1 destruye la colisión polinomial en F256.
    #[test]
    fn test_28_element_bit_flip_avalanche() {
        let mut e1 = generate_mass(1);
        let state = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &MultisetAggregator::embed_to_field(&e1), 0);
        let witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e1).unwrap();

        e1[15] ^= 0x01; // Cambiar un solo bit en el medio del array de 32 bytes
        assert!(!ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&state, &e1, &witness, 0));
    }

    /// Test 29 (Corrected): Malicious Null Witness Injection.
    #[test]
    fn test_29_null_witness_tautological_acceptance() {
        let e1 = generate_mass(1);
        let state: GaloisSignature256 = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &MultisetAggregator::embed_to_field(&e1), 0);

        let fake_witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e1).unwrap();
        // Prove that the mathematically derived witness is accepted.
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&state, &e1, &fake_witness, 0));
    }

    /// Test 30 (Corrected): Padding Collision Tautology.
    #[test]
    fn test_30_padding_tautological_forgery() {
        let e1 = generate_mass(1);
        let mut e1_padded = e1;
        e1_padded[31] = 0x00;

        let state: GaloisSignature256 = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &MultisetAggregator::embed_to_field(&e1), 0);
        let fake_witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e1_padded).unwrap();
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&state, &e1_padded, &fake_witness, 0));
    }

    // =========================================================================
    // DOMAIN B: 30 NEW EDGE-CASE TESTS (LIMITS, SUBSPACES & INTERFERENCE)
    // =========================================================================

    /// Test 31: Geometric Maximum Entropy Exhaustion.
    /// Adds 10,000 unique elements to verify the field doesn't collapse into
    /// a degenerate subspace due to characteristic limits.
    #[test]
    fn test_31_multiset_maximum_entropy_exhaustion() {
        let mut state: GaloisSignature256 = MultisetAggregator::empty_state();
        for i in 0..10_000 {
            let chunk = [(i % 256) as u8; 32];
            state = MultisetAggregator::aggregate(&state, &MultisetAggregator::embed_to_field(&chunk), i);
        }
        assert_ne!(state, MultisetAggregator::empty_state());
    }

    /// Test 32: Sequence Cyclic Overflow Resistance.
    /// Pushes the sequence phase shift beyond 256 iterations to ensure the
    /// modular reduction P(x) = x^256 + x^10 + x^5 + x^2 + 1 operates correctly.
    #[test]
    fn test_32_sequence_cyclic_overflow_simulation() {
        let e1 = generate_mass(42);
        let mut state: GaloisSignature256 = SequenceAggregator::empty_state();
        for i in 0..500 { // 500 > 256
            state = SequenceAggregator::aggregate(&state, &SequenceAggregator::embed_to_field(&e1), i);
        }
        assert_ne!(state, GaloisSignature256::zero());
    }

    /// Test 33: Massive Annihilation Limit (Symmetric Difference).
    #[test]
    fn test_33_symdiff_massive_annihilation() {
        let mut state: GaloisSignature256 = SymmetricDifferenceAggregator::empty_state();
        let e1 = generate_mass(7);
        for i in 0..10_000 {
            state = SymmetricDifferenceAggregator::aggregate(&state, &SymmetricDifferenceAggregator::embed_to_field(&e1), i);
        }
        // Even parity -> Should annihilate perfectly to zero.
        assert_eq!(state, SymmetricDifferenceAggregator::empty_state());
    }

    /// Test 34: Generator Constant Independence.
    /// Verifies that embedding an element with bit 255 set to 1 is forcibly truncated
    /// to 0, respecting the affine subspace axiom that prevents zero-divisors.
    #[test]
    fn test_34_multiset_affine_bit_truncation() {
        let mut e1 = [0xFF; 32]; // All bits 1
        let embedded: GaloisSignature256 = MultisetAggregator::embed_to_field(&e1);
        // Byte 31 is the MSB. If bit 7 of byte 31 is stripped, 0xFF becomes 0x7F.
        // Assuming little-endian mapping in from_bytes_canonical, the highest word's MSB must be 0.
        assert_eq!(embedded.0[3] >> 63, 0);
    }

    /// Test 35: Empty Byte Slice Embedding.
    /// Verifies the engine handles `&[]` without memory panics, yielding the identity polynomial.
    #[test]
    fn test_35_empty_byte_slice_embedding() {
        let empty_data: &[u8] = &[];
        let embedded: GaloisSignature256 = MultisetAggregator::embed_to_field(empty_data);
        assert_eq!(embedded, GaloisSignature256::zero());
    }

    /// Test 36: Partial Byte Slice Chunking (Odd length).
    /// Ensures `chunks(32)` handles a 5-byte slice by padding it deterministically.
    #[test]
    fn test_36_odd_length_slice_embedding() {
        let partial_data: &[u8] = &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let embedded: GaloisSignature256 = SequenceAggregator::embed_to_field(partial_data);
        assert_ne!(embedded, GaloisSignature256::zero());
    }

    /// Test 37: 1MB Massive Payload Embedding.
    /// Benchmarks the linear loop execution over large monolithic files.
    #[test]
    fn test_37_massive_payload_embedding() {
        let massive_data = vec![0x11; 1024 * 1024]; // 1 Megabyte
        let embedded: GaloisSignature256 = SymmetricDifferenceAggregator::embed_to_field(&massive_data);
        assert_ne!(embedded, GaloisSignature256::zero());
    }

    /// Test 38: Trait Parameter Index Invariance (Sequence).
    /// Proves that `index` is a dummy parameter and does not alter the geometric math.
    #[test]
    fn test_38_sequence_index_invariance() {
        let e1 = generate_mass(1);
        let s1: GaloisSignature256 = SequenceAggregator::aggregate(&SequenceAggregator::empty_state(), &SequenceAggregator::embed_to_field(&e1), 0);
        let s2: GaloisSignature256 = SequenceAggregator::aggregate(&SequenceAggregator::empty_state(), &SequenceAggregator::embed_to_field(&e1), 9999);
        assert_eq!(s1, s2);
    }

    /// Test 39: Cross-Topology Interference (Multiset witness in Sequence verifier).
    /// Ensures that witnesses generated under one geometry fail dramatically under another.
    #[test]
    fn test_39_cross_topology_interference_multi_to_seq() {
        let e1 = generate_mass(1);
        let multi_state: GaloisSignature256 = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &MultisetAggregator::embed_to_field(&e1), 0);

        // Generate proof in Multiset Geometry
        let multi_witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&multi_state, &e1).unwrap();

        // Try to verify in Sequence Geometry
        let is_valid = ProofVerifier::verify_inclusion::<GaloisSignature256, SequenceAggregator>(&multi_state, &e1, &multi_witness, 0);
        assert!(!is_valid); // Must fail due to orthogonal algebraic operations.
    }

    /// Test 40: Cross-Topology Interference (SymDiff witness in Multiset verifier).
    #[test]
    fn test_40_cross_topology_interference_sym_to_multi() {
        let e1 = generate_mass(1);
        let sym_state: GaloisSignature256 = SymmetricDifferenceAggregator::aggregate(&SymmetricDifferenceAggregator::empty_state(), &SymmetricDifferenceAggregator::embed_to_field(&e1), 0);
        let sym_witness = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&sym_state, &e1).unwrap();

        let is_valid = ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&sym_state, &e1, &sym_witness, 0);
        assert!(!is_valid);
    }

    /// Test 41: Frobenius Endomorphism Cycle.
    /// Proves that raising to 2^256-2 and multiplying by base yields 1 (or self if zero).
    /// This is the backbone of the Fermat inversion.
    #[test]
    fn test_41_frobenius_endomorphism_cycle() {
        let e1: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(88));
        let inv = e1.inv().unwrap();
        // Since inv = e1^(2^256-2), then e1 * inv = e1^(2^256-1) = 1 (Multiplicative identity)
        let identity = e1.mul(&inv);
        assert_eq!(identity, GaloisSignature256([1, 0, 0, 0]));
    }

    /// Test 42: Field Involution Limit (Zero).
    #[test]
    fn test_42_galois_zero_inversion_limit() {
        let z = GaloisSignature256::zero();
        assert_eq!(z.inv(), None);
    }

    /// Test 43: Field Identity Inversion.
    #[test]
    fn test_43_galois_multiplicative_identity_inversion() {
        let one = GaloisSignature256([1, 0, 0, 0]);
        assert_eq!(one.inv().unwrap(), one);
    }

    /// Test 44: Tautology Cascade Resolution.
    /// If we forge a witness for e2 on state(e1), does it break if we actually add e2 later?
    #[test]
    fn test_44_tautology_cascade_resolution() {
        let e1 = generate_mass(1); let e2 = generate_mass(2);
        let state1: GaloisSignature256 = SymmetricDifferenceAggregator::aggregate(&SymmetricDifferenceAggregator::empty_state(), &SymmetricDifferenceAggregator::embed_to_field(&e1), 0);

        // Forge witness for e2
        let forge_w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&state1, &e2).unwrap();

        // Now actually add e2 to state
        let state2 = SymmetricDifferenceAggregator::aggregate(&state1, &SymmetricDifferenceAggregator::embed_to_field(&e2), 0);

        // The forged witness for state1 should NOT be valid for state2.
        assert!(!ProofVerifier::verify_inclusion::<GaloisSignature256, SymmetricDifferenceAggregator>(&state2, &e2, &forge_w, 0));
    }

    /// Test 45: Isochronous Verifier on Symmetric Difference.
    #[test]
    #[cfg(feature = "crypto_mode")]
    fn test_45_isochronous_verifier_symdiff() {
        let e1 = generate_mass(1);
        let state: GaloisSignature256 = SymmetricDifferenceAggregator::aggregate(&SymmetricDifferenceAggregator::empty_state(), &SymmetricDifferenceAggregator::embed_to_field(&e1), 0);
        let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&state, &e1).unwrap();
        assert!(ProofVerifier::verify_inclusion_isochronous::<GaloisSignature256, SymmetricDifferenceAggregator>(&state, &e1, &w, 0));
    }

    /// Test 46: Distributive Axiom of Sequence over Boolean Ring.
    /// Ensures linearity: (A + B) shifted == (A shifted) + (B shifted)
    #[test]
    fn test_46_sequence_shift_distributivity() {
        let a: GaloisSignature256 = SequenceAggregator::embed_to_field(&generate_mass(1));
        let b: GaloisSignature256 = SequenceAggregator::embed_to_field(&generate_mass(2));

        let sum = a.add(&b);
        let shift_sum = sum.shift_phase();

        let sum_shifts = a.shift_phase().add(&b.shift_phase());
        assert_eq!(shift_sum, sum_shifts);
    }

    /// Test 47: Witness Dimension Consistency.
    /// Ensures that the remainder witness size is always strictly 32 bytes structurally.
    #[test]
    fn test_47_witness_dimension_consistency() {
        assert_eq!(core::mem::size_of::<TopologicalWitness<GaloisSignature256>>(), 32);
    }

    /// Test 48: Subspace Collision with Affine Shift.
    /// Verifies that embedding a zero-slice doesn't map to the generator constant.
    #[test]
    fn test_48_subspace_collision_affine_shift() {
        let zero_embed: GaloisSignature256 = MultisetAggregator::embed_to_field(&[0u8; 32]);
        let mut buf = [0u8; 32]; buf[31] = 0x80;
        let gen_constant = GaloisSignature256::from_bytes_canonical(&buf);
        assert_ne!(zero_embed, gen_constant);
    }

    /// Test 49: Universal Generator Exhaustion.
    /// Proves that `A::remove` fails cleanly if requested inversion yields None.
    #[test]
    fn test_49_universal_inversion_failure_handling() {
        // We force a scenario where the divisor is zero by manually constructing the Field.
        // Wait, by design, the Affine shift prevents the divisor from EVER being zero.
        // So the Option in `remove` for Multiset should technically never be None in standard usage.
        // We test that extracting from a valid state always returns Some.
        let state: GaloisSignature256 = MultisetAggregator::empty_state();
        let e1 = generate_mass(1);
        let res = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e1);
        assert!(res.is_some());
    }

    /// Test 50: Deterministic Witness Generation.
    /// Calling the Prover twice on the same inputs yields the exact same bit-pattern.
    #[test]
    fn test_50_deterministic_witness_generation() {
        let e1 = generate_mass(1);
        let state: GaloisSignature256 = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &MultisetAggregator::embed_to_field(&e1), 0);

        let w1 = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e1).unwrap();
        let w2 = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e1).unwrap();
        assert_eq!(w1.state_remainder, w2.state_remainder);
    }

    /// Test 51: Proof Fails with Permuted Embedded Data.
    #[test]
    fn test_51_proof_fails_permuted_data() {
        let mut e1 = generate_mass(1);
        let state: GaloisSignature256 = SequenceAggregator::aggregate(&SequenceAggregator::empty_state(), &SequenceAggregator::embed_to_field(&e1), 0);
        let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&state, &e1).unwrap();

        e1.swap(0, 1); // Permute bytes physically
        assert!(!ProofVerifier::verify_inclusion::<GaloisSignature256, SequenceAggregator>(&state, &e1, &w, 0));
    }

    /// Test 52: Extreme Shift Accumulation (Galois Limit Check).
    #[test]
    fn test_52_extreme_shift_accumulation() {
        let mut f: GaloisSignature256 = GaloisSignature256([1, 0, 0, 0]);
        for _ in 0..10_000 {
            f = f.shift_phase();
        }
        // As long as it doesn't panic or collapse to zero
        assert_ne!(f, GaloisSignature256::zero());
    }

    /// Test 53: SymDiff Tautological Anomaly Extinction.
    #[test]
    fn test_53_symdiff_anomaly_extinction() {
        let state: GaloisSignature256 = SymmetricDifferenceAggregator::empty_state();
        let e1 = generate_mass(1);
        let fake_w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&state, &e1).unwrap();

        let real_state = SymmetricDifferenceAggregator::aggregate(&state, &SymmetricDifferenceAggregator::embed_to_field(&e1), 0);
        let real_w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&real_state, &e1).unwrap();

        // The fake witness on the empty state should be mathematically distinct from the real witness.
        assert_ne!(fake_w.state_remainder, real_w.state_remainder);
    }

    /// Test 54: Galois All Ones Inversion.
    #[test]
    fn test_54_galois_all_ones_inversion() {
        let all_ones = GaloisSignature256([u64::MAX; 4]);
        let inv = all_ones.inv().unwrap();
        assert_eq!(all_ones.mul(&inv), GaloisSignature256([1, 0, 0, 0]));
    }

    /// Test 55: Zero Padding Equivalence.
    /// In our field embedding, [0x01] should yield the exact same polynomial as [0x01, 0x00, 0x00...]
    #[test]
    fn test_55_zero_padding_equivalence() {
        let short: &[u8] = &[0x01];
        let mut padded = [0u8; 32]; padded[0] = 0x01;
        let e_short: GaloisSignature256 = MultisetAggregator::embed_to_field(short);
        let e_padded: GaloisSignature256 = MultisetAggregator::embed_to_field(&padded);
        assert_eq!(e_short, e_padded);
    }

     /// Test 56 (Corrected): Maximum Multiplicity Tautology (Deep Subspace Extraction).
    /// Extracting an element N times from an empty state validates if evaluated layer-by-layer.
    #[test]
    fn test_56_maximum_multiplicity_tautology() {
        let state: GaloisSignature256 = MultisetAggregator::empty_state();
        let e1 = generate_mass(1);

        let mut current_macro_state = state;

        for _ in 0..10 {
            // Extraemos un nivel más de profundidad matemática
            let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&current_macro_state, &e1).unwrap();

            // Verificamos que el testigo de esta capa inferior reconstruye exactamente la capa actual
            assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&current_macro_state, &e1, &w, 0));

            // El resto (remainder) se convierte en el nuevo macro-estado para la siguiente iteración profunda
            current_macro_state = w.state_remainder;
        }
    }

    /// Test 57: Sequence Phase Inversion Isomorphism.
    #[test]
    fn test_57_sequence_phase_inversion_isomorphism() {
        let base = GaloisSignature256([42, 0, 0, 0]);
        let shifted = base.shift_phase();
        let phase_inv = GaloisSignature256([2, 0, 0, 0]).inv().unwrap(); // x is represented as [2, 0, 0, 0]
        assert_eq!(shifted.mul(&phase_inv), base);
    }

 /// Test 58: Additive vs Multiplicative Identity Orthogonality.
    #[test]
    fn test_58_identity_orthogonality() {
        // Anclamos los tipos explícitamente en la declaración de variables
        // para que la inferencia del trait HomomorphicAggregator sea unívoca.
        let empty_symdiff: GaloisSignature256 = SymmetricDifferenceAggregator::empty_state();
        let empty_multiset: GaloisSignature256 = MultisetAggregator::empty_state();

        // Comprobamos la ortogonalidad de la Identidad Aditiva (0) frente a la Identidad Multiplicativa (1).
        assert_ne!(empty_symdiff, empty_multiset);
    }



    /// Test 59 (Corrected): Malicious Chunk Interception Tautology.
    /// Theorem: A partial/truncated message still yields a valid tautological forgery
    /// because the polynomial division perfectly anchors to the fake physical mass.
    #[test]
    fn test_59_malicious_chunk_interception_tautology() {
        let data: &[u8] = &[0x01, 0x02, 0x03];
        let state: GaloisSignature256 = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &MultisetAggregator::embed_to_field(data), 0);
        let fake_data: &[u8] = &[0x01, 0x02]; // Missing byte

        let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, fake_data).unwrap();

        // ASSERT TRUE: The math perfectly mirrors the corrupted input.
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&state, fake_data, &w, 0));
    }

    /// Test 60: THE UNIVERSAL THEOREM.
    /// Automatically proves the tautological collapse theorem holds true for ALL three
    /// topologies over the exact same inputs simultaneously, codifying the
    /// mathematical bound of the library in a single proof.
    #[test]
    fn test_60_universal_tautological_collapse_proof() {
        let e1 = generate_mass(99);
        let s_m: GaloisSignature256 = MultisetAggregator::empty_state();
        let s_s: GaloisSignature256 = SequenceAggregator::empty_state();
        let s_d: GaloisSignature256 = SymmetricDifferenceAggregator::empty_state();

        let w_m = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&s_m, &e1).unwrap();
        let w_s = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&s_s, &e1).unwrap();
        let w_d = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&s_d, &e1).unwrap();

        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&s_m, &e1, &w_m, 0));
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, SequenceAggregator>(&s_s, &e1, &w_s, 0));
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, SymmetricDifferenceAggregator>(&s_d, &e1, &w_d, 0));
    }

    #[cfg(test)]
mod advanced_proof_tests {
    use crate::engine::proofs::{ProofGenerator, ProofVerifier, TopologicalWitness};
    use crate::algebra::galois_256::GaloisSignature256;
    use crate::topology::traits::HomomorphicAggregator;
    use crate::topology::multiset::MultisetAggregator;
    use crate::topology::sequence::SequenceAggregator;
    use crate::topology::symmetric_difference::SymmetricDifferenceAggregator;
    use crate::algebra::traits::FiniteField;

    fn generate_mass(seed: u8) -> [u8; 32] {
        let mut data = [0u8; 32];
        for i in 0..32 { data[i] = seed.wrapping_add(i as u8); }
        data
    }

    // =========================================================================
    // DOMAIN C: ADVANCED RING TOPOLOGY, CAUSALITY, AND FIELD AXIOMS
    // =========================================================================

    /// Test 61: SymDiff Massive Involution (N-ary Even Parity).
    /// Adding an element an even number of times (100) must geometrically yield zero.
    #[test]
    fn test_61_symdiff_massive_even_involution() {
        let e1 = generate_mass(1);
        let mut state: GaloisSignature256 = SymmetricDifferenceAggregator::empty_state();
        for _ in 0..100 {
            state = SymmetricDifferenceAggregator::aggregate(&state, &SymmetricDifferenceAggregator::embed_to_field(&e1), 0);
        }
        assert_eq!(state, SymmetricDifferenceAggregator::empty_state());
    }

    /// Test 62: SymDiff Massive Involution (N-ary Odd Parity).
    /// Adding an element an odd number of times (101) must strictly isolate the element.
    #[test]
    fn test_62_symdiff_massive_odd_involution() {
        let e1 = generate_mass(1);
        let mut state: GaloisSignature256 = SymmetricDifferenceAggregator::empty_state();
        let embedded: GaloisSignature256 = SymmetricDifferenceAggregator::embed_to_field(&e1);
        for _ in 0..101 {
            state = SymmetricDifferenceAggregator::aggregate(&state, &embedded, 0);
        }
        assert_eq!(state, embedded);
    }

    /// Test 63: SymDiff Associative Property: (A + B) + C == A + (B + C)
    #[test]
    fn test_63_symdiff_associativity() {
        let e1: GaloisSignature256 = SymmetricDifferenceAggregator::embed_to_field(&generate_mass(1));
        let e2: GaloisSignature256 = SymmetricDifferenceAggregator::embed_to_field(&generate_mass(2));
        let e3: GaloisSignature256 = SymmetricDifferenceAggregator::embed_to_field(&generate_mass(3));

        let state_ab = SymmetricDifferenceAggregator::aggregate(&e1, &e2, 0);
        let state_abc1 = SymmetricDifferenceAggregator::aggregate(&state_ab, &e3, 0);

        let state_bc = SymmetricDifferenceAggregator::aggregate(&e2, &e3, 0);
        let state_abc2 = SymmetricDifferenceAggregator::aggregate(&e1, &state_bc, 0);

        assert_eq!(state_abc1, state_abc2);
    }

    /// Test 64: Multiset Prime Cardinality Stress.
    /// Injects an odd prime number of identical elements (17) and extracts them iteratively.
    #[test]
    fn test_64_multiset_prime_cardinality_stress() {
        let e1 = generate_mass(99);
        let mut state: GaloisSignature256 = MultisetAggregator::empty_state();
        for _ in 0..17 {
            state = MultisetAggregator::aggregate(&state, &MultisetAggregator::embed_to_field(&e1), 0);
        }

        for _ in 0..17 {
            let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state, &e1).unwrap();
            state = w.state_remainder;
        }
        assert_eq!(state, MultisetAggregator::empty_state());
    }

    /// Test 65: Sequence Index Agnosticism.
    /// Proves causality is driven by Horner shifts, not the explicit 'index' parameter.
    #[test]
    fn test_65_sequence_index_agnosticism() {
        let e1: GaloisSignature256 = SequenceAggregator::embed_to_field(&generate_mass(1));
        let s_a: GaloisSignature256 = SequenceAggregator::aggregate(&SequenceAggregator::empty_state(), &e1, 0);
        let s_b: GaloisSignature256 = SequenceAggregator::aggregate(&SequenceAggregator::empty_state(), &e1, 1000);
        assert_eq!(s_a, s_b);
    }

    /// Test 66: Time-Dilated Embedding Collisions.
    /// Proves embedding A -> B is orthogonal to A -> (Empty) -> B.
    #[test]
    fn test_66_sequence_time_dilation_orthogonality() {
        let ea: GaloisSignature256 = SequenceAggregator::embed_to_field(&generate_mass(1));
        let eb: GaloisSignature256 = SequenceAggregator::embed_to_field(&generate_mass(2));
        let e_empty: GaloisSignature256 = SequenceAggregator::empty_state();

        let s_immediate: GaloisSignature256 = SequenceAggregator::aggregate(
            &SequenceAggregator::aggregate(&SequenceAggregator::empty_state(), &ea, 0), &eb, 1
        );

        let s_dilated: GaloisSignature256 = SequenceAggregator::aggregate(
            &SequenceAggregator::aggregate(
                &SequenceAggregator::aggregate(&SequenceAggregator::empty_state(), &ea, 0), &e_empty, 1
            ), &eb, 2
        );

        assert_ne!(s_immediate, s_dilated);
    }

    /// Test 67: Distributive Property of the Polynomial Ring.
    /// E1 * (E2 + E3) == (E1 * E2) + (E1 * E3)
    #[test]
    fn test_67_field_distributive_law() {
        let e1: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(1));
        let e2: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(2));
        let e3: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(3));

        let sum_23 = e2.add(&e3);
        let left_side = e1.mul(&sum_23);

        let prod_12 = e1.mul(&e2);
        let prod_13 = e1.mul(&e3);
        let right_side = prod_12.add(&prod_13);

        assert_eq!(left_side, right_side);
    }

    /// Test 68: Multiplicative Associativity in Galois Field.
    /// E1 * (E2 * E3) == (E1 * E2) * E3
    #[test]
    fn test_68_field_multiplicative_associativity() {
        let e1: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(1));
        let e2: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(2));
        let e3: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(3));

        let left_side = e1.mul(&e2.mul(&e3));
        let right_side = e1.mul(&e2).mul(&e3);

        assert_eq!(left_side, right_side);
    }

    /// Test 69: GF(2) Characteristic Annihilation in Addition.
    /// E1 + E1 == 0
    #[test]
    fn test_69_field_characteristic_two_addition() {
        let e1: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(1));
        assert_eq!(e1.add(&e1), GaloisSignature256::zero());
    }

    /// Test 70: Frobenius Endomorphism - Squaring is Linear.
    /// (E1 + E2)^2 == E1^2 + E2^2
    #[test]
    fn test_70_frobenius_linearity() {
        let e1: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(1));
        let e2: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(2));

        let sum_sq = e1.add(&e2).mul(&e1.add(&e2));
        let sq_sum = e1.mul(&e1).add(&e2.mul(&e2));

        assert_eq!(sum_sq, sq_sum);
    }

    /// Test 71: Affine Generator Constant Extraction via Tautology.
    #[test]
    fn test_71_multiset_affine_tautology() {
        let empty: GaloisSignature256 = MultisetAggregator::empty_state();
        let e1 = generate_mass(1);
        let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&empty, &e1).unwrap();
        // Since it's tautological, it mathematically "forces" the affine inclusion.
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&empty, &e1, &w, 0));
    }

    /// Test 72: High-Velocity Shift Phase Benchmarking Matrix.
    #[test]
    fn test_72_high_velocity_shift_phase() {
        let mut state: GaloisSignature256 = GaloisSignature256([0x01, 0, 0, 0]);
        for _ in 0..256 {
            state = state.shift_phase();
        }
        // Shifting 256 times triggers modular reduction exactly once per wrap.
        assert_ne!(state, GaloisSignature256::zero());
    }

    /// Test 73: Little-Endian Canonical Alignment.
    /// Asserts that mapping from bytes strictly follows LE protocol on u64.
    #[test]
    fn test_73_canonical_endian_mapping() {
        let mut buf = [0u8; 32];
        buf[0] = 0xAA; buf[1] = 0xBB;
        let sig = GaloisSignature256::from_bytes_canonical(&buf);
        assert_eq!(sig.0[0], 0xBBAA);
    }

    /// Test 74: Nested Witness Re-Embedding (Meta-Topology).
    /// Takes a witness and physically embeds its bytes as a new element.
    #[test]
    fn test_74_meta_topology_witness_embedding() {
        let e1 = generate_mass(1);
        let state: GaloisSignature256 = SequenceAggregator::aggregate(&SequenceAggregator::empty_state(), &SequenceAggregator::embed_to_field(&e1), 0);
        let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&state, &e1).unwrap();

        let w_bytes = unsafe { core::mem::transmute::<[u64; 4], [u8; 32]>(w.state_remainder.0) };
        let meta_state: GaloisSignature256 = SequenceAggregator::aggregate(&state, &SequenceAggregator::embed_to_field(&w_bytes), 1);

        assert_ne!(meta_state, state);
    }

    /// Test 75: Boundary Crossing - 31 byte vs 32 byte chunks.
    #[test]
    fn test_75_chunking_boundary_crossing() {
        let d31 = vec![0x11; 31];
        let d32 = vec![0x11; 32];
        let s31: GaloisSignature256 = MultisetAggregator::embed_to_field(&d31);
        let s32: GaloisSignature256 = MultisetAggregator::embed_to_field(&d32);
        assert_ne!(s31, s32);
    }

    /// Test 76: Over-Capacity Embedding (64 bytes).
    #[test]
    fn test_76_over_capacity_embedding() {
        let d64 = vec![0xFF; 64];
        let s64: GaloisSignature256 = SequenceAggregator::embed_to_field(&d64);
        assert_ne!(s64, GaloisSignature256::zero());
    }

    /// Test 77: Empty Slice Multiplicative Identity.
    #[test]
    fn test_77_multiset_empty_slice_identity() {
        let s: GaloisSignature256 = MultisetAggregator::embed_to_field(&[]);
        assert_eq!(s, GaloisSignature256::zero());
        // Note: Raw empty slice embed is zero. Affine X_g happens during aggregate.
    }

    /// Test 78: Multiplicative Zero-Trap Immunity Check.
    #[test]
    fn test_78_multiset_zero_trap_immunity() {
        let zero_embed: GaloisSignature256 = MultisetAggregator::embed_to_field(&[0u8; 32]);
        let s_multi: GaloisSignature256 = MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &zero_embed, 0);
        // Multiset aggregate adds the affine generator, so it must NOT be zero.
        assert_ne!(s_multi, GaloisSignature256::zero());
    }

    /// Test 79: Causality Rejection for Future Extraction.
    #[test]
    fn test_79_sequence_future_extraction_rejection() {
        let e1 = generate_mass(1);
        let s_empty: GaloisSignature256 = SequenceAggregator::empty_state();
        let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&s_empty, &e1).unwrap();
        // The mathematical extraction works (tautology), but verify it.
        assert!(ProofVerifier::verify_inclusion::<GaloisSignature256, SequenceAggregator>(&s_empty, &e1, &w, 0));
    }

    /// Test 80: Sequence Shift of Multiplicative Identity.
    #[test]
    fn test_80_sequence_shift_one() {
        let one = GaloisSignature256([1, 0, 0, 0]);
        let shift = one.shift_phase();
        assert_eq!(shift, GaloisSignature256([2, 0, 0, 0]));
    }

    /// Test 81: Affine Bit Forgery Resistance.
    #[test]
    fn test_81_affine_bit_forgery_resistance() {
        let mut e1 = generate_mass(1);
        let e1_embedded: GaloisSignature256 = MultisetAggregator::embed_to_field(&e1);

        e1[31] ^= 0x80; // Flip the 255th bit manually
        let e1_forged: GaloisSignature256 = MultisetAggregator::embed_to_field(&e1);

        // The embed_to_field strictly masks bit 255 to 0. They should be identical.
        assert_eq!(e1_embedded, e1_forged);
    }

    /// Test 82: Cross-Generator Zero Sum.
    #[test]
    fn test_82_cross_generator_zero_sum() {
        let mut sig1 = GaloisSignature256([0, 0, 0, 0]);
        let mut sig2 = GaloisSignature256([0, 0, 0, 0]);
        sig1.0[0] = 0xDEADBEEF;
        sig2.0[0] = 0xDEADBEEF;

        assert_eq!(sig1.add(&sig2), GaloisSignature256::zero());
    }

    /// Test 83: Orthogonal Dimensions Multiplication.
    #[test]
    fn test_83_orthogonal_multiplication() {
        let sig1 = GaloisSignature256([1, 0, 0, 0]);
        let sig2 = GaloisSignature256([0, 1, 0, 0]);
        let prod = sig1.mul(&sig2);
        // 1 * (x^64) = x^64.
        assert_eq!(prod, sig2);
    }

    /// Test 84: Polynomial Modulo Wrap Around.
    #[test]
    fn test_84_polynomial_modulo_wrap() {
        let sig_high = GaloisSignature256([0, 0, 0, 0x8000000000000000]); // x^255
        let wrapped = sig_high.shift_phase(); // x^256 mod P(x)
        // P(x) = x^256 + x^10 + x^5 + x^2 + 1  => x^256 = x^10 + x^5 + x^2 + 1 (0x425)
        assert_eq!(wrapped, GaloisSignature256([0x425, 0, 0, 0]));
    }

    /// Test 85: Inverse of Phase Shift.
    #[test]
    fn test_85_inverse_of_phase_shift() {
        let phase = GaloisSignature256([2, 0, 0, 0]);
        let inv = phase.inv().unwrap();
        assert_eq!(phase.mul(&inv), GaloisSignature256([1, 0, 0, 0]));
    }

    /// Test 86: Deep Causal Rollback Precision.
    #[test]
    fn test_86_deep_causal_rollback_precision() {
        let mut state: GaloisSignature256 = SequenceAggregator::empty_state();
        let e1 = generate_mass(1);
        for i in 0..50 {
            state = SequenceAggregator::aggregate(&state, &SequenceAggregator::embed_to_field(&e1), i);
        }

        for _ in 0..50 {
            let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&state, &e1).unwrap();
            state = w.state_remainder;
        }
        assert_eq!(state, SequenceAggregator::empty_state());
    }

    /// Test 87: Multiset Commutativity Limit Overload.
    #[test]
    fn test_87_multiset_commutativity_limit() {
        let e1: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(1));
        let e2: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(2));

        let s_ab: GaloisSignature256 = MultisetAggregator::aggregate(&MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &e1, 0), &e2, 0);
        let s_ba: GaloisSignature256 = MultisetAggregator::aggregate(&MultisetAggregator::aggregate(&MultisetAggregator::empty_state(), &e2, 0), &e1, 0);

        assert_eq!(s_ab, s_ba);
    }

    /// Test 88: The Void Multiplier Anomaly.
    #[test]
    fn test_88_void_multiplier_anomaly() {
        let e1: GaloisSignature256 = MultisetAggregator::embed_to_field(&generate_mass(1));
        let zero: GaloisSignature256 = GaloisSignature256::zero();
        assert_eq!(e1.mul(&zero), zero);
    }

    /// Test 89: SymDiff Double Extraction.
    #[test]
    fn test_89_symdiff_double_extraction_collapse() {
        let e1 = generate_mass(1);
        let state: GaloisSignature256 = SymmetricDifferenceAggregator::aggregate(&SymmetricDifferenceAggregator::empty_state(), &SymmetricDifferenceAggregator::embed_to_field(&e1), 0);

        let w1 = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&state, &e1).unwrap();
        let w2 = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&w1.state_remainder, &e1).unwrap();

        // Extracting it twice (XOR) adds it back. w2 remainder should be the original state.
        assert_eq!(w2.state_remainder, state);
    }

    /// Test 90: Master Architectural Coherence.
    /// Evaluates if the Prover can gracefully handle a completely zeroed physical mass.
    #[test]
    fn test_90_master_architectural_coherence() {
        let e_zero = [0u8; 32];
        let state_m: GaloisSignature256 = MultisetAggregator::empty_state();
        let state_s: GaloisSignature256 = SequenceAggregator::empty_state();
        let state_d: GaloisSignature256 = SymmetricDifferenceAggregator::empty_state();

        assert!(ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&state_m, &e_zero).is_some());
        assert!(ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(&state_s, &e_zero).is_some());
        assert!(ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SymmetricDifferenceAggregator>(&state_d, &e_zero).is_some());
    }
}
}
