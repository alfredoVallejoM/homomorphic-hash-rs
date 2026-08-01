//! x86-64 PCLMUL batch strategy.
//!
//! This is the only Microfield module allowed to use `unsafe`. Its private
//! safe kernel entry points are installed only in catalogs selected after a
//! trusted `pclmulqdq` capability check.

use core::{
    arch::x86_64::{__m128i, _mm_clmulepi64_si128, _mm_set_epi64x, _mm_storeu_si128},
    mem::MaybeUninit,
};

#[cfg(feature = "builtin-fields")]
mod builtins {
    use super::{
        wide_product_128_karatsuba, wide_product_256_karatsuba, wide_square_128, wide_square_256,
    };
    use crate::{
        Field, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, Square,
        binary::{reduce_128, reduce_256},
        kernel::KernelSet,
    };

    pub(crate) static GF2_128_V1_KERNELS: KernelSet<Gf2_128V1> = kernel_set::<Gf2_128V1>();
    pub(crate) static GF2_256_HH_V1_KERNELS: KernelSet<Gf2_256HhV1> = kernel_set::<Gf2_256HhV1>();
    pub(crate) static GF2_256_ALT_V1_KERNELS: KernelSet<Gf2_256AltV1> =
        kernel_set::<Gf2_256AltV1>();

    trait PclmulElement: Field + Square {
        /// Multiplies after the selector has established `pclmulqdq` support.
        ///
        /// # Safety
        ///
        /// The current CPU must support `pclmulqdq`.
        unsafe fn multiply_pclmul(self, rhs: Self) -> Self;
        /// Squares after the selector has established `pclmulqdq` support.
        ///
        /// # Safety
        ///
        /// The current CPU must support `pclmulqdq`.
        unsafe fn square_pclmul(self) -> Self;
    }

    impl PclmulElement for Gf2_128V1 {
        #[inline]
        unsafe fn multiply_pclmul(self, rhs: Self) -> Self {
            let wide = unsafe { wide_product_128_karatsuba(self.into_limbs(), rhs.into_limbs()) };
            Self::from_limbs(reduce_128::<{ Self::ISA_MODULUS_TAIL }>(wide))
        }

        #[inline]
        unsafe fn square_pclmul(self) -> Self {
            let wide = unsafe { wide_square_128(self.into_limbs()) };
            Self::from_limbs(reduce_128::<{ Self::ISA_MODULUS_TAIL }>(wide))
        }
    }

    macro_rules! impl_pclmul_256 {
        ($field:ty) => {
            impl PclmulElement for $field {
                #[inline]
                unsafe fn multiply_pclmul(self, rhs: Self) -> Self {
                    let wide =
                        unsafe { wide_product_256_karatsuba(self.into_limbs(), rhs.into_limbs()) };
                    Self::from_limbs(reduce_256::<{ Self::ISA_MODULUS_TAIL }>(wide))
                }

                #[inline]
                unsafe fn square_pclmul(self) -> Self {
                    let wide = unsafe { wide_square_256(self.into_limbs()) };
                    Self::from_limbs(reduce_256::<{ Self::ISA_MODULUS_TAIL }>(wide))
                }
            }
        };
    }

    impl_pclmul_256!(Gf2_256HhV1);
    impl_pclmul_256!(Gf2_256AltV1);

    const fn kernel_set<F: PclmulElement>() -> KernelSet<F> {
        KernelSet::new(
            crate::KernelMetadata::x86_pclmul::<F>(1),
            add::<F>,
            multiply::<F>,
            square::<F>,
            multiply_assign::<F>,
            square_assign::<F>,
        )
    }

