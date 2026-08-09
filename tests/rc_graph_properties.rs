//! RC.7 shrinkable properties for exact graph canonization and DAG persistence.

#![cfg(feature = "graph")]

use std::collections::BTreeMap;

use homomorphic_hash_rs::{
    CanonicalGraphDag, CanonicalGraphDagLimits, CanonicalGraphDocument, CanonicalSearchBudget,
    GraphDagResolveOutcome, GraphSchemaId, IncidenceGraph, IncidenceGraphBuilder, Microcanon,
    MicrocanonOutcome, VertexId,
};
use proptest::prelude::*;

#[derive(Clone, Debug)]
struct GraphCase {
    labels: Vec<u8>,
    arcs: Vec<(u8, u8, u8, u8, u8)>,
    permutation_seed: u64,
}

fn graph_cases() -> impl Strategy<Value = GraphCase> {
    (
        prop::collection::vec(any::<u8>(), 0..=6),
        prop::collection::vec(
            (
                any::<u8>(),
                any::<u8>(),
                any::<u8>(),
                any::<u8>(),
                any::<u8>(),
            ),
            0..=24,
        ),
        any::<u64>(),
    )
        .prop_map(|(labels, arcs, permutation_seed)| GraphCase {
            labels,
            arcs,
            permutation_seed,
        })
}

fn build_graph(case: &GraphCase) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = case
        .labels
        .iter()
        .map(|label| builder.add_vertex(vec![label % 4]))
        .collect::<Vec<_>>();
    if vertices.is_empty() {
        return builder.build().unwrap();
    }

    let mut normalized = BTreeMap::new();
    for &(source, target, relation, role, multiplicity) in &case.arcs {
        normalized.insert(
            (
                usize::from(source) % vertices.len(),
                usize::from(target) % vertices.len(),
                relation % 3,
                role % 2,
            ),
            u64::from(multiplicity % 4) + 1,
        );
    }
    for ((source, target, relation, role), multiplicity) in normalized {
        builder
            .add_directed_relation(
                vertices[source],
                vertices[target],
                vec![relation],
                vec![role],
                multiplicity,
            )
            .unwrap();
    }
    builder.build().unwrap()
}

fn permutation(length: usize, mut seed: u64) -> Vec<usize> {
    let mut order = (0..length).collect::<Vec<_>>();
    for index in (1..length).rev() {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        order.swap(index, seed as usize % (index + 1));
    }
    order
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn exact_canonization_and_dag_reuse_are_invariant_under_renumbering(case in graph_cases()) {
        let graph = build_graph(&case);
        let permuted = permute(&graph, &permutation(graph.vertex_count(), case.permutation_seed));
        let schema = GraphSchemaId::derive(b"rc7-property-graph-v1");
        let canonizer = Microcanon::new(schema);
        let budget = CanonicalSearchBudget::new(2_000_000);

        let original_form = match canonizer.canonicalize(&graph, budget).unwrap() {
            MicrocanonOutcome::Exact { form, .. } => form,
            MicrocanonOutcome::Inconclusive { report } => {
                return Err(TestCaseError::fail(format!("unexpected original budget exhaustion: {report:?}")));
            }
        };
        let permuted_form = match canonizer.canonicalize(&permuted, budget).unwrap() {
            MicrocanonOutcome::Exact { form, .. } => form,
            MicrocanonOutcome::Inconclusive { report } => {
                return Err(TestCaseError::fail(format!("unexpected permuted budget exhaustion: {report:?}")));
            }
        };
        prop_assert_eq!(original_form.bytes(), permuted_form.bytes());
        let original_document = original_form.decode().unwrap();
        let permuted_document = permuted_form.decode().unwrap();
        prop_assert_eq!(original_document.graph(), permuted_document.graph());

        let mut dag = CanonicalGraphDag::new(schema);
        let inserted = dag.resolve(&graph, &canonizer, budget, &[], None).unwrap();
        let node = match inserted {
            GraphDagResolveOutcome::Inserted { node, .. } => node,
            other => return Err(TestCaseError::fail(format!("first graph was not inserted: {other:?}"))),
        };
        let revision = dag.revision();
        match dag.resolve(&permuted, &canonizer, budget, &[], Some(revision)).unwrap() {
            GraphDagResolveOutcome::Reused { node: reused, report } => {
                prop_assert_eq!(reused, node);
                prop_assert!(report.exact_byte_comparisons() >= 1);
            }
            other => return Err(TestCaseError::fail(format!("isomorphic graph was not reused: {other:?}"))),
        }
        prop_assert_eq!(dag.nodes().len(), 1);

        let wire = dag.to_canonical_bytes();
        let restored = CanonicalGraphDag::from_canonical_bytes(
            &wire,
            &canonizer,
            budget,
            CanonicalGraphDagLimits::default(),
        ).unwrap();
        prop_assert_eq!(restored, dag);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_024))]

    #[test]
    fn graph_wire_parsers_never_panic_on_bounded_arbitrary_bytes(
        bytes in prop::collection::vec(any::<u8>(), 0..=4_096),
    ) {
        let _ = CanonicalGraphDocument::from_bytes(&bytes);
        let canonizer = Microcanon::new(GraphSchemaId::derive(b"rc7-property-graph-v1"));
        let _ = CanonicalGraphDag::from_canonical_bytes(
            &bytes,
            &canonizer,
            CanonicalSearchBudget::new(20_000),
            CanonicalGraphDagLimits::default(),
        );
    }
}
