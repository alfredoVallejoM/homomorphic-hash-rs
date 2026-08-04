//! G11 contracts for non-authoritative graph invariant channels.

use homomorphic_hash_rs::{
    CellMomentProfile, ClosedWalkAnalysisStatus, ClosedWalkOperator, ClosedWalkQueryPlan,
    DegreeHistogramProfile, DomainSeparatedHashToFieldEncoder, GraphError, GraphFieldChannel,
    GraphFieldSuitability, IncidenceGraph, IncidenceGraphBuilder, LoopPatternCatalog,
    MatrixAnalysisStatus, PatternAnalysisStatus, PatternFieldFingerprint,
    PatternProductFingerprint, PrimeIntegerEncoder, RelationalClosedWalkProfile,
    RelationalMatrixProfile, RelationalThetaProfile, SignatureAssurance, StaticGraphFieldProfile,
    ThetaAnalysisStatus, VertexId,
};
use microfield::{Field, Fp251V1, FpGoldilocks64V1, Gf2_256HhV1};
use structural_field_fixture::Gf2_9StructuralFixture;

const PROFILE: [u8; 32] = [0x47; 32];

fn path(order: usize) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..order)
        .map(|_| builder.add_vertex(b"v".to_vec()))
        .collect::<Vec<_>>();
    for edge in vertices.windows(2) {
        builder
            .add_undirected_relation(edge[0], edge[1], b"edge".to_vec(), b"support".to_vec(), 1)
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
                    b"edge".to_vec(),
                    b"support".to_vec(),
                    1,
                )
                .unwrap();
        }
        offset += length;
    }
    builder.build().unwrap()
}

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

fn disjoint_union(left: &IncidenceGraph, right: &IncidenceGraph) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    for graph in [left, right] {
        for index in 0..graph.vertex_count() {
            let vertex = VertexId::new(index);
            builder.add_typed_vertex(
                graph.vertex_kind(vertex),
                graph.vertex_label(vertex).to_vec(),
            );
        }
    }
    for (graph, offset) in [(left, 0), (right, left.vertex_count())] {
        for source in 0..graph.vertex_count() {
            for incidence in graph.outgoing(VertexId::new(source)) {
                let descriptor = graph.relation(incidence.relation());
                builder
                    .add_directed_relation(
                        VertexId::new(offset + source),
                        VertexId::new(offset + incidence.neighbor().index()),
                        descriptor.relation().to_vec(),
                        descriptor.role().to_vec(),
                        incidence.multiplicity(),
                    )
                    .unwrap();
            }
        }
    }
    builder.build().unwrap()
}

fn degree_offsets() -> [Fp251V1; 3] {
    [
        Fp251V1::ONE,
        Fp251V1::from_u64_mod(2),
        Fp251V1::from_u64_mod(3),
    ]
}

#[test]
fn degree_multiset_exposes_the_exact_ordinary_histogram_and_is_relabeling_invariant() {
    let graph = path(5);
    let permuted = relabel(&graph, &[3, 0, 4, 1, 2]);
    let encoder = PrimeIntegerEncoder::new(0x4445_4752_4545_0001);
    let original =
        DegreeHistogramProfile::<Fp251V1, _, 3>::analyze(&graph, encoder, degree_offsets())
            .unwrap();
    let relabeled =
        DegreeHistogramProfile::<Fp251V1, _, 3>::analyze(&permuted, encoder, degree_offsets())
            .unwrap();

    assert_eq!(original, relabeled);
    assert_eq!(original.support().vertex_count_at(0), 0);
    assert_eq!(original.support().vertex_count_at(1), 2);
    assert_eq!(original.support().vertex_count_at(2), 3);
    assert_eq!(original.outgoing_records(), original.support());
    assert_eq!(original.incoming_records(), original.support());
    assert_eq!(original.vertex_count(), 5);
    assert_eq!(original.assurance(), SignatureAssurance::Fingerprint);
    assert!(original.to_canonical_bytes().starts_with(b"MFDH"));
}