    #[inline]
    fn add<F: Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
        debug_assert_eq!(out.len(), lhs.len());
        debug_assert_eq!(lhs.len(), rhs.len());
        for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
            *output = left.add(*right);
        }
    }

    #[inline]
    fn multiply<F: PclmulElement>(out: &mut [F], lhs: &[F], rhs: &[F]) {
        debug_assert_eq!(out.len(), lhs.len());
        debug_assert_eq!(lhs.len(), rhs.len());
        for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
            // SAFETY: this private function pointer is reachable only from the
            // catalog selected after `CpuCapabilities::supports(X86Pclmul)`.
            *output = unsafe { left.multiply_pclmul(*right) };
        }
    }

    #[inline]
    fn square<F: PclmulElement>(out: &mut [F], values: &[F]) {
        debug_assert_eq!(out.len(), values.len());
        for (output, value) in out.iter_mut().zip(values) {
            // SAFETY: see `multiply`; the same selected kernel set owns this entry.
            *output = unsafe { value.square_pclmul() };
        }
    }

    #[inline]
    fn multiply_assign<F: PclmulElement>(lhs: &mut [F], rhs: &[F]) {
        debug_assert_eq!(lhs.len(), rhs.len());
        for (left, right) in lhs.iter_mut().zip(rhs) {
            // SAFETY: see `multiply`.
            *left = unsafe { left.multiply_pclmul(*right) };
        }
    }

    #[inline]
    fn square_assign<F: PclmulElement>(values: &mut [F]) {
        for value in values {
            // SAFETY: see `multiply`.
            *value = unsafe { value.square_pclmul() };
        }
    }
}

#[cfg(feature = "builtin-fields")]
pub(crate) use builtins::{GF2_128_V1_KERNELS, GF2_256_ALT_V1_KERNELS, GF2_256_HH_V1_KERNELS};

pub(crate) const fn verified_kernel_set<F, const LIMBS: usize, const WIDE_LIMBS: usize>()
-> crate::kernel::KernelSet<F>
where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    crate::kernel::KernelSet::new(
        crate::KernelMetadata::x86_pclmul_explicit::<F>(F::SCHEDULE),
        verified_add::<F, LIMBS, WIDE_LIMBS>,
        verified_multiply::<F, LIMBS, WIDE_LIMBS>,
        verified_square::<F, LIMBS, WIDE_LIMBS>,
        verified_multiply_assign::<F, LIMBS, WIDE_LIMBS>,
        verified_square_assign::<F, LIMBS, WIDE_LIMBS>,
    )
}

#[inline]
fn verified_add<F, const LIMBS: usize, const WIDE_LIMBS: usize>(out: &mut [F], lhs: &[F], rhs: &[F])
where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
        *output = left.add(*right);
    }
}

#[inline]
fn verified_multiply<F, const LIMBS: usize, const WIDE_LIMBS: usize>(
    out: &mut [F],
    lhs: &[F],
    rhs: &[F],
) where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
        // SAFETY: the selector admits this private entry only after the trusted
        // `pclmulqdq` capability snapshot has been checked.
        let wide = unsafe {
            wide_product_schoolbook::<LIMBS, WIDE_LIMBS>(left.__into_limbs(), right.__into_limbs())
        };
        *output = F::__from_reduced_limbs(F::__reduce_wide(wide));
    }
}

#[inline]
fn verified_square<F, const LIMBS: usize, const WIDE_LIMBS: usize>(out: &mut [F], values: &[F])
where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    debug_assert_eq!(out.len(), values.len());
    for (output, value) in out.iter_mut().zip(values) {
        // SAFETY: see `verified_multiply`.
        let wide = unsafe { wide_square_schoolbook::<LIMBS, WIDE_LIMBS>(value.__into_limbs()) };
        *output = F::__from_reduced_limbs(F::__reduce_wide(wide));
    }
}

#[inline]
fn verified_multiply_assign<F, const LIMBS: usize, const WIDE_LIMBS: usize>(
    lhs: &mut [F],
    rhs: &[F],
) where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    debug_assert_eq!(lhs.len(), rhs.len());
    for (left, right) in lhs.iter_mut().zip(rhs) {
        // SAFETY: see `verified_multiply`.
        let wide = unsafe {
            wide_product_schoolbook::<LIMBS, WIDE_LIMBS>(left.__into_limbs(), right.__into_limbs())
        };
        *left = F::__from_reduced_limbs(F::__reduce_wide(wide));
    }
}

#[inline]
fn verified_square_assign<F, const LIMBS: usize, const WIDE_LIMBS: usize>(values: &mut [F])
where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    for value in values {
        // SAFETY: see `verified_multiply`.
        let wide = unsafe { wide_square_schoolbook::<LIMBS, WIDE_LIMBS>(value.__into_limbs()) };
        *value = F::__from_reduced_limbs(F::__reduce_wide(wide));
    }
}

