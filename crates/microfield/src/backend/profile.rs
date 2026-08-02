//! Safe runtime bridge for target-neutral verified generated profiles.

use core::marker::PhantomData;

#[cfg(feature = "portable")]
use crate::kernel::KernelSet;
use crate::{
    __private::{PortableStrategy, VerifiedBinaryIsaField},
    kernel::KernelCatalog,
};

/// Opaque strategies attached to one generator-certified field profile.
///
/// Generated code can construct this value but cannot provide function
/// pointers or execute target-feature intrinsics. The architecture adapters
/// remain selected and owned by the Microfield runtime.
#[doc(hidden)]
pub struct VerifiedIsaStrategy<F, const LIMBS: usize, const WIDE_LIMBS: usize>
where
    F: VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    #[cfg(all(feature = "portable", target_arch = "x86_64"))]
    x86_pclmul: KernelSet<F>,
    #[cfg(all(feature = "portable", target_arch = "x86_64"))]
    x86_vpclmul: KernelSet<F>,
    #[cfg(all(feature = "portable", target_arch = "aarch64"))]
    aarch64_pmull: KernelSet<F>,
    marker: PhantomData<fn() -> F>,
}

impl<F, const LIMBS: usize, const WIDE_LIMBS: usize> VerifiedIsaStrategy<F, LIMBS, WIDE_LIMBS>
where
    F: VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    /// Constructs only Microfield-owned ISA adapters with the generated
    /// profile's scheduling classification.
    ///
    /// # Panics
    ///
    /// Panics at compile time if generated code supplies an empty profile,
    /// zero limbs or a wide product other than exactly twice the input width.
    #[must_use]
    pub const fn new() -> Self {
        assert!(LIMBS > 0);
        assert!(WIDE_LIMBS == LIMBS * 2);
        assert!(!F::PROFILE_DIGEST.is_empty());
        Self {
            #[cfg(all(feature = "portable", target_arch = "x86_64"))]
            x86_pclmul: super::x86_pclmul::verified_kernel_set::<F, LIMBS, WIDE_LIMBS>(),
            #[cfg(all(feature = "portable", target_arch = "x86_64"))]
            x86_vpclmul: super::x86_vpclmul::verified_kernel_set::<F, LIMBS, WIDE_LIMBS>(),
            #[cfg(all(feature = "portable", target_arch = "aarch64"))]
            aarch64_pmull: super::aarch64_pmull::verified_kernel_set::<F, LIMBS, WIDE_LIMBS>(),
            marker: PhantomData,
        }
    }

    /// Combines the safe portable strategy with target-compatible ISA slots.
    #[must_use]
    pub fn __kernel_catalog(
        &'static self,
        portable: &'static PortableStrategy<F>,
    ) -> KernelCatalog<F> {
        let catalog = KernelCatalog::portable(portable.kernels());
        #[cfg(all(feature = "portable", target_arch = "x86_64"))]
        {
            catalog
                .with_x86_pclmul(&self.x86_pclmul)
                .with_x86_vpclmul(&self.x86_vpclmul)
        }
        #[cfg(all(feature = "portable", target_arch = "aarch64"))]
        {
            catalog.with_aarch64_pmull(&self.aarch64_pmull)
        }
        #[cfg(any(
            not(feature = "portable"),
            not(any(target_arch = "x86_64", target_arch = "aarch64"))
        ))]
        {
            catalog
        }
    }
}

impl<F, const LIMBS: usize, const WIDE_LIMBS: usize> Default
    for VerifiedIsaStrategy<F, LIMBS, WIDE_LIMBS>
where
    F: VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>,
{
    fn default() -> Self {
        Self::new()
    }
}
