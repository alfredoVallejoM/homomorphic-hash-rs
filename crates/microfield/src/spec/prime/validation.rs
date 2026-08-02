//! Deterministic validation and replayable prime certificates.

use std::fmt;

use num_bigint::BigUint;
use num_integer::Integer as _;
use num_traits::{One as _, ToPrimitive as _, Zero as _};

use crate::{FieldId, ValidationAssurance};

use super::{GenerationLimits, NormalizedPrimeManifest, PocklingtonCertificate};

/// Completely validated prime definition and certificate report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedPrimeField {
    normalized: NormalizedPrimeManifest,
    field_id: FieldId,
    modulus_bits: u32,
    certificate_json: Vec<u8>,
}

impl ValidatedPrimeField {
    /// Returns the canonical input model.
    #[must_use]
    pub const fn normalized(&self) -> &NormalizedPrimeManifest {
        &self.normalized
    }

    /// Returns the semantic identity.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.field_id
    }

    /// Returns the significant modulus width.
    #[must_use]
    pub const fn modulus_bits(&self) -> u32 {
        self.modulus_bits
    }

    /// Returns the deterministic validation report/certificate JSON.
    #[must_use]
    pub fn certificate_json(&self) -> &[u8] {
        &self.certificate_json
    }

    /// Reports whether the result may authorize source generation.
    #[must_use]
    pub const fn permits_static_generation(&self) -> bool {
        self.normalized.assurance().permits_static_generation()
    }
}

/// Failure while proving or testing an external prime modulus.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PrimeValidationError {
    /// Modulus shape is invalid or the number is composite.
    InvalidModulus(String),
    /// An explicit resource ceiling stopped validation.
    LimitExceeded {
        /// Stable limit name.
        limit: &'static str,
        /// Configured maximum.
        maximum: u64,
    },
    /// A proven request above `u64` omitted its replayable certificate.
    CertificateRequired,
    /// A Pocklington certificate invariant failed.
    InvalidCertificate(String),
    /// A probable-prime result cannot authorize static Rust source.
    ProbablePrimeCannotGenerateStatic,
    /// Deterministic report serialization failed.
    Serialization(String),
}

impl fmt::Display for PrimeValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidModulus(reason) => write!(formatter, "invalid prime modulus: {reason}"),
            Self::LimitExceeded { limit, maximum } => {
                write!(formatter, "prime validation limit `{limit}` exceeded ({maximum})")
            }
            Self::CertificateRequired => formatter.write_str(
                "a replayable Pocklington certificate is required above the deterministic u64 range",
            ),
            Self::InvalidCertificate(reason) => {
                write!(formatter, "invalid Pocklington certificate: {reason}")
            }
            Self::ProbablePrimeCannotGenerateStatic => formatter.write_str(
                "probable-prime assurance cannot authorize a static field artifact",
            ),
            Self::Serialization(reason) => {
                write!(formatter, "serialize prime validation report: {reason}")
            }
        }
    }
}

impl std::error::Error for PrimeValidationError {}

