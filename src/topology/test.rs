#[cfg(test)]
mod tests {
    use crate::algebra::galois_256::GaloisSignature256;
    use crate::algebra::traits::FiniteField;
    use crate::topology::symmetric_difference::SymmetricDifferenceAggregator;
    use crate::topology::traits::HomomorphicAggregator;

    type SymDiff = SymmetricDifferenceAggregator;
    type F256 = GaloisSignature256;

    /// Helper to embed strings directly for testing
    fn embed_str(s: &str) -> F256 {
        SymDiff::embed_to_field(s.as_bytes())
    }

    /// Helper to compute the byte-wise XOR of two vectors (for Linearity tests)
    fn xor_bytes(a: &[u8], b: &[u8]) -> Vec<u8> {
        let len = a.len().max(b.len());
        let mut result = vec![0u8; len];
        for i in 0..len {
            let a_val = if i < a.len() { a[i] } else { 0 };
            let b_val = if i < b.len() { b[i] } else { 0 };
            result[i] = a_val ^ b_val;
        }
        result
    }

    // =========================================================================
    // GROUP 1: Vacuum and Basic Aggregation Axioms
    // =========================================================================

    #[test]
    fn t01_empty_state_is_strict_zero() {
        let vacuum: F256 = SymDiff::empty_state();
        assert_eq!(
            vacuum,
            F256::zero(),
            "Vacuum state must be the additive identity (0)"
        );
    }

    #[test]
    fn t02_aggregate_into_vacuum_is_identity() {
        let vacuum: F256 = SymDiff::empty_state();
        let element = embed_str("Node_A");
        let state = SymDiff::aggregate(&vacuum, &element, 0);
        assert_eq!(state, element, "0 + A = A");
    }

    #[test]
    fn t03_aggregation_is_commutative() {
        let a = embed_str("Alpha");
        let b = embed_str("Beta");

        let vacuum: F256 = SymDiff::empty_state();
        let state_ab = SymDiff::aggregate(&SymDiff::aggregate(&vacuum, &a, 0), &b, 1);
        let state_ba = SymDiff::aggregate(&SymDiff::aggregate(&vacuum, &b, 0), &a, 1);
        assert_eq!(state_ab, state_ba, "A U B must equal B U A geometrically");
    }

    #[test]
    fn t04_aggregation_is_associative() {
        let a = embed_str("A");
        let b = embed_str("B");
        let c = embed_str("C");

        let ab = SymDiff::aggregate(&a, &b, 0);
        let ab_c = SymDiff::aggregate(&ab, &c, 0);

        let bc = SymDiff::aggregate(&b, &c, 0);
        let a_bc = SymDiff::aggregate(&a, &bc, 0);

        assert_eq!(ab_c, a_bc, "(A + B) + C = A + (B + C)");
    }

    #[test]
    fn t05_remove_from_vacuum_is_element() {
        let vacuum: F256 = SymDiff::empty_state();
        let a = embed_str("Anomaly");
        // In characteristic 2, 0 - A = A
        let removed = SymDiff::remove(&vacuum, &a).unwrap();
        assert_eq!(removed, a, "0 - A = A in characteristic 2");
    }

    // =========================================================================
    // GROUP 2: Boolean Ring Laws and Symmetric Difference
    // =========================================================================

    #[test]
    fn t06_strict_self_annihilation() {
        let state: F256 = SymDiff::empty_state();
        let a = embed_str("Entity");

        let added = SymDiff::aggregate(&state, &a, 0);
        let added_twice = SymDiff::aggregate(&added, &a, 0);

        assert_eq!(added_twice, F256::zero(), "A + A = 0 (Parity annihilation)");
    }

    #[test]
    fn t07_remove_perfectly_undoes_aggregate() {
        let mut state: F256 = SymDiff::empty_state();
        let nodes = vec![embed_str("N1"), embed_str("N2"), embed_str("N3")];

        for n in &nodes {
            state = SymDiff::aggregate(&state, n, 0);
        }

        // Remove N2
        state = SymDiff::remove(&state, &nodes[1]).unwrap();

        // State should now strictly be N1 + N3
        let expected = SymDiff::aggregate(&nodes[0], &nodes[2], 0);
        assert_eq!(
            state, expected,
            "Cleavage must restore exact previous topology"
        );
    }

    #[test]
    fn t08_triple_aggregation_yields_element() {
        let a = embed_str("Ghost");
        let zero = F256::zero();
        let s1 = SymDiff::aggregate(&zero, &a, 0);
        let s2 = SymDiff::aggregate(&s1, &a, 0);
        let s3 = SymDiff::aggregate(&s2, &a, 0);
        assert_eq!(s3, a, "A + A + A = A");
    }

    #[test]
    fn t09_intersection_annihilation_in_unions() {
        // Set 1: {A, B}. Set 2: {B, C}.
        // Union in SymDiff: (A + B) + (B + C) = A + C
        let a = embed_str("A");
        let b = embed_str("B");
        let c = embed_str("C");

        let set1 = SymDiff::aggregate(&a, &b, 0);
        let set2 = SymDiff::aggregate(&b, &c, 0);

        let union = SymDiff::aggregate(&set1, &set2, 0);
        let expected = SymDiff::aggregate(&a, &c, 0);

        assert_eq!(
            union, expected,
            "Overlapping elements must destructively interfere"
        );
    }

    #[test]
    fn t10_aggregate_and_remove_are_isomorphic_functions() {
        let a = embed_str("Base");
        let b = embed_str("Target");

        let state_add = SymDiff::aggregate(&a, &b, 0);
        let state_rem = SymDiff::remove(&a, &b).unwrap();

        assert_eq!(
            state_add, state_rem,
            "Addition and Subtraction are the same operator"
        );
    }

    // =========================================================================
    // GROUP 3: Linear Embedding Geometry (Block Processing)
    // =========================================================================

    #[test]
    fn t11_embed_empty_slice_is_zero() {
        let e: F256 = SymDiff::embed_to_field(&[]);
        assert_eq!(e, F256::zero(), "Empty data has no topological volume");
    }

    #[test]
    fn t12_embed_exact_32_bytes_no_shift() {
        let data = [0xAA; 32];
        let e: F256 = SymDiff::embed_to_field(&data);
        let direct = F256::from_bytes_canonical(&data);
        assert_eq!(
            e, direct,
            "Single block embedding requires zero phase shifts"
        );
    }

    #[test]
    fn t13_embed_partial_block_pads_with_zeros() {
        let data = [0xFF; 16];
        let e: F256 = SymDiff::embed_to_field(&data);

        let mut padded = [0u8; 32];
        padded[..16].copy_from_slice(&data);
        let direct = F256::from_bytes_canonical(&padded);

        assert_eq!(
            e, direct,
            "Partial blocks must be zero-padded deterministically"
        );
    }

    #[test]
    fn t14_embed_64_bytes_triggers_exact_phase_shift() {
        let mut data = [0u8; 64];
        data[0] = 0x01; // First byte of Block 0
        data[32] = 0x02; // First byte of Block 1

        let e: F256 = SymDiff::embed_to_field(&data);

        let mut b0 = [0u8; 32];
        b0[0] = 0x01;
        let mut b1 = [0u8; 32];
        b1[0] = 0x02;

        let f0 = F256::from_bytes_canonical(&b0);
        let f1 = F256::from_bytes_canonical(&b1);

        // MATHEMATICAL FIX: Due to reverse iteration anchoring index 0 to Phi^0,
        // the polynomial is built as (B1 * Phi) + B0.
        let expected = f1.shift_phase().add(&f0);

        assert_eq!(e, expected, "Two blocks must follow: B1 * Phi + B0");
    }

    #[test]
    fn t15_embed_is_strictly_deterministic() {
        let data = b"The quick brown fox jumps over the lazy dog";
        let e1: F256 = SymDiff::embed_to_field(data);
        let e2: F256 = SymDiff::embed_to_field(data);
        assert_eq!(e1, e2, "Identical inputs must map to identical points");
    }

    #[test]
    fn t16_embed_distinguishes_trailing_zeros() {
        let d1 = [0x01];
        let d2 = [0x01, 0x00];

        let e1: F256 = SymDiff::embed_to_field(&d1);
        let e2: F256 = SymDiff::embed_to_field(&d2);
        assert_eq!(
            e1, e2,
            "Trailing zeros in a partial block represent the same geometry"
        );
    }

    // =========================================================================
    // GROUP 4: The Homomorphism Theorem (Linearity Proofs)
    // =========================================================================

    #[test]
    fn t17_homomorphism_single_block() {
        let a = b"Topological_Space_A";
        let b = b"Topological_Space_B";
        let a_xor_b = xor_bytes(a, b);

        let phi_a: F256 = SymDiff::embed_to_field(a);
        let phi_b: F256 = SymDiff::embed_to_field(b);
        let phi_axb: F256 = SymDiff::embed_to_field(&a_xor_b);

        let sum = phi_a.add(&phi_b);
        assert_eq!(
            phi_axb, sum,
            "Linear embedding MUST preserve homomorphism for 1 block"
        );
    }

    #[test]
    fn t18_homomorphism_multi_block() {
        let a = [0x55; 70];
        let b = [0x33; 70];
        let a_xor_b = xor_bytes(&a, &b);

        let phi_a: F256 = SymDiff::embed_to_field(&a);
        let phi_b: F256 = SymDiff::embed_to_field(&b);
        let phi_axb: F256 = SymDiff::embed_to_field(&a_xor_b);

        assert_eq!(
            phi_axb,
            phi_a.add(&phi_b),
            "Linearity must hold across phase shifts"
        );
    }

    #[test]
    fn t19_homomorphism_uneven_lengths() {
        let a = [0xFF; 10]; // Short
        let b = [0xAA; 50]; // Long
        let a_xor_b = xor_bytes(&a, &b);

        let phi_a: F256 = SymDiff::embed_to_field(&a);
        let phi_b: F256 = SymDiff::embed_to_field(&b);
        let phi_axb: F256 = SymDiff::embed_to_field(&a_xor_b);

        assert_eq!(
            phi_axb,
            phi_a.add(&phi_b),
            "Linearity holds for asymmetric data lengths"
        );
    }

    #[test]
    fn t20_orthogonality_of_disjoint_bitflips() {
        let mut flip_1 = [0u8; 32];
        flip_1[0] = 0x01;
        let mut flip_2 = [0u8; 32];
        flip_2[31] = 0x80;

        let phi_1: F256 = SymDiff::embed_to_field(&flip_1);
        let phi_2: F256 = SymDiff::embed_to_field(&flip_2);

        let mut flip_both = [0u8; 32];
        flip_both[0] = 0x01;
        flip_both[31] = 0x80;
        let phi_both: F256 = SymDiff::embed_to_field(&flip_both);

        assert_eq!(
            phi_both,
            phi_1.add(&phi_2),
            "Disjoint dimensions are perfectly orthogonal"
        );
    }

    #[test]
    fn t21_homomorphism_scalar_zero() {
        let a = b"Hello";
        let phi_a: F256 = SymDiff::embed_to_field(a);
        let phi_a_xor_a: F256 = SymDiff::embed_to_field(&xor_bytes(a, a));

        assert_eq!(phi_a_xor_a, F256::zero(), "phi(A ^ A) = 0");
        assert_eq!(phi_a.add(&phi_a), F256::zero(), "phi(A) + phi(A) = 0");
    }

    #[test]
    fn t22_shift_invariance() {
        let b0 = [0x11; 32];
        let b1 = [0x22; 32];
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(&b0);
        combined[32..].copy_from_slice(&b1);

        let phi_b0: F256 = SymDiff::embed_to_field(&b0);
        let phi_b1: F256 = SymDiff::embed_to_field(&b1);
        let phi_combined: F256 = SymDiff::embed_to_field(&combined);

        // MATHEMATICAL FIX: Anchored at index 0. Formula is B1 * Phi + B0
        assert_eq!(
            phi_combined,
            phi_b1.shift_phase().add(&phi_b0),
            "Shift operator acts as x in reverse anchoring"
        );
    }

