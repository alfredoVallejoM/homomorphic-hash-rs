//! Deterministic closure campaign for adaptive G13 filtering and G14 deltas.

use std::collections::BTreeMap;

use homomorphic_hash_rs::{
    AdaptiveFilterOutcome, AdaptiveFilterPolicy, AdaptiveGraphPipeline, FastGraphLabeler,
    GraphDelta, GraphDeltaPolicy, GraphDeltaUpdatePath, IncidenceGraph, IncidenceGraphBuilder,
    IncrementalGraphWorkspace, Microcanon, PrimeIntegerEncoder, RefinementProfile,
    VerifiedGraphMapping, VertexId,
};
use microfield::Fp251V1;
use serde::Serialize;

use crate::model::ValidationManifest;

const DOMAIN: u64 = 0x4731_3347_3134_0001;

/// Stable final acceptance metrics; wall-clock values are deliberately absent.
#[derive(Clone, Debug, Serialize)]
pub struct G13G14CampaignReport {
    /// Report schema.
    pub schema_version: u32,
    /// Manifest seed retained for provenance.
    pub seed: u64,
    /// Terminal tier counts over fixed negative and positive cases.
    pub terminal_tiers: BTreeMap<String, u64>,
    /// Exact mappings independently verified by the exact engine.
    pub verified_isomorphisms: u64,
    /// Unexpected positive or negative decisions.
    pub wrong_decisions: u64,
    /// Incremental and fallback route counts.
    pub delta_routes: BTreeMap<String, u64>,
    /// Differential delta/full-rebuild equalities.
    pub differential_updates: u64,
    /// Maximum exact input rows audited by a one-vertex local delta.
    pub maximum_local_audited_vertices: u64,
}

/// Runs frozen tier-routing and transactional differential gates.
pub fn run_campaign(manifest: &ValidationManifest) -> Result<G13G14CampaignReport, String> {
    let labeler = labeler()?;
    let pipeline = AdaptiveGraphPipeline::new(
        labeler.clone(),
        Microcanon::default(),
        AdaptiveFilterPolicy::default(),
    )
    .map_err(debug_error)?;
    let cases = [
        (path(5)?, path(6)?, false),
        (path(6)?, star(6)?, false),
        (cycles(&[6])?, cycles(&[3, 3])?, false),
        (cycles(&[7])?, reverse_relabel(&cycles(&[7])?)?, true),
    ];
    let mut terminal_tiers = BTreeMap::new();
    let mut verified_isomorphisms = 0_u64;
    let mut wrong_decisions = 0_u64;
    for (left, right, expected_isomorphic) in cases {
        let outcome = pipeline.compare(&left, &right).map_err(debug_error)?;
        increment(
            &mut terminal_tiers,
            format!("{:?}", outcome.report().terminal_tier()),
        )?;
        match (&outcome, expected_isomorphic) {
            (AdaptiveFilterOutcome::Isomorphic { mapping, .. }, true) => {
                VerifiedGraphMapping::verify(&left, &right, mapping.left_to_right())
                    .map_err(debug_error)?;
                verified_isomorphisms += 1;
            }
            (AdaptiveFilterOutcome::Different { .. }, false) => {}
            _ => wrong_decisions += 1,
        }
    }

    let mut delta_routes = BTreeMap::new();
    let mut differential_updates = 0_u64;
    let mut maximum_local_audited_vertices = 0_u64;
    for (index, ratio) in [(0_usize, 1_000_u16), (32, 1)] {
        let mut state = labeler.incremental_state(path(64)?).map_err(debug_error)?;
        let mut workspace = IncrementalGraphWorkspace::new();
        let mut delta = GraphDelta::new().with_expected_revision(0);
        delta
            .set_vertex_label(
                VertexId::new(index),
                format!("changed-{index}").into_bytes(),
            )
            .map_err(debug_error)?;
        let report = labeler
            .apply_delta(
                &mut state,
                &delta,
                GraphDeltaPolicy::new(8, ratio).map_err(debug_error)?,
                &mut workspace,
            )
            .map_err(debug_error)?;
        increment(&mut delta_routes, format!("{:?}", report.path()))?;
        if report.path() == GraphDeltaUpdatePath::IncrementalCone {
            maximum_local_audited_vertices = maximum_local_audited_vertices.max(
                u64::try_from(
                    report
                        .incremental()
                        .map_or(0, |value| value.audited_vertices()),
                )
                .unwrap_or(u64::MAX),
            );
        }
        if state.analysis().to_owned() != labeler.analyze(state.graph()).map_err(debug_error)? {
            return Err(format!("G14 differential mismatch at vertex {index}"));
        }
        differential_updates += 1;
    }

    if wrong_decisions != 0
        || terminal_tiers.get("Metadata") != Some(&1)
        || terminal_tiers.get("Degree") != Some(&1)
        || terminal_tiers.get("Exact") != Some(&1)
        || delta_routes.get("IncrementalCone") != Some(&1)
        || delta_routes.get("FullRebuild") != Some(&1)
    {
        return Err("G13/G14 routing gate drift".into());
    }
    Ok(G13G14CampaignReport {
        schema_version: 1,
        seed: manifest.seed,
        terminal_tiers,
        verified_isomorphisms,
        wrong_decisions,
        delta_routes,
        differential_updates,
        maximum_local_audited_vertices,
    })
}

fn labeler() -> Result<FastGraphLabeler<Fp251V1, PrimeIntegerEncoder, 3>, String> {
    FastGraphLabeler::new(PrimeIntegerEncoder::new(DOMAIN), RefinementProfile::fast())
        .map_err(debug_error)
}

fn path(order: usize) -> Result<IncidenceGraph, String> {
    edges(
        order,
        &(0..order.saturating_sub(1))
            .map(|i| (i, i + 1))
            .collect::<Vec<_>>(),
        b"path",
    )
}

fn star(order: usize) -> Result<IncidenceGraph, String> {
    edges(
        order,
        &(1..order).map(|i| (0, i)).collect::<Vec<_>>(),
        b"path",
    )
}

fn cycles(lengths: &[usize]) -> Result<IncidenceGraph, String> {
    let mut pairs = Vec::new();
    let mut offset = 0;
    for &length in lengths {
        pairs.extend((0..length).map(|i| (offset + i, offset + (i + 1) % length)));
        offset += length;
    }
    edges(offset, &pairs, b"cycle")
}

fn edges(order: usize, edges: &[(usize, usize)], role: &[u8]) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..order)
        .map(|_| builder.add_vertex(b"v".to_vec()))
        .collect::<Vec<_>>();
    for &(left, right) in edges {
        builder
            .add_undirected_relation(
                vertices[left],
                vertices[right],
                b"e".to_vec(),
                role.to_vec(),
                1,
            )
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

fn increment(values: &mut BTreeMap<String, u64>, key: String) -> Result<(), String> {
    *values.entry(key).or_default() = values
        .get(&key)
        .copied()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("counter overflow")?;
    Ok(())
}

fn debug_error(error: impl core::fmt::Debug) -> String {
    format!("{error:?}")
}
