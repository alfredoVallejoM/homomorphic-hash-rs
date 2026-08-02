//! Global graph invariants and bounded motif evidence for the v2 discriminator.
//!
//! The finite-field recurrence is intentionally local. This module supplies
//! independent, relabeling-invariant evidence that sees disconnectedness,
//! directed reachability, exact labels/relations and selected small motifs.
//! Equality remains evidence, never a proof of isomorphism.

use core::fmt;

use microfield::{CanonicalEncoding, Field, Pow, StaticField};
use sha2::{Digest as _, Sha256};

use crate::structural::StructuralEncoder;

use super::{
    FastGraphLabeler, GraphError, GraphSignatureId, HybridGraphAnalysis, IncidenceGraph, VertexId,
    VertexKind,
};

const GLOBAL_MAGIC: &[u8; 4] = b"MFGL";
const DISCRIMINATOR_MAGIC: &[u8; 4] = b"MFGD";
const GLOBAL_SCHEMA: u16 = 2;
const DEFAULT_MOTIF_WORK: u64 = 4_000_000;

/// Cheap and exact summary of one weakly connected component.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WeakComponentSummary {
    vertex_count: u64,
    entity_count: u64,
    hyperedge_count: u64,
    incidence_count: u64,
    total_multiplicity: u64,
    support_edge_count: u64,
    self_loop_count: u64,
    self_loop_multiplicity: u64,
    strongly_connected_component_count: u64,
    cycle_rank: u64,
}

impl WeakComponentSummary {
    /// Number of normalized entity and hyperedge vertices.
    #[must_use]
    pub const fn vertex_count(&self) -> u64 {
        self.vertex_count
    }

    /// Number of application vertices.
    #[must_use]
    pub const fn entity_count(&self) -> u64 {
        self.entity_count
    }

    /// Number of auxiliary hyperedge vertices.
    #[must_use]
    pub const fn hyperedge_count(&self) -> u64 {
        self.hyperedge_count
    }

    /// Number of normalized directed incidence records.
    #[must_use]
    pub const fn incidence_count(&self) -> u64 {
        self.incidence_count
    }

    /// Sum of directed incidence multiplicities.
    #[must_use]
    pub const fn total_multiplicity(&self) -> u64 {
        self.total_multiplicity
    }

    /// Number of edges in the simple undirected support, excluding loops.
    #[must_use]
    pub const fn support_edge_count(&self) -> u64 {
        self.support_edge_count
    }

    /// Number of normalized directed self-loop records.
    #[must_use]
    pub const fn self_loop_count(&self) -> u64 {
        self.self_loop_count
    }

    /// Sum of self-loop multiplicities.
    #[must_use]
    pub const fn self_loop_multiplicity(&self) -> u64 {
        self.self_loop_multiplicity
    }

    /// Number of directed strongly connected components.
    #[must_use]
    pub const fn strongly_connected_component_count(&self) -> u64 {
        self.strongly_connected_component_count
    }

    /// Cycle rank of the component's simple undirected support.
    #[must_use]
    pub const fn cycle_rank(&self) -> u64 {
        self.cycle_rank
    }
}

/// SHA-256 identity of the exact global-invariant serialization.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct GlobalInvariantDigest([u8; 32]);

impl GlobalInvariantDigest {
    /// Borrows the digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for GlobalInvariantDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for GlobalInvariantDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "GlobalInvariantDigest({self})")
    }
}

/// Exact, relabeling-invariant global descriptor independent of finite fields.
///
/// Equality compares the complete canonical descriptor, not only its digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalGraphProfile {
    weak_components: Vec<WeakComponentSummary>,
    strongly_connected_component_count: u64,
    canonical_bytes: Vec<u8>,
    digest: GlobalInvariantDigest,
}

