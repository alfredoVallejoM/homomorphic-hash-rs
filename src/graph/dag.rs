//! Exact persistent DAG for canonical subnetworks.
//!
//! Digest keys and structural signatures are lookup accelerators only. A node
//! is reused exclusively after complete canonical bytes compare equal.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    CanonicalGraphDocument, CanonicalGraphForm, CanonicalGraphKey, CanonicalSearchBudget,
    GraphDeltaUpdateReport, GraphError, GraphSchemaId, IncidenceGraph, IncidenceGraphBuilder,
    Microcanon, MicrocanonOutcome, RelationDescriptor, VertexId, VertexKind,
};

const MAGIC: &[u8; 4] = b"MFGD";
const VERSION: u16 = 1;

/// Stable insertion identifier inside one persisted DAG revision lineage.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct GraphDagNodeId(u64);

impl GraphDagNodeId {
    /// Stable wire value.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// One exact canonical graph and its already-persisted decomposition children.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphDagNode {
    id: GraphDagNodeId,
    key: CanonicalGraphKey,
    canonical_bytes: Vec<u8>,
    dependencies: Vec<GraphDagNodeId>,
}

impl GraphDagNode {
    #[must_use]
    pub const fn id(&self) -> GraphDagNodeId {
        self.id
    }
    #[must_use]
    pub const fn key(&self) -> CanonicalGraphKey {
        self.key
    }
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
    #[must_use]
    pub fn dependencies(&self) -> &[GraphDagNodeId] {
        &self.dependencies
    }
    /// Strictly decodes the retained canonical document.
    pub fn document(&self) -> Result<CanonicalGraphDocument, GraphError> {
        CanonicalGraphDocument::from_bytes(&self.canonical_bytes)
    }
}

/// Hard parsing/admission limits for an untrusted DAG snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanonicalGraphDagLimits {
    pub maximum_nodes: usize,
    pub maximum_dependencies_per_node: usize,
    pub maximum_canonical_bytes_per_node: usize,
    pub maximum_snapshot_bytes: usize,
}

impl Default for CanonicalGraphDagLimits {
    fn default() -> Self {
        Self {
            maximum_nodes: 1_000_000,
            maximum_dependencies_per_node: 1_000_000,
            maximum_canonical_bytes_per_node: 256 * 1024 * 1024,
            maximum_snapshot_bytes: 1024 * 1024 * 1024,
        }
    }
}

/// Why an existing graph state required a fresh exact canonical form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphDagUpdateKind {
    NoChange,
    Labels,
    Topology,
    LabelsAndTopology,
}

/// Observable exact lookup/commit work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphDagResolveReport {
    revision: u64,
    lookup_candidates: usize,
    digest_candidates: usize,
    exact_byte_comparisons: usize,
    update_kind: Option<GraphDagUpdateKind>,
}

impl GraphDagResolveReport {
    #[must_use]
    pub const fn revision(self) -> u64 {
        self.revision
    }
    /// Nodes retained by the complete cheap metadata lookup.
    #[must_use]
    pub const fn lookup_candidates(self) -> usize {
        self.lookup_candidates
    }
    #[must_use]
    pub const fn digest_candidates(self) -> usize {
        self.digest_candidates
    }
    #[must_use]
    pub const fn exact_byte_comparisons(self) -> usize {
        self.exact_byte_comparisons
    }
    #[must_use]
    pub const fn update_kind(self) -> Option<GraphDagUpdateKind> {
        self.update_kind
    }
}

/// Transactional result. Inconclusive exact searches never mutate the DAG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphDagResolveOutcome {
    Inserted {
        node: GraphDagNodeId,
        report: GraphDagResolveReport,
    },
    Reused {
        node: GraphDagNodeId,
        report: GraphDagResolveReport,
    },
    Inconclusive,
}

/// Persistent exact graph DAG. `key_index` is never authoritative.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalGraphDag {
    schema_id: GraphSchemaId,
    producer_version: String,
    revision: u64,
    nodes: Vec<GraphDagNode>,
    metadata_index: BTreeMap<(usize, usize, u64), Vec<GraphDagNodeId>>,
    key_index: BTreeMap<CanonicalGraphKey, Vec<GraphDagNodeId>>,
}

