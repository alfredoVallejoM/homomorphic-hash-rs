//! Builder and selector for immutable execution engines.

use core::{fmt, marker::PhantomData};

use crate::{
    __private::PortableField,
    BackendId, CpuCapabilities, ScheduleKind,
    kernel::{KernelCatalog, KernelSet},
};

use super::{Engine, ExecutionPolicy};

const AUTO_ORDER: [BackendId; 4] = [
    BackendId::X86Vpclmul,
    BackendId::X86Pclmul,
    BackendId::Aarch64Pmull,
    BackendId::Portable,
];
const LOW_LATENCY_ORDER: [BackendId; 4] = [
    BackendId::X86Pclmul,
    BackendId::Aarch64Pmull,
    BackendId::X86Vpclmul,
    BackendId::Portable,
];
const THROUGHPUT_ORDER: [BackendId; 4] = AUTO_ORDER;

/// Failure while selecting an immutable execution engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum EngineBuildError {
    /// The requested backend has no implementation in this build.
    BackendNotCompiled(BackendId),
    /// The requested backend has no certified strategy for this field.
    BackendUnsupportedByField(BackendId),
    /// The detected or injected CPU snapshot cannot execute the backend.
    BackendUnsupportedByCpu(BackendId),
    /// No available strategy satisfies the requested policy.
    PolicyUnsatisfied(ExecutionPolicy),
    /// Legacy catch-all retained for source compatibility.
    #[deprecated(note = "use the precise backend selection variants")]
    BackendUnavailable(BackendId),
}

impl fmt::Display for EngineBuildError {
    #[allow(deprecated)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendNotCompiled(backend) => {
                write!(formatter, "batch backend {backend:?} is not compiled")
            }
            Self::BackendUnsupportedByField(backend) => {
                write!(
                    formatter,
                    "batch backend {backend:?} does not support this field"
                )
            }
            Self::BackendUnsupportedByCpu(backend) => {
                write!(
                    formatter,
                    "batch backend {backend:?} is not supported by this CPU"
                )
            }
            Self::PolicyUnsatisfied(policy) => {
                write!(formatter, "batch policy {policy:?} cannot be satisfied")
            }
            Self::BackendUnavailable(backend) => {
                write!(formatter, "batch backend {backend:?} is unavailable")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EngineBuildError {}

/// Builder that selects one batch strategy before execution.
pub struct EngineBuilder<F: PortableField> {
    policy: ExecutionPolicy,
    expected_batch: Option<usize>,
    forced_backend: Option<BackendId>,
    capabilities: CpuCapabilities,
    field: PhantomData<F>,
}

impl<F: PortableField> EngineBuilder<F> {
    /// Creates a builder using [`ExecutionPolicy::Auto`] and portable-only
    /// capabilities.
    ///
    /// Call [`Self::detect`] with `std`, or inject an already detected snapshot
    /// through [`Self::capabilities`], to enable optional ISA strategies.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            policy: ExecutionPolicy::Auto,
            expected_batch: None,
            forced_backend: None,
            capabilities: CpuCapabilities::portable_only(),
            field: PhantomData,
        }
    }

    /// Sets the backend-independent selection policy.
    #[must_use]
    pub const fn policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Supplies an advisory expected batch length for strategy selection.
    #[must_use]
    pub const fn expected_batch(mut self, len: usize) -> Self {
        self.expected_batch = Some(len);
        self
    }

    /// Requires one exact backend.
    #[must_use]
    pub const fn force_backend(mut self, backend: BackendId) -> Self {
        self.forced_backend = Some(backend);
        self
    }

    /// Injects a trusted immutable capability snapshot.
    #[must_use]
    pub const fn capabilities(mut self, capabilities: CpuCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Detects this CPU once, selects one strategy and creates the engine.
    ///
    /// # Errors
    ///
    /// Returns [`EngineBuildError`] when the requested backend or policy cannot
    /// be satisfied. This method is available only with `std`.
    #[cfg(feature = "std")]
    pub fn detect(self) -> Result<Engine<F>, EngineBuildError> {
        self.capabilities(CpuCapabilities::detect()).build()
    }

    /// Selects the strategy exactly once and creates an immutable engine.
    ///
    /// This method never performs implicit CPU detection. With the default
    /// builder it is therefore deterministic and portable-only.
    ///
    /// # Errors
    ///
    /// Returns [`EngineBuildError`] when a forced backend fails compilation,
    /// field, CPU or policy validation, or when no strategy satisfies the
    /// requested scheduling policy.
    pub fn build(self) -> Result<Engine<F>, EngineBuildError> {
        self.build_with(CompiledBackends::current())
    }

    fn build_with(self, compiled: CompiledBackends) -> Result<Engine<F>, EngineBuildError> {
        let catalog = F::__kernel_catalog();
        let kernels = select(
            &catalog,
            compiled,
            self.capabilities,
            self.policy,
            self.expected_batch,
            self.forced_backend,
        )?;
        Ok(Engine::from_selection(
            kernels,
            self.policy,
            self.expected_batch,
        ))
    }
}

