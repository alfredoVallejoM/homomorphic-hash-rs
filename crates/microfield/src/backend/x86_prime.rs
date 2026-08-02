//! Narrow x86-64 adapters for independent prime residues.

// The unaligned load/store intrinsics intentionally accept byte-aligned
// addresses even though their pointer type carries vector alignment.
#![allow(clippy::cast_ptr_alignment)]
// Every narrowed tail or test value is reduced below its target lane width.
#![allow(clippy::cast_possible_truncation)]

use core::arch::x86_64::{
    __m128i, __m256i, _mm_loadu_si128, _mm_packus_epi16, _mm_storeu_si128, _mm256_add_epi16,
    _mm256_add_epi32, _mm256_add_epi64, _mm256_and_si256, _mm256_andnot_si256, _mm256_blend_epi32,
    _mm256_castsi256_si128, _mm256_cmpgt_epi16, _mm256_cmpgt_epi32, _mm256_cmpgt_epi64,
    _mm256_cvtepu8_epi16, _mm256_cvtepu16_epi32, _mm256_extracti128_si256, _mm256_loadu_si256,
    _mm256_mul_epu32, _mm256_mulhi_epu16, _mm256_mullo_epi16, _mm256_mullo_epi32, _mm256_or_si256,
    _mm256_packus_epi16, _mm256_packus_epi32, _mm256_permute4x64_epi64, _mm256_set1_epi16,
    _mm256_set1_epi32, _mm256_set1_epi64x, _mm256_slli_epi64, _mm256_srli_epi64,
    _mm256_storeu_si256, _mm256_sub_epi16, _mm256_sub_epi32, _mm256_sub_epi64, _mm256_xor_si256,
    _mm256_zeroupper, _mulx_u64,
};

#[cfg(all(test, feature = "std"))]
use core::arch::x86_64::_mm256_set_epi64x;

use crate::{
    __private::{
        VerifiedPrimeCanonical8Field, VerifiedPrimeCanonical16Field, VerifiedPrimeCanonical32Field,
        VerifiedPrimeMontgomery64Field,
    },
    Field, Fp251V1, Fp256GenericV1, FpGoldilocks64V1, KernelMetadata,
    kernel::{KernelSet, PackedKernelSet, PackedLaneKernels},
};

pub(crate) static FP251_AVX2_KERNELS: KernelSet<Fp251V1> = KernelSet::new(
    KernelMetadata::x86_prime_avx2(64).with_prime(crate::PrimeKernelMetadata::__from_generated(
        crate::PrimeRepresentationKind::CanonicalResidue,
        crate::PrimeReductionKind::Barrett,
        crate::RangeContract::__from_generated(1, 1, 16),
        crate::RangeContract::__from_generated(1, 1, 16),
        32,
        false,
    )),
    fp251_add,
    fp251_multiply,
    fp251_square,
    fp251_multiply_assign,
    fp251_square_assign,
)
.with_packed(PackedKernelSet::CanonicalU8(PackedLaneKernels::new(
    canonical8_pack::<Fp251V1>,
    canonical8_unpack::<Fp251V1>,
    packed8_add::<Fp251V1>,
    packed8_multiply::<Fp251V1>,
    packed8_square::<Fp251V1>,
    packed8_multiply_assign::<Fp251V1>,
    packed8_square_assign::<Fp251V1>,
)));

pub(crate) static FP_GOLDILOCKS_AVX2_KERNELS: KernelSet<FpGoldilocks64V1> = KernelSet::new(
    KernelMetadata::x86_prime_goldilocks_avx2::<FpGoldilocks64V1>().with_prime(
        crate::PrimeKernelMetadata::__from_generated(
            crate::PrimeRepresentationKind::CanonicalResidue,
            crate::PrimeReductionKind::Solinas,
            crate::RangeContract::__from_generated(1, 1, 128),
            crate::RangeContract::__from_generated(1, 1, 128),
            4,
            false,
        ),
    ),
    goldilocks_add,
    goldilocks_multiply,
    goldilocks_square,
    goldilocks_multiply_assign,
    goldilocks_square_assign,
);

pub(crate) static FP256_BMI2_KERNELS: KernelSet<Fp256GenericV1> =
    verified_radix64_kernel_set::<Fp256GenericV1, 4, 8>(
        crate::PrimeKernelMetadata::__from_generated(
            crate::PrimeRepresentationKind::Montgomery {
                radix_bits: 64,
                limbs: 4,
            },
            crate::PrimeReductionKind::Montgomery,
            crate::RangeContract::__from_generated(1, 1, 512),
            crate::RangeContract::__from_generated(1, 1, 512),
            1,
            false,
        ),
    );

/// Static factory shared by every generated radix-64 Montgomery field.
///
/// Compatibility is structural. Automatic promotion is deliberately kept in
/// `KernelMetadata`, where each concrete field must provide measured evidence.
pub(crate) const fn verified_radix64_kernel_set<F, const LIMBS: usize, const WIDE_LIMBS: usize>(
    prime: crate::PrimeKernelMetadata,
) -> KernelSet<F>
where
    F: VerifiedPrimeMontgomery64Field<LIMBS, WIDE_LIMBS>,
{
    assert!(LIMBS > 0);
    assert!(WIDE_LIMBS == LIMBS * 2);
    match prime.representation() {
        crate::PrimeRepresentationKind::Montgomery { radix_bits, limbs } => {
            assert!(radix_bits == 64);
            assert!(limbs as usize == LIMBS);
        }
        crate::PrimeRepresentationKind::CanonicalResidue => {
            panic!("BMI2 metadata must declare radix-64 Montgomery representation");
        }
    }
    assert!(matches!(
        prime.reduction(),
        crate::PrimeReductionKind::Montgomery
    ));
    assert!(F::__MODULUS[0] & 1 == 1);
    assert!(F::__MODULUS[0].wrapping_mul(F::__NEG_INV).wrapping_add(1) == 0);
    let metadata = KernelMetadata::x86_prime_bmi2_candidate::<F>(1).with_prime(prime);
    KernelSet::new(
        metadata,
        add_radix64::<F, LIMBS, WIDE_LIMBS>,
        bmi2_multiply::<F, LIMBS, WIDE_LIMBS>,
        bmi2_square::<F, LIMBS, WIDE_LIMBS>,
        bmi2_multiply_assign::<F, LIMBS, WIDE_LIMBS>,
        bmi2_square_assign::<F, LIMBS, WIDE_LIMBS>,
    )
}

fn add_radix64<F, const LIMBS: usize, const WIDE_LIMBS: usize>(out: &mut [F], lhs: &[F], rhs: &[F])
where
    F: VerifiedPrimeMontgomery64Field<LIMBS, WIDE_LIMBS>,
{
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
        let sum = crate::prime::add_mod(
            left.__into_montgomery_limbs(),
            right.__into_montgomery_limbs(),
            F::__MODULUS,
        );
        *output = F::__from_reduced_montgomery_limbs(sum);
    }
}

fn fp251_add(out: &mut [Fp251V1], lhs: &[Fp251V1], rhs: &[Fp251V1]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: EngineBuilder exposes this table only after AVX2 detection. All
    // loads/stores remain within equally sized slices and the vector reducer
    // produces canonical residues below 251.
    unsafe { fp251_binary_impl::<false>(out, lhs, rhs) };
}

fn fp251_multiply(out: &mut [Fp251V1], lhs: &[Fp251V1], rhs: &[Fp251V1]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: same preconditions and canonical-output proof as `fp251_add`.
    unsafe { fp251_binary_impl::<true>(out, lhs, rhs) };
}

fn fp251_square(out: &mut [Fp251V1], values: &[Fp251V1]) {
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: same AVX2 and slice bounds established by the selected engine.
    unsafe { fp251_binary_impl::<true>(out, values, values) };
}

fn fp251_multiply_assign(lhs: &mut [Fp251V1], rhs: &[Fp251V1]) {
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: each vector is loaded before the corresponding output store;
    // AVX2 support was established during engine construction.
    unsafe { fp251_assign_impl::<false>(lhs, rhs) };
}

fn fp251_square_assign(values: &mut [Fp251V1]) {
    // SAFETY: each vector is loaded before its in-place store and every output
    // lane is reduced to the canonical byte range.
    unsafe { fp251_assign_impl::<true>(values, &[]) };
}

