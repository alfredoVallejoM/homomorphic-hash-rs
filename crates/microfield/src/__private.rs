//! Implementation details shared with certified generated source.
//!
//! This module is public solely because Rust source emitted into a dependent
//! crate must cross the crate boundary. Its API is not a stable user contract.

#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation)]

use crate::{F2, Field, Square};

/// Opaque, safe portable strategy generated for one statically defined field.
#[cfg_attr(not(feature = "portable"), allow(dead_code))]
pub struct PortableStrategy<F: Field + Square> {
    kernels: crate::kernel::KernelSet<F>,
}

impl<F: Field + Square> PortableStrategy<F> {
    /// Constructs only Microfield's fixed safe portable strategy.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            kernels: crate::portable_kernel_set::<F>(),
        }
    }

    #[cfg_attr(not(feature = "portable"), allow(dead_code))]
    pub(crate) const fn kernels(&self) -> &crate::kernel::KernelSet<F> {
        &self.kernels
    }
}

impl<F: Field + Square> Default for PortableStrategy<F> {
    fn default() -> Self {
        Self::new()
    }
}

/// Generated capability that associates a field with its safe static strategy.
pub trait PortableField: Field + Square {
    /// Returns the strategy emitted beside the nominal field type.
    fn __portable_strategy() -> &'static PortableStrategy<Self>;
}

/// Oldest generated-source ABI accepted by this runtime.
pub const MIN_CODEGEN_ABI_VERSION: u32 = 1;

/// Newest generated-source ABI accepted by this runtime.
pub const MAX_CODEGEN_ABI_VERSION: u32 = 1;

/// Reports whether generated source can safely call this runtime helper set.
#[must_use]
pub const fn supports_codegen_abi(version: u32) -> bool {
    version >= MIN_CODEGEN_ABI_VERSION && version <= MAX_CODEGEN_ABI_VERSION
}

#[inline]
#[must_use]
pub fn add<const LIMBS: usize>(lhs: [u64; LIMBS], rhs: [u64; LIMBS]) -> [u64; LIMBS] {
    let mut result = [0; LIMBS];
    let mut index = 0;
    while index < LIMBS {
        result[index] = lhs[index] ^ rhs[index];
        index += 1;
    }
    result
}

#[inline]
#[must_use]
pub fn is_zero<const LIMBS: usize>(limbs: &[u64; LIMBS]) -> bool {
    limbs.iter().all(|limb| *limb == 0)
}

#[must_use]
pub fn multiply<const LIMBS: usize, const WIDE_LIMBS: usize>(
    lhs: [u64; LIMBS],
    rhs: [u64; LIMBS],
    degree: usize,
    modulus_exponents_desc: &[usize],
) -> [u64; LIMBS] {
    assert!(WIDE_LIMBS >= LIMBS.saturating_mul(2));
    let mut product = [0; WIDE_LIMBS];
    for (left_index, left) in lhs.iter().copied().enumerate() {
        for (right_index, right) in rhs.iter().copied().enumerate() {
            let partial = clmul64(left, right);
            product[left_index + right_index] ^= partial as u64;
            product[left_index + right_index + 1] ^= (partial >> 64) as u64;
        }
    }
    reduce_wide(product, degree, modulus_exponents_desc)
}

#[must_use]
pub fn square<const LIMBS: usize, const WIDE_LIMBS: usize>(
    value: [u64; LIMBS],
    degree: usize,
    modulus_exponents_desc: &[usize],
) -> [u64; LIMBS] {
    assert!(WIDE_LIMBS >= LIMBS.saturating_mul(2));
    let mut product = [0; WIDE_LIMBS];
    for source_bit in 0..degree {
        if bit(&value, source_bit) {
            toggle(&mut product, source_bit * 2);
        }
    }
    reduce_wide(product, degree, modulus_exponents_desc)
}

#[must_use]
pub fn mul_by_x<const LIMBS: usize>(
    mut value: [u64; LIMBS],
    degree: usize,
    modulus_exponents_desc: &[usize],
) -> [u64; LIMBS] {
    let overflow = bit(&value, degree - 1);
    let mut carry = 0;
    for limb in &mut value {
        let next = *limb >> 63;
        *limb = (*limb << 1) | carry;
        carry = next;
    }
    mask_unused_bits(&mut value, degree);
    if overflow {
        for &exponent in modulus_exponents_desc {
            if exponent != degree {
                toggle(&mut value, exponent);
            }
        }
    }
    value
}

#[must_use]
pub fn reduce_polynomial_bytes<const LIMBS: usize>(
    bytes_le: &[u8],
    degree: usize,
    modulus_exponents_desc: &[usize],
) -> [u64; LIMBS] {
    let mut result = [0; LIMBS];
    for byte in bytes_le.iter().rev().copied() {
        for bit_index in (0..8).rev() {
            result = mul_by_x(result, degree, modulus_exponents_desc);
            result[0] ^= u64::from((byte >> bit_index) & 1);
        }
    }
    result
}