pub(super) fn validate(
    normalized: NormalizedPrimeManifest,
    limits: GenerationLimits,
) -> Result<ValidatedPrimeField, PrimeValidationError> {
    let modulus = normalized.modulus();
    let modulus_bits = u32::try_from(modulus.bits()).unwrap_or(u32::MAX);
    if modulus_bits > limits.maximum_characteristic_bits {
        return Err(PrimeValidationError::LimitExceeded {
            limit: "maximum_characteristic_bits",
            maximum: u64::from(limits.maximum_characteristic_bits),
        });
    }
    if modulus < &BigUint::from(3_u8) || modulus.is_even() {
        return Err(PrimeValidationError::InvalidModulus(
            "the characteristic must be an odd integer of at least three".to_owned(),
        ));
    }
    let field_id = super::super::identity::field_id(normalized.identity_json().as_bytes());
    let mut steps = 0_u64;
    let primality = match normalized.assurance() {
        ValidationAssurance::Proven => {
            if let Some(candidate) = modulus.to_u64() {
                let bases = deterministic_u64_bases();
                if !miller_rabin_bases(
                    modulus,
                    &bases,
                    &mut steps,
                    limits.maximum_validation_steps,
                )? {
                    return Err(PrimeValidationError::InvalidModulus(
                        "deterministic Miller-Rabin found a composite".to_owned(),
                    ));
                }
                serde_json::json!({
                    "algorithm": "deterministic-miller-rabin-u64-v1",
                    "modulus": candidate.to_string(),
                    "bases": bases,
                    "range": "complete-u64",
                    "validation_steps": steps,
                })
            } else {
                let certificate = normalized
                    .certificate()
                    .ok_or(PrimeValidationError::CertificateRequired)?;
                verify_pocklington(modulus, certificate, limits, &mut steps)?
            }
        }
        ValidationAssurance::ProbablePrime { rounds } => {
            if !(16..=256).contains(&rounds) {
                return Err(PrimeValidationError::InvalidModulus(
                    "probable-prime rounds must be in 16..=256".to_owned(),
                ));
            }
            let bases = probable_bases(rounds);
            if !miller_rabin_bases(modulus, &bases, &mut steps, limits.maximum_validation_steps)? {
                return Err(PrimeValidationError::InvalidModulus(
                    "Miller-Rabin found a composite".to_owned(),
                ));
            }
            serde_json::json!({
                "algorithm": "miller-rabin-probable-v1",
                "modulus": normalized.modulus_decimal(),
                "bases": bases,
                "rounds": rounds,
                "validation_steps": steps,
            })
        }
    };
    let report = serde_json::json!({
        "schema": 1,
        "field_id": field_id,
        "validator": "microfield-prime-external-v1",
        "assurance": normalized.assurance(),
        "primality": primality,
    });
    let certificate_json = serde_json::to_vec_pretty(&report)
        .map_err(|error| PrimeValidationError::Serialization(error.to_string()))?;
    if certificate_json.len() as u64 > limits.maximum_certificate_bytes {
        return Err(PrimeValidationError::LimitExceeded {
            limit: "maximum_certificate_bytes",
            maximum: limits.maximum_certificate_bytes,
        });
    }
    Ok(ValidatedPrimeField {
        normalized,
        field_id,
        modulus_bits,
        certificate_json,
    })
}

fn verify_pocklington(
    modulus: &BigUint,
    certificate: &PocklingtonCertificate,
    limits: GenerationLimits,
    steps: &mut u64,
) -> Result<serde_json::Value, PrimeValidationError> {
    if certificate.algorithm != "pocklington-v1" {
        return Err(invalid_certificate("algorithm must be pocklington-v1"));
    }
    if certificate.factors.is_empty()
        || certificate.factors.len() > limits.maximum_certificate_factors as usize
    {
        return Err(invalid_certificate(
            "factor list must be non-empty and below the configured limit",
        ));
    }
    let known = parse_canonical_decimal("known_factor_product", &certificate.known_factor_product)?;
    let cofactor = parse_canonical_decimal("cofactor", &certificate.cofactor)?;
    let modulus_minus_one = modulus - 1_u8;
    if &known * &cofactor != modulus_minus_one {
        return Err(invalid_certificate(
            "known factor product times cofactor does not equal p - 1",
        ));
    }
    if &known * &known <= *modulus {
        return Err(invalid_certificate(
            "known factor product must be strictly greater than sqrt(p)",
        ));
    }
    let mut factors = certificate.factors.clone();
    factors.sort_unstable_by_key(|factor| factor.prime);
    if factors
        .windows(2)
        .any(|pair| pair[0].prime == pair[1].prime)
    {
        return Err(invalid_certificate("factor primes must be distinct"));
    }
    let mut reconstructed = BigUint::one();
    for factor in &factors {
        if factor.exponent == 0 || !is_prime_u64(factor.prime) {
            return Err(invalid_certificate(format!(
                "declared factor {} is not a proven prime power",
                factor.prime
            )));
        }
        reconstructed *= BigUint::from(factor.prime).pow(factor.exponent);
        let witness = BigUint::from(factor.witness) % modulus;
        if witness.is_zero() {
            return Err(invalid_certificate(format!(
                "witness for factor {} is zero modulo p",
                factor.prime
            )));
        }
        let fermat = modpow_counted(
            &witness,
            &modulus_minus_one,
            modulus,
            steps,
            limits.maximum_validation_steps,
        )?;
        if fermat != BigUint::one() {
            return Err(invalid_certificate(format!(
                "Fermat congruence failed for factor {}",
                factor.prime
            )));
        }
        let quotient = &modulus_minus_one / factor.prime;
        let residue = modpow_counted(
            &witness,
            &quotient,
            modulus,
            steps,
            limits.maximum_validation_steps,
        )?;
        let difference = if residue.is_zero() {
            modulus_minus_one.clone()
        } else {
            residue - 1_u8
        };
        if difference.gcd(modulus) != BigUint::one() {
            return Err(invalid_certificate(format!(
                "Pocklington gcd failed for factor {}",
                factor.prime
            )));
        }
    }
    if reconstructed != known {
        return Err(invalid_certificate(
            "factor powers do not reconstruct known_factor_product",
        ));
    }
    Ok(serde_json::json!({
        "algorithm": "pocklington-v1",
        "modulus": modulus.to_string(),
        "known_factor_product": known.to_string(),
        "cofactor": cofactor.to_string(),
        "factors": factors,
        "validation_steps": *steps,
    }))
}

