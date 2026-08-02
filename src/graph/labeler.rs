//! Generic finite-field refinement with fixed linear work per round.
//!
//! Exact degeneracy diagnosis and budgeted individualization are implemented
//! in the sibling canonicalization module so this hot-path implementation does
//! not acquire search state or exact-descriptor concerns.

use core::{cmp::Ordering, fmt};

use microfield::{CanonicalEncoding, Field, Invert, Pow, StaticField};
use rayon::prelude::*;
use sha2::{Digest as _, Sha256};

use crate::structural::{EncoderId, PrimeIntegerEncoder, StructuralEncoder};

use super::incremental::{AggregateDelta, GraphDependencyIndex, LabelUpdate};
use super::{
    GraphError, Incidence, IncidenceGraph, IncrementalGraphState, IncrementalGraphWorkspace,
    IncrementalUpdateStats, VertexId,
};

const PARAMETER_ATTEMPTS: u16 = 2048;
const SIGNATURE_MAGIC: &[u8; 4] = b"MFGR";
const CANONICAL_MAGIC: &[u8; 4] = b"MFCG";
const GRAPH_SCHEMA: u16 = 1;

/// Stable encoder domain used by the convenience F251 graph profile.
pub const DEFAULT_F251_GRAPH_DOMAIN: u64 = 0x4d46_4752_4632_3531;

/// Convenient F251 specialization of the generic static graph labeler.
pub type F251GraphLabeler<const K: usize> =
    FastGraphLabeler<microfield::Fp251V1, PrimeIntegerEncoder, K>;

/// Bounded execution policy for fast structural refinement.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RefinementProfile {
    /// Executes exactly `rounds` allocation-free propagation rounds.
    Fast {
        /// Exact number of local propagation rounds.
        rounds: usize,
    },
    /// Executes until the induced partition stabilizes or reaches a hard cap.
    Robust {
        /// Minimum number of rounds before testing stabilization.
        minimum_rounds: usize,
        /// Hard maximum; no unbounded graph search is performed.
        maximum_rounds: usize,
    },
}

impl RefinementProfile {
    /// Recommended low-latency profile with six propagation rounds.
    #[must_use]
    pub const fn fast() -> Self {
        Self::Fast { rounds: 6 }
    }

    /// Recommended bounded robustness profile.
    #[must_use]
    pub const fn robust() -> Self {
        Self::Robust {
            minimum_rounds: 4,
            maximum_rounds: 16,
        }
    }

    fn validate(self) -> Result<(), GraphError> {
        match self {
            Self::Fast { rounds } if rounds > 0 => Ok(()),
            Self::Robust {
                minimum_rounds,
                maximum_rounds,
            } if minimum_rounds > 0 && minimum_rounds <= maximum_rounds => Ok(()),
            _ => Err(GraphError::InvalidProfile),
        }
    }

    const fn maximum_rounds(self) -> usize {
        match self {
            Self::Fast { rounds } => rounds,
            Self::Robust { maximum_rounds, .. } => maximum_rounds,
        }
    }

    const fn minimum_rounds(self) -> usize {
        match self {
            Self::Fast { rounds } => rounds,
            Self::Robust { minimum_rounds, .. } => minimum_rounds,
        }
    }

    const fn is_robust(self) -> bool {
        matches!(self, Self::Robust { .. })
    }
}

impl Default for RefinementProfile {
    fn default() -> Self {
        Self::fast()
    }
}

/// Stable identity of field, encoder, lane parameters and refinement profile.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct GraphSignatureId([u8; 32]);

impl GraphSignatureId {
    /// Borrows the domain-separated digest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for GraphSignatureId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for GraphSignatureId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "GraphSignatureId({self})")
    }
}

/// One relabeling-invariant finite-field label with `K` independent lanes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralLabel<F: Field, const K: usize> {
    pub(super) lanes: [F; K],
}

impl<F: Field, const K: usize> StructuralLabel<F, K> {
    /// Borrows the field lanes in deterministic parameter order.
    #[must_use]
    pub const fn lanes(&self) -> &[F; K] {
        &self.lanes
    }
}

/// Deterministically derived field constants used by the graph recurrence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphFieldParameters<F: Field, const K: usize> {
    lane_salts: [F; K],
    neighbor_bases: [F; K],
    multiset_offsets: [F; K],
    outgoing_salts: [F; K],
    incoming_salts: [F; K],
    update_bases: [F; K],
    transcript_bases: [F; K],
    graph_offsets: [F; K],
}

impl<F, const K: usize> GraphFieldParameters<F, K>
where
    F: Field,
{
    /// Constructs an explicit experimental parameter suite.
    ///
    /// This is the extension point for research profiles over F251 or any
    /// generated field. The complete suite is bound into [`GraphSignatureId`].
    ///
    /// # Errors
    ///
    /// Rejects zero neighbor multipliers, zero/one Horner bases and repeated
    /// affine points.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        lane_salts: [F; K],
        neighbor_bases: [F; K],
        multiset_offsets: [F; K],
        outgoing_salts: [F; K],
        incoming_salts: [F; K],
        update_bases: [F; K],
        transcript_bases: [F; K],
        graph_offsets: [F; K],
    ) -> Result<Self, GraphError> {
        let parameters = Self {
            lane_salts,
            neighbor_bases,
            multiset_offsets,
            outgoing_salts,
            incoming_salts,
            update_bases,
            transcript_bases,
            graph_offsets,
        };
        parameters.validate_basic()?;
        Ok(parameters)
    }

    /// Lane salts mixed into exact initial labels.
    #[must_use]
    pub const fn lane_salts(&self) -> &[F; K] {
        &self.lane_salts
    }

    /// Affine points used by neighbor multiset products.
    #[must_use]
    pub const fn multiset_offsets(&self) -> &[F; K] {
        &self.multiset_offsets
    }

    /// Multipliers applied to neighboring labels before relation metadata.
    #[must_use]
    pub const fn neighbor_bases(&self) -> &[F; K] {
        &self.neighbor_bases
    }

    /// Direction separators for outgoing messages.
    #[must_use]
    pub const fn outgoing_salts(&self) -> &[F; K] {
        &self.outgoing_salts
    }

    /// Direction separators for incoming messages.
    #[must_use]
    pub const fn incoming_salts(&self) -> &[F; K] {
        &self.incoming_salts
    }

    /// Horner bases used by the vertex recurrence.
    #[must_use]
    pub const fn update_bases(&self) -> &[F; K] {
        &self.update_bases
    }

    /// Horner bases used by the graph-level round transcript.
    #[must_use]
    pub const fn transcript_bases(&self) -> &[F; K] {
        &self.transcript_bases
    }

    /// Affine points used to aggregate all vertex labels after each round.
    #[must_use]
    pub const fn graph_offsets(&self) -> &[F; K] {
        &self.graph_offsets
    }

    fn validate_basic(&self) -> Result<(), GraphError> {
        if K == 0
            || self.neighbor_bases.iter().any(Field::is_zero)
            || self.update_bases.iter().any(|value| !useful_base(*value))
            || self
                .transcript_bases
                .iter()
                .any(|value| !useful_base(*value))
            || has_duplicates(&self.multiset_offsets)
            || has_duplicates(&self.graph_offsets)
        {
            return Err(GraphError::InvalidFieldParameters);
        }
        Ok(())
    }
}

impl<F, const K: usize> GraphFieldParameters<F, K>
where
    F: Field + CanonicalEncoding,
{
    /// Derives versioned constants through the selected byte-to-field encoder.
    ///
    /// # Errors
    ///
    /// Rejects fields too small to provide `K` distinct affine points or an
    /// encoder that cannot encode the fixed parameter descriptors.
    pub fn derive<E: StructuralEncoder<F>>(encoder: &E) -> Result<Self, GraphError> {
        if K == 0 {
            return Err(GraphError::InvalidFieldParameters);
        }
        let lane_salts = derive_array(encoder, 1, |_| true)?;
        let neighbor_bases = derive_array(encoder, 2, |value| !value.is_zero())?;
        let multiset_offsets = derive_distinct_array(encoder, 3)?;
        let outgoing_salts = derive_array(encoder, 4, |_| true)?;
        let incoming_salts = derive_array(encoder, 5, |_| true)?;
        let update_bases = derive_array(encoder, 6, useful_base::<F>)?;
        let transcript_bases = derive_array(encoder, 7, useful_base::<F>)?;
        let graph_offsets = derive_distinct_array(encoder, 8)?;
        let parameters = Self {
            lane_salts,
            neighbor_bases,
            multiset_offsets,
            outgoing_salts,
            incoming_salts,
            update_bases,
            transcript_bases,
            graph_offsets,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    fn validate(&self) -> Result<(), GraphError> {
        self.validate_basic()
    }
}

fn useful_base<F: Field>(value: F) -> bool {
    !value.is_zero() && value != F::ONE
}

fn has_duplicates<F: Field, const K: usize>(values: &[F; K]) -> bool {
    (0..K).any(|left| (left + 1..K).any(|right| values[left] == values[right]))
}

fn derive_array<F, E, const K: usize>(
    encoder: &E,
    purpose: u8,
    accept: impl Fn(F) -> bool,
) -> Result<[F; K], GraphError>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    let mut output = [F::ZERO; K];
    for (lane, slot) in output.iter_mut().enumerate() {
        *slot = derive_candidate(encoder, purpose, lane, &accept)?;
    }
    Ok(output)
}

fn derive_distinct_array<F, E, const K: usize>(
    encoder: &E,
    purpose: u8,
) -> Result<[F; K], GraphError>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    let mut output = [F::ZERO; K];
    for lane in 0..K {
        output[lane] = derive_candidate(encoder, purpose, lane, |candidate| {
            !output[..lane].contains(&candidate)
        })?;
    }
    Ok(output)
}

fn derive_candidate<F, E>(
    encoder: &E,
    purpose: u8,
    lane: usize,
    accept: impl Fn(F) -> bool,
) -> Result<F, GraphError>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    let lane = u64::try_from(lane).map_err(|_| GraphError::ParameterDerivationFailed)?;
    let mut descriptor = [0_u8; 43];
    descriptor[..31].copy_from_slice(b"microfield-fast-graph-param-v1\0");
    descriptor[31] = purpose;
    descriptor[32..40].copy_from_slice(&lane.to_le_bytes());
    for attempt in 0..PARAMETER_ATTEMPTS {
        descriptor[40..42].copy_from_slice(&attempt.to_le_bytes());
        let candidate = encoder.encode(&descriptor[..42])?;
        if accept(candidate) {
            return Ok(candidate);
        }
    }
    Err(GraphError::ParameterDerivationFailed)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RoundAggregate<F: Field, const K: usize> {
    pub(super) nonzero_products: [F; K],
    pub(super) zero_factor_counts: [u64; K],
}

/// Self-identifying graph-level signature and exact cheap metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FastGraphSignature<F: Field, const K: usize> {
    signature_id: GraphSignatureId,
    lanes: [F; K],
    vertex_count: u64,
    incidence_count: u64,
    total_multiplicity: u64,
    rounds: u64,
    round_aggregates: Vec<RoundAggregate<F, K>>,
}

impl<F: Field, const K: usize> FastGraphSignature<F, K> {
    /// Complete identity of field, encoder, profile and recurrence parameters.
    #[must_use]
    pub const fn signature_id(&self) -> GraphSignatureId {
        self.signature_id
    }

    /// Final transcript lanes.
    #[must_use]
    pub const fn lanes(&self) -> &[F; K] {
        &self.lanes
    }

