//! G10 compact exact refinement and certified search optimizations.

use core::{cmp::Ordering, mem::size_of};
use std::time::Instant;

use super::super::{
    global::GlobalTopology, GraphError, GraphSchemaId, IncidenceGraph, IncidenceGraphBuilder,
    RelationDescriptor, VertexId,
};
use super::encoding::{canonical_form_from_order, CanonicalGraphForm};
use super::mapping::VerifiedGraphMapping;
use super::search::{
    CanonicalBudgetLimit, CanonicalSearchBudget, CanonicalizationRun, MicrocanonOutcome,
    MicrocanonPath, MicrocanonReport,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RefinementEntry {
    descriptor_size: usize,
    neighbor_color: usize,
    relation_rank: usize,
    multiplicity: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct VertexKey {
    color: usize,
    outgoing_start: usize,
    outgoing_len: usize,
    incoming_start: usize,
    incoming_len: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct RefinementTrace {
    events: u64,
    checksum: u64,
}

impl RefinementTrace {
    fn record(
        &mut self,
        cell_count: usize,
        largest: usize,
        ambiguous: usize,
    ) -> Result<(), GraphError> {
        self.events = self
            .events
            .checked_add(1)
            .ok_or(GraphError::GraphTooLarge)?;
        self.checksum = mix_trace(self.checksum, cell_count)?;
        self.checksum = mix_trace(self.checksum, largest)?;
        self.checksum = mix_trace(self.checksum, ambiguous)?;
        Ok(())
    }
}

fn mix_trace(state: u64, value: usize) -> Result<u64, GraphError> {
    let value = u64::try_from(value).map_err(|_| GraphError::GraphTooLarge)?;
    Ok(state.wrapping_mul(0x9e37_79b1_85eb_ca87).rotate_left(17) ^ value)
}

#[derive(Debug, Default)]
struct CompactRefiner {
    entries: Vec<RefinementEntry>,
    keys: Vec<VertexKey>,
    order: Vec<usize>,
    next_colors: Vec<usize>,
    cell_sizes: Vec<usize>,
    label_ranks: Vec<usize>,
    relation_ranks: Vec<usize>,
    relation_sizes: Vec<usize>,
}

/// Reusable O(V+I) storage for G10 compact refinement.
#[derive(Debug, Default)]
pub struct MicrocanonWorkspace {
    refiner: CompactRefiner,
}

impl MicrocanonWorkspace {
    /// Creates an empty workspace that grows on first use.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            refiner: CompactRefiner {
                entries: Vec::new(),
                keys: Vec::new(),
                order: Vec::new(),
                next_colors: Vec::new(),
                cell_sizes: Vec::new(),
                label_ranks: Vec::new(),
                relation_ranks: Vec::new(),
                relation_sizes: Vec::new(),
            },
        }
    }

    /// Reserves all buffers whose size is proportional to graph incidences.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::GraphTooLarge`] on arithmetic or allocation failure.
    pub fn reserve_for(
        &mut self,
        vertex_count: usize,
        incidence_count: usize,
    ) -> Result<(), GraphError> {
        self.refiner.reserve_for(vertex_count, incidence_count)
    }

    /// Retained capacity of controlled compact-refinement buffers.
    pub fn retained_bytes(&self) -> Result<usize, GraphError> {
        self.refiner.retained_bytes()
    }
}

impl CompactRefiner {
    fn prepare(graph: &IncidenceGraph) -> Result<Self, GraphError> {
        let mut refiner = Self::default();
        refiner.reserve_for(graph.vertex_count(), graph.incidence_count())?;
        refiner.prepare_interned_ranks(graph)?;
        Ok(refiner)
    }

    fn reserve_for(
        &mut self,
        vertex_count: usize,
        incidence_count: usize,
    ) -> Result<(), GraphError> {
        let entry_count = incidence_count
            .checked_mul(2)
            .ok_or(GraphError::GraphTooLarge)?;
        reserve_exact_capacity(&mut self.entries, entry_count)?;
        reserve_exact_capacity(&mut self.keys, vertex_count)?;
        reserve_exact_capacity(&mut self.order, vertex_count)?;
        reserve_exact_capacity(&mut self.next_colors, vertex_count)?;
        reserve_exact_capacity(&mut self.cell_sizes, vertex_count)?;
        reserve_exact_capacity(&mut self.label_ranks, vertex_count)?;
        reserve_exact_capacity(&mut self.relation_ranks, incidence_count)?;
        reserve_exact_capacity(&mut self.relation_sizes, incidence_count)
    }

    fn prepare_interned_ranks(&mut self, graph: &IncidenceGraph) -> Result<(), GraphError> {
        self.order.clear();
        self.order.extend(0..graph.labels().len());
        self.order.sort_unstable_by(|left, right| {
            compare_framed(
                graph.labels()[*left].as_slice(),
                graph.labels()[*right].as_slice(),
            )
            .then_with(|| left.cmp(right))
        });
        self.label_ranks.clear();
        self.label_ranks.resize(graph.labels().len(), 0);
        for (rank, &label) in self.order.iter().enumerate() {
            self.label_ranks[label] = rank;
        }

        self.order.clear();
        self.order.extend(0..graph.descriptors().len());
        self.order.sort_unstable_by(|left, right| {
            compare_descriptors(&graph.descriptors()[*left], &graph.descriptors()[*right])
                .then_with(|| left.cmp(right))
        });
        self.relation_ranks.clear();
        self.relation_ranks.resize(graph.descriptors().len(), 0);
        self.relation_sizes.clear();
        self.relation_sizes.resize(graph.descriptors().len(), 0);
        for (rank, &relation) in self.order.iter().enumerate() {
            let descriptor = &graph.descriptors()[relation];
            self.relation_ranks[relation] = rank;
            self.relation_sizes[relation] = descriptor
                .relation()
                .len()
                .checked_add(descriptor.role().len())
                .ok_or(GraphError::GraphTooLarge)?;
        }
        Ok(())
    }

