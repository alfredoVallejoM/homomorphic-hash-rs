//! Transactional storage and bounded dependency indexes for local updates.

use microfield::Field;

use super::labeler::RoundAggregate;
use super::{
    FastGraphSignature, GraphError, GraphSignatureId, IncidenceGraph, StructuralLabel, VertexId,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct LabelUpdate<F: Field, const K: usize> {
    pub(super) offset: usize,
    pub(super) value: StructuralLabel<F, K>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct AggregateDelta<F: Field, const K: usize> {
    pub(super) removed_nonzero: [F; K],
    pub(super) added_nonzero: [F; K],
    pub(super) removed_zeros: [u64; K],
    pub(super) added_zeros: [u64; K],
}

impl<F: Field, const K: usize> AggregateDelta<F, K> {
    pub(super) const fn identity() -> Self {
        Self {
            removed_nonzero: [F::ONE; K],
            added_nonzero: [F::ONE; K],
            removed_zeros: [0; K],
            added_zeros: [0; K],
        }
    }
}

/// Undirected dependency closure of the directed incidence model.
///
/// Refining a vertex reads both its incoming and outgoing rows. Consequently,
/// a label change can affect either endpoint of every directed incidence. The
/// index stores each distinct dependency in both directions and omits loops,
/// because every propagation frontier already contains its seed vertex.
#[derive(Clone, Debug, Default)]
pub(super) struct GraphDependencyIndex {
    pub(super) offsets: Vec<usize>,
    pub(super) neighbors: Vec<usize>,
    pub(super) components: Vec<usize>,
    pub(super) component_count: usize,
}

impl GraphDependencyIndex {
    pub(super) fn neighbors(&self, vertex: usize) -> &[usize] {
        &self.neighbors[self.offsets[vertex]..self.offsets[vertex + 1]]
    }

    pub(super) fn reserve_for(
        &mut self,
        vertex_count: usize,
        incidence_count: usize,
    ) -> Result<(), GraphError> {
        let dependency_bound = incidence_count
            .checked_mul(2)
            .ok_or(GraphError::GraphTooLarge)?;
        reserve_total(&mut self.offsets, vertex_count.saturating_add(1));
        reserve_total(&mut self.neighbors, dependency_bound);
        reserve_total(&mut self.components, vertex_count);
        Ok(())
    }
}

/// Persistent fixed-round analysis that can be updated transactionally.
///
/// The state owns the current graph, every round of structural labels and the
/// exact graph signature. Keeping `R + 1` label layers is what makes a later
/// edit proportional to its propagation cone instead of requiring all earlier
/// rounds to be replayed globally.
#[derive(Clone, Debug)]
pub struct IncrementalGraphState<F: Field, const K: usize> {
    pub(super) graph: IncidenceGraph,
    pub(super) signature_id: GraphSignatureId,
    pub(super) rounds: usize,
    pub(super) round_labels: Vec<StructuralLabel<F, K>>,
    pub(super) order: Vec<usize>,
    pub(super) partition: Vec<usize>,
    pub(super) cell_count: usize,
    pub(super) signature: FastGraphSignature<F, K>,
    pub(super) dependencies: GraphDependencyIndex,
    pub(super) revision: u64,
}

impl<F: Field, const K: usize> IncrementalGraphState<F, K> {
    /// Current immutable normalized graph owned by this state.
    #[must_use]
    pub const fn graph(&self) -> &IncidenceGraph {
        &self.graph
    }

    /// Number of successfully published graph replacements.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Fixed number of local propagation rounds retained by the state.
    #[must_use]
    pub const fn rounds(&self) -> usize {
        self.rounds
    }

    /// Number of weakly connected components in the current incidence graph.
    #[must_use]
    pub const fn component_count(&self) -> usize {
        self.dependencies.component_count
    }

    /// Component containing `vertex`, or `None` for an invalid identifier.
    #[must_use]
    pub fn component_of(&self, vertex: VertexId) -> Option<usize> {
        self.dependencies.components.get(vertex.index()).copied()
    }

    /// Number of distinct directed records in the symmetric dependency index.
    #[must_use]
    pub const fn dependency_record_count(&self) -> usize {
        self.dependencies.neighbors.len()
    }

    pub(super) fn labels_at(&self, round: usize) -> &[StructuralLabel<F, K>] {
        let vertex_count = self.graph.vertex_count();
        let start = round * vertex_count;
        &self.round_labels[start..start + vertex_count]
    }
}

/// Reusable scratch storage for a transactional incremental update.
///
/// Call [`Self::reserve_for`] before a latency-sensitive update. Once sized for
/// the largest expected graph, its journals, overlays and staged dependency
/// index do not grow. Preparing the owned replacement graph still creates its
/// immutable encoded metadata and is outside this scratch-space guarantee.
#[derive(Clone, Debug)]
pub struct IncrementalGraphWorkspace<F: Field, const K: usize> {
    pub(super) updates: Vec<LabelUpdate<F, K>>,
    pub(super) staged_aggregates: Vec<RoundAggregate<F, K>>,
    pub(super) aggregate_deltas: Vec<AggregateDelta<F, K>>,
    pub(super) previous_values: Vec<StructuralLabel<F, K>>,
    pub(super) current_values: Vec<StructuralLabel<F, K>>,
    pub(super) previous_marks: Vec<bool>,
    pub(super) current_marks: Vec<bool>,
    pub(super) previous_changed: Vec<usize>,
    pub(super) current_changed: Vec<usize>,
    pub(super) initial_changed: Vec<usize>,
    pub(super) topology_changed: Vec<usize>,
    pub(super) affected: Vec<usize>,
    pub(super) affected_marks: Vec<u32>,
    pub(super) affected_epoch: u32,
    pub(super) final_labels: Vec<StructuralLabel<F, K>>,
    pub(super) order: Vec<usize>,
    pub(super) merged_order: Vec<usize>,
    pub(super) partition: Vec<usize>,
    pub(super) component_stack: Vec<usize>,
    pub(super) staged_dependencies: GraphDependencyIndex,
}

impl<F: Field, const K: usize> IncrementalGraphWorkspace<F, K> {
    /// Creates empty scratch storage.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            updates: Vec::new(),
            staged_aggregates: Vec::new(),
            aggregate_deltas: Vec::new(),
            previous_values: Vec::new(),
            current_values: Vec::new(),
            previous_marks: Vec::new(),
            current_marks: Vec::new(),
            previous_changed: Vec::new(),
            current_changed: Vec::new(),
            initial_changed: Vec::new(),
            topology_changed: Vec::new(),
            affected: Vec::new(),
            affected_marks: Vec::new(),
            affected_epoch: 0,
            final_labels: Vec::new(),
            order: Vec::new(),
            merged_order: Vec::new(),
            partition: Vec::new(),
            component_stack: Vec::new(),
            staged_dependencies: GraphDependencyIndex {
                offsets: Vec::new(),
                neighbors: Vec::new(),
                components: Vec::new(),
                component_count: 0,
            },
        }
    }

    /// Reserves the strict worst case for a graph and its fixed round count.
    ///
    /// # Errors
    ///
    /// Rejects size products that cannot be represented by `usize`.
    pub fn reserve_for(
        &mut self,
        vertex_count: usize,
        incidence_count: usize,
        rounds: usize,
    ) -> Result<(), GraphError> {
        let layers = rounds.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
        let update_bound = vertex_count
            .checked_mul(layers)
            .ok_or(GraphError::GraphTooLarge)?;
        self.reserve_baseline(vertex_count, incidence_count, rounds)?;
        reserve_total(&mut self.updates, update_bound);
        Ok(())
    }

    pub(super) fn reserve_baseline(
        &mut self,
        vertex_count: usize,
        incidence_count: usize,
        rounds: usize,
    ) -> Result<(), GraphError> {
        let layers = rounds.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
        reserve_total(&mut self.staged_aggregates, layers);
        reserve_total(&mut self.aggregate_deltas, layers);
        for values in [&mut self.previous_values, &mut self.current_values] {
            reserve_total(values, vertex_count);
        }
        for marks in [&mut self.previous_marks, &mut self.current_marks] {
            reserve_total(marks, vertex_count);
        }
        for indices in [
            &mut self.previous_changed,
            &mut self.current_changed,
            &mut self.initial_changed,
            &mut self.topology_changed,
            &mut self.affected,
            &mut self.order,
            &mut self.merged_order,
            &mut self.partition,
            &mut self.component_stack,
        ] {
            reserve_total(indices, vertex_count);
        }
        reserve_total(&mut self.affected_marks, vertex_count);
        reserve_total(&mut self.final_labels, vertex_count);
        self.staged_dependencies
            .reserve_for(vertex_count, incidence_count)?;
        Ok(())
    }

    pub(super) fn reset_vertex_storage(&mut self, vertex_count: usize) {
        let zero = StructuralLabel {
            lanes: [F::ZERO; K],
        };
        self.previous_values.resize(vertex_count, zero);
        self.current_values.resize(vertex_count, zero);
        self.previous_marks.resize(vertex_count, false);
        self.previous_marks.fill(false);
        self.current_marks.resize(vertex_count, false);
        self.current_marks.fill(false);
        self.affected_marks.resize(vertex_count, 0);
        self.affected_marks.fill(0);
        self.affected_epoch = 0;
        self.updates.clear();
        self.aggregate_deltas.clear();
        self.previous_changed.clear();
        self.current_changed.clear();
        self.initial_changed.clear();
        self.topology_changed.clear();
        self.affected.clear();
    }

    pub(super) fn begin_frontier(&mut self) {
        self.affected.clear();
        self.affected_epoch = self.affected_epoch.wrapping_add(1);
        if self.affected_epoch == 0 {
            self.affected_marks.fill(0);
            self.affected_epoch = 1;
        }
    }

    pub(super) fn include_affected(&mut self, index: usize) {
        if self.affected_marks[index] != self.affected_epoch {
            self.affected_marks[index] = self.affected_epoch;
            self.affected.push(index);
        }
    }
}