#[target_feature(enable = "avx2")]
unsafe fn fp251_binary_impl<const MULTIPLY: bool>(
    out: &mut [Fp251V1],
    lhs: &[Fp251V1],
    rhs: &[Fp251V1],
) {
    let mut index = 0;
    while index + 32 <= out.len() {
        // SAFETY: the loop proves 32 available bytes in both inputs.
        let left_bytes = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(index).cast::<__m256i>()) };
        // SAFETY: identical bound proof for the right input.
        let right_bytes = unsafe { _mm256_loadu_si256(rhs.as_ptr().add(index).cast::<__m256i>()) };
        let left_low = _mm256_cvtepu8_epi16(_mm256_castsi256_si128(left_bytes));
        let left_high = _mm256_cvtepu8_epi16(_mm256_extracti128_si256::<1>(left_bytes));
        let right_low = _mm256_cvtepu8_epi16(_mm256_castsi256_si128(right_bytes));
        let right_high = _mm256_cvtepu8_epi16(_mm256_extracti128_si256::<1>(right_bytes));
        let (wide_low, wide_high) = if MULTIPLY {
            (
                _mm256_mullo_epi16(left_low, right_low),
                _mm256_mullo_epi16(left_high, right_high),
            )
        } else {
            (
                _mm256_add_epi16(left_low, right_low),
                _mm256_add_epi16(left_high, right_high),
            )
        };
        let packed = pack_fp251_lanes(reduce_fp251_lanes(wide_low), reduce_fp251_lanes(wide_high));
        // SAFETY: the loop proves 32 writable bytes and all lanes are valid.
        unsafe { _mm256_storeu_si256(out.as_mut_ptr().add(index).cast::<__m256i>(), packed) };
        index += 32;
    }
    while index + 16 <= out.len() {
        // SAFETY: the loop proves 16 available bytes in every slice. Prime
        // fields are transparent one-byte values.
        let left = unsafe { _mm_loadu_si128(lhs.as_ptr().add(index).cast::<__m128i>()) };
        // SAFETY: identical bound proof for the right operand.
        let right = unsafe { _mm_loadu_si128(rhs.as_ptr().add(index).cast::<__m128i>()) };
        let left = _mm256_cvtepu8_epi16(left);
        let right = _mm256_cvtepu8_epi16(right);
        let wide = if MULTIPLY {
            _mm256_mullo_epi16(left, right)
        } else {
            _mm256_add_epi16(left, right)
        };
        let reduced = reduce_fp251_lanes(wide);
        let packed = _mm_packus_epi16(
            _mm256_castsi256_si128(reduced),
            _mm256_extracti128_si256::<1>(reduced),
        );
        // SAFETY: the loop proves 16 writable output bytes. Every packed lane
        // is a valid `Fp251V1` representation.
        unsafe {
            _mm_storeu_si128(out.as_mut_ptr().add(index).cast::<__m128i>(), packed);
        }
        index += 16;
    }
    _mm256_zeroupper();
    for tail in index..out.len() {
        out[tail] = if MULTIPLY {
            lhs[tail].mul(rhs[tail])
        } else {
            lhs[tail].add(rhs[tail])
        };
    }
}

#[target_feature(enable = "avx2")]
unsafe fn fp251_assign_impl<const SQUARE: bool>(lhs: &mut [Fp251V1], rhs: &[Fp251V1]) {
    let mut index = 0;
    while index + 32 <= lhs.len() {
        // SAFETY: the loop proves a complete 32-byte left chunk.
        let left_bytes = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(index).cast::<__m256i>()) };
        let right_bytes = if SQUARE {
            left_bytes
        } else {
            // SAFETY: multiplication established equal slice lengths.
            unsafe { _mm256_loadu_si256(rhs.as_ptr().add(index).cast::<__m256i>()) }
        };
        let low = _mm256_mullo_epi16(
            _mm256_cvtepu8_epi16(_mm256_castsi256_si128(left_bytes)),
            _mm256_cvtepu8_epi16(_mm256_castsi256_si128(right_bytes)),
        );
        let high = _mm256_mullo_epi16(
            _mm256_cvtepu8_epi16(_mm256_extracti128_si256::<1>(left_bytes)),
            _mm256_cvtepu8_epi16(_mm256_extracti128_si256::<1>(right_bytes)),
        );
        let packed = pack_fp251_lanes(reduce_fp251_lanes(low), reduce_fp251_lanes(high));
        // SAFETY: the output chunk is in bounds and contains valid residues.
        unsafe { _mm256_storeu_si256(lhs.as_mut_ptr().add(index).cast::<__m256i>(), packed) };
        index += 32;
    }
    while index + 16 <= lhs.len() {
        // SAFETY: the loop proves a complete 16-byte left chunk.
        let left_bytes = unsafe { _mm_loadu_si128(lhs.as_ptr().add(index).cast::<__m128i>()) };
        let right_bytes = if SQUARE {
            left_bytes
        } else {
            // SAFETY: caller established equal lengths for multiplication.
            unsafe { _mm_loadu_si128(rhs.as_ptr().add(index).cast::<__m128i>()) }
        };
        let wide = _mm256_mullo_epi16(
            _mm256_cvtepu8_epi16(left_bytes),
            _mm256_cvtepu8_epi16(right_bytes),
        );
        let reduced = reduce_fp251_lanes(wide);
        let packed = _mm_packus_epi16(
            _mm256_castsi256_si128(reduced),
            _mm256_extracti128_si256::<1>(reduced),
        );
        // SAFETY: the output chunk is in bounds and contains valid residues.
        unsafe {
            _mm_storeu_si128(lhs.as_mut_ptr().add(index).cast::<__m128i>(), packed);
        }
        index += 16;
    }
    _mm256_zeroupper();
    for tail in index..lhs.len() {
        lhs[tail] = if SQUARE {
            lhs[tail].mul(lhs[tail])
        } else {
            lhs[tail].mul(rhs[tail])
        };
    }
}

#[target_feature(enable = "avx2")]
fn reduce_fp251_lanes(value: __m256i) -> __m256i {
    let modulus = _mm256_set1_epi16(251);
    let threshold = _mm256_set1_epi16(250);
    let quotient = _mm256_mulhi_epu16(value, _mm256_set1_epi16(261));
    let residue = _mm256_sub_epi16(value, _mm256_mullo_epi16(quotient, modulus));
    let subtract_mask = _mm256_cmpgt_epi16(residue, threshold);
    _mm256_sub_epi16(residue, _mm256_and_si256(subtract_mask, modulus))
}

#[target_feature(enable = "avx2")]
fn pack_fp251_lanes(low: __m256i, high: __m256i) -> __m256i {
    let interleaved = _mm256_packus_epi16(low, high);
    _mm256_permute4x64_epi64::<0xd8>(interleaved)
}

fn goldilocks_add(
    out: &mut [FpGoldilocks64V1],
    lhs: &[FpGoldilocks64V1],
    rhs: &[FpGoldilocks64V1],
) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: EngineBuilder selects this table only after AVX2 detection; the
    // maintained field is a transparent `u64` and every result is canonical.
    unsafe { goldilocks_binary_impl::<false>(out, lhs, rhs) };
}

fn goldilocks_multiply(
    out: &mut [FpGoldilocks64V1],
    lhs: &[FpGoldilocks64V1],
    rhs: &[FpGoldilocks64V1],
) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: same target-feature, layout and canonical-output proof as add.
    unsafe { goldilocks_binary_impl::<true>(out, lhs, rhs) };
}

fn goldilocks_square(out: &mut [FpGoldilocks64V1], values: &[FpGoldilocks64V1]) {
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: same proof as multiplication with identical inputs.
    unsafe { goldilocks_binary_impl::<true>(out, values, values) };
}

fn goldilocks_multiply_assign(lhs: &mut [FpGoldilocks64V1], rhs: &[FpGoldilocks64V1]) {
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: every vector is loaded before its corresponding output store.
    unsafe { goldilocks_assign_impl(lhs, rhs, false) };
}

fn goldilocks_square_assign(values: &mut [FpGoldilocks64V1]) {
    // SAFETY: every vector is loaded before its corresponding output store.
    unsafe { goldilocks_assign_impl(values, &[], true) };
}

#[target_feature(enable = "avx2")]
unsafe fn goldilocks_binary_impl<const MULTIPLY: bool>(
    out: &mut [FpGoldilocks64V1],
    lhs: &[FpGoldilocks64V1],
    rhs: &[FpGoldilocks64V1],
) {
    let mut index = 0;
    while index + 4 <= out.len() {
        // SAFETY: the loop proves four readable transparent-u64 elements.
        let left = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(index).cast::<__m256i>()) };
        // SAFETY: identical bound proof for the right input.
        let right = unsafe { _mm256_loadu_si256(rhs.as_ptr().add(index).cast::<__m256i>()) };
        let reduced = if MULTIPLY {
            goldilocks_multiply_lanes(left, right)
        } else {
            goldilocks_add_lanes(left, right)
        };
        // SAFETY: the loop proves four writable elements and reduction makes
        // every stored `u64` a canonical `FpGoldilocks64V1` representation.
        unsafe { _mm256_storeu_si256(out.as_mut_ptr().add(index).cast::<__m256i>(), reduced) };
        index += 4;
    }
    _mm256_zeroupper();
    for tail in index..out.len() {
        out[tail] = if MULTIPLY {
            lhs[tail].mul(rhs[tail])
        } else {
            lhs[tail].add(rhs[tail])
        };
    }
}

#[target_feature(enable = "avx2")]
unsafe fn goldilocks_assign_impl(
    lhs: &mut [FpGoldilocks64V1],
    rhs: &[FpGoldilocks64V1],
    square: bool,
) {
    let mut index = 0;
    while index + 4 <= lhs.len() {
        // SAFETY: the loop proves four readable left elements.
        let left = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(index).cast::<__m256i>()) };
        let right = if square {
            left
        } else {
            // SAFETY: multiplication established equal slice lengths.
            unsafe { _mm256_loadu_si256(rhs.as_ptr().add(index).cast::<__m256i>()) }
        };
        let reduced = goldilocks_multiply_lanes(left, right);
        // SAFETY: the vector was loaded before this in-place canonical store.
        unsafe { _mm256_storeu_si256(lhs.as_mut_ptr().add(index).cast::<__m256i>(), reduced) };
        index += 4;
    }
    _mm256_zeroupper();
    for tail in index..lhs.len() {
        lhs[tail] = if square {
            lhs[tail].mul(lhs[tail])
        } else {
            lhs[tail].mul(rhs[tail])
        };
    }
}

#[target_feature(enable = "avx2")]
fn goldilocks_add_lanes(lhs: __m256i, rhs: __m256i) -> __m256i {
    const EPSILON: i64 = 0xffff_ffff;
    let sum = _mm256_add_epi64(lhs, rhs);
    let carry = mask_to_one(unsigned_lt_lanes(sum, lhs));
    let folded = _mm256_add_epi64(sum, _mm256_mul_epu32(carry, _mm256_set1_epi64x(EPSILON)));
    canonicalize_goldilocks(folded)
}