#[target_feature(enable = "pclmulqdq")]
/// Computes one 64 × 64-bit carry-less product.
///
/// # Safety
///
/// The current CPU must support `pclmulqdq`.
unsafe fn clmul64(lhs: u64, rhs: u64) -> [u64; 2] {
    let left = _mm_set_epi64x(0, lhs.cast_signed());
    let right = _mm_set_epi64x(0, rhs.cast_signed());
    let product = _mm_clmulepi64_si128::<0x00>(left, right);
    lanes(product)
}

#[inline]
fn lanes(value: __m128i) -> [u64; 2] {
    #[repr(align(16))]
    struct AlignedLanes([u64; 2]);

    let mut output = MaybeUninit::<AlignedLanes>::uninit();
    unsafe { _mm_storeu_si128(output.as_mut_ptr().cast::<__m128i>(), value) };
    unsafe { output.assume_init().0 }
}

#[target_feature(enable = "pclmulqdq")]
#[cfg(feature = "builtin-fields")]
/// Computes a 128 × 128-bit product with three carry-less multiplications.
///
/// # Safety
///
/// The current CPU must support `pclmulqdq`.
unsafe fn wide_product_128_karatsuba(lhs: [u64; 2], rhs: [u64; 2]) -> [u64; 4] {
    let low = unsafe { clmul64(lhs[0], rhs[0]) };
    let high = unsafe { clmul64(lhs[1], rhs[1]) };
    let mixed = unsafe { clmul64(lhs[0] ^ lhs[1], rhs[0] ^ rhs[1]) };
    let middle = [mixed[0] ^ low[0] ^ high[0], mixed[1] ^ low[1] ^ high[1]];
    [low[0], low[1] ^ middle[0], high[0] ^ middle[1], high[1]]
}

#[target_feature(enable = "pclmulqdq")]
#[cfg(all(test, feature = "std", feature = "builtin-fields"))]
/// Computes the test-only 128-bit schoolbook product.
///
/// # Safety
///
/// The current CPU must support `pclmulqdq`.
unsafe fn wide_product_128_schoolbook(lhs: [u64; 2], rhs: [u64; 2]) -> [u64; 4] {
    unsafe { wide_product_schoolbook::<2, 4>(lhs, rhs) }
}

#[target_feature(enable = "pclmulqdq")]
#[cfg(feature = "builtin-fields")]
/// Computes a 256 × 256-bit product with one outer Karatsuba level.
///
/// # Safety
///
/// The current CPU must support `pclmulqdq`.
unsafe fn wide_product_256_karatsuba(lhs: [u64; 4], rhs: [u64; 4]) -> [u64; 8] {
    let lhs_low = [lhs[0], lhs[1]];
    let lhs_high = [lhs[2], lhs[3]];
    let rhs_low = [rhs[0], rhs[1]];
    let rhs_high = [rhs[2], rhs[3]];
    let low = unsafe { wide_product_128_karatsuba(lhs_low, rhs_low) };
    let high = unsafe { wide_product_128_karatsuba(lhs_high, rhs_high) };
    let mixed = unsafe {
        wide_product_128_karatsuba(
            [lhs[0] ^ lhs[2], lhs[1] ^ lhs[3]],
            [rhs[0] ^ rhs[2], rhs[1] ^ rhs[3]],
        )
    };

    let mut output = [0; 8];
    output[..4].copy_from_slice(&low);
    output[4..].copy_from_slice(&high);
    for index in 0..4 {
        output[index + 2] ^= mixed[index] ^ low[index] ^ high[index];
    }
    output
}

#[target_feature(enable = "pclmulqdq")]
#[cfg(all(test, feature = "std", feature = "builtin-fields"))]
/// Computes the test-only 256-bit schoolbook product.
///
/// # Safety
///
/// The current CPU must support `pclmulqdq`.
unsafe fn wide_product_256_schoolbook(lhs: [u64; 4], rhs: [u64; 4]) -> [u64; 8] {
    unsafe { wide_product_schoolbook::<4, 8>(lhs, rhs) }
}