#[test]
fn degree_multiset_is_disjoint_composable_and_identity_bound() {
    let left = path(3);
    let right = cycles(&[3]);
    let union = disjoint_union(&left, &right);
    let encoder = PrimeIntegerEncoder::new(0x4445_4752_4545_0002);
    let left_profile =
        DegreeHistogramProfile::<Fp251V1, _, 3>::analyze(&left, encoder, degree_offsets()).unwrap();
    let right_profile =
        DegreeHistogramProfile::<Fp251V1, _, 3>::analyze(&right, encoder, degree_offsets())
            .unwrap();
    let direct =
        DegreeHistogramProfile::<Fp251V1, _, 3>::analyze(&union, encoder, degree_offsets())
            .unwrap();
    let combined = left_profile.combine_disjoint(&right_profile).unwrap();
    assert_eq!(combined.support(), direct.support());
    assert_eq!(combined.outgoing_records(), direct.outgoing_records());
    assert_eq!(combined.incoming_records(), direct.incoming_records());
    assert_eq!(
        combined.outgoing_multiplicity(),
        direct.outgoing_multiplicity()
    );
    assert_eq!(
        combined.incoming_multiplicity(),
        direct.incoming_multiplicity()
    );
    assert_eq!(combined.joint_fingerprint(), direct.joint_fingerprint());
    assert_eq!(direct.graph_count(), 1);
    assert_eq!(combined.graph_count(), 2);

    let incompatible = DegreeHistogramProfile::<Fp251V1, _, 3>::analyze(
        &right,
        encoder,
        [
            Fp251V1::ONE,
            Fp251V1::from_u64_mod(2),
            Fp251V1::from_u64_mod(4),
        ],
    )
    .unwrap();
    assert_eq!(
        left_profile.combine_disjoint(&incompatible),
        Err(GraphError::DegreeHistogramProfileMismatch)
    );
}

#[test]
fn degree_multiset_distinguishes_support_records_and_exact_multiplicity() {
    let mut builder = IncidenceGraphBuilder::new();
    let a = builder.add_vertex(b"a".to_vec());
    let b = builder.add_vertex(b"b".to_vec());
    let c = builder.add_vertex(b"c".to_vec());
    builder
        .add_directed_relation(a, b, b"r".to_vec(), Vec::new(), 3)
        .unwrap();
    builder
        .add_directed_relation(c, a, b"r".to_vec(), Vec::new(), 2)
        .unwrap();
    builder
        .add_directed_relation(a, a, b"loop".to_vec(), Vec::new(), 4)
        .unwrap();
    let graph = builder.build().unwrap();
    let profile = DegreeHistogramProfile::<Fp251V1, _, 3>::analyze(
        &graph,
        PrimeIntegerEncoder::new(0x4445_4752_4545_0003),
        degree_offsets(),
    )
    .unwrap();

    assert_eq!(profile.support().vertex_count_at(1), 2);
    assert_eq!(profile.support().vertex_count_at(2), 1);
    assert_eq!(profile.outgoing_records().vertex_count_at(0), 1);
    assert_eq!(profile.outgoing_records().vertex_count_at(1), 1);
    assert_eq!(profile.outgoing_records().vertex_count_at(2), 1);
    assert_eq!(profile.outgoing_multiplicity().vertex_count_at(0), 1);
    assert_eq!(profile.outgoing_multiplicity().vertex_count_at(2), 1);
    assert_eq!(profile.outgoing_multiplicity().vertex_count_at(7), 1);
    assert_eq!(profile.incoming_multiplicity().vertex_count_at(0), 1);
    assert_eq!(profile.incoming_multiplicity().vertex_count_at(3), 1);
    assert_eq!(profile.incoming_multiplicity().vertex_count_at(6), 1);
}

#[test]
fn connected_pattern_catalog_is_relabeling_invariant_and_counts_path_subpatterns() {
    let graph = path(4);
    let permuted = relabel(&graph, &[2, 0, 3, 1]);
    let catalog = LoopPatternCatalog::l0_to_l3();
    let original = catalog.analyze(&graph, 10_000).unwrap();
    let relabeled = catalog.analyze(&permuted, 10_000).unwrap();

    assert_eq!(original.status(), PatternAnalysisStatus::Complete);
    assert_eq!(original, relabeled);
    assert_eq!(original.assurance(), SignatureAssurance::ExactTracked);
    assert_eq!(
        original
            .counts()
            .iter()
            .map(|pattern| (pattern.order(), pattern.loop_order(), pattern.count()))
            .collect::<Vec<_>>(),
        vec![(1, 0, 4), (2, 0, 3), (3, 0, 2), (4, 0, 1)]
    );
    assert!(original.to_canonical_bytes().starts_with(b"MFPC"));
}

