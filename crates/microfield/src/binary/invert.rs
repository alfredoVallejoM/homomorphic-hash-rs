//! Fixed-plan inversion for validated fields.

use crate::{Field, Square};

/// Executes the generated binary chain for the exponent `2^256 - 2`.
///
/// The loop bounds and sequence are independent of the input. The initial zero
/// check implements the public `Option` contract.
pub(crate) fn invert_256<F>(value: F) -> Option<F>
where
    F: Field + Square,
{
    if value.is_zero() {
        return None;
    }

    let mut accumulator = value;
    for _ in 0..254 {
        accumulator = accumulator.square().mul(value);
    }
    Some(accumulator.square())
}
