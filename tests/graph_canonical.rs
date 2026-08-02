//! Adversarial validation of degeneracy diagnosis and exact canonization.

use std::collections::{BTreeMap, BTreeSet};

use homomorphic_hash_rs::{
    BinaryPolynomialEncoder, CanonicalBudgetLimit, CanonicalSearchBudget, CanonicalizationPath,
    DiscriminatingGraphComparison, DiscriminationRecommendation, ExactCanonicalOutcome,
    FastGraphLabeler, GraphDiscriminationPolicy, GraphError, GraphEscalationAdvice,
    GraphEvidenceComparison, IncidenceGraph, IncidenceGraphBuilder, MotifAnalysisStatus,
    MultiFieldGraphEvidenceBuilder, PrimeIntegerEncoder, RefinementProfile, VertexId,
};
use microfield::{CanonicalEncoding, Fp251V1, Gf2_256HhV1};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};

const DOMAIN: u64 = 0x4752_4341_4e4f_4e31;

fn undirected_cycles(lengths: &[usize]) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let total: usize = lengths.iter().sum();
    let vertices: Vec<_> = (0..total).map(|_| builder.add_vertex(Vec::new())).collect();
    let mut offset = 0;
    for &length in lengths {
        for index in 0..length {
            builder
                .add_undirected_relation(
                    vertices[offset + index],
                    vertices[offset + (index + 1) % length],
                    b"edge".to_vec(),
                    Vec::new(),
                    1,
                )
                .unwrap();
        }
        offset += length;
    }
    builder.build().unwrap()
}

fn rook_graph_4x4() -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = (0..16).map(|_| builder.add_vertex(Vec::new())).collect();
    for left in 0..16 {
        let (left_row, left_column) = (left / 4, left % 4);
        for right in left + 1..16 {
            let (right_row, right_column) = (right / 4, right % 4);
            if left_row == right_row || left_column == right_column {
                builder
                    .add_undirected_relation(
                        vertices[left],
                        vertices[right],
                        b"edge".to_vec(),
                        Vec::new(),
                        1,
                    )
                    .unwrap();
            }
        }
    }
    builder.build().unwrap()
}

fn shrikhande_graph() -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = (0..16).map(|_| builder.add_vertex(Vec::new())).collect();
    let generators = [(1, 0), (3, 0), (0, 1), (0, 3), (1, 1), (3, 3)];
    let mut edges = BTreeSet::new();
    for row in 0..4 {
        for column in 0..4 {
            let source = row * 4 + column;
            for (delta_row, delta_column) in generators {
                let target = ((row + delta_row) % 4) * 4 + (column + delta_column) % 4;
                edges.insert((source.min(target), source.max(target)));
            }
        }
    }
    for (left, right) in edges {
        builder
            .add_undirected_relation(
                vertices[left],
                vertices[right],
                b"edge".to_vec(),
                Vec::new(),
                1,
            )
            .unwrap();
    }
    builder.build().unwrap()
}