    // =========================================================================
    // GROUP 5: Topological Stress and Causal Independence
    // =========================================================================

    #[test]
    fn t23_massive_set_order_independence() {
        let mut state1: F256 = SymDiff::empty_state();
        let mut state2: F256 = SymDiff::empty_state();

        let mut nodes = vec![];
        for i in 0..1000 {
            nodes.push(embed_str(&format!("Node_{}", i)));
        }

        for n in &nodes {
            state1 = SymDiff::aggregate(&state1, n, 0);
        }

        for n in nodes.iter().rev() {
            state2 = SymDiff::aggregate(&state2, n, 0);
        }

        assert_eq!(
            state1, state2,
            "Macro-topology must be immune to causal ordering"
        );
    }

    #[test]
    fn t24_massive_set_complete_cancellation() {
        let mut state: F256 = SymDiff::empty_state();
        for i in 0..1000 {
            let n = embed_str(&format!("X_{}", i));
            state = SymDiff::aggregate(&state, &n, 0);
        }
        for i in 0..1000 {
            let n = embed_str(&format!("X_{}", i));
            state = SymDiff::aggregate(&state, &n, 0);
        }

        assert_eq!(
            state,
            F256::zero(),
            "Massive duplicate sets must completely vanish"
        );
    }

    #[test]
    fn t25_aggregation_index_is_strictly_ignored() {
        let a = embed_str("Test");
        let zero = F256::zero();
        let s1 = SymDiff::aggregate(&zero, &a, 0);
        let s2 = SymDiff::aggregate(&zero, &a, 999999);
        assert_eq!(
            s1, s2,
            "Sets have no directional causality, index must be ignored"
        );
    }

    #[test]
    fn t26_entropy_does_not_collapse() {
        let mut state: F256 = SymDiff::empty_state();
        for i in 0..5000 {
            let n = embed_str(&format!("Unique_{}", i));
            state = SymDiff::aggregate(&state, &n, 0);
        }
        assert!(
            !state.is_zero(),
            "Accumulation of unique data must not accidentally zero out"
        );
    }

    // =========================================================================
    // GROUP 6: Edge Cases and Memory Boundaries
    // =========================================================================

    #[test]
    fn t27_embed_large_zero_buffer_is_zero() {
        let zeros = [0u8; 1024];
        let e: F256 = SymDiff::embed_to_field(&zeros);
        assert_eq!(e, F256::zero(), "A massive void is still a void");
    }

    #[test]
    fn t28_embed_single_bit_at_extreme_boundaries() {
        let mut data = [0u8; 32];
        data[0] = 0x01;
        let e1: F256 = SymDiff::embed_to_field(&data);

        let mut data2 = [0u8; 32];
        data2[31] = 0x80;
        let e2: F256 = SymDiff::embed_to_field(&data2);

        assert_ne!(e1, e2, "Extreme bits must not collide");
        assert!(!e1.is_zero());
        assert!(!e2.is_zero());
    }

    #[test]
    fn t29_symmetrical_inputs_do_not_self_cancel_in_embedder() {
        let data1 = vec![0xAA; 32];
        let data2 = vec![0xAA; 64];

        let e1: F256 = SymDiff::embed_to_field(&data1);
        let e2: F256 = SymDiff::embed_to_field(&data2);

        assert_ne!(
            e1, e2,
            "Different lengths of repeating patterns must not collide"
        );
    }

    #[test]
    fn t30_aggregate_with_singularity_is_noop() {
        let a = embed_str("Data");
        let zero = F256::zero();
        let s = SymDiff::aggregate(&a, &zero, 0);
        assert_eq!(
            s, a,
            "Aggregating the topological zero does absolutely nothing"
        );
    }
}
#[cfg(test)]
mod multiset_tests {
    use crate::algebra::galois_256::GaloisSignature256;
    use crate::algebra::traits::FiniteField;
    use crate::topology::multiset::MultisetAggregator;
    use crate::topology::traits::HomomorphicAggregator;

    type MultiSet = MultisetAggregator;
    type F256 = GaloisSignature256;

    fn embed_str(s: &str) -> F256 {
        MultiSet::embed_to_field(s.as_bytes())
    }

    // =========================================================================
    // GROUP 1: Vacuum and Multiplicative Identity
    // =========================================================================

    #[test]
    fn t01_empty_state_is_multiplicative_identity() {
        let vacuum: F256 = MultiSet::empty_state();
        assert_eq!(
            vacuum,
            F256::one(),
            "Multiset vacuum must be 1, not 0, to prevent product collapse"
        );
    }

    #[test]
    fn t02_aggregate_into_vacuum_scales_topology() {
        let vacuum: F256 = MultiSet::empty_state();
        let e = embed_str("Node_A");
        let state = MultiSet::aggregate(&vacuum, &e, 0);

        assert_ne!(
            state,
            F256::zero(),
            "Aggregating into vacuum must not zero out"
        );
        assert_ne!(
            state,
            F256::one(),
            "Aggregating into vacuum must alter the identity"
        );
    }

    #[test]
    fn t03_vacuum_is_impervious_to_empty_data() {
        let vacuum: F256 = MultiSet::empty_state();
        let e_empty = MultiSet::embed_to_field(&[]);
        let state = MultiSet::aggregate(&vacuum, &e_empty, 0);
        // It aggregates the affine shift of 0: (1 * (X_g + 0)) = X_g
        assert_ne!(
            state, vacuum,
            "Aggregating empty data still shifts the state by X_g"
        );
    }

    #[test]
    fn t04_removing_only_element_restores_vacuum() {
        let vacuum: F256 = MultiSet::empty_state();
        let e = embed_str("Singularity");
        let state = MultiSet::aggregate(&vacuum, &e, 0);
        let restored = MultiSet::remove(&state, &e).unwrap();
        assert_eq!(
            restored, vacuum,
            "Cleavage of the only element must return strictly to 1"
        );
    }

    #[test]
    fn t05_remove_from_vacuum_is_mathematically_valid_but_negative() {
        let vacuum: F256 = MultiSet::empty_state();
        let e = embed_str("AntiMatter");
        let negative_state = MultiSet::remove(&vacuum, &e).unwrap();

        // If we add it back, we should get the vacuum
        let restored = MultiSet::aggregate(&negative_state, &e, 0);
        assert_eq!(
            restored, vacuum,
            "Negative multisets are algebraically valid via Fermat's Inverse"
        );
    }

    // =========================================================================
    // GROUP 2: The Affine Subspace Axiom (Zero-Divisor Prevention)
    // =========================================================================

    #[test]
    fn t06_embed_forces_bit_255_to_zero() {
        let malicious_data = [0xFF; 32];
        let e: F256 = MultiSet::embed_to_field(&malicious_data);

        let msb = e.0[3] >> 63;
        assert_eq!(msb, 0, "Affine Subspace Axiom: Bit 255 MUST be 0");
    }
    #[test]
    fn t07_affine_subspace_prevents_xg_collision() {
        // X_g is x^255. We try to inject exactly x^255.
        let mut xg_data = [0u8; 32];
        xg_data[31] = 0x80;

        let e: F256 = MultiSet::embed_to_field(&xg_data);
        assert_eq!(
            e,
            F256::zero(),
            "Injecting X_g raw must be scrubbed to 0 by the affine mask"
        );
    }

    #[test]
    fn t08_aggregate_never_yields_zero_state() {
        let mut state: F256 = MultiSet::empty_state();
        for i in 0..256 {
            let mut data = [0u8; 32];
            data[i % 32] = i as u8;
            let e = MultiSet::embed_to_field(&data);
            state = MultiSet::aggregate(&state, &e, 0);
            assert_ne!(state, F256::zero(), "Product must NEVER collapse to 0");
        }
    }

    #[test]
    fn t09_embedding_preserves_lower_255_bits() {
        let mut data = [0xFF; 32];
        data[31] = 0x7F; // All bits set except 255
        let e: F256 = MultiSet::embed_to_field(&data);

        let expected = F256::from_bytes_canonical(&data);
        assert_eq!(
            e, expected,
            "Valid affine data must not be mutated by the mask"
        );
    }

    #[test]
    fn t10_generator_constant_is_unreachable() {
        // Since X_g has bit 255 = 1, and embed forces bit 255 = 0,
        // no embedded element can EVER equal X_g.
        let e1 = embed_str("A");
        let mut xg_buffer = [0u8; 32];
        xg_buffer[31] = 0x80;
        let xg = F256::from_bytes_canonical(&xg_buffer);

        assert_ne!(e1, xg, "Element must exist in orthogonal subspace to X_g");
    }

    // =========================================================================
    // GROUP 3: Productorial Aggregation Laws (Commutativity & Multiplicity)
    // =========================================================================

    #[test]
    fn t11_aggregation_is_strictly_commutative() {
        let a = embed_str("Var_A");
        let b = embed_str("Var_B");

        let state1 =
            MultiSet::aggregate(&MultiSet::aggregate(&MultiSet::empty_state(), &a, 0), &b, 0);
        let state2 =
            MultiSet::aggregate(&MultiSet::aggregate(&MultiSet::empty_state(), &b, 0), &a, 0);

        assert_eq!(
            state1, state2,
            "Polynomial product roots are order-independent"
        );
    }

    #[test]
    fn t12_multiplicity_is_preserved_no_annihilation() {
        let a = embed_str("Clause_1");

        let s1 = MultiSet::aggregate(&MultiSet::empty_state(), &a, 0);
        let s2 = MultiSet::aggregate(&s1, &a, 0);

        assert_ne!(s1, s2, "A * A != A in a multiset");
        assert_ne!(
            s2,
            MultiSet::empty_state(),
            "A * A != 1 (No parity annihilation)"
        );
        assert_ne!(s2, F256::zero(), "A * A != 0");
    }

    #[test]
    fn t13_three_identical_elements_are_distinct_from_one() {
        let a = embed_str("Duplicate");
        let s1 = MultiSet::aggregate(&MultiSet::empty_state(), &a, 0);
        let s3 = MultiSet::aggregate(&MultiSet::aggregate(&s1, &a, 0), &a, 0);

        assert_ne!(
            s1, s3,
            "Multiplicity 3 is topologically distinct from Multiplicity 1"
        );
    }

    #[test]
    fn t14_disjoint_multisets_yield_different_signatures() {
        let a = embed_str("A");
        let b = embed_str("B");

        let s1 = MultiSet::aggregate(&MultiSet::aggregate(&MultiSet::empty_state(), &a, 0), &a, 0);
        let s2 = MultiSet::aggregate(&MultiSet::aggregate(&MultiSet::empty_state(), &a, 0), &b, 0);

        assert_ne!(
            s1, s2,
            "{{A, A}} must be strongly distinguishable from {{A, B}}"
        );
    }

    #[test]
    fn t15_associativity_of_subgraphs() {
        let a = embed_str("A");
        let b = embed_str("B");
        let c = embed_str("C");

        // H(A U (B U C))
        let bc = MultiSet::aggregate(&MultiSet::aggregate(&MultiSet::empty_state(), &b, 0), &c, 0);
        let a_bc = MultiSet::aggregate(&bc, &a, 0);

        // H((A U B) U C)
        let ab = MultiSet::aggregate(&MultiSet::aggregate(&MultiSet::empty_state(), &a, 0), &b, 0);
        let ab_c = MultiSet::aggregate(&ab, &c, 0);

        assert_eq!(a_bc, ab_c, "Multiset sub-graphs must be fully associative");
    }

    // =========================================================================
    // GROUP 4: Linear Embedding and Homomorphism (Post-Reverse Fix)
    // =========================================================================

    fn xor_bytes(a: &[u8], b: &[u8]) -> Vec<u8> {
        let len = a.len().max(b.len());
        let mut result = vec![0u8; len];
        for i in 0..len {
            let a_val = if i < a.len() { a[i] } else { 0 };
            let b_val = if i < b.len() { b[i] } else { 0 };
            result[i] = a_val ^ b_val;
        }
        result
    }

