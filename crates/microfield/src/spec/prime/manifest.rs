//! Strict schema-v1 manifest for external prime fields.

use std::{fmt, fmt::Write as _, fs, path::Path};

use num_bigint::BigUint;
use num_traits::Zero as _;
use serde::{Deserialize, Serialize};

use crate::{PocklingtonCertificate, ValidationAssurance};

/// Deterministic generation surface selected independently from field identity.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationProfile {
    /// Portable scalar and batch support only.
    PortableOnly,
    /// Portable plus structurally compatible explicit ISA adapters.
    #[default]
    MultiBackend,
    /// Multi-backend output plus audit traces and expanded vectors.
    Audit,
}

/// Defensive ceilings for external prime validation and emission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationLimits {
    /// Maximum characteristic width.
    pub maximum_characteristic_bits: u32,
    /// Maximum accepted certificate bytes.
    pub maximum_certificate_bytes: u64,
    /// Maximum Pocklington factors.
    pub maximum_certificate_factors: u32,
    /// Maximum generated payload bytes.
    pub maximum_generated_bytes: u64,
    /// Maximum deterministic validation operations.
    pub maximum_validation_steps: u64,
}

impl Default for GenerationLimits {
    fn default() -> Self {
        Self {
            maximum_characteristic_bits: 4_096,
            maximum_certificate_bytes: 1_048_576,
            maximum_certificate_factors: 4_096,
            maximum_generated_bytes: 16 * 1_048_576,
            maximum_validation_steps: 20_000_000,
        }
    }
}

/// Parsed prime schema before normalization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimeFieldManifest(RawManifest);

/// Canonical structurally valid prime manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedPrimeManifest {
    name: String,
    modulus: BigUint,
    modulus_decimal: String,
    canonical_bytes: usize,
    assurance: ValidationAssurance,
    certificate: Option<PocklingtonCertificate>,
    profile: GenerationProfile,
    canonical_toml: String,
    identity_json: String,
}

impl NormalizedPrimeManifest {
    /// Presentation name excluded from `FieldId`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Canonical unsigned decimal modulus.
    #[must_use]
    pub fn modulus_decimal(&self) -> &str {
        &self.modulus_decimal
    }

    /// Validated modulus integer.
    #[must_use]
    pub const fn modulus(&self) -> &BigUint {
        &self.modulus
    }

    /// Exact canonical byte width.
    #[must_use]
    pub const fn canonical_bytes(&self) -> usize {
        self.canonical_bytes
    }

    /// Requested validation strength.
    #[must_use]
    pub const fn assurance(&self) -> ValidationAssurance {
        self.assurance
    }

    /// Optional deterministic certificate supplied by the caller.
    #[must_use]
    pub const fn certificate(&self) -> Option<&PocklingtonCertificate> {
        self.certificate.as_ref()
    }

    /// Non-semantic generation profile.
    #[must_use]
    pub const fn profile(&self) -> GenerationProfile {
        self.profile
    }

    /// Stable canonical TOML representation.
    #[must_use]
    pub fn canonical_toml(&self) -> &str {
        &self.canonical_toml
    }

    /// Fixed-order minified schema-2 identity descriptor.
    #[must_use]
    pub fn identity_json(&self) -> &str {
        &self.identity_json
    }
}

/// Prime manifest parse or normalization failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum PrimeManifestError {
    /// File I/O failed.
    Read {
        /// Manifest path.
        path: std::path::PathBuf,
        /// Operating-system error.
        source: std::io::Error,
    },
    /// TOML syntax or shape is invalid.
    Parse(String),
    /// One strict schema invariant failed.
    InvalidValue {
        /// Stable field path.
        path: &'static str,
        /// Human-readable invariant.
        reason: String,
    },
}

impl fmt::Display for PrimeManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "read prime manifest `{}`: {source}",
                    path.display()
                )
            }
            Self::Parse(reason) => write!(formatter, "parse prime manifest: {reason}"),
            Self::InvalidValue { path, reason } => {
                write!(formatter, "invalid prime manifest `{path}`: {reason}")
            }
        }
    }
}

