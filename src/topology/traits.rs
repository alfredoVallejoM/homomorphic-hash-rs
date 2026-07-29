use crate::algebra::traits::FiniteField;

/// Defines the Open/Closed Principle (OCP) strategy for pure structural aggregation.
/// Maps real-world byte streams into the characteristic-2 Galois field without
/// pseudo-random noise, strictly preserving the algebraic homomorphism.
pub trait HomomorphicAggregator<F: FiniteField> {
    /// The topological vacuum.
    /// Additive identity (0) for Symmetric Differences/Sequences.
    /// Multiplicative identity (1) for Multisets.
    fn empty_state() -> F;

    /// Pure Linear Embedding.
    /// PRECONDITION: Must NOT use PRFs or hashing. Must embed the raw data
    /// as a polynomial to guarantee phi(A (+) B) = phi(A) (+) phi(B).
    fn embed_to_field(data: &[u8]) -> F;

    /// Topological Sink.
    /// Absorbs a new element into the macro-state algebraically.
    fn aggregate(state: &F, new_element: &F, index: usize) -> F;

    /// Topological Cleavage.
    /// Reverses the aggregation if the geometry mathematically allows it.
    fn remove(state: &F, element: &F) -> Option<F>;
}
