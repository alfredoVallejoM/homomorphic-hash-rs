//! Narrow x86-64 adapters for independent prime residues.

// The unaligned load/store intrinsics intentionally accept byte-aligned
// addresses even though their pointer type carries vector alignment.
#![allow(clippy::cast_ptr_alignment)]

use core::arch::x86_64::{
    __m128i, __m256i, _mm_loadu_si128, _mm_packus_epi16, _mm_storeu_si128, _mm256_add_epi16,
    _mm256_and_si256, _mm256_castsi256_si128, _mm256_cmpgt_epi16, _mm256_cvtepu8_epi16,
    _mm256_extracti128_si256, _mm256_loadu_si256, _mm256_mulhi_epu16, _mm256_mullo_epi16,
    _mm256_packus_epi16, _mm256_permute4x64_epi64, _mm256_set1_epi16, _mm256_storeu_si256,
    _mm256_sub_epi16, _mm256_zeroupper, _mulx_u64,
};

use crate::{Field, Fp251V1, Fp256GenericV1, KernelMetadata, kernel::KernelSet};

pub(crate) static FP251_AVX2_KERNELS: KernelSet<Fp251V1> = KernelSet::new(
    KernelMetadata::x86_prime_avx2::<Fp251V1>(64).with_prime(
        crate::PrimeKernelMetadata::__from_generated(
            crate::PrimeRepresentationKind::CanonicalResidue,
            crate::PrimeReductionKind::Barrett,
            crate::RangeContract::__from_generated(1, 1, 16),
            crate::RangeContract::__from_generated(1, 1, 16),
            16,
            false,
        ),
    ),
    fp251_add,
    fp251_multiply,
    fp251_square,
    fp251_multiply_assign,
    fp251_square_assign,
);

pub(crate) static FP256_BMI2_KERNELS: KernelSet<Fp256GenericV1> = KernelSet::new(
    KernelMetadata::x86_prime_bmi2::<Fp256GenericV1>(1).with_prime(
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
    ),
    add_scalar::<Fp256GenericV1>,
    fp256_multiply,
    fp256_square,
    fp256_multiply_assign,
    fp256_square_assign,
);

fn add_scalar<F: Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
        *output = left.add(*right);
    }
}

fn fp251_add(out: &mut [Fp251V1], lhs: &[Fp251V1], rhs: &[Fp251V1]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: EngineBuilder exposes this table only after AVX2 detection. All
    // loads/stores remain within equally sized slices and the vector reducer
    // produces canonical residues below 251.
    unsafe { fp251_binary_impl(out, lhs, rhs, Fp251Operation::Add) };
}

fn fp251_multiply(out: &mut [Fp251V1], lhs: &[Fp251V1], rhs: &[Fp251V1]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: same preconditions and canonical-output proof as `fp251_add`.
    unsafe { fp251_binary_impl(out, lhs, rhs, Fp251Operation::Multiply) };
}

fn fp251_square(out: &mut [Fp251V1], values: &[Fp251V1]) {
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: same AVX2 and slice bounds established by the selected engine.
    unsafe { fp251_binary_impl(out, values, values, Fp251Operation::Multiply) };
}

fn fp251_multiply_assign(lhs: &mut [Fp251V1], rhs: &[Fp251V1]) {
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: each vector is loaded before the corresponding output store;
    // AVX2 support was established during engine construction.
    unsafe { fp251_assign_impl(lhs, rhs, false) };
}

fn fp251_square_assign(values: &mut [Fp251V1]) {
    // SAFETY: each vector is loaded before its in-place store and every output
    // lane is reduced to the canonical byte range.
    unsafe { fp251_assign_impl(values, &[], true) };
}

#[derive(Clone, Copy)]
enum Fp251Operation {
    Add,
    Multiply,
}

