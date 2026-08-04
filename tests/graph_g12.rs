//! G12 paired-comparison, decomposition and witness contracts.

use homomorphic_hash_rs::{
    CanonicalSearchBudget, DifferenceWitness, FastGraphLabeler, GraphComparison, IncidenceGraph,
    IncidenceGraphBuilder, Microcanon, PairedComparisonPath, PrimeIntegerEncoder,
    RefinementProfile, VerifiedGraphMapping, VertexId,
};
use microfield::Fp251V1;

const DOMAIN: u64 = 0x4731_322d_5041_4952;

fn relabel(graph: &IncidenceGraph, new_to_old: &[usize]) -> IncidenceGraph {
    assert_eq!(new_to_old.len(), graph.vertex_count());
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

fn path(order: usize) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..order)
        .map(|_| builder.add_vertex(b"v".to_vec()))
        .collect::<Vec<_>>();
    for edge in vertices.windows(2) {
        builder
            .add_undirected_relation(edge[0], edge[1], b"edge".to_vec(), b"tree".to_vec(), 1)
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
        for vertex in 0..length {
            builder
                .add_undirected_relation(
                    vertices[offset + vertex],
                    vertices[offset + (vertex + 1) % length],
                    b"edge".to_vec(),
                    b"cycle".to_vec(),
                    1,
                )
                .unwrap();
        }
        offset += length;
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
                b"edge".to_vec(),
                b"tree".to_vec(),
                1,
            )
            .unwrap();
    }
    builder.build().unwrap()
}

fn figure_eight() -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..5)
        .map(|_| builder.add_vertex(b"v".to_vec()))
        .collect::<Vec<_>>();
    for (left, right) in [(0, 1), (1, 2), (2, 0), (0, 3), (3, 4), (4, 0)] {
        builder
            .add_undirected_relation(
                vertices[left],
                vertices[right],
                b"edge".to_vec(),
                b"block".to_vec(),
                1,
            )
            .unwrap();
    }
    builder.build().unwrap()
}

fn graph_from_mask(order: usize, mask: u64) -> IncidenceGraph {
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
                        b"e".to_vec(),
                        Vec::new(),
                        1,
                    )
                    .unwrap();
            }
            bit += 1;
        }
    }
    builder.build().unwrap()
}

#[test]
fn exact_forest_route_scales_and_returns_only_a_verified_mapping() {
    let graph = path(4_096);
    let permutation = (0..graph.vertex_count()).rev().collect::<Vec<_>>();
    let permuted = relabel(&graph, &permutation);
    let result = Microcanon::default()
        .compare(
            &graph,
            &permuted,
            CanonicalSearchBudget::new(10_000)
                .with_max_retained_state_cells(100_000)
                .with_max_retained_bytes(64 * 1024 * 1024),
        )
        .unwrap();
    match result {
        GraphComparison::Isomorphic { mapping, report } => {
            VerifiedGraphMapping::verify(&graph, &permuted, mapping.left_to_right()).unwrap();
            let paired = report.paired().unwrap();
            assert_eq!(paired.path(), PairedComparisonPath::TreeForest);
            assert_eq!(paired.explored_nodes(), 4_096);
        }
        outcome => panic!("expected exact forest mapping, found {outcome:?}"),
    }
}

#[test]
fn block_cut_and_general_paths_are_selected_without_weakening_exactness() {
    let graph = figure_eight();
    let permuted = relabel(&graph, &[3, 1, 4, 0, 2]);
    match Microcanon::default()
        .compare(&graph, &permuted, CanonicalSearchBudget::new(100_000))
        .unwrap()
    {
        GraphComparison::Isomorphic { mapping, report } => {
            VerifiedGraphMapping::verify(&graph, &permuted, mapping.left_to_right()).unwrap();
            let paired = report.paired().unwrap();
            assert_eq!(paired.path(), PairedComparisonPath::BlockCutDecomposition);
            assert!(paired.block_count() >= 2);
            assert!(paired.articulation_vertex_count() >= 1);
        }
        outcome => panic!("expected block-cut mapping, found {outcome:?}"),
    }

    let cycle = cycles(&[17]);
    let permutation = (0..17).map(|index| (index * 5) % 17).collect::<Vec<_>>();
    let permuted = relabel(&cycle, &permutation);
    match Microcanon::default()
        .compare(&cycle, &permuted, CanonicalSearchBudget::new(100_000))
        .unwrap()
    {
        GraphComparison::Isomorphic { report, .. } => assert_eq!(
            report.paired().unwrap().path(),
            PairedComparisonPath::PairedSearch
        ),
        outcome => panic!("expected general paired mapping, found {outcome:?}"),
    }
}

#[test]
fn exact_degree_histogram_rejects_before_block_cut_or_search() {
    let left = path(8);
    let right = star(8);
    match Microcanon::default()
        .compare(&left, &right, CanonicalSearchBudget::new(100_000))
        .unwrap()
    {
        GraphComparison::Different {
            witness: DifferenceWitness::VertexDescriptors { .. },
            report,
        } => {
            let paired = report.paired().unwrap();
            assert_eq!(paired.path(), PairedComparisonPath::ExactPrefilter);
            assert_eq!(paired.explored_nodes(), 0);
        }
        outcome => panic!("expected exact degree-histogram rejection, found {outcome:?}"),
    }
}