    fn initial_partition(&mut self, graph: &IncidenceGraph) -> Result<Vec<usize>, GraphError> {
        self.order.clear();
        self.order.extend(0..graph.vertex_count());
        let label_ranks = &self.label_ranks;
        self.order.sort_unstable_by(|left, right| {
            compare_initial_vertices(graph, label_ranks, *left, *right)
                .then_with(|| left.cmp(right))
        });
        let mut colors = vec![0; graph.vertex_count()];
        let mut color = 0_usize;
        for (position, &vertex) in self.order.iter().enumerate() {
            if position != 0
                && compare_initial_vertices(
                    graph,
                    &self.label_ranks,
                    self.order[position - 1],
                    vertex,
                ) != Ordering::Equal
            {
                color = color.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
            }
            colors[vertex] = color;
        }
        Ok(colors)
    }

    fn refine(
        &mut self,
        graph: &IncidenceGraph,
        mut colors: Vec<usize>,
        control: &SearchControl,
        enforce_deadline: bool,
    ) -> Result<RefinementResult, GraphError> {
        let mut passes = 0_u64;
        let mut trace = RefinementTrace::default();
        loop {
            if enforce_deadline && control.deadline_exceeded() {
                return Ok(RefinementResult {
                    colors,
                    passes,
                    trace,
                    deadline_exhausted: true,
                });
            }
            passes = passes.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
            self.build_keys(graph, &colors)?;
            self.assign_colors()?;
            let (cell_count, largest, ambiguous) = self.partition_summary()?;
            trace.record(cell_count, largest, ambiguous)?;
            if self.next_colors == colors {
                return Ok(RefinementResult {
                    colors,
                    passes,
                    trace,
                    deadline_exhausted: false,
                });
            }
            core::mem::swap(&mut colors, &mut self.next_colors);
        }
    }

    fn build_keys(&mut self, graph: &IncidenceGraph, colors: &[usize]) -> Result<(), GraphError> {
        self.entries.clear();
        self.keys.clear();
        for index in 0..graph.vertex_count() {
            let vertex = VertexId::new(index);
            let outgoing_start = self.entries.len();
            for incidence in graph.outgoing(vertex) {
                self.entries.push(RefinementEntry {
                    descriptor_size: self.relation_sizes[incidence.relation().index()],
                    neighbor_color: colors[incidence.neighbor().index()],
                    relation_rank: self.relation_ranks[incidence.relation().index()],
                    multiplicity: incidence.multiplicity(),
                });
            }
            let outgoing_len = self.entries.len() - outgoing_start;
            self.entries[outgoing_start..].sort_unstable_by(compare_entry_content);

            let incoming_start = self.entries.len();
            for incidence in graph.incoming(vertex) {
                self.entries.push(RefinementEntry {
                    descriptor_size: self.relation_sizes[incidence.relation().index()],
                    neighbor_color: colors[incidence.neighbor().index()],
                    relation_rank: self.relation_ranks[incidence.relation().index()],
                    multiplicity: incidence.multiplicity(),
                });
            }
            let incoming_len = self.entries.len() - incoming_start;
            self.entries[incoming_start..].sort_unstable_by(compare_entry_content);
            self.keys.push(VertexKey {
                color: colors[index],
                outgoing_start,
                outgoing_len,
                incoming_start,
                incoming_len,
            });
        }
        Ok(())
    }

    fn assign_colors(&mut self) -> Result<(), GraphError> {
        self.order.clear();
        self.order.extend(0..self.keys.len());
        let keys = &self.keys;
        let entries = &self.entries;
        self.order.sort_unstable_by(|left, right| {
            compare_vertex_keys(entries, &keys[*left], &keys[*right]).then_with(|| left.cmp(right))
        });
        self.next_colors.clear();
        self.next_colors.resize(self.keys.len(), 0);
        let mut color = 0_usize;
        for (position, &vertex) in self.order.iter().enumerate() {
            if position != 0
                && compare_vertex_keys(
                    &self.entries,
                    &self.keys[self.order[position - 1]],
                    &self.keys[vertex],
                ) != Ordering::Equal
            {
                color = color.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
            }
            self.next_colors[vertex] = color;
        }
        Ok(())
    }

    fn partition_summary(&mut self) -> Result<(usize, usize, usize), GraphError> {
        let cell_count = partition_cell_count(&self.next_colors);
        self.cell_sizes.clear();
        self.cell_sizes.resize(cell_count, 0);
        for &color in &self.next_colors {
            self.cell_sizes[color] = self.cell_sizes[color]
                .checked_add(1)
                .ok_or(GraphError::GraphTooLarge)?;
        }
        let largest = self.cell_sizes.iter().copied().max().unwrap_or(0);
        let ambiguous = self
            .cell_sizes
            .iter()
            .copied()
            .filter(|size| *size > 1)
            .sum();
        Ok((cell_count, largest, ambiguous))
    }