#[test]
fn pattern_catalog_preserves_direction_role_multiplicity_and_loops() {
    fn relation(reverse: bool, role: &[u8], multiplicity: u64) -> IncidenceGraph {
        let mut builder = IncidenceGraphBuilder::new();
        let left = builder.add_vertex(b"left".to_vec());
        let right = builder.add_vertex(b"right".to_vec());
        let (source, target) = if reverse {
            (right, left)
        } else {
            (left, right)
        };
        builder
            .add_directed_relation(
                source,
                target,
                b"relation".to_vec(),
                role.to_vec(),
                multiplicity,
            )
            .unwrap();
        builder.build().unwrap()
    }

    let catalog = LoopPatternCatalog::l0_to_l3();
    let baseline = catalog.analyze(&relation(false, b"a", 1), 1_000).unwrap();
    let reverse = catalog.analyze(&relation(true, b"a", 1), 1_000).unwrap();
    let role = catalog.analyze(&relation(false, b"b", 1), 1_000).unwrap();
    let multiplicity = catalog.analyze(&relation(false, b"a", 2), 1_000).unwrap();
    assert_ne!(baseline, reverse);
    assert_ne!(baseline, role);
    assert_ne!(baseline, multiplicity);

    let triangle = LoopPatternCatalog::new(3, 1)
        .unwrap()
        .analyze(&cycles(&[3]), 10_000)
        .unwrap();
    assert!(triangle
        .counts()
        .iter()
        .any(|pattern| pattern.order() == 3 && pattern.loop_order() == 1));
}

#[test]
fn pattern_analysis_is_atomic_under_budget_and_disjoint_counts_add() {
    let catalog = LoopPatternCatalog::l0_to_l3();
    let graph = path(6);
    let skipped = catalog.analyze(&graph, 1).unwrap();
    assert_eq!(skipped.status(), PatternAnalysisStatus::SkippedBudget);
    assert!(skipped.counts().is_empty());
    let encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 1);
    assert_eq!(
        PatternFieldFingerprint::<Fp251V1, 3>::from_profile(&skipped, &encoder),
        Err(GraphError::PatternAnalysisIncomplete)
    );
    let complete = catalog.analyze(&path(2), 1_000).unwrap();
    let no_lanes = DomainSeparatedHashToFieldEncoder::<0>::new(PROFILE, 1);
    assert_eq!(
        PatternFieldFingerprint::<Fp251V1, 0>::from_profile(&complete, &no_lanes),
        Err(GraphError::InvalidPatternFingerprint)
    );
    assert_eq!(
        PatternProductFingerprint::<Fp251V1, 0>::from_profile(&complete, &no_lanes),
        Err(GraphError::InvalidPatternFingerprint)
    );

    let left = path(3);
    let right = cycles(&[3]);
    let union = disjoint_union(&left, &right);
    let left_profile = catalog.analyze(&left, 10_000).unwrap();
    let right_profile = catalog.analyze(&right, 10_000).unwrap();
    let combined = left_profile.combine_disjoint(&right_profile).unwrap();
    let direct = catalog.analyze(&union, 100_000).unwrap();
    assert_eq!(combined.counts(), direct.counts());
    assert_eq!(combined.graph_count(), 2);
    assert_eq!(combined.vertex_count(), 6);

    let left_fingerprint =
        PatternFieldFingerprint::<Fp251V1, 3>::from_profile(&left_profile, &encoder).unwrap();
    let right_fingerprint =
        PatternFieldFingerprint::<Fp251V1, 3>::from_profile(&right_profile, &encoder).unwrap();
    let combined_fingerprint = left_fingerprint
        .combine_disjoint(&right_fingerprint)
        .unwrap();
    let direct_fingerprint =
        PatternFieldFingerprint::<Fp251V1, 3>::from_profile(&direct, &encoder).unwrap();
    assert_eq!(combined_fingerprint.lanes(), direct_fingerprint.lanes());
    assert_eq!(
        combined_fingerprint.assurance(),
        SignatureAssurance::Fingerprint
    );
    assert!(combined_fingerprint
        .to_canonical_bytes()
        .starts_with(b"MFPF"));

    let left_product =
        PatternProductFingerprint::<Gf2_256HhV1, 3>::from_profile(&left_profile, &encoder).unwrap();
    let right_product =
        PatternProductFingerprint::<Gf2_256HhV1, 3>::from_profile(&right_profile, &encoder)
            .unwrap();
    let direct_product =
        PatternProductFingerprint::<Gf2_256HhV1, 3>::from_profile(&direct, &encoder).unwrap();
    assert_eq!(
        left_product
            .combine_disjoint(&right_product)
            .unwrap()
            .evaluated_products(),
        direct_product.evaluated_products()
    );
    assert!(direct_product.to_canonical_bytes().starts_with(b"MFPP"));
}

