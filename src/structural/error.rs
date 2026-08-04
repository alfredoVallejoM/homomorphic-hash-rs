//! Typed failures for structural encoders and signatures.

use core::fmt;

/// Failure produced before a structural state is mutated.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SignatureError {
    /// Input exceeds the encoder's explicit resource ceiling.
    InputTooLarge {
        /// Configured byte ceiling.
        maximum: usize,
        /// Supplied byte length.
        actual: usize,
    },
    /// The caller supplied bytes that are not one canonical field element.
    NonCanonicalElement,
    /// A counter would exceed `u64`.
    CounterOverflow,
    /// Temporary framing storage could not be reserved.
    AllocationFailed,
    /// A tracked snapshot exceeds one configured restoration ceiling.
    SnapshotLimitExceeded(&'static str),
    /// Deterministic rejection sampling did not find a canonical element
    /// within its explicit work ceiling.
    HashToFieldExhausted,
    /// Two states use different field, encoder, law or parameters.
    IdentityMismatch,
    /// Sequence base is zero or one and cannot encode useful position.
    DegenerateSequenceBase,
    /// A multi-evaluation signature has no points or repeats one.
    InvalidEvaluationPoints,
    /// An operation requires at least one term.
    EmptyState,
    /// A zero factor was requested but the state records none.
    ZeroFactorAbsent,
    /// A tracked collection does not contain the exact raw item.
    ItemAbsent,
    /// Canonical signature bytes are malformed or belong to another context.
    InvalidWireFormat(&'static str),
    /// The selected encoder does not apply to the runtime field family.
    #[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
    EncoderFamilyMismatch,
    /// A runtime field rejected an element or arithmetic operation.
    #[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
    DynamicField(microfield::DynFieldError),
}

impl fmt::Display for SignatureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge { maximum, actual } => {
                write!(formatter, "input length {actual} exceeds maximum {maximum}")
            }
            Self::NonCanonicalElement => formatter.write_str("non-canonical field element"),
            Self::CounterOverflow => formatter.write_str("structural counter overflow"),
            Self::AllocationFailed => formatter.write_str("structural framing allocation failed"),
            Self::SnapshotLimitExceeded(limit) => {
                write!(formatter, "tracked snapshot exceeds {limit} limit")
            }
            Self::HashToFieldExhausted => {
                formatter.write_str("hash-to-field rejection limit exhausted")
            }
            Self::IdentityMismatch => formatter.write_str("incompatible structural identities"),
            Self::DegenerateSequenceBase => {
                formatter.write_str("sequence base must be neither zero nor one")
            }
            Self::InvalidEvaluationPoints => {
                formatter.write_str("evaluation points must be non-empty and pairwise distinct")
            }
            Self::EmptyState => formatter.write_str("structural state is empty"),
            Self::ZeroFactorAbsent => formatter.write_str("zero factor is absent"),
            Self::ItemAbsent => formatter.write_str("tracked item is absent"),
            Self::InvalidWireFormat(reason) => {
                write!(formatter, "invalid structural wire format: {reason}")
            }
            #[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
            Self::EncoderFamilyMismatch => {
                formatter.write_str("structural encoder does not match dynamic field family")
            }
            #[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
            Self::DynamicField(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SignatureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        #[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
        if let Self::DynamicField(error) = self {
            return Some(error);
        }
        None
    }
}

#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
impl From<microfield::DynFieldError> for SignatureError {
    fn from(error: microfield::DynFieldError) -> Self {
        Self::DynamicField(error)
    }
}
