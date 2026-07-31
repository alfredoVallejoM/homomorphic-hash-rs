//! Readable streaming polynomial reduction.

use super::reduction::mul_by_x;

/// Reduces an arbitrary-length little-endian polynomial without allocation.
///
/// This is the non-hot-path implementation behind
/// `BinaryPolynomialField::from_polynomial_bytes_mod`.
pub(crate) fn reduce_polynomial_bytes<const LIMBS: usize, const MODULUS_TAIL: u64>(
    bytes_le: &[u8],
) -> [u64; LIMBS] {
    let mut result = [0; LIMBS];
    for byte in bytes_le.iter().rev().copied() {
        for bit in (0..u8::BITS).rev() {
            result = mul_by_x::<LIMBS, MODULUS_TAIL>(result);
            result[0] ^= u64::from((byte >> bit) & 1);
        }
    }
    result
}