#[target_feature(enable = "avx2")]
unsafe fn fp251_binary_impl(
    out: &mut [Fp251V1],
    lhs: &[Fp251V1],
    rhs: &[Fp251V1],
    operation: Fp251Operation,
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
        let (wide_low, wide_high) = match operation {
            Fp251Operation::Add => (
                _mm256_add_epi16(left_low, right_low),
                _mm256_add_epi16(left_high, right_high),
            ),
            Fp251Operation::Multiply => (
                _mm256_mullo_epi16(left_low, right_low),
                _mm256_mullo_epi16(left_high, right_high),
            ),
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
        let wide = match operation {
            Fp251Operation::Add => _mm256_add_epi16(left, right),
            Fp251Operation::Multiply => _mm256_mullo_epi16(left, right),
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
        out[tail] = match operation {
            Fp251Operation::Add => lhs[tail].add(rhs[tail]),
            Fp251Operation::Multiply => lhs[tail].mul(rhs[tail]),
        };
    }
}

#[target_feature(enable = "avx2")]
unsafe fn fp251_assign_impl(lhs: &mut [Fp251V1], rhs: &[Fp251V1], square: bool) {
    let mut index = 0;
    while index + 32 <= lhs.len() {
        // SAFETY: the loop proves a complete 32-byte left chunk.
        let left_bytes = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(index).cast::<__m256i>()) };
        let right_bytes = if square {
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
        let right_bytes = if square {
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
        lhs[tail] = if square {
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

fn fp256_multiply(out: &mut [Fp256GenericV1], lhs: &[Fp256GenericV1], rhs: &[Fp256GenericV1]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: BMI2 availability was checked once by EngineBuilder; all slice
    // accesses occur through safe iteration.
    unsafe { fp256_multiply_impl(out, lhs, rhs) };
}

fn fp256_square(out: &mut [Fp256GenericV1], values: &[Fp256GenericV1]) {
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: same target-feature precondition as multiplication.
    unsafe { fp256_multiply_impl(out, values, values) };
}

fn fp256_multiply_assign(lhs: &mut [Fp256GenericV1], rhs: &[Fp256GenericV1]) {
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: BMI2 was detected and every value is copied before replacement.
    unsafe {
        for (left, right) in lhs.iter_mut().zip(rhs) {
            *left = fp256_mul_bmi2(*left, *right);
        }
    }
}

fn fp256_square_assign(values: &mut [Fp256GenericV1]) {
    // SAFETY: BMI2 was detected and each value is copied before replacement.
    unsafe {
        for value in values {
            *value = fp256_mul_bmi2(*value, *value);
        }
    }
}

#[target_feature(enable = "bmi2")]
unsafe fn fp256_multiply_impl(
    out: &mut [Fp256GenericV1],
    lhs: &[Fp256GenericV1],
    rhs: &[Fp256GenericV1],
) {
    for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
        // SAFETY: inherited BMI2 target feature.
        *output = unsafe { fp256_mul_bmi2(*left, *right) };
    }
}

#[target_feature(enable = "bmi2")]
unsafe fn fp256_mul_bmi2(lhs: Fp256GenericV1, rhs: Fp256GenericV1) -> Fp256GenericV1 {
    let left = lhs.into_montgomery_limbs();
    let right = rhs.into_montgomery_limbs();
    let mut wide = [0_u64; 8];
    for (left_index, left_limb) in left.iter().copied().enumerate() {
        for (right_index, right_limb) in right.iter().copied().enumerate() {
            let mut high = 0_u64;
            let low = _mulx_u64(left_limb, right_limb, &mut high);
            add_wide_product(&mut wide, left_index + right_index, low, high);
        }
    }
    Fp256GenericV1::reduce_isa_product(wide)
}

fn add_wide_product(wide: &mut [u64; 8], index: usize, low: u64, high: u64) {
    let (sum, carry_low) = wide[index].overflowing_add(low);
    wide[index] = sum;
    let (sum, carry_high) = wide[index + 1].overflowing_add(high);
    let (sum, carry_from_low) = sum.overflowing_add(u64::from(carry_low));
    wide[index + 1] = sum;
    let mut carry = carry_high || carry_from_low;
    let mut carry_index = index + 2;
    while carry {
        debug_assert!(carry_index < wide.len());
        let (sum, overflow) = wide[carry_index].overflowing_add(1);
        wide[carry_index] = sum;
        carry = overflow;
        carry_index += 1;
    }
}
