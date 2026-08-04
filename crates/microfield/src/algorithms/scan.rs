//! Deterministic prefix and suffix product scans.

use core::{fmt, marker::PhantomData};

use crate::{__private::PortableField, BackendId, Engine, Field, FieldId, StaticField};

use super::{
    AlgorithmFamily, AlgorithmId, AllocationBehavior, BatchPlan, OperationKind, WorkspaceLayout,
};

/// Direction in which a product scan accumulates values.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ScanDirection {
    /// Accumulate from index zero towards the end.
    Prefix,
    /// Accumulate from the final index towards zero.
    Suffix,
}

/// Whether each output includes the input at the same index.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ScanMode {
    /// Include the current input value.
    Inclusive,
    /// Exclude the current input value; the boundary output is one.
    Exclusive,
}

/// Failure while executing a product scan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ScanError {
    /// Output and input lengths differ from each other or from the plan.
    LengthMismatch {
        /// Length fixed by the plan.
        expected: usize,
        /// Output length.
        out: usize,
        /// Input length.
        input: usize,
    },
    /// The plan was created for another selected backend.
    BackendMismatch {
        /// Backend selected by the executing engine.
        expected: BackendId,
        /// Backend recorded by the plan.
        actual: BackendId,
    },
}

impl fmt::Display for ScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch {
                expected,
                out,
                input,
            } => write!(
                formatter,
                "product-scan length mismatch: expected={expected}, out={out}, input={input}"
            ),
            Self::BackendMismatch { expected, actual } => write!(
                formatter,
                "product-scan backend mismatch: engine={expected:?}, plan={actual:?}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ScanError {}

/// Immutable, reusable plan for a product scan.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProductScanPlan<F: StaticField> {
    len: usize,
    direction: ScanDirection,
    mode: ScanMode,
    backend: BackendId,
    field_id: FieldId,
    field: PhantomData<F>,
}

impl<F> ProductScanPlan<F>
where
    F: PortableField + StaticField,
{
    /// Creates a sequential, allocation-free scan plan.
    #[must_use]
    pub fn new(engine: &Engine<F>, len: usize, direction: ScanDirection, mode: ScanMode) -> Self {
        Self {
            len,
            direction,
            mode,
            backend: engine.backend_id(),
            field_id: F::spec().field_id(),
            field: PhantomData,
        }
    }

    /// Returns the accumulation direction.
    #[must_use]
    pub const fn direction(&self) -> ScanDirection {
        self.direction
    }

    /// Returns whether the scan is inclusive or exclusive.
    #[must_use]
    pub const fn mode(&self) -> ScanMode {
        self.mode
    }

    /// Executes out of place after validating every fallible precondition.
    ///
    /// # Errors
    ///
    /// Returns a typed error without modifying `out` when the lengths or
    /// backend are incompatible with this plan.
    pub fn execute(
        &self,
        engine: &Engine<F>,
        out: &mut [F],
        values: &[F],
    ) -> Result<(), ScanError> {
        self.validate(engine, out.len(), values.len())?;
        scan_into(out, values, self.direction, self.mode);
        Ok(())
    }

    /// Executes in place without scratch storage.
    ///
    /// # Errors
    ///
    /// Returns a typed error without modifying `values` when the length or
    /// backend is incompatible with this plan.
    pub fn execute_assign(&self, engine: &Engine<F>, values: &mut [F]) -> Result<(), ScanError> {
        self.validate(engine, values.len(), values.len())?;
        scan_assign(values, self.direction, self.mode);
        Ok(())
    }

    fn validate(
        &self,
        engine: &Engine<F>,
        out_len: usize,
        input_len: usize,
    ) -> Result<(), ScanError> {
        if engine.backend_id() != self.backend {
            return Err(ScanError::BackendMismatch {
                expected: engine.backend_id(),
                actual: self.backend,
            });
        }
        if out_len != self.len || input_len != self.len {
            return Err(ScanError::LengthMismatch {
                expected: self.len,
                out: out_len,
                input: input_len,
            });
        }
        Ok(())
    }
}

