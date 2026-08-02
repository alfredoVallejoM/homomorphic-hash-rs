//! Implementation details shared with certified generated source.
//!
//! This module is public solely because Rust source emitted into a dependent
//! crate must cross the crate boundary. Its API is not a stable user contract.

#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation)]

use crate::{F2, Field, Square};

#[cfg(feature = "prime-fields")]
pub use crate::backend::prime_profile::{
    VerifiedPrimeIsaStrategy, VerifiedPrimeSimd8Strategy, VerifiedPrimeSimd16Strategy,
    VerifiedPrimeSimd32Strategy,
};
pub use crate::backend::profile::VerifiedIsaStrategy;

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

    /// Constructs the portable loops with certified prime-kernel metadata.
    #[cfg(feature = "prime-fields")]
    #[must_use]
    pub const fn new_prime(metadata: crate::PrimeKernelMetadata) -> Self {
        Self {
            kernels: crate::backend::portable::prime_kernel_set::<F>(metadata),
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

    /// Returns the immutable backend catalog associated with this field.
    ///
    /// ABI 1 and 2 generated fields inherit a portable-only catalog. Maintained
    /// fields can override this method when certified ISA strategies exist.
    #[must_use]
    fn __kernel_catalog() -> crate::kernel::KernelCatalog<Self> {
        crate::kernel::KernelCatalog::portable(Self::__portable_strategy().kernels())
    }
}

/// Generated field boundary used by verified generic ISA adapters.
///
/// Implementations are emitted only after manifest normalization,
/// irreducibility validation and deterministic reduction planning. Every
/// method remains safe even if downstream source is modified: ISA access and
/// target-feature preconditions stay inside Microfield's private backends.
pub trait VerifiedBinaryIsaField<const LIMBS: usize, const WIDE_LIMBS: usize>:
    PortableField
{
    /// Domain-separated digest of the generated compatibility profile.
    const PROFILE_DIGEST: &'static str;

    /// Value-dependence of the complete ISA product and generated reduction.
    const SCHEDULE: crate::ScheduleKind;

    /// Extracts the private polynomial limbs by value.
    fn __into_limbs(self) -> [u64; LIMBS];

    /// Reconstructs a value from already reduced canonical limbs.
    fn __from_reduced_limbs(limbs: [u64; LIMBS]) -> Self;

    /// Applies the generated reduction plan to an ISA-produced wide value.
    fn __reduce_wide(wide: [u64; WIDE_LIMBS]) -> [u64; LIMBS];
}

/// Generated radix-64 Montgomery boundary used by prime-field ISA adapters.
///
/// The trait is public only for source emitted into a dependent crate. It does
/// not expose a stable user contract: consumers receive an opaque strategy and
/// cannot supply function pointers or call target-feature code directly.
#[cfg(feature = "prime-fields")]
pub trait VerifiedPrimeMontgomery64Field<const LIMBS: usize, const WIDE_LIMBS: usize>:
    PortableField
{
    /// Canonical odd modulus used by the Microfield-owned reducer.
    const __MODULUS: [u64; LIMBS];

    /// `-p[0]^-1 mod 2^64` used to cancel each Montgomery row.
    const __NEG_INV: u64;

    /// Extracts the private Montgomery limbs by value.
    fn __into_montgomery_limbs(self) -> [u64; LIMBS];

    /// Reconstructs a value from already reduced Montgomery limbs.
    fn __from_reduced_montgomery_limbs(limbs: [u64; LIMBS]) -> Self;
}

/// Generated canonical-byte boundary used by the generic AVX2 prime adapter.
///
/// The generator proves primality and the representation contract. Microfield
/// owns the vector widening, Barrett reduction, tails and target-feature
/// boundary, so generated code never contains intrinsics or function pointers.
#[cfg(feature = "prime-fields")]
pub trait VerifiedPrimeCanonical8Field: PortableField {
    /// Odd canonical modulus. AVX2 widening supports moduli through 251.
    const __MODULUS: u16;

    /// `floor(2^16 / p)`, consumed by the Microfield-owned Barrett reducer.
    const __BARRETT_RECIPROCAL: u16;

    /// Extracts the canonical residue without exposing its storage layout.
    fn __into_canonical_u8(self) -> u8;

    /// Reconstructs a field value from a residue already proven below `p`.
    fn __from_reduced_canonical_u8(value: u8) -> Self;
}

/// Generated canonical-`u16` boundary used by the generic AVX2 prime adapter.
///
/// Products are widened to 32-bit AVX2 lanes. The upper modulus bound is the
/// largest 16-bit prime, ensuring every canonical product fits in one `u32`.
#[cfg(feature = "prime-fields")]
pub trait VerifiedPrimeCanonical16Field: PortableField {
    /// Odd canonical modulus no greater than `65_521`.
    const __MODULUS: u32;

    /// `floor(2^32 / p)`, consumed by the Microfield-owned Barrett reducer.
    const __BARRETT_RECIPROCAL: u32;

    /// Extracts the canonical residue without exposing its storage layout.
    fn __into_canonical_u16(self) -> u16;

    /// Reconstructs a field value from a residue already proven below `p`.
    fn __from_reduced_canonical_u16(value: u16) -> Self;
}

/// Generated canonical-`u32` boundary used by the experimental AVX2 adapter.
///
/// The reciprocal is `floor(2^64 / p)`. For canonical inputs, `x < p^2`
/// proves a one-correction Barrett result and keeps every wide product below
/// `2^64`. Promotion remains explicit until per-field calibration succeeds.
#[cfg(feature = "prime-fields")]
pub trait VerifiedPrimeCanonical32Field: PortableField {
    /// Odd canonical modulus no greater than `4_294_967_291`.
    const __MODULUS: u64;

    /// `floor(2^64 / p)`, consumed by the Microfield-owned Barrett reducer.
    const __BARRETT_RECIPROCAL: u64;

    /// Extracts the canonical residue without exposing its storage layout.
    fn __into_canonical_u32(self) -> u32;

    /// Reconstructs a field value from a residue already proven below `p`.
    fn __from_reduced_canonical_u32(value: u32) -> Self;
}

/// Oldest generated-source ABI accepted by this runtime.
pub const MIN_CODEGEN_ABI_VERSION: u32 = 1;

/// Newest generated-source ABI accepted by this runtime.
pub const MAX_CODEGEN_ABI_VERSION: u32 = 3;

/// ABI emitted by the current generator.
pub const CURRENT_CODEGEN_ABI_VERSION: u32 = 3;

const _: () = assert!(
    CURRENT_CODEGEN_ABI_VERSION >= MIN_CODEGEN_ABI_VERSION
        && CURRENT_CODEGEN_ABI_VERSION <= MAX_CODEGEN_ABI_VERSION
);

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

/// Optimized schoolbook product followed by sparse descending reduction.
#[must_use]
pub fn multiply_sparse<const LIMBS: usize, const WIDE_LIMBS: usize>(
    lhs: [u64; LIMBS],
    rhs: [u64; LIMBS],
    degree: usize,
    modulus_exponents_desc: &[usize],
) -> [u64; LIMBS] {
    let product = wide_product::<LIMBS, WIDE_LIMBS>(lhs, rhs);
    reduce_wide_sparse(product, degree, modulus_exponents_desc)
}

/// Optimized schoolbook product followed by packed dense-tail reduction.
#[must_use]
pub fn multiply_dense<const LIMBS: usize, const WIDE_LIMBS: usize>(
    lhs: [u64; LIMBS],
    rhs: [u64; LIMBS],
    degree: usize,
    modulus_tail: &[u64; LIMBS],
) -> [u64; LIMBS] {
    let product = wide_product::<LIMBS, WIDE_LIMBS>(lhs, rhs);
    reduce_wide_dense(product, degree, modulus_tail)
}

/// Optimized product and bounded fold for aligned low-tail moduli.
#[must_use]
pub fn multiply_low_tail<const LIMBS: usize, const WIDE_LIMBS: usize, const MODULUS_TAIL: u64>(
    lhs: [u64; LIMBS],
    rhs: [u64; LIMBS],
) -> [u64; LIMBS] {
    let product = wide_product::<LIMBS, WIDE_LIMBS>(lhs, rhs);
    reduce_wide_low_tail::<LIMBS, WIDE_LIMBS, MODULUS_TAIL>(product)
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

/// Dedicated bit-spreading square followed by sparse descending reduction.
#[must_use]
pub fn square_sparse<const LIMBS: usize, const WIDE_LIMBS: usize>(
    value: [u64; LIMBS],
    degree: usize,
    modulus_exponents_desc: &[usize],
) -> [u64; LIMBS] {
    let product = wide_square::<LIMBS, WIDE_LIMBS>(value);
    reduce_wide_sparse(product, degree, modulus_exponents_desc)
}

/// Dedicated bit-spreading square followed by packed dense-tail reduction.
#[must_use]
pub fn square_dense<const LIMBS: usize, const WIDE_LIMBS: usize>(
    value: [u64; LIMBS],
    degree: usize,
    modulus_tail: &[u64; LIMBS],
) -> [u64; LIMBS] {
    let product = wide_square::<LIMBS, WIDE_LIMBS>(value);
    reduce_wide_dense(product, degree, modulus_tail)
}

/// Dedicated bit-spreading square and bounded aligned low-tail fold.
#[must_use]
pub fn square_low_tail<const LIMBS: usize, const WIDE_LIMBS: usize, const MODULUS_TAIL: u64>(
    value: [u64; LIMBS],
) -> [u64; LIMBS] {
    let product = wide_square::<LIMBS, WIDE_LIMBS>(value);
    reduce_wide_low_tail::<LIMBS, WIDE_LIMBS, MODULUS_TAIL>(product)
}

/// Reduces a verified ISA product using the generated sparse plan.
#[must_use]
pub fn reduce_sparse<const LIMBS: usize, const WIDE_LIMBS: usize>(
    product: [u64; WIDE_LIMBS],
    degree: usize,
    modulus_exponents_desc: &[usize],
) -> [u64; LIMBS] {
    reduce_wide_sparse(product, degree, modulus_exponents_desc)
}

/// Reduces a verified ISA product using the generated dense-tail plan.
#[must_use]
pub fn reduce_dense<const LIMBS: usize, const WIDE_LIMBS: usize>(
    product: [u64; WIDE_LIMBS],
    degree: usize,
    modulus_tail: &[u64; LIMBS],
) -> [u64; LIMBS] {
    reduce_wide_dense(product, degree, modulus_tail)
}

/// Reduces a verified ISA product using the bounded aligned low-tail plan.
#[must_use]
pub fn reduce_low_tail<const LIMBS: usize, const WIDE_LIMBS: usize, const MODULUS_TAIL: u64>(
    product: [u64; WIDE_LIMBS],
) -> [u64; LIMBS] {
    reduce_wide_low_tail::<LIMBS, WIDE_LIMBS, MODULUS_TAIL>(product)
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

/// Inverts with an Itoh--Tsujii binary addition chain selected statically.
#[must_use]
pub fn invert_itoh_tsujii<F: Field + Square, const DEGREE: usize>(value: F) -> Option<F> {
    if value.is_zero() {
        return None;
    }
    debug_assert!(DEGREE >= 2);
    let target = DEGREE - 1;
    let highest_bit = usize::BITS as usize - 1 - target.leading_zeros() as usize;
    let mut block = 1_usize;
    let mut result = value;

    for bit_index in (0..highest_bit).rev() {
        let previous = result;
        result = repeat_square(result, block).mul(previous);
        block *= 2;
        if (target >> bit_index) & 1 != 0 {
            result = result.square().mul(value);
            block += 1;
        }
    }
    debug_assert_eq!(block, target);
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
fn wide_product<const LIMBS: usize, const WIDE_LIMBS: usize>(
    lhs: [u64; LIMBS],
    rhs: [u64; LIMBS],
) -> [u64; WIDE_LIMBS] {
    assert!(WIDE_LIMBS >= LIMBS.saturating_mul(2));
    let mut product = [0; WIDE_LIMBS];
    for (left_index, left) in lhs.iter().copied().enumerate() {
        for (right_index, right) in rhs.iter().copied().enumerate() {
            let (low, high) = clmul64_set_bits(left, right);
            product[left_index + right_index] ^= low;
            product[left_index + right_index + 1] ^= high;
        }
    }
    product
}

#[inline]
fn wide_square<const LIMBS: usize, const WIDE_LIMBS: usize>(
    value: [u64; LIMBS],
) -> [u64; WIDE_LIMBS] {
    assert!(WIDE_LIMBS >= LIMBS.saturating_mul(2));
    let mut product = [0; WIDE_LIMBS];
    for (index, limb) in value.iter().copied().enumerate() {
        product[index * 2] = spread32(limb);
        product[index * 2 + 1] = spread32(limb >> 32);
    }
    product
}

fn reduce_wide_sparse<const LIMBS: usize, const WIDE_LIMBS: usize>(
    mut product: [u64; WIDE_LIMBS],
    degree: usize,
    modulus_exponents_desc: &[usize],
) -> [u64; LIMBS] {
    if degree > 1 {
        for source_bit in (degree..=(degree * 2 - 2)).rev() {
            if bit(&product, source_bit) {
                toggle(&mut product, source_bit);
                let shift = source_bit - degree;
                for &exponent in &modulus_exponents_desc[1..] {
                    toggle(&mut product, shift + exponent);
                }
            }
        }
    }
    low_limbs(product, degree)
}

fn reduce_wide_dense<const LIMBS: usize, const WIDE_LIMBS: usize>(
    mut product: [u64; WIDE_LIMBS],
    degree: usize,
    modulus_tail: &[u64; LIMBS],
) -> [u64; LIMBS] {
    if degree > 1 {
        for source_bit in (degree..=(degree * 2 - 2)).rev() {
            if bit(&product, source_bit) {
                toggle(&mut product, source_bit);
                xor_shifted_words(&mut product, modulus_tail, source_bit - degree);
            }
        }
    }
    low_limbs(product, degree)
}

fn reduce_wide_low_tail<const LIMBS: usize, const WIDE_LIMBS: usize, const MODULUS_TAIL: u64>(
    mut product: [u64; WIDE_LIMBS],
) -> [u64; LIMBS] {
    assert!(LIMBS > 0);
    assert!(WIDE_LIMBS >= LIMBS.saturating_mul(2));
    debug_assert_eq!(MODULUS_TAIL & 1, 1);
    debug_assert!(u64::BITS - MODULUS_TAIL.leading_zeros() <= 33);

    for source_index in LIMBS..LIMBS * 2 {
        let high = product[source_index];
        product[source_index] = 0;
        xor_low_tail(&mut product, source_index - LIMBS, high, MODULUS_TAIL);
    }
    let overflow = product[LIMBS];
    product[LIMBS] = 0;
    let mut tail = MODULUS_TAIL;
    while tail != 0 {
        product[0] ^= overflow << tail.trailing_zeros();
        tail &= tail - 1;
    }
    low_limbs(product, LIMBS * 64)
}

#[inline]
fn xor_low_tail<const WIDE_LIMBS: usize>(
    product: &mut [u64; WIDE_LIMBS],
    target: usize,
    high: u64,
    mut tail: u64,
) {
    while tail != 0 {
        let shift = tail.trailing_zeros();
        product[target] ^= high << shift;
        if shift != 0 {
            product[target + 1] ^= high >> (u64::BITS - shift);
        }
        tail &= tail - 1;
    }
}

#[inline]
fn xor_shifted_words<const SOURCE_LIMBS: usize, const TARGET_LIMBS: usize>(
    target: &mut [u64; TARGET_LIMBS],
    source: &[u64; SOURCE_LIMBS],
    shift: usize,
) {
    let word_shift = shift / 64;
    let bit_shift = shift % 64;
    for (index, word) in source.iter().copied().enumerate() {
        target[word_shift + index] ^= word << bit_shift;
        if bit_shift != 0 && word_shift + index + 1 < TARGET_LIMBS {
            target[word_shift + index + 1] ^= word >> (64 - bit_shift);
        }
    }
}

fn low_limbs<const LIMBS: usize, const WIDE_LIMBS: usize>(
    product: [u64; WIDE_LIMBS],
    degree: usize,
) -> [u64; LIMBS] {
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

#[inline]
fn clmul64_set_bits(lhs: u64, rhs: u64) -> (u64, u64) {
    let mut low = 0;
    let mut high = 0;
    let mut remaining = rhs;
    while remaining != 0 {
        let shift = remaining.trailing_zeros();
        low ^= lhs << shift;
        if shift != 0 {
            high ^= lhs >> (u64::BITS - shift);
        }
        remaining &= remaining - 1;
    }
    (low, high)
}

#[inline]
fn spread32(mut value: u64) -> u64 {
    value &= 0x0000_0000_ffff_ffff;
    value = (value | (value << 16)) & 0x0000_ffff_0000_ffff;
    value = (value | (value << 8)) & 0x00ff_00ff_00ff_00ff;
    value = (value | (value << 4)) & 0x0f0f_0f0f_0f0f_0f0f;
    value = (value | (value << 2)) & 0x3333_3333_3333_3333;
    (value | (value << 1)) & 0x5555_5555_5555_5555
}

fn repeat_square<F: Square>(mut value: F, count: usize) -> F {
    for _ in 0..count {
        value = value.square();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_padding_is_zero, mul_by_x, multiply, multiply_dense, multiply_low_tail,
        multiply_sparse, reduce_polynomial_bytes, square, square_dense, square_low_tail,
        square_sparse,
    };

    const MODULUS_3: &[usize] = &[3, 1, 0];

    #[test]
    fn codegen_abi_keeps_the_n_minus_one_window() {
        assert_eq!(super::MIN_CODEGEN_ABI_VERSION, 1);
        assert_eq!(super::CURRENT_CODEGEN_ABI_VERSION, 3);
        assert_eq!(super::MAX_CODEGEN_ABI_VERSION, 3);
        for version in super::MIN_CODEGEN_ABI_VERSION..=super::MAX_CODEGEN_ABI_VERSION {
            assert!(super::supports_codegen_abi(version));
        }
        assert!(!super::supports_codegen_abi(0));
        assert!(!super::supports_codegen_abi(4));
        assert!(!super::supports_codegen_abi(u32::MAX));

        let matrix = include_str!("../abi/runtime-codegen-matrix-v1.csv");
        assert_eq!(
            matrix,
            "runtime_series,min_codegen_abi,max_codegen_abi,current_codegen_abi,manifest_schema,artifact_schema,compatibility\n\
             0.1.x,1,3,3,1,1,N_and_N_minus_1_or_longer\n"
        );
    }

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

    #[test]
    fn aligned_low_tail_path_matches_the_v1_reference() {
        const MODULUS: &[usize] = &[128, 7, 2, 1, 0];
        let mut state = 0x6a09_e667_f3bc_c909_u64;
        let rounds = if cfg!(miri) { 8 } else { 256 };
        for _ in 0..rounds {
            let lhs = [next(&mut state), next(&mut state)];
            let rhs = [next(&mut state), next(&mut state)];
            assert_eq!(
                multiply_low_tail::<2, 4, 0x87>(lhs, rhs),
                multiply::<2, 4>(lhs, rhs, 128, MODULUS)
            );
            assert_eq!(
                square_low_tail::<2, 4, 0x87>(lhs),
                square::<2, 4>(lhs, 128, MODULUS)
            );
        }
    }

    #[test]
    fn aligned_power_of_two_degrees_share_the_certified_fold() {
        check_aligned_power_of_two::<1, 2>(64);
        check_aligned_power_of_two::<2, 4>(128);
        check_aligned_power_of_two::<4, 8>(256);
        check_aligned_power_of_two::<8, 16>(512);
        check_aligned_power_of_two::<16, 32>(1024);
        check_aligned_power_of_two::<32, 64>(2048);
        check_aligned_power_of_two::<64, 128>(4096);
    }

    #[test]
    fn unaligned_sparse_path_matches_the_v1_reference() {
        const MODULUS: &[usize] = &[233, 74, 0];
        let mut state = 0xbb67_ae85_84ca_a73b_u64;
        let rounds = if cfg!(miri) { 4 } else { 96 };
        for _ in 0..rounds {
            let mut lhs = [0_u64; 4];
            let mut rhs = [0_u64; 4];
            for limb in &mut lhs {
                *limb = next(&mut state);
            }
            for limb in &mut rhs {
                *limb = next(&mut state);
            }
            lhs[3] &= (1_u64 << 41) - 1;
            rhs[3] &= (1_u64 << 41) - 1;
            assert_eq!(
                multiply_sparse::<4, 8>(lhs, rhs, 233, MODULUS),
                multiply::<4, 8>(lhs, rhs, 233, MODULUS)
            );
            assert_eq!(
                square_sparse::<4, 8>(lhs, 233, MODULUS),
                square::<4, 8>(lhs, 233, MODULUS)
            );
        }
    }

    #[test]
    fn dense_word_path_matches_term_by_term_reduction() {
        let modulus = core::array::from_fn::<_, 71, _>(|index| 70 - index);
        let tail = [u64::MAX, 0x3f];
        let mut state = 0x3c6e_f372_fe94_f82b_u64;
        let rounds = if cfg!(miri) { 4 } else { 96 };
        for _ in 0..rounds {
            let lhs = [next(&mut state), next(&mut state) & 0x3f];
            let rhs = [next(&mut state), next(&mut state) & 0x3f];
            assert_eq!(
                multiply_dense::<2, 4>(lhs, rhs, 70, &tail),
                multiply::<2, 4>(lhs, rhs, 70, &modulus)
            );
            assert_eq!(
                square_dense::<2, 4>(lhs, 70, &tail),
                square::<2, 4>(lhs, 70, &modulus)
            );
        }
    }

    fn next(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    fn check_aligned_power_of_two<const LIMBS: usize, const WIDE_LIMBS: usize>(degree: usize) {
        const TAIL: u64 = 0x125;
        let modulus = [degree, 8, 5, 2, 0];
        let mut state = 0xa54f_f53a_5f1d_36f1_u64 ^ degree as u64;
        let rounds = if cfg!(miri) { 1 } else { 8 };
        for _ in 0..rounds {
            let lhs = core::array::from_fn(|_| next(&mut state));
            let rhs = core::array::from_fn(|_| next(&mut state));
            assert_eq!(
                multiply_low_tail::<LIMBS, WIDE_LIMBS, TAIL>(lhs, rhs),
                multiply::<LIMBS, WIDE_LIMBS>(lhs, rhs, degree, &modulus)
            );
            assert_eq!(
                square_low_tail::<LIMBS, WIDE_LIMBS, TAIL>(lhs),
                square::<LIMBS, WIDE_LIMBS>(lhs, degree, &modulus)
            );
        }
    }
}
