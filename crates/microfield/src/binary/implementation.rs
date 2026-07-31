//! Static composition boundary for generated binary fields.
//!
//! Concrete field value objects delegate to the small algorithm modules in
//! this package. No runtime registry or dynamic dispatch participates in the
//! scalar path.

use core::hash::Hash;

use super::{
    portable::{wide_product_128, wide_product_256},
    reduction::{mul_by_x, reduce_128, reduce_256},
    reference::reduce_polynomial_bytes,
    representation::{Limbs128, Limbs256},
    square::{square_128, square_256},
};

/// Private representation capability used by static field implementations.
pub(crate) trait LimbArray:
    Copy + Default + Eq + Hash + AsRef<[u64]> + AsMut<[u64]>
{
}

impl LimbArray for Limbs128 {}
impl LimbArray for Limbs256 {}

/// Zero-cost strategy contract implemented by validated binary field shapes.
pub(crate) trait BinaryFieldImpl: Copy + 'static {
    /// Private reduced representation.
    type Limbs: LimbArray;

    /// Extension degree and representation width.
    const DEGREE: usize;
    /// Number of canonical bytes.
    const CANONICAL_BYTES: usize;

    /// Multiplies and reduces two values.
    fn multiply(lhs: Self::Limbs, rhs: Self::Limbs) -> Self::Limbs;
    /// Executes the dedicated square strategy.
    fn square(value: Self::Limbs) -> Self::Limbs;
    /// Multiplies by the polynomial-basis element `x`.
    fn mul_by_x(value: Self::Limbs) -> Self::Limbs;
    /// Reduces an arbitrary little-endian polynomial.
    fn reduce_polynomial_bytes(bytes_le: &[u8]) -> Self::Limbs;
}

/// Static strategy for a two-limb polynomial field.
#[derive(Clone, Copy)]
pub(crate) struct Polynomial128<const MODULUS_TAIL: u64>;

impl<const MODULUS_TAIL: u64> BinaryFieldImpl for Polynomial128<MODULUS_TAIL> {
    type Limbs = Limbs128;

    const DEGREE: usize = 128;
    const CANONICAL_BYTES: usize = 16;

    #[inline]
    fn multiply(lhs: Self::Limbs, rhs: Self::Limbs) -> Self::Limbs {
        reduce_128::<MODULUS_TAIL>(wide_product_128(lhs, rhs))
    }

    #[inline]
    fn square(value: Self::Limbs) -> Self::Limbs {
        square_128::<MODULUS_TAIL>(value)
    }

    #[inline]
    fn mul_by_x(value: Self::Limbs) -> Self::Limbs {
        mul_by_x::<2, MODULUS_TAIL>(value)
    }

    fn reduce_polynomial_bytes(bytes_le: &[u8]) -> Self::Limbs {
        reduce_polynomial_bytes::<2, MODULUS_TAIL>(bytes_le)
    }
}

/// Static strategy for a four-limb polynomial field.
#[derive(Clone, Copy)]
pub(crate) struct Polynomial256<const MODULUS_TAIL: u64>;

impl<const MODULUS_TAIL: u64> BinaryFieldImpl for Polynomial256<MODULUS_TAIL> {
    type Limbs = Limbs256;

    const DEGREE: usize = 256;
    const CANONICAL_BYTES: usize = 32;

    #[inline]
    fn multiply(lhs: Self::Limbs, rhs: Self::Limbs) -> Self::Limbs {
        reduce_256::<MODULUS_TAIL>(wide_product_256(lhs, rhs))
    }

    #[inline]
    fn square(value: Self::Limbs) -> Self::Limbs {
        square_256::<MODULUS_TAIL>(value)
    }

    #[inline]
    fn mul_by_x(value: Self::Limbs) -> Self::Limbs {
        mul_by_x::<4, MODULUS_TAIL>(value)
    }

    fn reduce_polynomial_bytes(bytes_le: &[u8]) -> Self::Limbs {
        reduce_polynomial_bytes::<4, MODULUS_TAIL>(bytes_le)
    }
}

/// XORs two equal-width private representations.
#[inline]
pub(crate) fn add_limbs<L: LimbArray>(mut lhs: L, rhs: L) -> L {
    for (output, input) in lhs.as_mut().iter_mut().zip(rhs.as_ref()) {
        *output ^= input;
    }
    lhs
}

/// Tests all limbs without exposing the representation to the public API.
#[inline]
pub(crate) fn limbs_are_zero<L: LimbArray>(limbs: &L) -> bool {
    limbs
        .as_ref()
        .iter()
        .copied()
        .fold(0, |accumulator, limb| accumulator | limb)
        == 0
}

/// Decodes little-endian bytes into an internal limb array.
pub(crate) fn decode_limbs<L: LimbArray>(bytes: &[u8]) -> L {
    debug_assert_eq!(bytes.len(), L::default().as_ref().len() * 8);
    let mut limbs = L::default();
    for (limb, chunk) in limbs.as_mut().iter_mut().zip(bytes.chunks_exact(8)) {
        *limb = u64::from_le_bytes(
            chunk
                .try_into()
                .expect("an eight-byte chunk always forms one limb"),
        );
    }
    limbs
}

/// Encodes an internal limb array without allocation.
pub(crate) fn encode_limbs<L: LimbArray>(limbs: L, bytes: &mut [u8]) {
    debug_assert_eq!(bytes.len(), limbs.as_ref().len() * 8);
    for (limb, output) in limbs.as_ref().iter().zip(bytes.chunks_exact_mut(8)) {
        output.copy_from_slice(&limb.to_le_bytes());
    }
}