#[test]
fn cell_moments_are_relabeling_invariant_composable_and_profile_bound() {
    let left = path(3);
    let right = cycles(&[3]);
    let union = disjoint_union(&left, &right);
    let permuted = relabel(&union, &[5, 2, 0, 4, 1, 3]);
    let encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 2);
    let direct =
        CellMomentProfile::<FpGoldilocks64V1, 3, 4>::analyze_initial(&union, &encoder).unwrap();
    let relabeled =
        CellMomentProfile::<FpGoldilocks64V1, 3, 4>::analyze_initial(&permuted, &encoder).unwrap();
    assert_eq!(direct, relabeled);
    assert_eq!(direct.value_count(), 6);
    assert_eq!(direct.assurance(), SignatureAssurance::Fingerprint);
    assert!(direct.to_canonical_bytes().starts_with(b"MFCM"));

    let left_profile =
        CellMomentProfile::<FpGoldilocks64V1, 3, 4>::analyze_initial(&left, &encoder).unwrap();
    let right_profile =
        CellMomentProfile::<FpGoldilocks64V1, 3, 4>::analyze_initial(&right, &encoder).unwrap();
    assert_eq!(
        left_profile.combine_disjoint(&right_profile).unwrap(),
        direct
    );

    let other = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 3);
    let incompatible =
        CellMomentProfile::<FpGoldilocks64V1, 3, 4>::analyze_initial(&right, &other).unwrap();
    assert_eq!(
        left_profile.combine_disjoint(&incompatible),
        Err(GraphError::CellMomentProfileMismatch)
    );
}

#[test]
fn relational_matrix_channels_obey_block_diagonal_laws_and_budget_atomicity() {
    let left = path(3);
    let right = cycles(&[3]);
    let union = disjoint_union(&left, &right);
    let encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 4);
    let left_profile =
        RelationalMatrixProfile::<FpGoldilocks64V1, 3>::analyze(&left, 5, &encoder, 100_000)
            .unwrap();
    let right_profile =
        RelationalMatrixProfile::<FpGoldilocks64V1, 3>::analyze(&right, 5, &encoder, 100_000)
            .unwrap();
    let direct =
        RelationalMatrixProfile::<FpGoldilocks64V1, 3>::analyze(&union, 5, &encoder, 100_000)
            .unwrap();
    let combined = left_profile.combine_disjoint(&right_profile).unwrap();
    assert_eq!(combined.traces(), direct.traces());
    assert_eq!(
        combined.characteristic_evaluations(),
        direct.characteristic_evaluations()
    );
    assert_eq!(combined.vertex_count(), 6);
    assert_eq!(combined.graph_count(), 2);
    assert_eq!(combined.assurance(), SignatureAssurance::Fingerprint);
    assert!(combined.to_canonical_bytes().starts_with(b"MFRM"));

    let skipped =
        RelationalMatrixProfile::<FpGoldilocks64V1, 3>::analyze(&union, 5, &encoder, 1).unwrap();
    assert_eq!(skipped.status(), MatrixAnalysisStatus::SkippedBudget);
    assert!(skipped.traces().is_empty());
    assert_eq!(skipped.characteristic_evaluations(), None);
    assert_eq!(
        skipped.combine_disjoint(&direct),
        Err(GraphError::MatrixAnalysisIncomplete)
    );
}

