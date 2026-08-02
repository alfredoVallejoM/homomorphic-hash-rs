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
    let mut out = [0_u64; 4];
    let mut carry = false;
    for index in 0..4 {
        let (sum, carry_a) = lhs[index].overflowing_add(rhs[index]);
        let (sum, carry_b) = sum.overflowing_add(u64::from(carry));
        out[index] = sum;
        carry = carry_a || carry_b;
    }
    if carry || cmp_limbs(&out, &modulus) != Ordering::Less {
        subtract_wrapping(out, modulus).0
    } else {
        out
    }
}

#[inline]
#[must_use]
pub(crate) fn sub_mod_256(lhs: [u64; 4], rhs: [u64; 4], modulus: [u64; 4]) -> [u64; 4] {
    let (difference, borrow) = subtract_wrapping(lhs, rhs);
    if borrow {
        add_wrapping(difference, modulus).0
    } else {
        difference
    }
}

#[inline]
#[must_use]
pub(crate) fn neg_mod_256(value: [u64; 4], modulus: [u64; 4]) -> [u64; 4] {
    if value == [0; 4] {
        value
    } else {
        subtract_wrapping(modulus, value).0
    }
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
    let mut accumulator = [0_u64; 9];
    accumulator[..8].copy_from_slice(&wide);

    for outer in 0..4 {
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
        add_carry(&mut accumulator, outer + 4, carry);
        debug_assert_eq!(accumulator[outer], 0);
    }

    let candidate = [
        accumulator[4],
        accumulator[5],
        accumulator[6],
        accumulator[7],
    ];
    if accumulator[8] != 0 || cmp_limbs(&candidate, &modulus) != Ordering::Less {
        let (reduced, borrow) = subtract_wrapping(candidate, modulus);
        let high = accumulator[8].wrapping_sub(u64::from(borrow));
        debug_assert_eq!(high, 0);
        reduced
    } else {
        candidate
    }
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
        add_carry_wide(&mut wide, left_index + 4, carry);
    }
    wide
}

fn add_carry(accumulator: &mut [u64; 9], mut index: usize, mut carry: u64) {
    while carry != 0 {
        let (sum, overflow) = accumulator[index].overflowing_add(carry);
        accumulator[index] = sum;
        carry = u64::from(overflow);
        index += 1;
        debug_assert!(index <= accumulator.len());
    }
}

fn add_carry_wide(accumulator: &mut [u64; 8], mut index: usize, mut carry: u64) {
    while carry != 0 {
        debug_assert!(index < accumulator.len());
        let (sum, overflow) = accumulator[index].overflowing_add(carry);
        accumulator[index] = sum;
        carry = u64::from(overflow);
        index += 1;
    }
}

#[inline]
fn subtract_wrapping(lhs: [u64; 4], rhs: [u64; 4]) -> ([u64; 4], bool) {
    let mut out = [0_u64; 4];
    let mut borrow = false;
    for index in 0..4 {
        let (difference, borrow_a) = lhs[index].overflowing_sub(rhs[index]);
        let (difference, borrow_b) = difference.overflowing_sub(u64::from(borrow));
        out[index] = difference;
        borrow = borrow_a || borrow_b;
    }
    (out, borrow)
}

#[inline]
fn add_wrapping(lhs: [u64; 4], rhs: [u64; 4]) -> ([u64; 4], bool) {
    let mut out = [0_u64; 4];
    let mut carry = false;
    for index in 0..4 {
        let (sum, carry_a) = lhs[index].overflowing_add(rhs[index]);
        let (sum, carry_b) = sum.overflowing_add(u64::from(carry));
        out[index] = sum;
        carry = carry_a || carry_b;
    }
    (out, carry)
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
