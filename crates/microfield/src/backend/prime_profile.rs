//! Safe runtime bridge for generated radix-64 Montgomery fields.

// Bounds checked immediately before each narrowing prove that reciprocals fit
// their advertised lane widths.
#![allow(clippy::cast_possible_truncation)]

use core::marker::PhantomData;

#[cfg(all(feature = "portable", target_arch = "x86_64"))]
use crate::kernel::KernelSet;
use crate::{
    __private::{
        PortableStrategy, VerifiedPrimeCanonical8Field, VerifiedPrimeCanonical16Field,
        VerifiedPrimeCanonical32Field, VerifiedPrimeMontgomery64Field,
    },
    PrimeKernelMetadata,
    kernel::KernelCatalog,
};

/// Opaque AVX2 candidate for a generator-certified canonical-byte prime.
///
/// The adapter processes 32 independent residues per full tile. It remains
/// explicit because an external field profile has no runtime-owned benchmark
/// evidence, even when its representation is structurally compatible.
#[doc(hidden)]
pub struct VerifiedPrimeSimd8Strategy<F>
where
    F: VerifiedPrimeCanonical8Field,
{
    #[cfg(all(feature = "portable", target_arch = "x86_64"))]
    x86_avx2: KernelSet<F>,
    marker: PhantomData<fn() -> F>,
}

/// Opaque AVX2 candidate for a generator-certified canonical-`u32` prime.
///
/// Eight independent residues are retained as persistent `u32` lanes. The
/// profile is deliberately explicit until a concrete field clears the F4.7
/// correctness and performance gates.
#[doc(hidden)]
pub struct VerifiedPrimeSimd32Strategy<F>
where
    F: VerifiedPrimeCanonical32Field,
{
    #[cfg(all(feature = "portable", target_arch = "x86_64"))]
    x86_avx2: KernelSet<F>,
    marker: PhantomData<fn() -> F>,
}

impl<F> VerifiedPrimeSimd32Strategy<F>
where
    F: VerifiedPrimeCanonical32Field,
{
    /// Constructs Microfield's layout-independent 32-bit AVX2 candidate.
    ///
    /// # Panics
    ///
    /// Panics at compile time if the modulus, reciprocal or metadata violates
    /// the bounded 32-bit Barrett profile.
    #[must_use]
    pub const fn new(prime: PrimeKernelMetadata) -> Self {
        assert!(F::__MODULUS >= 3 && F::__MODULUS <= 4_294_967_291);
        assert!(F::__MODULUS & 1 == 1);
        assert!(F::__BARRETT_RECIPROCAL == u64::MAX / F::__MODULUS);
        assert!(matches!(
            prime.representation(),
            crate::PrimeRepresentationKind::CanonicalResidue
        ));
        assert!(matches!(
            prime.reduction(),
            crate::PrimeReductionKind::Barrett
        ));
        assert!(prime.lanes() == 8);
        assert!(!prime.requires_packing());
        Self {
            #[cfg(all(feature = "portable", target_arch = "x86_64"))]
            x86_avx2: super::x86_prime::verified_canonical32_kernel_set::<F>(prime),
            marker: PhantomData,
        }
    }

    /// Combines portable arithmetic with the explicit AVX2 candidate.
    #[must_use]
    pub fn __kernel_catalog(
        &'static self,
        portable: &'static PortableStrategy<F>,
    ) -> KernelCatalog<F> {
        let catalog = KernelCatalog::portable(portable.kernels());
        #[cfg(all(feature = "portable", target_arch = "x86_64"))]
        {
            catalog.with_x86_prime_avx2(&self.x86_avx2)
        }
        #[cfg(any(not(feature = "portable"), not(target_arch = "x86_64")))]
        {
            catalog
        }
    }
}