fn relabel(graph: &IncidenceGraph, new_to_old: &[usize]) -> IncidenceGraph {
    let mut old_to_new = vec![usize::MAX; graph.vertex_count()];
    let mut builder = IncidenceGraphBuilder::new();
    for (new, &old) in new_to_old.iter().enumerate() {
        old_to_new[old] = new;
        let old = VertexId::new(old);
        builder.add_typed_vertex(graph.vertex_kind(old), graph.vertex_label(old).to_vec());
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

fn exact_form(outcome: ExactCanonicalOutcome) -> (Vec<u8>, Vec<VertexId>, u64) {
    match outcome {
        ExactCanonicalOutcome::Exact { form, report } => (
            form.bytes().to_vec(),
            form.canonical_to_original().to_vec(),
            report.explored_nodes(),
        ),
        ExactCanonicalOutcome::BudgetExhausted { report } => {
            panic!("unexpected budget exhaustion: {report:?}")
        }
    }
}

#[test]
fn regular_non_isomorphic_graphs_collide_in_every_local_field_profile() {
    let cycle6 = undirected_cycles(&[6]);
    let two_triangles = undirected_cycles(&[3, 3]);
    let f251 = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::Fast { rounds: 12 },
    )
    .unwrap();
    let binary = FastGraphLabeler::<Gf2_256HhV1, _, 3>::new(
        BinaryPolynomialEncoder::new(DOMAIN),
        RefinementProfile::Fast { rounds: 12 },
    )
    .unwrap();

    assert_eq!(
        f251.analyze(&cycle6).unwrap().signature(),
        f251.analyze(&two_triangles).unwrap().signature()
    );
    assert_eq!(
        binary.analyze(&cycle6).unwrap().signature(),
        binary.analyze(&two_triangles).unwrap().signature()
    );
    assert_eq!(
        f251.analyze_hybrid(&cycle6).unwrap().invariant_digest(),
        f251.analyze_hybrid(&two_triangles)
            .unwrap()
            .invariant_digest()
    );

    for graph in [&cycle6, &two_triangles] {
        let report = f251.diagnose_degeneracy(graph).unwrap();
        assert_eq!(report.exact_refinement_cell_count(), 1);
        assert_eq!(report.ambiguous_vertex_count(), 6);
        assert_eq!(report.largest_exact_refinement_cell(), 6);
        assert!(report.is_highly_regular());
        assert!(!report.has_field_aliasing());
        assert_eq!(
            report.recommendation(),
            DiscriminationRecommendation::ExactCanonicalizationRecommended
        );
    }

    let budget = CanonicalSearchBudget::new(100_000);
    let (left, _, _) = exact_form(f251.canonicalize_exact(&cycle6, budget).unwrap());
    let (right, _, _) = exact_form(f251.canonicalize_exact(&two_triangles, budget).unwrap());
    assert_ne!(left, right);
}

#[test]
fn v2_global_profile_separates_cycle_from_two_triangles_in_linear_mode() {
    let cycle = undirected_cycles(&[6]);
    let triangles = undirected_cycles(&[3, 3]);
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::Fast { rounds: 12 },
    )
    .unwrap();

    let left = labeler
        .analyze_discriminating(&cycle, GraphDiscriminationPolicy::GlobalLinear)
        .unwrap();
    let right = labeler
        .analyze_discriminating(&triangles, GraphDiscriminationPolicy::GlobalLinear)
        .unwrap();

    assert_eq!(left.global().weak_component_count(), 1);
    assert_eq!(right.global().weak_component_count(), 2);
    assert_eq!(left.global().strongly_connected_component_count(), 1);
    assert_eq!(right.global().strongly_connected_component_count(), 2);
    assert_eq!(left.global().weak_components()[0].cycle_rank(), 1);
    assert!(right
        .global()
        .weak_components()
        .iter()
        .all(|component| component.vertex_count() == 3 && component.cycle_rank() == 1));
    assert_ne!(left.global().digest(), right.global().digest());
    assert_ne!(left.digest(), right.digest());
    assert_eq!(
        left.compare(&right).unwrap(),
        DiscriminatingGraphComparison::Different
    );
}

#[test]
fn exact_canonization_decomposes_disconnected_graphs_transactionally() {
    let graph = undirected_cycles(&[3, 3, 4]);
    let permutation = [7, 2, 9, 0, 5, 3, 8, 1, 6, 4];
    let permuted = relabel(&graph, &permutation);
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::fast(),
    )
    .unwrap();
    let budget = CanonicalSearchBudget::new(100_000);
    let expected = labeler.canonicalize_exact(&graph, budget).unwrap();
    let actual = labeler.canonicalize_exact(&permuted, budget).unwrap();

    assert_eq!(
        expected.report().path(),
        CanonicalizationPath::WeakComponentDecomposition
    );
    assert_eq!(
        actual.report().path(),
        CanonicalizationPath::WeakComponentDecomposition
    );
    assert_eq!(exact_form(expected).0, exact_form(actual).0);

    assert!(matches!(
        labeler
            .canonicalize_exact(&graph, CanonicalSearchBudget::new(1))
            .unwrap(),
        ExactCanonicalOutcome::BudgetExhausted { report }
            if report.path() == CanonicalizationPath::WeakComponentDecomposition
                && report.exhausted_limit() == Some(CanonicalBudgetLimit::SearchNodes)
    ));
}