    fn retained_bytes(&self) -> Result<usize, GraphError> {
        bytes_for_capacity::<RefinementEntry>(self.entries.capacity())?
            .checked_add(bytes_for_capacity::<VertexKey>(self.keys.capacity())?)
            .and_then(|bytes| {
                bytes.checked_add(bytes_for_capacity::<usize>(self.order.capacity()).ok()?)
            })
            .and_then(|bytes| {
                bytes.checked_add(bytes_for_capacity::<usize>(self.next_colors.capacity()).ok()?)
            })
            .and_then(|bytes| {
                bytes.checked_add(bytes_for_capacity::<usize>(self.cell_sizes.capacity()).ok()?)
            })
            .and_then(|bytes| {
                bytes.checked_add(bytes_for_capacity::<usize>(self.label_ranks.capacity()).ok()?)
            })
            .and_then(|bytes| {
                bytes.checked_add(bytes_for_capacity::<usize>(self.relation_ranks.capacity()).ok()?)
            })
            .and_then(|bytes| {
                bytes.checked_add(bytes_for_capacity::<usize>(self.relation_sizes.capacity()).ok()?)
            })
            .ok_or(GraphError::GraphTooLarge)
    }
}

#[derive(Debug)]
struct RefinementResult {
    colors: Vec<usize>,
    passes: u64,
    trace: RefinementTrace,
    deadline_exhausted: bool,
}

fn compare_initial_vertices(
    graph: &IncidenceGraph,
    label_ranks: &[usize],
    left: usize,
    right: usize,
) -> Ordering {
    let left = VertexId::new(left);
    let right = VertexId::new(right);
    graph
        .vertex_kind(left)
        .cmp(&graph.vertex_kind(right))
        .then_with(|| {
            label_ranks[graph.vertex_label_id(left)].cmp(&label_ranks[graph.vertex_label_id(right)])
        })
}

fn compare_framed(left: &[u8], right: &[u8]) -> Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn compare_descriptors(left: &RelationDescriptor, right: &RelationDescriptor) -> Ordering {
    compare_framed(left.relation(), right.relation())
        .then_with(|| compare_framed(left.role(), right.role()))
}

fn compare_entry_content(left: &RefinementEntry, right: &RefinementEntry) -> Ordering {
    left.neighbor_color
        .cmp(&right.neighbor_color)
        .then_with(|| left.relation_rank.cmp(&right.relation_rank))
        .then_with(|| left.multiplicity.cmp(&right.multiplicity))
}

fn compare_framed_entries(left: &RefinementEntry, right: &RefinementEntry) -> Ordering {
    // G9 sorts raw entry contents and only then frames each sorted entry in the
    // vertex key. Cross-key comparison observes that frame length first.
    left.descriptor_size
        .cmp(&right.descriptor_size)
        .then_with(|| compare_entry_content(left, right))
}

fn compare_vertex_keys(
    entries: &[RefinementEntry],
    left: &VertexKey,
    right: &VertexKey,
) -> Ordering {
    left.color
        .cmp(&right.color)
        .then_with(|| left.outgoing_len.cmp(&right.outgoing_len))
        .then_with(|| {
            compare_entry_slices(
                &entries[left.outgoing_start..left.outgoing_start + left.outgoing_len],
                &entries[right.outgoing_start..right.outgoing_start + right.outgoing_len],
            )
        })
        .then_with(|| left.incoming_len.cmp(&right.incoming_len))
        .then_with(|| {
            compare_entry_slices(
                &entries[left.incoming_start..left.incoming_start + left.incoming_len],
                &entries[right.incoming_start..right.incoming_start + right.incoming_len],
            )
        })
}