impl GlobalGraphProfile {
    /// Computes the exact global descriptor without a finite-field analysis.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::GraphTooLarge`] if stable counters overflow.
    pub fn analyze(graph: &IncidenceGraph) -> Result<Self, GraphError> {
        let topology = GlobalTopology::build(graph)?;
        build_global_profile(graph, &topology)
    }

    /// Number of weakly connected components.
    #[must_use]
    pub fn weak_component_count(&self) -> usize {
        self.weak_components.len()
    }

    /// Sorted exact summaries of weak components.
    #[must_use]
    pub fn weak_components(&self) -> &[WeakComponentSummary] {
        &self.weak_components
    }

    /// Number of strongly connected components in the directed model.
    #[must_use]
    pub const fn strongly_connected_component_count(&self) -> u64 {
        self.strongly_connected_component_count
    }

    /// Stable v2 bytes used for exact profile equality and persistence.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// SHA-256 convenience identity of the complete canonical bytes.
    #[must_use]
    pub const fn digest(&self) -> GlobalInvariantDigest {
        self.digest
    }
}

/// Cost policy for independent motif evidence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GraphDiscriminationPolicy {
    /// Only global exact descriptors; no motif enumeration.
    GlobalLinear,
    /// Enumerate triangles and four-cliques when an invariant upper bound fits.
    Adaptive {
        /// Maximum candidate tuples inspected by the motif tier.
        max_motif_work: u64,
    },
}

impl GraphDiscriminationPolicy {
    /// Recommended bounded policy.
    #[must_use]
    pub const fn adaptive() -> Self {
        Self::Adaptive {
            max_motif_work: DEFAULT_MOTIF_WORK,
        }
    }
}

impl Default for GraphDiscriminationPolicy {
    fn default() -> Self {
        Self::adaptive()
    }
}

/// Whether and why the bounded motif tier ran.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MotifAnalysisStatus {
    /// The caller selected the global-only policy.
    NotRequested,
    /// The finite-field partition was not regular enough to justify escalation.
    NotNeeded,
    /// Every candidate tuple was evaluated within budget.
    Complete,
    /// An invariant upper bound exceeded the supplied budget; no partial count is exposed.
    SkippedBudget,
}

/// Independent motif counts over the simple undirected support graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedMotifProfile {
    status: MotifAnalysisStatus,
    estimated_work: u64,
    triangle_count: Option<u64>,
    four_clique_count: Option<u64>,
}

impl BoundedMotifProfile {
    /// Execution status of this tier.
    #[must_use]
    pub const fn status(&self) -> MotifAnalysisStatus {
        self.status
    }

    /// Relabeling-invariant upper bound used for admission.
    #[must_use]
    pub const fn estimated_work(&self) -> u64 {
        self.estimated_work
    }

    /// Exact triangle count when the complete tier ran.
    #[must_use]
    pub const fn triangle_count(&self) -> Option<u64> {
        self.triangle_count
    }

    /// Exact four-clique count when the complete tier ran.
    #[must_use]
    pub const fn four_clique_count(&self) -> Option<u64> {
        self.four_clique_count
    }
}

/// Stable identity of the v2 discriminator configuration.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct GraphDiscriminationId([u8; 32]);

impl GraphDiscriminationId {
    /// Borrows the identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for GraphDiscriminationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for GraphDiscriminationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "GraphDiscriminationId({self})")
    }
}

/// Digest combining independent algebraic, exact-global and motif channels.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct GraphDiscriminationDigest([u8; 32]);

impl GraphDiscriminationDigest {
    /// Borrows the digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for GraphDiscriminationDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for GraphDiscriminationDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "GraphDiscriminationDigest({self})")
    }
}

/// Conservative comparison of two compatible v2 analyses.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiscriminatingGraphComparison {
    /// At least one relabeling-invariant channel proves the graphs differ.
    Different,
    /// All enabled channels agree; isomorphism is not proven.
    Indistinguishable,
}