#[test]
fn adaptive_motif_tier_separates_shrikhande_from_rook() {
    let shrikhande = shrikhande_graph();
    let rook = rook_graph_4x4();
    let labeler = FastGraphLabeler::<Gf2_256HhV1, _, 4>::new(
        BinaryPolynomialEncoder::new(DOMAIN),
        RefinementProfile::Fast { rounds: 20 },
    )
    .unwrap();
    let policy = GraphDiscriminationPolicy::Adaptive {
        max_motif_work: 100_000,
    };
    let left = labeler.analyze_discriminating(&shrikhande, policy).unwrap();
    let right = labeler.analyze_discriminating(&rook, policy).unwrap();

    assert_eq!(left.global(), right.global());
    assert_eq!(left.motifs().status(), MotifAnalysisStatus::Complete);
    assert_eq!(right.motifs().status(), MotifAnalysisStatus::Complete);
    assert_eq!(left.motifs().triangle_count(), Some(32));
    assert_eq!(right.motifs().triangle_count(), Some(32));
    assert_eq!(left.motifs().four_clique_count(), Some(0));
    assert_eq!(right.motifs().four_clique_count(), Some(8));
    assert_eq!(left.advice(), GraphEscalationAdvice::MotifEvidenceAvailable);
    assert_eq!(
        left.compare(&right).unwrap(),
        DiscriminatingGraphComparison::Different
    );
}

#[test]
fn v2_profile_is_relabeling_invariant_and_budget_admission_is_fail_closed() {
    let graph = rook_graph_4x4();
    let permutation: Vec<_> = (0..graph.vertex_count()).rev().collect();
    let permuted = relabel(&graph, &permutation);
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::fast(),
    )
    .unwrap();
    let policy = GraphDiscriminationPolicy::Adaptive { max_motif_work: 1 };
    let left = labeler.analyze_discriminating(&graph, policy).unwrap();
    let right = labeler.analyze_discriminating(&permuted, policy).unwrap();

    assert_eq!(left, right);
    assert_eq!(left.motifs().status(), MotifAnalysisStatus::SkippedBudget);
    assert_eq!(left.motifs().triangle_count(), None);
    assert_eq!(left.motifs().four_clique_count(), None);
    assert_eq!(
        left.advice(),
        GraphEscalationAdvice::ExactCanonicalizationRecommended
    );
    assert_eq!(
        left.compare(&right).unwrap(),
        DiscriminatingGraphComparison::Indistinguishable
    );

    let incompatible = labeler
        .analyze_discriminating(&graph, GraphDiscriminationPolicy::GlobalLinear)
        .unwrap();
    assert!(matches!(
        left.compare(&incompatible),
        Err(GraphError::DiscriminationProfileMismatch)
    ));
}

#[test]
fn regularity_degeneracy_is_not_cured_by_graph_size_or_more_rounds() {
    for size in 6..=40 {
        let split = 3.max(size / 2);
        if size - split < 3 {
            continue;
        }
        let connected = undirected_cycles(&[size]);
        let disconnected = undirected_cycles(&[split, size - split]);
        for rounds in [1, 4, size + 3] {
            let labeler = FastGraphLabeler::<Fp251V1, _, 4>::new(
                PrimeIntegerEncoder::new(DOMAIN),
                RefinementProfile::Fast { rounds },
            )
            .unwrap();
            assert_eq!(
                labeler.analyze(&connected).unwrap().signature(),
                labeler.analyze(&disconnected).unwrap().signature(),
                "2-regular collision at V={size}, R={rounds}"
            );
            let diagnosis = labeler.diagnose_degeneracy(&connected).unwrap();
            assert!(diagnosis.is_highly_regular());
            assert_eq!(diagnosis.exact_refinement_cell_count(), 1);
        }
    }
}

#[test]
fn strongly_regular_shrikhande_and_rook_graphs_trigger_exact_escalation() {
    let shrikhande = shrikhande_graph();
    let rook = rook_graph_4x4();
    assert_eq!(shrikhande.vertex_count(), 16);
    assert_eq!(shrikhande.incidence_count(), 96);
    assert_eq!(rook.incidence_count(), 96);
    let labeler = FastGraphLabeler::<Gf2_256HhV1, _, 4>::new(
        BinaryPolynomialEncoder::new(DOMAIN),
        RefinementProfile::Fast { rounds: 20 },
    )
    .unwrap();
    assert_eq!(
        labeler.analyze(&shrikhande).unwrap().signature(),
        labeler.analyze(&rook).unwrap().signature()
    );
    for graph in [&shrikhande, &rook] {
        let report = labeler.diagnose_degeneracy(graph).unwrap();
        assert_eq!(report.exact_refinement_cell_count(), 1);
        assert_eq!(report.largest_exact_refinement_cell(), 16);
        assert!(report.is_highly_regular());
        assert_eq!(
            report.recommendation(),
            DiscriminationRecommendation::ExactCanonicalizationRecommended
        );
        assert!(matches!(
            labeler
                .canonicalize_exact(graph, CanonicalSearchBudget::new(1))
                .unwrap(),
            ExactCanonicalOutcome::BudgetExhausted { .. }
        ));
    }
}