fn compare_entry_slices(left: &[RefinementEntry], right: &[RefinementEntry]) -> Ordering {
    for (left, right) in left.iter().zip(right) {
        let ordering = compare_framed_entries(left, right);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

#[derive(Debug)]
struct SearchControl {
    budget: CanonicalSearchBudget,
    started: Instant,
}

impl SearchControl {
    fn new(budget: CanonicalSearchBudget) -> Self {
        Self {
            budget,
            started: Instant::now(),
        }
    }

    fn deadline_exceeded(&self) -> bool {
        self.budget
            .max_elapsed()
            .is_some_and(|limit| self.started.elapsed() >= limit)
    }

    fn elapsed(&self) -> std::time::Duration {
        self.started.elapsed()
    }
}

pub(super) fn canonicalize(
    graph: &IncidenceGraph,
    schema_id: GraphSchemaId,
    budget: CanonicalSearchBudget,
) -> Result<CanonicalizationRun, GraphError> {
    let mut workspace = MicrocanonWorkspace::new();
    canonicalize_with_workspace(graph, schema_id, budget, &mut workspace)
}

pub(super) fn canonicalize_with_workspace(
    graph: &IncidenceGraph,
    schema_id: GraphSchemaId,
    budget: CanonicalSearchBudget,
    workspace: &mut MicrocanonWorkspace,
) -> Result<CanonicalizationRun, GraphError> {
    let control = SearchControl::new(budget);
    workspace.reserve_for(graph.vertex_count(), graph.incidence_count())?;
    let refiner = &mut workspace.refiner;
    refiner.prepare_interned_ranks(graph)?;
    let initial = refiner.initial_partition(graph)?;
    let refined = refiner.refine(graph, initial, &control, false)?;
    let root_partition = refined.colors.clone();
    let mut outcome = canonicalize_from_root(graph, schema_id, refined, refiner, &control)?;
    outcome.report_mut().elapsed = control.elapsed();
    Ok(CanonicalizationRun {
        outcome,
        root_partition,
    })
}

fn canonicalize_core(
    graph: &IncidenceGraph,
    schema_id: GraphSchemaId,
    control: &SearchControl,
    node_budget: u64,
) -> Result<MicrocanonOutcome, GraphError> {
    let mut refiner = CompactRefiner::prepare(graph)?;
    let initial = refiner.initial_partition(graph)?;
    let refined = refiner.refine(graph, initial, control, true)?;
    let mut local_budget = CanonicalSearchBudget::new(node_budget)
        .with_max_retained_state_cells(control.budget.max_retained_state_cells())
        .with_max_retained_bytes(control.budget.max_retained_bytes())
        .with_max_depth(control.budget.max_depth());
    if let Some(elapsed) = control.budget.max_elapsed() {
        local_budget = local_budget.with_max_elapsed(elapsed);
    }
    let local_control = SearchControl {
        budget: local_budget,
        started: control.started,
    };
    canonicalize_from_root(graph, schema_id, refined, &mut refiner, &local_control)
}

fn canonicalize_from_root(
    graph: &IncidenceGraph,
    schema_id: GraphSchemaId,
    refined: RefinementResult,
    refiner: &mut CompactRefiner,
    control: &SearchControl,
) -> Result<MicrocanonOutcome, GraphError> {
    let root_partition = refined.colors;
    let cell_count = partition_cell_count(&root_partition);
    let sizes = cell_sizes(&root_partition, cell_count);
    let root_bytes = refiner
        .retained_bytes()?
        .checked_add(bytes_for_capacity::<usize>(root_partition.capacity())?)
        .ok_or(GraphError::GraphTooLarge)?;
    let mut report = MicrocanonReport {
        path: MicrocanonPath::ExactRefinementDiscrete,
        root_cell_count: cell_count,
        root_largest_cell: sizes.iter().copied().max().unwrap_or(0),
        root_ambiguous_vertices: sizes.into_iter().filter(|size| *size > 1).sum(),
        explored_nodes: 0,
        leaf_count: usize::from(cell_count == graph.vertex_count()) as u64,
        individualization_count: 0,
        exact_refinement_passes: refined.passes,
        maximum_depth: 0,
        peak_retained_state_cells: root_partition.len(),
        peak_tracked_bytes: root_bytes,
        trace_event_count: refined.trace.events,
        target_cell_count: 0,
        verified_automorphism_count: 0,
        orbit_pruned_child_count: 0,
        prefix_pruned_leaf_count: 0,
        elapsed: control.elapsed(),
        exhausted_limit: None,
    };
    if refined.deadline_exhausted || control.deadline_exceeded() {
        return Ok(inconclusive(
            report,
            CanonicalBudgetLimit::ElapsedTime,
            control,
        ));
    }
    if report.peak_tracked_bytes > control.budget.max_retained_bytes() {
        return Ok(inconclusive(
            report,
            CanonicalBudgetLimit::RetainedBytes,
            control,
        ));
    }
    if cell_count == graph.vertex_count() {
        let order = order_from_colors(&root_partition);
        let projected = root_bytes
            .checked_add(canonical_form_retained_size(graph)?)
            .ok_or(GraphError::GraphTooLarge)?;
        report.peak_tracked_bytes = report.peak_tracked_bytes.max(projected);
        if projected > control.budget.max_retained_bytes() {
            return Ok(inconclusive(
                report,
                CanonicalBudgetLimit::RetainedBytes,
                control,
            ));
        }
        let form = canonical_form_from_order(graph, order, schema_id)?;
        report.elapsed = control.elapsed();
        return Ok(MicrocanonOutcome::Exact { form, report });
    }

    let topology = GlobalTopology::build(graph)?;
    if topology.weak_component_count > 1 {
        let retained_base_bytes = refiner.retained_bytes()?;
        drop(root_partition);
        return canonicalize_by_weak_components(
            graph,
            schema_id,
            &topology.weak_components,
            topology.weak_component_count,
            report,
            control,
            retained_base_bytes,
        );
    }
    exact_search(graph, schema_id, root_partition, report, refiner, control)
}

fn canonicalize_by_weak_components(
    graph: &IncidenceGraph,
    schema_id: GraphSchemaId,
    components: &[usize],
    component_count: usize,
    mut report: MicrocanonReport,
    control: &SearchControl,
    retained_base_bytes: usize,
) -> Result<MicrocanonOutcome, GraphError> {
    report.path = MicrocanonPath::WeakComponentDecomposition;
    report.leaf_count = 0;
    let mut exact_components = Vec::with_capacity(component_count);
    for component in 0..component_count {
        let retained_components = component_storage_bytes(
            &exact_components,
            exact_components.capacity(),
            retained_base_bytes,
        )?;
        report.peak_tracked_bytes = report.peak_tracked_bytes.max(retained_components);
        if retained_components > control.budget.max_retained_bytes() {
            return Ok(inconclusive(
                report,
                CanonicalBudgetLimit::RetainedBytes,
                control,
            ));
        }
        if control.deadline_exceeded() {
            return Ok(inconclusive(
                report,
                CanonicalBudgetLimit::ElapsedTime,
                control,
            ));
        }
        let (subgraph, local_to_original) = extract_weak_component(graph, components, component)?;
        let remaining = control
            .budget
            .max_search_nodes()
            .saturating_sub(report.explored_nodes);
        match canonicalize_core(&subgraph, schema_id, control, remaining)? {
            MicrocanonOutcome::Exact {
                form,
                report: local,
            } => {
                let combined_peak = retained_components
                    .checked_add(local.peak_tracked_bytes())
                    .ok_or(GraphError::GraphTooLarge)?;
                accumulate_report(&mut report, &local)?;
                report.peak_tracked_bytes = report.peak_tracked_bytes.max(combined_peak);
                if combined_peak > control.budget.max_retained_bytes() {
                    return Ok(inconclusive(
                        report,
                        CanonicalBudgetLimit::RetainedBytes,
                        control,
                    ));
                }
                let order = form
                    .canonical_to_original()
                    .iter()
                    .map(|vertex| local_to_original[vertex.index()])
                    .collect::<Vec<_>>();
                exact_components.push((form.bytes().to_vec(), order));
                let retained_components = component_storage_bytes(
                    &exact_components,
                    exact_components.capacity(),
                    retained_base_bytes,
                )?;
                report.peak_tracked_bytes = report.peak_tracked_bytes.max(retained_components);
                if retained_components > control.budget.max_retained_bytes() {
                    return Ok(inconclusive(
                        report,
                        CanonicalBudgetLimit::RetainedBytes,
                        control,
                    ));
                }
            }
            MicrocanonOutcome::Inconclusive { report: local } => {
                accumulate_report(&mut report, &local)?;
                report.exhausted_limit = local.exhausted_limit;
                report.elapsed = control.elapsed();
                return Ok(MicrocanonOutcome::Inconclusive { report });
            }
        }
    }
    exact_components.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    let final_peak = component_storage_bytes(
        &exact_components,
        exact_components.capacity(),
        retained_base_bytes,
    )?
    .checked_add(canonical_form_retained_size(graph)?)
    .ok_or(GraphError::GraphTooLarge)?;
    report.peak_tracked_bytes = report.peak_tracked_bytes.max(final_peak);
    if final_peak > control.budget.max_retained_bytes() {
        return Ok(inconclusive(
            report,
            CanonicalBudgetLimit::RetainedBytes,
            control,
        ));
    }
    let order = exact_components
        .into_iter()
        .flat_map(|(_, order)| order)
        .collect();
    let form = canonical_form_from_order(graph, order, schema_id)?;
    report.elapsed = control.elapsed();
    Ok(MicrocanonOutcome::Exact { form, report })
}

fn extract_weak_component(
    graph: &IncidenceGraph,
    components: &[usize],
    selected: usize,
) -> Result<(IncidenceGraph, Vec<VertexId>), GraphError> {
    let mut builder = IncidenceGraphBuilder::new();
    let mut original_to_local = vec![usize::MAX; graph.vertex_count()];
    let mut local_to_original = Vec::new();
    for (index, &component) in components.iter().enumerate() {
        if component != selected {
            continue;
        }
        let original = VertexId::new(index);
        let local = builder.add_typed_vertex(
            graph.vertex_kind(original),
            graph.vertex_label(original).to_vec(),
        );
        original_to_local[index] = local.index();
        local_to_original.push(original);
    }
    for (source, &component) in components.iter().enumerate() {
        if component != selected {
            continue;
        }
        for incidence in graph.outgoing(VertexId::new(source)) {
            let target = incidence.neighbor().index();
            if components[target] != selected {
                return Err(GraphError::CanonicalizationInvariantViolation);
            }
            let descriptor = graph.relation(incidence.relation());
            builder.add_directed_relation(
                VertexId::new(original_to_local[source]),
                VertexId::new(original_to_local[target]),
                descriptor.relation().to_vec(),
                descriptor.role().to_vec(),
                incidence.multiplicity(),
            )?;
        }
    }
    Ok((builder.build()?, local_to_original))
}

#[derive(Debug)]
struct PendingNode {
    colors: Vec<usize>,
    fixed: Vec<usize>,
    depth: usize,
    already_refined: bool,
}

#[derive(Debug)]
struct BranchFrame {
    colors: Vec<usize>,
    candidates: Vec<usize>,
    fixed: Vec<usize>,
    next_candidate: usize,
    cell_count: usize,
    depth: usize,
}

fn exact_search(
    graph: &IncidenceGraph,
    schema_id: GraphSchemaId,
    root_colors: Vec<usize>,
    mut report: MicrocanonReport,
    refiner: &mut CompactRefiner,
    control: &SearchControl,
) -> Result<MicrocanonOutcome, GraphError> {
    report.path = MicrocanonPath::IndividualizationRefinement;
    report.leaf_count = 0;
    let mut pending = Some(PendingNode {
        colors: root_colors,
        fixed: Vec::new(),
        depth: 0,
        already_refined: true,
    });
    let mut stack = Vec::<BranchFrame>::new();
    let mut best: Option<CanonicalGraphForm> = None;
    let mut automorphisms = Vec::<Vec<usize>>::new();
    let mut orbit_parent = Vec::<usize>::new();

    loop {
        if let Some(node) = pending.take() {
            if control.deadline_exceeded() {
                return Ok(inconclusive(
                    report,
                    CanonicalBudgetLimit::ElapsedTime,
                    control,
                ));
            }
            if report.explored_nodes >= control.budget.max_search_nodes() {
                return Ok(inconclusive(
                    report,
                    CanonicalBudgetLimit::SearchNodes,
                    control,
                ));
            }
            report.explored_nodes = report
                .explored_nodes
                .checked_add(1)
                .ok_or(GraphError::GraphTooLarge)?;
            report.maximum_depth = report.maximum_depth.max(node.depth);
            let fixed = node.fixed;
            let colors = if node.already_refined {
                node.colors
            } else {
                let refined = refiner.refine(graph, node.colors, control, true)?;
                report.exact_refinement_passes = report
                    .exact_refinement_passes
                    .checked_add(refined.passes)
                    .ok_or(GraphError::GraphTooLarge)?;
                report.trace_event_count = report
                    .trace_event_count
                    .checked_add(refined.trace.events)
                    .ok_or(GraphError::GraphTooLarge)?;
                if refined.deadline_exhausted {
                    return Ok(inconclusive(
                        report,
                        CanonicalBudgetLimit::ElapsedTime,
                        control,
                    ));
                }
                refined.colors
            };
            let cell_count = partition_cell_count(&colors);
            if cell_count == graph.vertex_count() {
                report.leaf_count = report
                    .leaf_count
                    .checked_add(1)
                    .ok_or(GraphError::GraphTooLarge)?;
                let order = order_from_colors(&colors);
                if best.as_ref().is_some_and(|current| {
                    compare_vertex_sequences(graph, &order, current.canonical_to_original())
                        == Ordering::Greater
                }) {
                    report.prefix_pruned_leaf_count = report
                        .prefix_pruned_leaf_count
                        .checked_add(1)
                        .ok_or(GraphError::GraphTooLarge)?;
                } else {
                    let candidate = canonical_form_from_order(graph, order, schema_id)?;
                    match best
                        .as_ref()
                        .map(|current| candidate.bytes().cmp(current.bytes()))
                    {
                        None | Some(Ordering::Less) => best = Some(candidate),
                        Some(Ordering::Equal) => {
                            let current = best
                                .as_ref()
                                .ok_or(GraphError::CanonicalizationInvariantViolation)?;
                            if let Some(automorphism) =
                                verified_automorphism(graph, current, &candidate)?
                            {
                                if !automorphisms.contains(&automorphism) {
                                    automorphisms.push(automorphism);
                                    report.verified_automorphism_count = report
                                        .verified_automorphism_count
                                        .checked_add(1)
                                        .ok_or(GraphError::GraphTooLarge)?;
                                }
                            }
                        }
                        Some(Ordering::Greater) => {}
                    }
                }
            } else {
                let candidates = select_target_cell(graph, &colors, cell_count)?;
                report.target_cell_count = report
                    .target_cell_count
                    .checked_add(1)
                    .ok_or(GraphError::GraphTooLarge)?;
                stack.push(BranchFrame {
                    colors,
                    candidates,
                    fixed,
                    next_candidate: 0,
                    cell_count,
                    depth: node.depth,
                });
            }
            if let Some(limit) = update_peaks(
                &mut report,
                refiner,
                SearchMemoryView {
                    stack: &stack,
                    pending: pending.as_ref(),
                    automorphisms: &automorphisms,
                    orbit_parent: &orbit_parent,
                    best: best.as_ref(),
                },
                control.budget,
            )? {
                return Ok(inconclusive(report, limit, control));
            }
        }

        let mut scheduled = false;
        while let Some(frame) = stack.last_mut() {
            if frame.next_candidate < frame.candidates.len() {
                let candidate = frame.candidates[frame.next_candidate];
                frame.next_candidate += 1;
                if orbit_redundant(
                    candidate,
                    &frame.candidates[..frame.next_candidate - 1],
                    &frame.fixed,
                    &automorphisms,
                    &mut orbit_parent,
                ) {
                    report.orbit_pruned_child_count = report
                        .orbit_pruned_child_count
                        .checked_add(1)
                        .ok_or(GraphError::GraphTooLarge)?;
                    continue;
                }
                let child_depth = frame
                    .depth
                    .checked_add(1)
                    .ok_or(GraphError::GraphTooLarge)?;
                if child_depth > control.budget.max_depth() {
                    return Ok(inconclusive(
                        report,
                        CanonicalBudgetLimit::SearchDepth,
                        control,
                    ));
                }
                let mut child_colors = frame.colors.clone();
                child_colors[candidate] = frame.cell_count;
                let mut fixed = frame.fixed.clone();
                fixed.push(candidate);
                pending = Some(PendingNode {
                    colors: child_colors,
                    fixed,
                    depth: child_depth,
                    already_refined: false,
                });
                report.individualization_count = report
                    .individualization_count
                    .checked_add(1)
                    .ok_or(GraphError::GraphTooLarge)?;
                scheduled = true;
                break;
            }
            stack.pop();
        }

        if let Some(limit) = update_peaks(
            &mut report,
            refiner,
            SearchMemoryView {
                stack: &stack,
                pending: pending.as_ref(),
                automorphisms: &automorphisms,
                orbit_parent: &orbit_parent,
                best: best.as_ref(),
            },
            control.budget,
        )? {
            return Ok(inconclusive(report, limit, control));
        }
        if !scheduled && stack.is_empty() {
            let form = best.ok_or(GraphError::CanonicalizationInvariantViolation)?;
            report.elapsed = control.elapsed();
            return Ok(MicrocanonOutcome::Exact { form, report });
        }
    }
}

fn select_target_cell(
    _graph: &IncidenceGraph,
    colors: &[usize],
    cell_count: usize,
) -> Result<Vec<usize>, GraphError> {
    let sizes = cell_sizes(colors, cell_count);
    let (selected_cell, expected_size) = sizes
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, size)| *size > 1)
        .min_by_key(|(cell, size)| (*size, *cell))
        .ok_or(GraphError::CanonicalizationInvariantViolation)?;
    let candidates = colors
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(vertex, cell)| (cell == selected_cell).then_some(vertex))
        .collect::<Vec<_>>();
    if candidates.len() != expected_size {
        return Err(GraphError::CanonicalizationInvariantViolation);
    }
    Ok(candidates)
}

