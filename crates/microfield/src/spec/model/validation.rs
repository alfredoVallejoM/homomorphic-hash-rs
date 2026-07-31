//! Mathematical validation results and certificates.

use serde::Serialize;

use crate::{FieldId, spec::model::NormalizedManifest};

/// A successful Rabin divisor check.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RabinGcdCheck {
    prime_divisor: usize,
    frobenius_steps: usize,
    gcd_hex_le: String,
}

impl RabinGcdCheck {
    /// Returns the prime divisor of the extension degree.
    #[must_use]
    pub const fn prime_divisor(&self) -> usize {
        self.prime_divisor
    }

    /// Returns the number of repeated squarings used by the check.
    #[must_use]
    pub const fn frobenius_steps(&self) -> usize {
        self.frobenius_steps
    }

    /// Returns the fixed-width little-endian encoding of the GCD.
    #[must_use]
    pub fn gcd_hex_le(&self) -> &str {
        &self.gcd_hex_le
    }

    pub(crate) fn new(prime_divisor: usize, frobenius_steps: usize, gcd_hex_le: String) -> Self {
        Self {
            prime_divisor,
            frobenius_steps,
            gcd_hex_le,
        }
    }
}

/// Deterministic certificate for an irreducible binary polynomial.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IrreducibilityCertificate {
    algorithm: &'static str,
    degree: usize,
    modulus_exponents_desc: Vec<usize>,
    checks: Vec<RabinGcdCheck>,
    final_frobenius_residue_hex_le: String,
}

impl IrreducibilityCertificate {
    /// Returns all prime-divisor checks.
    #[must_use]
    pub fn checks(&self) -> &[RabinGcdCheck] {
        &self.checks
    }

    /// Returns the fixed-width final Frobenius residue.
    #[must_use]
    pub fn final_residue_hex_le(&self) -> &str {
        &self.final_frobenius_residue_hex_le
    }

    pub(crate) fn new(
        degree: usize,
        modulus_exponents_desc: Vec<usize>,
        checks: Vec<RabinGcdCheck>,
        final_frobenius_residue_hex_le: String,
    ) -> Self {
        Self {
            algorithm: "rabin-gf2-v1",
            degree,
            modulus_exponents_desc,
            checks,
            final_frobenius_residue_hex_le,
        }
    }
}

/// Complete deterministic certificate bundle for a field.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CertificateBundle {
    schema: u32,
    field_id: FieldId,
    validator: &'static str,
    irreducibility: IrreducibilityCertificate,
}

impl CertificateBundle {
    /// Returns the certified field identity.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.field_id
    }

    /// Returns the irreducibility evidence.
    #[must_use]
    pub const fn irreducibility(&self) -> &IrreducibilityCertificate {
        &self.irreducibility
    }

    pub(crate) fn new(field_id: FieldId, irreducibility: IrreducibilityCertificate) -> Self {
        Self {
            schema: 1,
            field_id,
            validator: "microfield-rabin-v1",
            irreducibility,
        }
    }
}

/// Validated typestate accepted by planning and artifact generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedFieldSpec {
    normalized: NormalizedManifest,
    field_id: FieldId,
    certificate: CertificateBundle,
}

impl ValidatedFieldSpec {
    /// Returns the canonical normalized manifest.
    #[must_use]
    pub const fn normalized(&self) -> &NormalizedManifest {
        &self.normalized
    }

    /// Returns the stable semantic field identity.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.field_id
    }

    /// Returns the repeatable mathematical certificate.
    #[must_use]
    pub const fn certificate(&self) -> &CertificateBundle {
        &self.certificate
    }

    pub(crate) fn new(
        normalized: NormalizedManifest,
        field_id: FieldId,
        certificate: CertificateBundle,
    ) -> Self {
        Self {
            normalized,
            field_id,
            certificate,
        }
    }
}
