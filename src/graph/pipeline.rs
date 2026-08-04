//! Adaptive fail-closed filtering from constant-time metadata to exact matching.

use std::time::Instant;

use microfield::{CanonicalEncoding, Field, Pow, StaticField};

use crate::structural::StructuralEncoder;

use super::{
    signature::exact_degree_histograms_equal, CanonicalSearchBudget, FastGraphLabeler,
    GraphComparison, GraphError, IncidenceGraph, LocalPairRefinementProfile, LoopPatternCatalog,
    Microcanon, PairRefinementStatus, PatternAnalysisStatus, RefinementProfile,
    VerifiedGraphMapping,
};

/// Ordered tiers; a ceiling includes every cheaper tier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum AdaptiveFilterTier {
    /// Vertex/incidence/multiplicity counters.
    Metadata = 1,
    /// Exact sparse degree histograms.
    Degree = 2,
    /// Fixed-round finite-field refinement.
    FieldRefinement = 3,
    /// Exact budget-admitted L0--L3 pattern counts.
    Patterns = 4,
    /// Ambiguity-localized pair refinement.
    LocalPairRefinement = 5,
    /// Field-independent exact paired matching.
    Exact = 6,
}

/// Admission limits and maximum assurance requested.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AdaptiveFilterPolicy {
    ceiling: AdaptiveFilterTier,
    pattern_maximum_work: u64,
    pair_rounds: u8,
    pair_maximum_work: u64,
    exact_budget: CanonicalSearchBudget,
}

impl AdaptiveFilterPolicy {
    /// Creates a fully resolving bounded policy.
    #[must_use]
    pub const fn new(
        pattern_maximum_work: u64,
        pair_rounds: u8,
        pair_maximum_work: u64,
        exact_budget: CanonicalSearchBudget,
    ) -> Self {
        Self {
            ceiling: AdaptiveFilterTier::Exact,
            pattern_maximum_work,
            pair_rounds,
            pair_maximum_work,
            exact_budget,
        }
    }

    /// Stops after `ceiling`; equality below exact remains inconclusive.
    #[must_use]
    pub const fn with_ceiling(mut self, ceiling: AdaptiveFilterTier) -> Self {
        self.ceiling = ceiling;
        self
    }

    /// Highest admitted tier.
    #[must_use]
    pub const fn ceiling(self) -> AdaptiveFilterTier {
        self.ceiling
    }
    /// Exact-pattern work ceiling.
    #[must_use]
    pub const fn pattern_maximum_work(self) -> u64 {
        self.pattern_maximum_work
    }
    /// Local pair-refinement rounds.
    #[must_use]
    pub const fn pair_rounds(self) -> u8 {
        self.pair_rounds
    }
    /// Local pair-refinement work ceiling.
    #[must_use]
    pub const fn pair_maximum_work(self) -> u64 {
        self.pair_maximum_work
    }
    /// Exact paired-search budget.
    #[must_use]
    pub const fn exact_budget(self) -> CanonicalSearchBudget {
        self.exact_budget
    }
}

impl Default for AdaptiveFilterPolicy {
    fn default() -> Self {
        Self::new(
            2_000_000,
            2,
            25_000_000,
            CanonicalSearchBudget::new(1_000_000),
        )
    }
}

/// Observable cost of one attempted tier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdaptiveTierReport {
    tier: AdaptiveFilterTier,
    elapsed_nanoseconds: u64,
    estimated_work: u64,
    skipped: bool,
}

impl AdaptiveTierReport {
    /// Attempted tier.
    #[must_use]
    pub const fn tier(self) -> AdaptiveFilterTier {
        self.tier
    }
    /// Saturating wall time; never part of identity.
    #[must_use]
    pub const fn elapsed_nanoseconds(self) -> u64 {
        self.elapsed_nanoseconds
    }
    /// Deterministic tier-specific estimate, or zero when unavailable.
    #[must_use]
    pub const fn estimated_work(self) -> u64 {
        self.estimated_work
    }
    /// Atomic preflight skipped the tier without partial evidence.
    #[must_use]
    pub const fn skipped(self) -> bool {
        self.skipped
    }
}

/// Ordered trace of a comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptiveFilterReport {
    terminal_tier: AdaptiveFilterTier,
    tiers: Vec<AdaptiveTierReport>,
}

