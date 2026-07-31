//! Generated polynomial reduction.

use super::representation::{Limbs256, Wide512};

/// Reduces a 512-bit polynomial modulo `x^256 + MODULUS_TAIL`.
///
/// This compact two-fold implementation is valid when the tail degree is at
/// most 32. Both maintained degree-256 fields satisfy that constraint. The
/// const parameter makes the field-specific strategy visible to the optimizer.
#[inline]
pub(crate) fn reduce_256<const MODULUS_TAIL: u64>(wide: Wide512) -> Limbs256 {
    debug_assert!(MODULUS_TAIL & 1 == 1);
    debug_assert!(u64::BITS - MODULUS_TAIL.leading_zeros() <= 33);

    let high = [wide[4], wide[5], wide[6], wide[7]];
    let mut folded = [wide[0], wide[1], wide[2], wide[3], 0];
    let mut tail = MODULUS_TAIL;

    while tail != 0 {
        let shift = tail.trailing_zeros();
        for (index, limb) in high.into_iter().enumerate() {
            folded[index] ^= limb << shift;
            if shift != 0 {
                folded[index + 1] ^= limb >> (u64::BITS - shift);
            }
        }
        tail &= tail - 1;
    }

    let overflow = folded[4];
    let mut tail = MODULUS_TAIL;
    while tail != 0 {
        folded[0] ^= overflow << tail.trailing_zeros();
        tail &= tail - 1;
    }

    [folded[0], folded[1], folded[2], folded[3]]
}

/// Multiplies a reduced value by the polynomial-basis element `x`.
#[inline]
pub(crate) fn mul_by_x_256<const MODULUS_TAIL: u64>(value: Limbs256) -> Limbs256 {
    let reduction_mask = 0_u64.wrapping_sub(value[3] >> 63);
    [
        (value[0] << 1) ^ (MODULUS_TAIL & reduction_mask),
        (value[1] << 1) | (value[0] >> 63),
        (value[2] << 1) | (value[1] >> 63),
        (value[3] << 1) | (value[2] >> 63),
    ]
}