impl<F: Field, const K: usize> Default for IncrementalGraphWorkspace<F, K> {
    fn default() -> Self {
        Self::new()
    }
}

/// Exact work and component transition produced by one published update.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IncrementalUpdateStats {
    pub(super) audited_vertices: usize,
    pub(super) audited_incidence_records: usize,
    pub(super) initial_seed_vertices: usize,
    pub(super) topology_seed_vertices: usize,
    pub(super) recomputed_vertex_rounds: usize,
    pub(super) changed_vertex_rounds: usize,
    pub(super) final_changed_vertices: usize,
    pub(super) peak_frontier_vertices: usize,
    pub(super) previous_component_count: usize,
    pub(super) component_count: usize,
    pub(super) dependency_records: usize,
    pub(super) revision: u64,
}

impl IncrementalUpdateStats {
    /// Vertices whose exact input rows were audited before publication.
    #[must_use]
    pub const fn audited_vertices(self) -> usize {
        self.audited_vertices
    }

    /// Old plus new normalized directed records covered by the audit.
    #[must_use]
    pub const fn audited_incidence_records(self) -> usize {
        self.audited_incidence_records
    }

    /// Vertices whose round-zero field label actually changed.
    #[must_use]
    pub const fn initial_seed_vertices(self) -> usize {
        self.initial_seed_vertices
    }

