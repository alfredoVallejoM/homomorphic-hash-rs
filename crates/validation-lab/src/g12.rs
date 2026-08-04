//! Deterministic acceptance campaign for G12 exact paired comparison.

use std::collections::BTreeMap;

use homomorphic_hash_rs::{
    CanonicalSearchBudget, GraphComparison, IncidenceGraph, IncidenceGraphBuilder, Microcanon,
    MicrocanonOutcome, VerifiedGraphMapping, VertexId,
};
use serde::Serialize;

use crate::{graphs::g11_adversarial_pairs, model::ValidationManifest};

/// Reproducible exact-comparison closure report.
#[derive(Clone, Debug, Serialize)]
pub struct G12CampaignReport {
    /// Report schema.
    pub schema_version: u32,
    /// Manifest seed retained for provenance.
    pub seed: u64,
    /// Differential pairs checked against independent canonical forms.
    pub small_differential_pairs: u64,
    /// Independently verified relabeling mappings.
    pub verified_relabelings: u64,
    /// Largest exact forest exercised without recursion.
    pub largest_forest_vertices: u64,
    /// Route counts observed in deterministic acceptance cases.
    pub route_counts: BTreeMap<String, u64>,
    /// Frozen adversarial exact outcomes.
    pub adversarial: Vec<G12AdversarialResult>,
    /// Number of unexpected incomplete outcomes.
    pub inconclusive_outcomes: u64,
}

/// Exact result for one frozen difficult non-isomorphic pair.
#[derive(Clone, Debug, Serialize)]
pub struct G12AdversarialResult {
    /// Frozen family name.
    pub family: String,
    /// Exact negative witness variant.
    pub witness: String,
    /// Paired candidate assignments explored.
    pub explored_nodes: u64,
}

/// Runs deterministic differential, relabeling, forest and adversarial gates.
pub fn run_campaign(manifest: &ValidationManifest) -> Result<G12CampaignReport, String> {
    let canon = Microcanon::default();
    let budget = CanonicalSearchBudget::new(50_000_000)
        .with_max_retained_state_cells(16 * 1024 * 1024)
        .with_max_retained_bytes(256 * 1024 * 1024);
    let mut route_counts = BTreeMap::new();
    let mut inconclusive = 0_u64;
    let mut differential = 0_u64;

    for mask in 0_u64..1024 {
        let reverse = mask.reverse_bits() >> (64 - 10);
        let left = graph_from_mask(5, mask)?;
        let right = graph_from_mask(5, reverse)?;
        let left_form = exact_form(canon.canonicalize(&left, budget).map_err(debug_error)?)?;
        let right_form = exact_form(canon.canonicalize(&right, budget).map_err(debug_error)?)?;
        let expected = left_form == right_form;
        let outcome = canon.compare(&left, &right, budget).map_err(debug_error)?;
        let actual = matches!(outcome, GraphComparison::Isomorphic { .. });
        if matches!(outcome, GraphComparison::Inconclusive { .. }) {
            inconclusive += 1;
        }
        if expected != actual {
            return Err(format!(
                "G12 paired/canonical mismatch for masks {mask:#x} and {reverse:#x}"
            ));
        }
        count_route(&mut route_counts, &outcome)?;
        differential += 1;
    }

    let mut verified_relabelings = 0_u64;
    for order in [1_usize, 2, 3, 16, 257, 4_096] {
        let graph = path(order)?;
        let relabeled = reverse_relabel(&graph)?;
        let outcome = canon
            .compare(&graph, &relabeled, budget)
            .map_err(debug_error)?;
        match &outcome {
            GraphComparison::Isomorphic { mapping, .. } => {
                VerifiedGraphMapping::verify(&graph, &relabeled, mapping.left_to_right())
                    .map_err(debug_error)?;
                verified_relabelings += 1;
            }
            other => return Err(format!("G12 forest relabeling {order} failed: {other:?}")),
        }
        count_route(&mut route_counts, &outcome)?;
    }

    let mut adversarial = Vec::new();
    for (family, left, right) in g11_adversarial_pairs()? {
        let outcome = canon.compare(&left, &right, budget).map_err(debug_error)?;
        match outcome {
            GraphComparison::Different { witness, report } => {
                let explored_nodes = report.paired().map_or(0, |paired| paired.explored_nodes());
                if let Some(paired) = report.paired() {
                    increment(&mut route_counts, format!("{:?}", paired.path()))?;
                }
                adversarial.push(G12AdversarialResult {
                    family: family.into(),
                    witness: format!("{witness:?}"),
                    explored_nodes,
                });
            }
            GraphComparison::Inconclusive { report } => {
                return Err(format!(
                    "G12 adversarial pair {family} exhausted budget: {report:?}"
                ));
            }
            GraphComparison::Isomorphic { .. } => {
                return Err(format!(
                    "G12 adversarial pair {family} was marked isomorphic"
                ));
            }
        }
    }

    Ok(G12CampaignReport {
        schema_version: 1,
        seed: manifest.seed,
        small_differential_pairs: differential,
        verified_relabelings,
        largest_forest_vertices: 4_096,
        route_counts,
        adversarial,
        inconclusive_outcomes: inconclusive,
    })
}

