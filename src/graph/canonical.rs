//! Exact refinement diagnostics and opt-in bounded graph canonization.
//!
//! The finite-field labeler remains the default path. This module uses exact
//! byte descriptors to distinguish arithmetic aliasing from limitations of
//! local color refinement, then offers exhaustive individualization only when
//! the caller supplies an explicit budget.

use std::collections::BTreeSet;

use microfield::{CanonicalEncoding, Field, Pow, StaticField};

use crate::structural::StructuralEncoder;

use super::labeler::{
    canonical_form_from_order, discrete_form, DiscreteCanonicalForm, FastGraphAnalysis,
    FastGraphLabeler,
};
use super::{global::GlobalTopology, GraphError, IncidenceGraph, IncidenceGraphBuilder, VertexId};

const DEFAULT_RETAINED_STATE_CELLS: usize = 16 * 1024 * 1024;

/// Action suggested by the exact-vs-field partition diagnosis.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiscriminationRecommendation {
    /// The fast invariant partition is discrete and already defines an exact order.
    FastPathSufficient,
    /// Exact local refinement separates values collapsed by the selected field profile.
    AddIndependentEvidenceOrCanonize,
    /// Local refinement itself is ambiguous; only exact search can certify a result.
    ExactCanonicalizationRecommended,
}

/// Measured reason why a fast graph signature may lose discrimination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphDegeneracyReport {
    vertex_count: usize,
    fast_cell_count: usize,
    exact_refinement_cell_count: usize,
    largest_fast_cell: usize,
    largest_exact_refinement_cell: usize,
    ambiguous_vertex_count: usize,
    field_aliasing_cell_count: usize,
    field_aliasing_vertex_count: usize,
    highly_regular: bool,
    recommendation: DiscriminationRecommendation,
}

impl GraphDegeneracyReport {
    /// Number of vertices in the normalized graph.
    #[must_use]
    pub const fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    /// Number of equality classes produced by the selected finite-field profile.
    #[must_use]
    pub const fn fast_cell_count(&self) -> usize {
        self.fast_cell_count
    }

    /// Number of stable 1-WL classes computed from exact byte descriptors.
    #[must_use]
    pub const fn exact_refinement_cell_count(&self) -> usize {
        self.exact_refinement_cell_count
    }

    /// Size of the largest finite-field class.
    #[must_use]
    pub const fn largest_fast_cell(&self) -> usize {
        self.largest_fast_cell
    }

    /// Size of the largest exact local-refinement class.
    #[must_use]
    pub const fn largest_exact_refinement_cell(&self) -> usize {
        self.largest_exact_refinement_cell
    }

    /// Vertices that remain in non-singleton exact local-refinement classes.
    #[must_use]
    pub const fn ambiguous_vertex_count(&self) -> usize {
        self.ambiguous_vertex_count
    }

    /// Finite-field classes containing more than one exact refinement class.
    #[must_use]
    pub const fn field_aliasing_cell_count(&self) -> usize {
        self.field_aliasing_cell_count
    }

    /// Vertices contained in finite-field classes affected by arithmetic aliasing.
    #[must_use]
    pub const fn field_aliasing_vertex_count(&self) -> usize {
        self.field_aliasing_vertex_count
    }

    /// Whether the documented high-regularity threshold was reached.
    ///
    /// Version 1 requires at least four vertices, at least 75% of vertices in
    /// non-singleton exact classes, and one class containing at least 25% of
    /// the graph. It is a routing signal, not a graph-theoretic proof.
    #[must_use]
    pub const fn is_highly_regular(&self) -> bool {
        self.highly_regular
    }

    /// Safe next action derived from the measured cause of ambiguity.
    #[must_use]
    pub const fn recommendation(&self) -> DiscriminationRecommendation {
        self.recommendation
    }

    /// Returns true when exact local descriptors distinguish vertices that the
    /// selected field/lane profile collapsed.
    #[must_use]
    pub const fn has_field_aliasing(&self) -> bool {
        self.field_aliasing_cell_count != 0
    }

    /// Returns true when exact 1-WL refinement is itself non-discrete.
    #[must_use]
    pub const fn has_local_ambiguity(&self) -> bool {
        self.exact_refinement_cell_count != self.vertex_count
    }
}

