//! RC.6 gates for exact canonical persistence, DAG reuse and loss-aware adapters.

use homomorphic_hash_rs::FastGraphLabeler;
use homomorphic_hash_rs::{
    CanonicalGraphDag, CanonicalGraphDagLimits, CanonicalSearchBudget, GraphDagResolveOutcome,
    GraphDagUpdateKind, GraphDelta, GraphDeltaPolicy, GraphError, GraphSchemaId,
    GraphSubnetworkAdapter, IncidenceGraph, IncidenceGraphBuilder, IncrementalGraphWorkspace,
    Microcanon, PrimeIntegerEncoder, RefinementProfile, VertexId,
};
use microfield::Fp251V1;

fn budget() -> CanonicalSearchBudget {
    CanonicalSearchBudget::new(2_000_000)
}

fn path(labels: &[&[u8]]) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = labels
        .iter()
        .map(|label| builder.add_vertex(label.to_vec()))
        .collect::<Vec<_>>();
    for pair in vertices.windows(2) {
        builder
            .add_undirected_relation(pair[0], pair[1], b"edge", b"path", 1)
            .unwrap();
    }
    builder.build().unwrap()
}

fn permute(graph: &IncidenceGraph, new_to_old: &[usize]) -> IncidenceGraph {
    let mut old_to_new = vec![usize::MAX; graph.vertex_count()];
    let mut builder = IncidenceGraphBuilder::new();
    for (new, old) in new_to_old.iter().copied().enumerate() {
        old_to_new[old] = new;
        let vertex = VertexId::new(old);
        builder.add_typed_vertex(graph.vertex_kind(vertex), graph.vertex_label(vertex));
    }
    for source in 0..graph.vertex_count() {
        for arc in graph.outgoing(VertexId::new(source)) {
            let descriptor = graph.relation(arc.relation());
            builder
                .add_directed_relation(
                    VertexId::new(old_to_new[source]),
                    VertexId::new(old_to_new[arc.neighbor().index()]),
                    descriptor.relation(),
                    descriptor.role(),
                    arc.multiplicity(),
                )
                .unwrap();
        }
    }
    builder.build().unwrap()
}

fn inserted(outcome: GraphDagResolveOutcome) -> homomorphic_hash_rs::GraphDagNodeId {
    match outcome {
        GraphDagResolveOutcome::Inserted { node, .. } => node,
        other => panic!("expected insertion, got {other:?}"),
    }
}

#[test]
fn relabelled_isomorphic_graph_reuses_only_after_exact_byte_comparison() {
    let schema = GraphSchemaId::derive(b"tests/rc6/reuse/v1");
    let canonizer = Microcanon::new(schema);
    let graph = path(&[b"same", b"same", b"same", b"same", b"same"]);
    let permuted = permute(&graph, &[4, 2, 0, 3, 1]);
    let mut dag = CanonicalGraphDag::new(schema);

    let node = inserted(
        dag.resolve(&graph, &canonizer, budget(), &[], Some(0))
            .unwrap(),
    );
    let revision = dag.revision();
    let reused = dag
        .resolve(&permuted, &canonizer, budget(), &[], Some(revision))
        .unwrap();
    match reused {
        GraphDagResolveOutcome::Reused {
            node: reused,
            report,
        } => {
            assert_eq!(reused, node);
            assert_eq!(report.lookup_candidates(), 1);
            assert_eq!(report.digest_candidates(), 1);
            assert_eq!(report.exact_byte_comparisons(), 1);
            assert_eq!(report.revision(), revision);
        }
        other => panic!("expected reuse, got {other:?}"),
    }
    assert_eq!(dag.nodes().len(), 1);
}