fn verified_automorphism(
    graph: &IncidenceGraph,
    best: &CanonicalGraphForm,
    candidate: &CanonicalGraphForm,
) -> Result<Option<Vec<usize>>, GraphError> {
    let mut mapping = vec![VertexId::new(0); graph.vertex_count()];
    for canonical in 0..graph.vertex_count() {
        let source = best.canonical_to_original()[canonical];
        let target = candidate.canonical_to_original()[canonical];
        mapping[source.index()] = target;
    }
    if mapping
        .iter()
        .copied()
        .enumerate()
        .all(|(source, target)| source == target.index())
    {
        return Ok(None);
    }
    let verified = VerifiedGraphMapping::verify(graph, graph, &mapping)?;
    Ok(Some(
        verified
            .left_to_right()
            .iter()
            .map(|vertex| vertex.index())
            .collect(),
    ))
}

fn orbit_redundant(
    candidate: usize,
    previous: &[usize],
    fixed: &[usize],
    automorphisms: &[Vec<usize>],
    parent: &mut Vec<usize>,
) -> bool {
    if previous.is_empty() || automorphisms.is_empty() {
        return false;
    }
    parent.clear();
    parent.extend(0..automorphisms[0].len());
    for automorphism in automorphisms {
        if fixed
            .iter()
            .copied()
            .all(|vertex| automorphism[vertex] == vertex)
        {
            for (vertex, &image) in automorphism.iter().enumerate() {
                union(parent, vertex, image);
            }
        }
    }
    let candidate_root = find(parent, candidate);
    previous
        .iter()
        .copied()
        .any(|vertex| find(parent, vertex) == candidate_root)
}