#[inline]
#[must_use]
pub fn canonical_padding_is_zero(bytes: &[u8], degree: usize) -> bool {
    let used = degree % 8;
    used == 0 || bytes.last().is_none_or(|last| last >> used == 0)
}

#[must_use]
pub fn decode<const LIMBS: usize>(bytes: &[u8]) -> [u64; LIMBS] {
    let mut result = [0; LIMBS];
    for (index, byte) in bytes.iter().copied().enumerate() {
        result[index / 8] |= u64::from(byte) << ((index % 8) * 8);
    }
    result
}

pub fn encode<const LIMBS: usize>(limbs: [u64; LIMBS], bytes: &mut [u8]) {
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = (limbs[index / 8] >> ((index % 8) * 8)) as u8;
    }
}

#[must_use]
pub fn invert<F: Field + Square, const DEGREE: usize>(value: F) -> Option<F> {
    if value.is_zero() {
        return None;
    }
    let mut result = value;
    for _ in 1..DEGREE - 1 {
        result = result.square().mul(value);
    }
    Some(result.square())
}

#[must_use]
pub fn frobenius<F: Field + Square, const DEGREE: usize>(mut value: F, power: usize) -> F {
    for _ in 0..power % DEGREE {
        value = value.square();
    }
    value
}

#[must_use]
pub fn trace<F: Field + Square, const DEGREE: usize>(value: F) -> F2 {
    let mut conjugate = value;
    let mut trace = F::ZERO;
    for _ in 0..DEGREE {
        trace = trace.add(conjugate);
        conjugate = conjugate.square();
    }
    F2::from_bool(trace == F::ONE)
}

fn reduce_wide<const LIMBS: usize, const WIDE_LIMBS: usize>(
    mut product: [u64; WIDE_LIMBS],
    degree: usize,
    modulus_exponents_desc: &[usize],
) -> [u64; LIMBS] {
    if degree > 1 {
        for source_bit in (degree..=(degree * 2 - 2)).rev() {
            if bit(&product, source_bit) {
                let shift = source_bit - degree;
                for &exponent in modulus_exponents_desc {
                    toggle(&mut product, shift + exponent);
                }
            }
        }
    }
    let mut result = [0; LIMBS];
    result.copy_from_slice(&product[..LIMBS]);
    mask_unused_bits(&mut result, degree);
    result
}

#[inline]
fn bit<const LIMBS: usize>(limbs: &[u64; LIMBS], position: usize) -> bool {
    ((limbs[position / 64] >> (position % 64)) & 1) != 0
}

#[inline]
fn toggle<const LIMBS: usize>(limbs: &mut [u64; LIMBS], position: usize) {
    limbs[position / 64] ^= 1_u64 << (position % 64);
}

#[inline]
fn mask_unused_bits<const LIMBS: usize>(limbs: &mut [u64; LIMBS], degree: usize) {
    let used = degree % 64;
    if used != 0 {
        limbs[LIMBS - 1] &= (1_u64 << used) - 1;
    }
}

#[inline]
fn clmul64(lhs: u64, rhs: u64) -> u128 {
    let mut result = 0_u128;
    for bit_index in 0..64 {
        let mask = 0_u128.wrapping_sub(u128::from((rhs >> bit_index) & 1));
        result ^= (u128::from(lhs) << bit_index) & mask;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{canonical_padding_is_zero, mul_by_x, multiply, reduce_polynomial_bytes, square};

    const MODULUS_3: &[usize] = &[3, 1, 0];

    #[test]
    fn gf8_known_products_and_square_agree() {
        for lhs in 0_u64..8 {
            for rhs in 0_u64..8 {
                let product = multiply::<1, 2>([lhs], [rhs], 3, MODULUS_3);
                let mut repeated = [0_u64; 1];
                let mut term = [lhs];
                for index in 0..3 {
                    if (rhs >> index) & 1 != 0 {
                        repeated[0] ^= term[0];
                    }
                    term = mul_by_x(term, 3, MODULUS_3);
                }
                assert_eq!(product, repeated);
            }
            assert_eq!(
                square::<1, 2>([lhs], 3, MODULUS_3),
                multiply::<1, 2>([lhs], [lhs], 3, MODULUS_3)
            );
        }
    }

    #[test]
    fn arbitrary_polynomial_reduction_uses_every_input_bit() {
        assert_eq!(reduce_polynomial_bytes::<1>(&[0, 1], 3, MODULUS_3), [2]);
        assert_eq!(
            reduce_polynomial_bytes::<1>(&[0xff, 0xff], 3, MODULUS_3),
            [3]
        );
    }

    #[test]
    fn partial_byte_canonicality_is_strict() {
        assert!(canonical_padding_is_zero(&[0xff, 0x01], 9));
        assert!(!canonical_padding_is_zero(&[0xff, 0x02], 9));
        assert!(canonical_padding_is_zero(&[0xff], 8));
    }
}
