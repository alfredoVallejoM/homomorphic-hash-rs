//! Rabin irreducibility validation for binary polynomial fields.

use crate::spec::{
    error::ValidationError,
    identity::field_id,
    model::{
        CertificateBundle, IrreducibilityCertificate, NormalizedManifest, RabinGcdCheck,
        SCHEMA_V1_MAXIMUM_DEGREE, ValidatedFieldSpec,
    },
    polynomial::BinaryPolynomial,
};

/// Configurable, stateless mathematical validator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidationEngine {
    maximum_degree: usize,
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self {
            maximum_degree: SCHEMA_V1_MAXIMUM_DEGREE,
        }
    }
}

impl ValidationEngine {
    /// Creates a validator with an explicit resource safety limit.
    #[must_use]
    pub const fn with_maximum_degree(maximum_degree: usize) -> Self {
        Self {
            maximum_degree: if maximum_degree < SCHEMA_V1_MAXIMUM_DEGREE {
                maximum_degree
            } else {
                SCHEMA_V1_MAXIMUM_DEGREE
            },
        }
    }

    /// Returns the effective validation degree limit.
    #[must_use]
    pub const fn maximum_degree(&self) -> usize {
        self.maximum_degree
    }

    /// Certifies a normalized manifest using Rabin's irreducibility criterion.
    ///
    /// # Errors
    ///
    /// Returns an error when the degree exceeds the configured limit or the
    /// modulus is reducible.
    pub fn validate(
        &self,
        normalized: NormalizedManifest,
    ) -> Result<ValidatedFieldSpec, ValidationError> {
        let descriptor = normalized.descriptor();
        let degree = descriptor.degree();
        if degree > self.maximum_degree {
            return Err(ValidationError::DegreeLimit {
                degree,
                maximum: self.maximum_degree,
            });
        }

        let modulus = BinaryPolynomial::from_exponents(descriptor.modulus_exponents());
        let x = BinaryPolynomial::x();
        let width = descriptor.canonical_bytes();
        let mut checks = Vec::new();

        for prime_divisor in prime_divisors(degree) {
            let frobenius_steps = degree / prime_divisor;
            let residue = repeated_square(&x, &modulus, frobenius_steps).xor(&x);
            let gcd = BinaryPolynomial::gcd(modulus.clone(), residue);
            let gcd_hex = gcd.to_fixed_hex(width);
            if !gcd.is_one() {
                return Err(ValidationError::ReduciblePolynomial {
                    prime_divisor,
                    gcd_hex,
                });
            }
            checks.push(RabinGcdCheck::new(prime_divisor, frobenius_steps, gcd_hex));
        }

        let final_residue = repeated_square(&x, &modulus, degree);
        if final_residue != x {
            return Err(ValidationError::FrobeniusMismatch {
                residue_hex: final_residue.to_fixed_hex(width),
            });
        }

        let id = field_id(normalized.identity_json().as_bytes());
        let certificate = CertificateBundle::new(
            id,
            IrreducibilityCertificate::new(
                degree,
                descriptor.modulus_exponents().to_vec(),
                checks,
                final_residue.to_fixed_hex(width),
            ),
        );
        Ok(ValidatedFieldSpec::new(normalized, id, certificate))
    }
}

fn repeated_square(
    value: &BinaryPolynomial,
    modulus: &BinaryPolynomial,
    count: usize,
) -> BinaryPolynomial {
    let mut result = value.clone();
    for _ in 0..count {
        result = result.square_mod(modulus);
    }
    result
}

fn prime_divisors(mut value: usize) -> Vec<usize> {
    let mut divisors = Vec::new();
    let mut candidate = 2;
    while candidate <= value / candidate {
        if value.is_multiple_of(candidate) {
            divisors.push(candidate);
            while value.is_multiple_of(candidate) {
                value /= candidate;
            }
        }
        candidate = if candidate == 2 { 3 } else { candidate + 2 };
    }
    if value > 1 {
        divisors.push(value);
    }
    divisors
}

#[cfg(test)]
mod tests {
    use super::prime_divisors;

    #[test]
    fn extracts_unique_prime_divisors() {
        assert_eq!(prime_divisors(128), vec![2]);
        assert_eq!(prime_divisors(255), vec![3, 5, 17]);
        assert_eq!(prime_divisors(2), vec![2]);
    }
}
