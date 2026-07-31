//! Small public traits following interface segregation.

use super::{StaticFieldSpec, pow::pow_vartime};
use crate::DecodeError;

/// Core field operations required by generic algebra.
pub trait Field: Copy + Clone + Eq + Send + Sync + 'static {
    /// Additive identity.
    const ZERO: Self;
    /// Multiplicative identity.
    const ONE: Self;

    /// Returns `self + rhs`.
    #[must_use]
    fn add(self, rhs: Self) -> Self;

    /// Returns `self - rhs`.
    #[must_use]
    fn sub(self, rhs: Self) -> Self;

    /// Returns `-self`.
    #[must_use]
    fn neg(self) -> Self;

    /// Returns `self * rhs`.
    #[must_use]
    fn mul(self, rhs: Self) -> Self;

    /// Reports whether the value is the additive identity.
    #[must_use]
    fn is_zero(&self) -> bool;
}

/// Capability for a dedicated squaring operation.
pub trait Square: Field {
    /// Returns `self²`.
    #[must_use]
    fn square(self) -> Self;
}

/// Capability for multiplicative inversion.
pub trait Invert: Field {
    /// Returns the inverse, or `None` for zero.
    #[must_use]
    fn invert(self) -> Option<Self>;
}

/// Capability for variable-time exponentiation.
pub trait Pow: Field + Square {
    /// Raises `self` to an unsigned little-endian exponent.
    ///
    /// An empty exponent is zero and `0⁰` is defined as [`Field::ONE`].
    /// Execution time depends on exponent length and bits.
    #[must_use]
    fn pow(self, exponent_le: &[u64]) -> Self {
        pow_vartime(self, exponent_le)
    }
}

/// Stable canonical byte representation for a field.
pub trait CanonicalEncoding: Field {
    /// Fixed-size representation type.
    type Repr: Copy + Clone + Default + AsRef<[u8]> + AsMut<[u8]>;

    /// Decodes a fixed-size canonical representation.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] when the representation contains padding or a
    /// value outside the canonical field range.
    fn from_canonical(repr: &Self::Repr) -> Result<Self, DecodeError>;

    /// Decodes a canonical representation from a slice.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] when the length is incorrect or the bytes are
    /// not canonical for the concrete field.
    fn from_canonical_slice(bytes: &[u8]) -> Result<Self, DecodeError>;

    /// Encodes the element canonically.
    #[must_use]
    fn to_canonical(self) -> Self::Repr;
}

/// A field represented as an extension of a base field.
pub trait ExtensionField: Field {
    /// Base field.
    type Base: Field;
    /// Extension degree over [`Self::Base`].
    const DEGREE: usize;

    /// Applies the Frobenius automorphism `power` times.
    #[must_use]
    fn frobenius(self, power: usize) -> Self;

    /// Computes the field trace.
    #[must_use]
    fn trace(self) -> Self::Base;

    /// Computes the field norm.
    #[must_use]
    fn norm(self) -> Self::Base;
}

/// A binary extension field represented in a polynomial basis.
pub trait BinaryPolynomialField: ExtensionField<Base = super::F2> {
    /// Degree of the defining irreducible polynomial.
    const MODULUS_DEGREE: usize;

    /// Multiplies by the polynomial basis element `x`.
    #[must_use]
    fn mul_by_x(self) -> Self;

    /// Interprets arbitrary little-endian polynomial bytes and reduces them.
    #[must_use]
    fn from_polynomial_bytes_mod(bytes_le: &[u8]) -> Self;
}

/// Associates a maintained field type with immutable generated metadata.
pub trait StaticField: Field {
    /// Returns the metadata for this field presentation.
    #[must_use]
    fn spec() -> &'static StaticFieldSpec;
}
