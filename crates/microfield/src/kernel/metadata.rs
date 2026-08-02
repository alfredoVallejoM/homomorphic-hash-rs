//! Backend-independent kernel capabilities and scheduling metadata.

#[cfg(feature = "prime-fields")]
use crate::{PrimeReductionKind, PrimeRepresentationKind, RangeContract};

/// Auditable representation and range metadata for a prime-field kernel.
#[cfg(feature = "prime-fields")]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PrimeKernelMetadata {
    representation: PrimeRepresentationKind,
    reduction: PrimeReductionKind,
    input_range: RangeContract,
    output_range: RangeContract,
    lanes: u16,
    requires_packing: bool,
}

#[cfg(feature = "prime-fields")]
impl PrimeKernelMetadata {
    /// Creates certified kernel metadata without exposing arithmetic constants.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_generated(
        representation: PrimeRepresentationKind,
        reduction: PrimeReductionKind,
        input_range: RangeContract,
        output_range: RangeContract,
        lanes: u16,
        requires_packing: bool,
    ) -> Self {
        Self {
            representation,
            reduction,
            input_range,
            output_range,
            lanes,
            requires_packing,
        }
    }

    /// Returns the representation expected by the kernel.
    #[must_use]
    pub const fn representation(self) -> PrimeRepresentationKind {
        self.representation
    }

    /// Returns the reduction family.
    #[must_use]
    pub const fn reduction(self) -> PrimeReductionKind {
        self.reduction
    }

    /// Returns the accepted input range.
    #[must_use]
    pub const fn input_range(self) -> RangeContract {
        self.input_range
    }

    /// Returns the canonical output range.
    #[must_use]
    pub const fn output_range(self) -> RangeContract {
        self.output_range
    }

    /// Returns the independent residues processed per vector tile.
    #[must_use]
    pub const fn lanes(self) -> u16 {
        self.lanes
    }

    /// Reports whether the kernel requires a persistent packed layout.
    #[must_use]
    pub const fn requires_packing(self) -> bool {
        self.requires_packing
    }
}

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
    /// x86-64 AVX2 backend processing independent prime residues.
    X86PrimeAvx2,
    /// x86-64 BMI2 backend for multi-limb prime products.
    X86PrimeBmi2,
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
    automatic_selection: bool,
    #[cfg(feature = "prime-fields")]
    prime: Option<PrimeKernelMetadata>,
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
            automatic_selection: true,
            #[cfg(feature = "prime-fields")]
            prime: None,
        }
    }

    #[cfg(all(
        feature = "portable",
        feature = "builtin-fields",
        target_arch = "x86_64"
    ))]
    pub(crate) const fn x86_pclmul<F>(
        calibration: super::calibration::SelectionCalibration,
    ) -> Self {
        Self::isa::<F>(
            BackendId::X86Pclmul,
            calibration.minimum_batch(),
            ScheduleKind::Fixed,
            calibration.automatic_selection(),
        )
    }

    #[cfg(all(feature = "portable", target_arch = "x86_64"))]
    pub(crate) const fn x86_pclmul_explicit<F>(schedule: ScheduleKind) -> Self {
        Self::isa::<F>(BackendId::X86Pclmul, 1, schedule, false)
    }

    #[cfg(all(
        feature = "portable",
        feature = "builtin-fields",
        target_arch = "x86_64"
    ))]
    pub(crate) const fn x86_vpclmul(calibration: super::calibration::SelectionCalibration) -> Self {
        Self {
            backend: BackendId::X86Vpclmul,
            minimum_batch: calibration.minimum_batch(),
            preferred_multiple: 2,
            required_alignment: 32,
            supports_in_place: true,
            requires_packing: true,
            scratch_bytes_per_element: 0,
            schedule: ScheduleKind::Fixed,
            automatic_selection: calibration.automatic_selection(),
            #[cfg(feature = "prime-fields")]
            prime: None,
        }
    }

    #[cfg(all(feature = "portable", target_arch = "x86_64"))]
    pub(crate) const fn x86_vpclmul_explicit(schedule: ScheduleKind) -> Self {
        Self {
            backend: BackendId::X86Vpclmul,
            minimum_batch: 2,
            preferred_multiple: 2,
            required_alignment: 32,
            supports_in_place: true,
            requires_packing: true,
            scratch_bytes_per_element: 0,
            schedule,
            automatic_selection: false,
            #[cfg(feature = "prime-fields")]
            prime: None,
        }
    }

    #[cfg(all(feature = "portable", target_arch = "aarch64"))]
    pub(crate) const fn aarch64_pmull_explicit<F>(schedule: ScheduleKind) -> Self {
        Self::isa::<F>(
            BackendId::Aarch64Pmull,
            super::calibration::AARCH64_PMULL.minimum_batch(),
            schedule,
            super::calibration::AARCH64_PMULL.automatic_selection(),
        )
    }

    #[cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]
    pub(crate) const fn x86_prime_avx2(minimum_batch: usize) -> Self {
        Self::x86_prime_avx2_lanes(minimum_batch, 32)
    }

    #[cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]
    pub(crate) const fn x86_prime_avx2_lanes(
        minimum_batch: usize,
        preferred_multiple: usize,
    ) -> Self {
        assert!(preferred_multiple > 0);
        Self {
            backend: BackendId::X86PrimeAvx2,
            minimum_batch,
            preferred_multiple,
            required_alignment: 32,
            supports_in_place: true,
            requires_packing: false,
            scratch_bytes_per_element: 0,
            schedule: ScheduleKind::Fixed,
            automatic_selection: true,
            prime: None,
        }
    }

    #[cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]
    pub(crate) const fn x86_prime_goldilocks_avx2<F>() -> Self {
        let calibration = super::calibration::X86_PRIME_AVX2_GOLDILOCKS;
        Self {
            backend: BackendId::X86PrimeAvx2,
            minimum_batch: calibration.minimum_batch(),
            preferred_multiple: 4,
            required_alignment: core::mem::align_of::<F>(),
            supports_in_place: true,
            requires_packing: false,
            scratch_bytes_per_element: 0,
            schedule: ScheduleKind::Fixed,
            automatic_selection: calibration.automatic_selection(),
            prime: None,
        }
    }

    #[cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]
    pub(crate) const fn x86_prime_avx2_candidate(
        minimum_batch: usize,
        preferred_multiple: usize,
    ) -> Self {
        assert!(preferred_multiple > 0);
        Self {
            backend: BackendId::X86PrimeAvx2,
            minimum_batch,
            preferred_multiple,
            required_alignment: 32,
            supports_in_place: true,
            requires_packing: false,
            scratch_bytes_per_element: 0,
            schedule: ScheduleKind::Fixed,
            automatic_selection: false,
            prime: None,
        }
    }

    #[cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]
    pub(crate) const fn x86_prime_bmi2_candidate<F>(minimum_batch: usize) -> Self {
        Self {
            backend: BackendId::X86PrimeBmi2,
            minimum_batch,
            preferred_multiple: 1,
            required_alignment: core::mem::align_of::<F>(),
            supports_in_place: true,
            requires_packing: false,
            scratch_bytes_per_element: 0,
            schedule: ScheduleKind::Fixed,
            automatic_selection: false,
            prime: None,
        }
    }

    #[cfg(all(
        feature = "portable",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    const fn isa<F>(
        backend: BackendId,
        minimum_batch: usize,
        schedule: ScheduleKind,
        automatic_selection: bool,
    ) -> Self {
        Self {
            backend,
            minimum_batch,
            preferred_multiple: 1,
            required_alignment: core::mem::align_of::<F>(),
            supports_in_place: true,
            requires_packing: false,
            scratch_bytes_per_element: 0,
            schedule,
            automatic_selection,
            #[cfg(feature = "prime-fields")]
            prime: None,
        }
    }

    #[cfg(test)]
    pub(crate) const fn for_test(
        backend: BackendId,
        minimum_batch: usize,
        schedule: ScheduleKind,
    ) -> Self {
        Self {
            backend,
            minimum_batch,
            preferred_multiple: 1,
            required_alignment: 1,
            supports_in_place: true,
            requires_packing: false,
            scratch_bytes_per_element: 0,
            schedule,
            automatic_selection: true,
            #[cfg(feature = "prime-fields")]
            prime: None,
        }
    }

    #[cfg(test)]
    pub(crate) const fn for_packing_test(
        preferred_multiple: usize,
        required_alignment: usize,
    ) -> Self {
        Self {
            backend: BackendId::Portable,
            minimum_batch: 0,
            preferred_multiple,
            required_alignment,
            supports_in_place: true,
            requires_packing: true,
            scratch_bytes_per_element: 0,
            schedule: ScheduleKind::Fixed,
            automatic_selection: false,
            #[cfg(feature = "prime-fields")]
            prime: None,
        }
    }

    /// Returns the selected backend identifier.
    #[must_use]
    pub const fn backend(&self) -> BackendId {
        self.backend
    }

    /// Returns the smallest batch length recommended for automatic selection.
    ///
    /// Every registered kernel must remain correct for shorter slices; this is
    /// a performance hint, not a precondition of batch operations.
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

    /// Reports whether the backend has a native persistent packed layout.
    ///
    /// Ordinary slice entry points remain correct for any valid slice. Packing
    /// supplies the alignment, tiling and initialized padding promised by the
    /// backend metadata and avoids repeating that preparation across calls.
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

    /// Reports whether unforced policy selection may choose this strategy.
    ///
    /// A `false` value means correctness is certified but representative
    /// target measurements have not yet established an automatic threshold.
    #[must_use]
    pub const fn automatic_selection(&self) -> bool {
        self.automatic_selection
    }

    /// Attaches certified prime-field representation and range metadata.
    #[cfg(feature = "prime-fields")]
    #[must_use]
    pub(crate) const fn with_prime(mut self, prime: PrimeKernelMetadata) -> Self {
        self.prime = Some(prime);
        self
    }

    /// Returns prime-specific metadata when this is a prime-field strategy.
    #[cfg(feature = "prime-fields")]
    #[must_use]
    pub const fn prime(&self) -> Option<&PrimeKernelMetadata> {
        self.prime.as_ref()
    }
}