/// Recommended next action after the bounded v2 analysis.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GraphEscalationAdvice {
    /// The fast partition is discrete and can provide an exact order.
    FastCanonicalOrderAvailable,
    /// Global and local evidence is strong, but equality is still not a proof.
    GlobalEvidenceAvailable,
    /// Motif evidence was added for a highly regular graph.
    MotifEvidenceAvailable,
    /// The graph stayed highly regular and exceeded motif budget.
    ExactCanonicalizationRecommended,
}

/// Recommended v2 result combining complementary evidence channels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscriminatingGraphAnalysis<F: Field, const K: usize> {
    profile_id: GraphDiscriminationId,
    hybrid: HybridGraphAnalysis<F, K>,
    global: GlobalGraphProfile,
    motifs: BoundedMotifProfile,
    digest: GraphDiscriminationDigest,
    advice: GraphEscalationAdvice,
}

impl<F: Field, const K: usize> DiscriminatingGraphAnalysis<F, K> {
    /// Complete identity of field recurrence and v2 policy.
    #[must_use]
    pub const fn profile_id(&self) -> GraphDiscriminationId {
        self.profile_id
    }

    /// Existing algebraic and independent SHA-256 local channels.
    #[must_use]
    pub const fn hybrid(&self) -> &HybridGraphAnalysis<F, K> {
        &self.hybrid
    }

    /// Exact global descriptor.
    #[must_use]
    pub const fn global(&self) -> &GlobalGraphProfile {
        &self.global
    }

    /// Bounded motif evidence and admission status.
    #[must_use]
    pub const fn motifs(&self) -> &BoundedMotifProfile {
        &self.motifs
    }

    /// Combined convenience digest; not a proof of isomorphism.
    #[must_use]
    pub const fn digest(&self) -> GraphDiscriminationDigest {
        self.digest
    }

    /// Safe routing advice.
    #[must_use]
    pub const fn advice(&self) -> GraphEscalationAdvice {
        self.advice
    }

    /// Compares compatible analyses without turning equality into a proof.
    ///
    /// # Errors
    ///
    /// Rejects analyses created by different fields, recurrences or policies.
    pub fn compare(&self, other: &Self) -> Result<DiscriminatingGraphComparison, GraphError> {
        if self.profile_id != other.profile_id {
            return Err(GraphError::DiscriminationProfileMismatch);
        }
        let different = self.global != other.global
            || self.hybrid.structural().signature() != other.hybrid.structural().signature()
            || self.hybrid.invariant_digest() != other.hybrid.invariant_digest()
            || self.motifs != other.motifs;
        Ok(if different {
            DiscriminatingGraphComparison::Different
        } else {
            DiscriminatingGraphComparison::Indistinguishable
        })
    }
}

impl<F, E, const K: usize> FastGraphLabeler<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField + Pow,
    E: StructuralEncoder<F>,
{
    /// Runs the recommended v2 discriminator without unbounded graph search.
    ///
    /// The global tier is `O(V + I + S log S)` for descriptor sorting. Motif
    /// enumeration is admitted only from a relabeling-invariant work bound.
    ///
    /// # Errors
    ///
    /// Propagates graph-size and structural-encoding failures.
    pub fn analyze_discriminating(
        &self,
        graph: &IncidenceGraph,
        policy: GraphDiscriminationPolicy,
    ) -> Result<DiscriminatingGraphAnalysis<F, K>, GraphError> {
        let hybrid = self.analyze_hybrid(graph)?;
        let topology = GlobalTopology::build(graph)?;
        let global = build_global_profile(graph, &topology)?;
        let highly_regular = is_highly_regular(
            hybrid.structural().partition(),
            hybrid.structural().cell_count(),
        );
        let motifs = analyze_motifs(&topology.adjacency, highly_regular, policy)?;
        let advice = if hybrid.structural().cell_count() == graph.vertex_count() {
            GraphEscalationAdvice::FastCanonicalOrderAvailable
        } else {
            match motifs.status {
                MotifAnalysisStatus::Complete => GraphEscalationAdvice::MotifEvidenceAvailable,
                MotifAnalysisStatus::SkippedBudget if highly_regular => {
                    GraphEscalationAdvice::ExactCanonicalizationRecommended
                }
                MotifAnalysisStatus::NotRequested
                | MotifAnalysisStatus::NotNeeded
                | MotifAnalysisStatus::SkippedBudget => {
                    GraphEscalationAdvice::GlobalEvidenceAvailable
                }
            }
        };
        let profile_id = derive_discrimination_id(self.signature_id(), policy);
        let digest = derive_discrimination_digest(profile_id, &hybrid, &global, &motifs);
        Ok(DiscriminatingGraphAnalysis {
            profile_id,
            hybrid,
            global,
            motifs,
            digest,
            advice,
        })
    }
}