    /// Exact number of logical and auxiliary vertices.
    #[must_use]
    pub const fn vertex_count(&self) -> u64 {
        self.vertex_count
    }

    /// Exact number of normalized directed incidence records.
    #[must_use]
    pub const fn incidence_count(&self) -> u64 {
        self.incidence_count
    }

    /// Exact sum of directed incidence multiplicities.
    #[must_use]
    pub const fn total_multiplicity(&self) -> u64 {
        self.total_multiplicity
    }

    /// Number of propagation rounds actually executed.
    #[must_use]
    pub const fn rounds(&self) -> u64 {
        self.rounds
    }
}

impl<F, const K: usize> FastGraphSignature<F, K>
where
    F: Field + CanonicalEncoding,
{
    /// Serializes the non-cryptographic signature in a stable envelope.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(SIGNATURE_MAGIC);
        bytes.extend_from_slice(&GRAPH_SCHEMA.to_le_bytes());
        bytes.extend_from_slice(self.signature_id.as_bytes());
        bytes.extend_from_slice(&self.vertex_count.to_le_bytes());
        bytes.extend_from_slice(&self.incidence_count.to_le_bytes());
        bytes.extend_from_slice(&self.total_multiplicity.to_le_bytes());
        bytes.extend_from_slice(&self.rounds.to_le_bytes());
        bytes.extend_from_slice(&(K as u64).to_le_bytes());
        for lane in self.lanes {
            bytes.extend_from_slice(lane.to_canonical().as_ref());
        }
        bytes.extend_from_slice(&(self.round_aggregates.len() as u64).to_le_bytes());
        for aggregate in &self.round_aggregates {
            for lane in 0..K {
                bytes.extend_from_slice(&aggregate.zero_factor_counts[lane].to_le_bytes());
                bytes.extend_from_slice(aggregate.nonzero_products[lane].to_canonical().as_ref());
            }
        }
        bytes
    }
}

/// Complete result of one bounded fast analysis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FastGraphAnalysis<F: Field, const K: usize> {
    labels: Vec<StructuralLabel<F, K>>,
    partition: Vec<usize>,
    cell_count: usize,
    stable_partition: bool,
    signature: FastGraphSignature<F, K>,
}

impl<F: Field, const K: usize> FastGraphAnalysis<F, K> {
    /// Relabeling-invariant labels in original vertex storage order.
    #[must_use]
    pub fn labels(&self) -> &[StructuralLabel<F, K>] {
        &self.labels
    }

    /// Compact equality classes induced by the final field labels.
    #[must_use]
    pub fn partition(&self) -> &[usize] {
        &self.partition
    }

    /// Number of distinct final structural classes.
    #[must_use]
    pub const fn cell_count(&self) -> usize {
        self.cell_count
    }

    /// Whether robust refinement stopped because the equality partition ceased changing.
    #[must_use]
    pub const fn stable_partition(&self) -> bool {
        self.stable_partition
    }

    /// Graph-level transcript signature.
    #[must_use]
    pub const fn signature(&self) -> &FastGraphSignature<F, K> {
        &self.signature
    }
}

/// SHA-256 digest of an invariant descriptor richer than the field signature.
///
/// The digest is not homomorphic and does not prove graph isomorphism. It is
/// intentionally computed from exact labels, relation-class tuples and every
/// round histogram rather than from [`FastGraphSignature`] alone.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct InvariantGraphDigest([u8; 32]);

impl InvariantGraphDigest {
    /// Borrows the SHA-256 bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for InvariantGraphDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for InvariantGraphDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "InvariantGraphDigest({self})")
    }
}

/// Optional dual-channel result: composable field signature plus SHA-256.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridGraphAnalysis<F: Field, const K: usize> {
    structural: FastGraphAnalysis<F, K>,
    invariant_digest: InvariantGraphDigest,
}

impl<F: Field, const K: usize> HybridGraphAnalysis<F, K> {
    /// Borrows the field labels, partition and homomorphic transcript.
    #[must_use]
    pub const fn structural(&self) -> &FastGraphAnalysis<F, K> {
        &self.structural
    }

    /// Returns SHA-256 of the independent invariant descriptor.
    #[must_use]
    pub const fn invariant_digest(&self) -> InvariantGraphDigest {
        self.invariant_digest
    }
}

/// Exact graph bytes emitted only after field labels form a discrete partition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscreteCanonicalForm {
    bytes: Vec<u8>,
    original_to_canonical: Vec<VertexId>,
    canonical_to_original: Vec<VertexId>,
}

impl DiscreteCanonicalForm {
    /// Exact versioned graph bytes under the field-derived discrete order.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Maps each original index to its canonical position.
    #[must_use]
    pub fn original_to_canonical(&self) -> &[VertexId] {
        &self.original_to_canonical
    }

    /// Maps each canonical position back to the supplied graph.
    #[must_use]
    pub fn canonical_to_original(&self) -> &[VertexId] {
        &self.canonical_to_original
    }
}

/// Bounded result: an exact discrete form or the complete fast analysis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TryCanonicalOutcome<F: Field, const K: usize> {
    /// Every final class is a singleton, so sorting invariant labels is exact.
    Canonical(DiscreteCanonicalForm),
    /// Symmetry remains; no potentially exponential search was attempted.
    SymmetryRemaining(FastGraphAnalysis<F, K>),
}

/// Execution policy for the vertex-independent part of every refinement round.
///
/// This policy never enters [`GraphSignatureId`]: all variants execute the
/// same field recurrence and must produce byte-identical results.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GraphExecution {
    /// Deterministic single-threaded traversal in CSR vertex order.
    #[default]
    Sequential,
    /// Deterministic Rayon traversal above the supplied vertex threshold.
    Parallel {
        /// Small graphs remain sequential to avoid scheduling overhead.
        minimum_vertices: usize,
    },
}

impl GraphExecution {
    /// Conservative parallel policy for large graph batches.
    #[must_use]
    pub const fn parallel() -> Self {
        Self::Parallel {
            minimum_vertices: 1_024,
        }
    }

    fn uses_parallelism(self, vertex_count: usize) -> bool {
        matches!(
            self,
            Self::Parallel { minimum_vertices }
                if vertex_count >= minimum_vertices && rayon::current_num_threads() > 1
        )
    }
}

/// Graph metadata encoded once for repeated analyses with one labeler.
///
/// Besides the initial labels, this plan hoists every relation-, direction-
/// and lane-dependent affine constant out of the hot incidence loop. It
/// borrows the immutable normalized graph, so a plan cannot accidentally be
/// applied to another graph.
#[derive(Clone, Debug)]
pub struct PreparedGraph<'graph, F: Field, const K: usize> {
    graph: &'graph IncidenceGraph,
    signature_id: GraphSignatureId,
    initial_labels: Vec<StructuralLabel<F, K>>,
    outgoing_affine: Vec<[F; K]>,
    incoming_affine: Vec<[F; K]>,
    refine_round_tokens: Vec<F>,
    transcript_round_tokens: Vec<F>,
}

impl<F: Field, const K: usize> PreparedGraph<'_, F, K> {
    /// Returns the normalized graph retained by this plan.
    #[must_use]
    pub const fn graph(&self) -> &IncidenceGraph {
        self.graph
    }

    /// Returns the graph/field/encoder/profile identity of this preparation.
    #[must_use]
    pub const fn signature_id(&self) -> GraphSignatureId {
        self.signature_id
    }
}

/// Reusable allocation owner for repeated analyses of similarly sized graphs.
///
/// A borrowed analysis view keeps this workspace exclusively borrowed. Once
/// the view is dropped, the same buffers can be reused without allocation.
#[derive(Clone, Debug)]
pub struct GraphWorkspace<F: Field, const K: usize> {
    labels: Vec<StructuralLabel<F, K>>,
    next: Vec<StructuralLabel<F, K>>,
    partition: Vec<usize>,
    previous_partition: Vec<usize>,
    order: Vec<usize>,
    left_to_right: Vec<usize>,
    right_to_left: Vec<usize>,
    round_aggregates: Vec<RoundAggregate<F, K>>,
}

impl<F: Field, const K: usize> GraphWorkspace<F, K> {
    /// Creates an empty workspace that grows on first use.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            labels: Vec::new(),
            next: Vec::new(),
            partition: Vec::new(),
            previous_partition: Vec::new(),
            order: Vec::new(),
            left_to_right: Vec::new(),
            right_to_left: Vec::new(),
            round_aggregates: Vec::new(),
        }
    }

    /// Reserves all buffers for one prepared graph without running it.
    pub fn reserve_for(&mut self, vertex_count: usize, round_count: usize) {
        reserve_total(&mut self.labels, vertex_count);
        reserve_total(&mut self.next, vertex_count);
        reserve_total(&mut self.partition, vertex_count);
        reserve_total(&mut self.previous_partition, vertex_count);
        reserve_total(&mut self.order, vertex_count);
        reserve_total(&mut self.left_to_right, vertex_count);
        reserve_total(&mut self.right_to_left, vertex_count);
        reserve_total(&mut self.round_aggregates, round_count.saturating_add(1));
    }
}

impl<F: Field, const K: usize> Default for GraphWorkspace<F, K> {
    fn default() -> Self {
        Self::new()
    }
}

/// Reusable SoA scratch space for the explicit F251 batch-kernel experiment.
///
/// The generic CSR reduction remains scalar/gather-bound. This workspace
/// batches only the dense Horner update across vertices and exposes the
/// selected Microfield backend so applications and benchmarks can decide from
/// end-to-end evidence instead of assuming that SIMD is always faster.
#[derive(Clone)]
pub struct F251BatchGraphWorkspace<const K: usize> {
    core: GraphWorkspace<microfield::Fp251V1, K>,
    engine: microfield::Engine<microfield::Fp251V1>,
    out_products: Vec<[microfield::Fp251V1; K]>,
    in_products: Vec<[microfield::Fp251V1; K]>,
    out_zero_tokens: Vec<[microfield::Fp251V1; K]>,
    in_zero_tokens: Vec<[microfield::Fp251V1; K]>,
    first: Vec<microfield::Fp251V1>,
    second: Vec<microfield::Fp251V1>,
    rhs: Vec<microfield::Fp251V1>,
}

impl<const K: usize> F251BatchGraphWorkspace<K> {
    /// Detects a certified backend once and reserves the expected graph size.
    #[must_use]
    pub fn detected(expected_vertices: usize, rounds: usize) -> Self {
        let engine = microfield::Engine::<microfield::Fp251V1>::builder()
            .expected_batch(expected_vertices)
            .detect()
            .unwrap_or_else(|_| microfield::Engine::portable());
        let mut workspace = Self {
            core: GraphWorkspace::new(),
            engine,
            out_products: Vec::new(),
            in_products: Vec::new(),
            out_zero_tokens: Vec::new(),
            in_zero_tokens: Vec::new(),
            first: Vec::new(),
            second: Vec::new(),
            rhs: Vec::new(),
        };
        workspace.reserve_for(expected_vertices, rounds);
        workspace
    }

    /// Returns the immutable Microfield backend selected for dense updates.
    #[must_use]
    pub const fn backend_id(&self) -> microfield::BackendId {
        self.engine.backend_id()
    }