/// Explicit hard limits for opt-in individualization/refinement search.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CanonicalSearchBudget {
    max_search_nodes: u64,
    max_retained_state_cells: usize,
}

impl CanonicalSearchBudget {
    /// Creates a node-bounded search with a conservative retained-state limit.
    #[must_use]
    pub const fn new(max_search_nodes: u64) -> Self {
        Self {
            max_search_nodes,
            max_retained_state_cells: DEFAULT_RETAINED_STATE_CELLS,
        }
    }

    /// Bounds retained `usize` cells used by the depth-first search frontier.
    #[must_use]
    pub const fn with_max_retained_state_cells(mut self, cells: usize) -> Self {
        self.max_retained_state_cells = cells;
        self
    }

    /// Maximum number of individualization/refinement nodes.
    #[must_use]
    pub const fn max_search_nodes(self) -> u64 {
        self.max_search_nodes
    }

    /// Maximum retained frontier cells.
    #[must_use]
    pub const fn max_retained_state_cells(self) -> usize {
        self.max_retained_state_cells
    }
}

/// Limit that stopped an exact search before certification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CanonicalBudgetLimit {
    /// The maximum number of search nodes was reached.
    SearchNodes,
    /// Retaining another DFS state would exceed the memory-shaped limit.
    RetainedStateCells,
}

/// Route used by a successful exact canonicalization.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CanonicalizationPath {
    /// Finite-field labels were already pairwise distinct.
    FastDiscrete,
    /// Weak components were certified independently and sorted by exact form.
    WeakComponentDecomposition,
    /// Exact byte refinement and exhaustive individualization were used.
    IndividualizationRefinement,
}

/// Auditable counters for one bounded exact-canonicalization request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalSearchReport {
    degeneracy: GraphDegeneracyReport,
    path: CanonicalizationPath,
    explored_nodes: u64,
    leaf_count: u64,
    individualization_count: u64,
    exact_refinement_passes: u64,
    maximum_depth: usize,
    peak_retained_state_cells: usize,
    exhausted_limit: Option<CanonicalBudgetLimit>,
}

impl CanonicalSearchReport {
    /// Exact diagnosis that selected the canonicalization route.
    #[must_use]
    pub const fn degeneracy(&self) -> &GraphDegeneracyReport {
        &self.degeneracy
    }

    /// Fast-discrete or individualization/refinement route.
    #[must_use]
    pub const fn path(&self) -> CanonicalizationPath {
        self.path
    }

    /// Number of exact search nodes entered.
    #[must_use]
    pub const fn explored_nodes(&self) -> u64 {
        self.explored_nodes
    }

    /// Number of complete discrete orders evaluated.
    #[must_use]
    pub const fn leaf_count(&self) -> u64 {
        self.leaf_count
    }

    /// Number of child individualizations scheduled.
    #[must_use]
    pub const fn individualization_count(&self) -> u64 {
        self.individualization_count
    }

    /// Number of exact color-refinement passes, including stability checks.
    #[must_use]
    pub const fn exact_refinement_passes(&self) -> u64 {
        self.exact_refinement_passes
    }

    /// Deepest individualization level reached.
    #[must_use]
    pub const fn maximum_depth(&self) -> usize {
        self.maximum_depth
    }

    /// Peak retained DFS state measured in `usize` cells.
    #[must_use]
    pub const fn peak_retained_state_cells(&self) -> usize {
        self.peak_retained_state_cells
    }

    /// Limit responsible for an incomplete result, if any.
    #[must_use]
    pub const fn exhausted_limit(&self) -> Option<CanonicalBudgetLimit> {
        self.exhausted_limit
    }
}

/// Exact canonical bytes or a fail-closed statement that the budget was insufficient.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactCanonicalOutcome {
    /// The returned form is an exact canonical representative.
    Exact {
        /// Injective ordered serialization of the normalized graph.
        form: DiscreteCanonicalForm,
        /// Diagnostics and work counters.
        report: CanonicalSearchReport,
    },
    /// No canonical form is published because the complete tree was not explored.
    BudgetExhausted {
        /// Diagnostics, partial-work counters and the exhausted limit.
        report: CanonicalSearchReport,
    },
}

