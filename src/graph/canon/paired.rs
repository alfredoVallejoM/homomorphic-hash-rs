//! Exact paired comparison with joint refinement and verified mappings.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use super::search::{CanonicalBudgetLimit, CanonicalSearchBudget};
use super::{DifferenceWitness, GraphComparison, GraphComparisonReport, VerifiedGraphMapping};
use crate::graph::{GraphError, Incidence, IncidenceGraph, VertexId};

/// Exact route selected by the G12 two-graph comparator.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PairedComparisonPath {
    /// Sorted exact metadata rejected the pair before refinement.
    ExactPrefilter,
    /// Both inputs were forests and exact relational tree coding was used.
    TreeForest,
    /// Articulation/block information participated in paired matching.
    BlockCutDecomposition,
    /// General joint refinement and fail-first candidate search.
    PairedSearch,
}

/// Auditable work counters for direct two-graph comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairedComparisonReport {
    path: PairedComparisonPath,
    refinement_passes: u64,
    explored_nodes: u64,
    candidate_pairs: u64,
    backtrack_count: u64,
    weak_component_count: usize,
    block_count: usize,
    articulation_vertex_count: usize,
    peak_tracked_bytes: usize,
    elapsed: Duration,
    exhausted_limit: Option<CanonicalBudgetLimit>,
}

impl PairedComparisonReport {
    fn new(path: PairedComparisonPath) -> Self {
        Self {
            path,
            refinement_passes: 0,
            explored_nodes: 0,
            candidate_pairs: 0,
            backtrack_count: 0,
            weak_component_count: 0,
            block_count: 0,
            articulation_vertex_count: 0,
            peak_tracked_bytes: 0,
            elapsed: Duration::ZERO,
            exhausted_limit: None,
        }
    }

    /// Route used by the exact comparator.
    #[must_use]
    pub const fn path(&self) -> PairedComparisonPath {
        self.path
    }

    /// Joint exact refinement passes.
    #[must_use]
    pub const fn refinement_passes(&self) -> u64 {
        self.refinement_passes
    }

    /// Candidate assignments entered.
    #[must_use]
    pub const fn explored_nodes(&self) -> u64 {
        self.explored_nodes
    }

    /// Candidate pairs admitted by exact cell/domain checks.
    #[must_use]
    pub const fn candidate_pairs(&self) -> u64 {
        self.candidate_pairs
    }

    /// Exhausted or contradicted decision frames.
    #[must_use]
    pub const fn backtrack_count(&self) -> u64 {
        self.backtrack_count
    }

    /// Weak components observed on either compatible input.
    #[must_use]
    pub const fn weak_component_count(&self) -> usize {
        self.weak_component_count
    }

    /// Biconnected support blocks.
    #[must_use]
    pub const fn block_count(&self) -> usize {
        self.block_count
    }

    /// Articulation vertices represented in the block-cut profile.
    #[must_use]
    pub const fn articulation_vertex_count(&self) -> usize {
        self.articulation_vertex_count
    }

    /// Logical bytes retained by mappings and decision domains.
    #[must_use]
    pub const fn peak_tracked_bytes(&self) -> usize {
        self.peak_tracked_bytes
    }

    /// Wall-clock time observed by paired comparison.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Hard limit that prevented a decision.
    #[must_use]
    pub const fn exhausted_limit(&self) -> Option<CanonicalBudgetLimit> {
        self.exhausted_limit
    }
}