    /// Reserves every AoS/SoA bridge and analysis buffer.
    pub fn reserve_for(&mut self, vertex_count: usize, rounds: usize) {
        self.core.reserve_for(vertex_count, rounds);
        reserve_total(&mut self.out_products, vertex_count);
        reserve_total(&mut self.in_products, vertex_count);
        reserve_total(&mut self.out_zero_tokens, vertex_count);
        reserve_total(&mut self.in_zero_tokens, vertex_count);
        reserve_total(&mut self.first, vertex_count);
        reserve_total(&mut self.second, vertex_count);
        reserve_total(&mut self.rhs, vertex_count);
    }
}

fn reserve_total<T>(values: &mut Vec<T>, total: usize) {
    if values.capacity() < total {
        values.reserve_exact(total - values.len());
    }
}

/// Borrowed, allocation-free view of a signature retained by a workspace.
#[derive(Clone, Copy, Debug)]
pub struct FastGraphSignatureView<'workspace, F: Field, const K: usize> {
    signature_id: GraphSignatureId,
    lanes: [F; K],
    vertex_count: u64,
    incidence_count: u64,
    total_multiplicity: u64,
    rounds: u64,
    round_aggregates: &'workspace [RoundAggregate<F, K>],
}

impl<F: Field, const K: usize> FastGraphSignatureView<'_, F, K> {
    /// Complete identity of the field recurrence.
    #[must_use]
    pub const fn signature_id(self) -> GraphSignatureId {
        self.signature_id
    }

    /// Final transcript lanes.
    #[must_use]
    pub const fn lanes(self) -> [F; K] {
        self.lanes
    }

    /// Number of logical and auxiliary vertices.
    #[must_use]
    pub const fn vertex_count(self) -> u64 {
        self.vertex_count
    }

    /// Number of normalized directed incidence records.
    #[must_use]
    pub const fn incidence_count(self) -> u64 {
        self.incidence_count
    }

    /// Exact sum of incidence multiplicities.
    #[must_use]
    pub const fn total_multiplicity(self) -> u64 {
        self.total_multiplicity
    }

    /// Number of rounds actually executed.
    #[must_use]
    pub const fn rounds(self) -> u64 {
        self.rounds
    }

    /// Materializes an owned signature when it must outlive the workspace.
    #[must_use]
    pub fn to_owned(self) -> FastGraphSignature<F, K> {
        FastGraphSignature {
            signature_id: self.signature_id,
            lanes: self.lanes,
            vertex_count: self.vertex_count,
            incidence_count: self.incidence_count,
            total_multiplicity: self.total_multiplicity,
            rounds: self.rounds,
            round_aggregates: self.round_aggregates.to_vec(),
        }
    }
}

/// Borrowed analysis result backed by [`GraphWorkspace`].
#[derive(Clone, Copy, Debug)]
pub struct FastGraphAnalysisView<'workspace, F: Field, const K: usize> {
    labels: &'workspace [StructuralLabel<F, K>],
    partition: &'workspace [usize],
    cell_count: usize,
    stable_partition: bool,
    signature: FastGraphSignatureView<'workspace, F, K>,
}

impl<'workspace, F: Field, const K: usize> FastGraphAnalysisView<'workspace, F, K> {
    /// Final labels in the input graph's vertex storage order.
    #[must_use]
    pub const fn labels(self) -> &'workspace [StructuralLabel<F, K>] {
        self.labels
    }

    /// Compact equality classes induced by the final labels.
    #[must_use]
    pub const fn partition(self) -> &'workspace [usize] {
        self.partition
    }

    /// Number of final structural cells.
    #[must_use]
    pub const fn cell_count(self) -> usize {
        self.cell_count
    }

    /// Whether a robust profile stopped after partition stabilization.
    #[must_use]
    pub const fn stable_partition(self) -> bool {
        self.stable_partition
    }

    /// Borrowed graph signature.
    #[must_use]
    pub const fn signature(self) -> FastGraphSignatureView<'workspace, F, K> {
        self.signature
    }

    /// Materializes the complete owned result when persistence is required.
    #[must_use]
    pub fn to_owned(self) -> FastGraphAnalysis<F, K> {
        FastGraphAnalysis {
            labels: self.labels.to_vec(),
            partition: self.partition.to_vec(),
            cell_count: self.cell_count,
            stable_partition: self.stable_partition,
            signature: self.signature.to_owned(),
        }
    }
}

/// Static, zero-dispatch graph refinement over one generated finite field.
#[derive(Clone, Debug)]
pub struct FastGraphLabeler<F, E, const K: usize>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    encoder: E,
    profile: RefinementProfile,
    parameters: GraphFieldParameters<F, K>,
    signature_id: GraphSignatureId,
}

