//! Typed failures for dynamic contexts and homogeneous batches.

use core::fmt;

use crate::FieldId;

/// Failure while validating a context or operating on a dynamic element.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DynFieldError {
    /// One scalar element belongs to a different field presentation.
    FieldMismatch {
        /// Context identity.
        expected: FieldId,
        /// Element identity.
        actual: FieldId,
    },
    /// A name, degree, modulus, encoding or assurance is invalid.
    InvalidDefinition(String),
    /// An explicit validation resource ceiling was reached.
    LimitExceeded {
        /// Limit that stopped validation.
        limit: &'static str,
        /// Configured maximum.
        maximum: u64,
    },
    /// The byte representation has an incorrect size.
    LengthMismatch {
        /// Required bytes.
        expected: usize,
        /// Supplied bytes.
        actual: usize,
    },
    /// The integer or polynomial is outside the canonical range.
    NonCanonicalValue,
    /// Zero has no multiplicative inverse.
    DivisionByZero,
    /// A deterministic proof is unavailable for this input.
    ProofRequired,
}

impl fmt::Display for DynFieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldMismatch { expected, actual } => {
                write!(
                    formatter,
                    "dynamic field mismatch: expected {expected}, got {actual}"
                )
            }
            Self::InvalidDefinition(reason) => write!(formatter, "invalid dynamic field: {reason}"),
            Self::LimitExceeded { limit, maximum } => {
                write!(
                    formatter,
                    "dynamic validation limit `{limit}` exceeded ({maximum})"
                )
            }
            Self::LengthMismatch { expected, actual } => {
                write!(
                    formatter,
                    "expected {expected} canonical bytes, got {actual}"
                )
            }
            Self::NonCanonicalValue => formatter.write_str("non-canonical dynamic field element"),
            Self::DivisionByZero => formatter.write_str("zero has no multiplicative inverse"),
            Self::ProofRequired => formatter.write_str(
                "static-strength assurance requires a deterministic proof for this modulus",
            ),
        }
    }
}

impl std::error::Error for DynFieldError {}

/// Failure before or during one homogeneous dynamic batch operation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DynBatchError {
    /// A batch belongs to another field.
    FieldMismatch,
    /// Input and output lengths differ.
    LengthMismatch {
        /// Output length.
        output: usize,
        /// Left input length.
        lhs: usize,
        /// Optional right input length.
        rhs: Option<usize>,
    },
    /// A source element belongs to another field.
    ElementFieldMismatch {
        /// Index of the invalid source element.
        index: usize,
    },
    /// The underlying arithmetic failed.
    Arithmetic(DynFieldError),
}

impl fmt::Display for DynBatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldMismatch => formatter.write_str("dynamic batches use different fields"),
            Self::LengthMismatch { output, lhs, rhs } => {
                write!(
                    formatter,
                    "dynamic batch lengths differ: out={output}, lhs={lhs}"
                )?;
                if let Some(rhs) = rhs {
                    write!(formatter, ", rhs={rhs}")?;
                }
                Ok(())
            }
            Self::ElementFieldMismatch { index } => {
                write!(
                    formatter,
                    "dynamic element {index} belongs to another field"
                )
            }
            Self::Arithmetic(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for DynBatchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Arithmetic(error) => Some(error),
            _ => None,
        }
    }
}

impl From<DynFieldError> for DynBatchError {
    fn from(error: DynFieldError) -> Self {
        Self::Arithmetic(error)
    }
}