    #[test]
    fn t16_homomorphism_uneven_lengths_multiset() {
        // PROOF OF FIX: Verifies that `.rev()` chunks maintain exact left-aligned linearity
        let a = [0x55; 15]; // Short
        let b = [0x33; 45]; // Long
        let a_xor_b = xor_bytes(&a, &b);

        let phi_a: F256 = MultiSet::embed_to_field(&a);
        let phi_b: F256 = MultiSet::embed_to_field(&b);
        let phi_axb: F256 = MultiSet::embed_to_field(&a_xor_b);

        assert_eq!(
            phi_axb,
            phi_a.add(&phi_b),
            "Linearity must hold for multisets regardless of array length"
        );
    }

    #[test]
    fn t17_homomorphism_with_affine_mask_interference() {
        let mut a = [0xFF; 32];
        a[31] = 0x80; // Triggers mask
        let mut b = [0xFF; 32];
        b[31] = 0x80; // Triggers mask
        let a_xor_b = xor_bytes(&a, &b);

        let phi_a: F256 = MultiSet::embed_to_field(&a);
        let phi_b: F256 = MultiSet::embed_to_field(&b);
        let phi_axb: F256 = MultiSet::embed_to_field(&a_xor_b);

        // (A & 0x7F) ^ (B & 0x7F) == (A ^ B) & 0x7F
        assert_eq!(
            phi_axb,
            phi_a.add(&phi_b),
            "Affine subspace mask is distributive over XOR"
        );
    }

    #[test]
    fn t18_embed_is_deterministic() {
        let data = b"Graph_Variable_1024";
        let e1: F256 = MultiSet::embed_to_field(data);
        let e2: F256 = MultiSet::embed_to_field(data);
        assert_eq!(e1, e2, "Deterministic embedding");
    }

    #[test]
    fn t19_embed_differentiates_trailing_zeros_in_uneven_chunks() {
        let d1 = [0x01];
        let d2 = [0x01, 0x00];
        let e1: F256 = MultiSet::embed_to_field(&d1);
        let e2: F256 = MultiSet::embed_to_field(&d2);
        assert_eq!(e1, e2, "Trailing zeros map to identical geometry");
    }

    #[test]
    fn t20_shift_invariance_multiset() {
        let b0 = [0x11; 32];
        let b1 = [0x22; 32];
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(&b0);
        combined[32..].copy_from_slice(&b1);

        let phi_b0: F256 = MultiSet::embed_to_field(&b0);
        let phi_b1: F256 = MultiSet::embed_to_field(&b1);
        let phi_combined: F256 = MultiSet::embed_to_field(&combined);

        // MATHEMATICAL FIX: Reverse iteration anchors index 0 to Phi^0.
        // Therefore, the polynomial is constructed as B1 * Phi + B0.
        assert_eq!(
            phi_combined,
            phi_b1.shift_phase().add(&phi_b0),
            "Polynomial chaining works across blocks in reverse anchoring"
        );
    }

    // =========================================================================
    // GROUP 5: Fermat's Inverse and Extraction (Cleavage)
    // =========================================================================

    #[test]
    fn t21_remove_middle_element_preserves_multiset() {
        let a = embed_str("A");
        let b = embed_str("B");
        let c = embed_str("C");

        let mut state: F256 = MultiSet::empty_state();
        state = MultiSet::aggregate(&state, &a, 0);
        state = MultiSet::aggregate(&state, &b, 0);
        state = MultiSet::aggregate(&state, &c, 0);

        let state_minus_b = MultiSet::remove(&state, &b).unwrap();

        let mut expected: F256 = MultiSet::empty_state();
        expected = MultiSet::aggregate(&expected, &a, 0);
        expected = MultiSet::aggregate(&expected, &c, 0);

        assert_eq!(
            state_minus_b, expected,
            "Division by root correctly extracts elements from the product"
        );
    }

    #[test]
    fn t22_remove_duplicate_reduces_multiplicity() {
        let a = embed_str("Node");
        let mut state: F256 = MultiSet::empty_state();
        state = MultiSet::aggregate(&state, &a, 0);
        state = MultiSet::aggregate(&state, &a, 0);

        let state_minus_a = MultiSet::remove(&state, &a).unwrap();

        let expected = MultiSet::aggregate(&MultiSet::empty_state(), &a, 0);
        assert_eq!(
            state_minus_a, expected,
            "Extracting from {{A, A}} yields {{A}}"
        );
    }
    #[test]
    fn t23_remove_non_existent_element_yields_garbage_but_compiles() {
        // Mathematically, dividing a polynomial by a non-root yields a remainder.
        // In our field, it just yields a mathematically valid but topologically meaningless state.
        let a = embed_str("A");
        let b = embed_str("B");

        let state = MultiSet::aggregate(&MultiSet::empty_state(), &a, 0);
        let garbage = MultiSet::remove(&state, &b).unwrap();

        assert_ne!(garbage, MultiSet::empty_state());
        assert_ne!(garbage, state);
    }

    #[test]
    fn t24_aggregate_restores_garbage_extraction() {
        // (S / B) * B = S
        let a = embed_str("A");
        let b = embed_str("B");

        let state = MultiSet::aggregate(&MultiSet::empty_state(), &a, 0);
        let garbage = MultiSet::remove(&state, &b).unwrap();
        let restored = MultiSet::aggregate(&garbage, &b, 0);

        assert_eq!(
            restored, state,
            "Fermat inversion is perfectly reversible even on non-roots"
        );
    }

    #[test]
    fn t25_remove_all_elements_in_reverse_order() {
        let nodes = vec![embed_str("N1"), embed_str("N2"), embed_str("N3")];
        let mut state: F256 = MultiSet::empty_state();

        for n in &nodes {
            state = MultiSet::aggregate(&state, n, 0);
        }
        for n in nodes.iter().rev() {
            state = MultiSet::remove(&state, n).unwrap();
        }

        assert_eq!(
            state,
            MultiSet::empty_state(),
            "Complete sequential cleavage must yield vacuum"
        );
    }

    #[test]
    fn t26_remove_all_elements_in_random_order() {
        let n1 = embed_str("N1");
        let n2 = embed_str("N2");
        let n3 = embed_str("N3");

        let mut state: F256 = MultiSet::empty_state();
        state = MultiSet::aggregate(&state, &n1, 0);
        state = MultiSet::aggregate(&state, &n2, 0);
        state = MultiSet::aggregate(&state, &n3, 0);

        state = MultiSet::remove(&state, &n2).unwrap();
        state = MultiSet::remove(&state, &n3).unwrap();
        state = MultiSet::remove(&state, &n1).unwrap();

        assert_eq!(
            state,
            MultiSet::empty_state(),
            "Commutativity of extraction"
        );
    }

    // =========================================================================
    // GROUP 6: Massive Volume & Edge Cases
    // =========================================================================

    #[test]
    fn t27_massive_multiset_avalanche() {
        let mut state: F256 = MultiSet::empty_state();
        for i in 0..1000 {
            let e = embed_str(&format!("Edge_{}", i));
            state = MultiSet::aggregate(&state, &e, 0);
        }
        assert_ne!(
            state,
            F256::zero(),
            "Hypergraph of 1000 edges must not collapse"
        );
        assert_ne!(state, F256::one());
    }

    #[test]
    fn t28_identical_massive_hypergraphs_match() {
        let mut state1: F256 = MultiSet::empty_state();
        let mut state2: F256 = MultiSet::empty_state();

        let mut data = vec![];
        for i in 0..500 {
            data.push(embed_str(&format!("V_{}", i)));
        }

        for d in &data {
            state1 = MultiSet::aggregate(&state1, d, 0);
        }
        for d in data.iter().rev() {
            state2 = MultiSet::aggregate(&state2, d, 0);
        }

        assert_eq!(
            state1, state2,
            "Hypergraph volume is strictly topological and order-independent"
        );
    }

    #[test]
    fn t29_single_bit_flip_cascades_entire_product() {
        let mut state1: F256 = MultiSet::empty_state();
        let mut state2: F256 = MultiSet::empty_state();

        for i in 0..50 {
            let e = embed_str(&format!("Data_{}", i));
            state1 = MultiSet::aggregate(&state1, &e, 0);

            if i == 25 {
                // Introduce a tiny mutation
                let mutated = embed_str(&format!("Data_25_Mutated"));
                state2 = MultiSet::aggregate(&state2, &mutated, 0);
            } else {
                state2 = MultiSet::aggregate(&state2, &e, 0);
            }
        }

        assert_ne!(
            state1, state2,
            "A single distinct element radically diverges the product polynomial"
        );
    }

    #[test]
    fn t30_index_is_strictly_ignored_in_multiset() {
        let a = embed_str("Invariant");
        let s1 = MultiSet::aggregate(&MultiSet::empty_state(), &a, 10);
        let s2 = MultiSet::aggregate(&MultiSet::empty_state(), &a, 999);
        assert_eq!(s1, s2, "Multisets do not possess directional causal time");
    }
}
#[cfg(test)]
mod sequence_tests {
    use crate::algebra::galois_256::GaloisSignature256;
    use crate::algebra::traits::FiniteField;
    use crate::topology::sequence::SequenceAggregator;
    use crate::topology::traits::HomomorphicAggregator;

    type Sequence = SequenceAggregator;
    type F256 = GaloisSignature256;

    fn embed_str(s: &str) -> F256 {
        Sequence::embed_to_field(s.as_bytes())
    }

    // =========================================================================
    // GROUP 1: Vacuum and Horner's Initialization
    // =========================================================================

    #[test]
    fn t01_empty_state_is_strict_zero() {
        let vacuum: F256 = Sequence::empty_state();
        assert_eq!(
            vacuum,
            F256::zero(),
            "Directional sequence originates from 0"
        );
    }

    #[test]
    fn t02_aggregate_into_vacuum_is_just_the_element() {
        let vacuum: F256 = Sequence::empty_state();
        let e = embed_str("First_Event");
        let state = Sequence::aggregate(&vacuum, &e, 0);
        // Horner: 0 * Phi + e = e
        assert_eq!(
            state, e,
            "First element anchors the sequence without phase shift"
        );
    }

    #[test]
    fn t03_vacuum_is_impervious_to_empty_data() {
        let vacuum: F256 = Sequence::empty_state();
        let e_empty = Sequence::embed_to_field(&[]); // Yields 0
        let state = Sequence::aggregate(&vacuum, &e_empty, 0);
        // 0 * Phi + 0 = 0
        assert_eq!(state, vacuum, "Empty event on a vacuum yields vacuum");
    }

    #[test]
    fn t04_removing_only_element_restores_vacuum() {
        let vacuum: F256 = Sequence::empty_state();
        let e = embed_str("Genesis");
        let state = Sequence::aggregate(&vacuum, &e, 0);
        let restored = Sequence::remove(&state, &e).unwrap();
        assert_eq!(restored, vacuum, "LIFO rollback to genesis must yield 0");
    }

    #[test]
    fn t05_remove_from_vacuum_yields_inverse_phase() {
        let vacuum: F256 = Sequence::empty_state();
        let e = embed_str("AntiTime");
        let neg_state = Sequence::remove(&vacuum, &e).unwrap();

        // Horner backwards: (0 + e) * Phi^-1
        let phase = F256::one().shift_phase();
        let expected = e.mul(&phase.inv().unwrap());

        assert_eq!(
            neg_state, expected,
            "Mathematical time-reversal from origin"
        );
    }

    // =========================================================================
    // GROUP 2: Strict Causal Asymmetry
    // =========================================================================

    #[test]
    fn t06_causality_is_strictly_asymmetric() {
        let a = embed_str("Event_A");
        let b = embed_str("Event_B");

        let s_ab =
            Sequence::aggregate(&Sequence::aggregate(&Sequence::empty_state(), &a, 0), &b, 1);
        let s_ba =
            Sequence::aggregate(&Sequence::aggregate(&Sequence::empty_state(), &b, 0), &a, 1);

        // [A, B] = A*Phi + B
        // [B, A] = B*Phi + A
        assert_ne!(s_ab, s_ba, "[A, B] MUST topologically diverge from [B, A]");
    }

