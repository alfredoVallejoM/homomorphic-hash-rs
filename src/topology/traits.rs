use crate::algebra::traits::FiniteField;

/// Legacy aggregation contract retained for source and byte compatibility.
///
/// New code should use [`crate::structural`], which identifies the field,
/// encoder, law and parameters and distinguishes residual equations from
/// membership guarantees.
pub trait HomomorphicAggregator<F: FiniteField> {
    /// The topological vacuum.
    /// Additive identity (0) for Symmetric Differences/Sequences.
    /// Multiplicative identity (1) for Multisets.
    fn empty_state() -> F;

    /// Historical byte embedding. It is deterministic but has known collisions
    /// and no explicit encoder identity.
    fn embed_to_field(data: &[u8]) -> F;

    /// Topological Sink.
    /// Absorbs a new element into the macro-state algebraically.
    fn aggregate(state: &F, new_element: &F, index: usize) -> F;

    /// Algebraic inverse relation. Success does not prove prior membership.
    fn remove(state: &F, element: &F) -> Option<F>;
}
