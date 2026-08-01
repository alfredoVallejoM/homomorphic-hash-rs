//! Backend-independent kernel capabilities and scheduling metadata.

/// Stable identifier for a batch execution backend.
///
/// An identifier does not claim that the backend was compiled or is available
/// on the current CPU. [`crate::EngineBuilder`] validates that separately.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BackendId {
    /// Allocation-free scalar portable loops.
    Portable,
    /// x86-64 carry-less multiplication backend.
    X86Pclmul,
    /// x86-64 vector carry-less multiplication backend.
    X86Vpclmul,
    /// `AArch64` polynomial multiplication backend.
    Aarch64Pmull,
}

/// Input-dependent scheduling property of a kernel strategy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ScheduleKind {
    /// The operation count or control flow may depend on field values.
    DataDependent,
    /// The strategy has a fixed operation schedule.
    Fixed,
}

/// Immutable diagnostic metadata for a selected kernel strategy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct KernelMetadata {
    backend: BackendId,
    minimum_batch: usize,
    preferred_multiple: usize,
    required_alignment: usize,
    supports_in_place: bool,
    requires_packing: bool,
    scratch_bytes_per_element: usize,
    schedule: ScheduleKind,
}

impl KernelMetadata {
    pub(crate) const fn portable<F>() -> Self {
        Self {
            backend: BackendId::Portable,
            minimum_batch: 0,
            preferred_multiple: 1,
            required_alignment: core::mem::align_of::<F>(),
            supports_in_place: true,
            requires_packing: false,
            scratch_bytes_per_element: 0,
            schedule: ScheduleKind::DataDependent,
        }
    }

    /// Returns the selected backend identifier.
    #[must_use]
    pub const fn backend(&self) -> BackendId {
        self.backend
    }

    /// Returns the smallest supported batch length.
    #[must_use]
    pub const fn minimum_batch(&self) -> usize {
        self.minimum_batch
    }

    /// Returns the strategy's preferred element multiple.
    #[must_use]
    pub const fn preferred_multiple(&self) -> usize {
        self.preferred_multiple
    }

    /// Returns the required element alignment in bytes.
    #[must_use]
    pub const fn required_alignment(&self) -> usize {
        self.required_alignment
    }

    /// Reports whether explicit in-place entry points are supported.
    #[must_use]
    pub const fn supports_in_place(&self) -> bool {
        self.supports_in_place
    }

    /// Reports whether values must be packed before execution.
    #[must_use]
    pub const fn requires_packing(&self) -> bool {
        self.requires_packing
    }

    /// Returns required scratch bytes per element.
    #[must_use]
    pub const fn scratch_bytes_per_element(&self) -> usize {
        self.scratch_bytes_per_element
    }

    /// Returns the scheduling property of the strategy.
    #[must_use]
    pub const fn schedule(&self) -> ScheduleKind {
        self.schedule
    }
}
