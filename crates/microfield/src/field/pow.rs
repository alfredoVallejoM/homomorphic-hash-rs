//! Allocation-free exponentiation shared by field implementations.

use super::{Field, Square};

pub(crate) fn pow_vartime<F>(base: F, exponent_le: &[u64]) -> F
where
    F: Field + Square,
{
    let mut result = F::ONE;
    let mut power = base;

    for &word in exponent_le {
        let mut remaining = word;
        for _ in 0..u64::BITS {
            if remaining & 1 == 1 {
                result = result.mul(power);
            }
            power = power.square();
            remaining >>= 1;
        }
    }

    result
}