#[test]
fn exact_canonicalization_is_relabeling_invariant_on_a_symmetric_graph() {
    let graph = undirected_cycles(&[7]);
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::fast(),
    )
    .unwrap();
    let budget = CanonicalSearchBudget::new(200_000);
    let (expected, _, expected_nodes) =
        exact_form(labeler.canonicalize_exact(&graph, budget).unwrap());
    let mut permutation: Vec<_> = (0..graph.vertex_count()).collect();
    let mut rng = StdRng::seed_from_u64(0x1a2b_3c4d_5e6f_7788);
    for _ in 0..64 {
        permutation.shuffle(&mut rng);
        let permuted = relabel(&graph, &permutation);
        let (actual, _, actual_nodes) =
            exact_form(labeler.canonicalize_exact(&permuted, budget).unwrap());
        assert_eq!(actual, expected);
        assert_eq!(actual_nodes, expected_nodes);
    }
}

#[test]
fn exact_search_never_publishes_a_partial_candidate_on_budget_exhaustion() {
    let graph = undirected_cycles(&[6]);
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::fast(),
    )
    .unwrap();
    let complete = labeler
        .canonicalize_exact(&graph, CanonicalSearchBudget::new(1_000_000))
        .unwrap();
    let required = complete.report().explored_nodes();
    assert!(required > 1);

    match labeler
        .canonicalize_exact(&graph, CanonicalSearchBudget::new(required - 1))
        .unwrap()
    {
        ExactCanonicalOutcome::BudgetExhausted { report } => {
            assert_eq!(
                report.exhausted_limit(),
                Some(CanonicalBudgetLimit::SearchNodes)
            );
            assert_eq!(report.explored_nodes(), required - 1);
        }
        ExactCanonicalOutcome::Exact { .. } => panic!("an incomplete tree cannot be exact"),
    }
    assert!(matches!(
        labeler
            .canonicalize_exact(&graph, CanonicalSearchBudget::new(required))
            .unwrap(),
        ExactCanonicalOutcome::Exact { .. }
    ));

    match labeler
        .canonicalize_exact(
            &graph,
            CanonicalSearchBudget::new(1_000_000).with_max_retained_state_cells(7),
        )
        .unwrap()
    {
        ExactCanonicalOutcome::BudgetExhausted { report } => assert_eq!(
            report.exhausted_limit(),
            Some(CanonicalBudgetLimit::RetainedStateCells)
        ),
        ExactCanonicalOutcome::Exact { .. } => panic!("the root state exceeds seven cells"),
    }
}

#[test]
fn finite_field_aliasing_is_separated_from_combinatorial_regularity() {
    let labeler = FastGraphLabeler::<Fp251V1, _, 1>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::Fast { rounds: 2 },
    )
    .unwrap();
    let mut observed = BTreeMap::new();
    let mut collision = None;
    for label in 0_u16..=255 {
        let mut builder = IncidenceGraphBuilder::new();
        builder.add_vertex(vec![u8::try_from(label).unwrap()]);
        let graph = builder.build().unwrap();
        let structural_label = labeler.analyze(&graph).unwrap().labels()[0];
        let key = structural_label.lanes()[0].to_canonical().as_ref().to_vec();
        if let Some(previous) = observed.insert(key, label) {
            collision = Some((previous, label));
            break;
        }
    }
    let (left, right) = collision.expect("256 exact labels cannot fit injectively in F251");
    let mut builder = IncidenceGraphBuilder::new();
    builder.add_vertex(vec![u8::try_from(left).unwrap()]);
    builder.add_vertex(vec![u8::try_from(right).unwrap()]);
    let graph = builder.build().unwrap();

    let report = labeler.diagnose_degeneracy(&graph).unwrap();
    assert_eq!(report.fast_cell_count(), 1);
    assert_eq!(report.exact_refinement_cell_count(), 2);
    assert_eq!(report.field_aliasing_cell_count(), 1);
    assert_eq!(report.field_aliasing_vertex_count(), 2);
    assert!(report.has_field_aliasing());
    assert!(!report.has_local_ambiguity());
    assert!(!report.is_highly_regular());
    assert_eq!(
        report.recommendation(),
        DiscriminationRecommendation::AddIndependentEvidenceOrCanonize
    );

    match labeler
        .canonicalize_exact(&graph, CanonicalSearchBudget::new(1))
        .unwrap()
    {
        ExactCanonicalOutcome::Exact { report, .. } => {
            assert_eq!(
                report.path(),
                CanonicalizationPath::WeakComponentDecomposition
            );
            assert_eq!(report.explored_nodes(), 0);
        }
        ExactCanonicalOutcome::BudgetExhausted { .. } => {
            panic!("exact byte refinement is already discrete")
        }
    }
}

