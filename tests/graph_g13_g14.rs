//! G13 adaptive-filter and G14 transactional-delta acceptance gates.

use homomorphic_hash_rs::{
    AdaptiveFilterOutcome, AdaptiveFilterPolicy, AdaptiveFilterTier, AdaptiveGraphPipeline,
    CanonicalSearchBudget, FastGraphLabeler, GraphDelta, GraphDeltaPolicy, GraphDeltaUpdatePath,
    GraphError, IncidenceGraph, IncidenceGraphBuilder, IncrementalGraphWorkspace,
    LocalPairRefinementProfile, Microcanon, PairRefinementStatus, PrimeIntegerEncoder,
    RefinementProfile, VertexId,
};
use microfield::Fp251V1;

const DOMAIN: u64 = 0x4731_332d_4731_3401;

fn path(order: usize) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..order)
        .map(|_| builder.add_vertex(b"v".to_vec()))
        .collect::<Vec<_>>();
    for edge in vertices.windows(2) {
        builder
            .add_undirected_relation(edge[0], edge[1], b"e".to_vec(), b"path".to_vec(), 1)
            .unwrap();
    }
    builder.build().unwrap()
}

fn star(order: usize) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..order)
        .map(|_| builder.add_vertex(b"v".to_vec()))
        .collect::<Vec<_>>();
    for leaf in 1..order {
        builder
            .add_undirected_relation(
                vertices[0],
                vertices[leaf],
                b"e".to_vec(),
                b"path".to_vec(),
                1,
            )
            .unwrap();
    }
    builder.build().unwrap()
}

fn cycles(lengths: &[usize]) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..lengths.iter().sum())
        .map(|_| builder.add_vertex(b"v".to_vec()))
        .collect::<Vec<_>>();
    let mut offset = 0;
    for &length in lengths {
        for index in 0..length {
            builder
                .add_undirected_relation(
                    vertices[offset + index],
                    vertices[offset + (index + 1) % length],
                    b"e".to_vec(),
                    b"cycle".to_vec(),
                    1,
                )
                .unwrap();
        }
        offset += length;
    }
    builder.build().unwrap()
}

fn relabel(graph: &IncidenceGraph, new_to_old: &[usize]) -> IncidenceGraph {
    let mut old_to_new = vec![usize::MAX; graph.vertex_count()];
    let mut builder = IncidenceGraphBuilder::new();
    for (new, &old) in new_to_old.iter().enumerate() {
        old_to_new[old] = new;
        let vertex = VertexId::new(old);
        builder.add_typed_vertex(
            graph.vertex_kind(vertex),
            graph.vertex_label(vertex).to_vec(),
        );
    }
    for source in 0..graph.vertex_count() {
        for incidence in graph.outgoing(VertexId::new(source)) {
            let descriptor = graph.relation(incidence.relation());
            builder
                .add_directed_relation(
                    VertexId::new(old_to_new[source]),
                    VertexId::new(old_to_new[incidence.neighbor().index()]),
                    descriptor.relation().to_vec(),
                    descriptor.role().to_vec(),
                    incidence.multiplicity(),
                )
                .unwrap();
        }
    }
    builder.build().unwrap()
}

fn labeler() -> FastGraphLabeler<Fp251V1, PrimeIntegerEncoder, 3> {
    FastGraphLabeler::new(PrimeIntegerEncoder::new(DOMAIN), RefinementProfile::fast()).unwrap()
}

fn pipeline(
    policy: AdaptiveFilterPolicy,
) -> AdaptiveGraphPipeline<Fp251V1, PrimeIntegerEncoder, 3> {
    AdaptiveGraphPipeline::new(labeler(), Microcanon::default(), policy).unwrap()
}

#[test]
fn pipeline_rejects_at_the_first_sufficient_tier() {
    let pipeline = pipeline(AdaptiveFilterPolicy::default());
    let metadata = pipeline.compare(&path(5), &path(6)).unwrap();
    assert!(matches!(metadata, AdaptiveFilterOutcome::Different { .. }));
    assert_eq!(
        metadata.report().terminal_tier(),
        AdaptiveFilterTier::Metadata
    );

    let degree = pipeline.compare(&path(6), &star(6)).unwrap();
    assert!(matches!(degree, AdaptiveFilterOutcome::Different { .. }));
    assert_eq!(degree.report().terminal_tier(), AdaptiveFilterTier::Degree);
    assert_eq!(degree.report().tiers().len(), 2);
}