impl CanonicalGraphDag {
    #[must_use]
    pub fn new(schema_id: GraphSchemaId) -> Self {
        Self {
            schema_id,
            producer_version: env!("CARGO_PKG_VERSION").to_owned(),
            revision: 0,
            nodes: Vec::new(),
            metadata_index: BTreeMap::new(),
            key_index: BTreeMap::new(),
        }
    }
    #[must_use]
    pub const fn schema_id(&self) -> GraphSchemaId {
        self.schema_id
    }
    /// Library version that produced this in-memory snapshot.
    #[must_use]
    pub fn producer_version(&self) -> &str {
        &self.producer_version
    }
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    #[must_use]
    pub fn nodes(&self) -> &[GraphDagNode] {
        &self.nodes
    }
    #[must_use]
    pub fn node(&self, id: GraphDagNodeId) -> Option<&GraphDagNode> {
        usize::try_from(id.0)
            .ok()
            .and_then(|index| self.nodes.get(index))
    }

    /// Runs exact canonization and atomically inserts or reuses one node.
    pub fn resolve(
        &mut self,
        graph: &IncidenceGraph,
        canonizer: &Microcanon,
        budget: CanonicalSearchBudget,
        dependencies: &[GraphDagNodeId],
        expected_revision: Option<u64>,
    ) -> Result<GraphDagResolveOutcome, GraphError> {
        self.resolve_with_kind(
            graph,
            canonizer,
            budget,
            dependencies,
            expected_revision,
            None,
        )
    }

    /// Same exact flow after a G14 edit, retaining the measured invalidation kind.
    pub fn resolve_after_delta(
        &mut self,
        graph: &IncidenceGraph,
        delta: GraphDeltaUpdateReport,
        canonizer: &Microcanon,
        budget: CanonicalSearchBudget,
        dependencies: &[GraphDagNodeId],
        expected_revision: Option<u64>,
    ) -> Result<GraphDagResolveOutcome, GraphError> {
        let kind = match (delta.label_changed(), delta.topology_changed()) {
            (false, false) => GraphDagUpdateKind::NoChange,
            (true, false) => GraphDagUpdateKind::Labels,
            (false, true) => GraphDagUpdateKind::Topology,
            (true, true) => GraphDagUpdateKind::LabelsAndTopology,
        };
        self.resolve_with_kind(
            graph,
            canonizer,
            budget,
            dependencies,
            expected_revision,
            Some(kind),
        )
    }

    fn resolve_with_kind(
        &mut self,
        graph: &IncidenceGraph,
        canonizer: &Microcanon,
        budget: CanonicalSearchBudget,
        dependencies: &[GraphDagNodeId],
        expected_revision: Option<u64>,
        update_kind: Option<GraphDagUpdateKind>,
    ) -> Result<GraphDagResolveOutcome, GraphError> {
        if canonizer.schema_id() != self.schema_id {
            return Err(GraphError::InvalidGraphDagEncoding);
        }
        if let Some(expected) = expected_revision {
            if expected != self.revision {
                return Err(GraphError::GraphDagRevisionMismatch {
                    expected,
                    actual: self.revision,
                });
            }
        }
        // Lookup/filter is deliberately negative-only: equality in this bucket
        // never creates identity and cannot skip exact canonization.
        let metadata = (
            graph.vertex_count(),
            graph.incidence_count(),
            graph.total_multiplicity(),
        );
        let lookup_candidates = self.metadata_index.get(&metadata).map_or(0, Vec::len);
        let form = match canonizer.canonicalize(graph, budget)? {
            MicrocanonOutcome::Exact { form, .. } => form,
            MicrocanonOutcome::Inconclusive { .. } => {
                return Ok(GraphDagResolveOutcome::Inconclusive)
            }
        };
        self.commit_form(form, dependencies, update_kind, lookup_candidates)
    }