impl<F: StaticField> BatchPlan<F> for ProductScanPlan<F> {
    fn algorithm_id(&self) -> AlgorithmId {
        AlgorithmId::new(
            match self.direction {
                ScanDirection::Prefix => OperationKind::PrefixProducts,
                ScanDirection::Suffix => OperationKind::SuffixProducts,
            },
            AlgorithmFamily::SequentialScan,
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
        WorkspaceLayout::new(0, 0, 1, true, AllocationBehavior::None)
    }
}

impl<F> Engine<F>
where
    F: PortableField + StaticField,
{
    /// Computes inclusive prefix products into distinct output.
    ///
    /// # Errors
    ///
    /// Returns [`ScanError::LengthMismatch`] before modifying `out` when the
    /// slice lengths differ.
    pub fn prefix_products_into(&self, out: &mut [F], values: &[F]) -> Result<(), ScanError> {
        ProductScanPlan::new(
            self,
            values.len(),
            ScanDirection::Prefix,
            ScanMode::Inclusive,
        )
        .execute(self, out, values)
    }

    /// Computes exclusive prefix products into distinct output.
    ///
    /// # Errors
    ///
    /// Returns [`ScanError::LengthMismatch`] before modifying `out` when the
    /// slice lengths differ.
    pub fn exclusive_prefix_products_into(
        &self,
        out: &mut [F],
        values: &[F],
    ) -> Result<(), ScanError> {
        ProductScanPlan::new(
            self,
            values.len(),
            ScanDirection::Prefix,
            ScanMode::Exclusive,
        )
        .execute(self, out, values)
    }

    /// Computes inclusive suffix products into distinct output.
    ///
    /// # Errors
    ///
    /// Returns [`ScanError::LengthMismatch`] before modifying `out` when the
    /// slice lengths differ.
    pub fn suffix_products_into(&self, out: &mut [F], values: &[F]) -> Result<(), ScanError> {
        ProductScanPlan::new(
            self,
            values.len(),
            ScanDirection::Suffix,
            ScanMode::Inclusive,
        )
        .execute(self, out, values)
    }

    /// Computes exclusive suffix products into distinct output.
    ///
    /// # Errors
    ///
    /// Returns [`ScanError::LengthMismatch`] before modifying `out` when the
    /// slice lengths differ.
    pub fn exclusive_suffix_products_into(
        &self,
        out: &mut [F],
        values: &[F],
    ) -> Result<(), ScanError> {
        ProductScanPlan::new(
            self,
            values.len(),
            ScanDirection::Suffix,
            ScanMode::Exclusive,
        )
        .execute(self, out, values)
    }
}

fn scan_into<F: Field>(out: &mut [F], values: &[F], direction: ScanDirection, mode: ScanMode) {
    let mut accumulator = F::ONE;
    match direction {
        ScanDirection::Prefix => {
            for (output, value) in out.iter_mut().zip(values) {
                if mode == ScanMode::Exclusive {
                    *output = accumulator;
                }
                accumulator = accumulator.mul(*value);
                if mode == ScanMode::Inclusive {
                    *output = accumulator;
                }
            }
        }
        ScanDirection::Suffix => {
            for index in (0..values.len()).rev() {
                if mode == ScanMode::Exclusive {
                    out[index] = accumulator;
                }
                accumulator = accumulator.mul(values[index]);
                if mode == ScanMode::Inclusive {
                    out[index] = accumulator;
                }
            }
        }
    }
}

fn scan_assign<F: Field>(values: &mut [F], direction: ScanDirection, mode: ScanMode) {
    let mut accumulator = F::ONE;
    match direction {
        ScanDirection::Prefix => {
            for value in values {
                let input = *value;
                if mode == ScanMode::Exclusive {
                    *value = accumulator;
                }
                accumulator = accumulator.mul(input);
                if mode == ScanMode::Inclusive {
                    *value = accumulator;
                }
            }
        }
        ScanDirection::Suffix => {
            for index in (0..values.len()).rev() {
                let input = values[index];
                if mode == ScanMode::Exclusive {
                    values[index] = accumulator;
                }
                accumulator = accumulator.mul(input);
                if mode == ScanMode::Inclusive {
                    values[index] = accumulator;
                }
            }
        }
    }
}
