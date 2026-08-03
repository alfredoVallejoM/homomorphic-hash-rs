//! Field-independent exact individualization/refinement baseline.

use std::time::Duration;

use super::super::{
    global::GlobalTopology, GraphError, GraphSchemaId, Incidence, IncidenceGraph,
    IncidenceGraphBuilder, VertexId,
};
use super::encoding::{canonical_form_from_order, CanonicalGraphForm};

const DEFAULT_RETAINED_STATE_CELLS: usize = 16 * 1024 * 1024;
const DEFAULT_RETAINED_BYTES: usize = 256 * 1024 * 1024;

/// Explicit hard limits for exact individualization/refinement search.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CanonicalSearchBudget {
    max_search_nodes: u64,
    max_retained_state_cells: usize,
    max_retained_bytes: usize,
    max_depth: usize,
    max_elapsed: Option<Duration>,
}

impl CanonicalSearchBudget {
    /// Creates a node-bounded search with a conservative frontier limit.
    #[must_use]
    pub const fn new(max_search_nodes: u64) -> Self {
        Self {
            max_search_nodes,
            max_retained_state_cells: DEFAULT_RETAINED_STATE_CELLS,
            max_retained_bytes: DEFAULT_RETAINED_BYTES,
            max_depth: usize::MAX,
            max_elapsed: None,
        }
    }

    /// Bounds retained `usize` cells in the DFS frontier.
    #[must_use]
    pub const fn with_max_retained_state_cells(mut self, cells: usize) -> Self {
        self.max_retained_state_cells = cells;
        self
    }

    /// Bounds the tracked retained working set of the exact engine.
    #[must_use]
    pub const fn with_max_retained_bytes(mut self, bytes: usize) -> Self {
        self.max_retained_bytes = bytes;
        self
    }

    /// Bounds the number of nested individualizations.
    #[must_use]
    pub const fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Adds a cooperative wall-clock deadline.
    ///
    /// Exact root preparation is atomic; the deadline is checked after it and
    /// at every subsequent refinement pass and search node.
    #[must_use]
    pub const fn with_max_elapsed(mut self, elapsed: Duration) -> Self {
        self.max_elapsed = Some(elapsed);
        self
    }

    /// Maximum individualization/refinement nodes.
    #[must_use]
    pub const fn max_search_nodes(self) -> u64 {
        self.max_search_nodes
    }

    /// Maximum retained frontier cells.
    #[must_use]
    pub const fn max_retained_state_cells(self) -> usize {
        self.max_retained_state_cells
    }

    /// Maximum tracked retained working-set bytes.
    #[must_use]
    pub const fn max_retained_bytes(self) -> usize {
        self.max_retained_bytes
    }

    /// Maximum individualization depth.
    #[must_use]
    pub const fn max_depth(self) -> usize {
        self.max_depth
    }

    /// Cooperative wall-clock deadline, if configured.
    #[must_use]
    pub const fn max_elapsed(self) -> Option<Duration> {
        self.max_elapsed
    }
}

/// Limit that stopped an exact search before certification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CanonicalBudgetLimit {
    /// The maximum number of search nodes was reached.
    SearchNodes,
    /// Retaining another DFS state would exceed the frontier limit.
    RetainedStateCells,
    /// The tracked working set would exceed its byte budget.
    RetainedBytes,
    /// Another individualization would exceed the depth budget.
    SearchDepth,
    /// The cooperative wall-clock deadline elapsed.
    ElapsedTime,
}

/// Exact route used by the field-independent canonizer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MicrocanonPath {
    /// Exact relational refinement already produced singleton cells.
    ExactRefinementDiscrete,
    /// Weak components were canonized independently and sorted by exact form.
    WeakComponentDecomposition,
    /// Exhaustive individualization/refinement was required.
    IndividualizationRefinement,
}