#[test]
fn regular_collision_escalates_and_exact_positive_is_verified() {
    let pipeline = pipeline(AdaptiveFilterPolicy::default());
    let different = pipeline.compare(&cycles(&[6]), &cycles(&[3, 3])).unwrap();
    assert!(matches!(different, AdaptiveFilterOutcome::Different { .. }));
    assert!(different.report().terminal_tier() >= AdaptiveFilterTier::Patterns);

    let graph = cycles(&[7]);
    let permuted = relabel(&graph, &[3, 6, 1, 5, 0, 4, 2]);
    let isomorphic = pipeline.compare(&graph, &permuted).unwrap();
    assert!(matches!(
        isomorphic,
        AdaptiveFilterOutcome::Isomorphic { .. }
    ));
    assert_eq!(
        isomorphic.report().terminal_tier(),
        AdaptiveFilterTier::Exact
    );
}

#[test]
fn ceilings_and_atomic_skips_never_publish_a_false_positive() {
    let policy = AdaptiveFilterPolicy::new(0, 2, 0, CanonicalSearchBudget::new(10))
        .with_ceiling(AdaptiveFilterTier::LocalPairRefinement);
    let outcome = pipeline(policy)
        .compare(&cycles(&[8]), &cycles(&[8]))
        .unwrap();
    assert!(matches!(
        outcome,
        AdaptiveFilterOutcome::Inconclusive { .. }
    ));
    assert!(
        outcome
            .report()
            .tiers()
            .iter()
            .filter(|tier| tier.skipped())
            .count()
            >= 2
    );
}

#[test]
fn localized_pair_profile_is_invariant_and_budget_preflight_is_atomic() {
    let graph = cycles(&[8]);
    let permuted = relabel(&graph, &[7, 2, 5, 0, 6, 3, 1, 4]);
    let left = LocalPairRefinementProfile::analyze(&graph, 2, u64::MAX).unwrap();
    let right = LocalPairRefinementProfile::analyze(&permuted, 2, u64::MAX).unwrap();
    assert_eq!(left, right);
    assert_eq!(left.status(), PairRefinementStatus::Complete);
    let skipped = LocalPairRefinementProfile::analyze(&graph, 2, 0).unwrap();
    assert_eq!(skipped.status(), PairRefinementStatus::SkippedBudget);
    assert!(skipped.color_histogram().is_empty());
    assert!(skipped.rooted_descriptors().is_empty());
}

#[test]
fn graph_delta_local_update_matches_a_complete_reanalysis() {
    let labeler = labeler();
    let mut state = labeler.incremental_state(path(64)).unwrap();
    let mut workspace = IncrementalGraphWorkspace::new();
    let mut delta = GraphDelta::new().with_expected_revision(0);
    delta
        .set_vertex_label(VertexId::new(0), b"changed".to_vec())
        .unwrap();
    let report = labeler
        .apply_delta(
            &mut state,
            &delta,
            GraphDeltaPolicy::new(8, 1_000).unwrap(),
            &mut workspace,
        )
        .unwrap();
    assert_eq!(report.path(), GraphDeltaUpdatePath::IncrementalCone);
    assert_eq!(report.incremental().unwrap().audited_vertices(), 1);
    assert_eq!(report.revision(), 1);
    assert!(report.label_changed());
    assert!(!report.topology_changed());
    assert_eq!(
        state.analysis().to_owned(),
        labeler.analyze(state.graph()).unwrap()
    );
}