impl<F, E, const K: usize> FastGraphLabeler<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField + Pow,
    E: StructuralEncoder<F>,
{
    /// Creates a labeler and derives non-degenerate parameters for its field.
    ///
    /// # Errors
    ///
    /// Rejects invalid bounds, fields too small for `K`, or encoder failures.
    pub fn new(encoder: E, profile: RefinementProfile) -> Result<Self, GraphError> {
        let parameters = GraphFieldParameters::derive(&encoder)?;
        Self::with_parameters(encoder, profile, parameters)
    }

    /// Creates a labeler with explicit, identity-bound field parameters.
    ///
    /// # Errors
    ///
    /// Rejects invalid bounds, degenerate bases and repeated affine points.
    pub fn with_parameters(
        encoder: E,
        profile: RefinementProfile,
        parameters: GraphFieldParameters<F, K>,
    ) -> Result<Self, GraphError> {
        profile.validate()?;
        parameters.validate()?;
        let signature_id = derive_signature_id::<F, E, K>(&encoder, profile, &parameters);
        Ok(Self {
            encoder,
            profile,
            parameters,
            signature_id,
        })
    }

    /// Returns the bounded execution policy.
    #[must_use]
    pub const fn profile(&self) -> RefinementProfile {
        self.profile
    }

    /// Returns the selected field constants.
    #[must_use]
    pub const fn parameters(&self) -> &GraphFieldParameters<F, K> {
        &self.parameters
    }

    /// Returns the complete compatibility identity.
    #[must_use]
    pub const fn signature_id(&self) -> GraphSignatureId {
        self.signature_id
    }

    /// Encodes immutable graph metadata once for repeated analyses.
    ///
    /// Relation descriptors, direction salts, affine offsets, initial labels
    /// and round-domain tokens are all hoisted out of the refinement loop.
    ///
    /// # Errors
    ///
    /// Rejects graph-size overflows or metadata that the selected encoder
    /// cannot represent.
    pub fn prepare<'graph>(
        &self,
        graph: &'graph IncidenceGraph,
    ) -> Result<PreparedGraph<'graph, F, K>, GraphError> {
        u64::try_from(graph.vertex_count()).map_err(|_| GraphError::GraphTooLarge)?;
        u64::try_from(graph.incidence_count()).map_err(|_| GraphError::GraphTooLarge)?;
        let descriptors = self.encode_descriptors(graph)?;
        let outgoing_affine = descriptors
            .iter()
            .map(|descriptor| {
                core::array::from_fn(|lane| {
                    descriptor
                        .add(self.parameters.outgoing_salts[lane])
                        .add(self.parameters.multiset_offsets[lane])
                })
            })
            .collect();
        let incoming_affine = descriptors
            .iter()
            .map(|descriptor| {
                core::array::from_fn(|lane| {
                    descriptor
                        .add(self.parameters.incoming_salts[lane])
                        .add(self.parameters.multiset_offsets[lane])
                })
            })
            .collect();
        let mut refine_round_tokens = Vec::with_capacity(self.profile.maximum_rounds() + 1);
        let mut transcript_round_tokens = Vec::with_capacity(self.profile.maximum_rounds() + 1);
        for round in 0..=self.profile.maximum_rounds() {
            let round = u64::try_from(round).map_err(|_| GraphError::GraphTooLarge)?;
            refine_round_tokens.push(self.encode_counter(3, round)?);
            transcript_round_tokens.push(self.encode_counter(6, round)?);
        }
        Ok(PreparedGraph {
            graph,
            signature_id: self.signature_id,
            initial_labels: self.initial_labels(graph)?,
            outgoing_affine,
            incoming_affine,
            refine_round_tokens,
            transcript_round_tokens,
        })
    }

    /// Labels a graph with fixed linear work per propagation round.
    ///
    /// # Errors
    ///
    /// Rejects metadata encoding and stable-size overflows without publishing
    /// a partial analysis.
    pub fn analyze(&self, graph: &IncidenceGraph) -> Result<FastGraphAnalysis<F, K>, GraphError> {
        let prepared = self.prepare(graph)?;
        let mut workspace = GraphWorkspace::new();
        Ok(self
            .analyze_prepared_with_workspace(&prepared, &mut workspace, GraphExecution::Sequential)?
            .to_owned())
    }

    /// Runs a prepared graph into caller-owned reusable buffers.
    ///
    /// After [`GraphWorkspace::reserve_for`] and one warm-up run the sequential
    /// variant performs no heap allocation. Rayon may allocate scheduler state.
    /// The returned view cannot outlive or alias a subsequent mutation of the
    /// workspace.
    ///
    /// # Errors
    ///
    /// Rejects a preparation produced by another labeler and propagates exact
    /// counter overflows without exposing a partial view.
    pub fn analyze_prepared_with_workspace<'workspace>(
        &self,
        prepared: &PreparedGraph<'_, F, K>,
        workspace: &'workspace mut GraphWorkspace<F, K>,
        execution: GraphExecution,
    ) -> Result<FastGraphAnalysisView<'workspace, F, K>, GraphError> {
        self.analyze_prepared_observed(prepared, workspace, execution, |_, _| Ok(()))
    }

    /// Computes the hybrid field/SHA result while reusing a prepared graph and
    /// all linear-refinement buffers.
    ///
    /// SHA entry digests and their sorting storage remain intentionally owned
    /// by the returned result path; the zero-allocation guarantee applies to
    /// [`Self::analyze_prepared_with_workspace`], not to this opt-in channel.
    ///
    /// # Errors
    ///
    /// Propagates preparation identity, encoding and exact-size failures.
    pub fn analyze_prepared_hybrid_with_workspace(
        &self,
        prepared: &PreparedGraph<'_, F, K>,
        workspace: &mut GraphWorkspace<F, K>,
        execution: GraphExecution,
    ) -> Result<HybridGraphAnalysis<F, K>, GraphError> {
        let mut round_digests = Vec::with_capacity(self.profile.maximum_rounds() + 1);
        let structural = self
            .analyze_prepared_observed(prepared, workspace, execution, |round, labels| {
                round_digests.push(round_histogram_digest(round, labels)?);
                Ok(())
            })?
            .to_owned();
        let invariant_digest = invariant_graph_digest(prepared.graph, &structural, &round_digests)?;
        Ok(HybridGraphAnalysis {
            structural,
            invariant_digest,
        })
    }

    /// Computes the field analysis and an independent invariant SHA-256 channel.
    ///
    /// The additional digest sorts per-round label digests and final exact
    /// relation-class digests. It reduces accidental global collisions when
    /// those richer descriptors differ, but remains a fingerprint rather than
    /// an exact isomorphism proof.
    ///
    /// # Errors
    ///
    /// Propagates the same fail-closed metadata and size errors as [`Self::analyze`].
    pub fn analyze_hybrid(
        &self,
        graph: &IncidenceGraph,
    ) -> Result<HybridGraphAnalysis<F, K>, GraphError> {
        let prepared = self.prepare(graph)?;
        let mut workspace = GraphWorkspace::new();
        self.analyze_prepared_hybrid_with_workspace(
            &prepared,
            &mut workspace,
            GraphExecution::Sequential,
        )
    }

    fn analyze_prepared_observed<'workspace, O>(
        &self,
        prepared: &PreparedGraph<'_, F, K>,
        workspace: &'workspace mut GraphWorkspace<F, K>,
        execution: GraphExecution,
        mut observe_round: O,
    ) -> Result<FastGraphAnalysisView<'workspace, F, K>, GraphError>
    where
        O: FnMut(usize, &[StructuralLabel<F, K>]) -> Result<(), GraphError>,
    {
        if prepared.signature_id != self.signature_id {
            return Err(GraphError::SignatureIdentityMismatch);
        }
        let graph = prepared.graph;
        let vertex_count =
            u64::try_from(graph.vertex_count()).map_err(|_| GraphError::GraphTooLarge)?;
        let incidence_count =
            u64::try_from(graph.incidence_count()).map_err(|_| GraphError::GraphTooLarge)?;
        workspace.reserve_for(graph.vertex_count(), self.profile.maximum_rounds());
        workspace.labels.clone_from(&prepared.initial_labels);
        workspace.next.resize(
            graph.vertex_count(),
            StructuralLabel {
                lanes: [F::ZERO; K],
            },
        );
        workspace.round_aggregates.clear();
        workspace.partition.clear();
        workspace.previous_partition.clear();
        workspace.order.clear();

        let mut transcript = [F::ZERO; K];
        let initial_aggregate = self.aggregate_labels(&workspace.labels)?;
        self.append_round_aggregate_with_token(
            &mut transcript,
            initial_aggregate,
            prepared.transcript_round_tokens[0],
        )?;
        workspace.round_aggregates.push(initial_aggregate);
        observe_round(0, &workspace.labels)?;

        let mut stable_partition = false;
        let mut rounds_executed = 0_usize;
        let mut previous_cells = 0_usize;
        if self.profile.is_robust() {
            previous_cells = partition_labels_into(
                &workspace.labels,
                &mut workspace.order,
                &mut workspace.previous_partition,
            );
        }

        for round in 1..=self.profile.maximum_rounds() {
            self.refine_prepared_round(
                prepared,
                &workspace.labels,
                &mut workspace.next,
                round,
                execution,
            )?;
            core::mem::swap(&mut workspace.labels, &mut workspace.next);
            rounds_executed = round;
            let aggregate = self.aggregate_labels(&workspace.labels)?;
            self.append_round_aggregate_with_token(
                &mut transcript,
                aggregate,
                prepared.transcript_round_tokens[round],
            )?;
            workspace.round_aggregates.push(aggregate);
            observe_round(round, &workspace.labels)?;

            if self.profile.is_robust() {
                let current_cells = partition_labels_into(
                    &workspace.labels,
                    &mut workspace.order,
                    &mut workspace.partition,
                );
                if round >= self.profile.minimum_rounds()
                    && same_partition_slices(
                        &workspace.previous_partition,
                        previous_cells,
                        &workspace.partition,
                        current_cells,
                        &mut workspace.left_to_right,
                        &mut workspace.right_to_left,
                    )
                {
                    stable_partition = true;
                    break;
                }
                workspace
                    .previous_partition
                    .clone_from(&workspace.partition);
                previous_cells = current_cells;
            }
        }

        let cell_count = if self.profile.is_robust() && stable_partition {
            workspace
                .partition
                .iter()
                .copied()
                .max()
                .map_or(0, |maximum| maximum + 1)
        } else {
            partition_labels_into(
                &workspace.labels,
                &mut workspace.order,
                &mut workspace.partition,
            )
        };
        let rounds = u64::try_from(rounds_executed).map_err(|_| GraphError::GraphTooLarge)?;
        Ok(FastGraphAnalysisView {
            labels: &workspace.labels,
            partition: &workspace.partition,
            cell_count,
            stable_partition,
            signature: FastGraphSignatureView {
                signature_id: self.signature_id,
                lanes: transcript,
                vertex_count,
                incidence_count,
                total_multiplicity: graph.total_multiplicity(),
                rounds,
                round_aggregates: &workspace.round_aggregates,
            },
        })
    }

    /// Combines fixed-round signatures of disjoint graph components.
    ///
    /// Every round retains its commutative multiset product and exact number
    /// of zero factors, so the result equals analyzing the disjoint union under
    /// the same labeler. SHA-256 hybrid digests are intentionally not part of
    /// this algebraic operation.
    ///
    /// # Errors
    ///
    /// Rejects robust/adaptive schedules, incompatible identities, different
    /// round counts and all exact counter overflows.
    pub fn combine_disjoint(
        &self,
        left: &FastGraphSignature<F, K>,
        right: &FastGraphSignature<F, K>,
    ) -> Result<FastGraphSignature<F, K>, GraphError> {
        if !matches!(self.profile, RefinementProfile::Fast { .. }) {
            return Err(GraphError::NonComposableProfile);
        }
        if left.signature_id != self.signature_id
            || right.signature_id != self.signature_id
            || left.rounds != right.rounds
            || left.round_aggregates.len() != right.round_aggregates.len()
        {
            return Err(GraphError::SignatureIdentityMismatch);
        }
        let vertex_count = left
            .vertex_count
            .checked_add(right.vertex_count)
            .ok_or(GraphError::GraphTooLarge)?;
        let incidence_count = left
            .incidence_count
            .checked_add(right.incidence_count)
            .ok_or(GraphError::GraphTooLarge)?;
        let total_multiplicity = left
            .total_multiplicity
            .checked_add(right.total_multiplicity)
            .ok_or(GraphError::MultiplicityOverflow)?;
        let mut transcript = [F::ZERO; K];
        let mut round_aggregates = Vec::with_capacity(left.round_aggregates.len());
        for (round, (left_round, right_round)) in left
            .round_aggregates
            .iter()
            .zip(&right.round_aggregates)
            .enumerate()
        {
            let mut aggregate = RoundAggregate {
                nonzero_products: [F::ONE; K],
                zero_factor_counts: [0_u64; K],
            };
            for lane in 0..K {
                aggregate.nonzero_products[lane] =
                    left_round.nonzero_products[lane].mul(right_round.nonzero_products[lane]);
                aggregate.zero_factor_counts[lane] = left_round.zero_factor_counts[lane]
                    .checked_add(right_round.zero_factor_counts[lane])
                    .ok_or(GraphError::MultiplicityOverflow)?;
            }
            self.append_round_aggregate(&mut transcript, aggregate, round)?;
            round_aggregates.push(aggregate);
        }
        Ok(FastGraphSignature {
            signature_id: self.signature_id,
            lanes: transcript,
            vertex_count,
            incidence_count,
            total_multiplicity,
            rounds: left.rounds,
            round_aggregates,
        })
    }

    /// Attempts exact canonical output without ever entering exponential search.
    ///
    /// A non-discrete result explicitly returns [`TryCanonicalOutcome::SymmetryRemaining`].
    ///
    /// # Errors
    ///
    /// Propagates normalization metadata encoding and stable-size failures.
    pub fn try_canonicalize(
        &self,
        graph: &IncidenceGraph,
    ) -> Result<TryCanonicalOutcome<F, K>, GraphError> {
        let analysis = self.analyze(graph)?;
        if analysis.cell_count != graph.vertex_count() {
            return Ok(TryCanonicalOutcome::SymmetryRemaining(analysis));
        }
        let form = discrete_form(graph, &analysis.labels, self.signature_id)?;
        Ok(TryCanonicalOutcome::Canonical(form))
    }

    fn encode_descriptors(&self, graph: &IncidenceGraph) -> Result<Vec<F>, GraphError> {
        graph
            .descriptors()
            .iter()
            .map(|descriptor| {
                let mut bytes = Vec::with_capacity(
                    1 + 8 + descriptor.relation().len() + 8 + descriptor.role().len(),
                );
                bytes.push(2);
                append_bytes(&mut bytes, descriptor.relation())?;
                append_bytes(&mut bytes, descriptor.role())?;
                self.encoder.encode(&bytes).map_err(GraphError::from)
            })
            .collect()
    }

    fn initial_labels(
        &self,
        graph: &IncidenceGraph,
    ) -> Result<Vec<StructuralLabel<F, K>>, GraphError> {
        (0..graph.vertex_count())
            .map(|index| {
                let vertex = VertexId::new(index);
                let incoming = sum_multiplicity(graph.incoming(vertex))?;
                let outgoing = sum_multiplicity(graph.outgoing(vertex))?;
                let mut bytes = Vec::with_capacity(26 + graph.vertex_label(vertex).len());
                bytes.push(1);
                bytes.push(graph.vertex_kind(vertex) as u8);
                append_bytes(&mut bytes, graph.vertex_label(vertex))?;
                bytes.extend_from_slice(&incoming.to_le_bytes());
                bytes.extend_from_slice(&outgoing.to_le_bytes());
                let exact = self.encoder.encode(&bytes)?;
                Ok(StructuralLabel {
                    lanes: core::array::from_fn(|lane| exact.add(self.parameters.lane_salts[lane])),
                })
            })
            .collect()
    }

    fn refine_prepared_round(
        &self,
        prepared: &PreparedGraph<'_, F, K>,
        labels: &[StructuralLabel<F, K>],
        output: &mut [StructuralLabel<F, K>],
        round: usize,
        execution: GraphExecution,
    ) -> Result<(), GraphError> {
        let update = |index: usize, slot: &mut StructuralLabel<F, K>| {
            self.refine_prepared_vertex(prepared, labels, index, slot, round)
        };
        if execution.uses_parallelism(output.len()) {
            output
                .par_iter_mut()
                .enumerate()
                .try_for_each(|(index, slot)| update(index, slot))?;
        } else {
            for (index, slot) in output.iter_mut().enumerate() {
                update(index, slot)?;
            }
        }
        Ok(())
    }

    fn refine_prepared_vertex(
        &self,
        prepared: &PreparedGraph<'_, F, K>,
        labels: &[StructuralLabel<F, K>],
        index: usize,
        slot: &mut StructuralLabel<F, K>,
        round: usize,
    ) -> Result<(), GraphError> {
        self.refine_prepared_vertex_with(prepared, index, slot, round, &|label_index| {
            labels[label_index]
        })
    }

    fn refine_prepared_vertex_with<G>(
        &self,
        prepared: &PreparedGraph<'_, F, K>,
        index: usize,
        slot: &mut StructuralLabel<F, K>,
        round: usize,
        label_at: &G,
    ) -> Result<(), GraphError>
    where
        G: Fn(usize) -> StructuralLabel<F, K> + Sync,
    {
        let vertex = VertexId::new(index);
        let (out_product, out_zeros) = self.neighborhood_product_prepared_with(
            prepared.graph.outgoing(vertex),
            &prepared.outgoing_affine,
            label_at,
        )?;
        let (in_product, in_zeros) = self.neighborhood_product_prepared_with(
            prepared.graph.incoming(vertex),
            &prepared.incoming_affine,
            label_at,
        )?;
        let mut out_zero_tokens = [F::ZERO; K];
        let mut in_zero_tokens = [F::ZERO; K];
        for lane in 0..K {
            out_zero_tokens[lane] = self.encode_optional_counter(4, out_zeros[lane])?;
            in_zero_tokens[lane] = self.encode_optional_counter(5, in_zeros[lane])?;
        }
        let round_token = prepared.refine_round_tokens[round];
        let previous = label_at(index);
        slot.lanes = core::array::from_fn(|lane| {
            let base = self.parameters.update_bases[lane];
            previous.lanes[lane]
                .mul(base)
                .add(out_product[lane])
                .mul(base)
                .add(out_zero_tokens[lane].add(self.parameters.lane_salts[lane]))
                .mul(base)
                .add(in_product[lane])
                .mul(base)
                .add(in_zero_tokens[lane].add(self.parameters.lane_salts[lane]))
                .mul(base)
                .add(round_token.add(self.parameters.lane_salts[lane]))
        });
        Ok(())
    }

    fn neighborhood_product_prepared(
        &self,
        incidences: &[Incidence],
        affine: &[[F; K]],
        labels: &[StructuralLabel<F, K>],
    ) -> Result<([F; K], [u64; K]), GraphError> {
        self.neighborhood_product_prepared_with(incidences, affine, &|index| labels[index])
    }

    fn neighborhood_product_prepared_with<G>(
        &self,
        incidences: &[Incidence],
        affine: &[[F; K]],
        label_at: &G,
    ) -> Result<([F; K], [u64; K]), GraphError>
    where
        G: Fn(usize) -> StructuralLabel<F, K> + Sync,
    {
        let mut products = [F::ONE; K];
        let mut zeros = [0_u64; K];
        for incidence in incidences {
            let neighbor = label_at(incidence.neighbor().index());
            let relation_affine = affine[incidence.relation().index()];
            for lane in 0..K {
                let factor = neighbor.lanes[lane]
                    .mul(self.parameters.neighbor_bases[lane])
                    .add(relation_affine[lane]);
                if factor.is_zero() {
                    zeros[lane] = zeros[lane]
                        .checked_add(incidence.multiplicity())
                        .ok_or(GraphError::MultiplicityOverflow)?;
                } else {
                    let contribution = match incidence.multiplicity() {
                        1 => factor,
                        2 => factor.square(),
                        3 => factor.square().mul(factor),
                        4 => factor.square().square(),
                        multiplicity => factor.pow(&[multiplicity]),
                    };
                    products[lane] = products[lane].mul(contribution);
                }
            }
        }
        Ok((products, zeros))
    }

    fn aggregate_labels(
        &self,
        labels: &[StructuralLabel<F, K>],
    ) -> Result<RoundAggregate<F, K>, GraphError> {
        let mut products = [F::ONE; K];
        let mut zeros = [0_u64; K];
        for label in labels {
            for lane in 0..K {
                let factor = label.lanes[lane].add(self.parameters.graph_offsets[lane]);
                if factor.is_zero() {
                    zeros[lane] = zeros[lane]
                        .checked_add(1)
                        .ok_or(GraphError::MultiplicityOverflow)?;
                } else {
                    products[lane] = products[lane].mul(factor);
                }
            }
        }
        Ok(RoundAggregate {
            nonzero_products: products,
            zero_factor_counts: zeros,
        })
    }

    fn append_round_aggregate(
        &self,
        transcript: &mut [F; K],
        aggregate: RoundAggregate<F, K>,
        round: usize,
    ) -> Result<(), GraphError> {
        let round = u64::try_from(round).map_err(|_| GraphError::GraphTooLarge)?;
        let round_token = self.encode_counter(6, round)?;
        self.append_round_aggregate_with_token(transcript, aggregate, round_token)
    }

    fn append_round_aggregate_with_token(
        &self,
        transcript: &mut [F; K],
        aggregate: RoundAggregate<F, K>,
        round_token: F,
    ) -> Result<(), GraphError> {
        let mut zero_tokens = [F::ZERO; K];
        for (lane, zero_token) in zero_tokens.iter_mut().enumerate() {
            *zero_token = self.encode_optional_counter(7, aggregate.zero_factor_counts[lane])?;
        }
        for lane in 0..K {
            let base = self.parameters.transcript_bases[lane];
            transcript[lane] = transcript[lane]
                .mul(base)
                .add(aggregate.nonzero_products[lane])
                .mul(base)
                .add(zero_tokens[lane].add(self.parameters.lane_salts[lane]))
                .mul(base)
                .add(round_token.add(self.parameters.lane_salts[lane]));
        }
        Ok(())
    }

    fn encode_optional_counter(&self, tag: u8, value: u64) -> Result<F, GraphError> {
        if value == 0 {
            Ok(F::ZERO)
        } else {
            self.encode_counter(tag, value)
        }
    }

    fn encode_counter(&self, tag: u8, value: u64) -> Result<F, GraphError> {
        let mut bytes = [0_u8; 9];
        bytes[0] = tag;
        bytes[1..].copy_from_slice(&value.to_le_bytes());
        self.encoder.encode(&bytes).map_err(GraphError::from)
    }
}