/// Auditable counters from one exact request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MicrocanonReport {
    pub(super) path: MicrocanonPath,
    pub(super) root_cell_count: usize,
    pub(super) root_largest_cell: usize,
    pub(super) root_ambiguous_vertices: usize,
    pub(super) explored_nodes: u64,
    pub(super) leaf_count: u64,
    pub(super) individualization_count: u64,
    pub(super) exact_refinement_passes: u64,
    pub(super) maximum_depth: usize,
    pub(super) peak_retained_state_cells: usize,
    pub(super) peak_tracked_bytes: usize,
    pub(super) trace_event_count: u64,
    pub(super) target_cell_count: u64,
    pub(super) verified_automorphism_count: u64,
    pub(super) orbit_pruned_child_count: u64,
    pub(super) prefix_pruned_leaf_count: u64,
    pub(super) elapsed: Duration,
    pub(super) exhausted_limit: Option<CanonicalBudgetLimit>,
}

impl MicrocanonReport {
    /// Route selected by the exact core.
    #[must_use]
    pub const fn path(&self) -> MicrocanonPath {
        self.path
    }

    /// Stable exact cells at the root.
    #[must_use]
    pub const fn root_cell_count(&self) -> usize {
        self.root_cell_count
    }

    /// Largest exact root cell.
    #[must_use]
    pub const fn root_largest_cell(&self) -> usize {
        self.root_largest_cell
    }

    /// Vertices retained in non-singleton root cells.
    #[must_use]
    pub const fn root_ambiguous_vertices(&self) -> usize {
        self.root_ambiguous_vertices
    }

    /// Exact search nodes entered.
    #[must_use]
    pub const fn explored_nodes(&self) -> u64 {
        self.explored_nodes
    }

    /// Complete discrete leaves evaluated.
    #[must_use]
    pub const fn leaf_count(&self) -> u64 {
        self.leaf_count
    }

    /// Child individualizations scheduled.
    #[must_use]
    pub const fn individualization_count(&self) -> u64 {
        self.individualization_count
    }

    /// Exact refinement passes, including stability checks.
    #[must_use]
    pub const fn exact_refinement_passes(&self) -> u64 {
        self.exact_refinement_passes
    }

    /// Deepest individualization level reached.
    #[must_use]
    pub const fn maximum_depth(&self) -> usize {
        self.maximum_depth
    }

    /// Peak frontier size measured in retained `usize` cells.
    #[must_use]
    pub const fn peak_retained_state_cells(&self) -> usize {
        self.peak_retained_state_cells
    }

    /// Peak logical bytes retained by controlled G10 buffers.
    #[must_use]
    pub const fn peak_tracked_bytes(&self) -> usize {
        self.peak_tracked_bytes
    }

    /// Number of partition events retained in refinement traces.
    #[must_use]
    pub const fn trace_event_count(&self) -> u64 {
        self.trace_event_count
    }

    /// Number of exact target-cell decisions.
    #[must_use]
    pub const fn target_cell_count(&self) -> u64 {
        self.target_cell_count
    }

    /// Number of non-identity automorphisms independently verified.
    #[must_use]
    pub const fn verified_automorphism_count(&self) -> u64 {
        self.verified_automorphism_count
    }

    /// Child branches skipped through verified stabilizer orbits.
    #[must_use]
    pub const fn orbit_pruned_child_count(&self) -> u64 {
        self.orbit_pruned_child_count
    }

    /// Discrete leaves rejected by a provable vertex-prefix bound.
    #[must_use]
    pub const fn prefix_pruned_leaf_count(&self) -> u64 {
        self.prefix_pruned_leaf_count
    }

    /// Wall-clock time observed by the exact engine.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Limit responsible for an incomplete result, if any.
    #[must_use]
    pub const fn exhausted_limit(&self) -> Option<CanonicalBudgetLimit> {
        self.exhausted_limit
    }
}

/// Exact canonical form or an explicit incomplete result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MicrocanonOutcome {
    /// The complete search certified the returned form.
    Exact {
        /// Profile-independent exact form.
        form: CanonicalGraphForm,
        /// Complete work report.
        report: MicrocanonReport,
    },
    /// No form is published because a hard limit stopped the search.
    Inconclusive {
        /// Partial-work report and exhausted limit.
        report: MicrocanonReport,
    },
}

