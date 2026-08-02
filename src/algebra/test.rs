#[cfg(test)]
mod tests {
    use crate::algebra::traits::FiniteField;
    use crate::GaloisSignature256;

    /// Helper to easily instantiate signatures for testing
    fn sig(w0: u64, w1: u64, w2: u64, w3: u64) -> GaloisSignature256 {
        GaloisSignature256([w0, w1, w2, w3])
    }

    // =========================================================================
    // GROUP 1: Instantiation and Memory Axioms
    // =========================================================================

    #[test]
    fn t01_zero_is_topological_singularity() {
        let z = GaloisSignature256::zero();
        assert!(
            z.is_zero(),
            "Zero identity must evaluate to true in is_zero()"
        );
        assert_eq!(z, sig(0, 0, 0, 0));
    }

    #[test]
    fn t02_one_is_multiplicative_identity() {
        let one = GaloisSignature256::one();
        assert!(!one.is_zero(), "One must not be a singularity");
        assert_eq!(one, sig(1, 0, 0, 0));
    }

    #[test]
    fn t03_is_zero_detects_non_zeros_in_all_words() {
        assert!(!sig(1, 0, 0, 0).is_zero());
        assert!(!sig(0, 1, 0, 0).is_zero());
        assert!(!sig(0, 0, 1, 0).is_zero());
        assert!(!sig(0, 0, 0, 1).is_zero());
    }

    #[test]
    fn t04_from_bytes_canonical_endianness() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0x12; // LSB of Word 0
        bytes[31] = 0x80; // MSB of Word 3