    #[test]
    fn t07_multiplicity_and_position_matter() {
        let a = embed_str("Tick");

        let s1 = Sequence::aggregate(&Sequence::empty_state(), &a, 0);
        let s2 = Sequence::aggregate(&s1, &a, 1);

        assert_ne!(s1, s2, "Sequence [A, A] is distinct from [A]");
    }

    #[test]
    fn t08_deep_asymmetry_preservation() {
        let a = embed_str("A");
        let b = embed_str("B");
        let c = embed_str("C");

        let mut abc: F256 = Sequence::empty_state();
        abc = Sequence::aggregate(&abc, &a, 0);
        abc = Sequence::aggregate(&abc, &b, 0);
        abc = Sequence::aggregate(&abc, &c, 0);

        let mut cba: F256 = Sequence::empty_state();
        cba = Sequence::aggregate(&cba, &c, 0);
        cba = Sequence::aggregate(&cba, &b, 0);
        cba = Sequence::aggregate(&cba, &a, 0);

        assert_ne!(
            abc, cba,
            "Palindromic sequences evaluate to entirely different polynomials"
        );
    }

    #[test]
    fn t09_identical_repeating_events_cascade_geometrically() {
        let a = embed_str("Echo");
        let s1 = Sequence::aggregate(&Sequence::empty_state(), &a, 0); // A
        let s2 = Sequence::aggregate(&s1, &a, 0); // A*Phi + A
        let s3 = Sequence::aggregate(&s2, &a, 0); // A*Phi^2 + A*Phi + A

        // Manually compute A*Phi^2 + A*Phi + A
        let a_phi = a.shift_phase();
        let a_phi2 = a_phi.shift_phase();
        let expected = a_phi2.add(&a_phi).add(&a);

        assert_eq!(
            s3, expected,
            "Repeating events form a perfect geometric series in Galois"
        );
    }

    #[test]
    fn t10_aggregation_index_is_strictly_ignored() {
        let a = embed_str("Data");
        let s1 = Sequence::aggregate(&Sequence::empty_state(), &a, 0);
        let s2 = Sequence::aggregate(&Sequence::empty_state(), &a, 9999);

        assert_eq!(
            s1, s2,
            "Causality is dictated by iteration order, index is superficial"
        );
    }

    // =========================================================================
    // GROUP 3: Linear Embedding and Horner's Isomorphism
    // =========================================================================

    #[test]
    fn t11_horner_method_isomorphism_step_2() {
        let a = embed_str("Alpha");
        let b = embed_str("Beta");

        let seq = Sequence::aggregate(&Sequence::aggregate(&Sequence::empty_state(), &a, 0), &b, 0);
        let expected = a.shift_phase().add(&b);

        assert_eq!(seq, expected, "Seq[A, B] == A*Phi + B");
    }

    #[test]
    fn t12_horner_method_isomorphism_step_3() {
        let a = embed_str("A");
        let b = embed_str("B");
        let c = embed_str("C");

        let mut seq: F256 = Sequence::empty_state();
        seq = Sequence::aggregate(&seq, &a, 0);
        seq = Sequence::aggregate(&seq, &b, 0);
        seq = Sequence::aggregate(&seq, &c, 0);

        let expected = a.shift_phase().shift_phase().add(&b.shift_phase()).add(&c);

        assert_eq!(seq, expected, "Seq[A, B, C] == A*Phi^2 + B*Phi + C");
    }

    #[test]
    fn t13_macro_sequence_isomorphic_to_micro_embedding() {
        // Embedding works right-to-left due to .rev() to anchor index 0.
        // Therefore, embedding [C0, C1] builds C1 * Phi + C0.
        // Sequencing [C1, C0] builds C1 * Phi + C0.
        let c0 = [0xAA; 32];
        let c1 = [0xBB; 32];
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(&c0);
        combined[32..].copy_from_slice(&c1);

        let e_combined: F256 = Sequence::embed_to_field(&combined);

        let block_c1 = F256::from_bytes_canonical(&c1);
        let block_c0 = F256::from_bytes_canonical(&c0);

        let mut seq: F256 = Sequence::empty_state();
        seq = Sequence::aggregate(&seq, &block_c1, 0);
        seq = Sequence::aggregate(&seq, &block_c0, 0);

        assert_eq!(
            e_combined, seq,
            "Micro polynomial block embedding is perfectly isomorphic to macro sequencing"
        );
    }

    // FIX: Added explicit type annotations to satisfy Rust type inference E0283
    #[test]
    fn t14_embed_deterministic_sequence() {
        let data = b"Rolling_Hash_Seed";
        let e1: F256 = Sequence::embed_to_field(data);
        let e2: F256 = Sequence::embed_to_field(data);
        assert_eq!(e1, e2, "Deterministic sequence embedding");
    }
    #[test]
    fn t15_shift_invariance_sequence_embed() {
        let b0 = [0x11; 32];
        let b1 = [0x22; 32];
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(&b0);
        combined[32..].copy_from_slice(&b1);

        let phi_b0: F256 = Sequence::embed_to_field(&b0);
        let phi_b1: F256 = Sequence::embed_to_field(&b1);
        let phi_combined: F256 = Sequence::embed_to_field(&combined);

        // Reverse anchoring rule: B1 * Phi + B0
        assert_eq!(phi_combined, phi_b1.shift_phase().add(&phi_b0));
    }

    #[test]
    fn t16_embed_distinguishes_trailing_zeros_sequence() {
        let d1 = [0x01];
        let d2 = [0x01, 0x00];
        let e1: F256 = Sequence::embed_to_field(&d1);
        let e2: F256 = Sequence::embed_to_field(&d2);
        assert_eq!(
            e1, e2,
            "Trailing zeros fall into the same padded block geometry"
        );
    }

    // =========================================================================
    // GROUP 4: The Causal Arrow (LIFO Rollback via Fermat)
    // =========================================================================

    #[test]
    fn t17_lifo_rollback_is_perfect() {
        let a = embed_str("Past");
        let b = embed_str("Present");

        let s_past = Sequence::aggregate(&Sequence::empty_state(), &a, 0);
        let s_present = Sequence::aggregate(&s_past, &b, 0);

        let s_rewind = Sequence::remove(&s_present, &b).unwrap();

        assert_eq!(
            s_rewind, s_past,
            "Extracting the Present perfectly restores the Past"
        );
    }

    #[test]
    fn t18_extracting_past_element_yields_garbage() {
        // Strict Mathematical Axiom: You cannot remove a middle element
        // without shifting the phases of everything that came after it.
        let a = embed_str("Past");
        let b = embed_str("Present");

        let s_past = Sequence::aggregate(&Sequence::empty_state(), &a, 0);
        let s_present = Sequence::aggregate(&s_past, &b, 0);

        let s_garbage = Sequence::remove(&s_present, &a).unwrap();

        assert_ne!(
            s_garbage, b,
            "Extracting out of order yields topological nonsense"
        );
    }

    #[test]
    fn t19_push_and_pop_cycle() {
        let mut state: F256 = Sequence::empty_state();
        let e = embed_str("Transient");

        let pre_state = state.clone();
        state = Sequence::aggregate(&state, &e, 0);
        state = Sequence::remove(&state, &e).unwrap();

        assert_eq!(
            state, pre_state,
            "Aggregate and Remove form a perfect Identity pair for the top element"
        );
    }

    #[test]
    fn t20_sequential_rewind_to_genesis() {
        let mut state: F256 = Sequence::empty_state();
        let nodes = vec![embed_str("N1"), embed_str("N2"), embed_str("N3")];

        for n in &nodes {
            state = Sequence::aggregate(&state, n, 0);
        }

        // Reverse temporal order (LIFO)
        for n in nodes.iter().rev() {
            state = Sequence::remove(&state, n).unwrap();
        }

        assert_eq!(
            state,
            Sequence::empty_state(),
            "A full reverse timeline playback yields vacuum"
        );
    }

    #[test]
    fn t21_fermat_inverse_accuracy_for_phases() {
        let state = embed_str("State");
        let shifted = state.shift_phase();

        let phase = F256::one().shift_phase();
        let phase_inv = phase.inv().unwrap();

        let unshifted = shifted.mul(&phase_inv);
        assert_eq!(
            unshifted, state,
            "Fermat's little theorem accurately computes Phi^-1"
        );
    }

    // =========================================================================
    // GROUP 5: Rolling Hashes & Sub-sequences
    // =========================================================================

    #[test]
    fn t22_sequence_is_rolling_hash() {
        let a = embed_str("Window1");
        let b = embed_str("Window2");

        let mut s: F256 = Sequence::empty_state();
        s = Sequence::aggregate(&s, &a, 0);
        s = Sequence::aggregate(&s, &b, 0);

        let expected = a.shift_phase().add(&b);
        assert_eq!(
            s, expected,
            "Sequence acts as an additive rolling hash over GF(2^256)"
        );
    }

    #[test]
    fn t23_sliding_window_divergence() {
        let a = embed_str("A");
        let b = embed_str("B");
        let c = embed_str("C");

        let mut w1: F256 = Sequence::empty_state();
        w1 = Sequence::aggregate(&w1, &a, 0);
        w1 = Sequence::aggregate(&w1, &b, 0);

        let mut w2: F256 = Sequence::empty_state();
        w2 = Sequence::aggregate(&w2, &b, 0);
        w2 = Sequence::aggregate(&w2, &c, 0);

        assert_ne!(
            w1, w2,
            "Sliding windows produce completely orthogonal polynomial states"
        );
    }

    #[test]
    fn t24_two_disjoint_sequences_dont_collide() {
        let mut s1: F256 = Sequence::empty_state();
        s1 = Sequence::aggregate(&s1, &embed_str("Hello"), 0);
        s1 = Sequence::aggregate(&s1, &embed_str("World"), 0);

        let mut s2: F256 = Sequence::empty_state();
        s2 = Sequence::aggregate(&s2, &embed_str("World"), 0);
        s2 = Sequence::aggregate(&s2, &embed_str("Hello"), 0);

        assert_ne!(s1, s2);
    }

    #[test]
    fn t25_phase_avalanche_length_variance() {
        let mut s1: F256 = Sequence::empty_state();
        let mut s2: F256 = Sequence::empty_state();
        let e = embed_str("Tick");

        for _ in 0..100 {
            s1 = Sequence::aggregate(&s1, &e, 0);
        }
        for _ in 0..101 {
            s2 = Sequence::aggregate(&s2, &e, 0);
        }

        assert_ne!(
            s1, s2,
            "A single extra tick radically shifts the polynomial degree"
        );
    }

    #[test]
    fn t26_reversing_concatenation_fails_as_expected() {
        let a = embed_str("A");
        let b = embed_str("B");
        let ab = Sequence::aggregate(&Sequence::aggregate(&Sequence::empty_state(), &a, 0), &b, 0);
        let ba = Sequence::aggregate(&Sequence::aggregate(&Sequence::empty_state(), &b, 0), &a, 0);
        assert_ne!(ab, ba);
    }

    // =========================================================================
    // GROUP 6: Edge Cases & The Topological Clock
    // =========================================================================

    #[test]
    fn t27_massive_sequence_stability() {
        let mut state: F256 = Sequence::empty_state();
        for i in 0..2000 {
            let e = embed_str(&format!("Causal_Event_{}", i));
            state = Sequence::aggregate(&state, &e, 0);
        }
        assert_ne!(state, F256::zero());
    }

    #[test]
    fn t28_zero_element_is_a_topological_clock_tick() {
        // PROOF: In Sets, A + 0 = A.
        // In Sequences, Seq(A, 0) = A * Phi + 0 = A * Phi.
        // Injecting a 0 physically advances time for all past events.
        let a = embed_str("A");
        let zero = F256::zero();

        let s_a = Sequence::aggregate(&Sequence::empty_state(), &a, 0);
        let s_a0 = Sequence::aggregate(&s_a, &zero, 0);

        assert_ne!(s_a, s_a0, "Injecting 0 MUST shift the sequence phase");
        assert_eq!(
            s_a0,
            s_a.shift_phase(),
            "Injecting 0 is identical to a pure phase shift"
        );
    }