#[test]
fn paired_matcher_agrees_with_canonical_forms_on_small_graphs() {
    let canon = Microcanon::default();
    let budget = CanonicalSearchBudget::new(1_000_000);
    for mask in 0_u64..256 {
        let reversed = mask.reverse_bits() >> (64 - 10);
        let left = graph_from_mask(5, mask);
        let right = graph_from_mask(5, reversed);
        let left_form = match canon.canonicalize(&left, budget).unwrap() {
            homomorphic_hash_rs::MicrocanonOutcome::Exact { form, .. } => form,
            outcome => panic!("left oracle incomplete for {mask}: {outcome:?}"),
        };
        let right_form = match canon.canonicalize(&right, budget).unwrap() {
            homomorphic_hash_rs::MicrocanonOutcome::Exact { form, .. } => form,
            outcome => panic!("right oracle incomplete for {mask}: {outcome:?}"),
        };
        let expected = left_form.bytes() == right_form.bytes();
        let actual = canon.compare(&left, &right, budget).unwrap();
        assert_eq!(
            matches!(actual, GraphComparison::Isomorphic { .. }),
            expected,
            "paired/canonical disagreement for masks {mask:#x} and {reversed:#x}: {actual:?}"
        );
        assert!(!matches!(actual, GraphComparison::Inconclusive { .. }));
    }
}

#[test]
fn paired_search_is_relabeling_invariant_across_deterministic_relational_graphs() {
    let canon = Microcanon::default();
    let budget = CanonicalSearchBudget::new(1_000_000);
    for seed in 0_u64..128 {
        let mut state = seed ^ 0x9e37_79b9_7f4a_7c15;
        let mut builder = IncidenceGraphBuilder::new();
        let vertices = (0..7)
            .map(|vertex| builder.add_vertex(vec![(vertex % 3) as u8]))
            .collect::<Vec<_>>();
        for source in 0..7 {
            for target in 0..7 {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                if state & 7 == 0 {
                    builder
                        .add_directed_relation(
                            vertices[source],
                            vertices[target],
                            vec![b'a' + ((state >> 8) % 3) as u8],
                            vec![b'p' + ((state >> 16) % 2) as u8],
                            1 + ((state >> 24) % 3),
                        )
                        .unwrap();
                }
            }
        }
        let graph = builder.build().unwrap();
        let permutation = [6, 2, 4, 0, 5, 1, 3];
        let permuted = relabel(&graph, &permutation);
        match canon.compare(&graph, &permuted, budget).unwrap() {
            GraphComparison::Isomorphic { mapping, .. } => {
                VerifiedGraphMapping::verify(&graph, &permuted, mapping.left_to_right()).unwrap();
            }
            outcome => panic!("seed {seed} did not produce a verified mapping: {outcome:?}"),
        }
    }
}

#[test]
fn finite_field_evidence_can_reject_but_never_authorize_isomorphism() {
    let cycle = path(6);
    let triangles = star(6);
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::Fast { rounds: 5 },
    )
    .unwrap();
    match Microcanon::default()
        .compare_with_field_profile(
            &cycle,
            &triangles,
            &labeler,
            CanonicalSearchBudget::new(100_000),
        )
        .unwrap()
    {
        GraphComparison::Different {
            witness: DifferenceWitness::FiniteFieldEvidence { signature_id },
            ..
        } => {
            assert_eq!(signature_id, labeler.signature_id());
        }
        outcome => panic!("expected finite-field rejection, found {outcome:?}"),
    }

    let same = Microcanon::default()
        .compare_with_field_profile(
            &cycle,
            &cycle,
            &labeler,
            CanonicalSearchBudget::new(100_000),
        )
        .unwrap();
    assert!(matches!(same, GraphComparison::Isomorphic { .. }));
    assert!(same_report_is_exact(&same));
}

fn same_report_is_exact(comparison: &GraphComparison) -> bool {
    match comparison {
        GraphComparison::Isomorphic { report, .. } => report.paired().is_some(),
        _ => false,
    }
}

#[test]
fn every_paired_limit_fails_closed() {
    use std::time::Duration;

    let graph = cycles(&[12]);
    let canon = Microcanon::default();
    for budget in [
        CanonicalSearchBudget::new(0),
        CanonicalSearchBudget::new(100_000).with_max_retained_state_cells(1),
        CanonicalSearchBudget::new(100_000).with_max_retained_bytes(1),
        CanonicalSearchBudget::new(100_000).with_max_elapsed(Duration::ZERO),
    ] {
        assert!(matches!(
            canon.compare(&graph, &graph, budget).unwrap(),
            GraphComparison::Inconclusive { .. }
        ));
    }

    let forest = path(32);
    for budget in [
        CanonicalSearchBudget::new(31),
        CanonicalSearchBudget::new(100_000).with_max_retained_state_cells(1),
        CanonicalSearchBudget::new(100_000).with_max_retained_bytes(1),
        CanonicalSearchBudget::new(100_000).with_max_elapsed(Duration::ZERO),
    ] {
        assert!(matches!(
            canon.compare(&forest, &forest, budget).unwrap(),
            GraphComparison::Inconclusive { .. }
        ));
    }
}
