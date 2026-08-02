//! x86-64 VPCLMUL batch strategy over pairs of independent field values.
//!
//! Safe function pointers enter this module only after the immutable selector
//! has checked `pclmulqdq`, `avx2` and `vpclmulqdq`. Each vector carry-less
//! multiplication uses the two 128-bit AVX lanes for two different field
//! values. The backend handles odd slice tails explicitly and executes one
//! `vzeroupper` at the end of each arithmetic batch call.

use core::{
    arch::x86_64::{
        __m256i, _mm256_clmulepi64_epi128, _mm256_set_epi64x, _mm256_storeu_si256, _mm256_zeroupper,
    },
    mem::MaybeUninit,
};

#[cfg(feature = "builtin-fields")]
use core::arch::x86_64::{_mm256_shuffle_epi32, _mm256_xor_si256};

#[cfg(feature = "builtin-fields")]
mod builtins {
    use super::{
        _mm256_zeroupper, wide_product_128_pair, wide_product_256_pair, wide_square_128_pair,
        wide_square_256_pair,
    };
    use crate::{
        Field, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, Square,
        binary::{reduce_128, reduce_256},
        kernel::KernelSet,
    };

    pub(crate) static GF2_128_V1_KERNELS: KernelSet<Gf2_128V1> = kernel_set::<Gf2_128V1>(64, false);
    pub(crate) static GF2_256_HH_V1_KERNELS: KernelSet<Gf2_256HhV1> =
        kernel_set::<Gf2_256HhV1>(usize::MAX, false);
    pub(crate) static GF2_256_ALT_V1_KERNELS: KernelSet<Gf2_256AltV1> =
        kernel_set::<Gf2_256AltV1>(usize::MAX, false);

    trait VpclmulElement: Field + Square {
        /// Multiplies two independent element pairs in the two AVX lanes.
        ///
        /// # Safety
        ///
        /// The current CPU must support `pclmulqdq`, `avx2` and
        /// `vpclmulqdq`.
        unsafe fn multiply_vpclmul(lhs: [Self; 2], rhs: [Self; 2]) -> [Self; 2];

        /// Squares two independent element pairs in the two AVX lanes.
        ///
        /// # Safety
        ///
        /// The current CPU must support `pclmulqdq`, `avx2` and
        /// `vpclmulqdq`.
        unsafe fn square_vpclmul(values: [Self; 2]) -> [Self; 2];
    }

    impl VpclmulElement for Gf2_128V1 {
        #[inline]
        unsafe fn multiply_vpclmul(lhs: [Self; 2], rhs: [Self; 2]) -> [Self; 2] {
            let wide = unsafe {
                wide_product_128_pair(
                    [lhs[0].into_limbs(), lhs[1].into_limbs()],
                    [rhs[0].into_limbs(), rhs[1].into_limbs()],
                )
            };
            [
                Self::from_limbs(reduce_128::<{ Self::ISA_MODULUS_TAIL }>(wide[0])),
                Self::from_limbs(reduce_128::<{ Self::ISA_MODULUS_TAIL }>(wide[1])),
            ]
        }

        #[inline]
        unsafe fn square_vpclmul(values: [Self; 2]) -> [Self; 2] {
            let wide =
                unsafe { wide_square_128_pair([values[0].into_limbs(), values[1].into_limbs()]) };
            [
                Self::from_limbs(reduce_128::<{ Self::ISA_MODULUS_TAIL }>(wide[0])),
                Self::from_limbs(reduce_128::<{ Self::ISA_MODULUS_TAIL }>(wide[1])),
            ]
        }
    }

    macro_rules! impl_vpclmul_256 {
        ($field:ty) => {
            impl VpclmulElement for $field {
                #[inline]
                unsafe fn multiply_vpclmul(lhs: [Self; 2], rhs: [Self; 2]) -> [Self; 2] {
                    let wide = unsafe {
                        wide_product_256_pair(
                            [lhs[0].into_limbs(), lhs[1].into_limbs()],
                            [rhs[0].into_limbs(), rhs[1].into_limbs()],
                        )
                    };
                    [
                        Self::from_limbs(reduce_256::<{ Self::ISA_MODULUS_TAIL }>(wide[0])),
                        Self::from_limbs(reduce_256::<{ Self::ISA_MODULUS_TAIL }>(wide[1])),
                    ]
                }

                #[inline]
                unsafe fn square_vpclmul(values: [Self; 2]) -> [Self; 2] {
                    let wide = unsafe {
                        wide_square_256_pair([values[0].into_limbs(), values[1].into_limbs()])
                    };
                    [
                        Self::from_limbs(reduce_256::<{ Self::ISA_MODULUS_TAIL }>(wide[0])),
                        Self::from_limbs(reduce_256::<{ Self::ISA_MODULUS_TAIL }>(wide[1])),
                    ]
                }
            }
        };
    }

