//! Errors shared by the public, allocation-free API.

use core::fmt;

/// Failure while decoding a canonical field representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    /// The input byte length does not match the field representation.
    LengthMismatch {
        /// Required byte length.
        expected: usize,
        /// Received byte length.
        actual: usize,
    },
    /// The byte string does not encode a canonical field element.
    NonCanonicalValue,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { expected, actual } => {
                write!(
                    formatter,
                    "canonical representation requires {expected} bytes, received {actual}"
                )
            }
            Self::NonCanonicalValue => formatter.write_str("non-canonical field representation"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

/// Failure reported before or during a batch operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchError {
    /// Output and operand lengths are not compatible.
    LengthMismatch {
        /// Output length.
        out: usize,
        /// Left-hand or unary-input length.
        lhs: usize,
        /// Right-hand length for a binary operation.
        rhs: Option<usize>,
    },
    /// A packed layout is incompatible with the selected engine.
    IncompatiblePacking,
    /// The requested execution backend is not available.
    BackendUnavailable,
}

impl fmt::Display for BatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { out, lhs, rhs } => {
                write!(formatter, "batch length mismatch: out={out}, lhs={lhs}")?;
                if let Some(rhs) = rhs {
                    write!(formatter, ", rhs={rhs}")?;
                }
                Ok(())
            }
            Self::IncompatiblePacking => formatter.write_str("incompatible packed layout"),
            Self::BackendUnavailable => formatter.write_str("requested backend is unavailable"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BatchError {}
