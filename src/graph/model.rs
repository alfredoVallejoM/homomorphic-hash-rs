//! Exact normalized graph model backed by incoming and outgoing CSR arrays.

use super::GraphError;

const HYPEREDGE_RELATION: &[u8] = b"microfield/hyperedge-incidence-v1";

/// Stable index of one vertex inside a normalized graph.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VertexId(usize);

impl VertexId {
    /// Creates an identifier from an index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the underlying contiguous index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Stable index of an exact relation/role descriptor.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RelationId(usize);

impl RelationId {
    /// Returns the underlying descriptor index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Semantic kind of a normalized vertex.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum VertexKind {
    /// A logical application vertex.
    Entity = 1,
    /// An auxiliary vertex representing one exact hyperedge.
    Hyperedge = 2,
}

/// Exact relation metadata shared by normalized incidences.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RelationDescriptor {
    relation: Vec<u8>,
    role: Vec<u8>,
}

impl RelationDescriptor {
    /// Exact relation label bytes.
    #[must_use]
    pub fn relation(&self) -> &[u8] {
        &self.relation
    }

    /// Exact role or port bytes within the relation.
    #[must_use]
    pub fn role(&self) -> &[u8] {
        &self.role
    }
}

/// One normalized directed incidence in a CSR row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Incidence {
    neighbor: VertexId,
    relation: RelationId,
    multiplicity: u64,
}

impl Incidence {
    /// Vertex at the other end of this directed incidence.
    #[must_use]
    pub const fn neighbor(self) -> VertexId {
        self.neighbor
    }

    /// Exact relation/role descriptor identifier.
    #[must_use]
    pub const fn relation(self) -> RelationId {
        self.relation
    }

    /// Number of identical directed incidences compressed into this record.
    #[must_use]
    pub const fn multiplicity(self) -> u64 {
        self.multiplicity
    }
}

#[derive(Clone, Debug)]
struct PendingVertex {
    kind: VertexKind,
    label: Vec<u8>,
}

#[derive(Clone, Debug)]
struct PendingArc {
    source: VertexId,
    target: VertexId,
    descriptor: RelationDescriptor,
    multiplicity: u64,
}

/// Owned incidence supplied to [`IncidenceGraphBuilder::add_hyperedge`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperedgeIncidence {
    /// Logical vertex participating in the hyperedge.
    pub vertex: VertexId,
    /// Exact port or role. Use empty bytes for an unordered hyperedge.
    pub role: Vec<u8>,
    /// Exact incidence multiplicity.
    pub multiplicity: u64,
}

impl HyperedgeIncidence {
    /// Creates an incidence with multiplicity one.
    #[must_use]
    pub fn new(vertex: VertexId, role: impl Into<Vec<u8>>) -> Self {
        Self {
            vertex,
            role: role.into(),
            multiplicity: 1,
        }
    }

    /// Replaces the exact incidence multiplicity.
    #[must_use]
    pub const fn with_multiplicity(mut self, multiplicity: u64) -> Self {
        self.multiplicity = multiplicity;
        self
    }
}

/// Transactional builder for the exact directed relation model.
#[derive(Clone, Debug, Default)]
pub struct IncidenceGraphBuilder {
    vertices: Vec<PendingVertex>,
    arcs: Vec<PendingArc>,
}