#[target_feature(enable = "avx2")]
fn goldilocks_multiply_lanes(lhs: __m256i, rhs: __m256i) -> __m256i {
    let lhs_high = _mm256_srli_epi64::<32>(lhs);
    let rhs_high = _mm256_srli_epi64::<32>(rhs);
    let low_low = _mm256_mul_epu32(lhs, rhs);
    let low_high = _mm256_mul_epu32(lhs, rhs_high);
    let high_low = _mm256_mul_epu32(lhs_high, rhs);
    let high_high = _mm256_mul_epu32(lhs_high, rhs_high);

    let middle = _mm256_add_epi64(low_high, high_low);
    let middle_carry = mask_to_one(unsigned_lt_lanes(middle, low_high));
    let low = _mm256_add_epi64(low_low, _mm256_slli_epi64::<32>(middle));
    let low_carry = mask_to_one(unsigned_lt_lanes(low, low_low));
    let high = _mm256_add_epi64(
        _mm256_add_epi64(high_high, _mm256_srli_epi64::<32>(middle)),
        _mm256_add_epi64(_mm256_slli_epi64::<32>(middle_carry), low_carry),
    );

    let (low, high) = fold_goldilocks_wide(low, high);
    let (low, high) = fold_goldilocks_wide(low, high);
    let (low, high) = fold_goldilocks_wide(low, high);
    let (low, _high) = fold_goldilocks_wide(low, high);
    canonicalize_goldilocks(low)
}

#[target_feature(enable = "avx2")]
fn fold_goldilocks_wide(low: __m256i, high: __m256i) -> (__m256i, __m256i) {
    // For p = 2^64 - 2^32 + 1, each high word contributes
    // high * (2^32 - 1). This is one fixed, branchless 128-bit fold.
    let shifted = _mm256_slli_epi64::<32>(high);
    let product_low = _mm256_sub_epi64(shifted, high);
    let borrow = mask_to_one(unsigned_lt_lanes(shifted, high));
    let product_high = _mm256_sub_epi64(_mm256_srli_epi64::<32>(high), borrow);
    let sum = _mm256_add_epi64(low, product_low);
    let carry = mask_to_one(unsigned_lt_lanes(sum, low));
    (sum, _mm256_add_epi64(product_high, carry))
}

#[target_feature(enable = "avx2")]
fn canonicalize_goldilocks(value: __m256i) -> __m256i {
    let modulus = _mm256_set1_epi64x(0xffff_ffff_0000_0001_u64.cast_signed());
    let below = unsigned_lt_lanes(value, modulus);
    _mm256_sub_epi64(value, _mm256_andnot_si256(below, modulus))
}

#[target_feature(enable = "avx2")]
fn unsigned_lt_lanes(lhs: __m256i, rhs: __m256i) -> __m256i {
    let sign = _mm256_set1_epi64x(i64::MIN);
    _mm256_cmpgt_epi64(_mm256_xor_si256(rhs, sign), _mm256_xor_si256(lhs, sign))
}

#[target_feature(enable = "avx2")]
fn mask_to_one(mask: __m256i) -> __m256i {
    _mm256_and_si256(mask, _mm256_set1_epi64x(1))
}

/// Static AVX2 factory for every generator-certified canonical-byte prime.
pub(crate) const fn verified_canonical8_kernel_set<F>(
    prime: crate::PrimeKernelMetadata,
) -> KernelSet<F>
where
    F: VerifiedPrimeCanonical8Field,
{
    KernelSet::new(
        KernelMetadata::x86_prime_avx2_candidate(64, 32).with_prime(prime),
        canonical8_add::<F>,
        canonical8_multiply::<F>,
        canonical8_square::<F>,
        canonical8_multiply_assign::<F>,
        canonical8_square_assign::<F>,
    )
    .with_packed(PackedKernelSet::CanonicalU8(PackedLaneKernels::new(
        canonical8_pack::<F>,
        canonical8_unpack::<F>,
        packed8_add::<F>,
        packed8_multiply::<F>,
        packed8_square::<F>,
        packed8_multiply_assign::<F>,
        packed8_square_assign::<F>,
    )))
}

/// Static AVX2 factory for every generator-certified canonical-`u16` prime.
pub(crate) const fn verified_canonical16_kernel_set<F>(
    prime: crate::PrimeKernelMetadata,
) -> KernelSet<F>
where
    F: VerifiedPrimeCanonical16Field,
{
    KernelSet::new(
        KernelMetadata::x86_prime_avx2_candidate(64, 16).with_prime(prime),
        canonical16_add::<F>,
        canonical16_multiply::<F>,
        canonical16_square::<F>,
        canonical16_multiply_assign::<F>,
        canonical16_square_assign::<F>,
    )
    .with_packed(PackedKernelSet::CanonicalU16(PackedLaneKernels::new(
        canonical16_pack::<F>,
        canonical16_unpack::<F>,
        packed16_add::<F>,
        packed16_multiply::<F>,
        packed16_square::<F>,
        packed16_multiply_assign::<F>,
        packed16_square_assign::<F>,
    )))
}

/// Static AVX2 candidate for every certified canonical-`u32` prime.
pub(crate) const fn verified_canonical32_kernel_set<F>(
    prime: crate::PrimeKernelMetadata,
) -> KernelSet<F>
where
    F: VerifiedPrimeCanonical32Field,
{
    KernelSet::new(
        KernelMetadata::x86_prime_avx2_candidate(64, 8).with_prime(prime),
        canonical32_add::<F>,
        canonical32_multiply::<F>,
        canonical32_square::<F>,
        canonical32_multiply_assign::<F>,
        canonical32_square_assign::<F>,
    )
    .with_packed(PackedKernelSet::CanonicalU32(PackedLaneKernels::new(
        canonical32_pack::<F>,
        canonical32_unpack::<F>,
        packed32_add::<F>,
        packed32_multiply::<F>,
        packed32_square::<F>,
        packed32_multiply_assign::<F>,
        packed32_square_assign::<F>,
    )))
}

fn canonical8_pack<F: VerifiedPrimeCanonical8Field>(out: &mut [u8], values: &[F]) {
    debug_assert!(out.len() >= values.len());
    out.fill(0);
    for (output, value) in out.iter_mut().zip(values) {
        *output = value.__into_canonical_u8();
    }
}

fn canonical8_unpack<F: VerifiedPrimeCanonical8Field>(out: &mut [F], values: &[u8]) {
    debug_assert_eq!(out.len(), values.len());
    for (output, value) in out.iter_mut().zip(values.iter().copied()) {
        *output = F::__from_reduced_canonical_u8(value);
    }
}

fn canonical16_pack<F: VerifiedPrimeCanonical16Field>(out: &mut [u16], values: &[F]) {
    debug_assert!(out.len() >= values.len());
    out.fill(0);
    for (output, value) in out.iter_mut().zip(values) {
        *output = value.__into_canonical_u16();
    }
}

fn canonical16_unpack<F: VerifiedPrimeCanonical16Field>(out: &mut [F], values: &[u16]) {
    debug_assert_eq!(out.len(), values.len());
    for (output, value) in out.iter_mut().zip(values.iter().copied()) {
        *output = F::__from_reduced_canonical_u16(value);
    }
}

fn canonical32_pack<F: VerifiedPrimeCanonical32Field>(out: &mut [u32], values: &[F]) {
    debug_assert!(out.len() >= values.len());
    out.fill(0);
    for (output, value) in out.iter_mut().zip(values) {
        *output = value.__into_canonical_u32();
    }
}

fn canonical32_unpack<F: VerifiedPrimeCanonical32Field>(out: &mut [F], values: &[u32]) {
    debug_assert_eq!(out.len(), values.len());
    for (output, value) in out.iter_mut().zip(values.iter().copied()) {
        *output = F::__from_reduced_canonical_u32(value);
    }
}

fn packed8_add<F: VerifiedPrimeCanonical8Field>(out: &mut [u8], lhs: &[u8], rhs: &[u8]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: the engine exposes this strategy only after AVX2 detection; all
    // slices contain canonical lanes created by the certified codec.
    unsafe { packed8_binary_impl::<F, false>(out, lhs, rhs) };
}

fn packed8_multiply<F: VerifiedPrimeCanonical8Field>(out: &mut [u8], lhs: &[u8], rhs: &[u8]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: same target-feature and canonical-lane proof as packed addition.
    unsafe { packed8_binary_impl::<F, true>(out, lhs, rhs) };
}

fn packed8_square<F: VerifiedPrimeCanonical8Field>(out: &mut [u8], values: &[u8]) {
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: same proof as multiplication with identical operands.
    unsafe { packed8_binary_impl::<F, true>(out, values, values) };
}

fn packed8_multiply_assign<F: VerifiedPrimeCanonical8Field>(lhs: &mut [u8], rhs: &[u8]) {
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: every input vector is loaded before its output store.
    unsafe { packed8_assign_impl::<F, false>(lhs, rhs) };
}

fn packed8_square_assign<F: VerifiedPrimeCanonical8Field>(values: &mut [u8]) {
    // SAFETY: every input vector is loaded before its output store.
    unsafe { packed8_assign_impl::<F, true>(values, &[]) };
}

fn packed16_add<F: VerifiedPrimeCanonical16Field>(out: &mut [u16], lhs: &[u16], rhs: &[u16]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: the engine exposes this strategy only after AVX2 detection.
    unsafe { packed16_binary_impl::<F, false>(out, lhs, rhs) };
}

fn packed16_multiply<F: VerifiedPrimeCanonical16Field>(out: &mut [u16], lhs: &[u16], rhs: &[u16]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: same target-feature and canonical-lane proof as packed addition.
    unsafe { packed16_binary_impl::<F, true>(out, lhs, rhs) };
}

fn packed16_square<F: VerifiedPrimeCanonical16Field>(out: &mut [u16], values: &[u16]) {
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: same proof as multiplication with identical operands.
    unsafe { packed16_binary_impl::<F, true>(out, values, values) };
}