#[derive(Debug)]
pub(super) struct GlobalTopology {
    pub(super) adjacency: Vec<Vec<usize>>,
    pub(super) weak_components: Vec<usize>,
    pub(super) weak_component_count: usize,
    sccs: Vec<usize>,
    scc_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct VertexInvariantRecord {
    kind: u8,
    label: usize,
    outgoing_count: usize,
    incoming_count: usize,
    outgoing_multiplicity: u64,
    incoming_multiplicity: u64,
    support_degree: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ArcInvariantRecord {
    source_kind: u8,
    source_label: usize,
    target_kind: u8,
    target_label: usize,
    relation: usize,
    multiplicity: u64,
}

impl GlobalTopology {
    pub(super) fn build(graph: &IncidenceGraph) -> Result<Self, GraphError> {
        let mut adjacency = vec![Vec::new(); graph.vertex_count()];
        for source in 0..graph.vertex_count() {
            for incidence in graph.outgoing(VertexId::new(source)) {
                let target = incidence.neighbor().index();
                if source != target {
                    adjacency[source].push(target);
                    adjacency[target].push(source);
                }
            }
        }
        for neighbors in &mut adjacency {
            neighbors.sort_unstable();
            neighbors.dedup();
        }
        let (weak_components, weak_component_count) = connected_components(&adjacency)?;
        let (sccs, scc_count) = strongly_connected_components(graph)?;
        Ok(Self {
            adjacency,
            weak_components,
            weak_component_count,
            sccs,
            scc_count,
        })
    }
}

fn connected_components(adjacency: &[Vec<usize>]) -> Result<(Vec<usize>, usize), GraphError> {
    let mut components = vec![usize::MAX; adjacency.len()];
    let mut stack = Vec::with_capacity(adjacency.len());
    let mut count = 0_usize;
    for root in 0..adjacency.len() {
        if components[root] != usize::MAX {
            continue;
        }
        let component = count;
        count = count.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
        components[root] = component;
        stack.push(root);
        while let Some(vertex) = stack.pop() {
            for &neighbor in &adjacency[vertex] {
                if components[neighbor] == usize::MAX {
                    components[neighbor] = component;
                    stack.push(neighbor);
                }
            }
        }
    }
    Ok((components, count))
}

fn strongly_connected_components(
    graph: &IncidenceGraph,
) -> Result<(Vec<usize>, usize), GraphError> {
    let mut visited = vec![false; graph.vertex_count()];
    let mut finish = Vec::with_capacity(graph.vertex_count());
    let mut frames = Vec::with_capacity(graph.vertex_count());
    for root in 0..graph.vertex_count() {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        frames.push((root, 0_usize));
        while let Some((vertex, next)) = frames.last_mut() {
            let row = graph.outgoing(VertexId::new(*vertex));
            if *next < row.len() {
                let neighbor = row[*next].neighbor().index();
                *next += 1;
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    frames.push((neighbor, 0));
                }
            } else {
                finish.push(*vertex);
                frames.pop();
            }
        }
    }

    let mut components = vec![usize::MAX; graph.vertex_count()];
    let mut stack = Vec::with_capacity(graph.vertex_count());
    let mut count = 0_usize;
    for &root in finish.iter().rev() {
        if components[root] != usize::MAX {
            continue;
        }
        let component = count;
        count = count.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
        components[root] = component;
        stack.push(root);
        while let Some(vertex) = stack.pop() {
            for incidence in graph.incoming(VertexId::new(vertex)) {
                let neighbor = incidence.neighbor().index();
                if components[neighbor] == usize::MAX {
                    components[neighbor] = component;
                    stack.push(neighbor);
                }
            }
        }
    }
    Ok((components, count))
}

fn build_global_profile(
    graph: &IncidenceGraph,
    topology: &GlobalTopology,
) -> Result<GlobalGraphProfile, GraphError> {
    let mut vertices = vec![Vec::new(); topology.weak_component_count];
    let mut arcs = vec![Vec::new(); topology.weak_component_count];
    let mut summaries = vec![empty_summary(); topology.weak_component_count];
    let mut scc_sizes = vec![0_u64; topology.scc_count];
    let mut scc_weak_components = vec![usize::MAX; topology.scc_count];

    for index in 0..graph.vertex_count() {
        let vertex = VertexId::new(index);
        let component = topology.weak_components[index];
        let summary = &mut summaries[component];
        summary.vertex_count = checked_add(summary.vertex_count, 1)?;
        match graph.vertex_kind(vertex) {
            VertexKind::Entity => summary.entity_count = checked_add(summary.entity_count, 1)?,
            VertexKind::Hyperedge => {
                summary.hyperedge_count = checked_add(summary.hyperedge_count, 1)?
            }
        }
        scc_sizes[topology.sccs[index]] = checked_add(scc_sizes[topology.sccs[index]], 1)?;
        scc_weak_components[topology.sccs[index]] = component;

        let outgoing = graph.outgoing(vertex);
        let incoming = graph.incoming(vertex);
        let outgoing_multiplicity = sum_multiplicity(outgoing)?;
        let incoming_multiplicity = sum_multiplicity(incoming)?;
        vertices[component].push(VertexInvariantRecord {
            kind: graph.vertex_kind(vertex) as u8,
            label: graph.vertex_label_id(vertex),
            outgoing_count: outgoing.len(),
            incoming_count: incoming.len(),
            outgoing_multiplicity,
            incoming_multiplicity,
            support_degree: topology.adjacency[index].len(),
        });

        for incidence in outgoing {
            let target = incidence.neighbor().index();
            summary.incidence_count = checked_add(summary.incidence_count, 1)?;
            summary.total_multiplicity =
                checked_add(summary.total_multiplicity, incidence.multiplicity())?;
            if target == index {
                summary.self_loop_count = checked_add(summary.self_loop_count, 1)?;
                summary.self_loop_multiplicity =
                    checked_add(summary.self_loop_multiplicity, incidence.multiplicity())?;
            }
            arcs[component].push(ArcInvariantRecord {
                source_kind: graph.vertex_kind(vertex) as u8,
                source_label: graph.vertex_label_id(vertex),
                target_kind: graph.vertex_kind(VertexId::new(target)) as u8,
                target_label: graph.vertex_label_id(VertexId::new(target)),
                relation: incidence.relation().index(),
                multiplicity: incidence.multiplicity(),
            });
        }
    }

    for (source, neighbors) in topology.adjacency.iter().enumerate() {
        for &target in neighbors {
            if source < target {
                let component = topology.weak_components[source];
                summaries[component].support_edge_count =
                    checked_add(summaries[component].support_edge_count, 1)?;
            }
        }
    }
    for summary in &mut summaries {
        summary.cycle_rank = summary
            .support_edge_count
            .checked_add(1)
            .and_then(|value| value.checked_sub(summary.vertex_count))
            .unwrap_or(0);
    }
    for &component in &scc_weak_components {
        debug_assert_ne!(component, usize::MAX);
        summaries[component].strongly_connected_component_count =
            checked_add(summaries[component].strongly_connected_component_count, 1)?;
    }
    let mut scc_sizes_by_weak = vec![Vec::new(); topology.weak_component_count];
    for (scc, &size) in scc_sizes.iter().enumerate() {
        let component = scc_weak_components[scc];
        scc_sizes_by_weak[component].push(size);
    }

    let mut component_records = Vec::with_capacity(topology.weak_component_count);
    for component in 0..topology.weak_component_count {
        vertices[component].sort_unstable();
        arcs[component].sort_unstable();
        let component_scc_sizes = &mut scc_sizes_by_weak[component];
        component_scc_sizes.sort_unstable();

        let mut bytes = Vec::new();
        append_summary(&mut bytes, &summaries[component]);
        append_u64(&mut bytes, component_scc_sizes.len())?;
        for &size in component_scc_sizes.iter() {
            bytes.extend_from_slice(&size.to_be_bytes());
        }
        append_vertex_records(&mut bytes, &vertices[component])?;
        append_arc_records(&mut bytes, &arcs[component])?;
        component_records.push((bytes, summaries[component].clone()));
    }
    component_records.sort_unstable_by(|left, right| left.0.cmp(&right.0));

    let mut canonical_bytes = Vec::new();
    canonical_bytes.extend_from_slice(GLOBAL_MAGIC);
    canonical_bytes.extend_from_slice(&GLOBAL_SCHEMA.to_be_bytes());
    append_u64(&mut canonical_bytes, graph.vertex_count())?;
    append_u64(&mut canonical_bytes, graph.incidence_count())?;
    canonical_bytes.extend_from_slice(&graph.total_multiplicity().to_be_bytes());
    append_u64(&mut canonical_bytes, component_records.len())?;
    append_u64(&mut canonical_bytes, topology.scc_count)?;
    append_u64(&mut canonical_bytes, graph.labels().len())?;
    for label in graph.labels() {
        append_framed(&mut canonical_bytes, label)?;
    }
    append_u64(&mut canonical_bytes, graph.descriptors().len())?;
    for descriptor in graph.descriptors() {
        append_framed(&mut canonical_bytes, descriptor.relation())?;
        append_framed(&mut canonical_bytes, descriptor.role())?;
    }
    for (bytes, _) in &component_records {
        append_framed(&mut canonical_bytes, bytes)?;
    }
    let digest = GlobalInvariantDigest(Sha256::digest(&canonical_bytes).into());
    let weak_components = component_records
        .into_iter()
        .map(|(_, summary)| summary)
        .collect();
    Ok(GlobalGraphProfile {
        weak_components,
        strongly_connected_component_count: u64::try_from(topology.scc_count)
            .map_err(|_| GraphError::GraphTooLarge)?,
        canonical_bytes,
        digest,
    })
}

const fn empty_summary() -> WeakComponentSummary {
    WeakComponentSummary {
        vertex_count: 0,
        entity_count: 0,
        hyperedge_count: 0,
        incidence_count: 0,
        total_multiplicity: 0,
        support_edge_count: 0,
        self_loop_count: 0,
        self_loop_multiplicity: 0,
        strongly_connected_component_count: 0,
        cycle_rank: 0,
    }
}

fn analyze_motifs(
    adjacency: &[Vec<usize>],
    highly_regular: bool,
    policy: GraphDiscriminationPolicy,
) -> Result<BoundedMotifProfile, GraphError> {
    let max_work = match policy {
        GraphDiscriminationPolicy::GlobalLinear => {
            return Ok(BoundedMotifProfile {
                status: MotifAnalysisStatus::NotRequested,
                estimated_work: 0,
                triangle_count: None,
                four_clique_count: None,
            });
        }
        GraphDiscriminationPolicy::Adaptive { max_motif_work: _ } if !highly_regular => {
            return Ok(BoundedMotifProfile {
                status: MotifAnalysisStatus::NotNeeded,
                estimated_work: 0,
                triangle_count: None,
                four_clique_count: None,
            });
        }
        GraphDiscriminationPolicy::Adaptive { max_motif_work } => max_motif_work,
    };
    let estimated_work = adjacency.iter().try_fold(0_u64, |total, neighbors| {
        let degree = u64::try_from(neighbors.len()).map_err(|_| GraphError::GraphTooLarge)?;
        let pairs = choose(degree, 2).ok_or(GraphError::GraphTooLarge)?;
        let triples = choose(degree, 3).ok_or(GraphError::GraphTooLarge)?;
        total
            .checked_add(pairs)
            .and_then(|value| value.checked_add(triples))
            .ok_or(GraphError::GraphTooLarge)
    })?;
    if estimated_work > max_work {
        return Ok(BoundedMotifProfile {
            status: MotifAnalysisStatus::SkippedBudget,
            estimated_work,
            triangle_count: None,
            four_clique_count: None,
        });
    }

    let mut triangles = 0_u64;
    let mut four_cliques = 0_u64;
    for root in 0..adjacency.len() {
        let start = adjacency[root].partition_point(|neighbor| *neighbor <= root);
        let forward = &adjacency[root][start..];
        for left in 0..forward.len() {
            for right in left + 1..forward.len() {
                if has_edge(adjacency, forward[left], forward[right]) {
                    triangles = checked_add(triangles, 1)?;
                }
            }
        }
        for first in 0..forward.len() {
            for second in first + 1..forward.len() {
                if !has_edge(adjacency, forward[first], forward[second]) {
                    continue;
                }
                for third in second + 1..forward.len() {
                    if has_edge(adjacency, forward[first], forward[third])
                        && has_edge(adjacency, forward[second], forward[third])
                    {
                        four_cliques = checked_add(four_cliques, 1)?;
                    }
                }
            }
        }
    }
    Ok(BoundedMotifProfile {
        status: MotifAnalysisStatus::Complete,
        estimated_work,
        triangle_count: Some(triangles),
        four_clique_count: Some(four_cliques),
    })
}

fn has_edge(adjacency: &[Vec<usize>], left: usize, right: usize) -> bool {
    adjacency[left].binary_search(&right).is_ok()
}

fn choose(value: u64, count: u8) -> Option<u64> {
    match count {
        2 if value >= 2 => value.checked_mul(value - 1).map(|product| product / 2),
        3 if value >= 3 => value
            .checked_mul(value - 1)?
            .checked_mul(value - 2)
            .map(|product| product / 6),
        2 | 3 => Some(0),
        _ => None,
    }
}

fn is_highly_regular(partition: &[usize], cell_count: usize) -> bool {
    if partition.len() < 4 {
        return false;
    }
    let mut sizes = vec![0_usize; cell_count];
    for &cell in partition {
        sizes[cell] += 1;
    }
    let ambiguous: usize = sizes.iter().copied().filter(|size| *size > 1).sum();
    let largest = sizes.iter().copied().max().unwrap_or(0);
    ambiguous >= partition.len().saturating_sub(partition.len() / 4)
        && largest >= partition.len().div_ceil(4)
}

fn derive_discrimination_id(
    signature_id: GraphSignatureId,
    policy: GraphDiscriminationPolicy,
) -> GraphDiscriminationId {
    let mut hasher = Sha256::new();
    hasher.update(DISCRIMINATOR_MAGIC);
    hasher.update(GLOBAL_SCHEMA.to_be_bytes());
    hasher.update(signature_id.as_bytes());
    match policy {
        GraphDiscriminationPolicy::GlobalLinear => hasher.update([0]),
        GraphDiscriminationPolicy::Adaptive { max_motif_work } => {
            hasher.update([1]);
            hasher.update(max_motif_work.to_be_bytes());
        }
    }
    GraphDiscriminationId(hasher.finalize().into())
}

fn derive_discrimination_digest<F, const K: usize>(
    profile_id: GraphDiscriminationId,
    hybrid: &HybridGraphAnalysis<F, K>,
    global: &GlobalGraphProfile,
    motifs: &BoundedMotifProfile,
) -> GraphDiscriminationDigest
where
    F: Field + CanonicalEncoding,
{
    let mut hasher = Sha256::new();
    hasher.update(DISCRIMINATOR_MAGIC);
    hasher.update(GLOBAL_SCHEMA.to_be_bytes());
    hasher.update(profile_id.as_bytes());
    hasher.update(hybrid.structural().signature().to_canonical_bytes());
    hasher.update(hybrid.invariant_digest().as_bytes());
    hasher.update(global.to_canonical_bytes());
    hasher.update([motifs.status as u8]);
    hasher.update(motifs.estimated_work.to_be_bytes());
    if let Some(value) = motifs.triangle_count {
        hasher.update([1]);
        hasher.update(value.to_be_bytes());
    } else {
        hasher.update([0]);
    }
    if let Some(value) = motifs.four_clique_count {
        hasher.update([1]);
        hasher.update(value.to_be_bytes());
    } else {
        hasher.update([0]);
    }
    GraphDiscriminationDigest(hasher.finalize().into())
}

fn append_summary(bytes: &mut Vec<u8>, summary: &WeakComponentSummary) {
    for value in [
        summary.vertex_count,
        summary.entity_count,
        summary.hyperedge_count,
        summary.incidence_count,
        summary.total_multiplicity,
        summary.support_edge_count,
        summary.self_loop_count,
        summary.self_loop_multiplicity,
        summary.strongly_connected_component_count,
        summary.cycle_rank,
    ] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
}

fn append_vertex_records(
    bytes: &mut Vec<u8>,
    records: &[VertexInvariantRecord],
) -> Result<(), GraphError> {
    append_u64(bytes, records.len())?;
    for record in records {
        bytes.push(record.kind);
        append_u64(bytes, record.label)?;
        append_u64(bytes, record.outgoing_count)?;
        append_u64(bytes, record.incoming_count)?;
        bytes.extend_from_slice(&record.outgoing_multiplicity.to_be_bytes());
        bytes.extend_from_slice(&record.incoming_multiplicity.to_be_bytes());
        append_u64(bytes, record.support_degree)?;
    }
    Ok(())
}

fn append_arc_records(
    bytes: &mut Vec<u8>,
    records: &[ArcInvariantRecord],
) -> Result<(), GraphError> {
    append_u64(bytes, records.len())?;
    for record in records {
        bytes.push(record.source_kind);
        append_u64(bytes, record.source_label)?;
        bytes.push(record.target_kind);
        append_u64(bytes, record.target_label)?;
        append_u64(bytes, record.relation)?;
        bytes.extend_from_slice(&record.multiplicity.to_be_bytes());
    }
    Ok(())
}

fn append_framed(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), GraphError> {
    append_u64(bytes, value.len())?;
    bytes.extend_from_slice(value);
    Ok(())
}

fn append_u64(bytes: &mut Vec<u8>, value: usize) -> Result<(), GraphError> {
    bytes.extend_from_slice(
        &u64::try_from(value)
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_be_bytes(),
    );
    Ok(())
}

fn sum_multiplicity(incidences: &[super::Incidence]) -> Result<u64, GraphError> {
    incidences.iter().try_fold(0_u64, |total, incidence| {
        checked_add(total, incidence.multiplicity())
    })
}

fn checked_add(left: u64, right: u64) -> Result<u64, GraphError> {
    left.checked_add(right).ok_or(GraphError::GraphTooLarge)
}

fn write_hex(formatter: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}
