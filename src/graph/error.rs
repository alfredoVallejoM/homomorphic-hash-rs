//! Fail-closed errors for graph normalization and structural analysis.

use core::fmt;

use crate::SignatureError;

/// Failure produced before publishing a normalized graph or analysis result.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GraphError {
    /// An operation refers to a vertex outside the current builder.
    InvalidVertex {
        /// Supplied vertex index.
        index: usize,
        /// Number of vertices available at validation time.
        vertex_count: usize,
    },
    /// Zero is not a meaningful edge or incidence multiplicity.
    ZeroMultiplicity,
    /// Summing duplicate edge multiplicities exceeded `u64`.
    MultiplicityOverflow,
    /// A graph size cannot be represented by the stable `u64` wire contract.
    GraphTooLarge,
    /// The requested refinement profile has inconsistent round limits.
    InvalidProfile,
    /// No suitable set of field parameters could be derived.
    ParameterDerivationFailed,
    /// Explicit field parameters are degenerate or repeat evaluation points.
    InvalidFieldParameters,
    /// Two signatures use different field, encoder, profile or parameters.
    SignatureIdentityMismatch,
    /// The requested operation requires the fixed-round `Fast` profile.
    NonComposableProfile,
    /// Incremental updates require the existing vertex-index domain.
    IncrementalVertexCountMismatch {
        /// Vertex count retained by the current state.
        expected: usize,
        /// Vertex count supplied by the replacement graph.
        actual: usize,
    },
    /// A supposedly non-zero field factor could not be inverted.
    NonInvertibleAggregateFactor,
    /// An internal or adapter-provided canonical order is not a permutation.
    InvalidCanonicalOrder,
    /// A multi-field evidence bundle must contain at least one channel.
    EmptyEvidenceProfile,
    /// A multi-field evidence profile cannot count one channel twice.
    DuplicateEvidenceChannel,
    /// Evidence channels describe graphs with different exact cheap metadata.
    EvidenceGraphMetadataMismatch,
    /// Two evidence bundles were produced by different identified channel sets.
    EvidenceProfileMismatch,
    /// Two v2 graph analyses use different fields, recurrences or policies.
    DiscriminationProfileMismatch,
    /// Encoding graph metadata into the selected field failed.
    Signature(SignatureError),
}

impl fmt::Display for GraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVertex {
                index,
                vertex_count,
            } => write!(
                formatter,
                "vertex index {index} is outside graph with {vertex_count} vertices"
            ),
            Self::ZeroMultiplicity => formatter.write_str("graph multiplicity must be non-zero"),
            Self::MultiplicityOverflow => formatter.write_str("graph multiplicity overflow"),
            Self::GraphTooLarge => formatter.write_str("graph exceeds stable wire-size limits"),
            Self::InvalidProfile => formatter.write_str("invalid graph refinement profile"),
            Self::ParameterDerivationFailed => {
                formatter.write_str("could not derive non-degenerate graph field parameters")
            }
            Self::InvalidFieldParameters => formatter.write_str("invalid graph field parameters"),
            Self::SignatureIdentityMismatch => {
                formatter.write_str("incompatible graph signature identities")
            }
            Self::NonComposableProfile => {
                formatter.write_str("graph signature composition requires a fixed-round profile")
            }
            Self::IncrementalVertexCountMismatch { expected, actual } => write!(
                formatter,
                "incremental graph update keeps {expected} vertex indices but replacement has {actual}"
            ),
            Self::NonInvertibleAggregateFactor => {
                formatter.write_str("non-zero graph aggregate factor is not invertible")
            }
            Self::InvalidCanonicalOrder => {
                formatter.write_str("canonical vertex order is not a complete permutation")
            }
            Self::EmptyEvidenceProfile => {
                formatter.write_str("multi-field graph evidence profile is empty")
            }
            Self::DuplicateEvidenceChannel => {
                formatter.write_str("multi-field graph evidence repeats one signature identity")
            }
            Self::EvidenceGraphMetadataMismatch => formatter
                .write_str("multi-field evidence channels use different graph metadata"),
            Self::EvidenceProfileMismatch => {
                formatter.write_str("multi-field graph evidence profiles are incompatible")
            }
            Self::DiscriminationProfileMismatch => {
                formatter.write_str("v2 graph discrimination profiles are incompatible")
            }
            Self::Signature(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for GraphError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Signature(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SignatureError> for GraphError {
    fn from(error: SignatureError) -> Self {
        Self::Signature(error)
    }
}