fn packed16_multiply_assign<F: VerifiedPrimeCanonical16Field>(lhs: &mut [u16], rhs: &[u16]) {
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: every input vector is loaded before its output store.
    unsafe { packed16_assign_impl::<F, false>(lhs, rhs) };
}

fn packed16_square_assign<F: VerifiedPrimeCanonical16Field>(values: &mut [u16]) {
    // SAFETY: every input vector is loaded before its output store.
    unsafe { packed16_assign_impl::<F, true>(values, &[]) };
}

fn packed32_add<F: VerifiedPrimeCanonical32Field>(out: &mut [u32], lhs: &[u32], rhs: &[u32]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: the engine exposes this strategy only after AVX2 detection.
    unsafe { packed32_binary_impl::<F, false>(out, lhs, rhs) };
}

fn packed32_multiply<F: VerifiedPrimeCanonical32Field>(out: &mut [u32], lhs: &[u32], rhs: &[u32]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: all lanes are canonical and every product fits in `u64`.
    unsafe { packed32_binary_impl::<F, true>(out, lhs, rhs) };
}

fn packed32_square<F: VerifiedPrimeCanonical32Field>(out: &mut [u32], values: &[u32]) {
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: same proof as multiplication with identical operands.
    unsafe { packed32_binary_impl::<F, true>(out, values, values) };
}

fn packed32_multiply_assign<F: VerifiedPrimeCanonical32Field>(lhs: &mut [u32], rhs: &[u32]) {
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: every input vector is loaded before its output store.
    unsafe { packed32_assign_impl::<F, false>(lhs, rhs) };
}

fn packed32_square_assign<F: VerifiedPrimeCanonical32Field>(values: &mut [u32]) {
    // SAFETY: every input vector is loaded before its output store.
    unsafe { packed32_assign_impl::<F, true>(values, &[]) };
}

#[target_feature(enable = "avx2")]
unsafe fn packed8_binary_impl<F: VerifiedPrimeCanonical8Field, const MULTIPLY: bool>(
    out: &mut [u8],
    lhs: &[u8],
    rhs: &[u8],
) {
    let mut index = 0;
    while index + 32 <= out.len() {
        // SAFETY: the loop proves a complete YMM tile in every slice.
        let left = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(index).cast::<__m256i>()) };
        // SAFETY: same bounds proof for the right input.
        let right = unsafe { _mm256_loadu_si256(rhs.as_ptr().add(index).cast::<__m256i>()) };
        let reduced = simd8_binary::<MULTIPLY>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        // SAFETY: the output has a complete writable YMM tile.
        unsafe { _mm256_storeu_si256(out.as_mut_ptr().add(index).cast::<__m256i>(), reduced) };
        index += 32;
    }
    _mm256_zeroupper();
    for tail in index..out.len() {
        let value = if MULTIPLY {
            u16::from(lhs[tail]) * u16::from(rhs[tail])
        } else {
            u16::from(lhs[tail]) + u16::from(rhs[tail])
        };
        out[tail] = (value % F::__MODULUS) as u8;
    }
}

#[target_feature(enable = "avx2")]
unsafe fn packed8_assign_impl<F: VerifiedPrimeCanonical8Field, const SQUARE: bool>(
    lhs: &mut [u8],
    rhs: &[u8],
) {
    let mut index = 0;
    while index + 32 <= lhs.len() {
        // SAFETY: the loop proves a complete writable/readable YMM tile.
        let left = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(index).cast::<__m256i>()) };
        let right = if SQUARE {
            left
        } else {
            // SAFETY: multiplication established equal slice lengths.
            unsafe { _mm256_loadu_si256(rhs.as_ptr().add(index).cast::<__m256i>()) }
        };
        let reduced = simd8_binary::<true>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        // SAFETY: the same tile is writable after both inputs were loaded.
        unsafe { _mm256_storeu_si256(lhs.as_mut_ptr().add(index).cast::<__m256i>(), reduced) };
        index += 32;
    }
    _mm256_zeroupper();
    for tail in index..lhs.len() {
        let right = if SQUARE { lhs[tail] } else { rhs[tail] };
        lhs[tail] = (u16::from(lhs[tail]) * u16::from(right) % F::__MODULUS) as u8;
    }
}

#[target_feature(enable = "avx2")]
unsafe fn packed16_binary_impl<F: VerifiedPrimeCanonical16Field, const MULTIPLY: bool>(
    out: &mut [u16],
    lhs: &[u16],
    rhs: &[u16],
) {
    let mut index = 0;
    while index + 16 <= out.len() {
        // SAFETY: the loop proves one complete YMM tile in every slice.
        let left = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(index).cast::<__m256i>()) };
        // SAFETY: same bounds proof for the right input.
        let right = unsafe { _mm256_loadu_si256(rhs.as_ptr().add(index).cast::<__m256i>()) };
        let reduced = simd16_binary::<MULTIPLY>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        // SAFETY: the output has one complete writable YMM tile.
        unsafe { _mm256_storeu_si256(out.as_mut_ptr().add(index).cast::<__m256i>(), reduced) };
        index += 16;
    }
    _mm256_zeroupper();
    for tail in index..out.len() {
        let value = if MULTIPLY {
            u64::from(lhs[tail]) * u64::from(rhs[tail])
        } else {
            u64::from(lhs[tail]) + u64::from(rhs[tail])
        };
        out[tail] = (value % u64::from(F::__MODULUS)) as u16;
    }
}

#[target_feature(enable = "avx2")]
unsafe fn packed16_assign_impl<F: VerifiedPrimeCanonical16Field, const SQUARE: bool>(
    lhs: &mut [u16],
    rhs: &[u16],
) {
    let mut index = 0;
    while index + 16 <= lhs.len() {
        // SAFETY: the loop proves one complete writable/readable YMM tile.
        let left = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(index).cast::<__m256i>()) };
        let right = if SQUARE {
            left
        } else {
            // SAFETY: multiplication established equal slice lengths.
            unsafe { _mm256_loadu_si256(rhs.as_ptr().add(index).cast::<__m256i>()) }
        };
        let reduced = simd16_binary::<true>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        // SAFETY: the same tile is writable after both inputs were loaded.
        unsafe { _mm256_storeu_si256(lhs.as_mut_ptr().add(index).cast::<__m256i>(), reduced) };
        index += 16;
    }
    _mm256_zeroupper();
    for tail in index..lhs.len() {
        let right = if SQUARE { lhs[tail] } else { rhs[tail] };
        lhs[tail] = (u64::from(lhs[tail]) * u64::from(right) % u64::from(F::__MODULUS)) as u16;
    }
}

#[target_feature(enable = "avx2")]
unsafe fn packed32_binary_impl<F: VerifiedPrimeCanonical32Field, const MULTIPLY: bool>(
    out: &mut [u32],
    lhs: &[u32],
    rhs: &[u32],
) {
    let mut index = 0;
    while index + 8 <= out.len() {
        // SAFETY: the loop proves one complete YMM tile in every slice.
        let left = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(index).cast::<__m256i>()) };
        // SAFETY: same bounds proof for the right input.
        let right = unsafe { _mm256_loadu_si256(rhs.as_ptr().add(index).cast::<__m256i>()) };
        let reduced = simd32_binary::<MULTIPLY>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        // SAFETY: the output has one complete writable YMM tile.
        unsafe { _mm256_storeu_si256(out.as_mut_ptr().add(index).cast::<__m256i>(), reduced) };
        index += 8;
    }
    _mm256_zeroupper();
    for tail in index..out.len() {
        let value = if MULTIPLY {
            u64::from(lhs[tail]) * u64::from(rhs[tail])
        } else {
            u64::from(lhs[tail]) + u64::from(rhs[tail])
        };
        out[tail] = (u128::from(value) % u128::from(F::__MODULUS)) as u32;
    }
}

#[target_feature(enable = "avx2")]
unsafe fn packed32_assign_impl<F: VerifiedPrimeCanonical32Field, const SQUARE: bool>(
    lhs: &mut [u32],
    rhs: &[u32],
) {
    let mut index = 0;
    while index + 8 <= lhs.len() {
        // SAFETY: the loop proves one complete readable/writable YMM tile.
        let left = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(index).cast::<__m256i>()) };
        let right = if SQUARE {
            left
        } else {
            // SAFETY: multiplication established equal slice lengths.
            unsafe { _mm256_loadu_si256(rhs.as_ptr().add(index).cast::<__m256i>()) }
        };
        let reduced = simd32_binary::<true>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        // SAFETY: the same tile is writable after both inputs were loaded.
        unsafe { _mm256_storeu_si256(lhs.as_mut_ptr().add(index).cast::<__m256i>(), reduced) };
        index += 8;
    }
    _mm256_zeroupper();
    for tail in index..lhs.len() {
        let right = if SQUARE { lhs[tail] } else { rhs[tail] };
        lhs[tail] = (u128::from(lhs[tail]) * u128::from(right) % u128::from(F::__MODULUS)) as u32;
    }
}

fn canonical8_add<F: VerifiedPrimeCanonical8Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: this table is selectable only after AVX2 capability detection.
    unsafe { canonical8_binary_impl::<F, false>(out, lhs, rhs) };
}

fn canonical8_multiply<F: VerifiedPrimeCanonical8Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: this table is selectable only after AVX2 capability detection.
    unsafe { canonical8_binary_impl::<F, true>(out, lhs, rhs) };
}

fn canonical8_square<F: VerifiedPrimeCanonical8Field>(out: &mut [F], values: &[F]) {
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: this table is selectable only after AVX2 capability detection.
    unsafe { canonical8_binary_impl::<F, true>(out, values, values) };
}