impl ExactCanonicalOutcome {
    /// Borrows the complete report regardless of success.
    #[must_use]
    pub const fn report(&self) -> &CanonicalSearchReport {
        match self {
            Self::Exact { report, .. } | Self::BudgetExhausted { report } => report,
        }
    }
}

impl<F, E, const K: usize> FastGraphLabeler<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField + Pow,
    E: StructuralEncoder<F>,
{
    /// Diagnoses field aliasing and exact local-refinement ambiguity.
    ///
    /// This is an opt-in control-plane operation. With `T` exact refinement
    /// passes it costs `O(T (V + I log d))`, where `T <= V` and `d` is the
    /// largest CSR row. It does not alter [`FastGraphLabeler::analyze`].
    ///
    /// # Errors
    ///
    /// Propagates graph-size and structural-encoding failures.
    pub fn diagnose_degeneracy(
        &self,
        graph: &IncidenceGraph,
    ) -> Result<GraphDegeneracyReport, GraphError> {
        let analysis = self.analyze(graph)?;
        let (exact_partition, _) = exact_stable_partition(graph)?;
        Ok(build_degeneracy_report(&analysis, &exact_partition))
    }

    /// Produces an exact canonical representative under an explicit budget.
    ///
    /// A discrete fast partition takes the existing linear route. Otherwise
    /// the method uses exact byte refinement and exhaustive
    /// individualization. Budget exhaustion never publishes a best-so-far
    /// candidate as canonical.
    ///
    /// # Errors
    ///
    /// Propagates graph-size and structural-encoding failures.
    pub fn canonicalize_exact(
        &self,
        graph: &IncidenceGraph,
        budget: CanonicalSearchBudget,
    ) -> Result<ExactCanonicalOutcome, GraphError> {
        let analysis = self.analyze(graph)?;
        let (exact_partition, root_refinement_passes) = exact_stable_partition(graph)?;
        let degeneracy = build_degeneracy_report(&analysis, &exact_partition);

        if analysis.cell_count() == graph.vertex_count() {
            let form = discrete_form(graph, analysis.labels(), self.signature_id())?;
            return Ok(ExactCanonicalOutcome::Exact {
                form,
                report: CanonicalSearchReport {
                    degeneracy,
                    path: CanonicalizationPath::FastDiscrete,
                    explored_nodes: 0,
                    leaf_count: 1,
                    individualization_count: 0,
                    exact_refinement_passes: root_refinement_passes,
                    maximum_depth: 0,
                    peak_retained_state_cells: exact_partition.len(),
                    exhausted_limit: None,
                },
            });
        }

        let topology = GlobalTopology::build(graph)?;
        if topology.weak_component_count > 1 {
            return self.canonicalize_by_weak_components(
                graph,
                &topology.weak_components,
                topology.weak_component_count,
                budget,
                degeneracy,
                root_refinement_passes,
            );
        }

        exact_search(
            graph,
            self.signature_id(),
            exact_partition,
            root_refinement_passes,
            degeneracy,
            budget,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn canonicalize_by_weak_components(
        &self,
        graph: &IncidenceGraph,
        components: &[usize],
        component_count: usize,
        budget: CanonicalSearchBudget,
        degeneracy: GraphDegeneracyReport,
        root_refinement_passes: u64,
    ) -> Result<ExactCanonicalOutcome, GraphError> {
        let mut report = CanonicalSearchReport {
            degeneracy,
            path: CanonicalizationPath::WeakComponentDecomposition,
            explored_nodes: 0,
            leaf_count: 0,
            individualization_count: 0,
            exact_refinement_passes: root_refinement_passes,
            maximum_depth: 0,
            peak_retained_state_cells: components.len(),
            exhausted_limit: None,
        };
        let mut exact_components = Vec::with_capacity(component_count);
        for component in 0..component_count {
            let (subgraph, local_to_original) =
                extract_weak_component(graph, components, component)?;
            let remaining_nodes = budget
                .max_search_nodes
                .saturating_sub(report.explored_nodes);
            let local_budget = CanonicalSearchBudget::new(remaining_nodes)
                .with_max_retained_state_cells(budget.max_retained_state_cells);
            match self.canonicalize_exact(&subgraph, local_budget)? {
                ExactCanonicalOutcome::Exact {
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
                ExactCanonicalOutcome::BudgetExhausted { report: local } => {
                    accumulate_report(&mut report, &local)?;
                    report.exhausted_limit = local.exhausted_limit;
                    return Ok(ExactCanonicalOutcome::BudgetExhausted { report });
                }
            }
        }
        exact_components.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        let canonical_to_original = exact_components
            .into_iter()
            .flat_map(|(_, order)| order)
            .collect();
        let form = canonical_form_from_order(graph, canonical_to_original, self.signature_id())?;
        Ok(ExactCanonicalOutcome::Exact { form, report })
    }
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
    total: &mut CanonicalSearchReport,
    local: &CanonicalSearchReport,
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
    Ok(())
}

fn build_degeneracy_report<F: Field, const K: usize>(
    analysis: &FastGraphAnalysis<F, K>,
    exact_partition: &[usize],
) -> GraphDegeneracyReport {
    let vertex_count = exact_partition.len();
    let exact_refinement_cell_count = partition_cell_count(exact_partition);
    let fast_sizes = cell_sizes(analysis.partition(), analysis.cell_count());
    let exact_sizes = cell_sizes(exact_partition, exact_refinement_cell_count);
    let largest_fast_cell = fast_sizes.iter().copied().max().unwrap_or(0);
    let largest_exact_refinement_cell = exact_sizes.iter().copied().max().unwrap_or(0);
    let ambiguous_vertex_count = exact_sizes.iter().copied().filter(|size| *size > 1).sum();

    let mut exact_colors_by_fast_cell = vec![BTreeSet::new(); analysis.cell_count()];
    for (&fast, &exact) in analysis.partition().iter().zip(exact_partition) {
        exact_colors_by_fast_cell[fast].insert(exact);
    }
    let mut field_aliasing_cell_count = 0;
    let mut field_aliasing_vertex_count = 0;
    for (fast_cell, exact_colors) in exact_colors_by_fast_cell.iter().enumerate() {
        if exact_colors.len() > 1 {
            field_aliasing_cell_count += 1;
            field_aliasing_vertex_count += fast_sizes[fast_cell];
        }
    }

    let minimum_ambiguous = vertex_count.saturating_sub(vertex_count / 4);
    let minimum_large_cell = (vertex_count / 4) + usize::from(!vertex_count.is_multiple_of(4));
    let highly_regular = vertex_count >= 4
        && ambiguous_vertex_count >= minimum_ambiguous
        && largest_exact_refinement_cell >= minimum_large_cell;
    let recommendation = if exact_refinement_cell_count != vertex_count {
        DiscriminationRecommendation::ExactCanonicalizationRecommended
    } else if field_aliasing_cell_count != 0 {
        DiscriminationRecommendation::AddIndependentEvidenceOrCanonize
    } else {
        DiscriminationRecommendation::FastPathSufficient
    };

    GraphDegeneracyReport {
        vertex_count,
        fast_cell_count: analysis.cell_count(),
        exact_refinement_cell_count,
        largest_fast_cell,
        largest_exact_refinement_cell,
        ambiguous_vertex_count,
        field_aliasing_cell_count,
        field_aliasing_vertex_count,
        highly_regular,
        recommendation,
    }
}

fn cell_sizes(partition: &[usize], cell_count: usize) -> Vec<usize> {
    let mut sizes = vec![0; cell_count];
    for &cell in partition {
        sizes[cell] += 1;
    }
    sizes
}

fn partition_cell_count(partition: &[usize]) -> usize {
    partition.iter().copied().max().map_or(0, |cell| cell + 1)
}

fn exact_stable_partition(graph: &IncidenceGraph) -> Result<(Vec<usize>, u64), GraphError> {
    let mut keys = Vec::with_capacity(graph.vertex_count());
    for index in 0..graph.vertex_count() {
        let vertex = VertexId::new(index);
        let mut key = vec![0, graph.vertex_kind(vertex) as u8];
        append_framed(&mut key, graph.vertex_label(vertex))?;
        keys.push(key);
    }
    let colors = canonical_colors(&keys);
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
        let next = canonical_colors(&keys);
        if next == colors {
            return Ok((colors, passes));
        }
        colors = next;
    }
}

fn append_incidence_multiset(
    output: &mut Vec<u8>,
    graph: &IncidenceGraph,
    incidences: &[super::Incidence],
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

fn canonical_colors(keys: &[Vec<u8>]) -> Vec<usize> {
    let mut unique = keys.to_vec();
    unique.sort_unstable();
    unique.dedup();
    keys.iter()
        .map(|key| {
            unique
                .binary_search(key)
                .expect("the canonical key pool contains every vertex key")
        })
        .collect()
}

#[derive(Debug)]
struct BranchFrame {
    colors: Vec<usize>,
    vertices: Vec<usize>,
    next_vertex: usize,
    cell_count: usize,
    depth: usize,
}

#[allow(clippy::too_many_arguments)]
fn exact_search(
    graph: &IncidenceGraph,
    signature_id: super::GraphSignatureId,
    root_colors: Vec<usize>,
    root_refinement_passes: u64,
    degeneracy: GraphDegeneracyReport,
    budget: CanonicalSearchBudget,
) -> Result<ExactCanonicalOutcome, GraphError> {
    let mut report = CanonicalSearchReport {
        degeneracy,
        path: CanonicalizationPath::IndividualizationRefinement,
        explored_nodes: 0,
        leaf_count: 0,
        individualization_count: 0,
        exact_refinement_passes: root_refinement_passes,
        maximum_depth: 0,
        peak_retained_state_cells: root_colors.len(),
        exhausted_limit: None,
    };
    if root_colors.len() > budget.max_retained_state_cells {
        report.exhausted_limit = Some(CanonicalBudgetLimit::RetainedStateCells);
        return Ok(ExactCanonicalOutcome::BudgetExhausted { report });
    }

    let mut retained_cells = root_colors.len();
    let mut pending = Some((root_colors, 0_usize, true));
    let mut stack: Vec<BranchFrame> = Vec::new();
    let mut best: Option<DiscreteCanonicalForm> = None;

    loop {
        if let Some((colors, depth, already_refined)) = pending.take() {
            retained_cells -= colors.len();
            if report.explored_nodes >= budget.max_search_nodes {
                report.exhausted_limit = Some(CanonicalBudgetLimit::SearchNodes);
                return Ok(ExactCanonicalOutcome::BudgetExhausted { report });
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
                let mut order: Vec<_> = (0..graph.vertex_count()).map(VertexId::new).collect();
                order.sort_unstable_by_key(|vertex| colors[vertex.index()]);
                let candidate = canonical_form_from_order(graph, order, signature_id)?;
                if best
                    .as_ref()
                    .is_none_or(|current| candidate.bytes() < current.bytes())
                {
                    best = Some(candidate);
                }
            } else {
                let vertices = select_individualization_cell(&colors, cell_count);
                let frame_cells = colors
                    .len()
                    .checked_add(vertices.len())
                    .ok_or(GraphError::GraphTooLarge)?;
                let next_retained = retained_cells
                    .checked_add(frame_cells)
                    .ok_or(GraphError::GraphTooLarge)?;
                if next_retained > budget.max_retained_state_cells {
                    report.exhausted_limit = Some(CanonicalBudgetLimit::RetainedStateCells);
                    return Ok(ExactCanonicalOutcome::BudgetExhausted { report });
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
                    return Ok(ExactCanonicalOutcome::BudgetExhausted { report });
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
            let finished = stack.pop().expect("the last DFS frame exists");
            retained_cells -= finished.colors.len() + finished.vertices.len();
        }

        if !scheduled && stack.is_empty() {
            let form = best.expect("a completed finite search has at least one leaf");
            return Ok(ExactCanonicalOutcome::Exact { form, report });
        }
    }
}

fn select_individualization_cell(colors: &[usize], cell_count: usize) -> Vec<usize> {
    let sizes = cell_sizes(colors, cell_count);
    let selected = sizes
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, size)| *size > 1)
        .min_by_key(|(cell, size)| (*size, *cell))
        .map(|(cell, _)| cell)
        .expect("a non-discrete partition contains a non-singleton cell");
    colors
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(vertex, cell)| (cell == selected).then_some(vertex))
        .collect()
}