    impl_vpclmul_256!(Gf2_256HhV1);
    impl_vpclmul_256!(Gf2_256AltV1);

    const fn kernel_set<F: VpclmulElement>(
        minimum_batch: usize,
        automatic_selection: bool,
    ) -> KernelSet<F> {
        KernelSet::new(
            crate::KernelMetadata::x86_vpclmul(minimum_batch, automatic_selection),
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
    fn multiply<F: VpclmulElement>(out: &mut [F], lhs: &[F], rhs: &[F]) {
        debug_assert_eq!(out.len(), lhs.len());
        debug_assert_eq!(lhs.len(), rhs.len());
        // SAFETY: this entry is installed only in the VPCLMUL catalog slot;
        // the selector validates all three required x86 features first.
        unsafe { multiply_impl(out, lhs, rhs) };
    }

    #[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
    unsafe fn multiply_impl<F: VpclmulElement>(out: &mut [F], lhs: &[F], rhs: &[F]) {
        let pair_count = lhs.len() / 2;
        for pair in 0..pair_count {
            let index = pair * 2;
            let product = unsafe {
                F::multiply_vpclmul([lhs[index], lhs[index + 1]], [rhs[index], rhs[index + 1]])
            };
            out[index] = product[0];
            out[index + 1] = product[1];
        }
        if !lhs.len().is_multiple_of(2) {
            let index = lhs.len() - 1;
            let product =
                unsafe { F::multiply_vpclmul([lhs[index], F::ZERO], [rhs[index], F::ZERO]) };
            out[index] = product[0];
        }
        _mm256_zeroupper();
    }

    #[inline]
    fn square<F: VpclmulElement>(out: &mut [F], values: &[F]) {
        debug_assert_eq!(out.len(), values.len());
        // SAFETY: see `multiply`.
        unsafe { square_impl(out, values) };
    }

    #[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
    unsafe fn square_impl<F: VpclmulElement>(out: &mut [F], values: &[F]) {
        let pair_count = values.len() / 2;
        for pair in 0..pair_count {
            let index = pair * 2;
            let squared = unsafe { F::square_vpclmul([values[index], values[index + 1]]) };
            out[index] = squared[0];
            out[index + 1] = squared[1];
        }
        if !values.len().is_multiple_of(2) {
            let index = values.len() - 1;
            let squared = unsafe { F::square_vpclmul([values[index], F::ZERO]) };
            out[index] = squared[0];
        }
        _mm256_zeroupper();
    }

    #[inline]
    fn multiply_assign<F: VpclmulElement>(lhs: &mut [F], rhs: &[F]) {
        debug_assert_eq!(lhs.len(), rhs.len());
        // SAFETY: see `multiply`.
        unsafe { multiply_assign_impl(lhs, rhs) };
    }

    #[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
    unsafe fn multiply_assign_impl<F: VpclmulElement>(lhs: &mut [F], rhs: &[F]) {
        let pair_count = lhs.len() / 2;
        for pair in 0..pair_count {
            let index = pair * 2;
            let product = unsafe {
                F::multiply_vpclmul([lhs[index], lhs[index + 1]], [rhs[index], rhs[index + 1]])
            };
            lhs[index] = product[0];
            lhs[index + 1] = product[1];
        }
        if !lhs.len().is_multiple_of(2) {
            let index = lhs.len() - 1;
            let product =
                unsafe { F::multiply_vpclmul([lhs[index], F::ZERO], [rhs[index], F::ZERO]) };
            lhs[index] = product[0];
        }
        _mm256_zeroupper();
    }

    #[inline]
    fn square_assign<F: VpclmulElement>(values: &mut [F]) {
        // SAFETY: see `multiply`.
        unsafe { square_assign_impl(values) };
    }

    #[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
    unsafe fn square_assign_impl<F: VpclmulElement>(values: &mut [F]) {
        let pair_count = values.len() / 2;
        for pair in 0..pair_count {
            let index = pair * 2;
            let squared = unsafe { F::square_vpclmul([values[index], values[index + 1]]) };
            values[index] = squared[0];
            values[index + 1] = squared[1];
        }
        if !values.len().is_multiple_of(2) {
            let index = values.len() - 1;
            let squared = unsafe { F::square_vpclmul([values[index], F::ZERO]) };
            values[index] = squared[0];
        }
        _mm256_zeroupper();
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
        crate::KernelMetadata::x86_vpclmul_explicit(F::SCHEDULE),
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
    // SAFETY: this entry is installed only in the verified VPCLMUL slot and
    // cannot be selected without the complete capability check.
    unsafe { verified_multiply_impl::<F, LIMBS, WIDE_LIMBS>(out, lhs, rhs) };
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
unsafe fn verified_multiply_impl<F, const LIMBS: usize, const WIDE_LIMBS: usize>(
    out: &mut [F],
    lhs: &[F],
    rhs: &[F],
) where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    let pair_count = lhs.len() / 2;
    for pair in 0..pair_count {
        let index = pair * 2;
        let wide = unsafe {
            wide_product_schoolbook_pair::<LIMBS, WIDE_LIMBS>(
                [lhs[index].__into_limbs(), lhs[index + 1].__into_limbs()],
                [rhs[index].__into_limbs(), rhs[index + 1].__into_limbs()],
            )
        };
        out[index] = F::__from_reduced_limbs(F::__reduce_wide(wide[0]));
        out[index + 1] = F::__from_reduced_limbs(F::__reduce_wide(wide[1]));
    }
    if !lhs.len().is_multiple_of(2) {
        let index = lhs.len() - 1;
        let wide = unsafe {
            wide_product_schoolbook_pair::<LIMBS, WIDE_LIMBS>(
                [lhs[index].__into_limbs(), [0; LIMBS]],
                [rhs[index].__into_limbs(), [0; LIMBS]],
            )
        };
        out[index] = F::__from_reduced_limbs(F::__reduce_wide(wide[0]));
    }
    _mm256_zeroupper();
}

#[inline]
fn verified_square<F, const LIMBS: usize, const WIDE_LIMBS: usize>(out: &mut [F], values: &[F])
where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    debug_assert_eq!(out.len(), values.len());
    // SAFETY: see `verified_multiply`.
    unsafe { verified_square_impl::<F, LIMBS, WIDE_LIMBS>(out, values) };
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
unsafe fn verified_square_impl<F, const LIMBS: usize, const WIDE_LIMBS: usize>(
    out: &mut [F],
    values: &[F],
) where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    let pair_count = values.len() / 2;
    for pair in 0..pair_count {
        let index = pair * 2;
        let wide = unsafe {
            wide_square_schoolbook_pair::<LIMBS, WIDE_LIMBS>([
                values[index].__into_limbs(),
                values[index + 1].__into_limbs(),
            ])
        };
        out[index] = F::__from_reduced_limbs(F::__reduce_wide(wide[0]));
        out[index + 1] = F::__from_reduced_limbs(F::__reduce_wide(wide[1]));
    }
    if !values.len().is_multiple_of(2) {
        let index = values.len() - 1;
        let wide = unsafe {
            wide_square_schoolbook_pair::<LIMBS, WIDE_LIMBS>([
                values[index].__into_limbs(),
                [0; LIMBS],
            ])
        };
        out[index] = F::__from_reduced_limbs(F::__reduce_wide(wide[0]));
    }
    _mm256_zeroupper();
}

#[inline]
fn verified_multiply_assign<F, const LIMBS: usize, const WIDE_LIMBS: usize>(
    lhs: &mut [F],
    rhs: &[F],
) where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    debug_assert_eq!(lhs.len(), rhs.len());
    // SAFETY: see `verified_multiply`.
    unsafe { verified_multiply_assign_impl::<F, LIMBS, WIDE_LIMBS>(lhs, rhs) };
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
unsafe fn verified_multiply_assign_impl<F, const LIMBS: usize, const WIDE_LIMBS: usize>(
    lhs: &mut [F],
    rhs: &[F],
) where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    let pair_count = lhs.len() / 2;
    for pair in 0..pair_count {
        let index = pair * 2;
        let wide = unsafe {
            wide_product_schoolbook_pair::<LIMBS, WIDE_LIMBS>(
                [lhs[index].__into_limbs(), lhs[index + 1].__into_limbs()],
                [rhs[index].__into_limbs(), rhs[index + 1].__into_limbs()],
            )
        };
        lhs[index] = F::__from_reduced_limbs(F::__reduce_wide(wide[0]));
        lhs[index + 1] = F::__from_reduced_limbs(F::__reduce_wide(wide[1]));
    }
    if !lhs.len().is_multiple_of(2) {
        let index = lhs.len() - 1;
        let wide = unsafe {
            wide_product_schoolbook_pair::<LIMBS, WIDE_LIMBS>(
                [lhs[index].__into_limbs(), [0; LIMBS]],
                [rhs[index].__into_limbs(), [0; LIMBS]],
            )
        };
        lhs[index] = F::__from_reduced_limbs(F::__reduce_wide(wide[0]));
    }
    _mm256_zeroupper();
}

#[inline]
fn verified_square_assign<F, const LIMBS: usize, const WIDE_LIMBS: usize>(values: &mut [F])
where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    // SAFETY: see `verified_multiply`.
    unsafe { verified_square_assign_impl::<F, LIMBS, WIDE_LIMBS>(values) };
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
unsafe fn verified_square_assign_impl<F, const LIMBS: usize, const WIDE_LIMBS: usize>(
    values: &mut [F],
) where
    F: crate::__private::VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    let pair_count = values.len() / 2;
    for pair in 0..pair_count {
        let index = pair * 2;
        let wide = unsafe {
            wide_square_schoolbook_pair::<LIMBS, WIDE_LIMBS>([
                values[index].__into_limbs(),
                values[index + 1].__into_limbs(),
            ])
        };
        values[index] = F::__from_reduced_limbs(F::__reduce_wide(wide[0]));
        values[index + 1] = F::__from_reduced_limbs(F::__reduce_wide(wide[1]));
    }
    if !values.len().is_multiple_of(2) {
        let index = values.len() - 1;
        let wide = unsafe {
            wide_square_schoolbook_pair::<LIMBS, WIDE_LIMBS>([
                values[index].__into_limbs(),
                [0; LIMBS],
            ])
        };
        values[index] = F::__from_reduced_limbs(F::__reduce_wide(wide[0]));
    }
    _mm256_zeroupper();
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
/// Computes two independent 64 × 64-bit carry-less products.
///
/// # Safety
///
/// The current CPU must support `pclmulqdq`, `avx2` and `vpclmulqdq`.
unsafe fn clmul64_pair(lhs: [u64; 2], rhs: [u64; 2]) -> [[u64; 2]; 2] {
    let left = _mm256_set_epi64x(0, lhs[1].cast_signed(), 0, lhs[0].cast_signed());
    let right = _mm256_set_epi64x(0, rhs[1].cast_signed(), 0, rhs[0].cast_signed());
    let product = _mm256_clmulepi64_epi128::<0x00>(left, right);
    let lanes = lanes(product);
    [[lanes[0], lanes[1]], [lanes[2], lanes[3]]]
}

#[inline]
fn lanes(value: __m256i) -> [u64; 4] {
    #[repr(align(32))]
    struct AlignedLanes([u64; 4]);

    let mut output = MaybeUninit::<AlignedLanes>::uninit();
    unsafe { _mm256_storeu_si256(output.as_mut_ptr().cast::<__m256i>(), value) };
    unsafe { output.assume_init().0 }
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
#[cfg(feature = "builtin-fields")]
unsafe fn wide_product_128_pair(lhs: [[u64; 2]; 2], rhs: [[u64; 2]; 2]) -> [[u64; 4]; 2] {
    let left = lane_pair_128(lhs);
    let right = lane_pair_128(rhs);
    let low = lanes(_mm256_clmulepi64_epi128::<0x00>(left, right));
    let high = lanes(_mm256_clmulepi64_epi128::<0x11>(left, right));
    let left_mixed = _mm256_xor_si256(left, _mm256_shuffle_epi32::<0x4e>(left));
    let right_mixed = _mm256_xor_si256(right, _mm256_shuffle_epi32::<0x4e>(right));
    let mixed = lanes(_mm256_clmulepi64_epi128::<0x00>(left_mixed, right_mixed));
    let mut output = [[0; 4]; 2];
    for (lane, lane_output) in output.iter_mut().enumerate() {
        let offset = lane * 2;
        let middle = [
            mixed[offset] ^ low[offset] ^ high[offset],
            mixed[offset + 1] ^ low[offset + 1] ^ high[offset + 1],
        ];
        *lane_output = [
            low[offset],
            low[offset + 1] ^ middle[0],
            high[offset] ^ middle[1],
            high[offset + 1],
        ];
    }
    output
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
#[cfg(feature = "builtin-fields")]
fn lane_pair_128(values: [[u64; 2]; 2]) -> __m256i {
    _mm256_set_epi64x(
        values[1][1].cast_signed(),
        values[1][0].cast_signed(),
        values[0][1].cast_signed(),
        values[0][0].cast_signed(),
    )
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
#[cfg(feature = "builtin-fields")]
unsafe fn wide_product_256_pair(lhs: [[u64; 4]; 2], rhs: [[u64; 4]; 2]) -> [[u64; 8]; 2] {
    let low = unsafe {
        wide_product_128_pair(
            [[lhs[0][0], lhs[0][1]], [lhs[1][0], lhs[1][1]]],
            [[rhs[0][0], rhs[0][1]], [rhs[1][0], rhs[1][1]]],
        )
    };
    let high = unsafe {
        wide_product_128_pair(
            [[lhs[0][2], lhs[0][3]], [lhs[1][2], lhs[1][3]]],
            [[rhs[0][2], rhs[0][3]], [rhs[1][2], rhs[1][3]]],
        )
    };
    let mixed = unsafe {
        wide_product_128_pair(
            [
                [lhs[0][0] ^ lhs[0][2], lhs[0][1] ^ lhs[0][3]],
                [lhs[1][0] ^ lhs[1][2], lhs[1][1] ^ lhs[1][3]],
            ],
            [
                [rhs[0][0] ^ rhs[0][2], rhs[0][1] ^ rhs[0][3]],
                [rhs[1][0] ^ rhs[1][2], rhs[1][1] ^ rhs[1][3]],
            ],
        )
    };
    let mut output = [[0; 8]; 2];
    for lane in 0..2 {
        output[lane][..4].copy_from_slice(&low[lane]);
        output[lane][4..].copy_from_slice(&high[lane]);
        for index in 0..4 {
            output[lane][index + 2] ^= mixed[lane][index] ^ low[lane][index] ^ high[lane][index];
        }
    }
    output
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
unsafe fn wide_product_schoolbook_pair<const LIMBS: usize, const WIDE: usize>(
    lhs: [[u64; LIMBS]; 2],
    rhs: [[u64; LIMBS]; 2],
) -> [[u64; WIDE]; 2] {
    debug_assert_eq!(WIDE, LIMBS * 2);
    let mut output = [[0; WIDE]; 2];
    for lhs_index in 0..LIMBS {
        for rhs_index in 0..LIMBS {
            let product = unsafe {
                clmul64_pair(
                    [lhs[0][lhs_index], lhs[1][lhs_index]],
                    [rhs[0][rhs_index], rhs[1][rhs_index]],
                )
            };
            for lane in 0..2 {
                output[lane][lhs_index + rhs_index] ^= product[lane][0];
                output[lane][lhs_index + rhs_index + 1] ^= product[lane][1];
            }
        }
    }
    output
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
unsafe fn wide_square_schoolbook_pair<const LIMBS: usize, const WIDE: usize>(
    values: [[u64; LIMBS]; 2],
) -> [[u64; WIDE]; 2] {
    debug_assert_eq!(WIDE, LIMBS * 2);
    let mut output = [[0; WIDE]; 2];
    for index in 0..LIMBS {
        let product = unsafe {
            clmul64_pair(
                [values[0][index], values[1][index]],
                [values[0][index], values[1][index]],
            )
        };
        for lane in 0..2 {
            output[lane][index * 2] = product[lane][0];
            output[lane][index * 2 + 1] = product[lane][1];
        }
    }
    output
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
#[cfg(feature = "builtin-fields")]
unsafe fn wide_square_128_pair(values: [[u64; 2]; 2]) -> [[u64; 4]; 2] {
    let packed = lane_pair_128(values);
    let low = lanes(_mm256_clmulepi64_epi128::<0x00>(packed, packed));
    let high = lanes(_mm256_clmulepi64_epi128::<0x11>(packed, packed));
    [
        [low[0], low[1], high[0], high[1]],
        [low[2], low[3], high[2], high[3]],
    ]
}

#[target_feature(enable = "pclmulqdq,avx2,vpclmulqdq")]
#[cfg(feature = "builtin-fields")]
unsafe fn wide_square_256_pair(values: [[u64; 4]; 2]) -> [[u64; 8]; 2] {
    let low = unsafe {
        wide_square_128_pair([[values[0][0], values[0][1]], [values[1][0], values[1][1]]])
    };
    let high = unsafe {
        wide_square_128_pair([[values[0][2], values[0][3]], [values[1][2], values[1][3]]])
    };
    [
        [
            low[0][0], low[0][1], low[0][2], low[0][3], high[0][0], high[0][1], high[0][2],
            high[0][3],
        ],
        [
            low[1][0], low[1][1], low[1][2], low[1][3], high[1][0], high[1][1], high[1][2],
            high[1][3],
        ],
    ]
}