impl<F> VerifiedPrimeSimd8Strategy<F>
where
    F: VerifiedPrimeCanonical8Field,
{
    /// Constructs Microfield's layout-independent 8-bit AVX2 strategy.
    ///
    /// # Panics
    ///
    /// Panics at compile time if the modulus, reciprocal or kernel metadata
    /// does not satisfy the certified 8-bit Barrett profile.
    #[must_use]
    pub const fn new(prime: PrimeKernelMetadata) -> Self {
        assert!(F::__MODULUS >= 3 && F::__MODULUS <= 251);
        assert!(F::__MODULUS & 1 == 1);
        assert!(F::__BARRETT_RECIPROCAL == (65_536_u32 / F::__MODULUS as u32) as u16);
        assert!(matches!(
            prime.representation(),
            crate::PrimeRepresentationKind::CanonicalResidue
        ));
        assert!(matches!(
            prime.reduction(),
            crate::PrimeReductionKind::Barrett
        ));
        assert!(prime.lanes() == 32);
        assert!(!prime.requires_packing());
        Self {
            #[cfg(all(feature = "portable", target_arch = "x86_64"))]
            x86_avx2: super::x86_prime::verified_canonical8_kernel_set::<F>(prime),
            marker: PhantomData,
        }
    }

    /// Combines portable arithmetic with the explicit AVX2 candidate.
    #[must_use]
    pub fn __kernel_catalog(
        &'static self,
        portable: &'static PortableStrategy<F>,
    ) -> KernelCatalog<F> {
        let catalog = KernelCatalog::portable(portable.kernels());
        #[cfg(all(feature = "portable", target_arch = "x86_64"))]
        {
            catalog.with_x86_prime_avx2(&self.x86_avx2)
        }
        #[cfg(any(not(feature = "portable"), not(target_arch = "x86_64")))]
        {
            catalog
        }
    }
}

/// Opaque AVX2 candidate for a generator-certified canonical-`u16` prime.
///
/// Sixteen independent residues are widened to `u32` lanes per full tile.
/// Like the byte strategy, this candidate is never promoted automatically
/// without a versioned calibration for the concrete generated field.
#[doc(hidden)]
pub struct VerifiedPrimeSimd16Strategy<F>
where
    F: VerifiedPrimeCanonical16Field,
{
    #[cfg(all(feature = "portable", target_arch = "x86_64"))]
    x86_avx2: KernelSet<F>,
    marker: PhantomData<fn() -> F>,
}

impl<F> VerifiedPrimeSimd16Strategy<F>
where
    F: VerifiedPrimeCanonical16Field,
{
    /// Constructs Microfield's layout-independent 16-bit AVX2 strategy.
    ///
    /// # Panics
    ///
    /// Panics at compile time if the modulus, reciprocal or metadata is not a
    /// supported canonical 16-bit Barrett profile.
    #[must_use]
    pub const fn new(prime: PrimeKernelMetadata) -> Self {
        assert!(F::__MODULUS >= 3 && F::__MODULUS <= 65_521);
        assert!(F::__MODULUS & 1 == 1);
        assert!(F::__BARRETT_RECIPROCAL == ((1_u64 << 32) / F::__MODULUS as u64) as u32);
        assert!(matches!(
            prime.representation(),
            crate::PrimeRepresentationKind::CanonicalResidue
        ));
        assert!(matches!(
            prime.reduction(),
            crate::PrimeReductionKind::Barrett
        ));
        assert!(prime.lanes() == 16);
        assert!(!prime.requires_packing());
        Self {
            #[cfg(all(feature = "portable", target_arch = "x86_64"))]
            x86_avx2: super::x86_prime::verified_canonical16_kernel_set::<F>(prime),
            marker: PhantomData,
        }
    }

    /// Combines portable arithmetic with the explicit AVX2 candidate.
    #[must_use]
    pub fn __kernel_catalog(
        &'static self,
        portable: &'static PortableStrategy<F>,
    ) -> KernelCatalog<F> {
        let catalog = KernelCatalog::portable(portable.kernels());
        #[cfg(all(feature = "portable", target_arch = "x86_64"))]
        {
            catalog.with_x86_prime_avx2(&self.x86_avx2)
        }
        #[cfg(any(not(feature = "portable"), not(target_arch = "x86_64")))]
        {
            catalog
        }
    }
}