fn exact_form(outcome: MicrocanonOutcome) -> Result<Vec<u8>, String> {
    match outcome {
        MicrocanonOutcome::Exact { form, .. } => Ok(form.bytes().to_vec()),
        MicrocanonOutcome::Inconclusive { report } => Err(format!(
            "canonical differential oracle incomplete: {report:?}"
        )),
    }
}

fn count_route(
    routes: &mut BTreeMap<String, u64>,
    outcome: &GraphComparison,
) -> Result<(), String> {
    let route = match outcome {
        GraphComparison::Different { report, .. }
        | GraphComparison::Isomorphic { report, .. }
        | GraphComparison::Inconclusive { report } => report
            .paired()
            .map(|paired| format!("{:?}", paired.path()))
            .unwrap_or_else(|| "CheapMetadata".into()),
    };
    increment(routes, route)
}

fn increment(routes: &mut BTreeMap<String, u64>, route: String) -> Result<(), String> {
    let count = routes.entry(route).or_default();
    *count = count.checked_add(1).ok_or("G12 route counter overflow")?;
    Ok(())
}

fn graph_from_mask(order: usize, mask: u64) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..order)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect::<Vec<_>>();
    let mut bit = 0;
    for left in 0..order {
        for right in left + 1..order {
            if mask & (1_u64 << bit) != 0 {
                builder
                    .add_undirected_relation(
                        vertices[left],
                        vertices[right],
                        b"edge".to_vec(),
                        Vec::new(),
                        1,
                    )
                    .map_err(debug_error)?;
            }
            bit += 1;
        }
    }
    builder.build().map_err(debug_error)
}

fn path(order: usize) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..order)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect::<Vec<_>>();
    for edge in vertices.windows(2) {
        builder
            .add_undirected_relation(edge[0], edge[1], b"edge".to_vec(), Vec::new(), 1)
            .map_err(debug_error)?;
    }
    builder.build().map_err(debug_error)
}

fn reverse_relabel(graph: &IncidenceGraph) -> Result<IncidenceGraph, String> {
    let count = graph.vertex_count();
    let mut builder = IncidenceGraphBuilder::new();
    for new in 0..count {
        let old = VertexId::new(count - new - 1);
        builder.add_typed_vertex(graph.vertex_kind(old), graph.vertex_label(old).to_vec());
    }
    for source in 0..count {
        for incidence in graph.outgoing(VertexId::new(source)) {
            let descriptor = graph.relation(incidence.relation());
            builder
                .add_directed_relation(
                    VertexId::new(count - source - 1),
                    VertexId::new(count - incidence.neighbor().index() - 1),
                    descriptor.relation().to_vec(),
                    descriptor.role().to_vec(),
                    incidence.multiplicity(),
                )
                .map_err(debug_error)?;
        }
    }
    builder.build().map_err(debug_error)
}

fn debug_error(error: impl core::fmt::Debug) -> String {
    format!("{error:?}")
}