fn canonical8_multiply_assign<F: VerifiedPrimeCanonical8Field>(lhs: &mut [F], rhs: &[F]) {
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: every tile is copied before its in-place replacement.
    unsafe { canonical8_assign_impl::<F, false>(lhs, rhs) };
}

fn canonical8_square_assign<F: VerifiedPrimeCanonical8Field>(values: &mut [F]) {
    // SAFETY: every tile is copied before its in-place replacement.
    unsafe { canonical8_assign_impl::<F, true>(values, &[]) };
}

fn canonical16_add<F: VerifiedPrimeCanonical16Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: this table is selectable only after AVX2 capability detection.
    unsafe { canonical16_binary_impl::<F, false>(out, lhs, rhs) };
}

fn canonical16_multiply<F: VerifiedPrimeCanonical16Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: this table is selectable only after AVX2 capability detection.
    unsafe { canonical16_binary_impl::<F, true>(out, lhs, rhs) };
}

fn canonical16_square<F: VerifiedPrimeCanonical16Field>(out: &mut [F], values: &[F]) {
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: this table is selectable only after AVX2 capability detection.
    unsafe { canonical16_binary_impl::<F, true>(out, values, values) };
}

fn canonical16_multiply_assign<F: VerifiedPrimeCanonical16Field>(lhs: &mut [F], rhs: &[F]) {
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: every tile is copied before its in-place replacement.
    unsafe { canonical16_assign_impl::<F, false>(lhs, rhs) };
}

fn canonical16_square_assign<F: VerifiedPrimeCanonical16Field>(values: &mut [F]) {
    // SAFETY: every tile is copied before its in-place replacement.
    unsafe { canonical16_assign_impl::<F, true>(values, &[]) };
}

fn canonical32_add<F: VerifiedPrimeCanonical32Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: this table is selectable only after AVX2 capability detection.
    unsafe { canonical32_binary_impl::<F, false>(out, lhs, rhs) };
}

fn canonical32_multiply<F: VerifiedPrimeCanonical32Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: this table is selectable only after AVX2 capability detection.
    unsafe { canonical32_binary_impl::<F, true>(out, lhs, rhs) };
}

fn canonical32_square<F: VerifiedPrimeCanonical32Field>(out: &mut [F], values: &[F]) {
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: same proof as multiplication with identical inputs.
    unsafe { canonical32_binary_impl::<F, true>(out, values, values) };
}

fn canonical32_multiply_assign<F: VerifiedPrimeCanonical32Field>(lhs: &mut [F], rhs: &[F]) {
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: every local tile is copied before its in-place replacement.
    unsafe { canonical32_assign_impl::<F, false>(lhs, rhs) };
}

fn canonical32_square_assign<F: VerifiedPrimeCanonical32Field>(values: &mut [F]) {
    // SAFETY: every local tile is copied before its in-place replacement.
    unsafe { canonical32_assign_impl::<F, true>(values, &[]) };
}

#[target_feature(enable = "avx2")]
unsafe fn canonical8_binary_impl<F: VerifiedPrimeCanonical8Field, const MULTIPLY: bool>(
    out: &mut [F],
    lhs: &[F],
    rhs: &[F],
) {
    let mut index = 0;
    while index + 32 <= out.len() {
        let mut left = [0_u8; 32];
        let mut right = [0_u8; 32];
        for lane in 0..32 {
            left[lane] = lhs[index + lane].__into_canonical_u8();
            right[lane] = rhs[index + lane].__into_canonical_u8();
        }
        // SAFETY: both local input tiles provide exactly 32 readable bytes.
        let left = unsafe { _mm256_loadu_si256(left.as_ptr().cast::<__m256i>()) };
        let right = unsafe { _mm256_loadu_si256(right.as_ptr().cast::<__m256i>()) };
        let reduced = simd8_binary::<MULTIPLY>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        let mut tile = [0_u8; 32];
        // SAFETY: the local output tile provides exactly 32 writable bytes.
        unsafe { _mm256_storeu_si256(tile.as_mut_ptr().cast::<__m256i>(), reduced) };
        for lane in 0..32 {
            out[index + lane] = F::__from_reduced_canonical_u8(tile[lane]);
        }
        index += 32;
    }
    _mm256_zeroupper();
    for tail in index..out.len() {
        out[tail] = if MULTIPLY {
            lhs[tail].mul(rhs[tail])
        } else {
            lhs[tail].add(rhs[tail])
        };
    }
}

#[target_feature(enable = "avx2")]
unsafe fn canonical8_assign_impl<F: VerifiedPrimeCanonical8Field, const SQUARE: bool>(
    lhs: &mut [F],
    rhs: &[F],
) {
    let mut index = 0;
    while index + 32 <= lhs.len() {
        let mut left = [0_u8; 32];
        let mut right = [0_u8; 32];
        for lane in 0..32 {
            left[lane] = lhs[index + lane].__into_canonical_u8();
            right[lane] = if SQUARE {
                left[lane]
            } else {
                rhs[index + lane].__into_canonical_u8()
            };
        }
        // SAFETY: both local input tiles provide exactly 32 readable bytes.
        let left = unsafe { _mm256_loadu_si256(left.as_ptr().cast::<__m256i>()) };
        let right = unsafe { _mm256_loadu_si256(right.as_ptr().cast::<__m256i>()) };
        let reduced = simd8_binary::<true>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        let mut tile = [0_u8; 32];
        // SAFETY: the local output tile provides exactly 32 writable bytes.
        unsafe { _mm256_storeu_si256(tile.as_mut_ptr().cast::<__m256i>(), reduced) };
        for lane in 0..32 {
            lhs[index + lane] = F::__from_reduced_canonical_u8(tile[lane]);
        }
        index += 32;
    }
    _mm256_zeroupper();
    for tail in index..lhs.len() {
        lhs[tail] = if SQUARE {
            lhs[tail].mul(lhs[tail])
        } else {
            lhs[tail].mul(rhs[tail])
        };
    }
}

#[target_feature(enable = "avx2")]
fn simd8_binary<const MULTIPLY: bool>(
    left: __m256i,
    right: __m256i,
    modulus: u16,
    reciprocal: u16,
) -> __m256i {
    let left_low = _mm256_cvtepu8_epi16(_mm256_castsi256_si128(left));
    let left_high = _mm256_cvtepu8_epi16(_mm256_extracti128_si256::<1>(left));
    let right_low = _mm256_cvtepu8_epi16(_mm256_castsi256_si128(right));
    let right_high = _mm256_cvtepu8_epi16(_mm256_extracti128_si256::<1>(right));
    let (wide_low, wide_high) = if MULTIPLY {
        (
            _mm256_mullo_epi16(left_low, right_low),
            _mm256_mullo_epi16(left_high, right_high),
        )
    } else {
        (
            _mm256_add_epi16(left_low, right_low),
            _mm256_add_epi16(left_high, right_high),
        )
    };
    let low = reduce_u16_lanes(wide_low, modulus, reciprocal);
    let high = reduce_u16_lanes(wide_high, modulus, reciprocal);
    _mm256_permute4x64_epi64::<0xd8>(_mm256_packus_epi16(low, high))
}

#[target_feature(enable = "avx2")]
fn reduce_u16_lanes(value: __m256i, modulus: u16, reciprocal: u16) -> __m256i {
    let threshold = _mm256_set1_epi16((modulus - 1).cast_signed());
    let modulus = _mm256_set1_epi16(modulus.cast_signed());
    let quotient = _mm256_mulhi_epu16(value, _mm256_set1_epi16(reciprocal.cast_signed()));
    let residue = _mm256_sub_epi16(value, _mm256_mullo_epi16(quotient, modulus));
    let subtract_mask = _mm256_cmpgt_epi16(residue, threshold);
    _mm256_sub_epi16(residue, _mm256_and_si256(subtract_mask, modulus))
}

#[target_feature(enable = "avx2")]
unsafe fn canonical16_binary_impl<F: VerifiedPrimeCanonical16Field, const MULTIPLY: bool>(
    out: &mut [F],
    lhs: &[F],
    rhs: &[F],
) {
    let mut index = 0;
    while index + 16 <= out.len() {
        let mut left = [0_u16; 16];
        let mut right = [0_u16; 16];
        for lane in 0..16 {
            left[lane] = lhs[index + lane].__into_canonical_u16();
            right[lane] = rhs[index + lane].__into_canonical_u16();
        }
        // SAFETY: both local input tiles provide exactly 32 readable bytes.
        let left = unsafe { _mm256_loadu_si256(left.as_ptr().cast::<__m256i>()) };
        let right = unsafe { _mm256_loadu_si256(right.as_ptr().cast::<__m256i>()) };
        let reduced = simd16_binary::<MULTIPLY>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        let mut tile = [0_u16; 16];
        // SAFETY: the local output tile provides exactly 32 writable bytes.
        unsafe { _mm256_storeu_si256(tile.as_mut_ptr().cast::<__m256i>(), reduced) };
        for lane in 0..16 {
            out[index + lane] = F::__from_reduced_canonical_u16(tile[lane]);
        }
        index += 16;
    }
    _mm256_zeroupper();
    for tail in index..out.len() {
        out[tail] = if MULTIPLY {
            lhs[tail].mul(rhs[tail])
        } else {
            lhs[tail].add(rhs[tail])
        };
    }
}