impl MicrocanonOutcome {
    /// Borrows the report regardless of completion.
    #[must_use]
    pub const fn report(&self) -> &MicrocanonReport {
        match self {
            Self::Exact { report, .. } | Self::Inconclusive { report } => report,
        }
    }

    pub(super) fn report_mut(&mut self) -> &mut MicrocanonReport {
        match self {
            Self::Exact { report, .. } | Self::Inconclusive { report } => report,
        }
    }
}

pub(crate) struct CanonicalizationRun {
    pub(crate) outcome: MicrocanonOutcome,
    pub(crate) root_partition: Vec<usize>,
}

pub(crate) fn canonicalize(
    graph: &IncidenceGraph,
    schema_id: GraphSchemaId,
    budget: CanonicalSearchBudget,
) -> Result<CanonicalizationRun, GraphError> {
    let (root_partition, root_passes) = exact_stable_partition(graph)?;
    let outcome = canonicalize_from_root(
        graph,
        schema_id,
        root_partition.clone(),
        root_passes,
        budget,
    )?;
    Ok(CanonicalizationRun {
        outcome,
        root_partition,
    })
}

fn canonicalize_core(
    graph: &IncidenceGraph,
    schema_id: GraphSchemaId,
    budget: CanonicalSearchBudget,
) -> Result<MicrocanonOutcome, GraphError> {
    let (root_partition, root_passes) = exact_stable_partition(graph)?;
    canonicalize_from_root(graph, schema_id, root_partition, root_passes, budget)
}

fn canonicalize_from_root(
    graph: &IncidenceGraph,
    schema_id: GraphSchemaId,
    root_partition: Vec<usize>,
    root_passes: u64,
    budget: CanonicalSearchBudget,
) -> Result<MicrocanonOutcome, GraphError> {
    let cell_count = partition_cell_count(&root_partition);
    let sizes = cell_sizes(&root_partition, cell_count);
    let root_largest_cell = sizes.iter().copied().max().unwrap_or(0);
    let root_ambiguous_vertices = sizes.into_iter().filter(|size| *size > 1).sum();
    let report = MicrocanonReport {
        path: MicrocanonPath::ExactRefinementDiscrete,
        root_cell_count: cell_count,
        root_largest_cell,
        root_ambiguous_vertices,
        explored_nodes: 0,
        leaf_count: 1,
        individualization_count: 0,
        exact_refinement_passes: root_passes,
        maximum_depth: 0,
        peak_retained_state_cells: root_partition.len(),
        peak_tracked_bytes: 0,
        trace_event_count: root_passes,
        target_cell_count: 0,
        verified_automorphism_count: 0,
        orbit_pruned_child_count: 0,
        prefix_pruned_leaf_count: 0,
        elapsed: Duration::ZERO,
        exhausted_limit: None,
    };

    if cell_count == graph.vertex_count() {
        let mut order = (0..graph.vertex_count())
            .map(VertexId::new)
            .collect::<Vec<_>>();
        order.sort_unstable_by_key(|vertex| root_partition[vertex.index()]);
        let form = canonical_form_from_order(graph, order, schema_id)?;
        return Ok(MicrocanonOutcome::Exact { form, report });
    }

    let topology = GlobalTopology::build(graph)?;
    if topology.weak_component_count > 1 {
        return canonicalize_by_weak_components(
            graph,
            schema_id,
            &topology.weak_components,
            topology.weak_component_count,
            budget,
            report,
        );
    }

    exact_search(graph, schema_id, root_partition, report, budget)
}