impl<F, E, const K: usize> FastGraphLabeler<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
    E: StructuralEncoder<F>,
{
    /// Creates a persistent fixed-round state for transactional local updates.
    ///
    /// The supplied normalized graph is consumed so the state can retain the
    /// exact previous input without cloning it. All `R + 1` label layers are
    /// recorded once; subsequent edits replay only their dependency cone.
    ///
    /// # Errors
    ///
    /// Rejects adaptive profiles and propagates the normal preparation and
    /// exact-size failures.
    pub fn incremental_state(
        &self,
        graph: IncidenceGraph,
    ) -> Result<IncrementalGraphState<F, K>, GraphError> {
        self.incremental_state_with_execution(graph, GraphExecution::Sequential)
    }

    /// Creates an incremental state using an explicit initial execution policy.
    ///
    /// The policy affects only how the first complete analysis is scheduled;
    /// it never enters the signature identity.
    ///
    /// # Errors
    ///
    /// Rejects adaptive profiles and propagates preparation failures.
    pub fn incremental_state_with_execution(
        &self,
        graph: IncidenceGraph,
        execution: GraphExecution,
    ) -> Result<IncrementalGraphState<F, K>, GraphError> {
        if !matches!(self.profile, RefinementProfile::Fast { .. }) {
            return Err(GraphError::NonComposableProfile);
        }
        let rounds = self.profile.maximum_rounds();
        let vertex_count = graph.vertex_count();
        let layers = rounds.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
        let history_capacity = vertex_count
            .checked_mul(layers)
            .ok_or(GraphError::GraphTooLarge)?;
        let prepared = self.prepare(&graph)?;
        let mut workspace = GraphWorkspace::new();
        let mut round_labels = Vec::with_capacity(history_capacity);
        let analysis = self
            .analyze_prepared_observed(&prepared, &mut workspace, execution, |_, labels| {
                round_labels.extend_from_slice(labels);
                Ok(())
            })?
            .to_owned();
        let order = workspace.order.clone();
        let mut dependencies = GraphDependencyIndex::default();
        let mut stack = Vec::new();
        rebuild_dependency_index(&graph, &mut dependencies, &mut stack)?;
        Ok(IncrementalGraphState {
            graph,
            signature_id: self.signature_id,
            rounds,
            round_labels,
            order,
            partition: analysis.partition,
            cell_count: analysis.cell_count,
            signature: analysis.signature,
            dependencies,
            revision: 0,
        })
    }

    /// Replaces the graph and recomputes exactly its affected propagation cone.
    ///
    /// Before doing field arithmetic this method audits every exact vertex and
    /// semantic CSR row against the retained graph. The caller therefore cannot
    /// accidentally omit a dependency. Labels, signature, partition, component
    /// index and graph are published only after all fallible work succeeds.
    ///
    /// Vertex indices must retain their meaning. Edge insertions/removals can
    /// merge or split components, but changing the number of vertices requires
    /// a new [`IncrementalGraphState`]. The fixed profile radius is exactly the
    /// number of refinement rounds retained by the state.
    ///
    /// # Errors
    ///
    /// Rejects adaptive profiles, state/labeler mismatches, changed vertex
    /// counts and all encoding, inversion or exact counter failures. On error,
    /// `state` is byte-for-byte unchanged.
    pub fn update_incremental(
        &self,
        state: &mut IncrementalGraphState<F, K>,
        graph: IncidenceGraph,
        workspace: &mut IncrementalGraphWorkspace<F, K>,
    ) -> Result<IncrementalUpdateStats, GraphError> {
        if !matches!(self.profile, RefinementProfile::Fast { .. }) {
            return Err(GraphError::NonComposableProfile);
        }
        if state.signature_id != self.signature_id || state.rounds != self.profile.maximum_rounds()
        {
            return Err(GraphError::SignatureIdentityMismatch);
        }
        let vertex_count = state.graph.vertex_count();
        if graph.vertex_count() != vertex_count {
            return Err(GraphError::IncrementalVertexCountMismatch {
                expected: vertex_count,
                actual: graph.vertex_count(),
            });
        }
        let audited_incidence_records = state
            .graph
            .incidence_count()
            .checked_add(graph.incidence_count())
            .ok_or(GraphError::GraphTooLarge)?;
        if state.graph == graph {
            return Ok(IncrementalUpdateStats {
                audited_vertices: vertex_count,
                audited_incidence_records,
                previous_component_count: state.dependencies.component_count,
                component_count: state.dependencies.component_count,
                dependency_records: state.dependencies.neighbors.len(),
                revision: state.revision,
                ..IncrementalUpdateStats::default()
            });
        }

        let prepared = self.prepare(&graph)?;
        workspace.reserve_baseline(
            vertex_count,
            state.graph.incidence_count().max(graph.incidence_count()),
            state.rounds,
        )?;
        workspace.reset_vertex_storage(vertex_count);
        workspace
            .staged_aggregates
            .clone_from(&state.signature.round_aggregates);
        workspace
            .aggregate_deltas
            .resize(state.rounds.saturating_add(1), AggregateDelta::identity());

        let initial = state.labels_at(0);
        for (index, old_initial) in initial.iter().enumerate() {
            if *old_initial != prepared.initial_labels[index] {
                workspace.initial_changed.push(index);
            }
            let vertex = VertexId::new(index);
            if !semantic_rows_equal(
                &state.graph,
                state.graph.outgoing(vertex),
                &graph,
                graph.outgoing(vertex),
            ) || !semantic_rows_equal(
                &state.graph,
                state.graph.incoming(vertex),
                &graph,
                graph.incoming(vertex),
            ) {
                workspace.topology_changed.push(index);
            }
        }

        let topology_changed = !workspace.topology_changed.is_empty();
        let previous_component_count = state.dependencies.component_count;
        let mut next_dependencies = if topology_changed {
            let mut staging = core::mem::take(&mut workspace.staged_dependencies);
            rebuild_dependency_index(&graph, &mut staging, &mut workspace.component_stack)?;
            Some(staging)
        } else {
            None
        };
        let dependencies = next_dependencies.as_ref().unwrap_or(&state.dependencies);

        for &index in &workspace.initial_changed {
            let old = initial[index];
            let new = prepared.initial_labels[index];
            self.record_aggregate_delta(&mut workspace.aggregate_deltas[0], old, new)?;
            workspace.updates.push(LabelUpdate {
                offset: index,
                value: new,
            });
            workspace.previous_values[index] = new;
            workspace.previous_marks[index] = true;
            workspace.previous_changed.push(index);
        }

        let mut recomputed_vertex_rounds = 0_usize;
        let mut changed_vertex_rounds = 0_usize;
        let mut final_changed_vertices = 0_usize;
        let mut peak_frontier_vertices = 0_usize;
        for round in 1..=state.rounds {
            for &index in &workspace.current_changed {
                workspace.current_marks[index] = false;
            }
            workspace.current_changed.clear();
            workspace.begin_frontier();
            for position in 0..workspace.topology_changed.len() {
                let index = workspace.topology_changed[position];
                workspace.include_affected(index);
            }
            for position in 0..workspace.previous_changed.len() {
                let index = workspace.previous_changed[position];
                workspace.include_affected(index);
                for &dependent in dependencies.neighbors(index) {
                    workspace.include_affected(dependent);
                }
            }
            recomputed_vertex_rounds = recomputed_vertex_rounds
                .checked_add(workspace.affected.len())
                .ok_or(GraphError::GraphTooLarge)?;
            peak_frontier_vertices = peak_frontier_vertices.max(workspace.affected.len());

            let previous_old = state.labels_at(round - 1);
            let current_old = state.labels_at(round);
            let previous_marks = &workspace.previous_marks;
            let previous_values = &workspace.previous_values;
            for &index in &workspace.affected {
                let mut value = StructuralLabel {
                    lanes: [F::ZERO; K],
                };
                self.refine_prepared_vertex_with(
                    &prepared,
                    index,
                    &mut value,
                    round,
                    &|label_index| {
                        if previous_marks[label_index] {
                            previous_values[label_index]
                        } else {
                            previous_old[label_index]
                        }
                    },
                )?;
                if value != current_old[index] {
                    let offset = round
                        .checked_mul(vertex_count)
                        .and_then(|start| start.checked_add(index))
                        .ok_or(GraphError::GraphTooLarge)?;
                    self.record_aggregate_delta(
                        &mut workspace.aggregate_deltas[round],
                        current_old[index],
                        value,
                    )?;
                    workspace.updates.push(LabelUpdate { offset, value });
                    workspace.current_values[index] = value;
                    workspace.current_marks[index] = true;
                    workspace.current_changed.push(index);
                }
            }
            changed_vertex_rounds = changed_vertex_rounds
                .checked_add(workspace.current_changed.len())
                .ok_or(GraphError::GraphTooLarge)?;
            if round == state.rounds {
                final_changed_vertices = workspace.current_changed.len();
            }
            core::mem::swap(
                &mut workspace.previous_values,
                &mut workspace.current_values,
            );
            core::mem::swap(&mut workspace.previous_marks, &mut workspace.current_marks);
            core::mem::swap(
                &mut workspace.previous_changed,
                &mut workspace.current_changed,
            );
        }

        for (aggregate, delta) in workspace
            .staged_aggregates
            .iter_mut()
            .zip(&workspace.aggregate_deltas)
        {
            self.apply_aggregate_delta(aggregate, *delta)?;
        }
        let mut transcript = [F::ZERO; K];
        for (round, aggregate) in workspace.staged_aggregates.iter().copied().enumerate() {
            self.append_round_aggregate_with_token(
                &mut transcript,
                aggregate,
                prepared.transcript_round_tokens[round],
            )?;
        }
        let signature_vertex_count =
            u64::try_from(vertex_count).map_err(|_| GraphError::GraphTooLarge)?;
        let signature_incidence_count =
            u64::try_from(graph.incidence_count()).map_err(|_| GraphError::GraphTooLarge)?;
        let signature_rounds =
            u64::try_from(state.rounds).map_err(|_| GraphError::GraphTooLarge)?;

        let final_start = state
            .rounds
            .checked_mul(vertex_count)
            .ok_or(GraphError::GraphTooLarge)?;
        workspace.final_labels.clear();
        workspace
            .final_labels
            .extend_from_slice(state.labels_at(state.rounds));
        if vertex_count != 0 {
            for update in &workspace.updates {
                if update.offset >= final_start {
                    workspace.final_labels[update.offset - final_start] = update.value;
                }
            }
        }
        let cell_count = if final_changed_vertices == 0 {
            state.cell_count
        } else {
            workspace.order.clear();
            workspace.order.extend(
                state
                    .order
                    .iter()
                    .copied()
                    .filter(|index| !workspace.previous_marks[*index]),
            );
            workspace.previous_changed.sort_unstable_by(|left, right| {
                compare_labels(
                    &workspace.final_labels[*left],
                    &workspace.final_labels[*right],
                )
            });
            merge_label_orders(
                &workspace.final_labels,
                &workspace.order,
                &workspace.previous_changed,
                &mut workspace.merged_order,
            );
            partition_labels_from_order(
                &workspace.final_labels,
                &workspace.merged_order,
                &mut workspace.partition,
            )
        };
        let revision = state
            .revision
            .checked_add(1)
            .ok_or(GraphError::GraphTooLarge)?;
        let component_count = dependencies.component_count;
        let dependency_records = dependencies.neighbors.len();
        let mut signature = FastGraphSignature {
            signature_id: self.signature_id,
            lanes: transcript,
            vertex_count: signature_vertex_count,
            incidence_count: signature_incidence_count,
            total_multiplicity: graph.total_multiplicity(),
            rounds: signature_rounds,
            round_aggregates: Vec::new(),
        };
        core::mem::swap(
            &mut signature.round_aggregates,
            &mut workspace.staged_aggregates,
        );

        for update in &workspace.updates {
            state.round_labels[update.offset] = update.value;
        }
        if final_changed_vertices != 0 {
            core::mem::swap(&mut state.order, &mut workspace.merged_order);
            core::mem::swap(&mut state.partition, &mut workspace.partition);
        }
        state.cell_count = cell_count;
        core::mem::swap(&mut state.signature, &mut signature);
        workspace.staged_aggregates = signature.round_aggregates;
        if let Some(mut replacement) = next_dependencies.take() {
            core::mem::swap(&mut state.dependencies, &mut replacement);
            workspace.staged_dependencies = replacement;
        }
        drop(prepared);
        state.graph = graph;
        state.revision = revision;

        Ok(IncrementalUpdateStats {
            audited_vertices: vertex_count,
            audited_incidence_records,
            initial_seed_vertices: workspace.initial_changed.len(),
            topology_seed_vertices: workspace.topology_changed.len(),
            recomputed_vertex_rounds,
            changed_vertex_rounds,
            final_changed_vertices,
            peak_frontier_vertices,
            previous_component_count,
            component_count,
            dependency_records,
            revision,
        })
    }

    fn record_aggregate_delta(
        &self,
        delta: &mut AggregateDelta<F, K>,
        old: StructuralLabel<F, K>,
        new: StructuralLabel<F, K>,
    ) -> Result<(), GraphError> {
        for lane in 0..K {
            let old_factor = old.lanes[lane].add(self.parameters.graph_offsets[lane]);
            let new_factor = new.lanes[lane].add(self.parameters.graph_offsets[lane]);
            if old_factor == new_factor {
                continue;
            }
            if old_factor.is_zero() {
                delta.removed_zeros[lane] = delta.removed_zeros[lane]
                    .checked_add(1)
                    .ok_or(GraphError::MultiplicityOverflow)?;
            } else {
                delta.removed_nonzero[lane] = delta.removed_nonzero[lane].mul(old_factor);
            }
            if new_factor.is_zero() {
                delta.added_zeros[lane] = delta.added_zeros[lane]
                    .checked_add(1)
                    .ok_or(GraphError::MultiplicityOverflow)?;
            } else {
                delta.added_nonzero[lane] = delta.added_nonzero[lane].mul(new_factor);
            }
        }
        Ok(())
    }

    fn apply_aggregate_delta(
        &self,
        aggregate: &mut RoundAggregate<F, K>,
        delta: AggregateDelta<F, K>,
    ) -> Result<(), GraphError> {
        for lane in 0..K {
            aggregate.zero_factor_counts[lane] = aggregate.zero_factor_counts[lane]
                .checked_sub(delta.removed_zeros[lane])
                .and_then(|count| count.checked_add(delta.added_zeros[lane]))
                .ok_or(GraphError::MultiplicityOverflow)?;
            if delta.removed_nonzero[lane] != F::ONE {
                let inverse = delta.removed_nonzero[lane]
                    .invert()
                    .ok_or(GraphError::NonInvertibleAggregateFactor)?;
                aggregate.nonzero_products[lane] = aggregate.nonzero_products[lane].mul(inverse);
            }
            aggregate.nonzero_products[lane] =
                aggregate.nonzero_products[lane].mul(delta.added_nonzero[lane]);
        }
        Ok(())
    }
}