#[target_feature(enable = "pclmulqdq")]
/// Computes a schoolbook product for fixed limb counts.
///
/// # Safety
///
/// The current CPU must support `pclmulqdq` and `WIDE` must equal
/// `2 * LIMBS`.
unsafe fn wide_product_schoolbook<const LIMBS: usize, const WIDE: usize>(
    lhs: [u64; LIMBS],
    rhs: [u64; LIMBS],
) -> [u64; WIDE] {
    debug_assert_eq!(WIDE, LIMBS * 2);
    let mut output = [0; WIDE];
    for (lhs_index, lhs_limb) in lhs.into_iter().enumerate() {
        for (rhs_index, rhs_limb) in rhs.into_iter().enumerate() {
            let product = unsafe { clmul64(lhs_limb, rhs_limb) };
            output[lhs_index + rhs_index] ^= product[0];
            output[lhs_index + rhs_index + 1] ^= product[1];
        }
    }
    output
}

#[target_feature(enable = "pclmulqdq")]
/// Expands a generic generated-field square without cross terms.
///
/// # Safety
///
/// The current CPU must support `pclmulqdq` and `WIDE` must equal
/// `2 * LIMBS`.
unsafe fn wide_square_schoolbook<const LIMBS: usize, const WIDE: usize>(
    value: [u64; LIMBS],
) -> [u64; WIDE] {
    debug_assert_eq!(WIDE, LIMBS * 2);
    let mut output = [0; WIDE];
    for (index, limb) in value.into_iter().enumerate() {
        let product = unsafe { clmul64(limb, limb) };
        output[index * 2] = product[0];
        output[index * 2 + 1] = product[1];
    }
    output
}

#[target_feature(enable = "pclmulqdq")]
#[cfg(feature = "builtin-fields")]
/// Expands a 128-bit square without cross terms.
///
/// # Safety
///
/// The current CPU must support `pclmulqdq`.
unsafe fn wide_square_128(value: [u64; 2]) -> [u64; 4] {
    let low = unsafe { clmul64(value[0], value[0]) };
    let high = unsafe { clmul64(value[1], value[1]) };
    [low[0], low[1], high[0], high[1]]
}

#[target_feature(enable = "pclmulqdq")]
#[cfg(feature = "builtin-fields")]
/// Expands a 256-bit square without cross terms.
///
/// # Safety
///
/// The current CPU must support `pclmulqdq`.
unsafe fn wide_square_256(value: [u64; 4]) -> [u64; 8] {
    let low = unsafe { wide_square_128([value[0], value[1]]) };
    let high = unsafe { wide_square_128([value[2], value[3]]) };
    [
        low[0], low[1], low[2], low[3], high[0], high[1], high[2], high[3],
    ]
}

#[cfg(all(test, feature = "std", feature = "builtin-fields"))]
mod tests {
    use super::*;

    #[test]
    fn karatsuba_matches_schoolbook_for_boundary_and_seeded_inputs() {
        if !std::arch::is_x86_feature_detected!("pclmulqdq") {
            return;
        }

        let values = [
            0,
            1,
            u64::MAX,
            1 << 63,
            0x0123_4567_89ab_cdef,
            0xfedc_ba98_7654_3210,
        ];
        for &a0 in &values {
            for &a1 in &values {
                for &b0 in &values {
                    let lhs = [a0, a1];
                    let rhs = [b0, !b0];
                    assert_eq!(unsafe { wide_product_128_karatsuba(lhs, rhs) }, unsafe {
                        wide_product_128_schoolbook(lhs, rhs)
                    });
                }
            }
        }

        let mut state = 0x243f_6a88_85a3_08d3_u64;
        for _ in 0..4096 {
            let mut lhs = [0; 4];
            let mut rhs = [0; 4];
            for limb in lhs.iter_mut().chain(&mut rhs) {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *limb = state;
            }
            assert_eq!(unsafe { wide_product_256_karatsuba(lhs, rhs) }, unsafe {
                wide_product_256_schoolbook(lhs, rhs)
            });
        }
    }

    #[test]
    fn dedicated_square_matches_product_for_seeded_inputs() {
        if !std::arch::is_x86_feature_detected!("pclmulqdq") {
            return;
        }

        let mut state = 0x1319_8a2e_0370_7344_u64;
        for _ in 0..4096 {
            let mut value = [0; 4];
            for limb in &mut value {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *limb = state;
            }
            assert_eq!(unsafe { wide_square_256(value) }, unsafe {
                wide_product_256_karatsuba(value, value)
            });
        }
    }
}