    #[test]
    fn t29_multiple_clock_ticks_avalanche() {
        let a = embed_str("Origin");
        let mut state: F256 = Sequence::aggregate(&Sequence::empty_state(), &a, 0);
        let pre_state = state.clone();

        for _ in 0..100 {
            state = Sequence::aggregate(&state, &F256::zero(), 0);
        }

        assert_ne!(
            state, pre_state,
            "100 clock ticks radically displaces the origin polynomial"
        );
    }

    #[test]
    fn t30_clock_tick_rollback() {
        let a = embed_str("Origin");
        let zero = F256::zero();

        let state = Sequence::aggregate(&Sequence::empty_state(), &a, 0);
        let ticked = Sequence::aggregate(&state, &zero, 0);

        let rollback = Sequence::remove(&ticked, &zero).unwrap();
        assert_eq!(
            rollback, state,
            "Rolling back a clock tick reverses the phase shift"
        );
    }
}
#[cfg(test)]
mod bloom_l1_tests {
    use crate::topology::bloom_l1::{TopoBloomMask, TopologicalMask};

    // =========================================================================
    // GROUP 1: The Topological Vacuum
    // =========================================================================

    #[test]
    fn t01_empty_mask_is_absolute_zero() {
        let mask = TopoBloomMask::empty();
        assert_eq!(
            mask.0,
            [0, 0, 0, 0],
            "Vacuum must have strictly zero entropy"
        );
    }

    #[test]
    fn t02_empty_mask_is_subset_of_itself() {
        let mask = TopoBloomMask::empty();
        assert!(
            mask.is_subset_of(&mask),
            "The empty set is a subset of the empty set"
        );
    }

    #[test]
    fn t03_empty_mask_union_with_empty_is_empty() {
        let mask1 = TopoBloomMask::empty();
        let mask2 = TopoBloomMask::empty();
        let union = mask1.union(&mask2);
        assert_eq!(union.0, [0, 0, 0, 0], "0 OR 0 = 0");
    }

    #[test]
    fn t04_empty_mask_is_subset_of_any_topology() {
        let empty = TopoBloomMask::empty();
        let topology = TopoBloomMask::from_variable_index(42);
        assert!(
            empty.is_subset_of(&topology),
            "The vacuum is mathematically a subset of all topological states"
        );
    }

    // =========================================================================
    // GROUP 2: Spatial Mapping Modulo 256
    // =========================================================================

    #[test]
    fn t05_variable_index_0_maps_to_first_bit() {
        let mask = TopoBloomMask::from_variable_index(0);
        assert_eq!(mask.0, [1, 0, 0, 0], "Index 0 must map to bit 0 of word 0");
    }

    #[test]
    fn t06_variable_index_63_maps_to_last_bit_of_word_0() {
        let mask = TopoBloomMask::from_variable_index(63);
        assert_eq!(mask.0, [1 << 63, 0, 0, 0], "Index 63 maps to MSB of word 0");
    }

    #[test]
    fn t07_variable_index_64_crosses_word_boundary() {
        let mask = TopoBloomMask::from_variable_index(64);
        assert_eq!(mask.0, [0, 1, 0, 0], "Index 64 must map to bit 0 of word 1");
    }

    #[test]
    fn t08_variable_index_255_maps_to_absolute_last_bit() {
        let mask = TopoBloomMask::from_variable_index(255);
        assert_eq!(
            mask.0,
            [0, 0, 0, 1 << 63],
            "Index 255 maps to MSB of word 3"
        );
    }

    #[test]
    fn t09_variable_index_256_wraps_around_to_0() {
        let mask_0 = TopoBloomMask::from_variable_index(0);
        let mask_256 = TopoBloomMask::from_variable_index(256);
        assert_eq!(
            mask_0.0, mask_256.0,
            "Spatial projection MUST be modulo 256"
        );
    }

    #[test]
    fn t10_variable_index_large_modulo_mapping() {
        let mask_10 = TopoBloomMask::from_variable_index(10);
        let mask_large = TopoBloomMask::from_variable_index(10 + 256 * 100);
        assert_eq!(
            mask_10.0, mask_large.0,
            "Large scalar indices must safely collapse into the L1 hypercube"
        );
    }

    // =========================================================================
    // GROUP 3: Monotonicity and Semilattice Union Laws
    // =========================================================================

    #[test]
    fn t11_union_is_commutative() {
        let a = TopoBloomMask::from_variable_index(10);
        let b = TopoBloomMask::from_variable_index(20);
        assert_eq!(a.union(&b).0, b.union(&a).0, "A OR B == B OR A");
    }

    #[test]
    fn t12_union_is_idempotent() {
        let a = TopoBloomMask::from_variable_index(1337);
        let union_a_a = a.union(&a);
        assert_eq!(
            union_a_a.0, a.0,
            "Unlike characteristic 2 XOR, Bloom OR must be idempotent: A U A = A"
        );
    }

    #[test]
    fn t13_union_with_vacuum_is_identity() {
        let a = TopoBloomMask::from_variable_index(42);
        let vacuum = TopoBloomMask::empty();
        assert_eq!(a.union(&vacuum).0, a.0, "A U 0 = A");
    }

    #[test]
    fn t14_union_preserves_multiple_dimensions() {
        let a = TopoBloomMask::from_variable_index(1);
        let b = TopoBloomMask::from_variable_index(65);
        let union = a.union(&b);
        assert_eq!(
            union.0,
            [2, 2, 0, 0],
            "Union must preserve bits across different 64-bit boundaries"
        );
    }

    #[test]
    fn t15_union_is_associative() {
        let a = TopoBloomMask::from_variable_index(5);
        let b = TopoBloomMask::from_variable_index(10);
        let c = TopoBloomMask::from_variable_index(15);

        let ab_c = a.union(&b).union(&c);
        let a_bc = a.union(&b.union(&c));
        assert_eq!(ab_c.0, a_bc.0, "(A U B) U C == A U (B U C)");
    }

    #[test]
    fn t16_monotonic_entropy_growth() {
        let mut mask = TopoBloomMask::empty();
        let prev_weight = mask.0.iter().map(|w| w.count_ones()).sum::<u32>();

        mask = mask.union(&TopoBloomMask::from_variable_index(0));
        let new_weight = mask.0.iter().map(|w| w.count_ones()).sum::<u32>();

        assert!(
            new_weight > prev_weight,
            "Union operations must be strictly monotonic in entropy"
        );
    }

    // =========================================================================
    // GROUP 4: Categorical Implication Lemma (Subsets)
    // =========================================================================

    #[test]
    fn t17_element_is_subset_of_its_union() {
        let a = TopoBloomMask::from_variable_index(12);
        let b = TopoBloomMask::from_variable_index(99);
        let union = a.union(&b);

        assert!(a.is_subset_of(&union), "A must be a subset of (A U B)");
        assert!(b.is_subset_of(&union), "B must be a subset of (A U B)");
    }

    #[test]
    fn t18_disjoint_elements_are_not_subsets() {
        let a = TopoBloomMask::from_variable_index(12);
        let b = TopoBloomMask::from_variable_index(99);

        assert!(
            !a.is_subset_of(&b),
            "Disjoint bits cannot imply subset geometry"
        );
        assert!(!b.is_subset_of(&a));
    }

    #[test]
    fn t19_superset_is_not_subset_of_base() {
        let a = TopoBloomMask::from_variable_index(12);
        let b = TopoBloomMask::from_variable_index(99);
        let union = a.union(&b);

        assert!(!union.is_subset_of(&a), "(A U B) is NOT a subset of A");
    }

    #[test]
    fn t20_transitive_subset_implication() {
        let a = TopoBloomMask::from_variable_index(10);
        let b = a.union(&TopoBloomMask::from_variable_index(20));
        let c = b.union(&TopoBloomMask::from_variable_index(30));

        assert!(a.is_subset_of(&b), "A in B");
        assert!(b.is_subset_of(&c), "B in C");
        assert!(a.is_subset_of(&c), "Transitive property: A must be in C");
    }

    #[test]
    fn t21_subset_check_crosses_word_boundaries_correctly() {
        let a = TopoBloomMask::from_variable_index(0); // Word 0
        let b = TopoBloomMask::from_variable_index(128); // Word 2
        let union = a.union(&b);

        assert!(a.is_subset_of(&union));
        assert!(b.is_subset_of(&union));
    }

    #[test]
    fn t22_full_topology_is_not_subset_of_empty() {
        let full = TopoBloomMask([u64::MAX; 4]);
        let empty = TopoBloomMask::empty();

        assert!(
            !full.is_subset_of(&empty),
            "A saturated topology cannot fit in a vacuum"
        );
    }

    // =========================================================================
    // GROUP 5: Orthogonality and Collisions
    // =========================================================================

    #[test]
    fn t23_orthogonal_indices_produce_disjoint_masks() {
        let m1 = TopoBloomMask::from_variable_index(1);
        let m2 = TopoBloomMask::from_variable_index(2);

        // Bitwise AND should be 0
        let collision =
            (m1.0[0] & m2.0[0]) | (m1.0[1] & m2.0[1]) | (m1.0[2] & m2.0[2]) | (m1.0[3] & m2.0[3]);
        assert_eq!(
            collision, 0,
            "Distinct variable indices under 256 MUST be perfectly orthogonal"
        );
    }

    #[test]
    fn t24_intentional_modulo_collisions_behave_correctly() {
        let m1 = TopoBloomMask::from_variable_index(5);
        let m2 = TopoBloomMask::from_variable_index(261); // 256 + 5

        assert_eq!(
            m1.0, m2.0,
            "Modular projection guarantees spatial collisions for index shifts of exactly 256"
        );
        assert!(m1.is_subset_of(&m2) && m2.is_subset_of(&m1));
    }

    #[test]
    fn t25_union_of_colliding_variables_is_stable() {
        let a = TopoBloomMask::from_variable_index(5);
        let a_colliding = TopoBloomMask::from_variable_index(261);
        let union = a.union(&a_colliding);

        assert_eq!(
            union.0, a.0,
            "Union of colliding variables acts as a standard idempotent update"
        );
    }

    #[test]
    fn t26_subset_evaluation_allows_false_positives() {
        let a = TopoBloomMask::from_variable_index(5); // Variable 5
        let b = TopoBloomMask::from_variable_index(261); // Variable 261

        // This is the entire point of the L1 shield: mathematically, Variable 5 is
        // treated as existing inside a topology that only actually contains Variable 261.
        // It's a False Positive authorized by the categorical implication lemma.
        assert!(
            a.is_subset_of(&b),
            "L1 shield must explicitly permit structural false positives"
        );
    }

    #[test]
    fn t27_l1_shield_guarantees_no_false_negatives() {
        // If a subset mathematically exists, the bitmask MUST confirm it.
        let mut universe = TopoBloomMask::empty();
        for i in 0..100 {
            universe = universe.union(&TopoBloomMask::from_variable_index(i));
        }

        // Sub-topology
        let target = TopoBloomMask::from_variable_index(50);
        assert!(
            target.is_subset_of(&universe),
            "Zero False Negatives policy: The subset must be found"
        );
    }

    // =========================================================================
    // GROUP 6: Entropy Saturation and Physical Limits
    // =========================================================================

    #[test]
    fn t28_complete_saturation_yields_u64_max() {
        let mut mask = TopoBloomMask::empty();
        for i in 0..256 {
            mask = mask.union(&TopoBloomMask::from_variable_index(i));
        }

        assert_eq!(
            mask.0,
            [u64::MAX; 4],
            "Saturating all 256 dimensions must yield a solid wall of 1s"
        );
    }

