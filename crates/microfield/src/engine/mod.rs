//! Batch execution façade.
//!
//! Kernel tables remain an internal construction detail:
//!
//! ```compile_fail
//! use microfield::KernelSet;
//! ```
//!
//! The opaque catalog type cannot be constructed by consumers either:
//!
//! ```compile_fail
//! use microfield::{Gf2_256HhV1, KernelCatalog};
//!
//! let _ = KernelCatalog::<Gf2_256HhV1> {};
//! ```
//!
//! CPU feature snapshots cannot be forged by consumers:
//!
//! ```compile_fail
//! use microfield::{Architecture, CpuCapabilities};
//!
//! let _ = CpuCapabilities {
//!     architecture: Architecture::X86_64,
//!     features: u8::MAX,
//! };
//! ```

mod batch;
mod builder;
mod capabilities;
mod policy;

use crate::{__private::PortableField, BackendId, BatchError, KernelMetadata, kernel::KernelSet};

pub use builder::{EngineBuildError, EngineBuilder};
pub use capabilities::{Architecture, CpuCapabilities};
pub use policy::ExecutionPolicy;

/// Immutable batch execution façade for any statically defined field.
#[derive(Clone, Copy)]
pub struct Engine<F: PortableField> {
    kernels: &'static KernelSet<F>,
    policy: ExecutionPolicy,
    expected_batch: Option<usize>,
}

impl<F: PortableField> Engine<F> {
    pub(crate) const fn from_selection(
        kernels: &'static KernelSet<F>,
        policy: ExecutionPolicy,
        expected_batch: Option<usize>,
    ) -> Self {
        Self {
            kernels,
            policy,
            expected_batch,
        }
    }

    /// Creates an engine pinned to the portable strategy.
    #[must_use]
    pub fn portable() -> Self {
        Self::from_selection(
            F::__portable_strategy().kernels(),
            ExecutionPolicy::PortableOnly,
            None,
        )
    }

    /// Creates a configurable strategy builder.
    #[must_use]
    pub const fn builder() -> EngineBuilder<F> {
        EngineBuilder::new()
    }

    /// Returns the strategy selected when the engine was built.
    #[must_use]
    pub const fn backend_id(&self) -> BackendId {
        self.kernels.metadata.backend()
    }

    /// Returns immutable strategy diagnostics.
    #[must_use]
    pub const fn metadata(&self) -> &KernelMetadata {
        &self.kernels.metadata
    }

    /// Returns the policy used to construct the engine.
    #[must_use]
    pub const fn policy(&self) -> ExecutionPolicy {
        self.policy
    }

    /// Returns the advisory batch length supplied during selection.
    #[must_use]
    pub const fn expected_batch(&self) -> Option<usize> {
        self.expected_batch
    }

    /// Adds two batches into a distinct output slice.
    ///
    /// # Errors
    ///
    /// Returns [`BatchError::LengthMismatch`] before writing when slice lengths
    /// differ.
    #[inline]
    pub fn add_into(&self, out: &mut [F], lhs: &[F], rhs: &[F]) -> Result<(), BatchError> {
        batch::validate_binary(out, lhs, rhs)?;
        (self.kernels.add)(out, lhs, rhs);
        Ok(())
    }

    /// Multiplies two batches into a distinct output slice.
    ///
    /// # Errors
    ///
    /// Returns [`BatchError::LengthMismatch`] before writing when slice lengths
    /// differ.
    #[inline]
    pub fn mul_into(&self, out: &mut [F], lhs: &[F], rhs: &[F]) -> Result<(), BatchError> {
        batch::validate_binary(out, lhs, rhs)?;
        (self.kernels.multiply)(out, lhs, rhs);
        Ok(())
    }

    /// Squares one batch into a distinct output slice.
    ///
    /// # Errors
    ///
    /// Returns [`BatchError::LengthMismatch`] before writing when slice lengths
    /// differ.
    #[inline]
    pub fn square_into(&self, out: &mut [F], values: &[F]) -> Result<(), BatchError> {
        batch::validate_unary(out, values)?;
        (self.kernels.square)(out, values);
        Ok(())
    }

    /// Multiplies a batch in place by a right-hand batch.
    ///
    /// # Errors
    ///
    /// Returns [`BatchError::LengthMismatch`] before writing when slice lengths
    /// differ.
    #[inline]
    pub fn mul_assign(&self, lhs: &mut [F], rhs: &[F]) -> Result<(), BatchError> {
        batch::validate_binary_assign(lhs, rhs)?;
        (self.kernels.multiply_assign)(lhs, rhs);
        Ok(())
    }

    /// Squares every value in a batch in place.
    #[inline]
    pub fn square_assign(&self, values: &mut [F]) {
        (self.kernels.square_assign)(values);
    }
}

impl<F: PortableField> Default for Engine<F> {
    fn default() -> Self {
        Self::portable()
    }
}