pub(super) fn compare(
    left: &IncidenceGraph,
    right: &IncidenceGraph,
    budget: CanonicalSearchBudget,
) -> Result<GraphComparison, GraphError> {
    let started = Instant::now();
    let left_support = undirected_support(left);
    let right_support = undirected_support(right);
    let left_descriptors = vertex_descriptor_profile(left, &left_support)?;
    let right_descriptors = vertex_descriptor_profile(right, &right_support)?;
    if left_descriptors != right_descriptors {
        return Ok(different(
            DifferenceWitness::VertexDescriptors {
                first_differing_byte: first_difference(&left_descriptors, &right_descriptors),
            },
            finish(
                PairedComparisonReport::new(PairedComparisonPath::ExactPrefilter),
                started,
            ),
        ));
    }

    let left_blocks = BlockCutSummary::build(left, left_support)?;
    let right_blocks = BlockCutSummary::build(right, right_support)?;
    if left_blocks.profile != right_blocks.profile {
        let mut report = PairedComparisonReport::new(PairedComparisonPath::ExactPrefilter);
        report.weak_component_count = left_blocks.component_count;
        report.block_count = left_blocks.blocks.len();
        report.articulation_vertex_count = left_blocks.articulation.iter().filter(|&&v| v).count();
        return Ok(different(
            DifferenceWitness::BlockCutProfile {
                first_differing_byte: first_difference(&left_blocks.profile, &right_blocks.profile),
            },
            finish(report, started),
        ));
    }

    let left_forest = left_blocks.is_forest;
    let right_forest = right_blocks.is_forest;
    if left_forest != right_forest {
        return Ok(different(
            DifferenceWitness::TreeForest,
            finish(
                PairedComparisonReport::new(PairedComparisonPath::ExactPrefilter),
                started,
            ),
        ));
    }

    let mut report = PairedComparisonReport::new(if left_forest {
        PairedComparisonPath::TreeForest
    } else if left_blocks.articulation.iter().any(|value| *value) {
        PairedComparisonPath::BlockCutDecomposition
    } else {
        PairedComparisonPath::PairedSearch
    });
    report.weak_component_count = left_blocks.component_count;
    report.block_count = left_blocks.blocks.len();
    report.articulation_vertex_count = left_blocks.articulation.iter().filter(|&&v| v).count();

    if budget.max_search_nodes() == 0 {
        report.exhausted_limit = Some(CanonicalBudgetLimit::SearchNodes);
        return Ok(inconclusive(finish(report, started)));
    }
    if budget
        .max_elapsed()
        .is_some_and(|limit| started.elapsed() >= limit)
    {
        report.exhausted_limit = Some(CanonicalBudgetLimit::ElapsedTime);
        return Ok(inconclusive(finish(report, started)));
    }

    if left_forest {
        let forest_nodes =
            u64::try_from(left.vertex_count()).map_err(|_| GraphError::GraphTooLarge)?;
        if forest_nodes > budget.max_search_nodes() {
            report.exhausted_limit = Some(CanonicalBudgetLimit::SearchNodes);
            return Ok(inconclusive(finish(report, started)));
        }
        let forest_cells = left
            .vertex_count()
            .checked_mul(6)
            .and_then(|cells| {
                left.incidence_count()
                    .checked_mul(2)
                    .and_then(|incidences| cells.checked_add(incidences))
            })
            .ok_or(GraphError::GraphTooLarge)?;
        if forest_cells > budget.max_retained_state_cells() {
            report.exhausted_limit = Some(CanonicalBudgetLimit::RetainedStateCells);
            return Ok(inconclusive(finish(report, started)));
        }
        let forest_bytes = forest_cells
            .checked_mul(core::mem::size_of::<usize>())
            .ok_or(GraphError::GraphTooLarge)?;
        report.peak_tracked_bytes = forest_bytes;
        if forest_bytes > budget.max_retained_bytes() {
            report.exhausted_limit = Some(CanonicalBudgetLimit::RetainedBytes);
            return Ok(inconclusive(finish(report, started)));
        }
        let Some(candidate) =
            joint_forest_mapping(left, right, &left_blocks.adjacency, &right_blocks.adjacency)?
        else {
            return Ok(different(
                DifferenceWitness::TreeForest,
                finish(report, started),
            ));
        };
        report.explored_nodes = forest_nodes;
        if budget
            .max_elapsed()
            .is_some_and(|limit| started.elapsed() >= limit)
        {
            report.exhausted_limit = Some(CanonicalBudgetLimit::ElapsedTime);
            return Ok(inconclusive(finish(report, started)));
        }
        let mapping = VerifiedGraphMapping::verify(left, right, &candidate)?;
        return Ok(isomorphic(mapping, finish(report, started)));
    }

    let partition = joint_stable_partition(left, right, &left_blocks, &right_blocks)?;
    report.refinement_passes = partition.passes;
    let Some((left_colors, right_colors)) = partition.colors else {
        return Ok(different(
            DifferenceWitness::StablePartition {
                pass: partition.passes,
            },
            finish(report, started),
        ));
    };

    paired_search(
        left,
        right,
        &left_colors,
        &right_colors,
        budget,
        started,
        report,
    )
}

