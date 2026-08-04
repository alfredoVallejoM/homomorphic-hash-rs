#[cfg(feature = "legacy")]
pub mod algebra;
#[cfg(feature = "legacy")]
pub mod engine;
#[cfg(feature = "graph")]
pub mod graph;
#[cfg(feature = "signatures")]
pub mod structural;
#[cfg(feature = "legacy")]
pub mod topology;

// Re-exports publicos para facilitar el consumo de la libreria
#[cfg(feature = "legacy")]
pub use algebra::galois_256::GaloisSignature256;
#[cfg(feature = "legacy")]
pub use algebra::traits::FiniteField;

#[cfg(feature = "legacy")]
pub use topology::bloom_l1::{TopoBloomMask, TopologicalMask};
#[cfg(feature = "legacy")]
pub use topology::multiset::MultisetAggregator;
#[cfg(feature = "legacy")]
pub use topology::sequence::SequenceAggregator;
#[cfg(feature = "legacy")]
pub use topology::symmetric_difference::SymmetricDifferenceAggregator;
#[cfg(feature = "legacy")]
pub use topology::traits::HomomorphicAggregator;

#[cfg(feature = "legacy")]
pub use engine::canonizer::{
    CanonicalNode, CellularGaloisCanonizer, LegacyGraphAnalysis, TopologyProvider,
};
#[cfg(feature = "legacy")]
pub use engine::hasher::TopoHasher;
#[cfg(feature = "legacy")]
pub use engine::spectral_f251::SpectralEngineF251;
#[cfg(all(feature = "graph", feature = "legacy"))]
pub use graph::from_legacy_topology;
#[cfg(all(feature = "graph", feature = "dynamic-fields"))]
pub use graph::DynamicGraphFieldProfile;
#[cfg(feature = "graph")]
pub use graph::{
    AdaptiveFilterOutcome, AdaptiveFilterPolicy, AdaptiveFilterReport, AdaptiveFilterTier,
    AdaptiveGraphPipeline, AdaptiveTierReport, BoundedMotifProfile, CanonicalBudgetLimit,
    CanonicalGraphDag, CanonicalGraphDagLimits, CanonicalGraphDocument, CanonicalGraphEncodingId,
    CanonicalGraphForm, CanonicalGraphKey, CanonicalSearchBudget, CanonicalSearchReport,
    CanonicalizationPath, CellMomentCell, CellMomentProfile, CellMomentProfileId,
    ClosedWalkAnalysisStatus, ClosedWalkOperator, ClosedWalkQueryPlan, ClosedWalkQueryPlanId,
    ConnectedPatternCount, ConnectedPatternProfile, DegreeHistogram, DegreeHistogramBin,
    DegreeHistogramProfile, DegreeHistogramProfileId, DifferenceWitness, DiscreteCanonicalForm,
    DiscriminatingGraphAnalysis, DiscriminatingGraphComparison, DiscriminationRecommendation,
    ExactCanonicalOutcome, F251BatchGraphWorkspace, F251GraphLabeler, FastGraphAnalysis,
    FastGraphAnalysisView, FastGraphLabeler, FastGraphSignature, FastGraphSignatureView,
    GlobalGraphProfile, GlobalInvariantDigest, GraphAnalysisProfileId, GraphChannelInvalidation,
    GraphComparison, GraphComparisonReport, GraphDagNode, GraphDagNodeId, GraphDagResolveOutcome,
    GraphDagResolveReport, GraphDagUpdateKind, GraphDegeneracyReport, GraphDelta, GraphDeltaPolicy,
    GraphDeltaUpdatePath, GraphDeltaUpdateReport, GraphDiscriminationDigest, GraphDiscriminationId,
    GraphDiscriminationPolicy, GraphError, GraphEscalationAdvice, GraphEvidenceChannel,
    GraphEvidenceComparison, GraphEvidenceProfileId, GraphExecution, GraphFieldChannel,
    GraphFieldParameters, GraphFieldSuitability, GraphSchemaId, GraphSignatureId,
    GraphSubnetworkAdapter, GraphWorkspace, HybridGraphAnalysis, HyperedgeIncidence, Incidence,
    IncidenceGraph, IncidenceGraphBuilder, IncrementalGraphState, IncrementalGraphWorkspace,
    IncrementalUpdateStats, InvariantGraphDigest, LocalPairRefinementProfile,
    LocalPairRefinementProfileId, LoopPatternCatalog, LoopPatternCatalogId, MatrixAnalysisStatus,
    Microcanon, MicrocanonOutcome, MicrocanonPath, MicrocanonReport, MicrocanonStrategy,
    MicrocanonWorkspace, MotifAnalysisStatus, MultiFieldGraphEvidence,
    MultiFieldGraphEvidenceBuilder, PairRefinementStatus, PairedComparisonPath,
    PairedComparisonReport, PatternAnalysisStatus, PatternFieldFingerprint, PatternFingerprintId,
    PatternProductFingerprint, PreparedGraph, RefinementProfile, RelationDescriptor, RelationId,
    RelationalClosedWalkProfile, RelationalClosedWalkProfileId, RelationalMatrixProfile,
    RelationalMatrixProfileId, RelationalThetaProfile, RelationalThetaProfileId,
    StaticGraphFieldProfile, StructuralLabel, ThetaAnalysisStatus, TryCanonicalOutcome,
    VerifiedGraphMapping, VertexId, VertexKind, WeakComponentSummary, DEFAULT_F251_GRAPH_DOMAIN,
};
#[cfg(feature = "signatures")]
pub use structural::{
    AdditiveDelta, AdditiveSignature, AlgebraicResidual, ApplicationNamespace,
    BidirectionalSequenceSignature, BinaryPolynomialEncoder, BoundedSetReconciler,
    CanonicalElementEncoder, CompactSignature, DatabaseApplyReport, DatabaseApplyStatus,
    DatabaseColumn, DatabaseColumnType, DatabaseError, DatabaseReplayReport, DatabaseRow,
    DatabaseRowKey, DatabaseSchema, DatabaseSchemaId, DatabaseSummary, DatabaseTransactionLimits,
    DatabaseTransactionLog, DatabaseValue, DeltaApplyReport, DeltaApplyStatus, DeltaEnvelope,
    DeltaError, DeltaId, DeltaJournal, DeltaJournalLimits, DeltaReplayReport, DeltaVerification,
    DomainSeparatedHashToFieldEncoder, EncoderId, FileChunkProfile, FileChunkProfileId,
    HomomorphicSummaryRoot, HomomorphicSummaryTree, LegacyAffineEncoderV1, LegacyLinearEncoderV1,
    MultiEvaluationMultisetSignature, MultiEvaluationSequenceSignature, MultisetDelta,
    MultisetSignature, PartitionedDatabase, PrimeIntegerEncoder, ReconciliationError,
    ReconciliationLimits, ReconciliationProfileId, RecoveredSetDifference, RevisionedSignature,
    RowMutation, SequenceAppend, SequenceSignature, SequenceTrim, SetReconciliationSketch,
    SignatureAssurance, SignatureBuilder, SignatureContext, SignatureDelta, SignatureError,
    SignatureEvaluationProfile, SignatureFieldBinding, SignatureFieldProfile, SignatureId,
    SignatureLaw, SignatureProfile, StructuralEncoder, StructuralLaneEncoder, SummaryEditPath,
    SummaryEditReport, SummaryTreeError, SummaryTreeLimits, TrackedMultiset, TrackedSequence,
    TrackedSnapshotLimits, TransactionDelta, TransactionId,
};
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
pub use structural::{
    DynamicAdditiveSignature, DynamicAlgebraicResidual, DynamicBidirectionalSequenceSignature,
    DynamicMultiEvaluationMultisetSignature, DynamicMultiEvaluationSequenceSignature,
    DynamicMultisetSignature, DynamicSequenceSignature, DynamicSignatureBuilder,
    DynamicStructuralEncoder,
};
#[cfg(feature = "legacy")]
pub mod domains;
#[cfg(feature = "legacy")]
pub mod harness;
