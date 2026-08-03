//! Compatibility facade for exact graph diagnostics and canonization.
//!
//! The finite-field labeler remains a fast analysis channel. Exact forms are
//! delegated to the field-independent [`super::Microcanon`] core so changing a
//! field, encoder, lane count or refinement profile cannot change canonical
//! graph bytes.

use std::collections::BTreeSet;
use std::time::Duration;

use microfield::{CanonicalEncoding, Field, Pow, StaticField};

use crate::structural::StructuralEncoder;

use super::canon::{exact_stable_partition, Microcanon, MicrocanonOutcome, MicrocanonPath};
use super::labeler::{DiscreteCanonicalForm, FastGraphAnalysis, FastGraphLabeler};
use super::{CanonicalBudgetLimit, CanonicalSearchBudget, GraphError, IncidenceGraph};

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

    /// Number of stable exact relational-refinement classes.
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

    /// Whether exact descriptors separate vertices collapsed by the field profile.
    #[must_use]
    pub const fn has_field_aliasing(&self) -> bool {
        self.field_aliasing_cell_count != 0
    }

    /// Whether exact relational refinement is itself non-discrete.
    #[must_use]
    pub const fn has_local_ambiguity(&self) -> bool {
        self.exact_refinement_cell_count != self.vertex_count
    }
}

/// Route used by a successful exact canonicalization.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CanonicalizationPath {
    /// The field analysis and exact refinement were both already discrete.
    FastDiscrete,
    /// Exact relational refinement repaired a collision in the fast profile.
    ExactRefinementDiscrete,
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
    peak_tracked_bytes: usize,
    verified_automorphism_count: u64,
    orbit_pruned_child_count: u64,
    prefix_pruned_leaf_count: u64,
    elapsed: Duration,
    exhausted_limit: Option<CanonicalBudgetLimit>,
}

impl CanonicalSearchReport {
    /// Exact diagnosis that selected the canonicalization route.
    #[must_use]
    pub const fn degeneracy(&self) -> &GraphDegeneracyReport {
        &self.degeneracy
    }

    /// Exact route taken by this request.
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

    /// Peak logical bytes retained by controlled exact-search buffers.
    #[must_use]
    pub const fn peak_tracked_bytes(&self) -> usize {
        self.peak_tracked_bytes
    }

    /// Non-identity automorphisms verified by the exact core.
    #[must_use]
    pub const fn verified_automorphism_count(&self) -> u64 {
        self.verified_automorphism_count
    }

    /// Branches removed through verified stabilizer orbits.
    #[must_use]
    pub const fn orbit_pruned_child_count(&self) -> u64 {
        self.orbit_pruned_child_count
    }

    /// Leaves whose exact vertex prefix already exceeded the incumbent.
    #[must_use]
    pub const fn prefix_pruned_leaf_count(&self) -> u64 {
        self.prefix_pruned_leaf_count
    }

    /// End-to-end wall-clock duration, including output verification.
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
    /// This is an opt-in control-plane operation and never changes the fast
    /// analysis or publishes a canonical claim.
    pub fn diagnose_degeneracy(
        &self,
        graph: &IncidenceGraph,
    ) -> Result<GraphDegeneracyReport, GraphError> {
        let analysis = self.analyze(graph)?;
        let (exact_partition, _) = exact_stable_partition(graph)?;
        Ok(build_degeneracy_report(&analysis, &exact_partition))
    }

    /// Produces an exact, profile-independent representative under a hard budget.
    ///
    /// This compatibility method retains the historical report types while
    /// delegating all exact work to [`Microcanon`]. Budget exhaustion never
    /// publishes a best-so-far candidate.
    pub fn canonicalize_exact(
        &self,
        graph: &IncidenceGraph,
        budget: CanonicalSearchBudget,
    ) -> Result<ExactCanonicalOutcome, GraphError> {
        let analysis = self.analyze(graph)?;
        let run = Microcanon::default().canonicalize_run(graph, budget)?;
        let degeneracy = build_degeneracy_report(&analysis, &run.root_partition);
        let fast_discrete = analysis.cell_count() == graph.vertex_count();

        match run.outcome {
            MicrocanonOutcome::Exact { form, report } => {
                let report = adapt_report(degeneracy, fast_discrete, &report);
                Ok(ExactCanonicalOutcome::Exact { form, report })
            }
            MicrocanonOutcome::Inconclusive { report } => {
                let report = adapt_report(degeneracy, fast_discrete, &report);
                Ok(ExactCanonicalOutcome::BudgetExhausted { report })
            }
        }
    }
}

fn adapt_report(
    degeneracy: GraphDegeneracyReport,
    fast_discrete: bool,
    report: &super::MicrocanonReport,
) -> CanonicalSearchReport {
    let path = if fast_discrete && report.path() == MicrocanonPath::ExactRefinementDiscrete {
        CanonicalizationPath::FastDiscrete
    } else {
        match report.path() {
            MicrocanonPath::ExactRefinementDiscrete => {
                CanonicalizationPath::ExactRefinementDiscrete
            }
            MicrocanonPath::WeakComponentDecomposition => {
                CanonicalizationPath::WeakComponentDecomposition
            }
            MicrocanonPath::IndividualizationRefinement => {
                CanonicalizationPath::IndividualizationRefinement
            }
        }
    };
    CanonicalSearchReport {
        degeneracy,
        path,
        explored_nodes: report.explored_nodes(),
        leaf_count: report.leaf_count(),
        individualization_count: report.individualization_count(),
        exact_refinement_passes: report.exact_refinement_passes(),
        maximum_depth: report.maximum_depth(),
        peak_retained_state_cells: report.peak_retained_state_cells(),
        peak_tracked_bytes: report.peak_tracked_bytes(),
        verified_automorphism_count: report.verified_automorphism_count(),
        orbit_pruned_child_count: report.orbit_pruned_child_count(),
        prefix_pruned_leaf_count: report.prefix_pruned_leaf_count(),
        elapsed: report.elapsed(),
        exhausted_limit: report.exhausted_limit(),
    }
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
