//! Exact, field-independent graph canonization and mapping verification.

use core::mem::size_of;
use microfield::{CanonicalEncoding, Field, Pow, StaticField};
use std::time::Instant;

mod compact;
mod encoding;
mod mapping;
mod paired;
mod search;

pub use compact::MicrocanonWorkspace;
pub use encoding::{
    CanonicalGraphDocument, CanonicalGraphEncodingId, CanonicalGraphForm, CanonicalGraphKey,
};
pub use mapping::VerifiedGraphMapping;
pub use paired::{PairedComparisonPath, PairedComparisonReport};
pub use search::{
    CanonicalBudgetLimit, CanonicalSearchBudget, MicrocanonOutcome, MicrocanonPath,
    MicrocanonReport,
};

/// Exact engine selected behind the stable Microcanon facade.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum MicrocanonStrategy {
    /// G10 compact refinement with certified orbit and prefix pruning.
    #[default]
    Compact,
    /// Allocation-heavy G9 baseline retained for differential verification.
    ///
    /// Node/frontier limits are cooperative. Newer depth/byte limits suppress
    /// publication after the reference run; use `Compact` for early stopping.
    Reference,
}

use super::{
    FastGraphLabeler, GraphError, GraphSchemaId, GraphSignatureId, IncidenceGraph, VertexId,
};
use crate::structural::StructuralEncoder;
use search::CanonicalizationRun;

/// Exact reason why two graphs cannot be isomorphic under one schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DifferenceWitness {
    /// Vertex counts differ.
    VertexCount {
        /// Left count.
        left: usize,
        /// Right count.
        right: usize,
    },
    /// Normalized directed record counts differ.
    IncidenceCount {
        /// Left count.
        left: usize,
        /// Right count.
        right: usize,
    },
    /// Total exact multiplicities differ.
    TotalMultiplicity {
        /// Left total.
        left: u64,
        /// Right total.
        right: u64,
    },
    /// Exact vertex kinds or labels have different multiplicities.
    VertexDescriptors {
        /// First differing byte in the sorted exact descriptor profile.
        first_differing_byte: usize,
    },
    /// Joint exact refinement produced different cell multiplicities.
    StablePartition {
        /// Refinement pass at which the exact histograms diverged.
        pass: u64,
    },
    /// Exact articulation/block membership profiles differ.
    BlockCutProfile {
        /// First differing byte in the exact sorted profile.
        first_differing_byte: usize,
    },
    /// Both inputs are forests and their exact relational tree codes differ.
    TreeForest,
    /// A compatible finite-field channel differs.
    FiniteFieldEvidence {
        /// Field, encoder, lanes, parameters and refinement-profile identity.
        signature_id: GraphSignatureId,
    },
    /// Exact paired candidate search exhausted every possible mapping.
    CandidateSpaceExhausted,
    /// Complete exact canonical forms differ.
    CanonicalForms {
        /// First differing byte, or the common length if one is a prefix.
        first_differing_byte: usize,
    },
}

/// Work reports retained by a two-graph comparison.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GraphComparisonReport {
    left: Option<MicrocanonReport>,
    right: Option<MicrocanonReport>,
    paired: Option<PairedComparisonReport>,
}

impl GraphComparisonReport {
    /// Left canonization report when exact search was reached.
    #[must_use]
    pub const fn left(&self) -> Option<&MicrocanonReport> {
        self.left.as_ref()
    }

    /// Right canonization report when exact search was reached.
    #[must_use]
    pub const fn right(&self) -> Option<&MicrocanonReport> {
        self.right.as_ref()
    }

    /// Direct paired-matcher report when G12 avoided two canonizations.
    #[must_use]
    pub const fn paired(&self) -> Option<&PairedComparisonReport> {
        self.paired.as_ref()
    }
}

/// Fail-closed result of comparing two normalized graphs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphComparison {
    /// A necessary invariant or complete canonical form differs.
    Different {
        /// Exact reason observed by this execution.
        witness: DifferenceWitness,
        /// Exact work performed after cheap metadata checks.
        report: GraphComparisonReport,
    },
    /// A complete mapping was found and independently verified.
    Isomorphic {
        /// Exact left-to-right bijection and inverse.
        mapping: VerifiedGraphMapping,
        /// Exact work performed.
        report: GraphComparisonReport,
    },
    /// The shared budget ended before a complete statement was possible.
    Inconclusive {
        /// Partial-work reports; no mapping or candidate form is published.
        report: GraphComparisonReport,
    },
}