impl AdaptiveFilterReport {
    /// Rejecting, exact, or requested ceiling tier.
    #[must_use]
    pub const fn terminal_tier(&self) -> AdaptiveFilterTier {
        self.terminal_tier
    }
    /// Every tier actually attempted, in order.
    #[must_use]
    pub fn tiers(&self) -> &[AdaptiveTierReport] {
        &self.tiers
    }
}

/// Fail-closed result: heuristics can reject but never prove isomorphism.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdaptiveFilterOutcome {
    /// A complete necessary invariant differed.
    Different { report: AdaptiveFilterReport },
    /// Exact matching found and verified a bijection.
    Isomorphic {
        mapping: VerifiedGraphMapping,
        report: AdaptiveFilterReport,
    },
    /// A ceiling, skip or exact budget prevented a definitive statement.
    Inconclusive { report: AdaptiveFilterReport },
}

impl AdaptiveFilterOutcome {
    /// Common execution trace.
    #[must_use]
    pub const fn report(&self) -> &AdaptiveFilterReport {
        match self {
            Self::Different { report }
            | Self::Isomorphic { report, .. }
            | Self::Inconclusive { report } => report,
        }
    }
}

/// Reusable strategy facade over all filtering tiers.
#[derive(Clone, Debug)]
pub struct AdaptiveGraphPipeline<F, E, const K: usize>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    labeler: FastGraphLabeler<F, E, K>,
    microcanon: Microcanon,
    policy: AdaptiveFilterPolicy,
}

impl<F, E, const K: usize> AdaptiveGraphPipeline<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField + Pow,
    E: StructuralEncoder<F> + Clone,
{
    /// Builds one immutable pipeline. Pair rounds must be in `1..=8`.
    pub fn new(
        labeler: FastGraphLabeler<F, E, K>,
        microcanon: Microcanon,
        policy: AdaptiveFilterPolicy,
    ) -> Result<Self, GraphError> {
        if K == 0 || !(1..=8).contains(&policy.pair_rounds) {
            return Err(GraphError::InvalidAdaptiveFilterPolicy);
        }
        Ok(Self {
            labeler,
            microcanon,
            policy,
        })
    }

    /// Current immutable admission policy.
    #[must_use]
    pub const fn policy(&self) -> AdaptiveFilterPolicy {
        self.policy
    }

    /// Escalates only after equality at every cheaper complete tier.
    pub fn compare(
        &self,
        left: &IncidenceGraph,
        right: &IncidenceGraph,
    ) -> Result<AdaptiveFilterOutcome, GraphError> {
        let mut tiers = Vec::with_capacity(6);
        let started = Instant::now();
        let equal = left.vertex_count() == right.vertex_count()
            && left.incidence_count() == right.incidence_count()
            && left.total_multiplicity() == right.total_multiplicity();
        push(&mut tiers, AdaptiveFilterTier::Metadata, started, 3, false);
        if !equal {
            return Ok(different(AdaptiveFilterTier::Metadata, tiers));
        }
        if self.policy.ceiling == AdaptiveFilterTier::Metadata {
            return Ok(inconclusive(AdaptiveFilterTier::Metadata, tiers));
        }

        let started = Instant::now();
        let equal = exact_degree_histograms_equal(left, right)?;
        let linear_work = to_u64(left.vertex_count()).saturating_add(to_u64(right.vertex_count()));
        push(
            &mut tiers,
            AdaptiveFilterTier::Degree,
            started,
            linear_work,
            false,
        );
        if !equal {
            return Ok(different(AdaptiveFilterTier::Degree, tiers));
        }
        if self.policy.ceiling == AdaptiveFilterTier::Degree {
            return Ok(inconclusive(AdaptiveFilterTier::Degree, tiers));
        }

        let started = Instant::now();
        let equal =
            self.labeler.analyze(left)?.signature() == self.labeler.analyze(right)?.signature();
        let rounds = match self.labeler.profile() {
            RefinementProfile::Fast { rounds } => rounds,
            RefinementProfile::Robust { maximum_rounds, .. } => maximum_rounds,
        };
        push(
            &mut tiers,
            AdaptiveFilterTier::FieldRefinement,
            started,
            linear_work.saturating_mul(to_u64(rounds)),
            false,
        );
        if !equal {
            return Ok(different(AdaptiveFilterTier::FieldRefinement, tiers));
        }
        if self.policy.ceiling == AdaptiveFilterTier::FieldRefinement {
            return Ok(inconclusive(AdaptiveFilterTier::FieldRefinement, tiers));
        }

        let started = Instant::now();
        let catalog = LoopPatternCatalog::l0_to_l3();
        let lp = catalog.analyze(left, self.policy.pattern_maximum_work)?;
        let rp = catalog.analyze(right, self.policy.pattern_maximum_work)?;
        let skipped = lp.status() != PatternAnalysisStatus::Complete
            || rp.status() != PatternAnalysisStatus::Complete;
        let work = lp.estimated_work().saturating_add(rp.estimated_work());
        let equal = skipped || lp == rp;
        push(
            &mut tiers,
            AdaptiveFilterTier::Patterns,
            started,
            work,
            skipped,
        );
        if !equal {
            return Ok(different(AdaptiveFilterTier::Patterns, tiers));
        }
        if self.policy.ceiling == AdaptiveFilterTier::Patterns {
            return Ok(inconclusive(AdaptiveFilterTier::Patterns, tiers));
        }

        let started = Instant::now();
        let lp = LocalPairRefinementProfile::analyze(
            left,
            self.policy.pair_rounds,
            self.policy.pair_maximum_work,
        )?;
        let rp = LocalPairRefinementProfile::analyze(
            right,
            self.policy.pair_rounds,
            self.policy.pair_maximum_work,
        )?;
        let skipped = lp.status() != PairRefinementStatus::Complete
            || rp.status() != PairRefinementStatus::Complete;
        let work = lp.estimated_work().saturating_add(rp.estimated_work());
        let equal = skipped || lp == rp;
        push(
            &mut tiers,
            AdaptiveFilterTier::LocalPairRefinement,
            started,
            work,
            skipped,
        );
        if !equal {
            return Ok(different(AdaptiveFilterTier::LocalPairRefinement, tiers));
        }
        if self.policy.ceiling == AdaptiveFilterTier::LocalPairRefinement {
            return Ok(inconclusive(AdaptiveFilterTier::LocalPairRefinement, tiers));
        }

        let started = Instant::now();
        let exact = self
            .microcanon
            .compare(left, right, self.policy.exact_budget)?;
        push(
            &mut tiers,
            AdaptiveFilterTier::Exact,
            started,
            exact_work(&exact),
            false,
        );
        let report = AdaptiveFilterReport {
            terminal_tier: AdaptiveFilterTier::Exact,
            tiers,
        };
        Ok(match exact {
            GraphComparison::Different { .. } => AdaptiveFilterOutcome::Different { report },
            GraphComparison::Isomorphic { mapping, .. } => {
                AdaptiveFilterOutcome::Isomorphic { mapping, report }
            }
            GraphComparison::Inconclusive { .. } => AdaptiveFilterOutcome::Inconclusive { report },
        })
    }
}

