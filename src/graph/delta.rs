//! Transactional, endpoint-validated edits for persistent graph analysis.

use std::collections::{BTreeMap, BTreeSet};

use super::{GraphError, IncidenceGraph, IncidenceGraphBuilder, VertexId};

const MAXIMUM_DELTA_OPERATIONS: usize = 1_000_000;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ArcKey {
    source: usize,
    target: usize,
    relation: Vec<u8>,
    role: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum GraphEdit {
    SetVertexLabel { vertex: VertexId, label: Vec<u8> },
    AddRelation { key: ArcKey, multiplicity: u64 },
    RemoveRelation { key: ArcKey, multiplicity: u64 },
}

/// Atomic edit transaction over a stable vertex-index domain.
///
/// Vertex insertion/removal deliberately requires rebuilding the persistent
/// state because it invalidates every retained layer index. Labels, directed
/// relation records and exact multiplicities can be edited here.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GraphDelta {
    expected_revision: Option<u64>,
    edits: Vec<GraphEdit>,
}

impl GraphDelta {
    /// Creates an empty transaction.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            expected_revision: None,
            edits: Vec::new(),
        }
    }

    /// Requires the persistent state to still have this revision.
    #[must_use]
    pub const fn with_expected_revision(mut self, revision: u64) -> Self {
        self.expected_revision = Some(revision);
        self
    }

    /// Expected revision, if optimistic concurrency control is enabled.
    #[must_use]
    pub const fn expected_revision(&self) -> Option<u64> {
        self.expected_revision
    }

    /// Number of commands in this transaction.
    #[must_use]
    pub const fn operation_count(&self) -> usize {
        self.edits.len()
    }

    /// Replaces one exact vertex label.
    ///
    /// # Errors
    ///
    /// Rejects duplicate label commands and the hard transaction-size limit.
    pub fn set_vertex_label(
        &mut self,
        vertex: VertexId,
        label: impl Into<Vec<u8>>,
    ) -> Result<&mut Self, GraphError> {
        self.reserve_command()?;
        if self.edits.iter().any(|edit| {
            matches!(edit, GraphEdit::SetVertexLabel { vertex: current, .. } if *current == vertex)
        }) {
            return Err(GraphError::ConflictingGraphDelta);
        }
        self.edits.push(GraphEdit::SetVertexLabel {
            vertex,
            label: label.into(),
        });
        Ok(self)
    }

    /// Adds multiplicity to one exact directed relation.
    ///
    /// # Errors
    ///
    /// Rejects zero multiplicity and the hard transaction-size limit.
    pub fn add_directed_relation(
        &mut self,
        source: VertexId,
        target: VertexId,
        relation: impl Into<Vec<u8>>,
        role: impl Into<Vec<u8>>,
        multiplicity: u64,
    ) -> Result<&mut Self, GraphError> {
        if multiplicity == 0 {
            return Err(GraphError::ZeroMultiplicity);
        }
        self.reserve_command()?;
        self.edits.push(GraphEdit::AddRelation {
            key: ArcKey {
                source: source.index(),
                target: target.index(),
                relation: relation.into(),
                role: role.into(),
            },
            multiplicity,
        });
        Ok(self)
    }

    /// Removes exactly `multiplicity` from one normalized relation.
    ///
    /// # Errors
    ///
    /// Rejects zero multiplicity and the hard transaction-size limit. Presence
    /// and sufficient retained multiplicity are checked atomically on apply.
    pub fn remove_directed_relation(
        &mut self,
        source: VertexId,
        target: VertexId,
        relation: impl Into<Vec<u8>>,
        role: impl Into<Vec<u8>>,
        multiplicity: u64,
    ) -> Result<&mut Self, GraphError> {
        if multiplicity == 0 {
            return Err(GraphError::ZeroMultiplicity);
        }
        self.reserve_command()?;
        self.edits.push(GraphEdit::RemoveRelation {
            key: ArcKey {
                source: source.index(),
                target: target.index(),
                relation: relation.into(),
                role: role.into(),
            },
            multiplicity,
        });
        Ok(self)
    }

    pub(crate) fn apply(&self, graph: &IncidenceGraph) -> Result<AppliedGraphDelta, GraphError> {
        if self
            .edits
            .iter()
            .all(|edit| matches!(edit, GraphEdit::SetVertexLabel { .. }))
        {
            let mut touched = BTreeSet::new();
            let mut updates = Vec::with_capacity(self.edits.len());
            let mut label_changed = false;
            for edit in &self.edits {
                let GraphEdit::SetVertexLabel { vertex, label } = edit else {
                    unreachable!("label-only transaction was preflighted")
                };
                validate_vertex(graph, *vertex)?;
                touched.insert(vertex.index());
                if graph.vertex_label(*vertex) != label {
                    label_changed = true;
                }
                updates.push((vertex.index(), label.clone()));
            }
            let updated = if label_changed {
                graph.with_vertex_label_updates(&updates)?
            } else {
                graph.clone()
            };
            return Ok(AppliedGraphDelta {
                graph: updated,
                touched_vertices: touched.into_iter().collect(),
                label_changed,
                topology_changed: false,
            });
        }
        let mut labels = (0..graph.vertex_count())
            .map(|index| graph.vertex_label(VertexId::new(index)).to_vec())
            .collect::<Vec<_>>();
        let mut arcs = BTreeMap::<ArcKey, u64>::new();
        for source in 0..graph.vertex_count() {
            for incidence in graph.outgoing(VertexId::new(source)) {
                let descriptor = graph.relation(incidence.relation());
                arcs.insert(
                    ArcKey {
                        source,
                        target: incidence.neighbor().index(),
                        relation: descriptor.relation().to_vec(),
                        role: descriptor.role().to_vec(),
                    },
                    incidence.multiplicity(),
                );
            }
        }
        let mut touched = BTreeSet::new();
        let mut label_changed = false;
        let mut topology_changed = false;
        for edit in &self.edits {
            match edit {
                GraphEdit::SetVertexLabel { vertex, label } => {
                    validate_vertex(graph, *vertex)?;
                    touched.insert(vertex.index());
                    if labels[vertex.index()] != *label {
                        labels[vertex.index()] = label.clone();
                        label_changed = true;
                    }
                }
                GraphEdit::AddRelation { key, multiplicity } => {
                    validate_arc_key(graph, key)?;
                    touched.insert(key.source);
                    touched.insert(key.target);
                    let current = arcs.entry(key.clone()).or_default();
                    *current = current
                        .checked_add(*multiplicity)
                        .ok_or(GraphError::MultiplicityOverflow)?;
                    topology_changed = true;
                }
                GraphEdit::RemoveRelation { key, multiplicity } => {
                    validate_arc_key(graph, key)?;
                    touched.insert(key.source);
                    touched.insert(key.target);
                    let current = arcs
                        .get_mut(key)
                        .ok_or(GraphError::GraphDeltaRelationAbsent)?;
                    *current = current
                        .checked_sub(*multiplicity)
                        .ok_or(GraphError::GraphDeltaMultiplicityUnderflow)?;
                    if *current == 0 {
                        arcs.remove(key);
                    }
                    topology_changed = true;
                }
            }
        }

        let mut builder = IncidenceGraphBuilder::new();
        for (index, label) in labels.into_iter().enumerate() {
            builder.add_typed_vertex(graph.vertex_kind(VertexId::new(index)), label);
        }
        for (key, multiplicity) in arcs {
            builder.add_directed_relation(
                VertexId::new(key.source),
                VertexId::new(key.target),
                key.relation,
                key.role,
                multiplicity,
            )?;
        }
        Ok(AppliedGraphDelta {
            graph: builder.build()?,
            touched_vertices: touched.into_iter().collect(),
            label_changed,
            topology_changed,
        })
    }

    fn reserve_command(&self) -> Result<(), GraphError> {
        if self.edits.len() >= MAXIMUM_DELTA_OPERATIONS {
            Err(GraphError::GraphDeltaTooLarge)
        } else {
            Ok(())
        }
    }
}