fn find(parent: &mut [usize], mut vertex: usize) -> usize {
    while parent[vertex] != vertex {
        parent[vertex] = parent[parent[vertex]];
        vertex = parent[vertex];
    }
    vertex
}

fn union(parent: &mut [usize], left: usize, right: usize) {
    let left = find(parent, left);
    let right = find(parent, right);
    if left != right {
        parent[right] = left;
    }
}

fn compare_vertex_sequences(
    graph: &IncidenceGraph,
    left: &[VertexId],
    right: &[VertexId],
) -> Ordering {
    for (&left, &right) in left.iter().zip(right) {
        let ordering = graph
            .vertex_kind(left)
            .cmp(&graph.vertex_kind(right))
            .then_with(|| {
                graph
                    .vertex_label(left)
                    .len()
                    .cmp(&graph.vertex_label(right).len())
            })
            .then_with(|| graph.vertex_label(left).cmp(graph.vertex_label(right)));
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

fn order_from_colors(colors: &[usize]) -> Vec<VertexId> {
    let mut order = (0..colors.len()).map(VertexId::new).collect::<Vec<_>>();
    order.sort_unstable_by_key(|vertex| colors[vertex.index()]);
    order
}

struct SearchMemoryView<'a> {
    stack: &'a [BranchFrame],
    pending: Option<&'a PendingNode>,
    automorphisms: &'a [Vec<usize>],
    orbit_parent: &'a [usize],
    best: Option<&'a CanonicalGraphForm>,
}

fn update_peaks(
    report: &mut MicrocanonReport,
    refiner: &CompactRefiner,
    memory: SearchMemoryView<'_>,
    budget: CanonicalSearchBudget,
) -> Result<Option<CanonicalBudgetLimit>, GraphError> {
    let mut cells = 0_usize;
    let mut bytes = refiner.retained_bytes()?;
    for frame in memory.stack {
        cells = cells
            .checked_add(frame.colors.len())
            .and_then(|value| value.checked_add(frame.candidates.len()))
            .and_then(|value| value.checked_add(frame.fixed.len()))
            .ok_or(GraphError::GraphTooLarge)?;
        bytes = bytes
            .checked_add(bytes_for_capacity::<usize>(frame.colors.capacity())?)
            .and_then(|value| {
                value.checked_add(bytes_for_capacity::<usize>(frame.candidates.capacity()).ok()?)
            })
            .and_then(|value| {
                value.checked_add(bytes_for_capacity::<usize>(frame.fixed.capacity()).ok()?)
            })
            .ok_or(GraphError::GraphTooLarge)?;
    }
    if let Some(node) = memory.pending {
        cells = cells
            .checked_add(node.colors.len())
            .and_then(|value| value.checked_add(node.fixed.len()))
            .ok_or(GraphError::GraphTooLarge)?;
        bytes = bytes
            .checked_add(bytes_for_capacity::<usize>(node.colors.capacity())?)
            .and_then(|value| {
                value.checked_add(bytes_for_capacity::<usize>(node.fixed.capacity()).ok()?)
            })
            .ok_or(GraphError::GraphTooLarge)?;
    }
    for automorphism in memory.automorphisms {
        cells = cells
            .checked_add(automorphism.len())
            .ok_or(GraphError::GraphTooLarge)?;
        bytes = bytes
            .checked_add(bytes_for_capacity::<usize>(automorphism.capacity())?)
            .ok_or(GraphError::GraphTooLarge)?;
    }
    cells = cells
        .checked_add(memory.orbit_parent.len())
        .ok_or(GraphError::GraphTooLarge)?;
    bytes = bytes
        .checked_add(bytes_for_capacity::<usize>(memory.orbit_parent.len())?)
        .ok_or(GraphError::GraphTooLarge)?;
    if let Some(best) = memory.best {
        cells = cells
            .checked_add(best.original_to_canonical().len())
            .and_then(|value| value.checked_add(best.canonical_to_original().len()))
            .ok_or(GraphError::GraphTooLarge)?;
        bytes = bytes
            .checked_add(best.bytes().len())
            .and_then(|value| {
                value.checked_add(
                    best.original_to_canonical()
                        .len()
                        .checked_mul(size_of::<VertexId>())?,
                )
            })
            .and_then(|value| {
                value.checked_add(
                    best.canonical_to_original()
                        .len()
                        .checked_mul(size_of::<VertexId>())?,
                )
            })
            .ok_or(GraphError::GraphTooLarge)?;
    }
    report.peak_retained_state_cells = report.peak_retained_state_cells.max(cells);
    report.peak_tracked_bytes = report.peak_tracked_bytes.max(bytes);
    if cells > budget.max_retained_state_cells() {
        Ok(Some(CanonicalBudgetLimit::RetainedStateCells))
    } else if bytes > budget.max_retained_bytes() {
        Ok(Some(CanonicalBudgetLimit::RetainedBytes))
    } else {
        Ok(None)
    }
}

fn bytes_for_capacity<T>(capacity: usize) -> Result<usize, GraphError> {
    capacity
        .checked_mul(size_of::<T>())
        .ok_or(GraphError::GraphTooLarge)
}

fn component_storage_bytes(
    components: &[(Vec<u8>, Vec<VertexId>)],
    outer_capacity: usize,
    retained_base_bytes: usize,
) -> Result<usize, GraphError> {
    let mut bytes = retained_base_bytes
        .checked_add(bytes_for_capacity::<(Vec<u8>, Vec<VertexId>)>(
            outer_capacity,
        )?)
        .ok_or(GraphError::GraphTooLarge)?;
    for (form, order) in components {
        bytes = bytes
            .checked_add(bytes_for_capacity::<u8>(form.capacity())?)
            .and_then(|value| {
                value.checked_add(bytes_for_capacity::<VertexId>(order.capacity()).ok()?)
            })
            .ok_or(GraphError::GraphTooLarge)?;
    }
    Ok(bytes)
}

fn reserve_exact_capacity<T>(buffer: &mut Vec<T>, required: usize) -> Result<(), GraphError> {
    buffer.clear();
    if buffer.capacity() < required {
        buffer
            // `try_reserve_exact` is relative to `len`, not to `capacity`.
            // The buffer is empty here, so requesting `required - capacity`
            // could be a no-op and allow a later, unbudgeted growth.
            .try_reserve_exact(required)
            .map_err(|_| GraphError::GraphTooLarge)?;
    }
    Ok(())
}

fn canonical_form_size(graph: &IncidenceGraph) -> Result<usize, GraphError> {
    let mut bytes = 64_usize;
    for index in 0..graph.vertex_count() {
        bytes = bytes
            .checked_add(1 + 8)
            .and_then(|value| value.checked_add(graph.vertex_label(VertexId::new(index)).len()))
            .ok_or(GraphError::GraphTooLarge)?;
    }
    for source in 0..graph.vertex_count() {
        for incidence in graph.outgoing(VertexId::new(source)) {
            let descriptor = graph.relation(incidence.relation());
            bytes = bytes
                .checked_add(40)
                .and_then(|value| value.checked_add(descriptor.relation().len()))
                .and_then(|value| value.checked_add(descriptor.role().len()))
                .ok_or(GraphError::GraphTooLarge)?;
        }
    }
    Ok(bytes)
}

fn canonical_form_retained_size(graph: &IncidenceGraph) -> Result<usize, GraphError> {
    canonical_form_size(graph)?
        .checked_add(
            graph
                .vertex_count()
                .checked_mul(2)
                .and_then(|vertices| vertices.checked_mul(size_of::<VertexId>()))
                .ok_or(GraphError::GraphTooLarge)?,
        )
        .ok_or(GraphError::GraphTooLarge)
}

fn inconclusive(
    mut report: MicrocanonReport,
    limit: CanonicalBudgetLimit,
    control: &SearchControl,
) -> MicrocanonOutcome {
    report.exhausted_limit = Some(limit);
    report.elapsed = control.elapsed();
    MicrocanonOutcome::Inconclusive { report }
}

fn accumulate_report(
    total: &mut MicrocanonReport,
    local: &MicrocanonReport,
) -> Result<(), GraphError> {
    total.explored_nodes = checked_add(total.explored_nodes, local.explored_nodes)?;
    total.leaf_count = checked_add(total.leaf_count, local.leaf_count)?;
    total.individualization_count =
        checked_add(total.individualization_count, local.individualization_count)?;
    total.exact_refinement_passes =
        checked_add(total.exact_refinement_passes, local.exact_refinement_passes)?;
    total.trace_event_count = checked_add(total.trace_event_count, local.trace_event_count)?;
    total.target_cell_count = checked_add(total.target_cell_count, local.target_cell_count)?;
    total.verified_automorphism_count = checked_add(
        total.verified_automorphism_count,
        local.verified_automorphism_count,
    )?;
    total.orbit_pruned_child_count = checked_add(
        total.orbit_pruned_child_count,
        local.orbit_pruned_child_count,
    )?;
    total.prefix_pruned_leaf_count = checked_add(
        total.prefix_pruned_leaf_count,
        local.prefix_pruned_leaf_count,
    )?;
    total.maximum_depth = total.maximum_depth.max(local.maximum_depth);
    total.peak_retained_state_cells = total
        .peak_retained_state_cells
        .max(local.peak_retained_state_cells);
    total.peak_tracked_bytes = total.peak_tracked_bytes.max(local.peak_tracked_bytes);
    Ok(())
}

fn checked_add(left: u64, right: u64) -> Result<u64, GraphError> {
    left.checked_add(right).ok_or(GraphError::GraphTooLarge)
}

fn partition_cell_count(partition: &[usize]) -> usize {
    partition.iter().copied().max().map_or(0, |cell| cell + 1)
}

fn cell_sizes(partition: &[usize], cell_count: usize) -> Vec<usize> {
    let mut sizes = vec![0; cell_count];
    for &cell in partition {
        sizes[cell] += 1;
    }
    sizes
}