fn paired_search(
    left: &IncidenceGraph,
    right: &IncidenceGraph,
    left_colors: &[usize],
    right_colors: &[usize],
    budget: CanonicalSearchBudget,
    started: Instant,
    mut report: PairedComparisonReport,
) -> Result<GraphComparison, GraphError> {
    let count = left.vertex_count();
    let base_cells = count.checked_mul(3).ok_or(GraphError::GraphTooLarge)?;
    if base_cells > budget.max_retained_state_cells() {
        report.exhausted_limit = Some(CanonicalBudgetLimit::RetainedStateCells);
        return Ok(inconclusive(finish(report, started)));
    }
    let base_bytes = count
        .checked_mul(core::mem::size_of::<usize>() * 2 + core::mem::size_of::<bool>())
        .ok_or(GraphError::GraphTooLarge)?;
    report.peak_tracked_bytes = base_bytes;
    if base_bytes > budget.max_retained_bytes() {
        report.exhausted_limit = Some(CanonicalBudgetLimit::RetainedBytes);
        return Ok(inconclusive(finish(report, started)));
    }

    let mut left_to_right = vec![usize::MAX; count];
    let mut right_used = vec![false; count];
    let Some(first) = select_frame(
        left,
        right,
        left_colors,
        right_colors,
        &left_to_right,
        &right_used,
    )?
    else {
        let empty = Vec::<VertexId>::new();
        let mapping = VerifiedGraphMapping::verify(left, right, &empty)?;
        return Ok(isomorphic(mapping, finish(report, started)));
    };
    report.candidate_pairs = report
        .candidate_pairs
        .checked_add(u64::try_from(first.candidates.len()).map_err(|_| GraphError::GraphTooLarge)?)
        .ok_or(GraphError::GraphTooLarge)?;
    let mut stack = vec![first];

    loop {
        let retained_cells = base_cells
            .checked_add(
                stack
                    .iter()
                    .map(|frame| frame.candidates.capacity())
                    .sum::<usize>(),
            )
            .ok_or(GraphError::GraphTooLarge)?;
        if retained_cells > budget.max_retained_state_cells() {
            report.exhausted_limit = Some(CanonicalBudgetLimit::RetainedStateCells);
            return Ok(inconclusive(finish(report, started)));
        }
        let tracked_bytes = base_bytes
            .checked_add(
                retained_cells
                    .saturating_sub(base_cells)
                    .saturating_mul(core::mem::size_of::<usize>()),
            )
            .ok_or(GraphError::GraphTooLarge)?;
        report.peak_tracked_bytes = report.peak_tracked_bytes.max(tracked_bytes);
        if tracked_bytes > budget.max_retained_bytes() {
            report.exhausted_limit = Some(CanonicalBudgetLimit::RetainedBytes);
            return Ok(inconclusive(finish(report, started)));
        }
        if budget
            .max_elapsed()
            .is_some_and(|limit| started.elapsed() >= limit)
        {
            report.exhausted_limit = Some(CanonicalBudgetLimit::ElapsedTime);
            return Ok(inconclusive(finish(report, started)));
        }

        let mapped_depth = stack.len();
        let Some(frame) = stack.last_mut() else {
            return Ok(different(
                DifferenceWitness::CandidateSpaceExhausted,
                finish(report, started),
            ));
        };
        if let Some(previous) = frame.assigned.take() {
            left_to_right[frame.left] = usize::MAX;
            right_used[previous] = false;
        }
        let Some(candidate) = frame.candidates.get(frame.next).copied() else {
            stack.pop();
            report.backtrack_count = report
                .backtrack_count
                .checked_add(1)
                .ok_or(GraphError::GraphTooLarge)?;
            continue;
        };
        frame.next += 1;
        if !candidate_feasible(left, right, frame.left, candidate, &left_to_right)? {
            continue;
        }
        if report.explored_nodes >= budget.max_search_nodes() {
            report.exhausted_limit = Some(CanonicalBudgetLimit::SearchNodes);
            return Ok(inconclusive(finish(report, started)));
        }
        if mapped_depth > budget.max_depth() {
            report.exhausted_limit = Some(CanonicalBudgetLimit::SearchDepth);
            return Ok(inconclusive(finish(report, started)));
        }
        report.explored_nodes = report
            .explored_nodes
            .checked_add(1)
            .ok_or(GraphError::GraphTooLarge)?;
        left_to_right[frame.left] = candidate;
        right_used[candidate] = true;
        frame.assigned = Some(candidate);

        if stack.len() == count {
            let candidate = left_to_right
                .iter()
                .copied()
                .map(VertexId::new)
                .collect::<Vec<_>>();
            if let Ok(mapping) = VerifiedGraphMapping::verify(left, right, &candidate) {
                return Ok(isomorphic(mapping, finish(report, started)));
            }
            continue;
        }
        if let Some(next) = select_frame(
            left,
            right,
            left_colors,
            right_colors,
            &left_to_right,
            &right_used,
        )? {
            report.candidate_pairs = report
                .candidate_pairs
                .checked_add(
                    u64::try_from(next.candidates.len()).map_err(|_| GraphError::GraphTooLarge)?,
                )
                .ok_or(GraphError::GraphTooLarge)?;
            stack.push(next);
        } else {
            return Err(GraphError::CanonicalizationInvariantViolation);
        }
    }
}

#[derive(Debug)]
struct DecisionFrame {
    left: usize,
    candidates: Vec<usize>,
    next: usize,
    assigned: Option<usize>,
}

fn select_frame(
    left: &IncidenceGraph,
    right: &IncidenceGraph,
    left_colors: &[usize],
    right_colors: &[usize],
    mapping: &[usize],
    right_used: &[bool],
) -> Result<Option<DecisionFrame>, GraphError> {
    let mut best: Option<DecisionFrame> = None;
    for left_vertex in 0..left.vertex_count() {
        if mapping[left_vertex] != usize::MAX {
            continue;
        }
        let mut candidates = Vec::new();
        for right_vertex in 0..right.vertex_count() {
            if right_used[right_vertex] || left_colors[left_vertex] != right_colors[right_vertex] {
                continue;
            }
            if candidate_feasible(left, right, left_vertex, right_vertex, mapping)? {
                candidates.push(right_vertex);
            }
        }
        if candidates.is_empty() {
            return Ok(Some(DecisionFrame {
                left: left_vertex,
                candidates,
                next: 0,
                assigned: None,
            }));
        }
        if best
            .as_ref()
            .is_none_or(|current| candidates.len() < current.candidates.len())
        {
            best = Some(DecisionFrame {
                left: left_vertex,
                candidates,
                next: 0,
                assigned: None,
            });
        }
    }
    Ok(best)
}