pub(crate) struct AppliedGraphDelta {
    pub(crate) graph: IncidenceGraph,
    pub(crate) touched_vertices: Vec<usize>,
    pub(crate) label_changed: bool,
    pub(crate) topology_changed: bool,
}

fn validate_vertex(graph: &IncidenceGraph, vertex: VertexId) -> Result<(), GraphError> {
    if graph.contains_vertex(vertex) {
        Ok(())
    } else {
        Err(GraphError::InvalidVertex {
            index: vertex.index(),
            vertex_count: graph.vertex_count(),
        })
    }
}

fn validate_arc_key(graph: &IncidenceGraph, key: &ArcKey) -> Result<(), GraphError> {
    validate_vertex(graph, VertexId::new(key.source))?;
    validate_vertex(graph, VertexId::new(key.target))
}

/// Adaptive route selected for one transactional delta.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GraphDeltaUpdatePath {
    /// Commands normalized to the already retained graph.
    NoChange,
    /// Only the certified dependency cone was recomputed.
    IncrementalCone,
    /// The estimated cone was too large and complete rebuilding was selected.
    FullRebuild,
}

/// Conservative admission policy for G14 incremental execution.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GraphDeltaPolicy {
    maximum_incremental_operations: usize,
    maximum_cone_per_mille: u16,
}

