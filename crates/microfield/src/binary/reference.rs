//! Readable streaming polynomial reduction.

use super::reduction::mul_by_x_256;
use super::representation::Limbs256;

/// Reduces an arbitrary-length little-endian polynomial without allocation.
///
/// This is the non-hot-path implementation behind
/// `BinaryPolynomialField::from_polynomial_bytes_mod`.
pub(crate) fn reduce_polynomial_bytes_256<const MODULUS_TAIL: u64>(bytes_le: &[u8]) -> Limbs256 {
    let mut result = [0; 4];
    for byte in bytes_le.iter().rev().copied() {
        for bit in (0..u8::BITS).rev() {
            result = mul_by_x_256::<MODULUS_TAIL>(result);
            result[0] ^= u64::from((byte >> bit) & 1);
        }
    }
    result
}
