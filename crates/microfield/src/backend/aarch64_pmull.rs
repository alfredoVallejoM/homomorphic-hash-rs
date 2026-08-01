//! `AArch64` PMULL batch strategy.
//!
//! This is the only `AArch64` Microfield module allowed to use `unsafe`. Its
//! private safe kernel entry points are installed only in catalogs selected
//! after trusted NEON and PMULL capability checks.

use core::arch::aarch64::vmull_p64;

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

    trait PmullElement: Field + Square {
        /// Multiplies after the selector has established PMULL support.
        ///
        /// # Safety
        ///
        /// The current CPU must support NEON and PMULL.
        unsafe fn multiply_pmull(self, rhs: Self) -> Self;
        /// Squares after the selector has established PMULL support.
        ///
        /// # Safety
        ///
        /// The current CPU must support NEON and PMULL.
        unsafe fn square_pmull(self) -> Self;
    }

    impl PmullElement for Gf2_128V1 {
        #[inline]
        unsafe fn multiply_pmull(self, rhs: Self) -> Self {
            let wide = unsafe { wide_product_128_karatsuba(self.into_limbs(), rhs.into_limbs()) };
            Self::from_limbs(reduce_128::<{ Self::ISA_MODULUS_TAIL }>(wide))
        }

        #[inline]
        unsafe fn square_pmull(self) -> Self {
            let wide = unsafe { wide_square_128(self.into_limbs()) };
            Self::from_limbs(reduce_128::<{ Self::ISA_MODULUS_TAIL }>(wide))
        }
    }

    macro_rules! impl_pmull_256 {
        ($field:ty) => {
            impl PmullElement for $field {
                #[inline]
                unsafe fn multiply_pmull(self, rhs: Self) -> Self {
                    let wide =
                        unsafe { wide_product_256_karatsuba(self.into_limbs(), rhs.into_limbs()) };
                    Self::from_limbs(reduce_256::<{ Self::ISA_MODULUS_TAIL }>(wide))
                }

                #[inline]
                unsafe fn square_pmull(self) -> Self {
                    let wide = unsafe { wide_square_256(self.into_limbs()) };
                    Self::from_limbs(reduce_256::<{ Self::ISA_MODULUS_TAIL }>(wide))
                }
            }
        };
    }

    impl_pmull_256!(Gf2_256HhV1);
    impl_pmull_256!(Gf2_256AltV1);

    const fn kernel_set<F: PmullElement>() -> KernelSet<F> {
        KernelSet::new(
            crate::KernelMetadata::aarch64_pmull_explicit::<F>(crate::ScheduleKind::Fixed),
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
    fn multiply<F: PmullElement>(out: &mut [F], lhs: &[F], rhs: &[F]) {
        debug_assert_eq!(out.len(), lhs.len());
        debug_assert_eq!(lhs.len(), rhs.len());
        for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
            // SAFETY: this function pointer is reachable only after the
            // selector validates NEON and PMULL support.
            *output = unsafe { left.multiply_pmull(*right) };
        }
    }

    #[inline]
    fn square<F: PmullElement>(out: &mut [F], values: &[F]) {
        debug_assert_eq!(out.len(), values.len());
        for (output, value) in out.iter_mut().zip(values) {
            // SAFETY: see `multiply`.
            *output = unsafe { value.square_pmull() };
        }
    }

    #[inline]
    fn multiply_assign<F: PmullElement>(lhs: &mut [F], rhs: &[F]) {
        debug_assert_eq!(lhs.len(), rhs.len());
        for (left, right) in lhs.iter_mut().zip(rhs) {
            // SAFETY: see `multiply`.
            *left = unsafe { left.multiply_pmull(*right) };
        }
    }

    #[inline]
    fn square_assign<F: PmullElement>(values: &mut [F]) {
        for value in values {
            // SAFETY: see `multiply`.
            *value = unsafe { value.square_pmull() };
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
        crate::KernelMetadata::aarch64_pmull_explicit::<F>(F::SCHEDULE),
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
        // SAFETY: the selector admits this private entry only after its trusted
        // NEON and PMULL checks.
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

#[target_feature(enable = "neon,aes")]
/// Computes one 64 × 64-bit polynomial product with PMULL.
///
/// # Safety
///
/// The current CPU must support NEON and PMULL (`aes` target feature).
unsafe fn clmul64(lhs: u64, rhs: u64) -> [u64; 2] {
    let product = vmull_p64(lhs, rhs);
    let bytes = product.to_le_bytes();
    [
        u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]),
        u64::from_le_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]),
    ]
}

#[target_feature(enable = "neon,aes")]
#[cfg(feature = "builtin-fields")]
/// Computes a 128 × 128-bit product with three polynomial multiplies.
///
/// # Safety
///
/// The current CPU must support NEON and PMULL.
unsafe fn wide_product_128_karatsuba(lhs: [u64; 2], rhs: [u64; 2]) -> [u64; 4] {
    let low = unsafe { clmul64(lhs[0], rhs[0]) };
    let high = unsafe { clmul64(lhs[1], rhs[1]) };
    let mixed = unsafe { clmul64(lhs[0] ^ lhs[1], rhs[0] ^ rhs[1]) };
    let middle = [mixed[0] ^ low[0] ^ high[0], mixed[1] ^ low[1] ^ high[1]];
    [low[0], low[1] ^ middle[0], high[0] ^ middle[1], high[1]]
}

#[target_feature(enable = "neon,aes")]
#[cfg(feature = "builtin-fields")]
/// Computes a 256 × 256-bit product with one outer Karatsuba level.
///
/// # Safety
///
/// The current CPU must support NEON and PMULL.
unsafe fn wide_product_256_karatsuba(lhs: [u64; 4], rhs: [u64; 4]) -> [u64; 8] {
    let low = unsafe { wide_product_128_karatsuba([lhs[0], lhs[1]], [rhs[0], rhs[1]]) };
    let high = unsafe { wide_product_128_karatsuba([lhs[2], lhs[3]], [rhs[2], rhs[3]]) };
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

#[target_feature(enable = "neon,aes")]
/// Computes a fixed-size schoolbook product for a generated profile.
///
/// # Safety
///
/// The current CPU must support NEON and PMULL and `WIDE` must equal
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

#[target_feature(enable = "neon,aes")]
/// Expands a generated-field square without cross terms.
///
/// # Safety
///
/// The current CPU must support NEON and PMULL and `WIDE` must equal
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

#[target_feature(enable = "neon,aes")]
#[cfg(feature = "builtin-fields")]
/// Expands a 128-bit square without cross terms.
///
/// # Safety
///
/// The current CPU must support NEON and PMULL.
unsafe fn wide_square_128(value: [u64; 2]) -> [u64; 4] {
    let low = unsafe { clmul64(value[0], value[0]) };
    let high = unsafe { clmul64(value[1], value[1]) };
    [low[0], low[1], high[0], high[1]]
}

#[target_feature(enable = "neon,aes")]
#[cfg(feature = "builtin-fields")]
/// Expands a 256-bit square without cross terms.
///
/// # Safety
///
/// The current CPU must support NEON and PMULL.
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
    fn karatsuba_and_square_match_schoolbook() {
        if !std::arch::is_aarch64_feature_detected!("pmull") {
            return;
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
                wide_product_schoolbook::<4, 8>(lhs, rhs)
            });
            assert_eq!(unsafe { wide_square_256(lhs) }, unsafe {
                wide_product_256_karatsuba(lhs, lhs)
            });
        }
    }
}