/// Opaque BMI2 candidate attached to one generator-certified prime field.
///
/// Constructing this value proves structural compatibility and delegates the
/// complete fixed-schedule reduction to Microfield. Its BMI2 slot remains
/// explicit until a future versioned calibration promotes the concrete field
/// and batch region.
#[doc(hidden)]
pub struct VerifiedPrimeIsaStrategy<F, const LIMBS: usize, const WIDE_LIMBS: usize>
where
    F: VerifiedPrimeMontgomery64Field<LIMBS, WIDE_LIMBS>,
{
    #[cfg(all(feature = "portable", target_arch = "x86_64"))]
    x86_bmi2: KernelSet<F>,
    marker: PhantomData<fn() -> F>,
}

impl<F, const LIMBS: usize, const WIDE_LIMBS: usize> VerifiedPrimeIsaStrategy<F, LIMBS, WIDE_LIMBS>
where
    F: VerifiedPrimeMontgomery64Field<LIMBS, WIDE_LIMBS>,
{
    /// Constructs only Microfield-owned BMI2 adapters.
    ///
    /// # Panics
    ///
    /// Panics at compile time if the generated product is not exactly twice
    /// the input width or if its prime metadata is not Montgomery radix 64.
    #[must_use]
    pub const fn new(prime: PrimeKernelMetadata) -> Self {
        assert!(LIMBS > 0);
        assert!(WIDE_LIMBS == LIMBS * 2);
        match prime.representation() {
            crate::PrimeRepresentationKind::Montgomery { radix_bits, limbs } => {
                assert!(radix_bits == 64);
                assert!(limbs as usize == LIMBS);
            }
            crate::PrimeRepresentationKind::CanonicalResidue => {
                panic!("BMI2 requires radix-64 Montgomery representation");
            }
        }
        assert!(matches!(
            prime.reduction(),
            crate::PrimeReductionKind::Montgomery
        ));
        Self {
            #[cfg(all(feature = "portable", target_arch = "x86_64"))]
            x86_bmi2: super::x86_prime::verified_radix64_kernel_set::<F, LIMBS, WIDE_LIMBS>(prime),
            marker: PhantomData,
        }
    }

    /// Combines a safe portable strategy with the explicit BMI2 candidate.
    #[must_use]
    pub fn __kernel_catalog(
        &'static self,
        portable: &'static PortableStrategy<F>,
    ) -> KernelCatalog<F> {
        let catalog = KernelCatalog::portable(portable.kernels());
        #[cfg(all(feature = "portable", target_arch = "x86_64"))]
        {
            catalog.with_x86_prime_bmi2(&self.x86_bmi2)
        }
        #[cfg(any(not(feature = "portable"), not(target_arch = "x86_64")))]
        {
            catalog
        }
    }
}

#[cfg(all(test, feature = "portable", target_arch = "x86_64"))]
mod tests {
    use super::*;
    use crate::{BackendId, Fp256GenericV1, PrimeReductionKind, PrimeRepresentationKind};

    const PRIME_METADATA: PrimeKernelMetadata = PrimeKernelMetadata::__from_generated(
        PrimeRepresentationKind::Montgomery {
            radix_bits: 64,
            limbs: 4,
        },
        PrimeReductionKind::Montgomery,
        crate::RangeContract::__from_generated(1, 1, 512),
        crate::RangeContract::__from_generated(1, 1, 512),
        1,
        false,
    );
    static STRATEGY: VerifiedPrimeIsaStrategy<Fp256GenericV1, 4, 8> =
        VerifiedPrimeIsaStrategy::new(PRIME_METADATA);

    #[test]
    fn generated_prime_bridge_builds_only_the_explicit_fixed_candidate() {
        let portable = <Fp256GenericV1 as crate::__private::PortableField>::__portable_strategy();
        let catalog = STRATEGY.__kernel_catalog(portable);
        let bmi2 = catalog.get(BackendId::X86PrimeBmi2).unwrap();
        assert!(!bmi2.metadata.automatic_selection());
        assert_eq!(bmi2.metadata.schedule(), crate::ScheduleKind::Fixed);
        assert_eq!(bmi2.metadata.prime(), Some(&PRIME_METADATA));
    }
}