/// Exact canonization facade for one application schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Microcanon {
    schema_id: GraphSchemaId,
    strategy: MicrocanonStrategy,
}

impl Microcanon {
    /// Creates a canonizer whose exact bytes are bound to `schema_id`.
    #[must_use]
    pub const fn new(schema_id: GraphSchemaId) -> Self {
        Self {
            schema_id,
            strategy: MicrocanonStrategy::Compact,
        }
    }

    /// Application schema embedded into every exact form.
    #[must_use]
    pub const fn schema_id(self) -> GraphSchemaId {
        self.schema_id
    }

    /// Selects the optimized engine or the retained G9 differential baseline.
    #[must_use]
    pub const fn with_strategy(mut self, strategy: MicrocanonStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Exact engine configured for this facade.
    #[must_use]
    pub const fn strategy(self) -> MicrocanonStrategy {
        self.strategy
    }

    /// Produces an exact profile-independent form or fails closed on budget.
    ///
    /// # Errors
    ///
    /// Returns normalization, stable-size or encoding failures. Budget
    /// exhaustion is represented by [`MicrocanonOutcome::Inconclusive`].
    pub fn canonicalize(
        &self,
        graph: &IncidenceGraph,
        budget: CanonicalSearchBudget,
    ) -> Result<MicrocanonOutcome, GraphError> {
        Ok(self.canonicalize_run(graph, budget)?.outcome)
    }

    /// Canonicalizes with reusable compact-refinement storage.
    ///
    /// # Errors
    ///
    /// Rejects the G9 reference strategy and propagates the same exact graph
    /// errors as [`Microcanon::canonicalize`].
    pub fn canonicalize_with_workspace(
        &self,
        graph: &IncidenceGraph,
        budget: CanonicalSearchBudget,
        workspace: &mut MicrocanonWorkspace,
    ) -> Result<MicrocanonOutcome, GraphError> {
        if self.strategy != MicrocanonStrategy::Compact {
            return Err(GraphError::IncompatibleCanonicalWorkspace);
        }
        let started = Instant::now();
        let run = compact::canonicalize_with_workspace(graph, self.schema_id, budget, workspace)?;
        Ok(finalize_run(graph, budget, started, run)?.outcome)
    }

    /// Compares two graphs with one shared node budget.
    ///
    /// A returned mapping is always rechecked against the complete normalized
    /// model. Equality of a fingerprint is never used as a positive result.
    ///
    /// # Errors
    ///
    /// Returns stable-size, encoding and internal mapping failures.
    pub fn compare(
        &self,
        left: &IncidenceGraph,
        right: &IncidenceGraph,
        budget: CanonicalSearchBudget,
    ) -> Result<GraphComparison, GraphError> {
        let empty_report = GraphComparisonReport::default();
        if left.vertex_count() != right.vertex_count() {
            return Ok(GraphComparison::Different {
                witness: DifferenceWitness::VertexCount {
                    left: left.vertex_count(),
                    right: right.vertex_count(),
                },
                report: empty_report,
            });
        }
        if left.incidence_count() != right.incidence_count() {
            return Ok(GraphComparison::Different {
                witness: DifferenceWitness::IncidenceCount {
                    left: left.incidence_count(),
                    right: right.incidence_count(),
                },
                report: empty_report,
            });
        }
        if left.total_multiplicity() != right.total_multiplicity() {
            return Ok(GraphComparison::Different {
                witness: DifferenceWitness::TotalMultiplicity {
                    left: left.total_multiplicity(),
                    right: right.total_multiplicity(),
                },
                report: empty_report,
            });
        }

        paired::compare(left, right, budget)
    }

    /// Computes one finite-field profile before exact paired matching.
    ///
    /// A differing invariant is a sound negative witness. Equality only falls
    /// through to the field-independent exact matcher.
    ///
    /// # Errors
    ///
    /// Propagates field encoding and exact paired-comparison failures.
    pub fn compare_with_field_profile<F, E, const K: usize>(
        &self,
        left: &IncidenceGraph,
        right: &IncidenceGraph,
        labeler: &FastGraphLabeler<F, E, K>,
        budget: CanonicalSearchBudget,
    ) -> Result<GraphComparison, GraphError>
    where
        F: Field + CanonicalEncoding + StaticField + Pow,
        E: StructuralEncoder<F>,
    {
        let left_profile = labeler.analyze(left)?;
        let right_profile = labeler.analyze(right)?;
        if left_profile.signature() != right_profile.signature() {
            return Ok(GraphComparison::Different {
                witness: DifferenceWitness::FiniteFieldEvidence {
                    signature_id: labeler.signature_id(),
                },
                report: GraphComparisonReport::default(),
            });
        }
        self.compare(left, right, budget)
    }

    pub(crate) fn canonicalize_run(
        &self,
        graph: &IncidenceGraph,
        budget: CanonicalSearchBudget,
    ) -> Result<CanonicalizationRun, GraphError> {
        let started = Instant::now();
        let mut run = match self.strategy {
            MicrocanonStrategy::Compact => compact::canonicalize(graph, self.schema_id, budget)?,
            MicrocanonStrategy::Reference => search::canonicalize(graph, self.schema_id, budget)?,
        };
        if self.strategy == MicrocanonStrategy::Reference {
            enforce_reference_limits(&mut run, budget)?;
        }
        finalize_run(graph, budget, started, run)
    }
}

impl Default for Microcanon {
    fn default() -> Self {
        Self::new(GraphSchemaId::default())
    }
}

pub(crate) use search::exact_stable_partition;

fn enforce_reference_limits(
    run: &mut CanonicalizationRun,
    budget: CanonicalSearchBudget,
) -> Result<(), GraphError> {
    let limit = match &mut run.outcome {
        MicrocanonOutcome::Exact { form, report } => {
            let tracked = report
                .peak_retained_state_cells()
                .checked_mul(size_of::<usize>())
                .and_then(|bytes| bytes.checked_add(form.bytes().len()))
                .and_then(|bytes| {
                    bytes.checked_add(
                        form.original_to_canonical()
                            .len()
                            .checked_mul(size_of::<VertexId>())?,
                    )
                })
                .and_then(|bytes| {
                    bytes.checked_add(
                        form.canonical_to_original()
                            .len()
                            .checked_mul(size_of::<VertexId>())?,
                    )
                })
                .ok_or(GraphError::GraphTooLarge)?;
            report.peak_tracked_bytes = report.peak_tracked_bytes.max(tracked);
            if report.maximum_depth() > budget.max_depth() {
                Some(CanonicalBudgetLimit::SearchDepth)
            } else if tracked > budget.max_retained_bytes() {
                Some(CanonicalBudgetLimit::RetainedBytes)
            } else {
                None
            }
        }
        MicrocanonOutcome::Inconclusive { .. } => None,
    };
    if let Some(limit) = limit {
        let mut report = run.outcome.report().clone();
        report.exhausted_limit = Some(limit);
        run.outcome = MicrocanonOutcome::Inconclusive { report };
    }
    Ok(())
}

fn verify_published_form(
    source: &IncidenceGraph,
    form: &CanonicalGraphForm,
) -> Result<(), GraphError> {
    let document = form.decode()?;
    if form.schema_id() != document.schema_id() {
        return Err(GraphError::InvalidCanonicalEncoding);
    }
    VerifiedGraphMapping::verify(source, document.graph(), form.original_to_canonical())?;
    for (original, canonical) in form.original_to_canonical().iter().copied().enumerate() {
        if form.canonical_to_original()[canonical.index()] != VertexId::new(original) {
            return Err(GraphError::InvalidCanonicalOrder);
        }
    }
    Ok(())
}

fn finalize_run(
    graph: &IncidenceGraph,
    budget: CanonicalSearchBudget,
    started: Instant,
    mut run: CanonicalizationRun,
) -> Result<CanonicalizationRun, GraphError> {
    if let MicrocanonOutcome::Exact { form, .. } = &run.outcome {
        verify_published_form(graph, form)?;
    }
    let elapsed = started.elapsed();
    run.outcome.report_mut().elapsed = elapsed;
    if matches!(&run.outcome, MicrocanonOutcome::Exact { .. })
        && budget.max_elapsed().is_some_and(|limit| elapsed >= limit)
    {
        let mut report = run.outcome.report().clone();
        report.exhausted_limit = Some(CanonicalBudgetLimit::ElapsedTime);
        run.outcome = MicrocanonOutcome::Inconclusive { report };
    }
    Ok(run)
}