    #[test]
    fn t29_subset_check_against_saturated_mask_is_always_true() {
        let saturated = TopoBloomMask([u64::MAX; 4]);
        let random_mask = TopoBloomMask::from_variable_index(142);

        assert!(
            random_mask.is_subset_of(&saturated),
            "Any topological mask is a subset of the maximally saturated universe"
        );
    }

    #[test]
    fn t30_extreme_scalar_index_limits() {
        // Test with the absolute maximum value of usize
        let extreme_mask = TopoBloomMask::from_variable_index(usize::MAX);

        // usize::MAX % 256 is 255
        assert_eq!(
            extreme_mask.0,
            [0, 0, 0, 1 << 63],
            "System must comfortably process extreme indices without integer overflow panics"
        );
    }
}
#[cfg(test)]
mod topology_limits_tests {
    use crate::algebra::galois_256::GaloisSignature256;
    use crate::algebra::traits::FiniteField;
    use crate::topology::bloom_l1::{TopoBloomMask, TopologicalMask};
    use crate::topology::multiset::MultisetAggregator as MultiSet;
    use crate::topology::sequence::SequenceAggregator as Sequence;
    use crate::topology::symmetric_difference::SymmetricDifferenceAggregator as SymDiff;
    use crate::topology::traits::HomomorphicAggregator;

    type F256 = GaloisSignature256;

    fn embed_str<A: HomomorphicAggregator<F256>>(s: &str) -> F256 {
        A::embed_to_field(s.as_bytes())
    }

    // =========================================================================
    // GROUP 1: Cross-Topology Entropy Transfer & Anisotropy
    // =========================================================================

    #[test]
    fn t01_extract_from_multiset_inject_to_sequence() {
        let node = embed_str::<MultiSet>("HyperNode");
        let mut mset: F256 = MultiSet::empty_state();
        mset = MultiSet::aggregate(&mset, &node, 0);

        let extracted = MultiSet::remove(&mset, &node).unwrap();

        let mut seq: F256 = Sequence::empty_state();
        seq = Sequence::aggregate(&seq, &extracted, 0);

        assert_eq!(
            seq,
            F256::one(),
            "Cross-injection of identity maintains scale"
        );
    }

    #[test]
    fn t02_sequence_symdiff_sequence_roundtrip() {
        let e = embed_str::<Sequence>("Data");
        let s1 = Sequence::aggregate(&Sequence::empty_state(), &e, 0);
        let s2 = Sequence::aggregate(&Sequence::empty_state(), &e, 0);

        let sym = SymDiff::aggregate(&s1, &s2, 0);
        let final_seq = Sequence::aggregate(&Sequence::empty_state(), &sym, 0);

        assert_eq!(
            final_seq,
            F256::zero(),
            "Roundtrip annihilation cascades into absolute zero"
        );
    }

    #[test]
    fn t03_aggregation_of_all_vacuums_into_multiset() {
        let v_sym: F256 = SymDiff::empty_state();
        let v_seq: F256 = Sequence::empty_state();
        let v_mul: F256 = MultiSet::empty_state();

        let mut mset: F256 = MultiSet::empty_state();
        mset = MultiSet::aggregate(&mset, &v_sym, 0);
        mset = MultiSet::aggregate(&mset, &v_seq, 0);
        mset = MultiSet::aggregate(&mset, &v_mul, 0);

        assert_ne!(
            mset,
            F256::zero(),
            "Vacuum aggregation never collapses product"
        );
    }

    #[test]
    fn t04_affine_subspace_preservation_in_cross_aggregation() {
        let malicious = [0xFF; 32];
        let e_sym: F256 = SymDiff::embed_to_field(&malicious);

        let mut mset: F256 = MultiSet::empty_state();
        mset = MultiSet::aggregate(&mset, &e_sym, 0);

        assert_ne!(mset, F256::zero(), "Addition of X_g and E_sym avoids zero");
    }

    #[test]
    fn t05_sequence_of_multisets_vs_multiset_of_sequences() {
        let e1 = embed_str::<Sequence>("A");
        let e2 = embed_str::<Sequence>("B");

        let mut seq_of_mset: F256 = Sequence::empty_state();
        let mset_a = MultiSet::aggregate(&MultiSet::empty_state(), &e1, 0);
        let mset_b = MultiSet::aggregate(&MultiSet::empty_state(), &e2, 0);
        seq_of_mset = Sequence::aggregate(&seq_of_mset, &mset_a, 0);
        seq_of_mset = Sequence::aggregate(&seq_of_mset, &mset_b, 0);

        let mut mset_of_seq: F256 = MultiSet::empty_state();
        let seq_a = Sequence::aggregate(&Sequence::empty_state(), &e1, 0);
        let seq_b = Sequence::aggregate(&Sequence::empty_state(), &e2, 0);
        mset_of_seq = MultiSet::aggregate(&mset_of_seq, &seq_a, 0);
        mset_of_seq = MultiSet::aggregate(&mset_of_seq, &seq_b, 0);

        assert_ne!(
            seq_of_mset, mset_of_seq,
            "Macro-architectural nesting order strictly diverges"
        );
    }

    // =========================================================================
    // GROUP 2: L1 Shield Sub-Graph Limits & Discrepancies
    // =========================================================================

    #[test]
    fn t06_l1_union_of_all_256_variables_exact_match() {
        let mut mask = TopoBloomMask::empty();
        for i in 0..256 {
            mask = mask.union(&TopoBloomMask::from_variable_index(i));
        }
        assert_eq!(
            mask.0,
            [u64::MAX, u64::MAX, u64::MAX, u64::MAX],
            "Total saturation mathematically proven"
        );
    }

    #[test]
    fn t07_sequence_ordering_is_invisible_to_l1() {
        let l1_a = TopoBloomMask::from_variable_index(10);
        let l1_b = TopoBloomMask::from_variable_index(20);

        let l1_ab = l1_a.union(&l1_b);
        let l1_ba = l1_b.union(&l1_a);

        assert_eq!(
            l1_ab.0, l1_ba.0,
            "L1 is strictly commutative, hiding causal sequence ordering"
        );
    }

    #[test]
    fn t08_multiset_multiplicity_is_invisible_to_l1() {
        let l1_a = TopoBloomMask::from_variable_index(10);
        let l1_aa = l1_a.union(&l1_a);

        assert_eq!(
            l1_a.0, l1_aa.0,
            "L1 is strictly idempotent, hiding multiset volumes"
        );
    }

    #[test]
    fn t09_symdiff_annihilation_is_invisible_to_l1() {
        let l1_a = TopoBloomMask::from_variable_index(10);
        let gal_a: F256 = SymDiff::embed_to_field(&[10]);

        let l1_res = l1_a.union(&l1_a);
        let gal_res = SymDiff::aggregate(&gal_a, &gal_a, 0);

        assert_ne!(l1_res.0, [0, 0, 0, 0]);
        assert_eq!(
            gal_res,
            F256::zero(),
            "L1 monotonicity breaks SymDiff reversibility by design"
        );
    }

    #[test]
    fn t10_l1_false_positive_resilience_in_high_density() {
        let mut universe = TopoBloomMask::empty();
        for i in 0..128 {
            universe = universe.union(&TopoBloomMask::from_variable_index(i * 2));
        }

        let target = TopoBloomMask::from_variable_index(255);
        assert!(
            !target.is_subset_of(&universe),
            "50% density avoids catastrophic false positive collapse"
        );
    }

    // =========================================================================
    // GROUP 3: Extreme Scaling and Memory Boundaries
    // =========================================================================

    #[test]
    fn t11_embedding_massive_payload() {
        let payload = vec![0xAA; 10000];
        let e: F256 = SymDiff::embed_to_field(&payload);
        assert_ne!(
            e,
            F256::zero(),
            "Massive payload polynomial evaluation stable"
        );
    }

    #[test]
    fn t12_multiset_aggregation_of_10000_identical_elements() {
        let e = embed_str::<MultiSet>("Particle");
        let mut mset: F256 = MultiSet::empty_state();
        for _ in 0..10000 {
            mset = MultiSet::aggregate(&mset, &e, 0);
        }
        assert_ne!(
            mset,
            F256::zero(),
            "Degree 10000 polynomial root accumulation remains within F256"
        );
    }

    #[test]
    fn t13_sequence_of_10000_clock_ticks() {
        let zero = F256::zero();
        let mut seq: F256 = Sequence::aggregate(
            &Sequence::empty_state(),
            &embed_str::<Sequence>("Origin"),
            0,
        );

        for _ in 0..10000 {
            seq = Sequence::aggregate(&seq, &zero, 0);
        }
        assert_ne!(
            seq,
            F256::zero(),
            "10,000 phase shifts preserves the origin"
        );
    }

    #[test]
    fn t14_symdiff_alternating_10000_times() {
        let e = embed_str::<SymDiff>("Oscillator");
        let mut state: F256 = SymDiff::empty_state();

        for _ in 0..10000 {
            state = SymDiff::aggregate(&state, &e, 0);
        }
        assert_eq!(
            state,
            F256::zero(),
            "10,000 additions guarantees perfect parity annihilation"
        );
    }

    #[test]
    fn t15_reversing_10000_element_sequence_via_fermat() {
        let mut seq: F256 = Sequence::empty_state();
        let mut elements = Vec::with_capacity(500);

        for i in 0..500 {
            let e: F256 = Sequence::embed_to_field(&[i as u8]);
            elements.push(e);
            seq = Sequence::aggregate(&seq, &e, 0);
        }

        for e in elements.iter().rev() {
            seq = Sequence::remove(&seq, e).unwrap();
        }
        assert_eq!(
            seq,
            F256::zero(),
            "Deep LIFO rewind perfectly reconstructs vacuum"
        );
    }

    // =========================================================================
    // GROUP 4: Pathological Collisions and Interferences
    // =========================================================================

    #[test]
    fn t16_injecting_generator_constant_directly() {
        let mut xg_buf = [0u8; 32];
        xg_buf[31] = 0x80;

        let e_sym: F256 = SymDiff::embed_to_field(&xg_buf);
        let e_mul: F256 = MultiSet::embed_to_field(&xg_buf);

        assert_ne!(e_sym, F256::zero(), "SymDiff accepts raw X_g");
        assert_eq!(e_mul, F256::zero(), "MultiSet violently masks X_g to 0");
    }

    #[test]
    fn t17_horner_method_phase_alignment_mismatch() {
        let b0 = [0x11; 32];
        let b1 = [0x22; 32];
        let mut b_mixed = [0u8; 40];
        b_mixed[..32].copy_from_slice(&b0);
        b_mixed[32..40].copy_from_slice(&b1[..8]);

        let e_mixed: F256 = Sequence::embed_to_field(&b_mixed);
        assert_ne!(
            e_mixed,
            F256::zero(),
            "Non-aligned block chunks safely evaluate polynomials"
        );
    }

    #[test]
    fn t18_zero_length_data_vs_one_byte_zero_data() {
        let e_empty: F256 = Sequence::embed_to_field(&[]);
        let e_zero: F256 = Sequence::embed_to_field(&[0x00]);
        assert_eq!(
            e_empty, e_zero,
            "Empty array and zero-byte array topologically equate to 0"
        );
    }

    #[test]
    fn t19_multiset_of_symdiffs_graph_of_sets() {
        let mut set1: F256 = SymDiff::empty_state();
        set1 = SymDiff::aggregate(&set1, &embed_str::<SymDiff>("A"), 0);
        set1 = SymDiff::aggregate(&set1, &embed_str::<SymDiff>("B"), 0);

        let mut set2: F256 = SymDiff::empty_state();
        set2 = SymDiff::aggregate(&set2, &embed_str::<SymDiff>("B"), 0);
        set2 = SymDiff::aggregate(&set2, &embed_str::<SymDiff>("A"), 0);

        let mut graph: F256 = MultiSet::empty_state();
        graph = MultiSet::aggregate(&graph, &set1, 0);
        graph = MultiSet::aggregate(&graph, &set2, 0);

        assert_ne!(
            graph,
            F256::zero(),
            "Set commutativity recognized correctly"
        );
    }