fn canonicalize_by_weak_components(
    graph: &IncidenceGraph,
    schema_id: GraphSchemaId,
    components: &[usize],
    component_count: usize,
    budget: CanonicalSearchBudget,
    mut report: MicrocanonReport,
) -> Result<MicrocanonOutcome, GraphError> {
    report.path = MicrocanonPath::WeakComponentDecomposition;
    report.leaf_count = 0;
    let mut exact_components = Vec::with_capacity(component_count);
    for component in 0..component_count {
        let (subgraph, local_to_original) = extract_weak_component(graph, components, component)?;
        let remaining_nodes = budget
            .max_search_nodes
            .saturating_sub(report.explored_nodes);
        let local_budget = CanonicalSearchBudget::new(remaining_nodes)
            .with_max_retained_state_cells(budget.max_retained_state_cells);
        match canonicalize_core(&subgraph, schema_id, local_budget)? {
            MicrocanonOutcome::Exact {
                form,
                report: local,
            } => {
                accumulate_report(&mut report, &local)?;
                let order = form
                    .canonical_to_original()
                    .iter()
                    .map(|vertex| local_to_original[vertex.index()])
                    .collect::<Vec<_>>();
                exact_components.push((form.bytes().to_vec(), order));
            }
            MicrocanonOutcome::Inconclusive { report: local } => {
                accumulate_report(&mut report, &local)?;
                report.exhausted_limit = local.exhausted_limit;
                return Ok(MicrocanonOutcome::Inconclusive { report });
            }
        }
    }
    exact_components.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    let order = exact_components
        .into_iter()
        .flat_map(|(_, order)| order)
        .collect();
    let form = canonical_form_from_order(graph, order, schema_id)?;
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
            debug_assert_eq!(components[target], selected);
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

fn accumulate_report(
    total: &mut MicrocanonReport,
    local: &MicrocanonReport,
) -> Result<(), GraphError> {
    total.explored_nodes = total
        .explored_nodes
        .checked_add(local.explored_nodes)
        .ok_or(GraphError::GraphTooLarge)?;
    total.leaf_count = total
        .leaf_count
        .checked_add(local.leaf_count)
        .ok_or(GraphError::GraphTooLarge)?;
    total.individualization_count = total
        .individualization_count
        .checked_add(local.individualization_count)
        .ok_or(GraphError::GraphTooLarge)?;
    total.exact_refinement_passes = total
        .exact_refinement_passes
        .checked_add(local.exact_refinement_passes)
        .ok_or(GraphError::GraphTooLarge)?;
    total.maximum_depth = total.maximum_depth.max(local.maximum_depth);
    total.peak_retained_state_cells = total
        .peak_retained_state_cells
        .max(local.peak_retained_state_cells);
    total.peak_tracked_bytes = total.peak_tracked_bytes.max(local.peak_tracked_bytes);
    total.trace_event_count = total
        .trace_event_count
        .checked_add(local.trace_event_count)
        .ok_or(GraphError::GraphTooLarge)?;
    total.target_cell_count = total
        .target_cell_count
        .checked_add(local.target_cell_count)
        .ok_or(GraphError::GraphTooLarge)?;
    total.verified_automorphism_count = total
        .verified_automorphism_count
        .checked_add(local.verified_automorphism_count)
        .ok_or(GraphError::GraphTooLarge)?;
    total.orbit_pruned_child_count = total
        .orbit_pruned_child_count
        .checked_add(local.orbit_pruned_child_count)
        .ok_or(GraphError::GraphTooLarge)?;
    total.prefix_pruned_leaf_count = total
        .prefix_pruned_leaf_count
        .checked_add(local.prefix_pruned_leaf_count)
        .ok_or(GraphError::GraphTooLarge)?;
    Ok(())
}

pub(crate) fn exact_stable_partition(
    graph: &IncidenceGraph,
) -> Result<(Vec<usize>, u64), GraphError> {
    let mut keys = Vec::with_capacity(graph.vertex_count());
    for index in 0..graph.vertex_count() {
        let vertex = VertexId::new(index);
        let mut key = vec![0, graph.vertex_kind(vertex) as u8];
        append_framed(&mut key, graph.vertex_label(vertex))?;
        keys.push(key);
    }
    let colors = canonical_colors(&keys)?;
    refine_exact(graph, colors)
}

fn refine_exact(
    graph: &IncidenceGraph,
    mut colors: Vec<usize>,
) -> Result<(Vec<usize>, u64), GraphError> {
    let mut passes = 0_u64;
    loop {
        passes = passes.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
        let mut keys = Vec::with_capacity(graph.vertex_count());
        for index in 0..graph.vertex_count() {
            let vertex = VertexId::new(index);
            let mut key = Vec::new();
            key.push(1);
            append_usize(&mut key, colors[index])?;
            append_incidence_multiset(&mut key, graph, graph.outgoing(vertex), &colors)?;
            append_incidence_multiset(&mut key, graph, graph.incoming(vertex), &colors)?;
            keys.push(key);
        }
        let next = canonical_colors(&keys)?;
        if next == colors {
            return Ok((colors, passes));
        }
        colors = next;
    }
}

fn append_incidence_multiset(
    output: &mut Vec<u8>,
    graph: &IncidenceGraph,
    incidences: &[Incidence],
    colors: &[usize],
) -> Result<(), GraphError> {
    let mut entries = Vec::with_capacity(incidences.len());
    for incidence in incidences {
        let descriptor = graph.relation(incidence.relation());
        let mut entry = Vec::new();
        append_usize(&mut entry, colors[incidence.neighbor().index()])?;
        append_framed(&mut entry, descriptor.relation())?;
        append_framed(&mut entry, descriptor.role())?;
        entry.extend_from_slice(&incidence.multiplicity().to_be_bytes());
        entries.push(entry);
    }
    entries.sort_unstable();
    append_usize(output, entries.len())?;
    for entry in entries {
        append_framed(output, &entry)?;
    }
    Ok(())
}

fn append_usize(output: &mut Vec<u8>, value: usize) -> Result<(), GraphError> {
    let value = u64::try_from(value).map_err(|_| GraphError::GraphTooLarge)?;
    output.extend_from_slice(&value.to_be_bytes());
    Ok(())
}

fn append_framed(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), GraphError> {
    append_usize(output, bytes.len())?;
    output.extend_from_slice(bytes);
    Ok(())
}

fn canonical_colors(keys: &[Vec<u8>]) -> Result<Vec<usize>, GraphError> {
    let mut unique = keys.to_vec();
    unique.sort_unstable();
    unique.dedup();
    keys.iter()
        .map(|key| {
            unique
                .binary_search(key)
                .map_err(|_| GraphError::CanonicalizationInvariantViolation)
        })
        .collect()
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

#[derive(Debug)]
struct BranchFrame {
    colors: Vec<usize>,
    vertices: Vec<usize>,
    next_vertex: usize,
    cell_count: usize,
    depth: usize,
}

fn exact_search(
    graph: &IncidenceGraph,
    schema_id: GraphSchemaId,
    root_colors: Vec<usize>,
    mut report: MicrocanonReport,
    budget: CanonicalSearchBudget,
) -> Result<MicrocanonOutcome, GraphError> {
    report.path = MicrocanonPath::IndividualizationRefinement;
    report.leaf_count = 0;
    if root_colors.len() > budget.max_retained_state_cells {
        report.exhausted_limit = Some(CanonicalBudgetLimit::RetainedStateCells);
        return Ok(MicrocanonOutcome::Inconclusive { report });
    }

    let mut retained_cells = root_colors.len();
    let mut pending = Some((root_colors, 0_usize, true));
    let mut stack: Vec<BranchFrame> = Vec::new();
    let mut best: Option<CanonicalGraphForm> = None;

    loop {
        if let Some((colors, depth, already_refined)) = pending.take() {
            retained_cells -= colors.len();
            if report.explored_nodes >= budget.max_search_nodes {
                report.exhausted_limit = Some(CanonicalBudgetLimit::SearchNodes);
                return Ok(MicrocanonOutcome::Inconclusive { report });
            }
            report.explored_nodes += 1;
            report.maximum_depth = report.maximum_depth.max(depth);
            let colors = if already_refined {
                colors
            } else {
                let (refined, passes) = refine_exact(graph, colors)?;
                report.exact_refinement_passes = report
                    .exact_refinement_passes
                    .checked_add(passes)
                    .ok_or(GraphError::GraphTooLarge)?;
                refined
            };
            let cell_count = partition_cell_count(&colors);
            if cell_count == graph.vertex_count() {
                report.leaf_count = report
                    .leaf_count
                    .checked_add(1)
                    .ok_or(GraphError::GraphTooLarge)?;
                let mut order = (0..graph.vertex_count())
                    .map(VertexId::new)
                    .collect::<Vec<_>>();
                order.sort_unstable_by_key(|vertex| colors[vertex.index()]);
                let candidate = canonical_form_from_order(graph, order, schema_id)?;
                if best
                    .as_ref()
                    .is_none_or(|current| candidate.bytes() < current.bytes())
                {
                    best = Some(candidate);
                }
            } else {
                let vertices = select_individualization_cell(&colors, cell_count)?;
                let frame_cells = colors
                    .len()
                    .checked_add(vertices.len())
                    .ok_or(GraphError::GraphTooLarge)?;
                let next_retained = retained_cells
                    .checked_add(frame_cells)
                    .ok_or(GraphError::GraphTooLarge)?;
                if next_retained > budget.max_retained_state_cells {
                    report.exhausted_limit = Some(CanonicalBudgetLimit::RetainedStateCells);
                    return Ok(MicrocanonOutcome::Inconclusive { report });
                }
                retained_cells = next_retained;
                report.peak_retained_state_cells =
                    report.peak_retained_state_cells.max(retained_cells);
                stack.push(BranchFrame {
                    colors,
                    vertices,
                    next_vertex: 0,
                    cell_count,
                    depth,
                });
            }
        }

        let mut scheduled = false;
        while let Some(frame) = stack.last_mut() {
            if frame.next_vertex < frame.vertices.len() {
                let required = frame.colors.len();
                let next_retained = retained_cells
                    .checked_add(required)
                    .ok_or(GraphError::GraphTooLarge)?;
                if next_retained > budget.max_retained_state_cells {
                    report.exhausted_limit = Some(CanonicalBudgetLimit::RetainedStateCells);
                    return Ok(MicrocanonOutcome::Inconclusive { report });
                }
                let vertex = frame.vertices[frame.next_vertex];
                frame.next_vertex += 1;
                let mut child = frame.colors.clone();
                child[vertex] = frame.cell_count;
                report.individualization_count = report
                    .individualization_count
                    .checked_add(1)
                    .ok_or(GraphError::GraphTooLarge)?;
                retained_cells = next_retained;
                report.peak_retained_state_cells =
                    report.peak_retained_state_cells.max(retained_cells);
                pending = Some((child, frame.depth + 1, false));
                scheduled = true;
                break;
            }
            let finished = stack
                .pop()
                .ok_or(GraphError::CanonicalizationInvariantViolation)?;
            retained_cells -= finished.colors.len() + finished.vertices.len();
        }

        if !scheduled && stack.is_empty() {
            let form = best.ok_or(GraphError::CanonicalizationInvariantViolation)?;
            return Ok(MicrocanonOutcome::Exact { form, report });
        }
    }
}

fn select_individualization_cell(
    colors: &[usize],
    cell_count: usize,
) -> Result<Vec<usize>, GraphError> {
    let sizes = cell_sizes(colors, cell_count);
    let selected = sizes
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, size)| *size > 1)
        .min_by_key(|(cell, size)| (*size, *cell))
        .map(|(cell, _)| cell)
        .ok_or(GraphError::CanonicalizationInvariantViolation)?;
    Ok(colors
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(vertex, cell)| (cell == selected).then_some(vertex))
        .collect())
}