#[test]
fn fast_discrete_graph_uses_no_individualization_nodes() {
    let mut builder = IncidenceGraphBuilder::new();
    let a = builder.add_vertex(b"a".to_vec());
    let b = builder.add_vertex(b"b".to_vec());
    let c = builder.add_vertex(b"c".to_vec());
    builder
        .add_directed_relation(a, b, b"r".to_vec(), b"out".to_vec(), 2)
        .unwrap();
    builder
        .add_directed_relation(b, c, b"s".to_vec(), b"out".to_vec(), 1)
        .unwrap();
    let graph = builder.build().unwrap();
    let labeler = FastGraphLabeler::<Gf2_256HhV1, _, 3>::new(
        BinaryPolynomialEncoder::new(DOMAIN),
        RefinementProfile::fast(),
    )
    .unwrap();
    match labeler
        .canonicalize_exact(&graph, CanonicalSearchBudget::new(0))
        .unwrap()
    {
        ExactCanonicalOutcome::Exact { report, .. } => {
            assert_eq!(report.path(), CanonicalizationPath::FastDiscrete);
            assert_eq!(report.explored_nodes(), 0);
            assert_eq!(
                report.degeneracy().recommendation(),
                DiscriminationRecommendation::FastPathSufficient
            );
        }
        ExactCanonicalOutcome::BudgetExhausted { .. } => {
            panic!("a discrete fast order requires no search budget")
        }
    }
}

#[test]
fn randomized_normalization_and_mass_relabeling_preserve_exact_bytes() {
    let mut source = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = (0_u8..9)
        .map(|index| source.add_vertex(vec![b'a' + index % 4]))
        .collect();
    for index in 0..vertices.len() {
        source
            .add_directed_relation(
                vertices[index],
                vertices[(index * 5 + 2) % vertices.len()],
                vec![b'r', u8::try_from(index % 3).unwrap()],
                vec![u8::try_from(index % 2).unwrap()],
                u64::try_from(index % 4 + 1).unwrap(),
            )
            .unwrap();
    }
    let graph = source.build().unwrap();
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::Fast { rounds: 8 },
    )
    .unwrap();
    let budget = CanonicalSearchBudget::new(200_000);
    let (expected, _, _) = exact_form(labeler.canonicalize_exact(&graph, budget).unwrap());
    let expected_signature = labeler.analyze(&graph).unwrap().signature().clone();
    let mut rng = StdRng::seed_from_u64(0xc001_d00d_55aa_9911);

    for _ in 0..128 {
        let mut new_to_old: Vec<_> = (0..graph.vertex_count()).collect();
        new_to_old.shuffle(&mut rng);
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
        let mut arcs = Vec::new();
        for source in 0..graph.vertex_count() {
            for incidence in graph.outgoing(VertexId::new(source)) {
                arcs.push((source, *incidence));
            }
        }
        arcs.shuffle(&mut rng);
        for (source, incidence) in arcs {
            let descriptor = graph.relation(incidence.relation());
            let multiplicity = incidence.multiplicity();
            let first = if multiplicity > 1 {
                rng.gen_range(1..multiplicity)
            } else {
                1
            };
            for part in [first, multiplicity - first]
                .into_iter()
                .filter(|part| *part != 0)
            {
                builder
                    .add_directed_relation(
                        VertexId::new(old_to_new[source]),
                        VertexId::new(old_to_new[incidence.neighbor().index()]),
                        descriptor.relation().to_vec(),
                        descriptor.role().to_vec(),
                        part,
                    )
                    .unwrap();
            }
        }
        let normalized = builder.build().unwrap();
        assert_eq!(
            labeler.analyze(&normalized).unwrap().signature(),
            &expected_signature
        );
        let (actual, _, _) = exact_form(labeler.canonicalize_exact(&normalized, budget).unwrap());
        assert_eq!(actual, expected);
    }
}

