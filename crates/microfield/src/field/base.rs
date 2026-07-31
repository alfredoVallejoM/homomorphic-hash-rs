//! Prime base field used by binary extensions.

use core::{
    fmt,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use super::{CanonicalEncoding, Field, Invert, Pow, Square};
use crate::DecodeError;

/// The two-element field GF(2).
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct F2(bool);

impl F2 {
    /// Constructs an element from a Boolean.
    #[must_use]
    pub const fn from_bool(value: bool) -> Self {
        Self(value)
    }

    /// Returns the Boolean representation.
    #[must_use]
    pub const fn as_bool(self) -> bool {
        self.0
    }
}

impl Field for F2 {
    const ZERO: Self = Self(false);
    const ONE: Self = Self(true);

    fn add(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0)
    }

    fn sub(self, rhs: Self) -> Self {
        Field::add(self, rhs)
    }

    fn neg(self) -> Self {
        self
    }

    fn mul(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }

    fn is_zero(&self) -> bool {
        !self.0
    }
}

impl Square for F2 {
    fn square(self) -> Self {
        self
    }
}

impl Invert for F2 {
    fn invert(self) -> Option<Self> {
        self.0.then_some(self)
    }
}

impl Pow for F2 {}

impl CanonicalEncoding for F2 {
    type Repr = [u8; 1];

    fn from_canonical(repr: &Self::Repr) -> Result<Self, DecodeError> {
        match repr[0] {
            0 => Ok(Self::ZERO),
            1 => Ok(Self::ONE),
            _ => Err(DecodeError::NonCanonicalValue),
        }
    }

    fn from_canonical_slice(bytes: &[u8]) -> Result<Self, DecodeError> {
        let repr: [u8; 1] = bytes.try_into().map_err(|_| DecodeError::LengthMismatch {
            expected: 1,
            actual: bytes.len(),
        })?;
        Self::from_canonical(&repr)
    }

    fn to_canonical(self) -> Self::Repr {
        [u8::from(self.0)]
    }
}

impl From<bool> for F2 {
    fn from(value: bool) -> Self {
        Self::from_bool(value)
    }
}

impl From<F2> for bool {
    fn from(value: F2) -> Self {
        value.as_bool()
    }
}

impl Add for F2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Field::add(self, rhs)
    }
}

impl AddAssign for F2 {
    fn add_assign(&mut self, rhs: Self) {
        *self = Field::add(*self, rhs);
    }
}

impl Sub for F2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Field::sub(self, rhs)
    }
}

impl SubAssign for F2 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = Field::sub(*self, rhs);
    }
}

impl Mul for F2 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Field::mul(self, rhs)
    }
}

impl MulAssign for F2 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = Field::mul(*self, rhs);
    }
}

impl Neg for F2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Field::neg(self)
    }
}

impl fmt::Debug for F2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "F2({})", u8::from(self.0))
    }
}

impl fmt::Display for F2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(if self.0 { "1" } else { "0" })
    }
}
