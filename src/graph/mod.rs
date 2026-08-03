//! Fast, relabeling-invariant structural analysis for large graphs.
//!
//! This module deliberately distinguishes finite-field structural signatures
//! from exact graph canonization. The fast path is linear per refinement round
//! and can use any statically generated `microfield` field. It emits canonical
//! bytes only for a discrete partition; an independent opt-in API diagnoses
//! degeneration and searches exact forms under explicit node/state budgets.
//! For general discrimination, [`FastGraphLabeler::analyze_discriminating`]
//! adds exact global invariants and budget-admitted motifs without starting an
//! unbounded search. Equality of any finite profile remains evidence, not an
//! isomorphism proof.
//! Fixed-round consumers can retain every layer in [`IncrementalGraphState`]
//! and publish audited local edits through a reusable
//! [`IncrementalGraphWorkspace`].

mod canon;
mod canonical;
mod error;
mod evidence;
mod global;
mod incremental;
mod labeler;
mod legacy;
mod model;
mod schema;

pub use canon::{
    CanonicalBudgetLimit, CanonicalGraphDocument, CanonicalGraphEncodingId, CanonicalGraphForm,
    CanonicalGraphKey, CanonicalSearchBudget, DifferenceWitness, GraphComparison,
    GraphComparisonReport, Microcanon, MicrocanonOutcome, MicrocanonPath, MicrocanonReport,
    MicrocanonStrategy, MicrocanonWorkspace, VerifiedGraphMapping,
};

pub use canonical::{
    CanonicalSearchReport, CanonicalizationPath, DiscriminationRecommendation,
    ExactCanonicalOutcome, GraphDegeneracyReport,
};
pub use error::GraphError;
pub use evidence::{
    GraphEvidenceChannel, GraphEvidenceComparison, GraphEvidenceProfileId, MultiFieldGraphEvidence,
    MultiFieldGraphEvidenceBuilder,
};
pub use global::{
    BoundedMotifProfile, DiscriminatingGraphAnalysis, DiscriminatingGraphComparison,
    GlobalGraphProfile, GlobalInvariantDigest, GraphDiscriminationDigest, GraphDiscriminationId,
    GraphDiscriminationPolicy, GraphEscalationAdvice, MotifAnalysisStatus, WeakComponentSummary,
};
pub use incremental::{IncrementalGraphState, IncrementalGraphWorkspace, IncrementalUpdateStats};
pub use labeler::{
    DiscreteCanonicalForm, F251BatchGraphWorkspace, F251GraphLabeler, FastGraphAnalysis,
    FastGraphAnalysisView, FastGraphLabeler, FastGraphSignature, FastGraphSignatureView,
    GraphExecution, GraphFieldParameters, GraphSignatureId, GraphWorkspace, HybridGraphAnalysis,
    InvariantGraphDigest, PreparedGraph, RefinementProfile, StructuralLabel, TryCanonicalOutcome,
    DEFAULT_F251_GRAPH_DOMAIN,
};
pub use legacy::from_legacy_topology;
pub use model::{
    HyperedgeIncidence, Incidence, IncidenceGraph, IncidenceGraphBuilder, RelationDescriptor,
    RelationId, VertexId, VertexKind,
};
pub use schema::{GraphAnalysisProfileId, GraphSchemaId};
