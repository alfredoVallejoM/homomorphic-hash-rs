//! Portable reduction for `2^64 - 2^32 + 1`.

// Each narrowing operation intentionally extracts the low word after a
// bounded fold or a remainder below the 64-bit modulus.
#![allow(clippy::cast_possible_truncation)]

pub(crate) const MODULUS: u64 = 0xffff_ffff_0000_0001;
const EPSILON: u128 = 0xffff_ffff;
const BARRETT_RECIPROCAL: u128 = (1_u128 << 64) + 0xffff_ffff;

/// Reduces one full `u128` product using `2^64 = 2^32 - 1 (mod p)`.
#[inline]
#[must_use]
pub(crate) fn reduce_goldilocks(wide: u128) -> u64 {
    let mut folded = fold(wide);
    folded = fold(folded);
    folded = fold(folded);
    folded = fold(folded);
    debug_assert!(folded <= u128::from(u64::MAX));
    let mut residue = folded as u64;
    if residue >= MODULUS {
        residue -= MODULUS;
    }
    residue
}

#[inline]
fn fold(value: u128) -> u128 {
    u128::from(value as u64) + (value >> 64) * EPSILON
}

/// Reduces a full product with the generated Barrett reciprocal.
#[inline]
#[must_use]
pub(crate) fn barrett_reduce_goldilocks(wide: u128) -> u64 {
    let quotient = mul_high_u128(wide, BARRETT_RECIPROCAL);
    let mut residue = wide - quotient * u128::from(MODULUS);
    if residue >= u128::from(MODULUS) {
        residue -= u128::from(MODULUS);
    }
    residue as u64
}

#[inline]
fn mul_high_u128(lhs: u128, rhs: u128) -> u128 {
    let left_low = lhs as u64;
    let left_high = (lhs >> 64) as u64;
    let right_low = rhs as u64;
    let right_high = (rhs >> 64) as u64;
    let low = u128::from(left_low) * u128::from(right_low);
    let cross_left = u128::from(left_low) * u128::from(right_high);
    let cross_right = u128::from(left_high) * u128::from(right_low);
    let middle = (low >> 64) + u128::from(cross_left as u64) + u128::from(cross_right as u64);
    u128::from(left_high) * u128::from(right_high)
        + (cross_left >> 64)
        + (cross_right >> 64)
        + (middle >> 64)
}

#[cfg(test)]
fn native_reduce_goldilocks(wide: u128) -> u64 {
    (wide % u128::from(MODULUS)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_reduction_matches_native_remainder_at_boundaries() {
        let values = [
            0,
            1,
            u128::from(MODULUS) - 1,
            u128::from(MODULUS),
            u128::from(MODULUS) + 1,
            u128::from(u64::MAX) * u128::from(u64::MAX),
            u128::MAX,
        ];
        for value in values {
            let expected = native_reduce_goldilocks(value);
            assert_eq!(reduce_goldilocks(value), expected);
            assert_eq!(barrett_reduce_goldilocks(value), expected);
        }
    }

    #[test]
    fn special_reduction_maps_every_wide_basis_bit() {
        let mut expected = 1_u64;
        for bit in 0..128 {
            let wide = 1_u128 << bit;
            assert_eq!(reduce_goldilocks(wide), expected, "basis bit {bit}");
            expected = ((u128::from(expected) * 2) % u128::from(MODULUS)) as u64;
        }
    }
}
