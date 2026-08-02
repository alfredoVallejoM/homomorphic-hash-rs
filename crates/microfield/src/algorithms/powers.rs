//! Reusable fixed-base power tables.

use core::fmt;

use crate::Field;

/// Failure while creating an owned fixed-base table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PowerTableError {
    /// `max_exponent + 1` overflowed `usize`.
    SizeOverflow,
    /// The table allocation could not be reserved.
    AllocationFailed,
}

impl fmt::Display for PowerTableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeOverflow => formatter.write_str("fixed-power table size overflow"),
            Self::AllocationFailed => formatter.write_str("fixed-power table allocation failed"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PowerTableError {}

/// Fills caller-provided storage with `1, base, base², ...`.
///
/// An empty output is accepted and performs no work.
pub fn fill_fixed_base_powers<F: Field>(out: &mut [F], base: F) {
    let mut power = F::ONE;
    for output in out {
        *output = power;
        power = power.mul(base);
    }
}

/// Owned table containing every power from zero through a fixed maximum.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedBasePowers<F: Field> {
    base: F,
    powers: alloc::vec::Vec<F>,
}

#[cfg(feature = "alloc")]
impl<F: Field> FixedBasePowers<F> {
    /// Allocates and computes every power through `max_exponent`.
    ///
    /// # Errors
    ///
    /// Returns a sizing or allocation error without exposing a partial table.
    pub fn new(base: F, max_exponent: usize) -> Result<Self, PowerTableError> {
        let len = max_exponent
            .checked_add(1)
            .ok_or(PowerTableError::SizeOverflow)?;
        let mut powers = alloc::vec::Vec::new();
        powers
            .try_reserve_exact(len)
            .map_err(|_| PowerTableError::AllocationFailed)?;
        powers.resize(len, F::ZERO);
        fill_fixed_base_powers(&mut powers, base);
        Ok(Self { base, powers })
    }

    /// Returns the table base.
    #[must_use]
    pub const fn base(&self) -> F {
        self.base
    }

    /// Returns the greatest exponent stored in the table.
    #[must_use]
    pub fn max_exponent(&self) -> usize {
        self.powers.len() - 1
    }

    /// Returns a stored power or `None` when it exceeds the table.
    #[must_use]
    pub fn power(&self, exponent: usize) -> Option<F> {
        self.powers.get(exponent).copied()
    }

    /// Returns all powers in ascending exponent order.
    #[must_use]
    pub fn as_slice(&self) -> &[F] {
        &self.powers
    }
}
