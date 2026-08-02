pub mod algebra;
pub mod engine;
pub mod graph;
pub mod structural;
pub mod topology;

// Re-exports publicos para facilitar el consumo de la libreria
pub use algebra::galois_256::GaloisSignature256;
pub use algebra::traits::FiniteField;

pub use topology::bloom_l1::{TopoBloomMask, TopologicalMask};
pub use topology::multiset::MultisetAggregator;
pub use topology::sequence::SequenceAggregator;
pub use topology::symmetric_difference::SymmetricDifferenceAggregator;
pub use topology::traits::HomomorphicAggregator;

pub use engine::canonizer::{
    CanonicalNode, CellularGaloisCanonizer, LegacyGraphAnalysis, TopologyProvider,
};
pub use engine::hasher::TopoHasher;
pub use engine::spectral_f251::SpectralEngineF251;
pub use graph::{
    from_legacy_topology, BoundedMotifProfile, CanonicalBudgetLimit, CanonicalSearchBudget,
    CanonicalSearchReport, CanonicalizationPath, DiscreteCanonicalForm,
    DiscriminatingGraphAnalysis, DiscriminatingGraphComparison, DiscriminationRecommendation,
    ExactCanonicalOutcome, F251BatchGraphWorkspace, F251GraphLabeler, FastGraphAnalysis,
    FastGraphAnalysisView, FastGraphLabeler, FastGraphSignature, FastGraphSignatureView,
    GlobalGraphProfile, GlobalInvariantDigest, GraphDegeneracyReport, GraphDiscriminationDigest,
    GraphDiscriminationId, GraphDiscriminationPolicy, GraphError, GraphEscalationAdvice,
    GraphEvidenceChannel, GraphEvidenceComparison, GraphEvidenceProfileId, GraphExecution,
    GraphFieldParameters, GraphSignatureId, GraphWorkspace, HybridGraphAnalysis,
    HyperedgeIncidence, Incidence, IncidenceGraph, IncidenceGraphBuilder, IncrementalGraphState,
    IncrementalGraphWorkspace, IncrementalUpdateStats, InvariantGraphDigest, MotifAnalysisStatus,
    MultiFieldGraphEvidence, MultiFieldGraphEvidenceBuilder, PreparedGraph, RefinementProfile,
    RelationDescriptor, RelationId, StructuralLabel, TryCanonicalOutcome, VertexId, VertexKind,
    WeakComponentSummary, DEFAULT_F251_GRAPH_DOMAIN,
};
pub use structural::{
    AdditiveSignature, AlgebraicResidual, BidirectionalSequenceSignature, BinaryPolynomialEncoder,
    CanonicalElementEncoder, EncoderId, LegacyAffineEncoderV1, LegacyLinearEncoderV1,
    MultiEvaluationMultisetSignature, MultisetSignature, PrimeIntegerEncoder, SequenceSignature,
    SignatureContext, SignatureError, SignatureId, SignatureLaw, StructuralEncoder,
    TrackedMultiset, TrackedSequence,
};
#[cfg(feature = "dynamic-fields")]
pub use structural::{
    DynamicAdditiveSignature, DynamicAlgebraicResidual, DynamicBidirectionalSequenceSignature,
    DynamicMultiEvaluationMultisetSignature, DynamicMultisetSignature, DynamicSequenceSignature,
    DynamicStructuralEncoder,
};
pub mod domains;
pub mod harness;
