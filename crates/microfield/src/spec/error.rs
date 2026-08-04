//! Typed failures produced by the generator pipeline.

use std::{fmt, path::PathBuf};

/// Failure while loading or parsing a field manifest.
#[derive(Debug)]
pub enum ManifestError {
    /// The manifest could not be read.
    Read {
        /// Path that was being read.
        path: PathBuf,
        /// Operating-system error.
        source: std::io::Error,
    },
    /// TOML syntax or type validation failed.
    Syntax(String),
    /// A key is not part of the strict schema.
    UnknownKey(String),
    /// The input exceeds the parser resource-safety limit.
    InputTooLarge {
        /// Input size in bytes.
        actual: u64,
        /// Maximum accepted size in bytes.
        maximum: usize,
    },
    /// The manifest uses a schema version unknown to this generator.
    UnsupportedSchema(u32),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "cannot read manifest {}: {source}",
                    path.display()
                )
            }
            Self::Syntax(message) => write!(formatter, "invalid manifest: {message}"),
            Self::UnknownKey(key) => write!(formatter, "unknown manifest key `{key}`"),
            Self::InputTooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "manifest has {actual} bytes; maximum is {maximum}"
                )
            }
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported manifest schema version {version}")
            }
        }
    }
}

impl std::error::Error for ManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Syntax(_)
            | Self::UnknownKey(_)
            | Self::InputTooLarge { .. }
            | Self::UnsupportedSchema(_) => None,
        }
    }
}

/// Failure while converting a parsed manifest into its canonical form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizationError {
    /// A field name is not stable `snake_case` ASCII.
    InvalidName(String),
    /// A numeric or structural invariant is invalid.
    InvalidValue {
        /// Manifest path of the invalid value.
        path: &'static str,
        /// Human-readable invariant violation.
        reason: String,
    },
    /// A schema option is well-formed but unsupported in version 1.
    UnsupportedValue {
        /// Manifest path of the unsupported option.
        path: &'static str,
        /// Received value.
        value: String,
    },
    /// Canonical serialization unexpectedly failed.
    Serialization(String),
}

impl fmt::Display for NormalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(
                formatter,
                "field name `{name}` must be 1..=64 lowercase ASCII letters, digits or underscores"
            ),
            Self::InvalidValue { path, reason } => {
                write!(formatter, "invalid `{path}`: {reason}")
            }
            Self::UnsupportedValue { path, value } => {
                write!(formatter, "unsupported `{path}` value `{value}`")
            }
            Self::Serialization(message) => {
                write!(formatter, "canonical serialization failed: {message}")
            }
        }
    }
}

impl std::error::Error for NormalizationError {}

/// Failure while certifying the mathematical field definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    /// The requested polynomial degree exceeds the configured safety bound.
    DegreeLimit {
        /// Manifest degree.
        degree: usize,
        /// Configured maximum.
        maximum: usize,
    },
    /// Rabin's criterion found a non-trivial factor.
    ReduciblePolynomial {
        /// Prime divisor used by the failed Rabin check.
        prime_divisor: usize,
        /// Greatest common divisor, encoded as a little-endian polynomial.
        gcd_hex: String,
    },
    /// Rabin's final Frobenius identity failed.
    FrobeniusMismatch {
        /// Computed final residue.
        residue_hex: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DegreeLimit { degree, maximum } => {
                write!(
                    formatter,
                    "degree {degree} exceeds validation limit {maximum}"
                )
            }
            Self::ReduciblePolynomial {
                prime_divisor,
                gcd_hex,
            } => write!(
                formatter,
                "modulus is reducible (Rabin divisor {prime_divisor}, gcd 0x{gcd_hex})"
            ),
            Self::FrobeniusMismatch { residue_hex } => write!(
                formatter,
                "modulus is reducible (final Frobenius residue 0x{residue_hex})"
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Failure while constructing plans or deterministic source artifacts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerationError {
    /// Deterministic JSON serialization failed.
    Serialization(String),
    /// An artifact path is absolute, empty or attempts traversal.
    InvalidArtifactPath(String),
    /// A generated artifact set contains the same path more than once.
    DuplicateArtifactPath(String),
    /// A plan from a different field was supplied to artifact generation.
    MismatchedPlan,
    /// A generated inversion chain failed symbolic verification.
    InvalidInversionPlan(String),
}

impl fmt::Display for GenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization(message) => write!(formatter, "serialization failed: {message}"),
            Self::InvalidArtifactPath(path) => write!(formatter, "invalid artifact path `{path}`"),
            Self::DuplicateArtifactPath(path) => {
                write!(formatter, "duplicate artifact path `{path}`")
            }
            Self::MismatchedPlan => {
                formatter.write_str("generation plan belongs to a different field")
            }
            Self::InvalidInversionPlan(message) => {
                write!(formatter, "invalid inversion plan: {message}")
            }
        }
    }
}

impl std::error::Error for GenerationError {}

/// A typed schema-v2 reference-vector contract violation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceVectorError {
    path: String,
    reason: String,
}

impl ReferenceVectorError {
    /// Returns the precise JSON path or semantic field that failed.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the violated invariant.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub(crate) fn new(path: String, reason: String) -> Self {
        Self { path, reason }
    }
}

impl fmt::Display for ReferenceVectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid reference vectors at `{}`: {}",
            self.path, self.reason
        )
    }
}

impl std::error::Error for ReferenceVectorError {}

/// Failure while executing the complete generator use case.
#[derive(Debug)]
pub enum PipelineError {
    /// Manifest loading failed.
    Manifest(ManifestError),
    /// Canonical normalization failed.
    Normalization(NormalizationError),
    /// Mathematical validation failed.
    Validation(ValidationError),
    /// Plan or artifact construction failed.
    Generation(GenerationError),
    /// An oracle response violates the typed reference-vector contract.
    ReferenceVectors(ReferenceVectorError),
    /// An infrastructure adapter rejected or failed publication.
    Adapter(String),
}

impl fmt::Display for PipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(error) => error.fmt(formatter),
            Self::Normalization(error) => error.fmt(formatter),
            Self::Validation(error) => error.fmt(formatter),
            Self::Generation(error) => error.fmt(formatter),
            Self::ReferenceVectors(error) => error.fmt(formatter),
            Self::Adapter(message) => write!(formatter, "adapter failed: {message}"),
        }
    }
}

impl std::error::Error for PipelineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Manifest(error) => Some(error),
            Self::Normalization(error) => Some(error),
            Self::Validation(error) => Some(error),
            Self::Generation(error) => Some(error),
            Self::ReferenceVectors(error) => Some(error),
            Self::Adapter(_) => None,
        }
    }
}

impl From<ManifestError> for PipelineError {
    fn from(error: ManifestError) -> Self {
        Self::Manifest(error)
    }
}

impl From<NormalizationError> for PipelineError {
    fn from(error: NormalizationError) -> Self {
        Self::Normalization(error)
    }
}

impl From<ValidationError> for PipelineError {
    fn from(error: ValidationError) -> Self {
        Self::Validation(error)
    }
}

impl From<GenerationError> for PipelineError {
    fn from(error: GenerationError) -> Self {
        Self::Generation(error)
    }
}

impl From<ReferenceVectorError> for PipelineError {
    fn from(error: ReferenceVectorError) -> Self {
        Self::ReferenceVectors(error)
    }
}