fn different(
    terminal_tier: AdaptiveFilterTier,
    tiers: Vec<AdaptiveTierReport>,
) -> AdaptiveFilterOutcome {
    AdaptiveFilterOutcome::Different {
        report: AdaptiveFilterReport {
            terminal_tier,
            tiers,
        },
    }
}
fn inconclusive(
    terminal_tier: AdaptiveFilterTier,
    tiers: Vec<AdaptiveTierReport>,
) -> AdaptiveFilterOutcome {
    AdaptiveFilterOutcome::Inconclusive {
        report: AdaptiveFilterReport {
            terminal_tier,
            tiers,
        },
    }
}
fn push(
    reports: &mut Vec<AdaptiveTierReport>,
    tier: AdaptiveFilterTier,
    started: Instant,
    estimated_work: u64,
    skipped: bool,
) {
    reports.push(AdaptiveTierReport {
        tier,
        elapsed_nanoseconds: u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        estimated_work,
        skipped,
    });
}
fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
fn exact_work(comparison: &GraphComparison) -> u64 {
    let report = match comparison {
        GraphComparison::Different { report, .. }
        | GraphComparison::Isomorphic { report, .. }
        | GraphComparison::Inconclusive { report } => report,
    };
    report
        .paired()
        .map_or(0_u64, |v| v.explored_nodes())
        .saturating_add(report.left().map_or(0, |v| v.explored_nodes()))
        .saturating_add(report.right().map_or(0, |v| v.explored_nodes()))
}