#[test]
fn exact_bytes_preserve_direction_role_multiplicity_and_components() {
    fn variant(reverse: bool, role: &[u8], multiplicity: u64) -> IncidenceGraph {
        let mut builder = IncidenceGraphBuilder::new();
        // Distinct exact labels make edge reversal semantic. With two
        // anonymous vertices, reversing the sole arc is an isomorphic rename.
        let a = builder.add_vertex(b"source-kind".to_vec());
        let b = builder.add_vertex(b"target-kind".to_vec());
        let (source, target) = if reverse { (b, a) } else { (a, b) };
        builder
            .add_directed_relation(source, target, b"r".to_vec(), role.to_vec(), multiplicity)
            .unwrap();
        builder.build().unwrap()
    }
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::fast(),
    )
    .unwrap();
    let budget = CanonicalSearchBudget::new(10_000);
    let baseline = exact_form(
        labeler
            .canonicalize_exact(&variant(false, b"x", 1), budget)
            .unwrap(),
    )
    .0;
    for different in [
        variant(true, b"x", 1),
        variant(false, b"y", 1),
        variant(false, b"x", 2),
        undirected_cycles(&[3]),
    ] {
        assert_ne!(
            baseline,
            exact_form(labeler.canonicalize_exact(&different, budget).unwrap()).0
        );
    }
}

