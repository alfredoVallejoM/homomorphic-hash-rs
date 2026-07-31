//! Shared extension-field algorithms for binary fields.

use crate::{F2, Field, Square};

/// Applies repeated Frobenius squaring with the period fixed by the degree.
pub(crate) fn frobenius_binary<F, const DEGREE: usize>(value: F, power: usize) -> F
where
    F: Field + Square,
{
    let mut result = value;
    for _ in 0..power % DEGREE {
        result = result.square();
    }
    result
}

/// Computes the absolute trace into GF(2).
pub(crate) fn trace_binary<F, const DEGREE: usize>(value: F) -> F2
where
    F: Field + Square,
{
    let mut conjugate = value;
    let mut result = value;
    for _ in 1..DEGREE {
        conjugate = conjugate.square();
        result = result.add(conjugate);
    }
    debug_assert!(result == F::ZERO || result == F::ONE);
    F2::from_bool(result == F::ONE)
}
