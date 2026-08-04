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
    /// An operation refers to a relation descriptor outside one normalized graph.
    InvalidRelation {
        /// Supplied relation index.
        index: usize,
        /// Number of relation descriptors available in this graph.
        relation_count: usize,
    },
    /// Zero is not a meaningful edge or incidence multiplicity.
    ZeroMultiplicity,
    /// Summing duplicate edge multiplicities exceeded `u64`.
    MultiplicityOverflow,
    /// A graph size cannot be represented by the stable `u64` wire contract.
    GraphTooLarge,
    /// The requested refinement profile has inconsistent round limits.
    InvalidProfile,
    /// A graph delta exceeds its hard command limit.
    GraphDeltaTooLarge,
    /// The same atomic delta contains conflicting commands.
    ConflictingGraphDelta,
    /// A requested relation removal has no matching normalized record.
    GraphDeltaRelationAbsent,
    /// A requested relation removal exceeds retained multiplicity.
    GraphDeltaMultiplicityUnderflow,
    /// Incremental delta admission uses an invalid command or cone threshold.
    InvalidGraphDeltaPolicy,
    /// Optimistic delta revision does not match the retained state.
    GraphDeltaRevisionMismatch {
        /// Revision requested by the transaction.
        expected: u64,
        /// Current persistent-state revision.
        actual: u64,
    },
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
    /// Canonical graph bytes are truncated, malformed or not normalized.
    InvalidCanonicalEncoding,
    /// Canonical graph bytes use an unsupported encoding version.
    UnsupportedCanonicalEncoding {
        /// Version found in the input envelope.
        version: u16,
    },
    /// A candidate graph mapping is not a complete exact isomorphism.
    InvalidGraphMapping,
    /// An invariant of the exact canonization implementation was violated.
    CanonicalizationInvariantViolation,
    /// A compact workspace was supplied to the retained reference strategy.
    IncompatibleCanonicalWorkspace,
    /// A connected-pattern catalog has unsupported order or loop limits.
    InvalidPatternCatalog,
    /// A compressed pattern fingerprint has no finite-field lanes.
    InvalidPatternFingerprint,
    /// Pattern profiles use different catalogs, fields or lane encoders.
    PatternProfileMismatch,
    /// An operation requires a complete pattern profile, not a skipped tier.
    PatternAnalysisIncomplete,
    /// A relational matrix profile has zero lanes or unsupported trace depth.
    InvalidMatrixProfile,
    /// Relational matrix profiles use different fields, encoders or depths.
    MatrixProfileMismatch,
    /// An operation requires a complete relational matrix profile.
    MatrixAnalysisIncomplete,
    /// A long closed-walk query is empty, contains zero, or is too large.
    InvalidClosedWalkPlan,
    /// A long closed-walk profile has no finite-field lanes.
    InvalidClosedWalkProfile,
    /// Long closed-walk profiles use different fields, encoders or queries.
    ClosedWalkProfileMismatch,
    /// An operation requires a complete long closed-walk profile.
    ClosedWalkAnalysisIncomplete,
    /// A cell-moment profile has zero lanes or zero retained powers.
    InvalidCellMomentProfile,
    /// Cell-moment profiles use different fields, encoders, depths or rounds.
    CellMomentProfileMismatch,
    /// Degree profiles use different fields, encoders or evaluation points.
    DegreeHistogramProfileMismatch,
    /// Localized pair refinement uses zero or unsupported rounds.
    InvalidPairRefinementProfile,
    /// Adaptive filtering uses zero lanes or unsupported pair-refinement rounds.
    InvalidAdaptiveFilterPolicy,
    /// A persistent graph-DAG snapshot exceeds configured resource limits.
    GraphDagLimitExceeded,
    /// A graph-DAG snapshot or transaction has invalid framing or references.
    InvalidGraphDagEncoding,
    /// Optimistic graph-DAG revision does not match retained state.
    GraphDagRevisionMismatch { expected: u64, actual: u64 },
    /// Reusing exact graph bytes with a different decomposition is ambiguous.
    GraphDagDependencyMismatch,
    /// A subnetwork selection repeats one vertex.
    DuplicateSubgraphVertex { index: usize },
    /// A closed subnetwork selection cuts an incidence boundary.
    OpenSubgraphBoundary { source: usize, target: usize },
    /// A selected vertex is not an entity where an entity was required.
    NonEntityCliqueVertex { index: usize },
    /// Selected vertices do not form the requested exact relational clique.
    MissingCliqueRelation { source: usize, target: usize },
    /// A theta-contraction profile has no finite-field lanes.
    InvalidThetaProfile,
    /// Theta profiles use different fields, encoders or catalogs.
    ThetaProfileMismatch,
    /// An operation requires a complete theta profile.
    ThetaAnalysisIncomplete,
    /// An incidence supplied to a hyperedge points at another auxiliary hyperedge.
    InvalidHyperedgeEndpoint {
        /// Supplied auxiliary vertex index.
        index: usize,
    },
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
            Self::InvalidRelation {
                index,
                relation_count,
            } => write!(
                formatter,
                "relation index {index} is outside graph with {relation_count} descriptors"
            ),
            Self::ZeroMultiplicity => formatter.write_str("graph multiplicity must be non-zero"),
            Self::MultiplicityOverflow => formatter.write_str("graph multiplicity overflow"),
            Self::GraphTooLarge => formatter.write_str("graph exceeds stable wire-size limits"),
            Self::InvalidProfile => formatter.write_str("invalid graph refinement profile"),
            Self::GraphDeltaTooLarge => formatter.write_str("graph delta exceeds command limit"),
            Self::ConflictingGraphDelta => {
                formatter.write_str("graph delta contains conflicting commands")
            }
            Self::GraphDeltaRelationAbsent => {
                formatter.write_str("graph delta relation is absent")
            }
            Self::GraphDeltaMultiplicityUnderflow => formatter
                .write_str("graph delta removes more multiplicity than retained"),
            Self::InvalidGraphDeltaPolicy => {
                formatter.write_str("invalid graph delta admission policy")
            }
            Self::GraphDeltaRevisionMismatch { expected, actual } => write!(
                formatter,
                "graph delta expects revision {expected}, current revision is {actual}"
            ),
            Self::ParameterDerivationFailed => {
                formatter.write_str("could not derive non-degenerate graph field parameters")
            }
            Self::InvalidFieldParameters => formatter.write_str("invalid graph field parameters"),
            Self::InvalidAdaptiveFilterPolicy => {
                formatter.write_str("invalid adaptive graph-filter policy")
            }
            Self::GraphDagLimitExceeded => formatter.write_str("graph DAG resource limit exceeded"),
            Self::InvalidGraphDagEncoding => formatter.write_str("invalid persistent graph DAG encoding"),
            Self::GraphDagRevisionMismatch { expected, actual } => write!(
                formatter,
                "graph DAG expects revision {expected}, current revision is {actual}"
            ),
            Self::GraphDagDependencyMismatch => formatter.write_str(
                "exact graph bytes already exist with a different DAG decomposition",
            ),
            Self::DuplicateSubgraphVertex { index } => {
                write!(formatter, "subgraph selection repeats vertex {index}")
            }
            Self::OpenSubgraphBoundary { source, target } => write!(
                formatter,
                "closed subgraph selection cuts incidence {source}->{target}"
            ),
            Self::NonEntityCliqueVertex { index } => {
                write!(formatter, "clique vertex {index} is not an entity")
            }
            Self::MissingCliqueRelation { source, target } => write!(
                formatter,
                "clique is missing requested relation {source}->{target}"
            ),
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
            Self::InvalidCanonicalEncoding => {
                formatter.write_str("invalid canonical graph encoding")
            }
            Self::UnsupportedCanonicalEncoding { version } => {
                write!(formatter, "unsupported canonical graph encoding version {version}")
            }
            Self::InvalidGraphMapping => {
                formatter.write_str("candidate mapping is not an exact graph isomorphism")
            }
            Self::CanonicalizationInvariantViolation => {
                formatter.write_str("exact graph canonization invariant violation")
            }
            Self::IncompatibleCanonicalWorkspace => {
                formatter.write_str("compact canonical workspace requires the G10 strategy")
            }
            Self::InvalidPatternCatalog => {
                formatter.write_str("invalid connected-pattern catalog limits")
            }
            Self::InvalidPatternFingerprint => {
                formatter.write_str("pattern fingerprint requires at least one lane")
            }
            Self::PatternProfileMismatch => {
                formatter.write_str("incompatible connected-pattern profiles")
            }
            Self::PatternAnalysisIncomplete => {
                formatter.write_str("connected-pattern analysis is incomplete")
            }
            Self::InvalidMatrixProfile => {
                formatter.write_str("invalid relational matrix profile")
            }
            Self::MatrixProfileMismatch => {
                formatter.write_str("incompatible relational matrix profiles")
            }
            Self::MatrixAnalysisIncomplete => {
                formatter.write_str("relational matrix analysis is incomplete")
            }
            Self::InvalidClosedWalkPlan => {
                formatter.write_str("invalid long closed-walk query plan")
            }
            Self::InvalidClosedWalkProfile => {
                formatter.write_str("long closed-walk profile requires at least one lane")
            }
            Self::ClosedWalkProfileMismatch => {
                formatter.write_str("incompatible long closed-walk profiles")
            }
            Self::ClosedWalkAnalysisIncomplete => {
                formatter.write_str("long closed-walk analysis is incomplete")
            }
            Self::InvalidCellMomentProfile => {
                formatter.write_str("invalid cell-moment profile")
            }
            Self::CellMomentProfileMismatch => {
                formatter.write_str("incompatible cell-moment profiles")
            }
            Self::DegreeHistogramProfileMismatch => {
                formatter.write_str("incompatible degree-histogram profiles")
            }
            Self::InvalidPairRefinementProfile => {
                formatter.write_str("invalid localized pair-refinement profile")
            }
            Self::InvalidThetaProfile => formatter.write_str("invalid theta-contraction profile"),
            Self::ThetaProfileMismatch => {
                formatter.write_str("incompatible theta-contraction profiles")
            }
            Self::ThetaAnalysisIncomplete => {
                formatter.write_str("theta-contraction analysis is incomplete")
            }
            Self::InvalidHyperedgeEndpoint { index } => write!(
                formatter,
                "hyperedge incidence endpoint {index} is not an entity vertex"
            ),
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