#[target_feature(enable = "avx2")]
unsafe fn canonical16_assign_impl<F: VerifiedPrimeCanonical16Field, const SQUARE: bool>(
    lhs: &mut [F],
    rhs: &[F],
) {
    let mut index = 0;
    while index + 16 <= lhs.len() {
        let mut left = [0_u16; 16];
        let mut right = [0_u16; 16];
        for lane in 0..16 {
            left[lane] = lhs[index + lane].__into_canonical_u16();
            right[lane] = if SQUARE {
                left[lane]
            } else {
                rhs[index + lane].__into_canonical_u16()
            };
        }
        // SAFETY: both local input tiles provide exactly 32 readable bytes.
        let left = unsafe { _mm256_loadu_si256(left.as_ptr().cast::<__m256i>()) };
        let right = unsafe { _mm256_loadu_si256(right.as_ptr().cast::<__m256i>()) };
        let reduced = simd16_binary::<true>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        let mut tile = [0_u16; 16];
        // SAFETY: the local output tile provides exactly 32 writable bytes.
        unsafe { _mm256_storeu_si256(tile.as_mut_ptr().cast::<__m256i>(), reduced) };
        for lane in 0..16 {
            lhs[index + lane] = F::__from_reduced_canonical_u16(tile[lane]);
        }
        index += 16;
    }
    _mm256_zeroupper();
    for tail in index..lhs.len() {
        lhs[tail] = if SQUARE {
            lhs[tail].mul(lhs[tail])
        } else {
            lhs[tail].mul(rhs[tail])
        };
    }
}

#[target_feature(enable = "avx2")]
unsafe fn canonical32_binary_impl<F: VerifiedPrimeCanonical32Field, const MULTIPLY: bool>(
    out: &mut [F],
    lhs: &[F],
    rhs: &[F],
) {
    let mut index = 0;
    while index + 8 <= out.len() {
        let mut left = [0_u32; 8];
        let mut right = [0_u32; 8];
        for lane in 0..8 {
            left[lane] = lhs[index + lane].__into_canonical_u32();
            right[lane] = rhs[index + lane].__into_canonical_u32();
        }
        // SAFETY: both local tiles provide one complete readable YMM vector.
        let left = unsafe { _mm256_loadu_si256(left.as_ptr().cast::<__m256i>()) };
        let right = unsafe { _mm256_loadu_si256(right.as_ptr().cast::<__m256i>()) };
        let reduced = simd32_binary::<MULTIPLY>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        let mut tile = [0_u32; 8];
        // SAFETY: the local output tile provides one complete writable vector.
        unsafe { _mm256_storeu_si256(tile.as_mut_ptr().cast::<__m256i>(), reduced) };
        for lane in 0..8 {
            out[index + lane] = F::__from_reduced_canonical_u32(tile[lane]);
        }
        index += 8;
    }
    _mm256_zeroupper();
    for tail in index..out.len() {
        out[tail] = if MULTIPLY {
            lhs[tail].mul(rhs[tail])
        } else {
            lhs[tail].add(rhs[tail])
        };
    }
}

#[target_feature(enable = "avx2")]
unsafe fn canonical32_assign_impl<F: VerifiedPrimeCanonical32Field, const SQUARE: bool>(
    lhs: &mut [F],
    rhs: &[F],
) {
    let mut index = 0;
    while index + 8 <= lhs.len() {
        let mut left = [0_u32; 8];
        let mut right = [0_u32; 8];
        for lane in 0..8 {
            left[lane] = lhs[index + lane].__into_canonical_u32();
            right[lane] = if SQUARE {
                left[lane]
            } else {
                rhs[index + lane].__into_canonical_u32()
            };
        }
        // SAFETY: both local tiles provide one complete readable YMM vector.
        let left = unsafe { _mm256_loadu_si256(left.as_ptr().cast::<__m256i>()) };
        let right = unsafe { _mm256_loadu_si256(right.as_ptr().cast::<__m256i>()) };
        let reduced = simd32_binary::<true>(left, right, F::__MODULUS, F::__BARRETT_RECIPROCAL);
        let mut tile = [0_u32; 8];
        // SAFETY: the local output tile provides one complete writable vector.
        unsafe { _mm256_storeu_si256(tile.as_mut_ptr().cast::<__m256i>(), reduced) };
        for lane in 0..8 {
            lhs[index + lane] = F::__from_reduced_canonical_u32(tile[lane]);
        }
        index += 8;
    }
    _mm256_zeroupper();
    for tail in index..lhs.len() {
        lhs[tail] = if SQUARE {
            lhs[tail].mul(lhs[tail])
        } else {
            lhs[tail].mul(rhs[tail])
        };
    }
}

#[target_feature(enable = "avx2")]
fn simd16_binary<const MULTIPLY: bool>(
    left: __m256i,
    right: __m256i,
    modulus: u32,
    reciprocal: u32,
) -> __m256i {
    let left_low = _mm256_cvtepu16_epi32(_mm256_castsi256_si128(left));
    let left_high = _mm256_cvtepu16_epi32(_mm256_extracti128_si256::<1>(left));
    let right_low = _mm256_cvtepu16_epi32(_mm256_castsi256_si128(right));
    let right_high = _mm256_cvtepu16_epi32(_mm256_extracti128_si256::<1>(right));
    let (wide_low, wide_high) = if MULTIPLY {
        (
            _mm256_mullo_epi32(left_low, right_low),
            _mm256_mullo_epi32(left_high, right_high),
        )
    } else {
        (
            _mm256_add_epi32(left_low, right_low),
            _mm256_add_epi32(left_high, right_high),
        )
    };
    let low = reduce_u32_lanes(wide_low, modulus, reciprocal);
    let high = reduce_u32_lanes(wide_high, modulus, reciprocal);
    _mm256_permute4x64_epi64::<0xd8>(_mm256_packus_epi32(low, high))
}

#[target_feature(enable = "avx2")]
fn reduce_u32_lanes(value: __m256i, modulus: u32, reciprocal: u32) -> __m256i {
    let reciprocal_lanes = _mm256_set1_epi32(reciprocal.cast_signed());
    let even_products = _mm256_mul_epu32(value, reciprocal_lanes);
    let odd_products = _mm256_mul_epu32(
        _mm256_srli_epi64::<32>(value),
        _mm256_srli_epi64::<32>(reciprocal_lanes),
    );
    let even_quotients = _mm256_srli_epi64::<32>(even_products);
    let odd_quotients = _mm256_and_si256(odd_products, _mm256_set1_epi64x(-4_294_967_296));
    let quotient = _mm256_or_si256(even_quotients, odd_quotients);
    let threshold = _mm256_set1_epi32((modulus - 1).cast_signed());
    let modulus = _mm256_set1_epi32(modulus.cast_signed());
    let residue = _mm256_sub_epi32(value, _mm256_mullo_epi32(quotient, modulus));
    let subtract_mask = _mm256_cmpgt_epi32(residue, threshold);
    _mm256_sub_epi32(residue, _mm256_and_si256(subtract_mask, modulus))
}

#[target_feature(enable = "avx2")]
fn simd32_binary<const MULTIPLY: bool>(
    left: __m256i,
    right: __m256i,
    modulus: u64,
    reciprocal: u64,
) -> __m256i {
    let low_mask = _mm256_set1_epi64x(u64::from(u32::MAX).cast_signed());
    let left_even = _mm256_and_si256(left, low_mask);
    let left_odd = _mm256_srli_epi64::<32>(left);
    let right_even = _mm256_and_si256(right, low_mask);
    let right_odd = _mm256_srli_epi64::<32>(right);
    let (even, odd) = if MULTIPLY {
        (
            _mm256_mul_epu32(left_even, right_even),
            _mm256_mul_epu32(left_odd, right_odd),
        )
    } else {
        (
            _mm256_add_epi64(left_even, right_even),
            _mm256_add_epi64(left_odd, right_odd),
        )
    };
    let (even, odd) = if MULTIPLY {
        (
            reduce_u64_barrett_lanes(even, modulus, reciprocal),
            reduce_u64_barrett_lanes(odd, modulus, reciprocal),
        )
    } else {
        (
            reduce_u64_once(even, modulus),
            reduce_u64_once(odd, modulus),
        )
    };
    interleave_u32_lanes(even, odd)
}

#[target_feature(enable = "avx2")]
fn reduce_u64_barrett_lanes(value: __m256i, modulus: u64, reciprocal: u64) -> __m256i {
    let reciprocal = _mm256_set1_epi64x(reciprocal.cast_signed());
    let quotient = multiply_high_u64_lanes(value, reciprocal);
    let modulus_lanes = _mm256_set1_epi64x(modulus.cast_signed());
    let quotient_times_modulus = _mm256_mul_epu32(quotient, modulus_lanes);
    reduce_u64_once(_mm256_sub_epi64(value, quotient_times_modulus), modulus)
}

#[target_feature(enable = "avx2")]
fn multiply_high_u64_lanes(lhs: __m256i, rhs: __m256i) -> __m256i {
    let low_mask = _mm256_set1_epi64x(u64::from(u32::MAX).cast_signed());
    let lhs_high = _mm256_srli_epi64::<32>(lhs);
    let rhs_high = _mm256_srli_epi64::<32>(rhs);
    let low_low = _mm256_mul_epu32(lhs, rhs);
    let low_high = _mm256_mul_epu32(lhs, rhs_high);
    let high_low = _mm256_mul_epu32(lhs_high, rhs);
    let high_high = _mm256_mul_epu32(lhs_high, rhs_high);
    let middle = _mm256_add_epi64(
        _mm256_add_epi64(
            _mm256_srli_epi64::<32>(low_low),
            _mm256_and_si256(low_high, low_mask),
        ),
        _mm256_and_si256(high_low, low_mask),
    );
    _mm256_add_epi64(
        _mm256_add_epi64(
            _mm256_add_epi64(high_high, _mm256_srli_epi64::<32>(low_high)),
            _mm256_srli_epi64::<32>(high_low),
        ),
        _mm256_srli_epi64::<32>(middle),
    )
}