fn candidate_feasible(
    left: &IncidenceGraph,
    right: &IncidenceGraph,
    left_vertex: usize,
    right_vertex: usize,
    mapping: &[usize],
) -> Result<bool, GraphError> {
    if left.vertex_kind(VertexId::new(left_vertex))
        != right.vertex_kind(VertexId::new(right_vertex))
        || left.vertex_label(VertexId::new(left_vertex))
            != right.vertex_label(VertexId::new(right_vertex))
        || arc_bundle(left, left_vertex, left_vertex)?
            != arc_bundle(right, right_vertex, right_vertex)?
    {
        return Ok(false);
    }
    for (mapped_left, &mapped_right) in mapping.iter().enumerate() {
        if mapped_right == usize::MAX {
            continue;
        }
        if arc_bundle(left, left_vertex, mapped_left)?
            != arc_bundle(right, right_vertex, mapped_right)?
            || arc_bundle(left, mapped_left, left_vertex)?
                != arc_bundle(right, mapped_right, right_vertex)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

struct JointPartition {
    colors: Option<(Vec<usize>, Vec<usize>)>,
    passes: u64,
}

fn joint_stable_partition(
    left: &IncidenceGraph,
    right: &IncidenceGraph,
    left_blocks: &BlockCutSummary,
    right_blocks: &BlockCutSummary,
) -> Result<JointPartition, GraphError> {
    let mut left_keys = initial_keys(left, left_blocks)?;
    let mut right_keys = initial_keys(right, right_blocks)?;
    let (mut left_colors, mut right_colors) = joint_colors(&left_keys, &right_keys)?;
    if color_histogram(&left_colors) != color_histogram(&right_colors) {
        return Ok(JointPartition {
            colors: None,
            passes: 0,
        });
    }
    let mut passes = 0_u64;
    loop {
        passes = passes.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
        left_keys = refinement_keys(left, &left_colors)?;
        right_keys = refinement_keys(right, &right_colors)?;
        let (next_left, next_right) = joint_colors(&left_keys, &right_keys)?;
        if color_histogram(&next_left) != color_histogram(&next_right) {
            return Ok(JointPartition {
                colors: None,
                passes,
            });
        }
        if next_left == left_colors && next_right == right_colors {
            return Ok(JointPartition {
                colors: Some((left_colors, right_colors)),
                passes,
            });
        }
        left_colors = next_left;
        right_colors = next_right;
    }
}

fn initial_keys(
    graph: &IncidenceGraph,
    blocks: &BlockCutSummary,
) -> Result<Vec<Vec<u8>>, GraphError> {
    (0..graph.vertex_count())
        .map(|vertex| {
            let mut key = vertex_token(graph, vertex)?;
            key.push(u8::from(blocks.articulation[vertex]));
            append_usize(&mut key, blocks.incident_block_sizes[vertex].len())?;
            for size in &blocks.incident_block_sizes[vertex] {
                append_usize(&mut key, *size)?;
            }
            Ok(key)
        })
        .collect()
}

fn refinement_keys(graph: &IncidenceGraph, colors: &[usize]) -> Result<Vec<Vec<u8>>, GraphError> {
    let mut keys = Vec::with_capacity(graph.vertex_count());
    for vertex in 0..graph.vertex_count() {
        let mut key = vec![1];
        append_usize(&mut key, colors[vertex])?;
        append_incidence_colors(
            &mut key,
            graph,
            graph.outgoing(VertexId::new(vertex)),
            colors,
        )?;
        append_incidence_colors(
            &mut key,
            graph,
            graph.incoming(VertexId::new(vertex)),
            colors,
        )?;
        keys.push(key);
    }
    Ok(keys)
}

fn append_incidence_colors(
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

fn joint_colors(
    left: &[Vec<u8>],
    right: &[Vec<u8>],
) -> Result<(Vec<usize>, Vec<usize>), GraphError> {
    let mut unique = left.iter().chain(right).cloned().collect::<Vec<_>>();
    unique.sort_unstable();
    unique.dedup();
    let convert = |keys: &[Vec<u8>]| -> Result<Vec<usize>, GraphError> {
        keys.iter()
            .map(|key| {
                unique
                    .binary_search(key)
                    .map_err(|_| GraphError::CanonicalizationInvariantViolation)
            })
            .collect()
    };
    Ok((convert(left)?, convert(right)?))
}

fn color_histogram(colors: &[usize]) -> Vec<(usize, usize)> {
    let mut sorted = colors.to_vec();
    sorted.sort_unstable();
    let mut histogram = Vec::new();
    for color in sorted {
        if let Some((last, count)) = histogram.last_mut() {
            if *last == color {
                *count += 1;
                continue;
            }
        }
        histogram.push((color, 1));
    }
    histogram
}

#[derive(Debug)]
struct BlockCutSummary {
    adjacency: Vec<Vec<usize>>,
    blocks: Vec<Vec<usize>>,
    articulation: Vec<bool>,
    incident_block_sizes: Vec<Vec<usize>>,
    profile: Vec<u8>,
    component_count: usize,
    is_forest: bool,
}

impl BlockCutSummary {
    fn build(graph: &IncidenceGraph, adjacency: Vec<Vec<usize>>) -> Result<Self, GraphError> {
        let edge_count = adjacency.iter().map(Vec::len).sum::<usize>() / 2;
        let component_count = component_count(&adjacency);
        let is_forest = edge_count == graph.vertex_count().saturating_sub(component_count);
        let (mut blocks, articulation) = if is_forest {
            let mut blocks = Vec::new();
            for (source, neighbors) in adjacency.iter().enumerate() {
                for &target in neighbors {
                    if source < target {
                        blocks.push(vec![source, target]);
                    }
                }
            }
            let articulation = adjacency
                .iter()
                .map(|neighbors| neighbors.len() > 1)
                .collect();
            (blocks, articulation)
        } else {
            biconnected_blocks(&adjacency)?
        };
        for (vertex, neighbors) in adjacency.iter().enumerate() {
            if neighbors.is_empty() {
                blocks.push(vec![vertex]);
            }
        }
        let mut incident_block_sizes = vec![Vec::new(); graph.vertex_count()];
        for block in &blocks {
            for &vertex in block {
                incident_block_sizes[vertex].push(block.len());
            }
        }
        for sizes in &mut incident_block_sizes {
            sizes.sort_unstable();
        }
        let profile = block_profile(graph, &blocks, &articulation)?;
        Ok(Self {
            adjacency,
            blocks,
            articulation,
            incident_block_sizes,
            profile,
            component_count,
            is_forest,
        })
    }
}

fn undirected_support(graph: &IncidenceGraph) -> Vec<Vec<usize>> {
    let mut sets = vec![BTreeSet::new(); graph.vertex_count()];
    for source in 0..graph.vertex_count() {
        for incidence in graph.outgoing(VertexId::new(source)) {
            let target = incidence.neighbor().index();
            if source != target {
                sets[source].insert(target);
                sets[target].insert(source);
            }
        }
    }
    sets.into_iter()
        .map(|set| set.into_iter().collect())
        .collect()
}

fn component_count(adjacency: &[Vec<usize>]) -> usize {
    let mut seen = vec![false; adjacency.len()];
    let mut count = 0;
    for root in 0..adjacency.len() {
        if seen[root] {
            continue;
        }
        count += 1;
        seen[root] = true;
        let mut stack = vec![root];
        while let Some(vertex) = stack.pop() {
            for &neighbor in &adjacency[vertex] {
                if !seen[neighbor] {
                    seen[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
    }
    count
}

struct TarjanState {
    discovery: Vec<usize>,
    low: Vec<usize>,
    parent: Vec<usize>,
    next_time: usize,
    edge_stack: Vec<(usize, usize)>,
    blocks: Vec<Vec<usize>>,
    articulation: Vec<bool>,
}

fn biconnected_blocks(
    adjacency: &[Vec<usize>],
) -> Result<(Vec<Vec<usize>>, Vec<bool>), GraphError> {
    let count = adjacency.len();
    let mut state = TarjanState {
        discovery: vec![usize::MAX; count],
        low: vec![0; count],
        parent: vec![usize::MAX; count],
        next_time: 0,
        edge_stack: Vec::new(),
        blocks: Vec::new(),
        articulation: vec![false; count],
    };
    let mut child_count = vec![0_usize; count];
    for root in 0..count {
        if state.discovery[root] != usize::MAX || adjacency[root].is_empty() {
            continue;
        }
        state.discovery[root] = state.next_time;
        state.low[root] = state.next_time;
        state.next_time = state
            .next_time
            .checked_add(1)
            .ok_or(GraphError::GraphTooLarge)?;
        let mut dfs = vec![(root, 0_usize)];
        while let Some((vertex, next_neighbor)) = dfs.last_mut() {
            if *next_neighbor < adjacency[*vertex].len() {
                let neighbor = adjacency[*vertex][*next_neighbor];
                *next_neighbor += 1;
                if state.discovery[neighbor] == usize::MAX {
                    child_count[*vertex] += 1;
                    state.parent[neighbor] = *vertex;
                    state.edge_stack.push((*vertex, neighbor));
                    state.discovery[neighbor] = state.next_time;
                    state.low[neighbor] = state.next_time;
                    state.next_time = state
                        .next_time
                        .checked_add(1)
                        .ok_or(GraphError::GraphTooLarge)?;
                    dfs.push((neighbor, 0));
                } else if neighbor != state.parent[*vertex]
                    && state.discovery[neighbor] < state.discovery[*vertex]
                {
                    state.low[*vertex] = state.low[*vertex].min(state.discovery[neighbor]);
                    state.edge_stack.push((*vertex, neighbor));
                }
                continue;
            }
            let (finished, _) = dfs
                .pop()
                .ok_or(GraphError::CanonicalizationInvariantViolation)?;
            let parent = state.parent[finished];
            if parent != usize::MAX {
                state.low[parent] = state.low[parent].min(state.low[finished]);
                if state.low[finished] >= state.discovery[parent] {
                    if state.parent[parent] != usize::MAX || child_count[parent] > 1 {
                        state.articulation[parent] = true;
                    }
                    state.pop_block(Some((parent, finished)));
                }
            }
        }
        if !state.edge_stack.is_empty() {
            state.pop_block(None);
        }
    }
    Ok((state.blocks, state.articulation))
}

impl TarjanState {
    fn pop_block(&mut self, stop: Option<(usize, usize)>) {
        let mut vertices = BTreeSet::new();
        while let Some(edge) = self.edge_stack.pop() {
            vertices.insert(edge.0);
            vertices.insert(edge.1);
            if stop == Some(edge) {
                break;
            }
        }
        if !vertices.is_empty() {
            self.blocks.push(vertices.into_iter().collect());
        }
    }
}

fn block_profile(
    graph: &IncidenceGraph,
    blocks: &[Vec<usize>],
    articulation: &[bool],
) -> Result<Vec<u8>, GraphError> {
    let mut descriptors = Vec::with_capacity(blocks.len());
    for block in blocks {
        let members = block.iter().copied().collect::<BTreeSet<_>>();
        let mut descriptor = Vec::new();
        append_usize(&mut descriptor, block.len())?;
        append_usize(
            &mut descriptor,
            block.iter().filter(|&&vertex| articulation[vertex]).count(),
        )?;
        let mut vertex_tokens = block
            .iter()
            .map(|&vertex| vertex_token(graph, vertex))
            .collect::<Result<Vec<_>, _>>()?;
        vertex_tokens.sort_unstable();
        for token in vertex_tokens {
            append_framed(&mut descriptor, &token)?;
        }
        let mut arcs = Vec::new();
        for &source in block {
            for incidence in graph.outgoing(VertexId::new(source)) {
                if !members.contains(&incidence.neighbor().index()) {
                    continue;
                }
                let relation = graph.relation(incidence.relation());
                let mut arc = Vec::new();
                append_framed(&mut arc, graph.vertex_label(VertexId::new(source)))?;
                append_framed(&mut arc, graph.vertex_label(incidence.neighbor()))?;
                append_framed(&mut arc, relation.relation())?;
                append_framed(&mut arc, relation.role())?;
                arc.extend_from_slice(&incidence.multiplicity().to_be_bytes());
                arcs.push(arc);
            }
        }
        arcs.sort_unstable();
        for arc in arcs {
            append_framed(&mut descriptor, &arc)?;
        }
        descriptors.push(descriptor);
    }
    descriptors.sort_unstable();
    let mut profile = Vec::new();
    append_usize(&mut profile, descriptors.len())?;
    for descriptor in descriptors {
        append_framed(&mut profile, &descriptor)?;
    }
    Ok(profile)
}

#[derive(Clone, Copy, Debug)]
enum TreeRoot {
    Single(usize),
    Double(usize, usize),
}

#[derive(Debug)]
struct PreparedForest {
    parent: Vec<usize>,
    postorder: Vec<usize>,
    roots: Vec<TreeRoot>,
}

fn joint_forest_mapping(
    left: &IncidenceGraph,
    right: &IncidenceGraph,
    left_adjacency: &[Vec<usize>],
    right_adjacency: &[Vec<usize>],
) -> Result<Option<Vec<VertexId>>, GraphError> {
    let left_prepared = prepare_forest(left_adjacency)?;
    let right_prepared = prepare_forest(right_adjacency)?;
    let mut interner = BTreeMap::<Vec<u8>, usize>::new();
    let mut next_rank = 0_usize;
    let left_ranks = intern_subtrees(
        left,
        left_adjacency,
        &left_prepared,
        &mut interner,
        &mut next_rank,
    )?;
    let right_ranks = intern_subtrees(
        right,
        right_adjacency,
        &right_prepared,
        &mut interner,
        &mut next_rank,
    )?;
    let mut left_components =
        forest_component_records(left, left_adjacency, &left_prepared, &left_ranks)?;
    let mut right_components =
        forest_component_records(right, right_adjacency, &right_prepared, &right_ranks)?;
    left_components.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    right_components.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    if left_components
        .iter()
        .map(|component| &component.0)
        .ne(right_components.iter().map(|component| &component.0))
    {
        return Ok(None);
    }
    let left_order = forest_order(
        left,
        left_adjacency,
        &left_prepared,
        &left_ranks,
        &left_components,
    )?;
    let right_order = forest_order(
        right,
        right_adjacency,
        &right_prepared,
        &right_ranks,
        &right_components,
    )?;
    Ok(Some(mapping_from_orders(
        left.vertex_count(),
        &left_order,
        &right_order,
    )?))
}

fn prepare_forest(adjacency: &[Vec<usize>]) -> Result<PreparedForest, GraphError> {
    let mut parent = vec![usize::MAX; adjacency.len()];
    let mut postorder = Vec::with_capacity(adjacency.len());
    let mut roots = Vec::new();
    for component in components(adjacency) {
        let centers = tree_centers(&component, adjacency);
        match centers.as_slice() {
            [root] => {
                roots.push(TreeRoot::Single(*root));
                orient_tree(*root, usize::MAX, adjacency, &mut parent, &mut postorder);
            }
            [left, right] => {
                roots.push(TreeRoot::Double(*left, *right));
                orient_tree(*left, *right, adjacency, &mut parent, &mut postorder);
                orient_tree(*right, *left, adjacency, &mut parent, &mut postorder);
            }
            _ => return Err(GraphError::CanonicalizationInvariantViolation),
        }
    }
    Ok(PreparedForest {
        parent,
        postorder,
        roots,
    })
}

fn orient_tree(
    root: usize,
    excluded_parent: usize,
    adjacency: &[Vec<usize>],
    parent: &mut [usize],
    postorder: &mut Vec<usize>,
) {
    parent[root] = usize::MAX;
    let mut traversal = Vec::new();
    let mut pending = vec![root];
    while let Some(vertex) = pending.pop() {
        traversal.push(vertex);
        for &neighbor in &adjacency[vertex] {
            if neighbor == parent[vertex] || (vertex == root && neighbor == excluded_parent) {
                continue;
            }
            parent[neighbor] = vertex;
            pending.push(neighbor);
        }
    }
    postorder.extend(traversal.into_iter().rev());
}

fn intern_subtrees(
    graph: &IncidenceGraph,
    adjacency: &[Vec<usize>],
    prepared: &PreparedForest,
    interner: &mut BTreeMap<Vec<u8>, usize>,
    next_rank: &mut usize,
) -> Result<Vec<usize>, GraphError> {
    let mut ranks = vec![usize::MAX; graph.vertex_count()];
    for &vertex in &prepared.postorder {
        let key = subtree_key(graph, adjacency, prepared, &ranks, vertex)?;
        let rank = if let Some(rank) = interner.get(&key) {
            *rank
        } else {
            let rank = *next_rank;
            *next_rank = next_rank.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
            interner.insert(key, rank);
            rank
        };
        ranks[vertex] = rank;
    }
    Ok(ranks)
}

fn subtree_key(
    graph: &IncidenceGraph,
    adjacency: &[Vec<usize>],
    prepared: &PreparedForest,
    ranks: &[usize],
    vertex: usize,
) -> Result<Vec<u8>, GraphError> {
    let mut children = Vec::new();
    for &child in &adjacency[vertex] {
        if prepared.parent[child] != vertex {
            continue;
        }
        let mut entry = edge_token(graph, vertex, child)?;
        append_usize(&mut entry, ranks[child])?;
        children.push(entry);
    }
    children.sort_unstable();
    let mut key = vertex_token(graph, vertex)?;
    append_usize(&mut key, children.len())?;
    for child in children {
        append_framed(&mut key, &child)?;
    }
    Ok(key)
}

type ComponentRecord = (Vec<u8>, Vec<usize>);

fn forest_component_records(
    graph: &IncidenceGraph,
    _adjacency: &[Vec<usize>],
    prepared: &PreparedForest,
    ranks: &[usize],
) -> Result<Vec<ComponentRecord>, GraphError> {
    let mut records = Vec::with_capacity(prepared.roots.len());
    for root in &prepared.roots {
        match *root {
            TreeRoot::Single(vertex) => {
                let mut key = vec![1];
                append_usize(&mut key, ranks[vertex])?;
                records.push((key, vec![vertex]));
            }
            TreeRoot::Double(left, right) => {
                let mut forward = vec![2];
                append_usize(&mut forward, ranks[left])?;
                append_framed(&mut forward, &edge_token(graph, left, right)?)?;
                append_usize(&mut forward, ranks[right])?;
                let mut reverse = vec![2];
                append_usize(&mut reverse, ranks[right])?;
                append_framed(&mut reverse, &edge_token(graph, right, left)?)?;
                append_usize(&mut reverse, ranks[left])?;
                if forward <= reverse {
                    records.push((forward, vec![left, right]));
                } else {
                    records.push((reverse, vec![right, left]));
                }
            }
        }
    }
    Ok(records)
}

fn forest_order(
    graph: &IncidenceGraph,
    adjacency: &[Vec<usize>],
    prepared: &PreparedForest,
    ranks: &[usize],
    components: &[ComponentRecord],
) -> Result<Vec<VertexId>, GraphError> {
    let mut order = Vec::with_capacity(graph.vertex_count());
    for (_, roots) in components {
        for &root in roots {
            append_rooted_order(graph, adjacency, prepared, ranks, root, &mut order)?;
        }
    }
    Ok(order)
}

fn append_rooted_order(
    graph: &IncidenceGraph,
    adjacency: &[Vec<usize>],
    prepared: &PreparedForest,
    ranks: &[usize],
    root: usize,
    output: &mut Vec<VertexId>,
) -> Result<(), GraphError> {
    let mut pending = vec![root];
    while let Some(vertex) = pending.pop() {
        output.push(VertexId::new(vertex));
        let mut children = adjacency[vertex]
            .iter()
            .copied()
            .filter(|&child| prepared.parent[child] == vertex)
            .map(|child| {
                let mut key = edge_token(graph, vertex, child)?;
                append_usize(&mut key, ranks[child])?;
                Ok((key, child))
            })
            .collect::<Result<Vec<_>, GraphError>>()?;
        children.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        pending.extend(children.into_iter().rev().map(|(_, child)| child));
    }
    Ok(())
}

fn components(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut seen = vec![false; adjacency.len()];
    let mut result = Vec::new();
    for root in 0..adjacency.len() {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut stack = vec![root];
        let mut component = Vec::new();
        while let Some(vertex) = stack.pop() {
            component.push(vertex);
            for &neighbor in &adjacency[vertex] {
                if !seen[neighbor] {
                    seen[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
        result.push(component);
    }
    result
}

fn tree_centers(component: &[usize], adjacency: &[Vec<usize>]) -> Vec<usize> {
    if component.len() <= 2 {
        let mut centers = component.to_vec();
        centers.sort_unstable();
        return centers;
    }
    let mut degree = vec![0_usize; adjacency.len()];
    let mut leaves = Vec::new();
    for &vertex in component {
        degree[vertex] = adjacency[vertex].len();
        if degree[vertex] <= 1 {
            leaves.push(vertex);
        }
    }
    let mut remaining = component.len();
    while remaining > 2 {
        remaining -= leaves.len();
        let mut next = Vec::new();
        for leaf in leaves {
            degree[leaf] = 0;
            for &neighbor in &adjacency[leaf] {
                if degree[neighbor] > 0 {
                    degree[neighbor] -= 1;
                    if degree[neighbor] == 1 {
                        next.push(neighbor);
                    }
                }
            }
        }
        leaves = next;
    }
    leaves.sort_unstable();
    leaves
}

fn edge_token(graph: &IncidenceGraph, source: usize, target: usize) -> Result<Vec<u8>, GraphError> {
    let mut token = vec![3];
    let forward = arc_bundle(graph, source, target)?;
    let reverse = arc_bundle(graph, target, source)?;
    append_framed(&mut token, &forward)?;
    append_framed(&mut token, &reverse)?;
    Ok(token)
}

fn vertex_descriptor_profile(
    graph: &IncidenceGraph,
    support: &[Vec<usize>],
) -> Result<Vec<u8>, GraphError> {
    let mut descriptors = Vec::with_capacity(graph.vertex_count());
    for (vertex, neighbors) in support.iter().enumerate() {
        let id = VertexId::new(vertex);
        let outgoing = graph.outgoing(id);
        let incoming = graph.incoming(id);
        let mut descriptor = vertex_token(graph, vertex)?;
        append_usize(&mut descriptor, neighbors.len())?;
        append_usize(&mut descriptor, outgoing.len())?;
        append_usize(&mut descriptor, incoming.len())?;
        descriptor.extend_from_slice(&incidence_multiplicity(outgoing)?.to_be_bytes());
        descriptor.extend_from_slice(&incidence_multiplicity(incoming)?.to_be_bytes());
        descriptors.push(descriptor);
    }
    descriptors.sort_unstable();
    let mut profile = Vec::new();
    append_usize(&mut profile, descriptors.len())?;
    for descriptor in descriptors {
        append_framed(&mut profile, &descriptor)?;
    }
    Ok(profile)
}

fn incidence_multiplicity(incidences: &[Incidence]) -> Result<u64, GraphError> {
    incidences.iter().try_fold(0_u64, |total, incidence| {
        total
            .checked_add(incidence.multiplicity())
            .ok_or(GraphError::MultiplicityOverflow)
    })
}

fn vertex_token(graph: &IncidenceGraph, vertex: usize) -> Result<Vec<u8>, GraphError> {
    let id = VertexId::new(vertex);
    let mut token = vec![graph.vertex_kind(id) as u8];
    append_framed(&mut token, graph.vertex_label(id))?;
    append_framed(&mut token, &arc_bundle(graph, vertex, vertex)?)?;
    Ok(token)
}

fn arc_bundle(graph: &IncidenceGraph, source: usize, target: usize) -> Result<Vec<u8>, GraphError> {
    let mut records = Vec::new();
    for incidence in graph.outgoing(VertexId::new(source)) {
        if incidence.neighbor().index() != target {
            continue;
        }
        let descriptor = graph.relation(incidence.relation());
        let mut record = Vec::new();
        append_framed(&mut record, descriptor.relation())?;
        append_framed(&mut record, descriptor.role())?;
        record.extend_from_slice(&incidence.multiplicity().to_be_bytes());
        records.push(record);
    }
    records.sort_unstable();
    let mut bundle = Vec::new();
    append_usize(&mut bundle, records.len())?;
    for record in records {
        append_framed(&mut bundle, &record)?;
    }
    Ok(bundle)
}

fn mapping_from_orders(
    count: usize,
    left: &[VertexId],
    right: &[VertexId],
) -> Result<Vec<VertexId>, GraphError> {
    if left.len() != count || right.len() != count {
        return Err(GraphError::CanonicalizationInvariantViolation);
    }
    let mut mapping = vec![VertexId::new(0); count];
    for (&left_vertex, &right_vertex) in left.iter().zip(right) {
        mapping[left_vertex.index()] = right_vertex;
    }
    Ok(mapping)
}

fn append_usize(output: &mut Vec<u8>, value: usize) -> Result<(), GraphError> {
    output.extend_from_slice(
        &u64::try_from(value)
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_be_bytes(),
    );
    Ok(())
}

fn append_framed(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), GraphError> {
    append_usize(output, bytes.len())?;
    output.extend_from_slice(bytes);
    Ok(())
}

fn first_difference(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right)
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| left.len().min(right.len()))
}

fn finish(mut report: PairedComparisonReport, started: Instant) -> PairedComparisonReport {
    report.elapsed = started.elapsed();
    report
}

fn report(paired: PairedComparisonReport) -> GraphComparisonReport {
    GraphComparisonReport {
        left: None,
        right: None,
        paired: Some(paired),
    }
}

fn different(witness: DifferenceWitness, paired: PairedComparisonReport) -> GraphComparison {
    GraphComparison::Different {
        witness,
        report: report(paired),
    }
}

fn isomorphic(mapping: VerifiedGraphMapping, paired: PairedComparisonReport) -> GraphComparison {
    GraphComparison::Isomorphic {
        mapping,
        report: report(paired),
    }
}

fn inconclusive(paired: PairedComparisonReport) -> GraphComparison {
    GraphComparison::Inconclusive {
        report: report(paired),
    }
}
