/// Pure algebraic contract for the Finite Field GF(2^n).
/// Closed for modification: Defines the strict mathematics of the substrate.
pub trait FiniteField: Sized + Eq + PartialEq + Clone + std::fmt::Debug {
    /// Additive identity element (0). Axiom: A + 0 = A
    fn zero() -> Self;

    /// Multiplicative identity element (1). Axiom: A * 1 = A
    fn one() -> Self;

    /// Homomorphic Addition.
    /// In characteristic 2, this must be strictly idempotent: A + A = 0.
    fn add(&self, other: &Self) -> Self;

    /// Carry-less polynomial multiplication modulo P(x).
    /// Behavior changes based on the `crypto_mode` feature flag.
    fn mul(&self, other: &Self) -> Self;

    /// Multiplicative inverse in the cyclic group using Fermat's Little Theorem.
    /// Returns `None` if attempting to invert the absorbing element (0).
    fn inv(&self) -> Option<Self>;

    /// Phase shift operator for positional asymmetry injection.
    /// Mathematically equivalent to multiplying by the generating root modulo P(x).
    fn shift_phase(&self) -> Self;

    /// Canonical injection from the ambient Euclidean space (256 bits) into the Finite Field.
    /// MATHEMATICAL AXIOM: This must be a bijective mapping.
    fn from_bytes_canonical(data: &[u8; 32]) -> Self;
}