    fn commit_form(
        &mut self,
        form: CanonicalGraphForm,
        dependencies: &[GraphDagNodeId],
        update_kind: Option<GraphDagUpdateKind>,
        lookup_candidates: usize,
    ) -> Result<GraphDagResolveOutcome, GraphError> {
        if form.schema_id() != self.schema_id {
            return Err(GraphError::InvalidGraphDagEncoding);
        }
        let dependencies = self.preflight_dependencies(dependencies)?;
        let candidates = self.key_index.get(&form.key()).cloned().unwrap_or_default();
        let mut comparisons = 0;
        for id in &candidates {
            comparisons += 1;
            let node = self.node(*id).ok_or(GraphError::InvalidGraphDagEncoding)?;
            if node.canonical_bytes == form.bytes() {
                if node.dependencies != dependencies {
                    return Err(GraphError::GraphDagDependencyMismatch);
                }
                return Ok(GraphDagResolveOutcome::Reused {
                    node: *id,
                    report: GraphDagResolveReport {
                        revision: self.revision,
                        lookup_candidates,
                        digest_candidates: candidates.len(),
                        exact_byte_comparisons: comparisons,
                        update_kind,
                    },
                });
            }
        }
        let id =
            GraphDagNodeId(u64::try_from(self.nodes.len()).map_err(|_| GraphError::GraphTooLarge)?);
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(GraphError::GraphTooLarge)?;
        let document = form.decode()?;
        let graph_metadata = (
            document.graph().vertex_count(),
            document.graph().incidence_count(),
            document.graph().total_multiplicity(),
        );
        self.nodes.push(GraphDagNode {
            id,
            key: form.key(),
            canonical_bytes: form.bytes().to_vec(),
            dependencies,
        });
        self.metadata_index
            .entry(graph_metadata)
            .or_default()
            .push(id);
        self.key_index.entry(form.key()).or_default().push(id);
        self.revision = revision;
        Ok(GraphDagResolveOutcome::Inserted {
            node: id,
            report: GraphDagResolveReport {
                revision,
                lookup_candidates,
                digest_candidates: candidates.len(),
                exact_byte_comparisons: comparisons,
                update_kind,
            },
        })
    }

    fn preflight_dependencies(
        &self,
        dependencies: &[GraphDagNodeId],
    ) -> Result<Vec<GraphDagNodeId>, GraphError> {
        let mut normalized = dependencies.to_vec();
        normalized.sort_unstable();
        normalized.dedup();
        if normalized.iter().any(|id| self.node(*id).is_none()) {
            return Err(GraphError::InvalidGraphDagEncoding);
        }
        Ok(normalized)
    }

