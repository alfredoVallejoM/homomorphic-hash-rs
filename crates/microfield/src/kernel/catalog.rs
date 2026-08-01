//! Static catalogs implementing the Strategy and Abstract Factory patterns.

use crate::{Field, Square};

use super::KernelMetadata;

pub(crate) type BinaryKernel<F> = fn(out: &mut [F], lhs: &[F], rhs: &[F]);
pub(crate) type UnaryKernel<F> = fn(out: &mut [F], values: &[F]);
pub(crate) type BinaryAssignKernel<F> = fn(lhs: &mut [F], rhs: &[F]);
pub(crate) type UnaryAssignKernel<F> = fn(values: &mut [F]);

/// Internal immutable strategy table selected once per engine.
pub(crate) struct KernelSet<F: Field> {
    pub(crate) metadata: KernelMetadata,
    pub(crate) add: BinaryKernel<F>,
    pub(crate) multiply: BinaryKernel<F>,
    pub(crate) square: UnaryKernel<F>,
    pub(crate) multiply_assign: BinaryAssignKernel<F>,
    pub(crate) square_assign: UnaryAssignKernel<F>,
}

impl<F: Field> KernelSet<F> {
    #[cfg(feature = "portable")]
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
        }
    }
}

/// Opaque static catalog associated with a maintained field.
///
/// This type is public only because it appears in the sealed [`BuiltinField`]
/// contract. It has no public constructor or kernel accessors.
#[doc(hidden)]
pub struct KernelCatalog<F: Field> {
    portable: &'static KernelSet<F>,
}

impl<F: Field> KernelCatalog<F> {
    #[cfg(feature = "portable")]
    pub(crate) const fn portable(kernels: &'static KernelSet<F>) -> Self {
        Self { portable: kernels }
    }

    pub(crate) const fn portable_kernels(&self) -> &'static KernelSet<F> {
        self.portable
    }
}

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// Sealed capability that enables the batch engine for maintained fields.
///
/// Consumers can use this trait as a generic bound, but cannot implement it or
/// construct kernel catalogs.
#[doc(hidden)]
#[allow(private_bounds)]
pub trait BuiltinField: Field + Square + sealed::Sealed {
    /// Returns the immutable strategies certified for this field.
    #[doc(hidden)]
    fn __kernel_catalog() -> &'static KernelCatalog<Self>;
}
