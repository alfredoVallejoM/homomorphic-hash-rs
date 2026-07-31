//! Fixed-plan inversion for validated fields.

use crate::{Field, Square};

/// Executes the binary chain for the exponent `2^DEGREE - 2`.
///
/// The loop bounds and sequence are independent of the input. The initial zero
/// check implements the public `Option` contract.
pub(crate) fn invert_binary<F, const DEGREE: usize>(value: F) -> Option<F>
where
    F: Field + Square,
{
    if value.is_zero() {
        return None;
    }

    let mut accumulator = value;
    debug_assert!(DEGREE >= 2);
    for _ in 0..DEGREE - 2 {
        accumulator = accumulator.square().mul(value);
    }
    Some(accumulator.square())
}