    /// Stable self-delimiting snapshot. Exact graph bytes remain the identity source.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_be_bytes());
        out.extend_from_slice(self.schema_id.as_bytes());
        put_len(&mut out, self.producer_version.len());
        out.extend_from_slice(self.producer_version.as_bytes());
        out.extend_from_slice(&self.revision.to_be_bytes());
        put_len(&mut out, self.nodes.len());
        for node in &self.nodes {
            out.extend_from_slice(&node.id.0.to_be_bytes());
            put_len(&mut out, node.dependencies.len());
            for dependency in &node.dependencies {
                out.extend_from_slice(&dependency.0.to_be_bytes());
            }
            put_len(&mut out, node.canonical_bytes.len());
            out.extend_from_slice(&node.canonical_bytes);
        }
        out
    }

    /// Restores and independently re-canonicalizes every retained node before publication.
    pub fn from_canonical_bytes(
        bytes: &[u8],
        canonizer: &Microcanon,
        budget_per_node: CanonicalSearchBudget,
        limits: CanonicalGraphDagLimits,
    ) -> Result<Self, GraphError> {
        if bytes.len() > limits.maximum_snapshot_bytes {
            return Err(GraphError::GraphDagLimitExceeded);
        }
        let mut input = Input::new(bytes);
        if input.take(4)? != MAGIC || input.u16()? != VERSION {
            return Err(GraphError::InvalidGraphDagEncoding);
        }
        let mut schema = [0; 32];
        schema.copy_from_slice(input.take(32)?);
        let schema_id = GraphSchemaId::from_bytes(schema);
        if canonizer.schema_id() != schema_id {
            return Err(GraphError::InvalidGraphDagEncoding);
        }
        let producer_length = input.len()?;
        if producer_length > 256 {
            return Err(GraphError::GraphDagLimitExceeded);
        }
        let producer_version = core::str::from_utf8(input.take(producer_length)?)
            .map_err(|_| GraphError::InvalidGraphDagEncoding)?
            .to_owned();
        let revision = input.u64()?;
        let count = input.len()?;
        if count > limits.maximum_nodes {
            return Err(GraphError::GraphDagLimitExceeded);
        }
        let mut dag = Self::new(schema_id);
        dag.producer_version = producer_version;
        for index in 0..count {
            if input.u64()? != u64::try_from(index).map_err(|_| GraphError::GraphTooLarge)? {
                return Err(GraphError::InvalidGraphDagEncoding);
            }
            let dep_count = input.len()?;
            if dep_count > limits.maximum_dependencies_per_node {
                return Err(GraphError::GraphDagLimitExceeded);
            }
            let mut dependencies = Vec::with_capacity(dep_count);
            for _ in 0..dep_count {
                let dependency = GraphDagNodeId(input.u64()?);
                if dependency.0 >= index as u64 {
                    return Err(GraphError::InvalidGraphDagEncoding);
                }
                dependencies.push(dependency);
            }
            if !dependencies.windows(2).all(|pair| pair[0] < pair[1]) {
                return Err(GraphError::InvalidGraphDagEncoding);
            }
            let byte_count = input.len()?;
            if byte_count > limits.maximum_canonical_bytes_per_node {
                return Err(GraphError::GraphDagLimitExceeded);
            }
            let canonical_bytes = input.take(byte_count)?.to_vec();
            let document = CanonicalGraphDocument::from_bytes(&canonical_bytes)?;
            if document.schema_id() != schema_id {
                return Err(GraphError::InvalidGraphDagEncoding);
            }
            let verified = match canonizer.canonicalize(document.graph(), budget_per_node)? {
                MicrocanonOutcome::Exact { form, .. } if form.bytes() == canonical_bytes => form,
                _ => return Err(GraphError::InvalidGraphDagEncoding),
            };
            let graph = document.graph();
            let lookup_candidates = dag
                .metadata_index
                .get(&(
                    graph.vertex_count(),
                    graph.incidence_count(),
                    graph.total_multiplicity(),
                ))
                .map_or(0, Vec::len);
            match dag.commit_form(verified, &dependencies, None, lookup_candidates)? {
                GraphDagResolveOutcome::Inserted { .. } => {}
                _ => return Err(GraphError::InvalidGraphDagEncoding),
            }
        }
        if input.remaining() != 0 || dag.revision != revision || revision != count as u64 {
            return Err(GraphError::InvalidGraphDagEncoding);
        }
        Ok(dag)
    }
}

/// Explicit, loss-aware adapters for exact induced subnetworks and cliques.
pub struct GraphSubnetworkAdapter;

impl GraphSubnetworkAdapter {
    /// Extracts the exact induced graph. Dropping boundary incidences is explicit in this API.
    pub fn induced(
        graph: &IncidenceGraph,
        vertices: &[VertexId],
    ) -> Result<IncidenceGraph, GraphError> {
        extract(graph, vertices, false)
    }

    /// Extracts only when the selection has no incoming or outgoing boundary incidence.
    pub fn closed(
        graph: &IncidenceGraph,
        vertices: &[VertexId],
    ) -> Result<IncidenceGraph, GraphError> {
        extract(graph, vertices, true)
    }