#[test]
fn rg2_theta_contractions_are_invariant_composable_and_budget_atomic() {
    let left = path(3);
    let right = cycles(&[3]);
    let union = disjoint_union(&left, &right);
    let permuted = relabel(&union, &[4, 1, 5, 0, 3, 2]);
    let encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 41);
    let direct =
        RelationalThetaProfile::<FpGoldilocks64V1, 3>::analyze(&union, &encoder, 100_000).unwrap();
    let relabeled =
        RelationalThetaProfile::<FpGoldilocks64V1, 3>::analyze(&permuted, &encoder, 100_000)
            .unwrap();
    assert_eq!(direct, relabeled);
    let left_profile =
        RelationalThetaProfile::<FpGoldilocks64V1, 3>::analyze(&left, &encoder, 100_000).unwrap();
    let right_profile =
        RelationalThetaProfile::<FpGoldilocks64V1, 3>::analyze(&right, &encoder, 100_000).unwrap();
    assert_eq!(
        left_profile
            .combine_disjoint(&right_profile)
            .unwrap()
            .contractions(),
        direct.contractions()
    );
    assert_eq!(direct.assurance(), SignatureAssurance::Fingerprint);
    assert!(direct.to_canonical_bytes().starts_with(b"MFTH"));

    let skipped =
        RelationalThetaProfile::<FpGoldilocks64V1, 3>::analyze(&union, &encoder, 1).unwrap();
    assert_eq!(skipped.status(), ThetaAnalysisStatus::SkippedBudget);
    assert_eq!(skipped.contractions(), &[[FpGoldilocks64V1::ZERO; 3]; 6]);
    assert_eq!(
        skipped.combine_disjoint(&direct),
        Err(GraphError::ThetaAnalysisIncomplete)
    );
}

#[test]
fn matrix_and_pattern_channels_separate_a_six_cycle_from_two_triangles() {
    let cycle = cycles(&[6]);
    let triangles = cycles(&[3, 3]);
    let catalog = LoopPatternCatalog::l0_to_l3();
    assert_ne!(
        catalog.analyze(&cycle, 100_000).unwrap().counts(),
        catalog.analyze(&triangles, 100_000).unwrap().counts()
    );

    let encoder = DomainSeparatedHashToFieldEncoder::<4>::new(PROFILE, 5);
    let cycle_patterns = catalog.analyze(&cycle, 100_000).unwrap();
    let triangle_patterns = catalog.analyze(&triangles, 100_000).unwrap();
    assert_ne!(
        PatternProductFingerprint::<Gf2_256HhV1, 4>::from_profile(&cycle_patterns, &encoder,)
            .unwrap()
            .evaluated_products(),
        PatternProductFingerprint::<Gf2_256HhV1, 4>::from_profile(&triangle_patterns, &encoder,)
            .unwrap()
            .evaluated_products()
    );
    let cycle_matrix =
        RelationalMatrixProfile::<Fp251V1, 4>::analyze(&cycle, 6, &encoder, 100_000).unwrap();
    let triangles_matrix =
        RelationalMatrixProfile::<Fp251V1, 4>::analyze(&triangles, 6, &encoder, 100_000).unwrap();
    assert_ne!(cycle_matrix.traces(), triangles_matrix.traces());
    let cycle_theta =
        RelationalThetaProfile::<Fp251V1, 4>::analyze(&cycle, &encoder, 100_000).unwrap();
    let triangles_theta =
        RelationalThetaProfile::<Fp251V1, 4>::analyze(&triangles, &encoder, 100_000).unwrap();
    assert_ne!(cycle_theta.contractions(), triangles_theta.contractions());
}

