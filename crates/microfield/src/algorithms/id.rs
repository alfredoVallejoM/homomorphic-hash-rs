//! Stable diagnostics for derived algorithms.

use crate::{BackendId, FieldId, StaticField};

/// High-level operation executed by a reusable plan.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum OperationKind {
    /// Invert a homogeneous batch with one scalar inversion.
    InvertBatch,
    /// Compute prefix products.
    PrefixProducts,
    /// Compute suffix products.
    SuffixProducts,
    /// Evaluate one polynomial at several points.
    HornerManyPoints,
    /// Evaluate several polynomials at one point.
    HornerManyPolynomials,
}

/// Mathematical family used to implement an operation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AlgorithmFamily {
    /// Montgomery's batch-inversion trick, unrelated to Montgomery residues.
    BatchInversionMontgomery,
    /// Deterministic sequential product scan.
    SequentialScan,
    /// Classical Horner recurrence.
    Horner,
}

/// Allocation contract of an algorithm entry point.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AllocationBehavior {
    /// No allocation and no caller-provided workspace.
    None,
    /// The hot path uses explicitly caller-provided typed workspace.
    CallerProvidedWorkspace,
    /// An explicitly named convenience route is available with `alloc`.
    AllocFeature,
}

/// Public, backend-independent memory contract of an immutable plan.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WorkspaceLayout {
    field_elements: usize,
    mask_words: usize,
    alignment: usize,
    supports_in_place: bool,
    allocation: AllocationBehavior,
}

impl WorkspaceLayout {
    pub(crate) const fn new(
        field_elements: usize,
        mask_words: usize,
        alignment: usize,
        supports_in_place: bool,
        allocation: AllocationBehavior,
    ) -> Self {
        Self {
            field_elements,
            mask_words,
            alignment,
            supports_in_place,
            allocation,
        }
    }

    /// Returns the number of typed field-element slots required.
    #[must_use]
    pub const fn field_elements(self) -> usize {
        self.field_elements
    }

    /// Returns the number of compact mask words required.
    #[must_use]
    pub const fn mask_words(self) -> usize {
        self.mask_words
    }

    /// Returns the minimum natural alignment of typed workspace elements.
    #[must_use]
    pub const fn alignment(self) -> usize {
        self.alignment
    }

    /// Reports whether the plan has an explicit in-place execution route.
    #[must_use]
    pub const fn supports_in_place(self) -> bool {
        self.supports_in_place
    }

    /// Returns the allocation contract of the primary entry point.
    #[must_use]
    pub const fn allocation(self) -> AllocationBehavior {
        self.allocation
    }
}

/// Versioned identifier for one derived algorithm.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AlgorithmId {
    operation: OperationKind,
    family: AlgorithmFamily,
    revision: u16,
}

impl AlgorithmId {
    pub(crate) const fn new(
        operation: OperationKind,
        family: AlgorithmFamily,
        revision: u16,
    ) -> Self {
        Self {
            operation,
            family,
            revision,
        }
    }

    /// Returns the high-level operation.
    #[must_use]
    pub const fn operation(self) -> OperationKind {
        self.operation
    }

    /// Returns the selected algorithm family.
    #[must_use]
    pub const fn family(self) -> AlgorithmFamily {
        self.family
    }

    /// Returns the algorithm revision.
    #[must_use]
    pub const fn revision(self) -> u16 {
        self.revision
    }
}

/// Common immutable metadata exposed by derived batch plans.
pub trait BatchPlan<F: StaticField> {
    /// Returns the operation and algorithm revision.
    fn algorithm_id(&self) -> AlgorithmId;

    /// Returns the logical element count fixed by the plan.
    fn logical_len(&self) -> usize;

    /// Returns the backend selected when the plan was created.
    fn backend_id(&self) -> BackendId;

    /// Returns the semantic field identity fixed by the nominal type.
    fn field_id(&self) -> FieldId;

    /// Returns the complete caller-visible workspace and allocation contract.
    fn workspace_layout(&self) -> WorkspaceLayout;
}
