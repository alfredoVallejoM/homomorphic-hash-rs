//! Verified fixed-exponent plans for maintained prime fields.

use crate::{Field, Square};

/// Exact operation count of a fixed prime-field exponentiation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PrimeExponentiationCost {
    squares: u32,
    multiplications: u32,
}

impl PrimeExponentiationCost {
    /// Returns the number of squarings in the public schedule.
    #[must_use]
    pub const fn squares(self) -> u32 {
        self.squares
    }

    /// Returns the number of multiplications in the public schedule.
    #[must_use]
    pub const fn multiplications(self) -> u32 {
        self.multiplications
    }
}

/// Generated, immutable square-and-multiply plan for one public exponent.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PrimeExponentiationPlan<const LIMBS: usize> {
    exponent_le: [u64; LIMBS],
    significant_bits: u32,
    cost: PrimeExponentiationCost,
}

impl<const LIMBS: usize> PrimeExponentiationPlan<LIMBS> {
    /// Creates and counts a generated public exponent.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_generated(exponent_le: [u64; LIMBS], significant_bits: u32) -> Self {
        assert!(significant_bits > 0);
        assert!(significant_bits as usize <= LIMBS * 64);
        let mut multiplications = 0_u32;
        let mut bit = 0_u32;
        while bit < significant_bits {
            let limb = bit as usize / 64;
            let offset = bit % 64;
            multiplications += ((exponent_le[limb] >> offset) & 1) as u32;
            bit += 1;
        }
        Self {
            exponent_le,
            significant_bits,
            cost: PrimeExponentiationCost {
                squares: significant_bits,
                multiplications,
            },
        }
    }

    /// Returns the public little-endian exponent.
    #[must_use]
    pub const fn exponent_le(&self) -> &[u64; LIMBS] {
        &self.exponent_le
    }

    /// Returns the exact static operation count.
    #[must_use]
    pub const fn cost(self) -> PrimeExponentiationCost {
        self.cost
    }

    /// Verifies that code generation recorded the required exponent exactly.
    #[must_use]
    pub const fn verifies_target(self, target_le: [u64; LIMBS]) -> bool {
        let mut index = 0;
        while index < LIMBS {
            if self.exponent_le[index] != target_le[index] {
                return false;
            }
            index += 1;
        }
        true
    }

    /// Evaluates the fixed schedule. Timing depends on the public plan, never
    /// on field-element bits.
    #[must_use]
    pub fn evaluate<F: Field + Square>(self, base: F) -> F {
        pow_fixed(base, &self.exponent_le, self.significant_bits)
    }
}

#[inline]
pub(crate) fn pow_fixed<F: Field + Square>(base: F, exponent: &[u64], bits: u32) -> F {
    let mut result = F::ONE;
    let mut bit = bits;
    while bit != 0 {
        bit -= 1;
        result = result.square();
        if (exponent[bit as usize / 64] >> (bit % 64)) & 1 == 1 {
            result = result.mul(base);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CanonicalEncoding, Fp251V1};

    #[test]
    fn fixed_plan_counts_and_evaluates_the_recorded_exponent() {
        let plan = PrimeExponentiationPlan::__from_generated([249], 8);
        assert!(plan.verifies_target([249]));
        assert!(!plan.verifies_target([250]));
        assert_eq!(plan.cost().squares(), 8);
        assert_eq!(plan.cost().multiplications(), 6);
        let value = Fp251V1::from_canonical(&[17]).unwrap();
        assert_eq!(plan.evaluate(value).mul(value), Fp251V1::ONE);
    }
}