impl<F: Field, const K: usize> IncrementalGraphState<F, K> {
    /// Borrows the exact current analysis without allocation.
    #[must_use]
    pub fn analysis(&self) -> FastGraphAnalysisView<'_, F, K> {
        FastGraphAnalysisView {
            labels: self.labels_at(self.rounds),
            partition: &self.partition,
            cell_count: self.cell_count,
            stable_partition: false,
            signature: FastGraphSignatureView {
                signature_id: self.signature.signature_id,
                lanes: self.signature.lanes,
                vertex_count: self.signature.vertex_count,
                incidence_count: self.signature.incidence_count,
                total_multiplicity: self.signature.total_multiplicity,
                rounds: self.signature.rounds,
                round_aggregates: &self.signature.round_aggregates,
            },
        }
    }
}

fn semantic_rows_equal(
    left_graph: &IncidenceGraph,
    left: &[Incidence],
    right_graph: &IncidenceGraph,
    right: &[Incidence],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.neighbor() == right.neighbor()
                && left.multiplicity() == right.multiplicity()
                && left_graph.relation(left.relation()) == right_graph.relation(right.relation())
        })
}

fn rebuild_dependency_index(
    graph: &IncidenceGraph,
    index: &mut GraphDependencyIndex,
    stack: &mut Vec<usize>,
) -> Result<(), GraphError> {
    index.reserve_for(graph.vertex_count(), graph.incidence_count())?;
    index.offsets.clear();
    index.offsets.push(0);
    index.neighbors.clear();
    for vertex_index in 0..graph.vertex_count() {
        let vertex = VertexId::new(vertex_index);
        let outgoing = graph.outgoing(vertex);
        let incoming = graph.incoming(vertex);
        let mut outgoing_index = 0_usize;
        let mut incoming_index = 0_usize;
        while outgoing_index < outgoing.len() || incoming_index < incoming.len() {
            let outgoing_neighbor = outgoing
                .get(outgoing_index)
                .map(|incidence| incidence.neighbor().index());
            let incoming_neighbor = incoming
                .get(incoming_index)
                .map(|incidence| incidence.neighbor().index());
            let neighbor = match (outgoing_neighbor, incoming_neighbor) {
                (Some(left), Some(right)) => left.min(right),
                (Some(left), None) => left,
                (None, Some(right)) => right,
                (None, None) => break,
            };
            while outgoing
                .get(outgoing_index)
                .is_some_and(|incidence| incidence.neighbor().index() == neighbor)
            {
                outgoing_index += 1;
            }
            while incoming
                .get(incoming_index)
                .is_some_and(|incidence| incidence.neighbor().index() == neighbor)
            {
                incoming_index += 1;
            }
            if neighbor != vertex_index {
                index.neighbors.push(neighbor);
            }
        }
        index.offsets.push(index.neighbors.len());
    }

    index.components.clear();
    index.components.resize(graph.vertex_count(), usize::MAX);
    index.component_count = 0;
    stack.clear();
    for root in 0..graph.vertex_count() {
        if index.components[root] != usize::MAX {
            continue;
        }
        let component = index.component_count;
        index.component_count = index
            .component_count
            .checked_add(1)
            .ok_or(GraphError::GraphTooLarge)?;
        index.components[root] = component;
        stack.push(root);
        while let Some(vertex) = stack.pop() {
            let start = index.offsets[vertex];
            let end = index.offsets[vertex + 1];
            for neighbor_index in start..end {
                let neighbor = index.neighbors[neighbor_index];
                if index.components[neighbor] == usize::MAX {
                    index.components[neighbor] = component;
                    stack.push(neighbor);
                }
            }
        }
    }
    Ok(())
}

