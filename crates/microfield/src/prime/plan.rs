//! Immutable, auditable plans for prime-field representations and reductions.

use core::fmt;

/// Private representation family used by a maintained prime field.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PrimeRepresentationKind {
    /// Every stored value is the canonical residue in `[0, p)`.
    CanonicalResidue,
    /// Every stored value is `aR mod p` in a fixed radix.
    Montgomery {
        /// Bits in each radix limb.
        radix_bits: u8,
        /// Number of limbs.
        limbs: u16,
    },
}

/// Reduction family selected for an artifact.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PrimeReductionKind {
    /// Native small-integer remainder.
    Native,
    /// Reciprocal-based Barrett reduction.
    Barrett,
    /// Montgomery reduction.
    Montgomery,
    /// Sparse signed-power reduction.
    Solinas,
}

/// Proven range of an internal operation, expressed as multiples of `p`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RangeContract {
    input_multiple: u8,
    output_multiple: u8,
    accumulator_bits: u16,
}

impl RangeContract {
    /// Creates a generated range contract.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_generated(
        input_multiple: u8,
        output_multiple: u8,
        accumulator_bits: u16,
    ) -> Self {
        Self {
            input_multiple,
            output_multiple,
            accumulator_bits,
        }
    }

    /// Maximum input as a multiple of the modulus.
    #[must_use]
    pub const fn input_multiple(self) -> u8 {
        self.input_multiple
    }

    /// Maximum output as a multiple of the modulus.
    #[must_use]
    pub const fn output_multiple(self) -> u8 {
        self.output_multiple
    }

    /// Width of the accumulator used to establish the range.
    #[must_use]
    pub const fn accumulator_bits(self) -> u16 {
        self.accumulator_bits
    }

    /// Conservatively verifies that both declared multiples fit.
    ///
    /// # Errors
    ///
    /// Returns [`RangeProofError`] when a multiple is zero or the declared
    /// accumulator cannot contain the conservative bound.
    pub const fn verify(self, modulus_bits: u16) -> Result<(), RangeProofError> {
        if self.input_multiple == 0 || self.output_multiple == 0 {
            return Err(RangeProofError::ZeroMultiple);
        }
        let input_extra = ceil_log2(self.input_multiple);
        let output_extra = ceil_log2(self.output_multiple);
        if modulus_bits.saturating_add(input_extra) > self.accumulator_bits
            || modulus_bits.saturating_add(output_extra) > self.accumulator_bits
        {
            return Err(RangeProofError::AccumulatorTooNarrow);
        }
        Ok(())
    }
}

#[allow(clippy::cast_possible_truncation)]
const fn ceil_log2(value: u8) -> u16 {
    if value <= 1 {
        return 0;
    }
    (u8::BITS - (value - 1).leading_zeros()) as u16
}

/// Failure while checking generated prime reduction metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RangeProofError {
    /// A range multiple must be non-zero.
    ZeroMultiple,
    /// The declared accumulator cannot contain the declared range.
    AccumulatorTooNarrow,
    /// Plan limb counts and embedded constants disagree.
    InvalidPlanShape,
    /// Montgomery reduction requires an odd modulus.
    EvenMontgomeryModulus,
    /// `-p⁻¹ mod 2^64` does not cancel the low word.
    InvalidMontgomeryInverse,
    /// The Barrett reciprocal does not equal the declared radix quotient.
    InvalidBarrettReciprocal,
}

impl fmt::Display for RangeProofError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroMultiple => formatter.write_str("prime range multiple must be non-zero"),
            Self::AccumulatorTooNarrow => {
                formatter.write_str("prime range exceeds the declared accumulator")
            }
            Self::InvalidPlanShape => formatter.write_str("invalid prime reduction plan shape"),
            Self::EvenMontgomeryModulus => formatter.write_str("Montgomery modulus must be odd"),
            Self::InvalidMontgomeryInverse => {
                formatter.write_str("invalid Montgomery low-limb inverse")
            }
            Self::InvalidBarrettReciprocal => formatter.write_str("invalid Barrett reciprocal"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RangeProofError {}

/// Static Barrett reduction metadata.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BarrettPlan {
    /// Radix limb width.
    limb_bits: u8,
    /// Number of radix limbs.
    limbs: u16,
    /// Canonical modulus limbs, little-endian.
    modulus: &'static [u64],
    /// Reciprocal limbs, little-endian.
    reciprocal: &'static [u64],
    /// Approximation shift.
    approximation_shift: u16,
    /// Maximum number of final conditional corrections.
    correction_steps_max: u8,
    /// Auditable range contract.
    range: RangeContract,
}