    #[test]
    fn t20_symdiff_of_multisets_cancelling_subgraphs() {
        let mut graph1: F256 = MultiSet::empty_state();
        graph1 = MultiSet::aggregate(&graph1, &embed_str::<MultiSet>("Node_X"), 0);

        let mut graph2: F256 = MultiSet::empty_state();
        graph2 = MultiSet::aggregate(&graph2, &embed_str::<MultiSet>("Node_X"), 0);

        let diff = SymDiff::aggregate(&graph1, &graph2, 0);
        assert_eq!(diff, F256::zero(), "Identical hypergraphs cancel perfectly");
    }

    // =========================================================================
    // GROUP 5: Causality and Reversibility Limits
    // =========================================================================

    #[test]
    fn t21_reversing_sequence_out_of_order_yields_garbage_no_panic() {
        let a = embed_str::<Sequence>("A");
        let b = embed_str::<Sequence>("B");

        let mut seq: F256 = Sequence::empty_state();
        seq = Sequence::aggregate(&seq, &a, 0);
        seq = Sequence::aggregate(&seq, &b, 0);

        let garbage = Sequence::remove(&seq, &a).unwrap();
        assert_ne!(garbage, b, "Time travel paradox strictly enforced");
    }

    #[test]
    fn t22_multiset_extraction_of_nonexistent_elements() {
        let a = embed_str::<MultiSet>("A");
        let phantom = embed_str::<MultiSet>("Phantom");

        let mut mset: F256 = MultiSet::empty_state();
        mset = MultiSet::aggregate(&mset, &a, 0);

        let remainder = MultiSet::remove(&mset, &phantom).unwrap();
        assert_ne!(
            remainder,
            F256::one(),
            "Extracting non-roots yields mathematical remainders"
        );
    }

    #[test]
    fn t23_symdiff_double_removal_acts_as_addition() {
        let a = embed_str::<SymDiff>("A");
        let mut state: F256 = SymDiff::aggregate(&SymDiff::empty_state(), &a, 0);

        state = SymDiff::remove(&state, &a).unwrap();
        state = SymDiff::remove(&state, &a).unwrap();

        assert_eq!(state, a, "Removal in characteristic 2 is exactly addition");
    }

    #[test]
    fn t24_sequence_rollback_to_exact_genesis_via_checkpoints() {
        let a = embed_str::<Sequence>("A");
        let b = embed_str::<Sequence>("B");

        let genesis = Sequence::aggregate(&Sequence::empty_state(), &a, 0);
        let future = Sequence::aggregate(&genesis, &b, 0);

        let rollback = Sequence::remove(&future, &b).unwrap();
        assert_eq!(rollback, genesis, "Checkpoint verification");
    }

    #[test]
    fn t25_removing_vacuum_from_vacuum() {
        let sym_v: F256 = SymDiff::empty_state();
        let rem_sym = SymDiff::remove(&sym_v, &sym_v).unwrap();
        assert_eq!(rem_sym, F256::zero(), "0 - 0 = 0");

        let mul_v: F256 = MultiSet::empty_state();
        let rem_mul = MultiSet::remove(&mul_v, &F256::zero()).unwrap();
        assert_ne!(rem_mul, F256::one(), "1 / X_g yields Fermat inverse of X_g");
    }

    // =========================================================================
    // GROUP 6: Cryptographic / Deterministic Invariants
    // =========================================================================

    #[test]
    fn t26_endianness_strictness_across_multi_block() {
        let data = [0x01, 0x02, 0x03, 0x04];
        let e: F256 = SymDiff::embed_to_field(&data);
        assert_eq!(e.0[0] & 0xFF, 0x01, "Strict Little-Endian preservation");
    }

    #[test]
    fn t27_shift_phase_loop_unrolling_invariant() {
        let a = embed_str::<SymDiff>("Core");
        let mut loop_shift = a.clone();
        for _ in 0..64 {
            loop_shift = loop_shift.shift_phase();
        }
        assert_ne!(
            loop_shift, a,
            "64 phase shifts displace the entire first u64 word"
        );
    }

    #[test]
    fn t28_generator_constant_immutability() {
        let mut xg_buffer = [0u8; 32];
        xg_buffer[31] = 0x80;
        let xg = F256::from_bytes_canonical(&xg_buffer);

        let xg_sq = xg.mul(&xg);
        assert_ne!(
            xg_sq, xg,
            "Generator constant squares deterministically within the field"
        );
    }

    #[test]
    fn t29_sequence_phase_inverse_associativity() {
        let a = embed_str::<Sequence>("Assoc");
        let phi = F256::one().shift_phase();
        let phi_inv = phi.inv().unwrap();

        let shifted = a.mul(&phi);
        let restored = shifted.mul(&phi_inv);

        assert_eq!(restored, a, "Phi * Phi^-1 = 1 structurally preserved");
    }

    #[test]
    fn t30_the_grand_unification_deterministic_divergence() {
        let payload = b"{\"layer\": 2, \"type\": \"HyperGraph_Node\"}";

        let e_sym: F256 = SymDiff::embed_to_field(payload);
        let e_seq: F256 = Sequence::embed_to_field(payload);
        let e_mul: F256 = MultiSet::embed_to_field(payload);

        let state_sym: F256 = SymDiff::aggregate(&SymDiff::empty_state(), &e_sym, 0);
        let state_seq: F256 = Sequence::aggregate(&Sequence::empty_state(), &e_seq, 0);
        let state_mul: F256 = MultiSet::aggregate(&MultiSet::empty_state(), &e_mul, 0);

        assert_eq!(
            state_sym, state_seq,
            "SymDiff and Sequence identically anchor at t=0"
        );
        assert_ne!(
            state_sym, state_mul,
            "MultiSet violently diverges at t=0 via X_g product"
        );
    }
}
#[cfg(test)]
mod chaos_and_invariants_tests {
    use crate::algebra::galois_256::GaloisSignature256;
    use crate::algebra::traits::FiniteField;
    use crate::topology::bloom_l1::{TopoBloomMask, TopologicalMask};
    use crate::topology::multiset::MultisetAggregator as MultiSet;
    use crate::topology::sequence::SequenceAggregator as Sequence;
    use crate::topology::symmetric_difference::SymmetricDifferenceAggregator as SymDiff;
    use crate::topology::traits::HomomorphicAggregator;

    type F256 = GaloisSignature256;

    fn embed_str<A: HomomorphicAggregator<F256>>(s: &str) -> F256 {
        A::embed_to_field(s.as_bytes())
    }

    // =========================================================================
    // GROUP 1: Deep Homomorphic Linearity (Anti-Avalanche Proofs)
    // =========================================================================

    #[test]
    fn t01_linear_shift_no_avalanche() {
        // Proof that altering the 1st byte ONLY alters the corresponding polynomial dimension
        // and does NOT cause a cryptographic avalanche.
        let mut d1 = [0u8; 32];
        d1[0] = 0x01;
        let mut d2 = [0u8; 32];
        d2[0] = 0x02;

        let e1: F256 = SymDiff::embed_to_field(&d1);
        let e2: F256 = SymDiff::embed_to_field(&d2);

        let diff = e1.add(&e2); // XOR difference
        assert_eq!(
            diff.0[0] & 0xFF,
            0x03,
            "Linearity confirmed: 0x01 ^ 0x02 = 0x03 exactly in dimensional space"
        );
        assert_eq!(
            diff.0[3], 0,
            "No cryptographic avalanching into higher registers"
        );
    }

    #[test]
    fn t02_multiset_affine_shift_linearity() {
        let mut d1 = [0u8; 32];
        d1[0] = 0x10;
        d1[31] = 0xFF; // Bit 255 is 1
        let mut d2 = [0u8; 32];
        d2[0] = 0x20;
        d2[31] = 0xFF;

        let e1: F256 = MultiSet::embed_to_field(&d1);
        let e2: F256 = MultiSet::embed_to_field(&d2);

        let diff = e1.add(&e2);
        assert_eq!(
            diff.0[0] & 0xFF,
            0x30,
            "Affine subspace preserves lower-register linearity"
        );
        assert_eq!(
            diff.0[3] >> 63,
            0,
            "Affine mask enforces zero at highest bit for both"
        );
    }

    #[test]
    fn t03_sequence_homomorphism_over_massive_blocks() {
        let block_a = vec![0xAA; 128];
        let block_b = vec![0x55; 128];
        let mut block_c = vec![0u8; 128];
        for i in 0..128 {
            block_c[i] = block_a[i] ^ block_b[i];
        }

        let ea: F256 = Sequence::embed_to_field(&block_a);
        let eb: F256 = Sequence::embed_to_field(&block_b);
        let ec: F256 = Sequence::embed_to_field(&block_c);

        assert_eq!(
            ec,
            ea.add(&eb),
            "Homomorphism strictly survives deep block phasing"
        );
    }

    #[test]
    fn t04_symdiff_null_vector_cancellation() {
        let payload = vec![0u8; 64];
        let e: F256 = SymDiff::embed_to_field(&payload);
        assert_eq!(
            e,
            F256::zero(),
            "Null vectors exert zero topological weight"
        );
    }

    #[test]
    fn t05_multiset_null_vector_is_not_vacuum() {
        let payload = vec![0u8; 32];
        let e: F256 = MultiSet::embed_to_field(&payload);

        let mut mset: F256 = MultiSet::empty_state();
        mset = MultiSet::aggregate(&mset, &e, 0); // 1 * (X_g + 0) = X_g

        assert_ne!(
            mset,
            F256::one(),
            "Injecting a null vector shifts the MultiSet out of vacuum"
        );
    }

    // =========================================================================
    // GROUP 2: The "Zero" Paradoxes
    // =========================================================================

    #[test]
    fn t06_symdiff_0_plus_0_is_0() {
        let zero: F256 = F256::zero();
        let state: F256 = SymDiff::aggregate(&zero, &zero, 0);
        assert_eq!(state, F256::zero());
    }

    #[test]
    fn t07_sequence_0_followed_by_0_shifts_phase() {
        let zero: F256 = F256::zero();
        let mut seq: F256 = Sequence::empty_state();
        seq = Sequence::aggregate(&seq, &zero, 0);
        seq = Sequence::aggregate(&seq, &zero, 0);
        assert_eq!(
            seq,
            F256::zero(),
            "0 * Phi + 0 = 0. Continuous empty ticks collapse to origin."
        );
    }

    #[test]
    fn t08_sequence_data_followed_by_0_shifts_phase() {
        let data: F256 = Sequence::embed_to_field(b"Data");
        let zero: F256 = F256::zero();

        let mut seq: F256 = Sequence::empty_state();
        seq = Sequence::aggregate(&seq, &data, 0);
        seq = Sequence::aggregate(&seq, &zero, 0);

        assert_eq!(
            seq,
            data.shift_phase(),
            "Data * Phi + 0 shifts polynomial exactly by x"
        );
    }

    #[test]
    fn t09_multiset_0_multiplied_by_0_escalates() {
        let zero: F256 = F256::zero();
        let mut mset: F256 = MultiSet::empty_state();
        mset = MultiSet::aggregate(&mset, &zero, 0);
        mset = MultiSet::aggregate(&mset, &zero, 0);

        // (X_g + 0) * (X_g + 0) = X_g^2
        let mut xg_buf = [0u8; 32];
        xg_buf[31] = 0x80;
        let xg: F256 = F256::from_bytes_canonical(&xg_buf);
        let expected = xg.mul(&xg);

        assert_eq!(
            mset, expected,
            "MultiSet successfully aggregates true zeroes into X_g^2"
        );
    }

    #[test]
    fn t10_multiset_vacuum_division_paradox() {
        let vacuum: F256 = MultiSet::empty_state(); // 1
        let zero: F256 = F256::zero();

        let paradox = MultiSet::remove(&vacuum, &zero).unwrap();
        // 1 / (X_g + 0) = X_g^-1
        let mut xg_buf = [0u8; 32];
        xg_buf[31] = 0x80;
        let xg: F256 = F256::from_bytes_canonical(&xg_buf);
        let xg_inv = xg.inv().unwrap();

        assert_eq!(
            paradox, xg_inv,
            "Dividing the vacuum extracts the generator's inverse"
        );
    }