#[test]
fn different_regular_graphs_never_reuse_a_node() {
    fn cycles(lengths: &[usize]) -> IncidenceGraph {
        let mut builder = IncidenceGraphBuilder::new();
        let count = lengths.iter().sum();
        let vertices = (0..count)
            .map(|_| builder.add_vertex(b"v"))
            .collect::<Vec<_>>();
        let mut offset = 0;
        for length in lengths {
            for index in 0..*length {
                builder
                    .add_undirected_relation(
                        vertices[offset + index],
                        vertices[offset + (index + 1) % length],
                        b"edge",
                        b"cycle",
                        1,
                    )
                    .unwrap();
            }
            offset += length;
        }
        builder.build().unwrap()
    }
    let schema = GraphSchemaId::derive(b"tests/rc6/regular/v1");
    let canonizer = Microcanon::new(schema);
    let mut dag = CanonicalGraphDag::new(schema);
    inserted(
        dag.resolve(&cycles(&[6]), &canonizer, budget(), &[], None)
            .unwrap(),
    );
    inserted(
        dag.resolve(&cycles(&[3, 3]), &canonizer, budget(), &[], None)
            .unwrap(),
    );
    assert_eq!(dag.nodes().len(), 2);
}

#[test]
fn dag_dependencies_are_exact_acyclic_and_transactional() {
    let schema = GraphSchemaId::derive(b"tests/rc6/dependencies/v1");
    let canonizer = Microcanon::new(schema);
    let mut dag = CanonicalGraphDag::new(schema);
    let leaf = inserted(
        dag.resolve(&path(&[b"a"]), &canonizer, budget(), &[], None)
            .unwrap(),
    );
    let parent_graph = path(&[b"a", b"b"]);
    let parent = inserted(
        dag.resolve(&parent_graph, &canonizer, budget(), &[leaf, leaf], None)
            .unwrap(),
    );
    assert_eq!(dag.node(parent).unwrap().dependencies(), &[leaf]);

    let before = dag.clone();
    assert_eq!(
        dag.resolve(
            &parent_graph,
            &canonizer,
            budget(),
            &[],
            Some(dag.revision())
        ),
        Err(GraphError::GraphDagDependencyMismatch)
    );
    assert_eq!(dag, before);
    assert!(matches!(
        dag.resolve(&path(&[b"x"]), &canonizer, budget(), &[parent], Some(0)),
        Err(GraphError::GraphDagRevisionMismatch { .. })
    ));
    assert_eq!(dag, before);
}

#[test]
fn snapshot_roundtrip_recanonicalizes_and_rejects_corruption_and_limits() {
    let schema = GraphSchemaId::derive(b"tests/rc6/persistence/v1");
    let canonizer = Microcanon::new(schema);
    let mut dag = CanonicalGraphDag::new(schema);
    let first = inserted(
        dag.resolve(&path(&[b"a"]), &canonizer, budget(), &[], None)
            .unwrap(),
    );
    inserted(
        dag.resolve(&path(&[b"a", b"b"]), &canonizer, budget(), &[first], None)
            .unwrap(),
    );
    let wire = dag.to_canonical_bytes();
    let restored = CanonicalGraphDag::from_canonical_bytes(
        &wire,
        &canonizer,
        budget(),
        CanonicalGraphDagLimits::default(),
    )
    .unwrap();
    assert_eq!(restored, dag);

    for length in 0..wire.len() {
        assert!(CanonicalGraphDag::from_canonical_bytes(
            &wire[..length],
            &canonizer,
            budget(),
            CanonicalGraphDagLimits::default(),
        )
        .is_err());
    }
    let mut corrupt = wire.clone();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 1;
    assert!(CanonicalGraphDag::from_canonical_bytes(
        &corrupt,
        &canonizer,
        budget(),
        CanonicalGraphDagLimits::default(),
    )
    .is_err());

    let limits = CanonicalGraphDagLimits {
        maximum_nodes: 1,
        ..CanonicalGraphDagLimits::default()
    };
    assert_eq!(
        CanonicalGraphDag::from_canonical_bytes(&wire, &canonizer, budget(), limits),
        Err(GraphError::GraphDagLimitExceeded)
    );
}