impl<E, const K: usize> FastGraphLabeler<microfield::Fp251V1, E, K>
where
    E: StructuralEncoder<microfield::Fp251V1>,
{
    /// Executes the fixed-round F251 recurrence with an explicit batched SoA
    /// Horner stage.
    ///
    /// This is deliberately opt-in. CSR neighborhood products still require
    /// irregular gathers; only the regular per-vertex update is sent through
    /// the selected Microfield engine. End-to-end benchmarks determine whether
    /// its AoS↔SoA passes are worthwhile for a concrete graph and CPU.
    ///
    /// # Errors
    ///
    /// Rejects adaptive profiles, mismatched preparations and exact counter
    /// overflows. All batch slice lengths are established by the workspace.
    pub fn analyze_prepared_f251_batched<'workspace>(
        &self,
        prepared: &PreparedGraph<'_, microfield::Fp251V1, K>,
        workspace: &'workspace mut F251BatchGraphWorkspace<K>,
        neighborhood_execution: GraphExecution,
    ) -> Result<FastGraphAnalysisView<'workspace, microfield::Fp251V1, K>, GraphError> {
        if !matches!(self.profile, RefinementProfile::Fast { .. }) {
            return Err(GraphError::NonComposableProfile);
        }
        if prepared.signature_id != self.signature_id {
            return Err(GraphError::SignatureIdentityMismatch);
        }
        let graph = prepared.graph;
        let vertex_count = graph.vertex_count();
        workspace.reserve_for(vertex_count, self.profile.maximum_rounds());
        workspace.core.labels.clone_from(&prepared.initial_labels);
        workspace.core.next.resize(
            vertex_count,
            StructuralLabel {
                lanes: [microfield::Fp251V1::ZERO; K],
            },
        );
        for values in [
            &mut workspace.out_products,
            &mut workspace.in_products,
            &mut workspace.out_zero_tokens,
            &mut workspace.in_zero_tokens,
        ] {
            values.resize(vertex_count, [microfield::Fp251V1::ZERO; K]);
        }
        workspace
            .first
            .resize(vertex_count, microfield::Fp251V1::ZERO);
        workspace
            .second
            .resize(vertex_count, microfield::Fp251V1::ZERO);
        workspace
            .rhs
            .resize(vertex_count, microfield::Fp251V1::ZERO);
        workspace.core.round_aggregates.clear();
        workspace.core.partition.clear();
        workspace.core.order.clear();

        let mut transcript = [microfield::Fp251V1::ZERO; K];
        let initial_aggregate = self.aggregate_labels(&workspace.core.labels)?;
        self.append_round_aggregate_with_token(
            &mut transcript,
            initial_aggregate,
            prepared.transcript_round_tokens[0],
        )?;
        workspace.core.round_aggregates.push(initial_aggregate);

        for round in 1..=self.profile.maximum_rounds() {
            self.refine_f251_batched_round(
                prepared,
                &workspace.core.labels,
                &mut workspace.core.next,
                &mut workspace.out_products,
                &mut workspace.in_products,
                &mut workspace.out_zero_tokens,
                &mut workspace.in_zero_tokens,
                &mut workspace.first,
                &mut workspace.second,
                &mut workspace.rhs,
                workspace.engine,
                round,
                neighborhood_execution,
            )?;
            core::mem::swap(&mut workspace.core.labels, &mut workspace.core.next);
            let aggregate = self.aggregate_labels(&workspace.core.labels)?;
            self.append_round_aggregate_with_token(
                &mut transcript,
                aggregate,
                prepared.transcript_round_tokens[round],
            )?;
            workspace.core.round_aggregates.push(aggregate);
        }

        let cell_count = partition_labels_into(
            &workspace.core.labels,
            &mut workspace.core.order,
            &mut workspace.core.partition,
        );
        Ok(FastGraphAnalysisView {
            labels: &workspace.core.labels,
            partition: &workspace.core.partition,
            cell_count,
            stable_partition: false,
            signature: FastGraphSignatureView {
                signature_id: self.signature_id,
                lanes: transcript,
                vertex_count: u64::try_from(vertex_count).map_err(|_| GraphError::GraphTooLarge)?,
                incidence_count: u64::try_from(graph.incidence_count())
                    .map_err(|_| GraphError::GraphTooLarge)?,
                total_multiplicity: graph.total_multiplicity(),
                rounds: u64::try_from(self.profile.maximum_rounds())
                    .map_err(|_| GraphError::GraphTooLarge)?,
                round_aggregates: &workspace.core.round_aggregates,
            },
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn refine_f251_batched_round(
        &self,
        prepared: &PreparedGraph<'_, microfield::Fp251V1, K>,
        labels: &[StructuralLabel<microfield::Fp251V1, K>],
        output: &mut [StructuralLabel<microfield::Fp251V1, K>],
        out_products: &mut [[microfield::Fp251V1; K]],
        in_products: &mut [[microfield::Fp251V1; K]],
        out_zero_tokens: &mut [[microfield::Fp251V1; K]],
        in_zero_tokens: &mut [[microfield::Fp251V1; K]],
        first: &mut [microfield::Fp251V1],
        second: &mut [microfield::Fp251V1],
        rhs: &mut [microfield::Fp251V1],
        engine: microfield::Engine<microfield::Fp251V1>,
        round: usize,
        execution: GraphExecution,
    ) -> Result<(), GraphError> {
        let evaluate = |index: usize,
                        out_product: &mut [microfield::Fp251V1; K],
                        in_product: &mut [microfield::Fp251V1; K],
                        out_zero: &mut [microfield::Fp251V1; K],
                        in_zero: &mut [microfield::Fp251V1; K]| {
            let vertex = VertexId::new(index);
            let (out, out_counts) = self.neighborhood_product_prepared(
                prepared.graph.outgoing(vertex),
                &prepared.outgoing_affine,
                labels,
            )?;
            let (incoming, in_counts) = self.neighborhood_product_prepared(
                prepared.graph.incoming(vertex),
                &prepared.incoming_affine,
                labels,
            )?;
            *out_product = out;
            *in_product = incoming;
            for lane in 0..K {
                out_zero[lane] = self.encode_optional_counter(4, out_counts[lane])?;
                in_zero[lane] = self.encode_optional_counter(5, in_counts[lane])?;
            }
            Ok::<(), GraphError>(())
        };
        if execution.uses_parallelism(labels.len()) {
            out_products
                .par_iter_mut()
                .zip(in_products.par_iter_mut())
                .zip(out_zero_tokens.par_iter_mut())
                .zip(in_zero_tokens.par_iter_mut())
                .enumerate()
                .try_for_each(|(index, (((out, incoming), out_zero), in_zero))| {
                    evaluate(index, out, incoming, out_zero, in_zero)
                })?;
        } else {
            for index in 0..labels.len() {
                evaluate(
                    index,
                    &mut out_products[index],
                    &mut in_products[index],
                    &mut out_zero_tokens[index],
                    &mut in_zero_tokens[index],
                )?;
            }
        }

        let round_token = prepared.refine_round_tokens[round];
        for lane in 0..K {
            for (target, label) in first.iter_mut().zip(labels) {
                *target = label.lanes[lane];
            }
            f251_horner_stage(
                engine,
                first,
                second,
                rhs,
                self.parameters.update_bases[lane],
                |index| out_products[index][lane],
            );
            f251_horner_stage(
                engine,
                first,
                second,
                rhs,
                self.parameters.update_bases[lane],
                |index| out_zero_tokens[index][lane].add(self.parameters.lane_salts[lane]),
            );
            f251_horner_stage(
                engine,
                first,
                second,
                rhs,
                self.parameters.update_bases[lane],
                |index| in_products[index][lane],
            );
            f251_horner_stage(
                engine,
                first,
                second,
                rhs,
                self.parameters.update_bases[lane],
                |index| in_zero_tokens[index][lane].add(self.parameters.lane_salts[lane]),
            );
            f251_horner_stage(
                engine,
                first,
                second,
                rhs,
                self.parameters.update_bases[lane],
                |_| round_token.add(self.parameters.lane_salts[lane]),
            );
            for (slot, value) in output.iter_mut().zip(first.iter().copied()) {
                slot.lanes[lane] = value;
            }
        }
        Ok(())
    }
}

fn f251_horner_stage(
    engine: microfield::Engine<microfield::Fp251V1>,
    accumulator: &mut [microfield::Fp251V1],
    product: &mut [microfield::Fp251V1],
    rhs: &mut [microfield::Fp251V1],
    base: microfield::Fp251V1,
    addend: impl Fn(usize) -> microfield::Fp251V1,
) {
    rhs.fill(base);
    engine
        .mul_into(product, accumulator, rhs)
        .expect("F251 graph workspace establishes equal batch lengths");
    for (index, value) in rhs.iter_mut().enumerate() {
        *value = addend(index);
    }
    engine
        .add_into(accumulator, product, rhs)
        .expect("F251 graph workspace establishes equal batch lengths");
}

impl<const K: usize> F251GraphLabeler<K> {
    /// Builds the maintained F251 profile with a stable prime-field encoder.
    ///
    /// # Errors
    ///
    /// Rejects invalid round bounds or more independent lanes than F251 can
    /// provide as distinct affine points.
    pub fn f251(profile: RefinementProfile) -> Result<Self, GraphError> {
        Self::new(PrimeIntegerEncoder::new(DEFAULT_F251_GRAPH_DOMAIN), profile)
    }
}

fn derive_signature_id<F, E, const K: usize>(
    encoder: &E,
    profile: RefinementProfile,
    parameters: &GraphFieldParameters<F, K>,
) -> GraphSignatureId
where
    F: Field + CanonicalEncoding + StaticField,
    E: StructuralEncoder<F>,
{
    let mut hasher = Sha256::new();
    hasher.update(b"microfield-fast-graph-signature-v1\0");
    hasher.update(F::spec().field_id().as_bytes());
    let encoder_id: EncoderId = encoder.encoder_id();
    hasher.update(encoder_id.as_bytes());
    hasher.update((K as u64).to_le_bytes());
    match profile {
        RefinementProfile::Fast { rounds } => {
            hasher.update([1]);
            hasher.update((rounds as u64).to_le_bytes());
        }
        RefinementProfile::Robust {
            minimum_rounds,
            maximum_rounds,
        } => {
            hasher.update([2]);
            hasher.update((minimum_rounds as u64).to_le_bytes());
            hasher.update((maximum_rounds as u64).to_le_bytes());
        }
    }
    for values in [
        &parameters.lane_salts,
        &parameters.neighbor_bases,
        &parameters.multiset_offsets,
        &parameters.outgoing_salts,
        &parameters.incoming_salts,
        &parameters.update_bases,
        &parameters.transcript_bases,
        &parameters.graph_offsets,
    ] {
        for value in values {
            hasher.update(value.to_canonical().as_ref());
        }
    }
    GraphSignatureId(hasher.finalize().into())
}

fn sum_multiplicity(incidences: &[Incidence]) -> Result<u64, GraphError> {
    incidences.iter().try_fold(0_u64, |sum, incidence| {
        sum.checked_add(incidence.multiplicity())
            .ok_or(GraphError::MultiplicityOverflow)
    })
}

fn append_bytes(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), GraphError> {
    let length = u64::try_from(bytes.len()).map_err(|_| GraphError::GraphTooLarge)?;
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(bytes);
    Ok(())
}

fn compare_labels<F, const K: usize>(
    left: &StructuralLabel<F, K>,
    right: &StructuralLabel<F, K>,
) -> Ordering
where
    F: Field + CanonicalEncoding,
{
    for lane in 0..K {
        let ordering = left.lanes[lane]
            .to_canonical()
            .as_ref()
            .cmp(right.lanes[lane].to_canonical().as_ref());
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}

fn partition_labels_into<F, const K: usize>(
    labels: &[StructuralLabel<F, K>],
    order: &mut Vec<usize>,
    partition: &mut Vec<usize>,
) -> usize
where
    F: Field + CanonicalEncoding,
{
    order.clear();
    order.extend(0..labels.len());
    order.sort_unstable_by(|left, right| compare_labels(&labels[*left], &labels[*right]));
    partition_labels_from_order(labels, order, partition)
}

fn partition_labels_from_order<F, const K: usize>(
    labels: &[StructuralLabel<F, K>],
    order: &[usize],
    partition: &mut Vec<usize>,
) -> usize
where
    F: Field + CanonicalEncoding,
{
    partition.clear();
    partition.resize(labels.len(), 0);
    let mut cells = 0_usize;
    let mut previous: Option<usize> = None;
    for &index in order.iter() {
        if previous.is_none_or(|prior| labels[prior] != labels[index]) {
            cells += 1;
        }
        partition[index] = cells - 1;
        previous = Some(index);
    }
    cells
}

fn merge_label_orders<F, const K: usize>(
    labels: &[StructuralLabel<F, K>],
    retained: &[usize],
    changed: &[usize],
    output: &mut Vec<usize>,
) where
    F: Field + CanonicalEncoding,
{
    output.clear();
    let mut retained_index = 0_usize;
    let mut changed_index = 0_usize;
    while retained_index < retained.len() || changed_index < changed.len() {
        let take_retained = match (retained.get(retained_index), changed.get(changed_index)) {
            (Some(left), Some(right)) => {
                compare_labels(&labels[*left], &labels[*right]) != Ordering::Greater
            }
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        if take_retained {
            output.push(retained[retained_index]);
            retained_index += 1;
        } else {
            output.push(changed[changed_index]);
            changed_index += 1;
        }
    }
}

fn same_partition_slices(
    left: &[usize],
    left_cells: usize,
    right: &[usize],
    right_cells: usize,
    left_to_right: &mut Vec<usize>,
    right_to_left: &mut Vec<usize>,
) -> bool {
    if left.len() != right.len() || left_cells != right_cells {
        return false;
    }
    left_to_right.clear();
    left_to_right.resize(left_cells, usize::MAX);
    right_to_left.clear();
    right_to_left.resize(right_cells, usize::MAX);
    for (&left_cell, &right_cell) in left.iter().zip(right) {
        if left_to_right[left_cell] == usize::MAX {
            left_to_right[left_cell] = right_cell;
        } else if left_to_right[left_cell] != right_cell {
            return false;
        }
        if right_to_left[right_cell] == usize::MAX {
            right_to_left[right_cell] = left_cell;
        } else if right_to_left[right_cell] != left_cell {
            return false;
        }
    }
    true
}

fn round_histogram_digest<F, const K: usize>(
    round: usize,
    labels: &[StructuralLabel<F, K>],
) -> Result<[u8; 32], GraphError>
where
    F: Field + CanonicalEncoding,
{
    let mut entries: Vec<[u8; 32]> = Vec::with_capacity(labels.len());
    for label in labels {
        let mut entry = Sha256::new();
        entry.update(b"microfield-graph-round-label-v1\0");
        update_label_digest(&mut entry, label);
        entries.push(entry.finalize().into());
    }
    entries.sort_unstable();
    let mut root = Sha256::new();
    root.update(b"microfield-graph-round-histogram-v1\0");
    root.update(
        u64::try_from(round)
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_le_bytes(),
    );
    root.update(
        u64::try_from(labels.len())
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_le_bytes(),
    );
    root.update((K as u64).to_le_bytes());
    for entry in entries {
        root.update(entry);
    }
    Ok(root.finalize().into())
}

fn invariant_graph_digest<F, const K: usize>(
    graph: &IncidenceGraph,
    analysis: &FastGraphAnalysis<F, K>,
    round_digests: &[[u8; 32]],
) -> Result<InvariantGraphDigest, GraphError>
where
    F: Field + CanonicalEncoding,
{
    let mut vertex_entries: Vec<[u8; 32]> = Vec::with_capacity(graph.vertex_count());
    for index in 0..graph.vertex_count() {
        let vertex = VertexId::new(index);
        let mut entry = Sha256::new();
        entry.update(b"microfield-graph-exact-vertex-v1\0");
        entry.update([graph.vertex_kind(vertex) as u8]);
        update_framed_digest(&mut entry, graph.vertex_label(vertex))?;
        update_label_digest(&mut entry, &analysis.labels[index]);
        vertex_entries.push(entry.finalize().into());
    }
    vertex_entries.sort_unstable();

    let mut edge_entries: Vec<[u8; 32]> = Vec::with_capacity(graph.incidence_count());
    for source_index in 0..graph.vertex_count() {
        let source = VertexId::new(source_index);
        for incidence in graph.outgoing(source) {
            let target = incidence.neighbor();
            let descriptor = graph.relation(incidence.relation());
            let mut entry = Sha256::new();
            entry.update(b"microfield-graph-exact-relation-class-v1\0");
            entry.update([graph.vertex_kind(source) as u8]);
            update_framed_digest(&mut entry, graph.vertex_label(source))?;
            update_label_digest(&mut entry, &analysis.labels[source.index()]);
            update_framed_digest(&mut entry, descriptor.relation())?;
            update_framed_digest(&mut entry, descriptor.role())?;
            entry.update([graph.vertex_kind(target) as u8]);
            update_framed_digest(&mut entry, graph.vertex_label(target))?;
            update_label_digest(&mut entry, &analysis.labels[target.index()]);
            entry.update(incidence.multiplicity().to_le_bytes());
            edge_entries.push(entry.finalize().into());
        }
    }
    edge_entries.sort_unstable();

    let mut root = Sha256::new();
    root.update(b"microfield-hybrid-graph-fingerprint-v1\0");
    root.update(analysis.signature.signature_id.as_bytes());
    root.update(analysis.signature.vertex_count.to_le_bytes());
    root.update(analysis.signature.incidence_count.to_le_bytes());
    root.update(analysis.signature.total_multiplicity.to_le_bytes());
    root.update(analysis.signature.rounds.to_le_bytes());
    root.update(
        u64::try_from(round_digests.len())
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_le_bytes(),
    );
    for digest in round_digests {
        root.update(digest);
    }
    for digest in vertex_entries {
        root.update(digest);
    }
    for digest in edge_entries {
        root.update(digest);
    }
    Ok(InvariantGraphDigest(root.finalize().into()))
}

fn update_label_digest<F, const K: usize>(hasher: &mut Sha256, label: &StructuralLabel<F, K>)
where
    F: Field + CanonicalEncoding,
{
    hasher.update((K as u64).to_le_bytes());
    for lane in label.lanes {
        hasher.update(lane.to_canonical().as_ref());
    }
}

fn update_framed_digest(hasher: &mut Sha256, bytes: &[u8]) -> Result<(), GraphError> {
    hasher.update(
        u64::try_from(bytes.len())
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_le_bytes(),
    );
    hasher.update(bytes);
    Ok(())
}

pub(super) fn discrete_form<F, const K: usize>(
    graph: &IncidenceGraph,
    labels: &[StructuralLabel<F, K>],
    signature_id: GraphSignatureId,
) -> Result<DiscreteCanonicalForm, GraphError>
where
    F: Field + CanonicalEncoding,
{
    let mut canonical_to_original: Vec<VertexId> =
        (0..graph.vertex_count()).map(VertexId::new).collect();
    canonical_to_original.sort_unstable_by(|left, right| {
        compare_labels(&labels[left.index()], &labels[right.index()])
    });
    canonical_form_from_order(graph, canonical_to_original, signature_id)
}

pub(super) fn canonical_form_from_order(
    graph: &IncidenceGraph,
    canonical_to_original: Vec<VertexId>,
    signature_id: GraphSignatureId,
) -> Result<DiscreteCanonicalForm, GraphError> {
    if canonical_to_original.len() != graph.vertex_count() {
        return Err(GraphError::InvalidCanonicalOrder);
    }
    let mut original_to_canonical = vec![VertexId::new(0); graph.vertex_count()];
    let mut seen = vec![false; graph.vertex_count()];
    for (canonical, original) in canonical_to_original.iter().copied().enumerate() {
        if original.index() >= graph.vertex_count() || seen[original.index()] {
            return Err(GraphError::InvalidCanonicalOrder);
        }
        seen[original.index()] = true;
        original_to_canonical[original.index()] = VertexId::new(canonical);
    }

    let vertex_count =
        u64::try_from(graph.vertex_count()).map_err(|_| GraphError::GraphTooLarge)?;
    let incidence_count =
        u64::try_from(graph.incidence_count()).map_err(|_| GraphError::GraphTooLarge)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(CANONICAL_MAGIC);
    bytes.extend_from_slice(&GRAPH_SCHEMA.to_le_bytes());
    bytes.extend_from_slice(signature_id.as_bytes());
    bytes.extend_from_slice(&vertex_count.to_le_bytes());
    bytes.extend_from_slice(&incidence_count.to_le_bytes());
    bytes.extend_from_slice(&graph.total_multiplicity().to_le_bytes());
    for original in &canonical_to_original {
        bytes.push(graph.vertex_kind(*original) as u8);
        append_bytes(&mut bytes, graph.vertex_label(*original))?;
    }

    let mut arcs = Vec::with_capacity(graph.incidence_count());
    for source in 0..graph.vertex_count() {
        for incidence in graph.outgoing(VertexId::new(source)) {
            arcs.push((
                original_to_canonical[source].index(),
                original_to_canonical[incidence.neighbor().index()].index(),
                incidence.relation(),
                incidence.multiplicity(),
            ));
        }
    }
    arcs.sort_unstable_by(|left, right| {
        let left_relation = graph.relation(left.2);
        let right_relation = graph.relation(right.2);
        (left.0, left.1)
            .cmp(&(right.0, right.1))
            .then_with(|| left_relation.cmp(right_relation))
            .then_with(|| left.3.cmp(&right.3))
    });
    for (source, target, relation, multiplicity) in arcs {
        bytes.extend_from_slice(
            &u64::try_from(source)
                .map_err(|_| GraphError::GraphTooLarge)?
                .to_le_bytes(),
        );
        bytes.extend_from_slice(
            &u64::try_from(target)
                .map_err(|_| GraphError::GraphTooLarge)?
                .to_le_bytes(),
        );
        let descriptor = graph.relation(relation);
        append_bytes(&mut bytes, descriptor.relation())?;
        append_bytes(&mut bytes, descriptor.role())?;
        bytes.extend_from_slice(&multiplicity.to_le_bytes());
    }

    Ok(DiscreteCanonicalForm {
        bytes,
        original_to_canonical,
        canonical_to_original,
    })
}
