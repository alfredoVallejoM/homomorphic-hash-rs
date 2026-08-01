//! Backend-independent execution policies.

/// Backend-independent strategy selection policy.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ExecutionPolicy {
    /// Chooses the best available strategy for the supplied hints.
    #[default]
    Auto,
    /// Prefers strategies with low setup cost.
    LowLatency,
    /// Prefers strategies optimized for large batches.
    Throughput,
    /// Restricts selection to the portable backend.
    PortableOnly,
    /// Requires a strategy with a fixed operation schedule.
    FixedSchedule,
}