impl IncidenceGraphBuilder {
    /// Creates an empty graph builder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            vertices: Vec::new(),
            arcs: Vec::new(),
        }
    }

    /// Adds one logical vertex with an exact byte label.
    pub fn add_vertex(&mut self, label: impl Into<Vec<u8>>) -> VertexId {
        self.add_typed_vertex(VertexKind::Entity, label)
    }

    /// Adds one vertex with an explicit semantic kind.
    pub fn add_typed_vertex(&mut self, kind: VertexKind, label: impl Into<Vec<u8>>) -> VertexId {
        let id = VertexId(self.vertices.len());
        self.vertices.push(PendingVertex {
            kind,
            label: label.into(),
        });
        id
    }

    /// Adds one directed relation without expanding its multiplicity.
    ///
    /// # Errors
    ///
    /// Rejects invalid endpoints and zero multiplicity before mutation.
    pub fn add_directed_relation(
        &mut self,
        source: VertexId,
        target: VertexId,
        relation: impl Into<Vec<u8>>,
        role: impl Into<Vec<u8>>,
        multiplicity: u64,
    ) -> Result<(), GraphError> {
        self.validate_vertex(source)?;
        self.validate_vertex(target)?;
        if multiplicity == 0 {
            return Err(GraphError::ZeroMultiplicity);
        }
        self.arcs.push(PendingArc {
            source,
            target,
            descriptor: RelationDescriptor {
                relation: relation.into(),
                role: role.into(),
            },
            multiplicity,
        });
        Ok(())
    }

    /// Adds a symmetric relation as two directed incidences.
    ///
    /// A self-loop is represented once because the same directed record is
    /// visible in both its incoming and outgoing CSR rows.
    ///
    /// # Errors
    ///
    /// Rejects invalid endpoints and zero multiplicity before mutation.
    pub fn add_undirected_relation(
        &mut self,
        left: VertexId,
        right: VertexId,
        relation: impl Into<Vec<u8>>,
        role: impl Into<Vec<u8>>,
        multiplicity: u64,
    ) -> Result<(), GraphError> {
        self.validate_vertex(left)?;
        self.validate_vertex(right)?;
        if multiplicity == 0 {
            return Err(GraphError::ZeroMultiplicity);
        }
        let descriptor = RelationDescriptor {
            relation: relation.into(),
            role: role.into(),
        };
        self.arcs.push(PendingArc {
            source: left,
            target: right,
            descriptor: descriptor.clone(),
            multiplicity,
        });
        if left != right {
            self.arcs.push(PendingArc {
                source: right,
                target: left,
                descriptor,
                multiplicity,
            });
        }
        Ok(())
    }

    /// Adds an exact hyperedge vertex and bidirectional incidence relations.
    ///
    /// # Errors
    ///
    /// Validates every logical vertex and multiplicity before publishing the
    /// hyperedge or any of its incidences.
    pub fn add_hyperedge(
        &mut self,
        label: impl Into<Vec<u8>>,
        incidences: &[HyperedgeIncidence],
    ) -> Result<VertexId, GraphError> {
        for incidence in incidences {
            self.validate_vertex(incidence.vertex)?;
            if self.vertices[incidence.vertex.index()].kind != VertexKind::Entity {
                return Err(GraphError::InvalidHyperedgeEndpoint {
                    index: incidence.vertex.index(),
                });
            }
            if incidence.multiplicity == 0 {
                return Err(GraphError::ZeroMultiplicity);
            }
        }
        let hyperedge = self.add_typed_vertex(VertexKind::Hyperedge, label);
        for incidence in incidences {
            self.add_directed_relation(
                incidence.vertex,
                hyperedge,
                HYPEREDGE_RELATION,
                incidence.role.clone(),
                incidence.multiplicity,
            )?;
            self.add_directed_relation(
                hyperedge,
                incidence.vertex,
                HYPEREDGE_RELATION,
                incidence.role.clone(),
                incidence.multiplicity,
            )?;
        }
        Ok(hyperedge)
    }

    /// Normalizes labels, descriptors, duplicate arcs and both CSR directions.
    ///
    /// # Errors
    ///
    /// Rejects multiplicity and stable-size overflows without publishing a
    /// partial graph.
    pub fn build(self) -> Result<IncidenceGraph, GraphError> {
        let vertex_count = self.vertices.len();
        u64::try_from(vertex_count).map_err(|_| GraphError::GraphTooLarge)?;

        let mut labels: Vec<Vec<u8>> = self.vertices.iter().map(|v| v.label.clone()).collect();
        labels.sort();
        labels.dedup();
        let vertex_label_ids = self
            .vertices
            .iter()
            .map(|vertex| {
                labels
                    .binary_search(&vertex.label)
                    .expect("label pool was built from every vertex")
            })
            .collect();
        let vertex_kinds = self.vertices.iter().map(|vertex| vertex.kind).collect();

        let mut descriptors: Vec<RelationDescriptor> =
            self.arcs.iter().map(|arc| arc.descriptor.clone()).collect();
        descriptors.sort();
        descriptors.dedup();

        let mut arcs = Vec::with_capacity(self.arcs.len());
        for arc in self.arcs {
            let relation = RelationId(
                descriptors
                    .binary_search(&arc.descriptor)
                    .expect("descriptor pool was built from every arc"),
            );
            arcs.push(NormalizedArc {
                source: arc.source,
                target: arc.target,
                relation,
                multiplicity: arc.multiplicity,
            });
        }
        arcs.sort_by_key(|arc| (arc.source, arc.target, arc.relation));

        let mut merged: Vec<NormalizedArc> = Vec::with_capacity(arcs.len());
        for arc in arcs {
            let duplicate = merged.last().is_some_and(|previous| {
                previous.source == arc.source
                    && previous.target == arc.target
                    && previous.relation == arc.relation
            });
            if duplicate {
                let previous = merged
                    .last_mut()
                    .expect("duplicate detection requires a previous arc");
                previous.multiplicity = previous
                    .multiplicity
                    .checked_add(arc.multiplicity)
                    .ok_or(GraphError::MultiplicityOverflow)?;
            } else {
                merged.push(arc);
            }
        }

        let total_multiplicity = merged.iter().try_fold(0_u64, |total, arc| {
            total
                .checked_add(arc.multiplicity)
                .ok_or(GraphError::MultiplicityOverflow)
        })?;
        let (outgoing_offsets, outgoing) = build_csr(vertex_count, &merged, false);
        let (incoming_offsets, incoming) = build_csr(vertex_count, &merged, true);

        Ok(IncidenceGraph {
            labels,
            descriptors,
            vertex_kinds,
            vertex_label_ids,
            outgoing_offsets,
            outgoing,
            incoming_offsets,
            incoming,
            total_multiplicity,
        })
    }

    fn validate_vertex(&self, vertex: VertexId) -> Result<(), GraphError> {
        if vertex.index() >= self.vertices.len() {
            return Err(GraphError::InvalidVertex {
                index: vertex.index(),
                vertex_count: self.vertices.len(),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NormalizedArc {
    source: VertexId,
    target: VertexId,
    relation: RelationId,
    multiplicity: u64,
}

fn build_csr(
    vertex_count: usize,
    arcs: &[NormalizedArc],
    incoming_direction: bool,
) -> (Vec<usize>, Vec<Incidence>) {
    let mut ordered = arcs.to_vec();
    if incoming_direction {
        ordered.sort_by_key(|arc| (arc.target, arc.source, arc.relation));
    }
    let mut offsets = vec![0_usize; vertex_count + 1];
    for arc in &ordered {
        let row = if incoming_direction {
            arc.target.index()
        } else {
            arc.source.index()
        };
        offsets[row + 1] += 1;
    }
    for index in 1..offsets.len() {
        offsets[index] += offsets[index - 1];
    }
    let incidences = ordered
        .into_iter()
        .map(|arc| Incidence {
            neighbor: if incoming_direction {
                arc.source
            } else {
                arc.target
            },
            relation: arc.relation,
            multiplicity: arc.multiplicity,
        })
        .collect();
    (offsets, incidences)
}

/// Immutable exact graph normalized for allocation-free refinement rounds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncidenceGraph {
    labels: Vec<Vec<u8>>,
    descriptors: Vec<RelationDescriptor>,
    vertex_kinds: Vec<VertexKind>,
    vertex_label_ids: Vec<usize>,
    outgoing_offsets: Vec<usize>,
    outgoing: Vec<Incidence>,
    incoming_offsets: Vec<usize>,
    incoming: Vec<Incidence>,
    total_multiplicity: u64,
}

impl IncidenceGraph {
    pub(super) fn with_vertex_label_updates(
        &self,
        updates: &[(usize, Vec<u8>)],
    ) -> Result<Self, GraphError> {
        use std::collections::{BTreeMap, BTreeSet};

        let mut vertex_labels = (0..self.vertex_count())
            .map(|index| self.vertex_label(VertexId::new(index)).to_vec())
            .collect::<Vec<_>>();
        for (index, label) in updates {
            if *index >= vertex_labels.len() {
                return Err(GraphError::InvalidVertex {
                    index: *index,
                    vertex_count: vertex_labels.len(),
                });
            }
            vertex_labels[*index].clone_from(label);
        }
        let labels = vertex_labels
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let ids = labels
            .iter()
            .cloned()
            .enumerate()
            .map(|(id, label)| (label, id))
            .collect::<BTreeMap<_, _>>();
        let vertex_label_ids = vertex_labels
            .iter()
            .map(|label| {
                ids.get(label)
                    .copied()
                    .ok_or(GraphError::CanonicalizationInvariantViolation)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            labels,
            descriptors: self.descriptors.clone(),
            vertex_kinds: self.vertex_kinds.clone(),
            vertex_label_ids,
            outgoing_offsets: self.outgoing_offsets.clone(),
            outgoing: self.outgoing.clone(),
            incoming_offsets: self.incoming_offsets.clone(),
            incoming: self.incoming.clone(),
            total_multiplicity: self.total_multiplicity,
        })
    }

    /// Number of entity and auxiliary hyperedge vertices.
    #[must_use]
    pub const fn vertex_count(&self) -> usize {
        self.vertex_kinds.len()
    }

    /// Number of normalized directed records after duplicate compression.
    #[must_use]
    pub const fn incidence_count(&self) -> usize {
        self.outgoing.len()
    }

    /// Sum of exact directed multiplicities.
    #[must_use]
    pub const fn total_multiplicity(&self) -> u64 {
        self.total_multiplicity
    }

    /// Returns whether an externally supplied identifier belongs to this graph.
    #[must_use]
    pub const fn contains_vertex(&self, vertex: VertexId) -> bool {
        vertex.index() < self.vertex_kinds.len()
    }

    /// Returns the semantic kind after validating an external identifier.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidVertex`] for an out-of-range identifier.
    pub fn try_vertex_kind(&self, vertex: VertexId) -> Result<VertexKind, GraphError> {
        self.validate_vertex(vertex)?;
        Ok(self.vertex_kinds[vertex.index()])
    }

    /// Returns the exact label after validating an external identifier.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidVertex`] for an out-of-range identifier.
    pub fn try_vertex_label(&self, vertex: VertexId) -> Result<&[u8], GraphError> {
        self.validate_vertex(vertex)?;
        Ok(&self.labels[self.vertex_label_ids[vertex.index()]])
    }

    /// Returns an outgoing CSR row after validating an external identifier.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidVertex`] for an out-of-range identifier.
    pub fn try_outgoing(&self, vertex: VertexId) -> Result<&[Incidence], GraphError> {
        self.validate_vertex(vertex)?;
        Ok(self.outgoing(vertex))
    }

    /// Returns an incoming CSR row after validating an external identifier.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidVertex`] for an out-of-range identifier.
    pub fn try_incoming(&self, vertex: VertexId) -> Result<&[Incidence], GraphError> {
        self.validate_vertex(vertex)?;
        Ok(self.incoming(vertex))
    }

    /// Returns whether a descriptor identifier belongs to this graph.
    #[must_use]
    pub const fn contains_relation(&self, relation: RelationId) -> bool {
        relation.index() < self.descriptors.len()
    }

    /// Returns a relation descriptor after validating its graph-local identifier.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidRelation`] for an out-of-range identifier.
    pub fn try_relation(&self, relation: RelationId) -> Result<&RelationDescriptor, GraphError> {
        if self.contains_relation(relation) {
            Ok(&self.descriptors[relation.index()])
        } else {
            Err(GraphError::InvalidRelation {
                index: relation.index(),
                relation_count: self.descriptors.len(),
            })
        }
    }

    /// Semantic kind of a vertex.
    #[must_use]
    pub fn vertex_kind(&self, vertex: VertexId) -> VertexKind {
        self.vertex_kinds[vertex.index()]
    }

    /// Exact application label of a vertex.
    #[must_use]
    pub fn vertex_label(&self, vertex: VertexId) -> &[u8] {
        &self.labels[self.vertex_label_ids[vertex.index()]]
    }

    /// Exact descriptor associated with one relation identifier.
    #[must_use]
    pub fn relation(&self, relation: RelationId) -> &RelationDescriptor {
        &self.descriptors[relation.index()]
    }

    /// Outgoing CSR row without allocation.
    #[must_use]
    pub fn outgoing(&self, vertex: VertexId) -> &[Incidence] {
        let index = vertex.index();
        &self.outgoing[self.outgoing_offsets[index]..self.outgoing_offsets[index + 1]]
    }

    /// Incoming CSR row without allocation.
    #[must_use]
    pub fn incoming(&self, vertex: VertexId) -> &[Incidence] {
        let index = vertex.index();
        &self.incoming[self.incoming_offsets[index]..self.incoming_offsets[index + 1]]
    }

    pub(crate) fn descriptors(&self) -> &[RelationDescriptor] {
        &self.descriptors
    }

    pub(crate) fn labels(&self) -> &[Vec<u8>] {
        &self.labels
    }

    pub(crate) fn vertex_label_id(&self, vertex: VertexId) -> usize {
        self.vertex_label_ids[vertex.index()]
    }

    fn validate_vertex(&self, vertex: VertexId) -> Result<(), GraphError> {
        if self.contains_vertex(vertex) {
            Ok(())
        } else {
            Err(GraphError::InvalidVertex {
                index: vertex.index(),
                vertex_count: self.vertex_count(),
            })
        }
    }
}