fn simple_graph(vertex_count: usize, edge_mask: u64) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = (0..vertex_count)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect();
    let mut bit = 0;
    for left in 0..vertex_count {
        for right in left + 1..vertex_count {
            if edge_mask & (1_u64 << bit) != 0 {
                builder
                    .add_undirected_relation(
                        vertices[left],
                        vertices[right],
                        b"edge".to_vec(),
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

fn for_each_permutation(values: &mut [usize], start: usize, visit: &mut impl FnMut(&[usize])) {
    if start == values.len() {
        visit(values);
        return;
    }
    for index in start..values.len() {
        values.swap(start, index);
        for_each_permutation(values, start + 1, visit);
        values.swap(start, index);
    }
}

fn brute_force_simple_canonical(vertex_count: usize, edge_mask: u64) -> u64 {
    let mut edges = BTreeSet::new();
    let mut bit = 0;
    for left in 0..vertex_count {
        for right in left + 1..vertex_count {
            if edge_mask & (1_u64 << bit) != 0 {
                edges.insert((left, right));
            }
            bit += 1;
        }
    }
    let mut permutation: Vec<_> = (0..vertex_count).collect();
    let mut minimum = u64::MAX;
    for_each_permutation(&mut permutation, 0, &mut |order| {
        let mut encoded = 0_u64;
        let mut position = 0;
        for left in 0..vertex_count {
            for right in left + 1..vertex_count {
                let endpoints = if order[left] < order[right] {
                    (order[left], order[right])
                } else {
                    (order[right], order[left])
                };
                if edges.contains(&endpoints) {
                    encoded |= 1_u64 << position;
                }
                position += 1;
            }
        }
        minimum = minimum.min(encoded);
    });
    minimum
}

#[test]
fn exact_results_match_an_independent_exhaustive_oracle_through_five_vertices() {
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::fast(),
    )
    .unwrap();
    for (vertex_count, edge_bits, expected_classes) in [(4, 6, 11), (5, 10, 34)] {
        let mut oracle_to_production = BTreeMap::new();
        let mut production_to_oracle = BTreeMap::new();
        for mask in 0_u64..(1 << edge_bits) {
            let oracle = brute_force_simple_canonical(vertex_count, mask);
            let graph = simple_graph(vertex_count, mask);
            let production = exact_form(
                labeler
                    .canonicalize_exact(&graph, CanonicalSearchBudget::new(100_000))
                    .unwrap(),
            )
            .0;
            if let Some(previous) = oracle_to_production.insert(oracle, production.clone()) {
                assert_eq!(
                    previous, production,
                    "isomorphic V={vertex_count} masks disagree at {mask:#x}"
                );
            }
            if let Some(previous) = production_to_oracle.insert(production, oracle) {
                assert_eq!(
                    previous, oracle,
                    "non-isomorphic V={vertex_count} masks collided at {mask:#x}"
                );
            }
        }
        assert_eq!(oracle_to_production.len(), expected_classes);
        assert_eq!(production_to_oracle.len(), expected_classes);
    }
}

#[test]
fn multi_field_evidence_is_identified_but_never_claims_isomorphism() {
    let cycle = undirected_cycles(&[6]);
    let triangles = undirected_cycles(&[3, 3]);
    let path = {
        let mut builder = IncidenceGraphBuilder::new();
        let vertices: Vec<_> = (0..6).map(|_| builder.add_vertex(Vec::new())).collect();
        for index in 0..5 {
            builder
                .add_undirected_relation(
                    vertices[index],
                    vertices[index + 1],
                    b"edge".to_vec(),
                    Vec::new(),
                    1,
                )
                .unwrap();
        }
        builder.build().unwrap()
    };
    let f251 = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::fast(),
    )
    .unwrap();
    let binary = FastGraphLabeler::<Gf2_256HhV1, _, 2>::new(
        BinaryPolynomialEncoder::new(DOMAIN),
        RefinementProfile::fast(),
    )
    .unwrap();

    fn evidence(
        graph: &IncidenceGraph,
        f251: &FastGraphLabeler<Fp251V1, PrimeIntegerEncoder, 3>,
        binary: &FastGraphLabeler<Gf2_256HhV1, BinaryPolynomialEncoder, 2>,
        reverse: bool,
    ) -> homomorphic_hash_rs::MultiFieldGraphEvidence {
        let first = f251.analyze(graph).unwrap();
        let second = binary.analyze(graph).unwrap();
        let mut builder = MultiFieldGraphEvidenceBuilder::new();
        if reverse {
            builder.add(second.signature()).unwrap();
            builder.add(first.signature()).unwrap();
        } else {
            builder.add(first.signature()).unwrap();
            builder.add(second.signature()).unwrap();
        }
        builder.build().unwrap()
    }

    let cycle_evidence = evidence(&cycle, &f251, &binary, false);
    let same_profile_reverse_order = evidence(&cycle, &f251, &binary, true);
    let triangle_evidence = evidence(&triangles, &f251, &binary, false);
    let path_evidence = evidence(&path, &f251, &binary, false);
    assert_eq!(cycle_evidence, same_profile_reverse_order);
    assert_eq!(cycle_evidence.channels().len(), 2);
    assert_eq!(
        cycle_evidence.compare(&triangle_evidence).unwrap(),
        GraphEvidenceComparison::Indistinguishable
    );
    assert_eq!(
        cycle_evidence.compare(&path_evidence).unwrap(),
        GraphEvidenceComparison::Different
    );

    let cycle_analysis = f251.analyze(&cycle).unwrap();
    let mut duplicate = MultiFieldGraphEvidenceBuilder::new();
    duplicate.add(cycle_analysis.signature()).unwrap();
    assert!(matches!(
        duplicate.add(cycle_analysis.signature()),
        Err(GraphError::DuplicateEvidenceChannel)
    ));
    assert_eq!(
        MultiFieldGraphEvidenceBuilder::new().build(),
        Err(GraphError::EmptyEvidenceProfile)
    );

    let path_analysis = f251.analyze(&path).unwrap();
    let mut incompatible_graphs = MultiFieldGraphEvidenceBuilder::new();
    incompatible_graphs.add(cycle_analysis.signature()).unwrap();
    assert!(matches!(
        incompatible_graphs.add(path_analysis.signature()),
        Err(GraphError::EvidenceGraphMetadataMismatch)
    ));

    let mut one_channel = MultiFieldGraphEvidenceBuilder::new();
    one_channel.add(cycle_analysis.signature()).unwrap();
    assert_eq!(
        cycle_evidence.compare(&one_channel.build().unwrap()),
        Err(GraphError::EvidenceProfileMismatch)
    );
}