#[target_feature(enable = "avx2")]
fn reduce_u64_once(value: __m256i, modulus: u64) -> __m256i {
    let modulus = _mm256_set1_epi64x(modulus.cast_signed());
    let below = unsigned_lt_lanes(value, modulus);
    _mm256_sub_epi64(value, _mm256_andnot_si256(below, modulus))
}

#[target_feature(enable = "avx2")]
fn interleave_u32_lanes(even: __m256i, odd: __m256i) -> __m256i {
    _mm256_blend_epi32::<0xaa>(even, _mm256_slli_epi64::<32>(odd))
}

fn bmi2_multiply<F, const LIMBS: usize, const WIDE_LIMBS: usize>(
    out: &mut [F],
    lhs: &[F],
    rhs: &[F],
) where
    F: VerifiedPrimeMontgomery64Field<LIMBS, WIDE_LIMBS>,
{
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: BMI2 availability was checked once by EngineBuilder; all slice
    // accesses occur through safe iteration.
    unsafe { bmi2_multiply_impl::<F, LIMBS, WIDE_LIMBS>(out, lhs, rhs) };
}

fn bmi2_square<F, const LIMBS: usize, const WIDE_LIMBS: usize>(out: &mut [F], values: &[F])
where
    F: VerifiedPrimeMontgomery64Field<LIMBS, WIDE_LIMBS>,
{
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: same target-feature precondition as multiplication.
    unsafe { bmi2_multiply_impl::<F, LIMBS, WIDE_LIMBS>(out, values, values) };
}

fn bmi2_multiply_assign<F, const LIMBS: usize, const WIDE_LIMBS: usize>(lhs: &mut [F], rhs: &[F])
where
    F: VerifiedPrimeMontgomery64Field<LIMBS, WIDE_LIMBS>,
{
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: BMI2 was detected and every value is copied before replacement.
    unsafe {
        for (left, right) in lhs.iter_mut().zip(rhs) {
            *left = bmi2_mul::<F, LIMBS, WIDE_LIMBS>(*left, *right);
        }
    }
}

fn bmi2_square_assign<F, const LIMBS: usize, const WIDE_LIMBS: usize>(values: &mut [F])
where
    F: VerifiedPrimeMontgomery64Field<LIMBS, WIDE_LIMBS>,
{
    // SAFETY: BMI2 was detected and each value is copied before replacement.
    unsafe {
        for value in values {
            *value = bmi2_mul::<F, LIMBS, WIDE_LIMBS>(*value, *value);
        }
    }
}

#[target_feature(enable = "bmi2")]
unsafe fn bmi2_multiply_impl<F, const LIMBS: usize, const WIDE_LIMBS: usize>(
    out: &mut [F],
    lhs: &[F],
    rhs: &[F],
) where
    F: VerifiedPrimeMontgomery64Field<LIMBS, WIDE_LIMBS>,
{
    for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
        // SAFETY: inherited BMI2 target feature.
        *output = unsafe { bmi2_mul::<F, LIMBS, WIDE_LIMBS>(*left, *right) };
    }
}

#[target_feature(enable = "bmi2")]
unsafe fn bmi2_mul<F, const LIMBS: usize, const WIDE_LIMBS: usize>(lhs: F, rhs: F) -> F
where
    F: VerifiedPrimeMontgomery64Field<LIMBS, WIDE_LIMBS>,
{
    let left = lhs.__into_montgomery_limbs();
    let right = rhs.__into_montgomery_limbs();
    let mut wide = [0_u64; WIDE_LIMBS];
    // SAFETY: this function already requires BMI2 and the const-generic shape
    // was checked when the opaque strategy was constructed.
    unsafe { bmi2_wide_product::<LIMBS, WIDE_LIMBS>(&mut wide, &left, &right) };
    let reduced =
        crate::prime::montgomery_reduce_wide::<LIMBS, WIDE_LIMBS>(wide, F::__MODULUS, F::__NEG_INV);
    F::__from_reduced_montgomery_limbs(reduced)
}