impl<F: PortableField> Default for EngineBuilder<F> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
struct CompiledBackends(u8);

impl CompiledBackends {
    const PORTABLE: u8 = 1 << 0;
    const X86_PCLMUL: u8 = 1 << 1;
    const X86_VPCLMUL: u8 = 1 << 2;
    const AARCH64_PMULL: u8 = 1 << 3;

    const fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self(Self::PORTABLE | Self::X86_PCLMUL)
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self(Self::PORTABLE | Self::AARCH64_PMULL)
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            Self(Self::PORTABLE)
        }
    }

    const fn contains(self, backend: BackendId) -> bool {
        let bit = match backend {
            BackendId::Portable => Self::PORTABLE,
            BackendId::X86Pclmul => Self::X86_PCLMUL,
            BackendId::X86Vpclmul => Self::X86_VPCLMUL,
            BackendId::Aarch64Pmull => Self::AARCH64_PMULL,
        };
        self.0 & bit != 0
    }

    #[cfg(test)]
    const fn from_test_mask(mask: u8) -> Self {
        Self(mask & 0x0f)
    }
}

fn select<F: PortableField>(
    catalog: &KernelCatalog<F>,
    compiled: CompiledBackends,
    capabilities: CpuCapabilities,
    policy: ExecutionPolicy,
    expected_batch: Option<usize>,
    forced_backend: Option<BackendId>,
) -> Result<&'static KernelSet<F>, EngineBuildError> {
    if let Some(backend) = forced_backend {
        return select_forced(catalog, compiled, capabilities, policy, backend);
    }

    let order = match policy {
        ExecutionPolicy::LowLatency => &LOW_LATENCY_ORDER,
        ExecutionPolicy::Auto
        | ExecutionPolicy::Throughput
        | ExecutionPolicy::PortableOnly
        | ExecutionPolicy::FixedSchedule => &THROUGHPUT_ORDER,
    };

    for backend in order {
        if !compiled.contains(*backend) || !capabilities.supports(*backend) {
            continue;
        }
        let Some(kernels) = catalog.get(*backend) else {
            continue;
        };
        if !kernels.metadata.automatic_selection() {
            continue;
        }
        if !policy_accepts(policy, *backend, kernels.metadata.schedule()) {
            continue;
        }
        if policy == ExecutionPolicy::Auto
            && expected_batch.is_some_and(|len| len < kernels.metadata.minimum_batch())
        {
            continue;
        }
        return Ok(kernels);
    }

    Err(EngineBuildError::PolicyUnsatisfied(policy))
}

fn select_forced<F: PortableField>(
    catalog: &KernelCatalog<F>,
    compiled: CompiledBackends,
    capabilities: CpuCapabilities,
    policy: ExecutionPolicy,
    backend: BackendId,
) -> Result<&'static KernelSet<F>, EngineBuildError> {
    if !compiled.contains(backend) {
        return Err(EngineBuildError::BackendNotCompiled(backend));
    }
    let Some(kernels) = catalog.get(backend) else {
        return Err(EngineBuildError::BackendUnsupportedByField(backend));
    };
    if !capabilities.supports(backend) {
        return Err(EngineBuildError::BackendUnsupportedByCpu(backend));
    }
    if !policy_accepts(policy, backend, kernels.metadata.schedule()) {
        return Err(EngineBuildError::PolicyUnsatisfied(policy));
    }
    Ok(kernels)
}