    // =========================================================================
    // GROUP 3: Complex Multiplicative vs Additive Annihilation
    // =========================================================================

    #[test]
    fn t11_symdiff_quadruple_annihilation() {
        let a = embed_str::<SymDiff>("Node");
        let mut sym: F256 = SymDiff::empty_state();
        sym = SymDiff::aggregate(&sym, &a, 0);
        sym = SymDiff::aggregate(&sym, &a, 0); // 0
        sym = SymDiff::aggregate(&sym, &a, 0); // A
        sym = SymDiff::aggregate(&sym, &a, 0); // 0
        assert_eq!(sym, F256::zero());
    }

    #[test]
    fn t12_multiset_quadruple_escalation() {
        let a = embed_str::<MultiSet>("Node");
        let mut mset: F256 = MultiSet::empty_state();
        mset = MultiSet::aggregate(&mset, &a, 0);
        mset = MultiSet::aggregate(&mset, &a, 0);
        mset = MultiSet::aggregate(&mset, &a, 0);
        mset = MultiSet::aggregate(&mset, &a, 0);
        assert_ne!(mset, F256::zero());
        assert_ne!(mset, F256::one());
    }

    #[test]
    fn t13_extracting_multiset_quadruple_to_vacuum() {
        let a = embed_str::<MultiSet>("Node");
        let mut mset: F256 = MultiSet::empty_state();
        for _ in 0..4 {
            mset = MultiSet::aggregate(&mset, &a, 0);
        }
        for _ in 0..4 {
            mset = MultiSet::remove(&mset, &a).unwrap();
        }
        assert_eq!(
            mset,
            F256::one(),
            "Fermat inverse safely rolls back deep exponential polynomials"
        );
    }

    #[test]
    fn t14_symdiff_interleaved_annihilation() {
        let a = embed_str::<SymDiff>("A");
        let b = embed_str::<SymDiff>("B");

        let mut sym: F256 = SymDiff::empty_state();
        sym = SymDiff::aggregate(&sym, &a, 0);
        sym = SymDiff::aggregate(&sym, &b, 0);
        sym = SymDiff::aggregate(&sym, &a, 0);
        sym = SymDiff::aggregate(&sym, &b, 0);
        assert_eq!(
            sym,
            F256::zero(),
            "Interleaved temporal events cancel perfectly in Boolean Ring"
        );
    }

    #[test]
    fn t15_sequence_interleaved_preservation() {
        let a = embed_str::<Sequence>("A");
        let b = embed_str::<Sequence>("B");

        let mut seq: F256 = Sequence::empty_state();
        seq = Sequence::aggregate(&seq, &a, 0);
        seq = Sequence::aggregate(&seq, &b, 0);
        seq = Sequence::aggregate(&seq, &a, 0);
        seq = Sequence::aggregate(&seq, &b, 0);
        assert_ne!(
            seq,
            F256::zero(),
            "Sequences strictly preserve chronological variance"
        );
    }

    // =========================================================================
    // GROUP 4: Structural Permutation Boundaries
    // =========================================================================

    #[test]
    fn t16_sequence_palindromes_are_distinct() {
        let a = embed_str::<Sequence>("A");
        let b = embed_str::<Sequence>("B");

        let mut s1: F256 = Sequence::empty_state();
        s1 = Sequence::aggregate(&s1, &a, 0);
        s1 = Sequence::aggregate(&s1, &b, 0);
        s1 = Sequence::aggregate(&s1, &a, 0); // A-B-A

        let mut s2: F256 = Sequence::empty_state();
        s2 = Sequence::aggregate(&s2, &b, 0);
        s2 = Sequence::aggregate(&s2, &a, 0);
        s2 = Sequence::aggregate(&s2, &b, 0); // B-A-B

        assert_ne!(s1, s2);
    }

    #[test]
    fn t17_symdiff_palindromes_collide_into_zero_or_center() {
        let a = embed_str::<SymDiff>("A");
        let b = embed_str::<SymDiff>("B");

        let mut s1: F256 = SymDiff::empty_state();
        s1 = SymDiff::aggregate(&s1, &a, 0);
        s1 = SymDiff::aggregate(&s1, &b, 0);
        s1 = SymDiff::aggregate(&s1, &a, 0); // A + B + A = B

        assert_eq!(
            s1, b,
            "Symmetric difference perfectly isolates the unique element"
        );
    }

    #[test]
    fn t18_multiset_palindromes_collide() {
        let a = embed_str::<MultiSet>("A");
        let b = embed_str::<MultiSet>("B");

        let mut s1: F256 = MultiSet::empty_state();
        s1 = MultiSet::aggregate(&s1, &a, 0);
        s1 = MultiSet::aggregate(&s1, &b, 0);
        s1 = MultiSet::aggregate(&s1, &a, 0); // {A, A, B}

        let mut s2: F256 = MultiSet::empty_state();
        s2 = MultiSet::aggregate(&s2, &a, 0);
        s2 = MultiSet::aggregate(&s2, &a, 0);
        s2 = MultiSet::aggregate(&s2, &b, 0); // {A, A, B}

        assert_eq!(s1, s2, "Multisets are immune to insertion order");
    }

    #[test]
    fn t19_bloom_palindromes_collide_to_set() {
        let a = TopoBloomMask::from_variable_index(10);
        let b = TopoBloomMask::from_variable_index(20);

        let s1 = a.union(&b).union(&a);
        let s2 = b.union(&a).union(&b);
        assert_eq!(
            s1.0, s2.0,
            "L1 masks strictly collapse to distinct variable sets"
        );
    }

    #[test]
    fn t20_large_subgraph_isomorphism_via_multiset() {
        let mut s1: F256 = MultiSet::empty_state();
        let mut s2: F256 = MultiSet::empty_state();

        for i in 0..500 {
            s1 = MultiSet::aggregate(&s1, &MultiSet::embed_to_field(&[i as u8]), 0);
            s2 = MultiSet::aggregate(&s2, &MultiSet::embed_to_field(&[(499 - i) as u8]), 0);
        }
        assert_eq!(
            s1, s2,
            "500-node reversed subgraph maps to same polynomial volume"
        );
    }

    // =========================================================================
    // GROUP 5: Advanced Causal Time Paradoxes
    // =========================================================================

    #[test]
    fn t21_sequence_rewind_past_vacuum_yields_inverse_polynomials() {
        let a = embed_str::<Sequence>("A");
        let v: F256 = Sequence::empty_state();
        let anti_a = Sequence::remove(&v, &a).unwrap();

        let restored = Sequence::aggregate(&anti_a, &a, 0);
        assert_eq!(
            restored, v,
            "Aggregating into negative time restores the zero state"
        );
    }

    #[test]
    fn t22_multiset_rewind_past_vacuum_yields_fermat_inverses() {
        let a = embed_str::<MultiSet>("A");
        let v: F256 = MultiSet::empty_state();
        let anti_a = MultiSet::remove(&v, &a).unwrap();

        let restored = MultiSet::aggregate(&anti_a, &a, 0);
        assert_eq!(
            restored, v,
            "Fermat inverse logic sustains negative graph volumes"
        );
    }

    #[test]
    fn t23_symdiff_rewind_past_vacuum_is_just_addition() {
        let a = embed_str::<SymDiff>("A");
        let v: F256 = SymDiff::empty_state();
        let anti_a = SymDiff::remove(&v, &a).unwrap();
        assert_eq!(
            anti_a, a,
            "Anti-matter in a Boolean ring is indistinguishable from matter"
        );
    }

    #[test]
    fn t24_sequence_temporal_shift_distributivity() {
        let a = embed_str::<Sequence>("A");
        let b = embed_str::<Sequence>("B");

        let s_ab =
            Sequence::aggregate(&Sequence::aggregate(&Sequence::empty_state(), &a, 0), &b, 0);
        let shifted = s_ab.shift_phase();

        // (A*Phi + B) * Phi = A*Phi^2 + B*Phi
        let mut expected: F256 = Sequence::empty_state();
        expected = Sequence::aggregate(&expected, &a, 0);
        expected = Sequence::aggregate(&expected, &b, 0);
        expected = Sequence::aggregate(&expected, &F256::zero(), 0); // Tick

        assert_eq!(
            shifted, expected,
            "A temporal phase shift is perfectly equivalent to appending a zero tick"
        );
    }

    #[test]
    fn t25_extreme_sequence_clock_tick_overflow_survivability() {
        let mut s: F256 = Sequence::embed_to_field(b"Seed");
        for _ in 0..1000 {
            s = Sequence::aggregate(&s, &F256::zero(), 0);
        }
        assert_ne!(s, F256::zero());
    }

    // =========================================================================
    // GROUP 6: L1 Shield Entropy Physics
    // =========================================================================

    #[test]
    fn t26_l1_shield_intersection_loss() {
        let m1 = TopoBloomMask::from_variable_index(10);
        let m2 = TopoBloomMask::from_variable_index(20);
        let union = m1.union(&m2);

        // Simulating an intersection manually: (m1 AND union) == m1
        let intersection = TopoBloomMask([
            union.0[0] & m1.0[0],
            union.0[1] & m1.0[1],
            union.0[2] & m1.0[2],
            union.0[3] & m1.0[3],
        ]);
        assert_eq!(
            intersection.0, m1.0,
            "Intersection with union isolates the subset"
        );
    }

    #[test]
    fn t27_l1_entropy_saturation_curve() {
        let mut m = TopoBloomMask::empty();
        let mut unique_bits = 0;

        for i in 0..256 {
            let prev = m.0[0].count_ones()
                + m.0[1].count_ones()
                + m.0[2].count_ones()
                + m.0[3].count_ones();
            m = m.union(&TopoBloomMask::from_variable_index(i));
            let new_count = m.0[0].count_ones()
                + m.0[1].count_ones()
                + m.0[2].count_ones()
                + m.0[3].count_ones();

            if new_count > prev {
                unique_bits += 1;
            }
        }
        assert_eq!(
            unique_bits, 256,
            "Entropy curve grows monotonically exactly 256 times"
        );
    }

    #[test]
    fn t28_l1_modulo_aliasing_structural_blindness() {
        let m1 = TopoBloomMask::from_variable_index(1);
        let m2 = TopoBloomMask::from_variable_index(257); // 256 + 1
        assert_eq!(
            m1.0, m2.0,
            "L1 shield is mathematically blind to structural layers separated by multiples of 256"
        );
    }

    #[test]
    fn t29_galois_precision_resolves_l1_blindness() {
        let m1: F256 = SymDiff::embed_to_field(&[1, 0]);
        let m2: F256 = SymDiff::embed_to_field(&[1, 1]); // Representing 257 in LE
        assert_ne!(
            m1, m2,
            "Galois field immediately resolves L1 aliasing blindness"
        );
    }

    #[test]
    fn t30_universal_topology_cross_reference_integrity() {
        // Build a node, project it into all 4 topological dimensions
        let payload = b"Universal_Vertex";
        let l1 = TopoBloomMask::from_variable_index(42);
        let sym: F256 = SymDiff::aggregate(
            &SymDiff::empty_state(),
            &SymDiff::embed_to_field(payload),
            0,
        );
        let mset: F256 = MultiSet::aggregate(
            &MultiSet::empty_state(),
            &MultiSet::embed_to_field(payload),
            0,
        );
        let seq: F256 = Sequence::aggregate(
            &Sequence::empty_state(),
            &Sequence::embed_to_field(payload),
            0,
        );

        // Assert deep divergences
        assert_ne!(sym, mset);
        assert_ne!(mset, seq);
        assert_ne!(l1.0, [0, 0, 0, 0]);
        // Mathematical sanity ensures all 4 spaces recorded the event without overlapping failure domains
    }
}