impl BarrettPlan {
    /// Creates metadata emitted by the certified generator.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_generated(
        limb_bits: u8,
        limbs: u16,
        modulus: &'static [u64],
        reciprocal: &'static [u64],
        approximation_shift: u16,
        correction_steps_max: u8,
        range: RangeContract,
    ) -> Self {
        Self {
            limb_bits,
            limbs,
            modulus,
            reciprocal,
            approximation_shift,
            correction_steps_max,
            range,
        }
    }

    /// Returns the radix limb width.
    #[must_use]
    pub const fn limb_bits(self) -> u8 {
        self.limb_bits
    }

    /// Returns the number of private radix limbs without exposing them.
    #[must_use]
    pub const fn limbs(self) -> u16 {
        self.limbs
    }

    /// Returns the declared correction bound.
    #[must_use]
    pub const fn correction_steps_max(self) -> u8 {
        self.correction_steps_max
    }

    /// Returns the range proof contract.
    #[must_use]
    pub const fn range(self) -> RangeContract {
        self.range
    }

    /// Verifies shape and conservative range metadata.
    ///
    /// # Errors
    ///
    /// Returns [`RangeProofError`] if the generated arrays disagree with the
    /// declared limb count or the range contract is inconsistent.
    #[allow(clippy::cast_lossless)]
    pub const fn verify(self, modulus_bits: u16) -> Result<(), RangeProofError> {
        if self.limbs == 0
            || self.modulus.len() != self.limbs as usize
            || self.reciprocal.is_empty()
            || self.correction_steps_max == 0
        {
            return Err(RangeProofError::InvalidPlanShape);
        }
        if self.limb_bits == 64 && self.limbs == 1 && self.reciprocal.len() == 2 {
            let modulus = self.modulus[0] as u128;
            let quotient = u128::MAX / modulus;
            let remainder = u128::MAX % modulus;
            let expected = quotient + if remainder == modulus - 1 { 1 } else { 0 };
            let recorded = ((self.reciprocal[1] as u128) << 64) | self.reciprocal[0] as u128;
            if recorded != expected {
                return Err(RangeProofError::InvalidBarrettReciprocal);
            }
        }
        self.range.verify(modulus_bits)
    }
}

/// Portable Montgomery multiplication schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MontgomeryAlgorithm {
    /// Coarsely integrated operand scanning.
    Cios,
}

/// Static Montgomery reduction metadata.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MontgomeryPlan {
    /// Radix limb width.
    limb_bits: u8,
    /// Number of radix limbs.
    limbs: u16,
    /// Prime modulus limbs, little-endian.
    modulus: &'static [u64],
    /// `R mod p`.
    r: &'static [u64],
    /// `R² mod p`.
    r2: &'static [u64],
    /// `-p[0]⁻¹ mod 2^64`.
    neg_inv_mod_radix: u64,
    /// Selected fixed schedule.
    algorithm: MontgomeryAlgorithm,
    /// Auditable range contract.
    range: RangeContract,
}

impl MontgomeryPlan {
    /// Creates metadata emitted by the certified generator.
    #[doc(hidden)]
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn __from_generated(
        limb_bits: u8,
        limbs: u16,
        modulus: &'static [u64],
        r: &'static [u64],
        r2: &'static [u64],
        neg_inv_mod_radix: u64,
        algorithm: MontgomeryAlgorithm,
        range: RangeContract,
    ) -> Self {
        Self {
            limb_bits,
            limbs,
            modulus,
            r,
            r2,
            neg_inv_mod_radix,
            algorithm,
            range,
        }
    }

    /// Returns the private radix limb width.
    #[must_use]
    pub const fn limb_bits(self) -> u8 {
        self.limb_bits
    }

    /// Returns the number of private limbs without exposing their values.
    #[must_use]
    pub const fn limbs(self) -> u16 {
        self.limbs
    }