        let s = GaloisSignature256::from_bytes_canonical(&bytes);
        assert_eq!(s.0[0] & 0xFF, 0x12);
        assert_eq!(s.0[3] >> 56, 0x80);
    }

    // =========================================================================
    // GROUP 2: Characteristic 2 Additive Ring Laws
    // =========================================================================

    #[test]
    fn t05_add_identity_element() {
        let a = sig(12345, 67890, 11111, 22222);
        let z = GaloisSignature256::zero();
        assert_eq!(a.add(&z), a, "A + 0 = A");
        assert_eq!(z.add(&a), a, "0 + A = A");
    }

    #[test]
    fn t06_add_characteristic_two_annihilation() {
        let a = sig(0xDEADBEEF, 0xCAFEBABE, 0x12345678, 0x9ABCDEF0);
        let result = a.add(&a);
        assert!(result.is_zero(), "A + A = 0 in GF(2^n)");
    }

    #[test]
    fn t07_add_commutativity() {
        let a = sig(1, 2, 3, 4);
        let b = sig(5, 6, 7, 8);
        assert_eq!(a.add(&b), b.add(&a), "A + B = B + A");
    }

    #[test]
    fn t08_add_associativity() {
        let a = sig(1, 2, 3, 4);
        let b = sig(5, 6, 7, 8);
        let c = sig(9, 10, 11, 12);
        let ab_c = (a.add(&b)).add(&c);
        let a_bc = a.add(&(b.add(&c)));
        assert_eq!(ab_c, a_bc, "(A + B) + C = A + (B + C)");
    }

    #[test]
    fn t09_add_restoration() {
        let a = sig(999, 888, 777, 666);
        let b = sig(111, 222, 333, 444);
        let sum = a.add(&b);
        assert_eq!(sum.add(&b), a, "(A + B) + B = A");
    }

    // =========================================================================
    // GROUP 3: Phase Geometry and Modular Reduction
    // =========================================================================

    #[test]
    fn t10_shift_phase_within_word() {
        let a = sig(1, 0, 0, 0); // x^0
        let b = a.shift_phase(); // x^1
        assert_eq!(b, sig(2, 0, 0, 0));
    }

    #[test]
    fn t11_shift_phase_cross_word_0_to_1() {
        let a = sig(1 << 63, 0, 0, 0); // MSB of word 0
        let b = a.shift_phase();
        assert_eq!(b, sig(0, 1, 0, 0), "Bit must cleanly cross to word 1");
    }

    #[test]
    fn t12_shift_phase_cross_word_1_to_2() {
        let a = sig(0, 1 << 63, 0, 0);
        let b = a.shift_phase();
        assert_eq!(b, sig(0, 0, 1, 0), "Bit must cleanly cross to word 2");
    }

    #[test]
    fn t13_shift_phase_cross_word_2_to_3() {
        let a = sig(0, 0, 1 << 63, 0);
        let b = a.shift_phase();
        assert_eq!(b, sig(0, 0, 0, 1), "Bit must cleanly cross to word 3");
    }

    #[test]
    fn t14_shift_phase_modular_reduction_exact() {
        let a = sig(0, 0, 0, 1 << 63); // x^255
        let b = a.shift_phase(); // x^256 -> reduces to P(x)
                                 // Irreducible polynomial is x^256 + x^10 + x^5 + x^2 + 1 = 0x425
        assert_eq!(
            b,
            sig(0x425, 0, 0, 0),
            "Must reduce via irreducible polynomial"
        );
    }

    #[test]
    fn t15_shift_phase_preserves_zero() {
        let z = GaloisSignature256::zero();
        assert_eq!(z.shift_phase(), z, "0 * x = 0");
    }

    // =========================================================================
    // GROUP 4: Multiplicative Group Laws
    // =========================================================================

    #[test]
    fn t16_mul_identity_element() {
        let a = sig(1337, 420, 9000, 42);
        let one = GaloisSignature256::one();
        assert_eq!(a.mul(&one), a, "A * 1 = A");
        assert_eq!(one.mul(&a), a, "1 * A = A");
    }

    #[test]
    fn t17_mul_absorbing_element() {
        let a = sig(1337, 420, 9000, 42);
        let z = GaloisSignature256::zero();
        assert_eq!(a.mul(&z), z, "A * 0 = 0");
        assert_eq!(z.mul(&a), z, "0 * A = 0");
    }

    #[test]
    fn t18_mul_commutativity() {
        let a = sig(0xABC, 0xDEF, 0x123, 0x456);
        let b = sig(0x999, 0x888, 0x777, 0x666);
        assert_eq!(a.mul(&b), b.mul(&a), "A * B = B * A");
    }

    #[test]
    fn t19_mul_distributivity() {
        let a = sig(0x11, 0x22, 0x33, 0x44);
        let b = sig(0x55, 0x66, 0x77, 0x88);
        let c = sig(0x99, 0xAA, 0xBB, 0xCC);
        let b_plus_c = b.add(&c);
        let a_times_bc = a.mul(&b_plus_c);

        let ab = a.mul(&b);
        let ac = a.mul(&c);
        let ab_plus_ac = ab.add(&ac);

        assert_eq!(a_times_bc, ab_plus_ac, "A * (B + C) = A*B + A*C");
    }

    #[test]
    fn t20_mul_x_by_x_equivalence() {
        let x = GaloisSignature256::one().shift_phase();
        let a = sig(0x123, 0x456, 0x789, 0xABC);

        // Multiplying by 'x' should be identical to shift_phase
        assert_eq!(a.mul(&x), a.shift_phase(), "A * x = shift_phase(A)");
    }

    #[test]
    fn t21_mul_forces_deep_reduction() {
        let x255 = sig(0, 0, 0, 1 << 63);
        let x = sig(2, 0, 0, 0);
        // x^255 * x = x^256 = 0x425
        assert_eq!(x255.mul(&x), sig(0x425, 0, 0, 0));
    }

    #[test]
    fn t22_mul_associativity() {
        let a = sig(0x10, 0x0, 0x0, 0x0);
        let b = sig(0x20, 0x0, 0x0, 0x0);
        let c = sig(0x30, 0x0, 0x0, 0x0);
        let ab_c = (a.mul(&b)).mul(&c);
        let a_bc = a.mul(&(b.mul(&c)));
        assert_eq!(ab_c, a_bc, "(A * B) * C = A * (B * C)");
    }

    // =========================================================================
    // GROUP 5: Fermat's Little Theorem and Division
    // =========================================================================

    #[test]
    fn t23_inv_one_is_one() {
        let one = GaloisSignature256::one();
        assert_eq!(one.inv().unwrap(), one, "1^-1 = 1");
    }

    #[test]
    fn t24_inv_zero_is_none() {
        let z = GaloisSignature256::zero();
        assert!(z.inv().is_none(), "0 has no multiplicative inverse");
    }

    #[test]
    fn t25_inv_fermat_identity() {
        // We pick an arbitrary polynomial
        let a = sig(0x1337BEEF, 0xCAFE, 0xABCD, 0x1);
        let a_inv = a.inv().expect("Must have inverse");
        assert_eq!(a.mul(&a_inv), GaloisSignature256::one(), "A * A^-1 = 1");
    }

    #[test]
    fn t26_inv_of_inv_is_original() {
        let a = sig(0x9999, 0x8888, 0x7777, 0x6666);
        let a_inv = a.inv().unwrap();
        let a_inv_inv = a_inv.inv().unwrap();
        assert_eq!(a_inv_inv, a, "(A^-1)^-1 = A");
    }

    #[test]
    fn t27_inv_distributes_over_multiplication() {
        let a = sig(0x12, 0x34, 0x56, 0x78);
        let b = sig(0x9A, 0xBC, 0xDE, 0xF0);

        let ab = a.mul(&b);
        let ab_inv = ab.inv().unwrap();

        let a_inv = a.inv().unwrap();
        let b_inv = b.inv().unwrap();
        let a_inv_b_inv = a_inv.mul(&b_inv);

        assert_eq!(ab_inv, a_inv_b_inv, "(A * B)^-1 = A^-1 * B^-1");
    }

    // =========================================================================
    // GROUP 6: Endomorphisms and Extreme Boundaries
    // =========================================================================

    #[test]
    fn t28_frobenius_endomorphism_addition() {
        // In GF(2^n), (A + B)^2 = A^2 + B^2
        let a = sig(0x111, 0x222, 0x333, 0x444);
        let b = sig(0x555, 0x666, 0x777, 0x888);

        let a_plus_b = a.add(&b);
        let frobenius_sum = a_plus_b.mul(&a_plus_b);

        let a_sq = a.mul(&a);
        let b_sq = b.mul(&b);
        let sum_of_sq = a_sq.add(&b_sq);

        assert_eq!(frobenius_sum, sum_of_sq, "(A + B)^2 = A^2 + B^2");
    }

    #[test]
    fn t29_frobenius_endomorphism_multiplication() {
        // (A * B)^2 = A^2 * B^2
        let a = sig(0xAAA, 0xBBB, 0xCCC, 0xDDD);
        let b = sig(0x123, 0x456, 0x789, 0x012);

        let ab = a.mul(&b);
        let frobenius_ab = ab.mul(&ab);

        let a_sq = a.mul(&a);
        let b_sq = b.mul(&b);
        let a_sq_b_sq = a_sq.mul(&b_sq);

        assert_eq!(frobenius_ab, a_sq_b_sq, "(A * B)^2 = A^2 * B^2");
    }

    #[test]
    fn t30_maximum_polynomial_avalanche() {
        // The densest possible state: All 256 bits set to 1.
        let max_poly = sig(u64::MAX, u64::MAX, u64::MAX, u64::MAX);

        // Squaring it causes a massive chain of reductions
        let sq = max_poly.mul(&max_poly);

        // It must not crash, and its inverse must perfectly restore it.
        let sq_inv = sq.inv().unwrap();
        let restored = sq.mul(&sq_inv);

        assert_eq!(
            restored,
            GaloisSignature256::one(),
            "Max polynomial inversion failed"
        );
    }
}