#[test]
fn every_g11_channel_is_invariant_over_all_target_field_families() {
    fn check<F>()
    where
        F: microfield::Field
            + microfield::CanonicalEncoding
            + microfield::Invert
            + microfield::StaticField
            + core::fmt::Debug,
    {
        let graph = cycles(&[5]);
        let permuted = relabel(&graph, &[3, 0, 4, 1, 2]);
        let encoder = DomainSeparatedHashToFieldEncoder::<2>::new(PROFILE, 9);
        let moments = CellMomentProfile::<F, 2, 3>::analyze_initial(&graph, &encoder).unwrap();
        let relabeled_moments =
            CellMomentProfile::<F, 2, 3>::analyze_initial(&permuted, &encoder).unwrap();
        assert_eq!(moments, relabeled_moments);
        let matrix =
            RelationalMatrixProfile::<F, 2>::analyze(&graph, 4, &encoder, 100_000).unwrap();
        let relabeled_matrix =
            RelationalMatrixProfile::<F, 2>::analyze(&permuted, 4, &encoder, 100_000).unwrap();
        assert_eq!(matrix, relabeled_matrix);
        let theta = RelationalThetaProfile::<F, 2>::analyze(&graph, &encoder, 100_000).unwrap();
        let relabeled_theta =
            RelationalThetaProfile::<F, 2>::analyze(&permuted, &encoder, 100_000).unwrap();
        assert_eq!(theta, relabeled_theta);
        let plan = ClosedWalkQueryPlan::new(vec![1, 3, 64, 1_000_000_000_000]).unwrap();
        let walks = RelationalClosedWalkProfile::<F, 2>::analyze(
            &graph,
            plan.clone(),
            &encoder,
            10_000_000,
        )
        .unwrap();
        let relabeled_walks =
            RelationalClosedWalkProfile::<F, 2>::analyze(&permuted, plan, &encoder, 10_000_000)
                .unwrap();
        assert_eq!(walks, relabeled_walks);
    }

    check::<Fp251V1>();
    check::<FpGoldilocks64V1>();
    check::<Gf2_256HhV1>();
    check::<Gf2_9StructuralFixture>();
}

#[test]
fn long_closed_walk_recurrence_matches_dense_traces_and_jumps_to_u64_lengths() {
    let left = cycles(&[5]);
    let right = path(4);
    let union = disjoint_union(&left, &right);
    let permuted = relabel(&union, &[7, 2, 5, 0, 8, 4, 1, 6, 3]);
    let encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 77);
    let lengths = vec![1, 2, 3, 4, 8, 16, 1_000_000_000_000];
    let plan = ClosedWalkQueryPlan::new(lengths.clone()).unwrap();
    let long = RelationalClosedWalkProfile::<FpGoldilocks64V1, 3>::analyze(
        &union,
        plan.clone(),
        &encoder,
        100_000_000,
    )
    .unwrap();
    let relabeled = RelationalClosedWalkProfile::<FpGoldilocks64V1, 3>::analyze(
        &permuted,
        plan.clone(),
        &encoder,
        100_000_000,
    )
    .unwrap();
    assert_eq!(long, relabeled);
    assert_eq!(long.status(), ClosedWalkAnalysisStatus::Complete);
    assert_eq!(long.traces().len(), lengths.len());
    assert!(long.recurrence_orders().iter().all(|order| *order <= 9));
    assert!(long.to_canonical_bytes().starts_with(b"MFCW"));
    assert_eq!(long.assurance(), SignatureAssurance::Fingerprint);

    let dense =
        RelationalMatrixProfile::<FpGoldilocks64V1, 3>::analyze(&union, 16, &encoder, 100_000_000)
            .unwrap();
    for (query_index, &length) in lengths.iter().enumerate().take(6) {
        assert_eq!(
            long.traces()[query_index],
            dense.traces()[length as usize - 1]
        );
    }

    let left_profile = RelationalClosedWalkProfile::<FpGoldilocks64V1, 3>::analyze(
        &left,
        plan.clone(),
        &encoder,
        100_000_000,
    )
    .unwrap();
    let right_profile = RelationalClosedWalkProfile::<FpGoldilocks64V1, 3>::analyze(
        &right,
        plan.clone(),
        &encoder,
        100_000_000,
    )
    .unwrap();
    assert_eq!(
        left_profile
            .combine_disjoint(&right_profile)
            .unwrap()
            .traces(),
        long.traces()
    );

    let skipped =
        RelationalClosedWalkProfile::<FpGoldilocks64V1, 3>::analyze(&union, plan, &encoder, 1)
            .unwrap();
    assert_eq!(skipped.status(), ClosedWalkAnalysisStatus::SkippedBudget);
    assert!(skipped.traces().is_empty());
    assert_eq!(
        skipped.combine_disjoint(&long),
        Err(GraphError::ClosedWalkAnalysisIncomplete)
    );
}