impl std::error::Error for PrimeManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    prime_schema_version: u32,
    prime: RawPrime,
    encoding: RawEncoding,
    validation: RawValidation,
    certificate: Option<PocklingtonCertificate>,
    #[serde(default)]
    build: RawBuild,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPrime {
    name: String,
    modulus: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEncoding {
    byte_order: String,
    integer: String,
    canonical_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawValidation {
    assurance: String,
    rounds: Option<u32>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBuild {
    #[serde(default)]
    profile: GenerationProfile,
}

impl PrimeFieldManifest {
    /// Parses one strict UTF-8 TOML document.
    ///
    /// # Errors
    ///
    /// Unknown fields and malformed types are rejected.
    pub fn parse_toml(source: &str) -> Result<Self, PrimeManifestError> {
        toml::from_str(source)
            .map(Self)
            .map_err(|error| PrimeManifestError::Parse(error.to_string()))
    }

    /// Loads one strict UTF-8 TOML document.
    ///
    /// # Errors
    ///
    /// Returns contextual I/O or strict TOML parsing failures.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PrimeManifestError> {
        let path = path.as_ref();
        let source = fs::read_to_string(path).map_err(|source| PrimeManifestError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse_toml(&source)
    }

    /// Produces the unique canonical form without claiming primality.
    ///
    /// # Errors
    ///
    /// Rejects unsupported schema values and non-canonical encodings.
    pub fn normalize(self) -> Result<NormalizedPrimeManifest, PrimeManifestError> {
        let raw = self.0;
        require(
            "prime_schema_version",
            raw.prime_schema_version == 1,
            "only schema 1 is supported",
        )?;
        validate_name(&raw.prime.name)?;
        require(
            "prime.modulus",
            !raw.prime.modulus.is_empty()
                && !raw.prime.modulus.starts_with('0')
                && raw.prime.modulus.bytes().all(|byte| byte.is_ascii_digit()),
            "must be canonical unsigned decimal",
        )?;
        let modulus = BigUint::parse_bytes(raw.prime.modulus.as_bytes(), 10)
            .ok_or_else(|| invalid("prime.modulus", "must contain a valid unsigned integer"))?;
        require(
            "prime.modulus",
            !modulus.is_zero(),
            "zero is not a field characteristic",
        )?;
        let expected_bytes = usize::try_from(modulus.bits().div_ceil(8)).map_err(|_| {
            invalid(
                "prime.modulus",
                "bit length cannot be represented on this target",
            )
        })?;
        require(
            "encoding.byte_order",
            raw.encoding.byte_order == "little",
            "schema 1 requires little",
        )?;
        require(
            "encoding.integer",
            raw.encoding.integer == "canonical",
            "schema 1 requires canonical",
        )?;
        require(
            "encoding.canonical_bytes",
            raw.encoding.canonical_bytes == expected_bytes,
            format!("expected {expected_bytes} bytes for this modulus"),
        )?;
        let assurance = match raw.validation.assurance.as_str() {
            "proven" => {
                require(
                    "validation.rounds",
                    raw.validation.rounds.is_none(),
                    "rounds are only valid for probable_prime",
                )?;
                ValidationAssurance::Proven
            }
            "probable_prime" => {
                let rounds = raw.validation.rounds.ok_or_else(|| {
                    invalid(
                        "validation.rounds",
                        "probable_prime requires an explicit round count",
                    )
                })?;
                ValidationAssurance::ProbablePrime { rounds }
            }
            _ => {
                return Err(invalid(
                    "validation.assurance",
                    "expected proven or probable_prime",
                ));
            }
        };
        require(
            "certificate",
            raw.certificate.is_none() || assurance == ValidationAssurance::Proven,
            "certificates cannot be attached to probable-prime requests",
        )?;

        let identity_json = format!(
            "{{\"schema\":2,\"characteristic\":\"{}\",\"degree\":1,\"basis\":{{\"kind\":\"prime\"}},\"modulus\":\"{}\",\"encoding\":{{\"byte_order\":\"little\",\"integer\":\"canonical\",\"bytes\":{expected_bytes}}}}}",
            raw.prime.modulus, raw.prime.modulus
        );
        let canonical_toml = canonical_toml(
            &raw.prime.name,
            &raw.prime.modulus,
            expected_bytes,
            assurance,
            raw.certificate.as_ref(),
            raw.build.profile,
        );
        Ok(NormalizedPrimeManifest {
            name: raw.prime.name,
            modulus,
            modulus_decimal: raw.prime.modulus,
            canonical_bytes: expected_bytes,
            assurance,
            certificate: raw.certificate,
            profile: raw.build.profile,
            canonical_toml,
            identity_json,
        })
    }
}

fn canonical_toml(
    name: &str,
    modulus: &str,
    bytes: usize,
    assurance: ValidationAssurance,
    certificate: Option<&PocklingtonCertificate>,
    profile: GenerationProfile,
) -> String {
    let mut result = format!(
        "prime_schema_version = 1\n\n[prime]\nname = \"{name}\"\nmodulus = \"{modulus}\"\n\n[encoding]\nbyte_order = \"little\"\ninteger = \"canonical\"\ncanonical_bytes = {bytes}\n\n[validation]\n"
    );
    match assurance {
        ValidationAssurance::Proven => result.push_str("assurance = \"proven\"\n"),
        ValidationAssurance::ProbablePrime { rounds } => {
            let _ = write!(
                result,
                "assurance = \"probable_prime\"\nrounds = {rounds}\n"
            );
        }
    }
    if let Some(certificate) = certificate {
        let mut factors = certificate.factors.clone();
        factors.sort_unstable_by_key(|factor| factor.prime);
        let _ = write!(
            result,
            "\n[certificate]\nalgorithm = \"{}\"\nknown_factor_product = \"{}\"\ncofactor = \"{}\"\n",
            certificate.algorithm, certificate.known_factor_product, certificate.cofactor
        );
        for factor in factors {
            let _ = write!(
                result,
                "\n[[certificate.factors]]\nprime = {}\nexponent = {}\nwitness = {}\n",
                factor.prime, factor.exponent, factor.witness
            );
        }
    }
    let profile = match profile {
        GenerationProfile::PortableOnly => "portable_only",
        GenerationProfile::MultiBackend => "multi_backend",
        GenerationProfile::Audit => "audit",
    };
    let _ = write!(result, "\n[build]\nprofile = \"{profile}\"\n");
    result
}

fn validate_name(name: &str) -> Result<(), PrimeManifestError> {
    let mut bytes = name.bytes();
    let valid_first = bytes.next().is_some_and(|byte| byte.is_ascii_lowercase());
    require(
        "prime.name",
        valid_first
            && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            && !name.ends_with('_')
            && !name.contains("__"),
        "must use normalized lower_snake_case",
    )
}

fn require(
    path: &'static str,
    condition: bool,
    reason: impl Into<String>,
) -> Result<(), PrimeManifestError> {
    if condition {
        Ok(())
    } else {
        Err(invalid(path, reason))
    }
}

fn invalid(path: &'static str, reason: impl Into<String>) -> PrimeManifestError {
    PrimeManifestError::InvalidValue {
        path,
        reason: reason.into(),
    }
}