    /// Vertices with a changed incoming or outgoing semantic CSR row.
    #[must_use]
    pub const fn topology_seed_vertices(self) -> usize {
        self.topology_seed_vertices
    }

    /// Candidate vertex/round cells evaluated inside the propagation cone.
    #[must_use]
    pub const fn recomputed_vertex_rounds(self) -> usize {
        self.recomputed_vertex_rounds
    }

    /// Recomputed cells whose field label differed from the retained value.
    #[must_use]
    pub const fn changed_vertex_rounds(self) -> usize {
        self.changed_vertex_rounds
    }

    /// Number of final-round labels changed by this update.
    #[must_use]
    pub const fn final_changed_vertices(self) -> usize {
        self.final_changed_vertices
    }

    /// Largest propagation frontier evaluated in any one round.
    #[must_use]
    pub const fn peak_frontier_vertices(self) -> usize {
        self.peak_frontier_vertices
    }

    /// Weak component count before publication.
    #[must_use]
    pub const fn previous_component_count(self) -> usize {
        self.previous_component_count
    }

    /// Weak component count after publication.
    #[must_use]
    pub const fn component_count(self) -> usize {
        self.component_count
    }

    /// Number of bounded dependency records after publication.
    #[must_use]
    pub const fn dependency_records(self) -> usize {
        self.dependency_records
    }

    /// State revision after publication.
    #[must_use]
    pub const fn revision(self) -> u64 {
        self.revision
    }
}

fn reserve_total<T>(values: &mut Vec<T>, total: usize) {
    if values.capacity() < total {
        values.reserve_exact(total - values.len());
    }
}