const fn policy_accepts(
    policy: ExecutionPolicy,
    backend: BackendId,
    schedule: ScheduleKind,
) -> bool {
    match policy {
        ExecutionPolicy::PortableOnly => matches!(backend, BackendId::Portable),
        ExecutionPolicy::FixedSchedule => matches!(schedule, ScheduleKind::Fixed),
        ExecutionPolicy::Auto | ExecutionPolicy::LowLatency | ExecutionPolicy::Throughput => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{__private::PortableStrategy, Architecture, Field, KernelMetadata, Square};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestField(u8);

    impl Field for TestField {
        const ZERO: Self = Self(0);
        const ONE: Self = Self(1);

        fn add(self, rhs: Self) -> Self {
            Self(self.0 ^ rhs.0)
        }

        fn sub(self, rhs: Self) -> Self {
            self.add(rhs)
        }

        fn neg(self) -> Self {
            self
        }

        fn mul(self, rhs: Self) -> Self {
            Self(self.0 & rhs.0)
        }

        fn is_zero(&self) -> bool {
            self.0 == 0
        }
    }

    impl Square for TestField {
        fn square(self) -> Self {
            self
        }
    }

    static PORTABLE_STRATEGY: PortableStrategy<TestField> = PortableStrategy::new();

    impl PortableField for TestField {
        fn __portable_strategy() -> &'static PortableStrategy<Self> {
            &PORTABLE_STRATEGY
        }
    }

    fn binary(out: &mut [TestField], lhs: &[TestField], rhs: &[TestField]) {
        for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
            *output = left.add(*right);
        }
    }

    fn unary(out: &mut [TestField], values: &[TestField]) {
        out.copy_from_slice(values);
    }

    fn binary_assign(lhs: &mut [TestField], rhs: &[TestField]) {
        for (left, right) in lhs.iter_mut().zip(rhs) {
            *left = left.add(*right);
        }
    }

    fn unary_assign(_values: &mut [TestField]) {}

    static PORTABLE: KernelSet<TestField> = KernelSet::new(
        KernelMetadata::for_test(BackendId::Portable, 0, ScheduleKind::DataDependent),
        binary,
        binary,
        unary,
        binary_assign,
        unary_assign,
    );
    static PCLMUL: KernelSet<TestField> = KernelSet::new(
        KernelMetadata::for_test(BackendId::X86Pclmul, 1, ScheduleKind::Fixed),
        binary,
        binary,
        unary,
        binary_assign,
        unary_assign,
    );
    static VPCLMUL: KernelSet<TestField> = KernelSet::new(
        KernelMetadata::for_test(BackendId::X86Vpclmul, 8, ScheduleKind::Fixed),
        binary,
        binary,
        unary,
        binary_assign,
        unary_assign,
    );
    static PMULL: KernelSet<TestField> = KernelSet::new(
        KernelMetadata::for_test(BackendId::Aarch64Pmull, 1, ScheduleKind::Fixed),
        binary,
        binary,
        unary,
        binary_assign,
        unary_assign,
    );

    const POLICIES: [ExecutionPolicy; 5] = [
        ExecutionPolicy::Auto,
        ExecutionPolicy::LowLatency,
        ExecutionPolicy::Throughput,
        ExecutionPolicy::PortableOnly,
        ExecutionPolicy::FixedSchedule,
    ];
    const BACKENDS: [BackendId; 4] = [
        BackendId::Portable,
        BackendId::X86Pclmul,
        BackendId::X86Vpclmul,
        BackendId::Aarch64Pmull,
    ];
    const ARCHITECTURES: [Architecture; 3] = [
        Architecture::X86_64,
        Architecture::Aarch64,
        Architecture::Other,
    ];

    fn catalog(mask: u8) -> KernelCatalog<TestField> {
        let mut catalog = KernelCatalog::portable(&PORTABLE);
        if mask & CompiledBackends::X86_PCLMUL != 0 {
            catalog = catalog.with_x86_pclmul(&PCLMUL);
        }
        if mask & CompiledBackends::X86_VPCLMUL != 0 {
            catalog = catalog.with_x86_vpclmul(&VPCLMUL);
        }
        if mask & CompiledBackends::AARCH64_PMULL != 0 {
            catalog = catalog.with_aarch64_pmull(&PMULL);
        }
        catalog
    }

    fn capabilities(architecture: Architecture, mask: u8) -> CpuCapabilities {
        CpuCapabilities::from_test_parts(architecture, mask)
    }

    fn field_contains(mask: u8, backend: BackendId) -> bool {
        match backend {
            BackendId::Portable => true,
            BackendId::X86Pclmul => mask & CompiledBackends::X86_PCLMUL != 0,
            BackendId::X86Vpclmul => mask & CompiledBackends::X86_VPCLMUL != 0,
            BackendId::Aarch64Pmull => mask & CompiledBackends::AARCH64_PMULL != 0,
        }
    }

    fn policy_contains(policy: ExecutionPolicy, backend: BackendId) -> bool {
        match policy {
            ExecutionPolicy::PortableOnly => backend == BackendId::Portable,
            ExecutionPolicy::FixedSchedule => backend != BackendId::Portable,
            ExecutionPolicy::Auto | ExecutionPolicy::LowLatency | ExecutionPolicy::Throughput => {
                true
            }
        }
    }

    #[test]
    fn forced_backend_diagnostics_cover_every_validation_dimension() {
        let backend_mask_limit = if cfg!(miri) { 3 } else { 16 };
        let capability_mask_limit = if cfg!(miri) { 3 } else { 32 };
        for compilation_index in 0..backend_mask_limit {
            let compilation_mask = representative_mask(compilation_index, 0x0f);
            let compiled = CompiledBackends::from_test_mask(compilation_mask);
            for field_index in 0..backend_mask_limit {
                let field_mask = representative_mask(field_index, 0x0f);
                let catalog = catalog(field_mask);
                for architecture in ARCHITECTURES {
                    for capability_index in 0..capability_mask_limit {
                        let capability_mask = representative_mask(capability_index, 0x1f);
                        let capabilities = capabilities(architecture, capability_mask);
                        for policy in POLICIES {
                            for backend in BACKENDS {
                                let selected = select(
                                    &catalog,
                                    compiled,
                                    capabilities,
                                    policy,
                                    None,
                                    Some(backend),
                                );
                                if !compiled.contains(backend) {
                                    assert_eq!(
                                        selected.map(|_| ()),
                                        Err(EngineBuildError::BackendNotCompiled(backend))
                                    );
                                } else if !field_contains(field_mask, backend) {
                                    assert_eq!(
                                        selected.map(|_| ()),
                                        Err(EngineBuildError::BackendUnsupportedByField(backend))
                                    );
                                } else if !capabilities.supports(backend) {
                                    assert_eq!(
                                        selected.map(|_| ()),
                                        Err(EngineBuildError::BackendUnsupportedByCpu(backend))
                                    );
                                } else if !policy_contains(policy, backend) {
                                    assert_eq!(
                                        selected.map(|_| ()),
                                        Err(EngineBuildError::PolicyUnsatisfied(policy))
                                    );
                                } else {
                                    assert_eq!(
                                        selected
                                            .expect("all dimensions permit the backend")
                                            .metadata
                                            .backend(),
                                        backend
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn automatic_selection_is_deterministic_for_all_capability_snapshots() {
        let catalog = catalog(0x0f);
        let compiled = CompiledBackends::from_test_mask(0x0f);

        let capability_mask_limit = if cfg!(miri) { 3 } else { 32 };
        for architecture in ARCHITECTURES {
            for capability_index in 0..capability_mask_limit {
                let capability_mask = representative_mask(capability_index, 0x1f);
                let capabilities = capabilities(architecture, capability_mask);
                for policy in POLICIES {
                    for expected_batch in [None, Some(0), Some(1), Some(7), Some(8), Some(4096)] {
                        let first = select(
                            &catalog,
                            compiled,
                            capabilities,
                            policy,
                            expected_batch,
                            None,
                        )
                        .map(|kernels| kernels.metadata.backend());
                        for _ in 0..4 {
                            assert_eq!(
                                select(
                                    &catalog,
                                    compiled,
                                    capabilities,
                                    policy,
                                    expected_batch,
                                    None,
                                )
                                .map(|kernels| kernels.metadata.backend()),
                                first
                            );
                        }

                        if policy == ExecutionPolicy::PortableOnly {
                            assert_eq!(first, Ok(BackendId::Portable));
                        }
                        if architecture == Architecture::Other {
                            let expected = if policy == ExecutionPolicy::FixedSchedule {
                                Err(EngineBuildError::PolicyUnsatisfied(policy))
                            } else {
                                Ok(BackendId::Portable)
                            };
                            assert_eq!(first, expected);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn expected_batch_filters_only_automatic_candidates() {
        let catalog = catalog(0x0f);
        let compiled = CompiledBackends::from_test_mask(0x0f);
        let capabilities = capabilities(Architecture::X86_64, 0b00111);

        let selected = |policy, len| {
            select(&catalog, compiled, capabilities, policy, Some(len), None)
                .map(|kernels| kernels.metadata.backend())
        };

        assert_eq!(selected(ExecutionPolicy::Auto, 0), Ok(BackendId::Portable));
        assert_eq!(selected(ExecutionPolicy::Auto, 1), Ok(BackendId::X86Pclmul));
        assert_eq!(
            selected(ExecutionPolicy::Auto, 8),
            Ok(BackendId::X86Vpclmul)
        );
        assert_eq!(
            selected(ExecutionPolicy::LowLatency, 4096),
            Ok(BackendId::X86Pclmul)
        );
        assert_eq!(
            selected(ExecutionPolicy::Throughput, 0),
            Ok(BackendId::X86Vpclmul)
        );
        assert_eq!(
            select(
                &catalog,
                compiled,
                capabilities,
                ExecutionPolicy::Throughput,
                Some(0),
                Some(BackendId::X86Vpclmul),
            )
            .map(|kernels| kernels.metadata.backend()),
            Ok(BackendId::X86Vpclmul)
        );
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn portable_catalog_rejects_mislabeled_metadata() {
        let _ = KernelCatalog::portable(&PCLMUL);
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn isa_catalog_slot_rejects_mislabeled_metadata() {
        let _ = KernelCatalog::portable(&PORTABLE).with_x86_pclmul(&PMULL);
    }

    const fn representative_mask(index: u8, full_mask: u8) -> u8 {
        if cfg!(miri) && index == 2 {
            full_mask
        } else {
            index
        }
    }
}