impl GraphDeltaPolicy {
    /// Creates a policy. `maximum_cone_per_mille` must be in `1..=1000`.
    ///
    /// # Errors
    ///
    /// Rejects zero command limits and invalid ratios.
    pub fn new(
        maximum_incremental_operations: usize,
        maximum_cone_per_mille: u16,
    ) -> Result<Self, GraphError> {
        if maximum_incremental_operations == 0 || !(1..=1000).contains(&maximum_cone_per_mille) {
            return Err(GraphError::InvalidGraphDeltaPolicy);
        }
        Ok(Self {
            maximum_incremental_operations,
            maximum_cone_per_mille,
        })
    }

    /// Maximum commands admitted to the local path.
    #[must_use]
    pub const fn maximum_incremental_operations(self) -> usize {
        self.maximum_incremental_operations
    }

    /// Maximum estimated fraction of all vertex-round cells, in per mille.
    #[must_use]
    pub const fn maximum_cone_per_mille(self) -> u16 {
        self.maximum_cone_per_mille
    }
}

impl Default for GraphDeltaPolicy {
    fn default() -> Self {
        Self {
            maximum_incremental_operations: 64,
            maximum_cone_per_mille: 350,
        }
    }
}

/// Selective cache invalidation caused by a delta.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GraphChannelInvalidation {
    /// Local labels and degree profiles changed.
    pub local: bool,
    /// Component/SCC/global descriptors changed.
    pub global: bool,
    /// Pattern, matrix and walk channels changed.
    pub higher_order: bool,
}

/// Auditable result of applying a G14 transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphDeltaUpdateReport {
    pub(crate) path: GraphDeltaUpdatePath,
    pub(crate) operation_count: usize,
    pub(crate) touched_vertices: usize,
    pub(crate) estimated_vertex_rounds: usize,
    pub(crate) invalidation: GraphChannelInvalidation,
    pub(crate) label_changed: bool,
    pub(crate) topology_changed: bool,
    pub(crate) incremental: Option<super::IncrementalUpdateStats>,
    pub(crate) revision: u64,
}

impl GraphDeltaUpdateReport {
    /// Execution route chosen before field recomputation.
    #[must_use]
    pub const fn path(self) -> GraphDeltaUpdatePath {
        self.path
    }
    /// Commands in the published transaction.
    #[must_use]
    pub const fn operation_count(self) -> usize {
        self.operation_count
    }
    /// Distinct validated endpoints.
    #[must_use]
    pub const fn touched_vertices(self) -> usize {
        self.touched_vertices
    }
    /// Conservative dependency-cone estimate.
    #[must_use]
    pub const fn estimated_vertex_rounds(self) -> usize {
        self.estimated_vertex_rounds
    }
    /// Cache families that callers must discard.
    #[must_use]
    pub const fn invalidation(self) -> GraphChannelInvalidation {
        self.invalidation
    }
    /// Whether at least one exact vertex label changed.
    #[must_use]
    pub const fn label_changed(self) -> bool {
        self.label_changed
    }
    /// Whether a relation, endpoint or multiplicity changed.
    #[must_use]
    pub const fn topology_changed(self) -> bool {
        self.topology_changed
    }
    /// Detailed cone statistics, only for the incremental route.
    #[must_use]
    pub const fn incremental(self) -> Option<super::IncrementalUpdateStats> {
        self.incremental
    }
    /// Published state revision.
    #[must_use]
    pub const fn revision(self) -> u64 {
        self.revision
    }
}