#[inline]
#[target_feature(enable = "bmi2")]
unsafe fn bmi2_wide_product<const LIMBS: usize, const WIDE_LIMBS: usize>(
    wide: &mut [u64; WIDE_LIMBS],
    left: &[u64; LIMBS],
    right: &[u64; LIMBS],
) {
    debug_assert_eq!(WIDE_LIMBS, LIMBS * 2);
    for (left_index, left_limb) in left.iter().copied().enumerate() {
        let mut carry = 0_u64;
        for (right_index, right_limb) in right.iter().copied().enumerate() {
            let index = left_index + right_index;
            let mut high = 0_u64;
            let low = _mulx_u64(left_limb, right_limb, &mut high);
            let (sum, carry_low) = wide[index].overflowing_add(low);
            let (sum, carry_in) = sum.overflowing_add(carry);
            wide[index] = sum;
            carry = high
                .wrapping_add(u64::from(carry_low))
                .wrapping_add(u64::from(carry_in));
        }
        // No earlier row reaches this limb. Storing the final carry therefore
        // completes a fixed N-by-N row without data-dependent propagation.
        wide[left_index + LIMBS] = carry;
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn bmi2_product_builder_covers_64_128_192_and_256_bit_shapes() {
        if !std::arch::is_x86_feature_detected!("bmi2") {
            return;
        }
        check_limb_shape::<1, 2>();
        check_limb_shape::<2, 4>();
        check_limb_shape::<3, 6>();
        check_limb_shape::<4, 8>();
    }

    #[test]
    fn goldilocks_vector_product_matches_scalar_for_basis_boundaries_and_seeded_inputs() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }
        for left_bit in 0..64 {
            for right_bit in 0..64 {
                check_goldilocks_lanes(
                    [
                        1_u64 << left_bit,
                        1_u64 << right_bit,
                        (1_u64 << left_bit) ^ (1_u64 << right_bit),
                        FpGoldilocks64V1::MODULUS - 1,
                    ],
                    [
                        1_u64 << right_bit,
                        1_u64 << left_bit,
                        FpGoldilocks64V1::MODULUS - 1,
                        (1_u64 << left_bit) ^ (1_u64 << right_bit),
                    ],
                );
            }
        }

        let boundaries = [
            0,
            1,
            2,
            0xffff_ffff,
            1_u64 << 32,
            1_u64 << 63,
            FpGoldilocks64V1::MODULUS - 2,
            FpGoldilocks64V1::MODULUS - 1,
        ];
        for left in boundaries {
            for right in boundaries {
                check_goldilocks_lanes(
                    [left, right, left, right],
                    [right, left, FpGoldilocks64V1::MODULUS - 1, 0xffff_ffff],
                );
            }
        }

        let mut state = 0x243f_6a88_85a3_08d3_u64;
        for _ in 0..25_000 {
            let mut lhs = [0; 4];
            let mut rhs = [0; 4];
            for value in lhs.iter_mut().chain(&mut rhs) {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *value = state % FpGoldilocks64V1::MODULUS;
            }
            check_goldilocks_lanes(lhs, rhs);
        }
    }

    #[test]
    fn generic_small_prime_simd_covers_profile_extremes_and_intermediate_moduli() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        for modulus in [3_u16, 17, 251] {
            let pair_count = u32::from(modulus) * u32::from(modulus);
            for base in (0..pair_count).step_by(32) {
                let lhs = core::array::from_fn(|lane| {
                    u8::try_from(
                        ((base + u32::try_from(lane).unwrap()) % pair_count) / u32::from(modulus),
                    )
                    .unwrap()
                });
                let rhs = core::array::from_fn(|lane| {
                    u8::try_from(
                        ((base + u32::try_from(lane).unwrap()) % pair_count) % u32::from(modulus),
                    )
                    .unwrap()
                });
                check_simd8_tile(modulus, lhs, rhs);
            }
        }

        exhaustive_simd16_modulus(257);
        let mut state = 0x243f_6a88_85a3_08d3_u64;
        for modulus in [769_u32, 4_093, 65_521] {
            let boundary = [0, 1, 2, modulus / 2, modulus - 2, modulus - 1];
            for left in boundary {
                for right in boundary {
                    check_simd16_tile(
                        modulus,
                        [u16::try_from(left).unwrap(); 16],
                        [u16::try_from(right).unwrap(); 16],
                    );
                }
            }
            for _ in 0..1_024 {
                let lhs = core::array::from_fn(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    u16::try_from(state % u64::from(modulus)).unwrap()
                });
                let rhs = core::array::from_fn(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    u16::try_from(state % u64::from(modulus)).unwrap()
                });
                check_simd16_tile(modulus, lhs, rhs);
            }
        }
    }

    #[test]
    fn generic_u32_simd_covers_max_prime_basis_boundaries_and_seeded_inputs() {
        const MODULUS: u64 = 4_294_967_291;
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }

        for left_bit in 0..32 {
            for right_bit in 0..32 {
                check_simd32_tile(
                    MODULUS,
                    [
                        1_u32 << left_bit,
                        1_u32 << right_bit,
                        (1_u32 << left_bit) ^ (1_u32 << right_bit),
                        (MODULUS - 1) as u32,
                        0,
                        1,
                        2,
                        (MODULUS / 2) as u32,
                    ],
                    [
                        1_u32 << right_bit,
                        1_u32 << left_bit,
                        (MODULUS - 1) as u32,
                        (1_u32 << left_bit) ^ (1_u32 << right_bit),
                        (MODULUS - 1) as u32,
                        (MODULUS - 2) as u32,
                        (MODULUS / 2) as u32,
                        2,
                    ],
                );
            }
        }

        let boundaries = [
            0_u32,
            1,
            2,
            (MODULUS / 2) as u32,
            (MODULUS - 2) as u32,
            (MODULUS - 1) as u32,
        ];
        for left in boundaries {
            for right in boundaries {
                check_simd32_tile(MODULUS, [left; 8], [right; 8]);
            }
        }

        let mut state = 0x243f_6a88_85a3_08d3_u64;
        for _ in 0..12_500 {
            let lhs = core::array::from_fn(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state % MODULUS) as u32
            });
            let rhs = core::array::from_fn(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state % MODULUS) as u32
            });
            check_simd32_tile(MODULUS, lhs, rhs);
        }
    }

    fn exhaustive_simd16_modulus(modulus: u32) {
        let pair_count = modulus * modulus;
        for base in (0..pair_count).step_by(16) {
            let lhs = core::array::from_fn(|lane| {
                u16::try_from(((base + u32::try_from(lane).unwrap()) % pair_count) / modulus)
                    .unwrap()
            });
            let rhs = core::array::from_fn(|lane| {
                u16::try_from(((base + u32::try_from(lane).unwrap()) % pair_count) % modulus)
                    .unwrap()
            });
            check_simd16_tile(modulus, lhs, rhs);
        }
    }

    fn check_simd8_tile(modulus: u16, lhs: [u8; 32], rhs: [u8; 32]) {
        let reciprocal = u16::try_from(65_536_u32 / u32::from(modulus)).unwrap();
        // SAFETY: the caller returned early unless AVX2 was detected. Both
        // local arrays are complete YMM tiles and contain canonical residues.
        let (product, sum) = unsafe {
            let left = _mm256_loadu_si256(lhs.as_ptr().cast::<__m256i>());
            let right = _mm256_loadu_si256(rhs.as_ptr().cast::<__m256i>());
            let mut product = [0_u8; 32];
            let mut sum = [0_u8; 32];
            _mm256_storeu_si256(
                product.as_mut_ptr().cast::<__m256i>(),
                simd8_binary::<true>(left, right, modulus, reciprocal),
            );
            _mm256_storeu_si256(
                sum.as_mut_ptr().cast::<__m256i>(),
                simd8_binary::<false>(left, right, modulus, reciprocal),
            );
            (product, sum)
        };
        for lane in 0..32 {
            assert_eq!(
                u16::from(product[lane]),
                u16::from(lhs[lane]) * u16::from(rhs[lane]) % modulus
            );
            assert_eq!(
                u16::from(sum[lane]),
                (u16::from(lhs[lane]) + u16::from(rhs[lane])) % modulus
            );
        }
    }

    fn check_simd16_tile(modulus: u32, lhs: [u16; 16], rhs: [u16; 16]) {
        let reciprocal = u32::try_from((1_u64 << 32) / u64::from(modulus)).unwrap();
        // SAFETY: the caller returned early unless AVX2 was detected. Both
        // local arrays are complete YMM tiles and contain canonical residues.
        let (product, sum) = unsafe {
            let left = _mm256_loadu_si256(lhs.as_ptr().cast::<__m256i>());
            let right = _mm256_loadu_si256(rhs.as_ptr().cast::<__m256i>());
            let mut product = [0_u16; 16];
            let mut sum = [0_u16; 16];
            _mm256_storeu_si256(
                product.as_mut_ptr().cast::<__m256i>(),
                simd16_binary::<true>(left, right, modulus, reciprocal),
            );
            _mm256_storeu_si256(
                sum.as_mut_ptr().cast::<__m256i>(),
                simd16_binary::<false>(left, right, modulus, reciprocal),
            );
            (product, sum)
        };
        for lane in 0..16 {
            assert_eq!(
                u32::from(product[lane]),
                u32::from(lhs[lane]) * u32::from(rhs[lane]) % modulus
            );
            assert_eq!(
                u32::from(sum[lane]),
                (u32::from(lhs[lane]) + u32::from(rhs[lane])) % modulus
            );
        }
    }

    fn check_simd32_tile(modulus: u64, lhs: [u32; 8], rhs: [u32; 8]) {
        let reciprocal = u64::MAX / modulus;
        // SAFETY: the caller returned early unless AVX2 was detected. Both
        // inputs and outputs contain one complete YMM tile.
        let (product, sum) = unsafe {
            let left = _mm256_loadu_si256(lhs.as_ptr().cast::<__m256i>());
            let right = _mm256_loadu_si256(rhs.as_ptr().cast::<__m256i>());
            let mut product = [0_u32; 8];
            let mut sum = [0_u32; 8];
            _mm256_storeu_si256(
                product.as_mut_ptr().cast::<__m256i>(),
                simd32_binary::<true>(left, right, modulus, reciprocal),
            );
            _mm256_storeu_si256(
                sum.as_mut_ptr().cast::<__m256i>(),
                simd32_binary::<false>(left, right, modulus, reciprocal),
            );
            (product, sum)
        };
        for lane in 0..8 {
            assert_eq!(
                u64::from(product[lane]),
                (u64::from(lhs[lane]) * u64::from(rhs[lane])) % modulus,
                "product lane={lane} lhs={lhs:?} rhs={rhs:?}"
            );
            assert_eq!(
                u64::from(sum[lane]),
                (u64::from(lhs[lane]) + u64::from(rhs[lane])) % modulus,
                "sum lane={lane} lhs={lhs:?} rhs={rhs:?}"
            );
        }
    }

    fn check_goldilocks_lanes(lhs: [u64; 4], rhs: [u64; 4]) {
        // SAFETY: the caller returned early unless AVX2 was detected. Values
        // are canonical and the output array has one complete YMM tile.
        let actual = unsafe {
            let left = _mm256_set_epi64x(
                lhs[3].cast_signed(),
                lhs[2].cast_signed(),
                lhs[1].cast_signed(),
                lhs[0].cast_signed(),
            );
            let right = _mm256_set_epi64x(
                rhs[3].cast_signed(),
                rhs[2].cast_signed(),
                rhs[1].cast_signed(),
                rhs[0].cast_signed(),
            );
            let product = goldilocks_multiply_lanes(left, right);
            let mut lanes = [0_u64; 4];
            _mm256_storeu_si256(lanes.as_mut_ptr().cast::<__m256i>(), product);
            lanes
        };
        for lane in 0..4 {
            let expected = crate::prime::barrett_reduce_goldilocks(
                u128::from(lhs[lane]) * u128::from(rhs[lane]),
            );
            assert_eq!(
                actual[lane], expected,
                "lane={lane} lhs={lhs:?} rhs={rhs:?}"
            );
        }
    }

    fn check_limb_shape<const LIMBS: usize, const WIDE_LIMBS: usize>() {
        assert_eq!(WIDE_LIMBS, LIMBS * 2);
        for left_bit in 0..LIMBS * 64 {
            for right_bit in 0..LIMBS * 64 {
                let mut left = [0_u64; LIMBS];
                let mut right = [0_u64; LIMBS];
                left[left_bit / 64] = 1_u64 << (left_bit % 64);
                right[right_bit / 64] = 1_u64 << (right_bit % 64);
                let mut expected = [0_u64; WIDE_LIMBS];
                let product_bit = left_bit + right_bit;
                expected[product_bit / 64] = 1_u64 << (product_bit % 64);
                assert_eq!(bmi2_product::<LIMBS, WIDE_LIMBS>(left, right), expected);
            }
        }

        check_product::<LIMBS, WIDE_LIMBS>([0; LIMBS], [u64::MAX; LIMBS]);
        check_product::<LIMBS, WIDE_LIMBS>([u64::MAX; LIMBS], [u64::MAX; LIMBS]);
        check_product::<LIMBS, WIDE_LIMBS>(
            [0xaaaa_aaaa_aaaa_aaaa; LIMBS],
            [0x5555_5555_5555_5555; LIMBS],
        );

        let mut state = 0x243f_6a88_85a3_08d3_u64 ^ LIMBS as u64;
        for _ in 0..128 {
            let mut left = [0_u64; LIMBS];
            let mut right = [0_u64; LIMBS];
            for limb in left.iter_mut().chain(&mut right) {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *limb = state;
            }
            check_product::<LIMBS, WIDE_LIMBS>(left, right);
        }
    }

    fn check_product<const LIMBS: usize, const WIDE_LIMBS: usize>(
        left: [u64; LIMBS],
        right: [u64; LIMBS],
    ) {
        assert_eq!(
            bmi2_product::<LIMBS, WIDE_LIMBS>(left, right),
            portable_product::<LIMBS, WIDE_LIMBS>(left, right)
        );
    }

    fn bmi2_product<const LIMBS: usize, const WIDE_LIMBS: usize>(
        left: [u64; LIMBS],
        right: [u64; LIMBS],
    ) -> [u64; WIDE_LIMBS] {
        let mut wide = [0_u64; WIDE_LIMBS];
        // SAFETY: the caller test returned early unless this CPU reported BMI2;
        // each instantiation supplies a double-width output.
        unsafe { bmi2_wide_product::<LIMBS, WIDE_LIMBS>(&mut wide, &left, &right) };
        wide
    }

    #[allow(clippy::cast_possible_truncation)]
    fn portable_product<const LIMBS: usize, const WIDE_LIMBS: usize>(
        left: [u64; LIMBS],
        right: [u64; LIMBS],
    ) -> [u64; WIDE_LIMBS] {
        let mut wide = [0_u64; WIDE_LIMBS];
        for (left_index, left_limb) in left.iter().copied().enumerate() {
            let mut carry = 0_u64;
            for (right_index, right_limb) in right.iter().copied().enumerate() {
                let index = left_index + right_index;
                let combined = u128::from(wide[index])
                    + u128::from(left_limb) * u128::from(right_limb)
                    + u128::from(carry);
                wide[index] = combined as u64;
                carry = (combined >> 64) as u64;
            }
            for limb in &mut wide[left_index + LIMBS..] {
                let (sum, overflow) = limb.overflowing_add(carry);
                *limb = sum;
                carry = u64::from(overflow);
            }
            assert_eq!(carry, 0);
        }
        wide
    }
}
