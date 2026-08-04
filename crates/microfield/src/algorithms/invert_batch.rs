//! Zero-tolerant batch inversion with explicit reusable workspace.

use core::{fmt, marker::PhantomData};

use crate::{__private::PortableField, BackendId, Engine, FieldId, Invert, StaticField};

use super::{
    AlgorithmFamily, AlgorithmId, AllocationBehavior, BatchInvertWorkspace, BatchPlan,
    BitMaskViewMut, OperationKind, WorkspaceLayout, required_mask_words,
};
#[cfg(feature = "alloc")]
use super::{BitMask, OwnedBatchInvertWorkspace};

/// Immutable storage requirements for one batch-inversion plan.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BatchInvertRequirements {
    prefix_elements: usize,
    mask_words: usize,
}

impl BatchInvertRequirements {
    /// Returns the number of caller-provided field elements required.
    #[must_use]
    pub const fn prefix_elements(self) -> usize {
        self.prefix_elements
    }

    /// Returns the number of compact `u64` mask words required.
    #[must_use]
    pub const fn mask_words(self) -> usize {
        self.mask_words
    }
}

/// Failure while planning or executing batch inversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum BatchInvertError {
    /// Rounding a mask or allocation length overflowed `usize`.
    SizeOverflow,
    /// Output and input lengths differ from each other or from the plan.
    LengthMismatch {
        /// Length fixed by the plan.
        expected: usize,
        /// Output length.
        out: usize,
        /// Input length.
        input: usize,
    },
    /// The logical mask length differs from the plan.
    MaskLengthMismatch {
        /// Length fixed by the plan.
        expected: usize,
        /// Logical mask length supplied by the caller.
        actual: usize,
    },
    /// Typed workspace cannot contain all prefix products.
    WorkspaceTooSmall {
        /// Required number of field elements.
        required: usize,
        /// Supplied number of field elements.
        provided: usize,
    },
    /// The plan was created for another selected backend.
    BackendMismatch {
        /// Backend selected by the executing engine.
        expected: BackendId,
        /// Backend recorded by the plan.
        actual: BackendId,
    },
    /// Owned result, mask or workspace storage could not be reserved.
    AllocationFailed,
    /// A non-zero accumulated product unexpectedly failed to invert.
    NonInvertibleProduct,
}

impl fmt::Display for BatchInvertError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeOverflow => formatter.write_str("batch-inversion size overflow"),
            Self::LengthMismatch {
                expected,
                out,
                input,
            } => write!(
                formatter,
                "batch-inversion length mismatch: expected={expected}, out={out}, input={input}"
            ),
            Self::MaskLengthMismatch { expected, actual } => write!(
                formatter,
                "batch-inversion mask length mismatch: expected={expected}, actual={actual}"
            ),
            Self::WorkspaceTooSmall { required, provided } => write!(
                formatter,
                "batch-inversion workspace too small: required={required}, provided={provided}"
            ),
            Self::BackendMismatch { expected, actual } => write!(
                formatter,
                "batch-inversion backend mismatch: engine={expected:?}, plan={actual:?}"
            ),
            Self::AllocationFailed => formatter.write_str("batch-inversion allocation failed"),
            Self::NonInvertibleProduct => {
                formatter.write_str("non-zero batch product unexpectedly has no inverse")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BatchInvertError {}

/// Reusable, backend-bound plan for one batch length.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BatchInvertPlan<F: StaticField> {
    len: usize,
    backend: BackendId,
    field_id: FieldId,
    requirements: BatchInvertRequirements,
    field: PhantomData<F>,
}

impl<F> BatchInvertPlan<F>
where
    F: PortableField + StaticField + Invert,
{
    /// Builds and validates the immutable plan.
    ///
    /// # Errors
    ///
    /// Returns [`BatchInvertError::SizeOverflow`] if mask sizing overflows.
    pub fn new(engine: &Engine<F>, len: usize) -> Result<Self, BatchInvertError> {
        let mask_words = required_mask_words(len).map_err(|_| BatchInvertError::SizeOverflow)?;
        Ok(Self {
            len,
            backend: engine.backend_id(),
            field_id: F::spec().field_id(),
            requirements: BatchInvertRequirements {
                prefix_elements: len,
                mask_words,
            },
            field: PhantomData,
        })
    }

    /// Returns the typed workspace and mask requirements.
    #[must_use]
    pub const fn requirements(&self) -> BatchInvertRequirements {
        self.requirements
    }

    /// Executes out of place after validating every fallible precondition.
    ///
    /// Zero inputs produce zero outputs and a cleared mask bit. Non-zero inputs
    /// produce their inverse and a set bit.
    ///
    /// # Errors
    ///
    /// Returns a typed error without modifying `out` or `nonzero` when lengths,
    /// workspace or backend are incompatible.
    pub fn execute(
        &self,
        engine: &Engine<F>,
        out: &mut [F],
        values: &[F],
        nonzero: &mut BitMaskViewMut<'_>,
        workspace: &mut BatchInvertWorkspace<'_, F>,
    ) -> Result<(), BatchInvertError> {
        self.validate(engine, out.len(), values.len(), nonzero, workspace)?;
        let prefixes = workspace.prefix_storage(self.len);
        let mut inverse = build_prefixes_and_invert(values, prefixes)?;

        nonzero.clear();
        for index in (0..self.len).rev() {
            let value = values[index];
            if value.is_zero() {
                out[index] = F::ZERO;
            } else {
                let previous = if index == 0 {
                    F::ONE
                } else {
                    prefixes[index - 1]
                };
                out[index] = inverse.mul(previous);
                inverse = inverse.mul(value);
                nonzero.set(index);
            }
        }
        Ok(())
    }

    /// Executes in place using the same validated algorithm.
    ///
    /// # Errors
    ///
    /// Returns a typed error without modifying `values` or `nonzero` when a
    /// precondition fails.
    pub fn execute_assign(
        &self,
        engine: &Engine<F>,
        values: &mut [F],
        nonzero: &mut BitMaskViewMut<'_>,
        workspace: &mut BatchInvertWorkspace<'_, F>,
    ) -> Result<(), BatchInvertError> {
        self.validate(engine, values.len(), values.len(), nonzero, workspace)?;
        let prefixes = workspace.prefix_storage(self.len);
        let mut inverse = build_prefixes_and_invert(values, prefixes)?;

        nonzero.clear();
        for index in (0..self.len).rev() {
            let value = values[index];
            if value.is_zero() {
                values[index] = F::ZERO;
            } else {
                let previous = if index == 0 {
                    F::ONE
                } else {
                    prefixes[index - 1]
                };
                values[index] = inverse.mul(previous);
                inverse = inverse.mul(value);
                nonzero.set(index);
            }
        }
        Ok(())
    }

    fn validate(
        &self,
        engine: &Engine<F>,
        out_len: usize,
        input_len: usize,
        nonzero: &BitMaskViewMut<'_>,
        workspace: &BatchInvertWorkspace<'_, F>,
    ) -> Result<(), BatchInvertError> {
        if engine.backend_id() != self.backend {
            return Err(BatchInvertError::BackendMismatch {
                expected: engine.backend_id(),
                actual: self.backend,
            });
        }
        if out_len != self.len || input_len != self.len {
            return Err(BatchInvertError::LengthMismatch {
                expected: self.len,
                out: out_len,
                input: input_len,
            });
        }
        if nonzero.len() != self.len {
            return Err(BatchInvertError::MaskLengthMismatch {
                expected: self.len,
                actual: nonzero.len(),
            });
        }
        if workspace.capacity() < self.len {
            return Err(BatchInvertError::WorkspaceTooSmall {
                required: self.len,
                provided: workspace.capacity(),
            });
        }
        Ok(())
    }
}