    /// Validates a directed complete relation and preserves every internal label,
    /// role, direction and multiplicity in the returned induced graph.
    pub fn relational_clique(
        graph: &IncidenceGraph,
        vertices: &[VertexId],
        relation: &[u8],
        role: &[u8],
    ) -> Result<IncidenceGraph, GraphError> {
        let selected = validate_selection(graph, vertices)?;
        for vertex in vertices {
            if graph.vertex_kind(*vertex) != VertexKind::Entity {
                return Err(GraphError::NonEntityCliqueVertex {
                    index: vertex.index(),
                });
            }
        }
        for source in vertices {
            for target in vertices {
                if source == target {
                    continue;
                }
                let found = graph.outgoing(*source).iter().any(|arc| {
                    let descriptor = graph.relation(arc.relation());
                    arc.neighbor() == *target
                        && descriptor.relation() == relation
                        && descriptor.role() == role
                });
                if !found {
                    return Err(GraphError::MissingCliqueRelation {
                        source: source.index(),
                        target: target.index(),
                    });
                }
            }
        }
        extract_selected(graph, vertices, &selected)
    }
}

fn extract(
    graph: &IncidenceGraph,
    vertices: &[VertexId],
    closed: bool,
) -> Result<IncidenceGraph, GraphError> {
    let selected = validate_selection(graph, vertices)?;
    if closed {
        for source in vertices {
            for arc in graph.outgoing(*source) {
                if !selected.contains(&arc.neighbor().index()) {
                    return Err(GraphError::OpenSubgraphBoundary {
                        source: source.index(),
                        target: arc.neighbor().index(),
                    });
                }
            }
            for arc in graph.incoming(*source) {
                if !selected.contains(&arc.neighbor().index()) {
                    return Err(GraphError::OpenSubgraphBoundary {
                        source: arc.neighbor().index(),
                        target: source.index(),
                    });
                }
            }
        }
    }
    extract_selected(graph, vertices, &selected)
}

fn validate_selection(
    graph: &IncidenceGraph,
    vertices: &[VertexId],
) -> Result<BTreeSet<usize>, GraphError> {
    let mut selected = BTreeSet::new();
    for vertex in vertices {
        if !graph.contains_vertex(*vertex) {
            return Err(GraphError::InvalidVertex {
                index: vertex.index(),
                vertex_count: graph.vertex_count(),
            });
        }
        if !selected.insert(vertex.index()) {
            return Err(GraphError::DuplicateSubgraphVertex {
                index: vertex.index(),
            });
        }
    }
    Ok(selected)
}

fn extract_selected(
    graph: &IncidenceGraph,
    vertices: &[VertexId],
    selected: &BTreeSet<usize>,
) -> Result<IncidenceGraph, GraphError> {
    let mut builder = IncidenceGraphBuilder::new();
    let mut mapping = BTreeMap::new();
    for vertex in vertices {
        let new = builder.add_typed_vertex(graph.vertex_kind(*vertex), graph.vertex_label(*vertex));
        mapping.insert(vertex.index(), new);
    }
    for source in vertices {
        for arc in graph.outgoing(*source) {
            if selected.contains(&arc.neighbor().index()) {
                let descriptor: &RelationDescriptor = graph.relation(arc.relation());
                builder.add_directed_relation(
                    mapping[&source.index()],
                    mapping[&arc.neighbor().index()],
                    descriptor.relation(),
                    descriptor.role(),
                    arc.multiplicity(),
                )?;
            }
        }
    }
    builder.build()
}

fn put_len(output: &mut Vec<u8>, value: usize) {
    output.extend_from_slice(&(value as u64).to_be_bytes());
}

struct Input<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Input<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    const fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], GraphError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(GraphError::InvalidGraphDagEncoding)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(GraphError::InvalidGraphDagEncoding)?;
        self.offset = end;
        Ok(value)
    }
    fn u16(&mut self) -> Result<u16, GraphError> {
        let mut b = [0; 2];
        b.copy_from_slice(self.take(2)?);
        Ok(u16::from_be_bytes(b))
    }
    fn u64(&mut self) -> Result<u64, GraphError> {
        let mut b = [0; 8];
        b.copy_from_slice(self.take(8)?);
        Ok(u64::from_be_bytes(b))
    }
    fn len(&mut self) -> Result<usize, GraphError> {
        usize::try_from(self.u64()?).map_err(|_| GraphError::InvalidGraphDagEncoding)
    }
}
