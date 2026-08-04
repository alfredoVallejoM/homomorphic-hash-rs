//! Validation strength attached to externally supplied field definitions.

/// Mathematical assurance established for an external field modulus.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(
    any(feature = "generator", feature = "dynamic"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    any(feature = "generator", feature = "dynamic"),
    serde(rename_all = "snake_case", tag = "kind")
)]
pub enum ValidationAssurance {
    /// A deterministic proof or certificate established primality.
    Proven,
    /// Miller-Rabin established probable primality for the recorded rounds.
    ProbablePrime {
        /// Number of independent deterministic bases checked.
        rounds: u32,
    },
}

/// One proven prime-power factor and its Pocklington witness.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    any(feature = "generator", feature = "dynamic"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    any(feature = "generator", feature = "dynamic"),
    serde(deny_unknown_fields)
)]
pub struct PocklingtonFactor {
    /// Proven prime factor `q` of `p - 1`.
    pub prime: u64,
    /// Exponent of `q` in the known factor product.
    pub exponent: u32,
    /// Base satisfying the Pocklington congruences for `q`.
    pub witness: u64,
}

/// Replayable deterministic Pocklington certificate input.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    any(feature = "generator", feature = "dynamic"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    any(feature = "generator", feature = "dynamic"),
    serde(deny_unknown_fields)
)]
pub struct PocklingtonCertificate {
    /// Must be `pocklington-v1`.
    pub algorithm: String,
    /// Known completely factored divisor of `p - 1`.
    pub known_factor_product: String,
    /// Remaining quotient `(p - 1) / F`.
    pub cofactor: String,
    /// Distinct factor witnesses in arbitrary input order.
    pub factors: Vec<PocklingtonFactor>,
}

impl ValidationAssurance {
    /// Reports whether this assurance may authorize static source generation.
    #[must_use]
    pub const fn permits_static_generation(self) -> bool {
        matches!(self, Self::Proven)
    }
}
