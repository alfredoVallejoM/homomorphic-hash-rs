//! Compatibility façade from the historical bipartite API to the maintained
//! finite-field graph engine.

use microfield::{CanonicalEncoding, Fp251V1};

use crate::{
    algebra::{galois_256::GaloisSignature256, traits::FiniteField},
    graph::{
        from_legacy_topology, F251GraphLabeler, FastGraphAnalysis, GraphError, IncidenceGraph,
        RefinementProfile,
    },
    topology::bloom_l1::{TopoBloomMask, TopologicalMask},
};

const LEGACY_LANES: usize = 3;
const MAX_COMPATIBILITY_ROUNDS: usize = 64;

/// Legacy bipartite interface for cellular-complex experiments.
pub trait TopologyProvider {
    fn num_variables(&self) -> usize;
    fn num_clauses(&self) -> usize;
    fn variables_in_clause(&self, clause_index: usize) -> Vec<usize>;
    fn clauses_for_variable(&self, variable_index: usize) -> Vec<usize>;

    /// Provides an optional variable seed, such as atom type or gate polarity.
    fn initial_state(&self, _variable_index: usize) -> Option<GaloisSignature256> {
        None
    }
}

/// Output record retained for source compatibility.
///
/// `signature` packs the three canonical F251 lane bytes into the low bytes of
/// the historical 256-bit container. `bloom_mask` remains a non-authoritative,
/// index-dependent prefilter; it is not part of the maintained graph identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalNode {
    pub original_index: usize,
    pub signature: GaloisSignature256,
    pub bloom_mask: TopoBloomMask,
}

/// Full maintained result produced through a legacy provider.
///
/// Entity vertices occupy `0..variable_count`; clause/hyperedge vertices are
/// retained after them instead of being flattened into pairwise edges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyGraphAnalysis {
    graph: IncidenceGraph,
    structural: FastGraphAnalysis<Fp251V1, LEGACY_LANES>,
    variable_count: usize,
}

impl LegacyGraphAnalysis {
    /// Exact normalized incidence graph used by the new engine.
    #[must_use]
    pub const fn graph(&self) -> &IncidenceGraph {
        &self.graph
    }

    /// Generic F251 structural analysis, including clause vertices and the
    /// composable graph signature.
    #[must_use]
    pub const fn structural(&self) -> &FastGraphAnalysis<Fp251V1, LEGACY_LANES> {
        &self.structural
    }

    /// Number of entity vertices supplied by the historical provider.
    #[must_use]
    pub const fn variable_count(&self) -> usize {
        self.variable_count
    }
}

/// Deprecated name retained as a façade over [`F251GraphLabeler`].
pub struct CellularGaloisCanonizer;

impl CellularGaloisCanonizer {
    /// Converts a legacy provider and runs the maintained generic graph engine.
    ///
    /// # Errors
    ///
    /// Rejects invalid provider indices and graph/encoding overflows before
    /// returning any partial analysis. Zero rounds and values above the
    /// compatibility ceiling are rejected by this precise API.
    pub fn try_analyze<T: TopologyProvider + ?Sized>(
        provider: &T,
        rounds: usize,
    ) -> Result<LegacyGraphAnalysis, GraphError> {
        if rounds == 0 || rounds > MAX_COMPATIBILITY_ROUNDS {
            return Err(GraphError::InvalidProfile);
        }
        let variable_count = provider.num_variables();
        let graph = from_legacy_topology(provider)?;
        let labeler = F251GraphLabeler::<LEGACY_LANES>::f251(RefinementProfile::Fast { rounds })?;
        let structural = labeler.analyze(&graph)?;
        Ok(LegacyGraphAnalysis {
            graph,
            structural,
            variable_count,
        })
    }

    /// Produces compatibility records while executing the maintained engine.
    ///
    /// New code should call [`Self::try_analyze`] and consume its identified
    /// field signature directly. To preserve the infallible historical shape,
    /// this adapter maps zero rounds to one and caps extreme requests at 64.
    #[must_use]
    pub fn canonize<T: TopologyProvider + ?Sized>(
        provider: &T,
        requested_rounds: usize,
    ) -> Vec<CanonicalNode> {
        let rounds = requested_rounds.clamp(1, MAX_COMPATIBILITY_ROUNDS);
        let analysis = Self::try_analyze(provider, rounds)
            .expect("legacy topology must contain only valid variable indices");
        let masks = compatibility_masks(provider, analysis.variable_count, rounds);
        analysis.structural.labels()[..analysis.variable_count]
            .iter()
            .enumerate()
            .zip(masks)
            .map(|((original_index, label), bloom_mask)| {
                let mut bytes = [0_u8; 32];
                for (target, lane) in bytes.iter_mut().zip(label.lanes()) {
                    *target = lane.to_canonical()[0];
                }
                CanonicalNode {
                    original_index,
                    signature: GaloisSignature256::from_bytes_canonical(&bytes),
                    bloom_mask,
                }
            })
            .collect()
    }
}

/// Synchronous compatibility prefilter. It is deliberately kept outside the
/// field recurrence so index-based Bloom collisions cannot affect signatures.
fn compatibility_masks<T: TopologyProvider + ?Sized>(
    provider: &T,
    variable_count: usize,
    rounds: usize,
) -> Vec<TopoBloomMask> {
    let mut current: Vec<_> = (0..variable_count)
        .map(TopoBloomMask::from_variable_index)
        .collect();
    let mut next = current.clone();
    for _ in 0..rounds {
        for (variable, output) in next.iter_mut().enumerate() {
            let mut mask = current[variable];
            for clause in provider.clauses_for_variable(variable) {
                for neighbor in provider.variables_in_clause(clause) {
                    if let Some(neighbor_mask) = current.get(neighbor) {
                        mask = mask.union(neighbor_mask);
                    }
                }
            }
            *output = mask;
        }
        core::mem::swap(&mut current, &mut next);
    }
    current
}