impl<F: StaticField> BatchPlan<F> for BatchInvertPlan<F> {
    fn algorithm_id(&self) -> AlgorithmId {
        AlgorithmId::new(
            OperationKind::InvertBatch,
            AlgorithmFamily::BatchInversionMontgomery,
            1,
        )
    }

    fn logical_len(&self) -> usize {
        self.len
    }

    fn backend_id(&self) -> BackendId {
        self.backend
    }

    fn field_id(&self) -> FieldId {
        self.field_id
    }

    fn workspace_layout(&self) -> WorkspaceLayout {
        WorkspaceLayout::new(
            self.requirements.prefix_elements,
            self.requirements.mask_words,
            core::mem::align_of::<F>(),
            true,
            AllocationBehavior::CallerProvidedWorkspace,
        )
    }
}

impl<F> Engine<F>
where
    F: PortableField + StaticField + Invert,
{
    /// Inverts a batch into separate output using caller-provided storage.
    ///
    /// # Errors
    ///
    /// Returns a validation or algebraic-invariant error as documented by
    /// [`BatchInvertError`].
    pub fn invert_batch_into(
        &self,
        out: &mut [F],
        values: &[F],
        nonzero: &mut BitMaskViewMut<'_>,
        workspace: &mut BatchInvertWorkspace<'_, F>,
    ) -> Result<(), BatchInvertError> {
        BatchInvertPlan::new(self, values.len())?.execute(self, out, values, nonzero, workspace)
    }

    /// Inverts a batch in place using caller-provided storage.
    ///
    /// # Errors
    ///
    /// Returns a validation or algebraic-invariant error as documented by
    /// [`BatchInvertError`].
    pub fn invert_batch_assign(
        &self,
        values: &mut [F],
        nonzero: &mut BitMaskViewMut<'_>,
        workspace: &mut BatchInvertWorkspace<'_, F>,
    ) -> Result<(), BatchInvertError> {
        BatchInvertPlan::new(self, values.len())?.execute_assign(self, values, nonzero, workspace)
    }

    /// Allocates owned output, mask and workspace explicitly.
    ///
    /// # Errors
    ///
    /// Returns a sizing, allocation or execution error.
    #[cfg(feature = "alloc")]
    pub fn invert_batch_alloc(
        &self,
        values: &[F],
    ) -> Result<(alloc::vec::Vec<F>, BitMask), BatchInvertError> {
        let mut out = alloc::vec::Vec::new();
        out.try_reserve_exact(values.len())
            .map_err(|_| BatchInvertError::AllocationFailed)?;
        out.resize(values.len(), F::ZERO);
        let mut mask = BitMask::new(values.len()).map_err(|error| match error {
            super::BitMaskError::SizeOverflow => BatchInvertError::SizeOverflow,
            super::BitMaskError::AllocationFailed
            | super::BitMaskError::InsufficientStorage { .. } => BatchInvertError::AllocationFailed,
        })?;
        let mut workspace = OwnedBatchInvertWorkspace::new(values.len())
            .map_err(|_| BatchInvertError::AllocationFailed)?;
        self.invert_batch_into(
            &mut out,
            values,
            &mut mask.as_view_mut(),
            &mut workspace.as_workspace(),
        )?;
        Ok((out, mask))
    }
}

fn build_prefixes_and_invert<F: Invert>(
    values: &[F],
    prefixes: &mut [F],
) -> Result<F, BatchInvertError> {
    let mut accumulator = F::ONE;
    for (prefix, value) in prefixes.iter_mut().zip(values) {
        if !value.is_zero() {
            accumulator = accumulator.mul(*value);
        }
        *prefix = accumulator;
    }
    accumulator
        .invert()
        .ok_or(BatchInvertError::NonInvertibleProduct)
}
