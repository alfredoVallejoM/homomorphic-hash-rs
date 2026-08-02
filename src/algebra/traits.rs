/// Compatibility contract for the original fixed-width binary field API.
///
/// New generic code should use the segregated traits from [`microfield`]. This
/// trait remains so existing consumers can migrate without changing their
/// serialized bytes or topology adapters in one step.
pub trait FiniteField: Sized + Eq + PartialEq + Clone + std::fmt::Debug {
    /// Additive identity element (0). Axiom: A + 0 = A
    fn zero() -> Self;

    /// Multiplicative identity element (1). Axiom: A * 1 = A
    fn one() -> Self;

    /// Field addition. In characteristic two, `a + a = 0`.
    fn add(&self, other: &Self) -> Self;

    /// Carry-less polynomial multiplication modulo the field polynomial.
    fn mul(&self, other: &Self) -> Self;

    /// Multiplicative inverse in the cyclic group using Fermat's Little Theorem.
    /// Returns `None` if attempting to invert the absorbing element (0).
    fn inv(&self) -> Option<Self>;

    /// Phase shift operator for positional asymmetry injection.
    /// Mathematically equivalent to multiplying by the generating root modulo P(x).
    fn shift_phase(&self) -> Self;

    /// Canonical bijection between the legacy 32-byte representation and the
    /// concrete 256-bit field presentation.
    fn from_bytes_canonical(data: &[u8; 32]) -> Self;
}