#[test]
fn adapters_preserve_internal_semantics_and_refuse_silent_boundary_loss() {
    let mut builder = IncidenceGraphBuilder::new();
    let a = builder.add_vertex(b"a");
    let b = builder.add_vertex(b"b");
    let c = builder.add_vertex(b"c");
    builder
        .add_directed_relation(a, b, b"knows", b"source", 7)
        .unwrap();
    builder
        .add_directed_relation(b, a, b"knows", b"source", 3)
        .unwrap();
    builder
        .add_directed_relation(b, c, b"external", b"target", 11)
        .unwrap();
    let graph = builder.build().unwrap();

    let induced = GraphSubnetworkAdapter::induced(&graph, &[b, a]).unwrap();
    assert_eq!(induced.vertex_label(VertexId::new(0)), b"b");
    assert_eq!(induced.vertex_label(VertexId::new(1)), b"a");
    let b_to_a = induced.outgoing(VertexId::new(0))[0];
    assert_eq!(b_to_a.multiplicity(), 3);
    assert_eq!(induced.relation(b_to_a.relation()).role(), b"source");
    assert!(matches!(
        GraphSubnetworkAdapter::closed(&graph, &[a, b]),
        Err(GraphError::OpenSubgraphBoundary { .. })
    ));
    assert!(matches!(
        GraphSubnetworkAdapter::induced(&graph, &[a, a]),
        Err(GraphError::DuplicateSubgraphVertex { .. })
    ));
}

#[test]
fn relational_clique_requires_both_directions_and_preserves_extra_internal_arcs() {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..3)
        .map(|i| builder.add_vertex(vec![i]))
        .collect::<Vec<_>>();
    for source in 0..3 {
        for target in 0..3 {
            if source != target {
                builder
                    .add_directed_relation(vertices[source], vertices[target], b"adjacent", b"", 2)
                    .unwrap();
            }
        }
    }
    builder
        .add_directed_relation(vertices[0], vertices[1], b"weight", b"kg", 9)
        .unwrap();
    let clique = builder.build().unwrap();
    let extracted =
        GraphSubnetworkAdapter::relational_clique(&clique, &vertices, b"adjacent", b"").unwrap();
    assert_eq!(extracted.incidence_count(), 7);
    assert_eq!(extracted.total_multiplicity(), 21);

    let not_clique = path(&[b"a", b"b", b"c"]);
    assert!(matches!(
        GraphSubnetworkAdapter::relational_clique(
            &not_clique,
            &[VertexId::new(0), VertexId::new(1), VertexId::new(2)],
            b"edge",
            b"path"
        ),
        Err(GraphError::MissingCliqueRelation { .. })
    ));
}

#[test]
fn delta_refresh_records_label_policy_but_still_recanonicalizes_exactly() {
    let schema = GraphSchemaId::derive(b"tests/rc6/delta/v1");
    let canonizer = Microcanon::new(schema);
    let labeler = FastGraphLabeler::<Fp251V1, _, 2>::new(
        PrimeIntegerEncoder::new(0x5243_3601),
        RefinementProfile::fast(),
    )
    .unwrap();
    let original = path(&[b"a", b"b", b"c"]);
    let mut state = labeler.incremental_state(original.clone()).unwrap();
    let mut workspace = IncrementalGraphWorkspace::new();
    let mut dag = CanonicalGraphDag::new(schema);
    inserted(
        dag.resolve(&original, &canonizer, budget(), &[], None)
            .unwrap(),
    );

    let mut delta = GraphDelta::new().with_expected_revision(0);
    delta
        .set_vertex_label(VertexId::new(1), b"changed")
        .unwrap();
    let delta_report = labeler
        .apply_delta(
            &mut state,
            &delta,
            GraphDeltaPolicy::default(),
            &mut workspace,
        )
        .unwrap();
    let outcome = dag
        .resolve_after_delta(
            state.graph(),
            delta_report,
            &canonizer,
            budget(),
            &[],
            Some(1),
        )
        .unwrap();
    match outcome {
        GraphDagResolveOutcome::Inserted { report, .. } => {
            assert_eq!(report.update_kind(), Some(GraphDagUpdateKind::Labels));
            assert_eq!(report.revision(), 2);
        }
        other => panic!("expected changed exact node, got {other:?}"),
    }
}