#[test]
fn graph_delta_falls_back_early_and_errors_are_transactional() {
    let labeler = labeler();
    let mut state = labeler.incremental_state(path(64)).unwrap();
    let mut workspace = IncrementalGraphWorkspace::new();
    let original_graph = state.graph().clone();
    let mut invalid = GraphDelta::new();
    invalid
        .set_vertex_label(VertexId::new(999), b"bad".to_vec())
        .unwrap();
    assert!(matches!(
        labeler.apply_delta(
            &mut state,
            &invalid,
            GraphDeltaPolicy::default(),
            &mut workspace
        ),
        Err(GraphError::InvalidVertex { .. })
    ));
    assert_eq!(state.graph(), &original_graph);
    assert_eq!(state.revision(), 0);

    let mut delta = GraphDelta::new().with_expected_revision(0);
    delta
        .set_vertex_label(VertexId::new(32), b"changed".to_vec())
        .unwrap();
    let report = labeler
        .apply_delta(
            &mut state,
            &delta,
            GraphDeltaPolicy::new(8, 1).unwrap(),
            &mut workspace,
        )
        .unwrap();
    assert_eq!(report.path(), GraphDeltaUpdatePath::FullRebuild);
    assert_eq!(state.revision(), 1);
    assert_eq!(
        state.analysis().to_owned(),
        labeler.analyze(state.graph()).unwrap()
    );

    let stale = GraphDelta::new().with_expected_revision(0);
    assert!(matches!(
        labeler.apply_delta(
            &mut state,
            &stale,
            GraphDeltaPolicy::default(),
            &mut workspace
        ),
        Err(GraphError::GraphDeltaRevisionMismatch { .. })
    ));
}

#[test]
fn graph_delta_relations_multiplicity_and_noop_are_differentially_exact() {
    let labeler = labeler();
    let mut state = labeler.incremental_state(path(16)).unwrap();
    let mut workspace = IncrementalGraphWorkspace::new();

    let empty = GraphDelta::new().with_expected_revision(0);
    let report = labeler
        .apply_delta(
            &mut state,
            &empty,
            GraphDeltaPolicy::default(),
            &mut workspace,
        )
        .unwrap();
    assert!(!report.label_changed());
    assert!(!report.topology_changed());
    assert_eq!(report.path(), GraphDeltaUpdatePath::NoChange);
    assert_eq!(state.revision(), 0);

    let mut remove = GraphDelta::new().with_expected_revision(0);
    remove
        .remove_directed_relation(
            VertexId::new(7),
            VertexId::new(8),
            b"e".to_vec(),
            b"path".to_vec(),
            1,
        )
        .unwrap();
    let report = labeler
        .apply_delta(
            &mut state,
            &remove,
            GraphDeltaPolicy::new(8, 1_000).unwrap(),
            &mut workspace,
        )
        .unwrap();
    assert!(!report.label_changed());
    assert!(report.topology_changed());
    assert_eq!(report.touched_vertices(), 2);
    assert!(report.invalidation().higher_order);
    assert_eq!(
        state.analysis().to_owned(),
        labeler.analyze(state.graph()).unwrap()
    );

    let revision = state.revision();
    let graph_before_error = state.graph().clone();
    let mut underflow = GraphDelta::new().with_expected_revision(revision);
    underflow
        .remove_directed_relation(
            VertexId::new(7),
            VertexId::new(8),
            b"e".to_vec(),
            b"path".to_vec(),
            2,
        )
        .unwrap();
    assert!(matches!(
        labeler.apply_delta(
            &mut state,
            &underflow,
            GraphDeltaPolicy::default(),
            &mut workspace
        ),
        Err(GraphError::GraphDeltaRelationAbsent)
    ));
    assert_eq!(state.graph(), &graph_before_error);
    assert_eq!(state.revision(), revision);

    let mut restore = GraphDelta::new().with_expected_revision(revision);
    restore
        .add_directed_relation(
            VertexId::new(7),
            VertexId::new(8),
            b"e".to_vec(),
            b"path".to_vec(),
            1,
        )
        .unwrap();
    labeler
        .apply_delta(
            &mut state,
            &restore,
            GraphDeltaPolicy::new(8, 1_000).unwrap(),
            &mut workspace,
        )
        .unwrap();
    assert_eq!(
        state.analysis().to_owned(),
        labeler.analyze(state.graph()).unwrap()
    );
}