#[test]
fn graph_field_policy_exposes_characteristic_specific_degeneracy() {
    let prime = StaticGraphFieldProfile::for_field::<FpGoldilocks64V1>();
    let binary = StaticGraphFieldProfile::for_field::<Gf2_256HhV1>();
    assert!(!prime.characteristic_is_two());
    assert!(binary.characteristic_is_two());
    assert_eq!(
        prime.suitability(GraphFieldChannel::ThetaContractions),
        GraphFieldSuitability::Preferred
    );
    assert_eq!(
        binary.suitability(GraphFieldChannel::MultiplicativePatterns),
        GraphFieldSuitability::Preferred
    );
    assert_eq!(
        binary.suitability(GraphFieldChannel::RelationalMatrix),
        GraphFieldSuitability::AvoidForSymmetricGraphs
    );
    assert_eq!(
        binary.suitability(GraphFieldChannel::LongClosedWalks),
        GraphFieldSuitability::CompatibleWithAliasing
    );
    assert_eq!(
        ClosedWalkQueryPlan::new(Vec::new()),
        Err(GraphError::InvalidClosedWalkPlan)
    );
    assert_eq!(
        ClosedWalkQueryPlan::new(vec![0, 1]),
        Err(GraphError::InvalidClosedWalkPlan)
    );
}

#[test]
fn non_backtracking_recurrence_detects_long_cycles_without_tree_bounces() {
    let tree = path(12);
    let cycle = cycles(&[12]);
    let permuted = relabel(&cycle, &[7, 0, 9, 2, 11, 4, 1, 6, 3, 10, 5, 8]);
    let plan = ClosedWalkQueryPlan::new(vec![1, 2, 3, 12, 24, 1_000_000_000_000]).unwrap();
    let encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 91);
    let tree_profile =
        RelationalClosedWalkProfile::<FpGoldilocks64V1, 3>::analyze_non_backtracking(
            &tree,
            plan.clone(),
            &encoder,
            100_000_000,
        )
        .unwrap();
    let cycle_profile =
        RelationalClosedWalkProfile::<FpGoldilocks64V1, 3>::analyze_non_backtracking(
            &cycle,
            plan.clone(),
            &encoder,
            100_000_000,
        )
        .unwrap();
    let relabeled = RelationalClosedWalkProfile::<FpGoldilocks64V1, 3>::analyze_non_backtracking(
        &permuted,
        plan,
        &encoder,
        100_000_000,
    )
    .unwrap();
    assert_eq!(tree_profile.operator(), ClosedWalkOperator::NonBacktracking);
    assert_eq!(cycle_profile, relabeled);
    assert!(tree_profile
        .traces()
        .iter()
        .flatten()
        .all(microfield::Field::is_zero));
    assert!(cycle_profile.traces()[3]
        .iter()
        .any(|value| !value.is_zero()));
    assert_ne!(
        tree_profile.to_canonical_bytes(),
        cycle_profile.to_canonical_bytes()
    );
}

#[cfg(feature = "dynamic-fields")]
#[test]
fn runtime_field_policy_matches_the_generated_external_profile() {
    let runtime = microfield::DynField::builder("runtime_gf2_9")
        .binary(9, vec![9, 4, 0])
        .build()
        .unwrap();
    let dynamic = homomorphic_hash_rs::DynamicGraphFieldProfile::for_field(&runtime);
    let generated = StaticGraphFieldProfile::for_field::<Gf2_9StructuralFixture>();
    assert_eq!(dynamic.field_id(), generated.field_id());
    assert_eq!(dynamic.extension_degree(), generated.extension_degree());
    assert_eq!(
        dynamic.suitability(GraphFieldChannel::RelationalMatrix),
        generated.suitability(GraphFieldChannel::RelationalMatrix)
    );
    assert!(runtime.export_manifest().contains("degree = 9"));
}
