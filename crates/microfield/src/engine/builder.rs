//! Builder for immutable execution engines.

use core::{fmt, marker::PhantomData};

use crate::{__private::PortableField, BackendId, ScheduleKind};

use super::{Engine, ExecutionPolicy};

/// Failure while selecting an immutable execution engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineBuildError {
    /// The requested backend is not compiled or not available for this field.
    BackendUnavailable(BackendId),
    /// No available strategy satisfies the requested policy.
    PolicyUnsatisfied(ExecutionPolicy),
}

impl fmt::Display for EngineBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendUnavailable(backend) => {
                write!(formatter, "batch backend {backend:?} is unavailable")
            }
            Self::PolicyUnsatisfied(policy) => {
                write!(formatter, "batch policy {policy:?} cannot be satisfied")
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
    field: PhantomData<F>,
}

impl<F: PortableField> EngineBuilder<F> {
    /// Creates a builder using [`ExecutionPolicy::Auto`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            policy: ExecutionPolicy::Auto,
            expected_batch: None,
            forced_backend: None,
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

    /// Selects the strategy exactly once and creates an immutable engine.
    ///
    /// # Errors
    ///
    /// Returns [`EngineBuildError`] when a forced backend is unavailable or no
    /// compiled strategy satisfies the requested scheduling policy.
    pub fn build(self) -> Result<Engine<F>, EngineBuildError> {
        let kernels = F::__portable_strategy().kernels();

        if let Some(backend) = self.forced_backend
            && backend != BackendId::Portable
        {
            return Err(EngineBuildError::BackendUnavailable(backend));
        }
        if self.policy == ExecutionPolicy::FixedSchedule
            && kernels.metadata.schedule() != ScheduleKind::Fixed
        {
            return Err(EngineBuildError::PolicyUnsatisfied(self.policy));
        }

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
