//! Generated polynomial reduction.

use super::representation::{Limbs128, Limbs256, Wide256, Wide512};

#[inline]
fn fold_high(folded: &mut [u64], high: &[u64], modulus_tail: u64) {
    debug_assert_eq!(folded.len(), high.len() + 1);
    let mut tail = modulus_tail;
    while tail != 0 {
        let shift = tail.trailing_zeros();
        for (index, limb) in high.iter().copied().enumerate() {
            folded[index] ^= limb << shift;
            if shift != 0 {
                folded[index + 1] ^= limb >> (u64::BITS - shift);
            }
        }
        tail &= tail - 1;
    }
}

#[inline]
fn fold_overflow(low: &mut u64, overflow: u64, modulus_tail: u64) {
    let mut tail = modulus_tail;
    while tail != 0 {
        *low ^= overflow << tail.trailing_zeros();
        tail &= tail - 1;
    }
}

#[inline]
fn validate_tail(modulus_tail: u64) {
    debug_assert_eq!(modulus_tail & 1, 1);
    debug_assert!(u64::BITS - modulus_tail.leading_zeros() <= 33);
}

/// Reduces a 256-bit polynomial modulo `x^128 + MODULUS_TAIL`.
#[inline]
pub(crate) fn reduce_128<const MODULUS_TAIL: u64>(wide: Wide256) -> Limbs128 {
    validate_tail(MODULUS_TAIL);

    let high = [wide[2], wide[3]];
    let mut folded = [wide[0], wide[1], 0];
    fold_high(&mut folded, &high, MODULUS_TAIL);
    let overflow = folded[2];
    fold_overflow(&mut folded[0], overflow, MODULUS_TAIL);
    [folded[0], folded[1]]
}

/// Reduces a 512-bit polynomial modulo `x^256 + MODULUS_TAIL`.
///
/// This compact two-fold implementation is valid when the tail degree is at
/// most 32. Both maintained degree-256 fields satisfy that constraint. The
/// const parameter makes the field-specific strategy visible to the optimizer.
#[inline]
pub(crate) fn reduce_256<const MODULUS_TAIL: u64>(wide: Wide512) -> Limbs256 {
    validate_tail(MODULUS_TAIL);

    let high = [wide[4], wide[5], wide[6], wide[7]];
    let mut folded = [wide[0], wide[1], wide[2], wide[3], 0];
    fold_high(&mut folded, &high, MODULUS_TAIL);
    let overflow = folded[4];
    fold_overflow(&mut folded[0], overflow, MODULUS_TAIL);
    [folded[0], folded[1], folded[2], folded[3]]
}

/// Multiplies a reduced value by the polynomial-basis element `x`.
#[inline]
pub(crate) fn mul_by_x<const LIMBS: usize, const MODULUS_TAIL: u64>(
    mut value: [u64; LIMBS],
) -> [u64; LIMBS] {
    debug_assert!(LIMBS > 0);
    let reduction_mask = 0_u64.wrapping_sub(value[LIMBS - 1] >> 63);
    for index in (1..LIMBS).rev() {
        value[index] = (value[index] << 1) | (value[index - 1] >> 63);
    }
    value[0] = (value[0] << 1) ^ (MODULUS_TAIL & reduction_mask);
    value
}