fn parse_canonical_decimal(name: &str, value: &str) -> Result<BigUint, PrimeValidationError> {
    if value.is_empty()
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid_certificate(format!(
            "{name} must be canonical unsigned decimal"
        )));
    }
    BigUint::parse_bytes(value.as_bytes(), 10)
        .ok_or_else(|| invalid_certificate(format!("{name} is not an integer")))
}

fn deterministic_u64_bases() -> Vec<u64> {
    vec![2, 325, 9_375, 28_178, 450_775, 9_780_504, 1_795_265_022]
}

fn probable_bases(rounds: u32) -> Vec<u64> {
    const BASES: [u64; 32] = [
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89,
        97, 101, 103, 107, 109, 113, 127, 131,
    ];
    (0..rounds)
        .map(|index| BASES[index as usize % BASES.len()])
        .collect()
}

fn miller_rabin_bases(
    candidate: &BigUint,
    bases: &[u64],
    steps: &mut u64,
    maximum_steps: u64,
) -> Result<bool, PrimeValidationError> {
    let one = BigUint::one();
    let minus_one = candidate - &one;
    let mut odd_part = minus_one.clone();
    let mut twos = 0_u32;
    while odd_part.is_even() {
        odd_part >>= 1_u8;
        twos += 1;
    }
    'bases: for base in bases {
        let base = BigUint::from(*base) % candidate;
        if base.is_zero() {
            continue;
        }
        let mut witness = modpow_counted(&base, &odd_part, candidate, steps, maximum_steps)?;
        if witness == one || witness == minus_one {
            continue;
        }
        for _ in 1..twos {
            count_step(steps, maximum_steps)?;
            witness = (&witness * &witness) % candidate;
            if witness == minus_one {
                continue 'bases;
            }
        }
        return Ok(false);
    }
    Ok(true)
}

fn modpow_counted(
    base: &BigUint,
    exponent: &BigUint,
    modulus: &BigUint,
    steps: &mut u64,
    maximum_steps: u64,
) -> Result<BigUint, PrimeValidationError> {
    let mut result = BigUint::one();
    let mut base = base % modulus;
    let mut exponent = exponent.clone();
    while !exponent.is_zero() {
        if exponent.bit(0) {
            count_step(steps, maximum_steps)?;
            result = (result * &base) % modulus;
        }
        exponent >>= 1_u8;
        if !exponent.is_zero() {
            count_step(steps, maximum_steps)?;
            base = (&base * &base) % modulus;
        }
    }
    Ok(result)
}

fn count_step(steps: &mut u64, maximum: u64) -> Result<(), PrimeValidationError> {
    *steps = steps.saturating_add(1);
    if *steps > maximum {
        return Err(PrimeValidationError::LimitExceeded {
            limit: "maximum_validation_steps",
            maximum,
        });
    }
    Ok(())
}

fn is_prime_u64(candidate: u64) -> bool {
    if candidate < 2 {
        return false;
    }
    for prime in [2_u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if candidate == prime {
            return true;
        }
        if candidate.is_multiple_of(prime) {
            return false;
        }
    }
    let modulus = BigUint::from(candidate);
    miller_rabin_bases(&modulus, &deterministic_u64_bases(), &mut 0, u64::MAX).unwrap_or(false)
}

fn invalid_certificate(reason: impl Into<String>) -> PrimeValidationError {
    PrimeValidationError::InvalidCertificate(reason.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::prime::PrimeFieldManifest;

    #[test]
    fn proven_u64_rejects_complete_range_pseudoprime() {
        let source = "prime_schema_version=1\n[prime]\nname='bad_prime'\nmodulus='341550071728321'\n[encoding]\nbyte_order='little'\ninteger='canonical'\ncanonical_bytes=7\n[validation]\nassurance='proven'\n";
        let normalized = PrimeFieldManifest::parse_toml(source)
            .unwrap()
            .normalize()
            .unwrap();
        assert!(matches!(
            validate(normalized, GenerationLimits::default()),
            Err(PrimeValidationError::InvalidModulus(_))
        ));
    }
}
