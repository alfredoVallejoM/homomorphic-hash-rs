//! Static catalogs implementing the Strategy and Abstract Factory patterns.

use crate::Field;
#[cfg(feature = "portable")]
use crate::Square;

use super::{KernelMetadata, PackedKernelSet};

pub(crate) type BinaryKernel<F> = fn(out: &mut [F], lhs: &[F], rhs: &[F]);
pub(crate) type UnaryKernel<F> = fn(out: &mut [F], values: &[F]);
pub(crate) type BinaryAssignKernel<F> = fn(lhs: &mut [F], rhs: &[F]);
pub(crate) type UnaryAssignKernel<F> = fn(values: &mut [F]);

/// Internal immutable strategy table selected once per engine.
#[cfg_attr(not(feature = "portable"), allow(dead_code))]
pub(crate) struct KernelSet<F: Field> {
    pub(crate) metadata: KernelMetadata,
    pub(crate) add: BinaryKernel<F>,
    pub(crate) multiply: BinaryKernel<F>,
    pub(crate) square: UnaryKernel<F>,
    pub(crate) multiply_assign: BinaryAssignKernel<F>,
    pub(crate) square_assign: UnaryAssignKernel<F>,
    pub(crate) packed: PackedKernelSet<F>,
}

impl<F: Field> KernelSet<F> {
    pub(crate) const fn new(
        metadata: KernelMetadata,
        add: BinaryKernel<F>,
        multiply: BinaryKernel<F>,
        square: UnaryKernel<F>,
        multiply_assign: BinaryAssignKernel<F>,
        square_assign: UnaryAssignKernel<F>,
    ) -> Self {
        Self {
            metadata,
            add,
            multiply,
            square,
            multiply_assign,
            square_assign,
            packed: PackedKernelSet::Aos,
        }
    }

    #[cfg_attr(
        not(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64")),
        allow(dead_code)
    )]
    #[must_use]
    pub(crate) const fn with_packed(mut self, packed: PackedKernelSet<F>) -> Self {
        self.packed = packed;
        self
    }
}

/// Opaque static catalog associated with a maintained field.
///
/// This type is public only because it appears in the sealed [`BuiltinField`]
/// contract. It has no public constructor or kernel accessors.
#[doc(hidden)]
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct KernelCatalog<F: Field> {
    portable: &'static KernelSet<F>,
    x86_pclmul: Option<&'static KernelSet<F>>,
    x86_vpclmul: Option<&'static KernelSet<F>>,
    aarch64_pmull: Option<&'static KernelSet<F>>,
    x86_prime_avx2: Option<&'static KernelSet<F>>,
    x86_prime_bmi2: Option<&'static KernelSet<F>>,
}

impl<F: Field> KernelCatalog<F> {
    pub(crate) const fn portable(kernels: &'static KernelSet<F>) -> Self {
        assert!(matches!(
            kernels.metadata.backend(),
            super::BackendId::Portable
        ));
        Self {
            portable: kernels,
            x86_pclmul: None,
            x86_vpclmul: None,
            aarch64_pmull: None,
            x86_prime_avx2: None,
            x86_prime_bmi2: None,
        }
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) const fn with_x86_pclmul(mut self, kernels: &'static KernelSet<F>) -> Self {
        assert!(matches!(
            kernels.metadata.backend(),
            super::BackendId::X86Pclmul
        ));
        self.x86_pclmul = Some(kernels);
        self
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) const fn with_x86_vpclmul(mut self, kernels: &'static KernelSet<F>) -> Self {
        assert!(matches!(
            kernels.metadata.backend(),
            super::BackendId::X86Vpclmul
        ));
        self.x86_vpclmul = Some(kernels);
        self
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) const fn with_aarch64_pmull(mut self, kernels: &'static KernelSet<F>) -> Self {
        assert!(matches!(
            kernels.metadata.backend(),
            super::BackendId::Aarch64Pmull
        ));
        self.aarch64_pmull = Some(kernels);
        self
    }

    #[cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]
    #[must_use]
    pub(crate) const fn with_x86_prime_avx2(mut self, kernels: &'static KernelSet<F>) -> Self {
        assert!(matches!(
            kernels.metadata.backend(),
            super::BackendId::X86PrimeAvx2
        ));
        self.x86_prime_avx2 = Some(kernels);
        self
    }

    #[cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]
    #[must_use]
    pub(crate) const fn with_x86_prime_bmi2(mut self, kernels: &'static KernelSet<F>) -> Self {
        assert!(matches!(
            kernels.metadata.backend(),
            super::BackendId::X86PrimeBmi2
        ));
        self.x86_prime_bmi2 = Some(kernels);
        self
    }

    #[cfg(feature = "portable")]
    pub(crate) const fn get(&self, backend: super::BackendId) -> Option<&'static KernelSet<F>> {
        match backend {
            super::BackendId::Portable => Some(self.portable),
            super::BackendId::X86Pclmul => self.x86_pclmul,
            super::BackendId::X86Vpclmul => self.x86_vpclmul,
            super::BackendId::Aarch64Pmull => self.aarch64_pmull,
            super::BackendId::X86PrimeAvx2 => self.x86_prime_avx2,
            super::BackendId::X86PrimeBmi2 => self.x86_prime_bmi2,
        }
    }
}

#[cfg(feature = "portable")]
pub(crate) mod sealed {
    pub trait Sealed {}
}

/// Sealed capability that enables the batch engine for maintained fields.
///
/// Consumers can use this trait as a generic bound, but cannot implement it or
/// construct kernel catalogs.
#[doc(hidden)]
#[cfg(feature = "portable")]
#[allow(private_bounds)]
pub trait BuiltinField: Field + Square + crate::__private::PortableField + sealed::Sealed {
    /// Returns the immutable strategies certified for this field.
    #[doc(hidden)]
    fn __kernel_catalog() -> &'static KernelCatalog<Self>;
}