    /// Returns the selected multiplication schedule.
    #[must_use]
    pub const fn algorithm(self) -> MontgomeryAlgorithm {
        self.algorithm
    }

    /// Returns the range proof contract.
    #[must_use]
    pub const fn range(self) -> RangeContract {
        self.range
    }

    /// Verifies shape, range and the low-limb cancellation identity.
    ///
    /// # Errors
    ///
    /// Returns [`RangeProofError`] if the Montgomery shape, odd-modulus
    /// requirement, low-limb inverse or range proof is invalid.
    pub const fn verify(self, modulus_bits: u16) -> Result<(), RangeProofError> {
        if self.limb_bits != 64
            || self.limbs == 0
            || self.modulus.len() != self.limbs as usize
            || self.r.len() != self.limbs as usize
            || self.r2.len() != self.limbs as usize
        {
            return Err(RangeProofError::InvalidPlanShape);
        }
        if self.modulus[0] & 1 == 0 {
            return Err(RangeProofError::EvenMontgomeryModulus);
        }
        if self.modulus[0]
            .wrapping_mul(self.neg_inv_mod_radix)
            .wrapping_add(1)
            != 0
        {
            return Err(RangeProofError::InvalidMontgomeryInverse);
        }
        self.range.verify(modulus_bits)
    }
}

/// One signed power-of-two term in a Solinas modulus identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SignedPowerOfTwo {
    /// Positive or negative coefficient.
    pub positive: bool,
    /// Power of two.
    pub exponent: u16,
}

/// Static Solinas reduction metadata.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SolinasPlan {
    /// Significant bits of the modulus.
    modulus_bits: u16,
    /// Signed powers describing the modulus.
    terms: &'static [SignedPowerOfTwo],
    /// Maximum final corrections.
    correction_steps_max: u8,
    /// Auditable range contract.
    range: RangeContract,
}

impl SolinasPlan {
    /// Creates metadata emitted by the certified generator.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_generated(
        modulus_bits: u16,
        terms: &'static [SignedPowerOfTwo],
        correction_steps_max: u8,
        range: RangeContract,
    ) -> Self {
        Self {
            modulus_bits,
            terms,
            correction_steps_max,
            range,
        }
    }

    /// Returns the modulus width.
    #[must_use]
    pub const fn modulus_bits(self) -> u16 {
        self.modulus_bits
    }

    /// Returns the signed public modulus identity.
    #[must_use]
    pub const fn terms(self) -> &'static [SignedPowerOfTwo] {
        self.terms
    }

    /// Returns the declared correction bound.
    #[must_use]
    pub const fn correction_steps_max(self) -> u8 {
        self.correction_steps_max
    }

    /// Returns the range proof contract.
    #[must_use]
    pub const fn range(self) -> RangeContract {
        self.range
    }

    /// Verifies non-empty shape and range metadata.
    ///
    /// # Errors
    ///
    /// Returns [`RangeProofError`] when the term list or correction bound is
    /// empty, or when the accumulator range cannot contain the modulus.
    pub const fn verify(self) -> Result<(), RangeProofError> {
        if self.modulus_bits == 0 || self.terms.is_empty() || self.correction_steps_max == 0 {
            return Err(RangeProofError::InvalidPlanShape);
        }
        self.range.verify(self.modulus_bits)
    }
}

/// Reduction plan selected by a maintained artifact.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PrimeReductionPlan {
    /// Small native reduction.
    Native {
        /// Width of the native reduction accumulator.
        word_bits: u8,
    },
    /// Barrett reduction.
    Barrett(BarrettPlan),
    /// Montgomery reduction.
    Montgomery(MontgomeryPlan),
    /// Solinas reduction.
    Solinas(SolinasPlan),
}

impl PrimeReductionPlan {
    /// Returns the selected family without exposing private field limbs.
    #[must_use]
    pub const fn kind(self) -> PrimeReductionKind {
        match self {
            Self::Native { .. } => PrimeReductionKind::Native,
            Self::Barrett(_) => PrimeReductionKind::Barrett,
            Self::Montgomery(_) => PrimeReductionKind::Montgomery,
            Self::Solinas(_) => PrimeReductionKind::Solinas,
        }
    }
}
