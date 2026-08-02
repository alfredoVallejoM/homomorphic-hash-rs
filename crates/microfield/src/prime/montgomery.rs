//! Fixed-width portable Montgomery arithmetic for four 64-bit limbs.

// REDC and schoolbook multiplication deliberately split u128 accumulators
// into low limbs and explicit high carries.
#![allow(clippy::cast_possible_truncation)]

use core::cmp::Ordering;

#[inline]
#[must_use]
pub(crate) fn cmp_limbs(lhs: &[u64; 4], rhs: &[u64; 4]) -> Ordering {
    for index in (0..4).rev() {
        match lhs[index].cmp(&rhs[index]) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    Ordering::Equal
}

#[inline]
#[must_use]
pub(crate) fn add_mod_256(lhs: [u64; 4], rhs: [u64; 4], modulus: [u64; 4]) -> [u64; 4] {
    add_mod(lhs, rhs, modulus)
}

/// Adds two canonical residues using a fixed carry chain and masked correction.
#[inline]
#[must_use]
pub(crate) fn add_mod<const LIMBS: usize>(
    lhs: [u64; LIMBS],
    rhs: [u64; LIMBS],
    modulus: [u64; LIMBS],
) -> [u64; LIMBS] {
    let (sum, carry) = add_wrapping(lhs, rhs);
    let (reduced, borrow) = subtract_wrapping(sum, modulus);
    // If the addition overflowed R, or the low limbs were at least p, the
    // wrapped subtraction is the canonical residue. Both selectors are bits.
    conditional_select(sum, reduced, carry | (borrow ^ 1))
}

#[inline]
#[must_use]
pub(crate) fn sub_mod_256(lhs: [u64; 4], rhs: [u64; 4], modulus: [u64; 4]) -> [u64; 4] {
    let (difference, borrow) = subtract_wrapping(lhs, rhs);
    let corrected = add_wrapping(difference, modulus).0;
    conditional_select(difference, corrected, borrow)
}

#[inline]
#[must_use]
pub(crate) fn neg_mod_256(value: [u64; 4], modulus: [u64; 4]) -> [u64; 4] {
    let negated = subtract_wrapping(modulus, value).0;
    let mut aggregate = 0_u64;
    for limb in value {
        aggregate |= limb;
    }
    conditional_select([0; 4], negated, nonzero_bit(aggregate))
}

#[must_use]
pub(crate) fn montgomery_mul_256(
    lhs: [u64; 4],
    rhs: [u64; 4],
    modulus: [u64; 4],
    neg_inv: u64,
) -> [u64; 4] {
    let wide = wide_product(lhs, rhs);
    montgomery_reduce_wide_256(wide, modulus, neg_inv)
}

#[must_use]
pub(crate) fn to_montgomery_256(
    canonical: [u64; 4],
    r2: [u64; 4],
    modulus: [u64; 4],
    neg_inv: u64,
) -> [u64; 4] {
    montgomery_mul_256(canonical, r2, modulus, neg_inv)
}

#[must_use]
pub(crate) fn from_montgomery_256(value: [u64; 4], modulus: [u64; 4], neg_inv: u64) -> [u64; 4] {
    montgomery_mul_256(value, [1, 0, 0, 0], modulus, neg_inv)
}

#[must_use]
pub(crate) fn montgomery_reduce_wide_256(
    wide: [u64; 8],
    modulus: [u64; 4],
    neg_inv: u64,
) -> [u64; 4] {
    montgomery_reduce_wide::<4, 8>(wide, modulus, neg_inv)
}

/// Reduces a double-width Montgomery product with value-independent control.
///
/// Both loop bounds depend only on the const-generic representation shape. A
/// complete carry sweep is executed after every cancellation row, and the
/// final conditional subtraction is implemented with a mask rather than a
/// data-dependent branch.
#[inline]
#[must_use]
pub(crate) fn montgomery_reduce_wide<const LIMBS: usize, const WIDE_LIMBS: usize>(
    wide: [u64; WIDE_LIMBS],
    modulus: [u64; LIMBS],
    neg_inv: u64,
) -> [u64; LIMBS] {
    assert!(LIMBS > 0);
    assert_eq!(WIDE_LIMBS, LIMBS * 2);

    let mut accumulator = wide;
    let mut high = 0_u64;
    for outer in 0..LIMBS {
        let multiplier = accumulator[outer].wrapping_mul(neg_inv);
        let mut carry = 0_u64;
        for (inner, modulus_limb) in modulus.iter().copied().enumerate() {
            let index = outer + inner;
            let combined = u128::from(accumulator[index])
                + u128::from(multiplier) * u128::from(modulus_limb)
                + u128::from(carry);
            accumulator[index] = combined as u64;
            carry = (combined >> 64) as u64;
        }

        let carry_index = outer + LIMBS;
        let (sum, overflow) = accumulator[carry_index].overflowing_add(carry);
        accumulator[carry_index] = sum;
        let mut propagation = u64::from(overflow);
        for limb in &mut accumulator[carry_index + 1..] {
            let (sum, overflow) = limb.overflowing_add(propagation);
            *limb = sum;
            propagation = u64::from(overflow);
        }
        high = high.wrapping_add(propagation);
    }

    let mut candidate = [0_u64; LIMBS];
    candidate.copy_from_slice(&accumulator[LIMBS..]);
    let (reduced, borrow) = subtract_wrapping(candidate, modulus);
    let subtract = nonzero_bit(high) | (borrow ^ 1);
    conditional_select(candidate, reduced, subtract)
}

#[must_use]
pub(crate) fn wide_product(lhs: [u64; 4], rhs: [u64; 4]) -> [u64; 8] {
    let mut wide = [0_u64; 8];
    for (left_index, left_limb) in lhs.iter().copied().enumerate() {
        let mut carry = 0_u64;
        for (right_index, right_limb) in rhs.iter().copied().enumerate() {
            let index = left_index + right_index;
            let combined = u128::from(wide[index])
                + u128::from(left_limb) * u128::from(right_limb)
                + u128::from(carry);
            wide[index] = combined as u64;
            carry = (combined >> 64) as u64;
        }
        // Earlier rows end before this limb, so the row carry is stored once
        // without a value-dependent propagation loop.
        wide[left_index + 4] = carry;
    }
    wide
}

#[inline]
fn subtract_wrapping<const LIMBS: usize>(
    lhs: [u64; LIMBS],
    rhs: [u64; LIMBS],
) -> ([u64; LIMBS], u64) {
    let mut out = [0_u64; LIMBS];
    let mut borrow = 0_u64;
    for index in 0..LIMBS {
        let (difference, borrow_a) = lhs[index].overflowing_sub(rhs[index]);
        let (difference, borrow_b) = difference.overflowing_sub(borrow);
        out[index] = difference;
        borrow = u64::from(borrow_a) | u64::from(borrow_b);
    }
    (out, borrow)
}

#[inline]
fn add_wrapping<const LIMBS: usize>(lhs: [u64; LIMBS], rhs: [u64; LIMBS]) -> ([u64; LIMBS], u64) {
    let mut out = [0_u64; LIMBS];
    let mut carry = 0_u64;
    for index in 0..LIMBS {
        let (sum, carry_a) = lhs[index].overflowing_add(rhs[index]);
        let (sum, carry_b) = sum.overflowing_add(carry);
        out[index] = sum;
        carry = u64::from(carry_a) | u64::from(carry_b);
    }
    (out, carry)
}

#[inline]
fn conditional_select<const LIMBS: usize>(
    keep: [u64; LIMBS],
    replace: [u64; LIMBS],
    choice: u64,
) -> [u64; LIMBS] {
    let opaque_choice = core::hint::black_box(choice & 1);
    let mask = opaque_choice.wrapping_neg();
    let mut selected = [0_u64; LIMBS];
    for index in 0..LIMBS {
        selected[index] = keep[index] ^ (mask & (keep[index] ^ replace[index]));
    }
    selected
}

#[inline]
const fn nonzero_bit(value: u64) -> u64 {
    (value | value.wrapping_neg()) >> 63
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODULUS: [u64; 4] = [
        0x60d7_67ee_a528_073f,
        0x59b0_47d9_a719_3eed,
        0xa2df_4d6d_fbec_a16e,
        0x9dad_4f18_e672_38cb,
    ];
    const R: [u64; 4] = [
        0x9f28_9811_5ad7_f8c1,
        0xa64f_b826_58e6_c112,
        0x5d20_b292_0413_5e91,
        0x6252_b0e7_198d_c734,
    ];
    const R2: [u64; 4] = [
        0x0dd2_f2a9_c0b6_0e80,
        0x91ef_bf81_c4cb_0056,
        0x55a3_ac4e_36a4_0349,
        0x6ba2_65a9_ee77_837f,
    ];
    const NEG_INV: u64 = 0x5479_78e4_7770_9741;

    #[test]
    fn generated_constants_round_trip_zero_one_and_modulus_minus_one() {
        assert_eq!(to_montgomery_256([0; 4], R2, MODULUS, NEG_INV), [0; 4]);
        assert_eq!(to_montgomery_256([1, 0, 0, 0], R2, MODULUS, NEG_INV), R);
        assert_eq!(from_montgomery_256(R, MODULUS, NEG_INV), [1, 0, 0, 0]);
        let minus_one = sub_mod_256(MODULUS, [1, 0, 0, 0], MODULUS);
        let encoded = to_montgomery_256(minus_one, R2, MODULUS, NEG_INV);
        assert_eq!(from_montgomery_256(encoded, MODULUS, NEG_INV), minus_one);
        assert_eq!(montgomery_mul_256(R, R, MODULUS, NEG_INV), R);
    }

    #[test]
    fn masked_modular_corrections_cover_overflow_borrow_and_zero() {
        let one = [1, 0, 0, 0];
        let zero = [0; 4];
        let minus_one = subtract_wrapping(MODULUS, one).0;
        let minus_two = subtract_wrapping(MODULUS, [2, 0, 0, 0]).0;

        // This sum crosses the 256-bit radix because p is greater than 2^255.
        assert_eq!(add_mod_256(minus_one, minus_one, MODULUS), minus_two);
        assert_eq!(sub_mod_256(zero, minus_one, MODULUS), one);
        assert_eq!(neg_mod_256(zero, MODULUS), zero);
        assert_eq!(neg_mod_256(minus_one, MODULUS), one);
    }

    #[test]
    fn redc_maps_every_512_bit_basis_vector_consistently() {
        let mut expected = montgomery_reduce_wide_256([1, 0, 0, 0, 0, 0, 0, 0], MODULUS, NEG_INV);
        for bit in 0..512 {
            let mut wide = [0_u64; 8];
            wide[bit / 64] = 1_u64 << (bit % 64);
            assert_eq!(
                montgomery_reduce_wide_256(wide, MODULUS, NEG_INV),
                expected,
                "REDC basis bit {bit}"
            );
            expected = add_mod_256(expected, expected, MODULUS);
        }
    }
}
