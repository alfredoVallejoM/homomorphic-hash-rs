//! Semantic migration contract between the legacy and Microfield fields.

use homomorphic_hash_rs::{FiniteField as LegacyField, GaloisSignature256};
use microfield::{BinaryPolynomialField, CanonicalEncoding, Field, Gf2_256HhV1, Invert};

#[test]
fn canonical_bytes_and_layout_policy_are_explicitly_compatible() {
    assert_eq!(size_of::<GaloisSignature256>(), 32);
    assert_eq!(align_of::<GaloisSignature256>(), 32);
    assert_eq!(size_of::<Gf2_256HhV1>(), 32);
    assert_eq!(align_of::<Gf2_256HhV1>(), 8);

    let mut state = 0x243f_6a88_85a3_08d3;
    for _ in 0..64 {
        let bytes = deterministic_bytes(&mut state);
        assert_eq!(
            modern(bytes).to_canonical(),
            legacy_bytes(LegacyField::from_bytes_canonical(&bytes))
        );
    }
}

#[test]
fn addition_multiplication_and_phase_shift_match_the_legacy_field() {
    let mut state = 0x1319_8a2e_0370_7344;
    for _ in 0..64 {
        let lhs_bytes = deterministic_bytes(&mut state);
        let rhs_bytes = deterministic_bytes(&mut state);
        let legacy_lhs = LegacyField::from_bytes_canonical(&lhs_bytes);
        let legacy_rhs = LegacyField::from_bytes_canonical(&rhs_bytes);
        let modern_lhs = modern(lhs_bytes);
        let modern_rhs = modern(rhs_bytes);

        assert_eq!(
            (modern_lhs + modern_rhs).to_canonical(),
            legacy_bytes(LegacyField::add(&legacy_lhs, &legacy_rhs))
        );
        assert_eq!(
            (modern_lhs * modern_rhs).to_canonical(),
            legacy_bytes(LegacyField::mul(&legacy_lhs, &legacy_rhs))
        );
        assert_eq!(
            modern_lhs.mul_by_x().to_canonical(),
            legacy_bytes(LegacyField::shift_phase(&legacy_lhs))
        );
    }
}

#[test]
fn inversion_matches_the_legacy_fermat_schedule() {
    let mut state = 0xa409_3822_299f_31d0;
    assert_eq!(Gf2_256HhV1::ZERO.invert(), None);
    let legacy_zero = GaloisSignature256::zero();
    assert_eq!(LegacyField::inv(&legacy_zero), None);

    for _ in 0..8 {
        let mut bytes = deterministic_bytes(&mut state);
        bytes[0] |= 1;
        let legacy = LegacyField::from_bytes_canonical(&bytes);
        let modern = modern(bytes);
        assert_eq!(
            modern.invert().map(CanonicalEncoding::to_canonical),
            LegacyField::inv(&legacy).map(legacy_bytes)
        );
    }
}

fn modern(bytes: [u8; 32]) -> Gf2_256HhV1 {
    Gf2_256HhV1::from_canonical(&bytes).expect("all 256-bit values are canonical")
}

fn legacy_bytes(value: GaloisSignature256) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (limb, chunk) in value.0.into_iter().zip(bytes.chunks_exact_mut(8)) {
        chunk.copy_from_slice(&limb.to_le_bytes());
    }
    bytes
}

fn deterministic_bytes(state: &mut u64) -> [u8; 32] {
    let mut output = [0; 32];
    for chunk in output.chunks_exact_mut(8) {
        *state ^= *state >> 12;
        *state ^= *state << 25;
        *state ^= *state >> 27;
        chunk.copy_from_slice(&(*state).wrapping_mul(0x2545_f491_4f6c_dd1d).to_le_bytes());
    }
    output
}
