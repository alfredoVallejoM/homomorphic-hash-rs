//! Adversarial and cross-field tests for bounded structural graph refinement.

use std::collections::BTreeMap;

use allocation_counter::measure;
use homomorphic_hash_rs::{
    from_legacy_topology, BinaryPolynomialEncoder, CellularGaloisCanonizer,
    F251BatchGraphWorkspace, F251GraphLabeler, FastGraphAnalysis, FastGraphLabeler,
    GaloisSignature256, GraphError, GraphExecution, GraphWorkspace, HyperedgeIncidence,
    IncidenceGraph, IncidenceGraphBuilder, IncrementalGraphWorkspace, PrimeIntegerEncoder,
    RefinementProfile, TopologyProvider, TryCanonicalOutcome, VertexId, VertexKind,
};
use microfield::{BackendId, CpuCapabilities, Fp251V1, FpGoldilocks64V1, Gf2_256HhV1};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use structural_field_fixture::Gf2_9StructuralFixture;

const GRAPH_DOMAIN: u64 = 0x4752_4150_485f_0001;

fn directed_fixture() -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let carbon = builder.add_vertex(b"C".to_vec());
    let oxygen = builder.add_vertex(b"O".to_vec());
    let nitrogen = builder.add_vertex(b"N".to_vec());
    builder
        .add_directed_relation(carbon, oxygen, b"bond".to_vec(), b"donor".to_vec(), 2)
        .unwrap();
    builder
        .add_directed_relation(oxygen, nitrogen, b"bond".to_vec(), b"acceptor".to_vec(), 1)
        .unwrap();
    builder
        .add_directed_relation(nitrogen, nitrogen, b"loop".to_vec(), Vec::new(), 3)
        .unwrap();
    builder.build().unwrap()
}

fn relabel(graph: &IncidenceGraph, new_to_old: &[usize]) -> (IncidenceGraph, Vec<usize>) {
    assert_eq!(new_to_old.len(), graph.vertex_count());
    let mut old_to_new = vec![usize::MAX; graph.vertex_count()];
    let mut builder = IncidenceGraphBuilder::new();
    for (new, &old) in new_to_old.iter().enumerate() {
        assert!(old < graph.vertex_count());
        assert_eq!(old_to_new[old], usize::MAX);
        old_to_new[old] = new;
        let old = VertexId::new(old);
        builder.add_typed_vertex(graph.vertex_kind(old), graph.vertex_label(old).to_vec());
    }
    for old_source in 0..graph.vertex_count() {
        for incidence in graph.outgoing(VertexId::new(old_source)) {
            let descriptor = graph.relation(incidence.relation());
            builder
                .add_directed_relation(
                    VertexId::new(old_to_new[old_source]),
                    VertexId::new(old_to_new[incidence.neighbor().index()]),
                    descriptor.relation().to_vec(),
                    descriptor.role().to_vec(),
                    incidence.multiplicity(),
                )
                .unwrap();
        }
    }
    (builder.build().unwrap(), old_to_new)
}

fn assert_equivariant<F, const K: usize>(
    original: &FastGraphAnalysis<F, K>,
    permuted: &FastGraphAnalysis<F, K>,
    old_to_new: &[usize],
) where
    F: microfield::Field + core::fmt::Debug,
{
    assert_eq!(original.signature(), permuted.signature());
    for (old, &new) in old_to_new.iter().enumerate() {
        assert_eq!(original.labels()[old], permuted.labels()[new]);
    }
}

#[test]
fn normalization_is_transactional_and_compresses_exact_duplicates() {
    let mut builder = IncidenceGraphBuilder::new();
    let left = builder.add_vertex(b"left".to_vec());
    let right = builder.add_vertex(b"right".to_vec());
    builder
        .add_directed_relation(left, right, b"r".to_vec(), b"p".to_vec(), 2)
        .unwrap();
    builder
        .add_directed_relation(left, right, b"r".to_vec(), b"p".to_vec(), 3)
        .unwrap();
    assert_eq!(
        builder.add_directed_relation(left, right, b"r".to_vec(), Vec::new(), 0),
        Err(GraphError::ZeroMultiplicity)
    );
    assert!(matches!(
        builder.add_directed_relation(left, VertexId::new(99), b"r".to_vec(), Vec::new(), 1),
        Err(GraphError::InvalidVertex { .. })
    ));

    let graph = builder.build().unwrap();
    assert_eq!(graph.vertex_count(), 2);
    assert_eq!(graph.incidence_count(), 1);
    assert_eq!(graph.total_multiplicity(), 5);
    assert_eq!(graph.outgoing(left)[0].multiplicity(), 5);
    assert_eq!(graph.incoming(right)[0].multiplicity(), 5);
}

#[test]
fn legacy_adapter_preserves_clauses_and_repeated_membership_as_incidences() {
    struct Legacy;

    impl TopologyProvider for Legacy {
        fn num_variables(&self) -> usize {
            2
        }

        fn num_clauses(&self) -> usize {
            1
        }

        fn variables_in_clause(&self, _clause_index: usize) -> Vec<usize> {
            vec![0, 1, 1]
        }

        fn clauses_for_variable(&self, variable_index: usize) -> Vec<usize> {
            match variable_index {
                0 | 1 => vec![0],
                _ => Vec::new(),
            }
        }

        fn initial_state(&self, variable_index: usize) -> Option<GaloisSignature256> {
            (variable_index == 0).then_some(GaloisSignature256([7, 0, 0, 0]))
        }
    }

    let graph = from_legacy_topology(&Legacy).unwrap();
    assert_eq!(graph.vertex_count(), 3);
    assert_eq!(graph.incidence_count(), 4);
    assert_eq!(graph.total_multiplicity(), 6);
    assert_eq!(graph.outgoing(VertexId::new(1))[0].multiplicity(), 2);

    let migrated = CellularGaloisCanonizer::try_analyze(&Legacy, 4).unwrap();
    let direct = F251GraphLabeler::<3>::f251(RefinementProfile::Fast { rounds: 4 })
        .unwrap()
        .analyze(&graph)
        .unwrap();
    assert_eq!(migrated.graph(), &graph);
    assert_eq!(migrated.structural(), &direct);
}

#[test]
fn f251_fast_profile_is_relabeling_invariant() {
    let graph = directed_fixture();
    let (permuted, old_to_new) = relabel(&graph, &[2, 0, 1]);
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 7 },
    )
    .unwrap();

    let original = labeler.analyze(&graph).unwrap();
    let relabeled = labeler.analyze(&permuted).unwrap();
    assert_equivariant(&original, &relabeled, &old_to_new);
    assert_eq!(original.signature().rounds(), 7);
    assert_eq!(original.signature().vertex_count(), 3);
    assert_eq!(original.signature().incidence_count(), 3);
    assert_eq!(original.signature().total_multiplicity(), 6);
    assert!(original
        .signature()
        .to_canonical_bytes()
        .starts_with(b"MFGR"));
}

#[test]
fn hybrid_sha256_channel_is_invariant_and_not_a_hash_of_the_field_signature() {
    let graph = directed_fixture();
    let (permuted, _) = relabel(&graph, &[2, 0, 1]);
    let labeler = FastGraphLabeler::<Fp251V1, _, 1>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 1 },
    )
    .unwrap();
    let original = labeler.analyze_hybrid(&graph).unwrap();
    let relabeled = labeler.analyze_hybrid(&permuted).unwrap();
    assert_eq!(
        original.structural().signature(),
        relabeled.structural().signature()
    );
    assert_eq!(original.invariant_digest(), relabeled.invariant_digest());

    // Pigeonhole: 256 exact one-byte labels enter a one-lane field with only
    // 251 values, so at least one complete structural signature must collide.
    let mut observed = BTreeMap::new();
    let mut separated_collision = None;
    for exact_label in 0_u16..=255 {
        let mut builder = IncidenceGraphBuilder::new();
        builder.add_vertex(vec![u8::try_from(exact_label).unwrap()]);
        let singleton = builder.build().unwrap();
        let hybrid = labeler.analyze_hybrid(&singleton).unwrap();
        let structural = hybrid.structural().signature().to_canonical_bytes();
        if let Some((previous_label, previous_digest)) =
            observed.insert(structural, (exact_label, hybrid.invariant_digest()))
        {
            if previous_label != exact_label {
                separated_collision = Some((previous_digest, hybrid.invariant_digest()));
                break;
            }
        }
    }
    let (left, right) = separated_collision.expect("F251 pigeonhole collision must exist");
    assert_ne!(left, right);
}

#[test]
fn many_deterministic_random_relabelings_preserve_labels_and_signature() {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = (0_u8..32)
        .map(|index| builder.add_vertex(vec![b'A' + index % 5]))
        .collect();
    for source in 0..vertices.len() {
        for step in [1_usize, 3, 7] {
            let target = (source * 11 + step) % vertices.len();
            builder
                .add_directed_relation(
                    vertices[source],
                    vertices[target],
                    vec![b'r', u8::try_from(step).unwrap()],
                    vec![u8::try_from(source % 3).unwrap()],
                    u64::try_from(source % 4 + 1).unwrap(),
                )
                .unwrap();
        }
    }
    let graph = builder.build().unwrap();
    let labeler = FastGraphLabeler::<Gf2_256HhV1, _, 3>::new(
        BinaryPolynomialEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 5 },
    )
    .unwrap();
    let expected = labeler.analyze(&graph).unwrap();
    let mut permutation: Vec<usize> = (0..graph.vertex_count()).collect();
    let mut rng = StdRng::seed_from_u64(0x5eed_cafe_f00d_beef);
    for _ in 0..64 {
        permutation.shuffle(&mut rng);
        let (permuted, old_to_new) = relabel(&graph, &permutation);
        let actual = labeler.analyze(&permuted).unwrap();
        assert_equivariant(&expected, &actual, &old_to_new);
    }
}

#[test]
fn maintained_binary_and_prime_fields_share_the_algorithm_not_the_identity() {
    let graph = directed_fixture();
    let binary = FastGraphLabeler::<Gf2_256HhV1, _, 3>::new(
        BinaryPolynomialEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::fast(),
    )
    .unwrap();
    let prime = FastGraphLabeler::<FpGoldilocks64V1, _, 3>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::fast(),
    )
    .unwrap();

    let binary_result = binary.analyze(&graph).unwrap();
    let prime_result = prime.analyze(&graph).unwrap();
    assert_ne!(
        binary_result.signature().signature_id().as_bytes(),
        prime_result.signature().signature_id().as_bytes()
    );
    assert_eq!(binary_result.signature().vertex_count(), 3);
    assert_eq!(prime_result.signature().vertex_count(), 3);
}

#[test]
fn fixed_round_signature_is_homomorphic_over_disjoint_union() {
    fn left_component() -> IncidenceGraph {
        let mut builder = IncidenceGraphBuilder::new();
        let a = builder.add_vertex(b"a".to_vec());
        let b = builder.add_vertex(b"b".to_vec());
        builder
            .add_directed_relation(a, b, b"left".to_vec(), Vec::new(), 2)
            .unwrap();
        builder.build().unwrap()
    }

    fn right_component() -> IncidenceGraph {
        let mut builder = IncidenceGraphBuilder::new();
        let c = builder.add_vertex(b"c".to_vec());
        let d = builder.add_vertex(b"d".to_vec());
        let e = builder.add_vertex(b"e".to_vec());
        builder
            .add_directed_relation(c, d, b"right".to_vec(), b"x".to_vec(), 1)
            .unwrap();
        builder
            .add_directed_relation(d, e, b"right".to_vec(), b"y".to_vec(), 3)
            .unwrap();
        builder.build().unwrap()
    }

    fn disjoint_union() -> IncidenceGraph {
        let mut builder = IncidenceGraphBuilder::new();
        let a = builder.add_vertex(b"a".to_vec());
        let b = builder.add_vertex(b"b".to_vec());
        let c = builder.add_vertex(b"c".to_vec());
        let d = builder.add_vertex(b"d".to_vec());
        let e = builder.add_vertex(b"e".to_vec());
        builder
            .add_directed_relation(a, b, b"left".to_vec(), Vec::new(), 2)
            .unwrap();
        builder
            .add_directed_relation(c, d, b"right".to_vec(), b"x".to_vec(), 1)
            .unwrap();
        builder
            .add_directed_relation(d, e, b"right".to_vec(), b"y".to_vec(), 3)
            .unwrap();
        builder.build().unwrap()
    }

    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 5 },
    )
    .unwrap();
    let left = labeler.analyze(&left_component()).unwrap();
    let right = labeler.analyze(&right_component()).unwrap();
    let union = labeler.analyze(&disjoint_union()).unwrap();
    let combined = labeler
        .combine_disjoint(left.signature(), right.signature())
        .unwrap();
    assert_eq!(&combined, union.signature());
    assert_eq!(combined.vertex_count(), 5);
    assert_eq!(combined.incidence_count(), 3);
    assert_eq!(combined.total_multiplicity(), 6);

    let incompatible = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 6 },
    )
    .unwrap();
    let incompatible_signature = incompatible.analyze(&left_component()).unwrap();
    assert_eq!(
        labeler.combine_disjoint(left.signature(), incompatible_signature.signature()),
        Err(GraphError::SignatureIdentityMismatch)
    );

    let robust = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::robust(),
    )
    .unwrap();
    assert_eq!(
        robust.combine_disjoint(left.signature(), right.signature()),
        Err(GraphError::NonComposableProfile)
    );
}

#[test]
fn externally_generated_binary_field_runs_the_same_relabeling_contract() {
    let graph = directed_fixture();
    let (permuted, old_to_new) = relabel(&graph, &[1, 2, 0]);
    let labeler = FastGraphLabeler::<Gf2_9StructuralFixture, _, 2>::new(
        BinaryPolynomialEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 5 },
    )
    .unwrap();

    let original = labeler.analyze(&graph).unwrap();
    let relabeled = labeler.analyze(&permuted).unwrap();
    assert_equivariant(&original, &relabeled, &old_to_new);
}

#[test]
fn direction_role_and_multiplicity_are_not_squashed() {
    fn graph(reverse: bool, role: &[u8], multiplicity: u64) -> IncidenceGraph {
        let mut builder = IncidenceGraphBuilder::new();
        let a = builder.add_vertex(b"a".to_vec());
        let b = builder.add_vertex(b"b".to_vec());
        let (source, target) = if reverse { (b, a) } else { (a, b) };
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

    let labeler = FastGraphLabeler::<Gf2_256HhV1, _, 3>::new(
        BinaryPolynomialEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 4 },
    )
    .unwrap();
    let baseline = labeler.analyze(&graph(false, b"left", 1)).unwrap();
    let reversed = labeler.analyze(&graph(true, b"left", 1)).unwrap();
    let other_role = labeler.analyze(&graph(false, b"right", 1)).unwrap();
    let repeated = labeler.analyze(&graph(false, b"left", 2)).unwrap();

    assert_ne!(baseline.signature(), reversed.signature());
    assert_ne!(baseline.signature(), other_role.signature());
    assert_ne!(baseline.signature(), repeated.signature());
}

#[test]
fn hyperedges_remain_linear_incidence_nodes_and_are_relabeling_invariant() {
    let mut builder = IncidenceGraphBuilder::new();
    let a = builder.add_vertex(b"a".to_vec());
    let b = builder.add_vertex(b"b".to_vec());
    let c = builder.add_vertex(b"c".to_vec());
    let hyperedge = builder
        .add_hyperedge(
            b"reaction".to_vec(),
            &[
                HyperedgeIncidence::new(a, b"reactant-0".to_vec()),
                HyperedgeIncidence::new(b, b"reactant-1".to_vec()),
                HyperedgeIncidence::new(c, b"product".to_vec()).with_multiplicity(2),
            ],
        )
        .unwrap();
    let graph = builder.build().unwrap();
    assert_eq!(graph.vertex_count(), 4);
    assert_eq!(hyperedge.index(), 3);
    assert_eq!(graph.incidence_count(), 6);
    assert_eq!(graph.total_multiplicity(), 8);

    let (permuted, old_to_new) = relabel(&graph, &[3, 1, 0, 2]);
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::robust(),
    )
    .unwrap();
    let original = labeler.analyze(&graph).unwrap();
    let relabeled = labeler.analyze(&permuted).unwrap();
    assert_equivariant(&original, &relabeled, &old_to_new);
    assert!(original.signature().rounds() <= 16);
}

#[test]
fn bounded_canonicalization_succeeds_only_for_a_discrete_partition() {
    let graph = directed_fixture();
    let (permuted, _) = relabel(&graph, &[2, 0, 1]);
    let labeler = FastGraphLabeler::<Gf2_256HhV1, _, 3>::new(
        BinaryPolynomialEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 6 },
    )
    .unwrap();
    let first = match labeler.try_canonicalize(&graph).unwrap() {
        TryCanonicalOutcome::Canonical(form) => form,
        TryCanonicalOutcome::SymmetryRemaining(_) => panic!("fixture must become discrete"),
    };
    let second = match labeler.try_canonicalize(&permuted).unwrap() {
        TryCanonicalOutcome::Canonical(form) => form,
        TryCanonicalOutcome::SymmetryRemaining(_) => panic!("fixture must become discrete"),
    };
    assert_eq!(first.bytes(), second.bytes());
    assert_eq!(first.original_to_canonical().len(), 3);
    assert_eq!(first.canonical_to_original().len(), 3);

    let mut cycle = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = (0..6).map(|_| cycle.add_vertex(Vec::new())).collect();
    for index in 0..vertices.len() {
        cycle
            .add_undirected_relation(
                vertices[index],
                vertices[(index + 1) % vertices.len()],
                b"edge".to_vec(),
                Vec::new(),
                1,
            )
            .unwrap();
    }
    let cycle = cycle.build().unwrap();
    match labeler.try_canonicalize(&cycle).unwrap() {
        TryCanonicalOutcome::Canonical(_) => panic!("a homogeneous cycle remains symmetric"),
        TryCanonicalOutcome::SymmetryRemaining(analysis) => {
            assert!(analysis.cell_count() < cycle.vertex_count());
        }
    }
}

#[test]
fn profile_and_field_parameters_are_bound_to_signature_identity() {
    let encoder = PrimeIntegerEncoder::new(GRAPH_DOMAIN);
    let fast =
        FastGraphLabeler::<Fp251V1, _, 2>::new(encoder, RefinementProfile::Fast { rounds: 4 })
            .unwrap();
    let longer =
        FastGraphLabeler::<Fp251V1, _, 2>::new(encoder, RefinementProfile::Fast { rounds: 5 })
            .unwrap();
    assert_ne!(fast.signature_id(), longer.signature_id());
    assert_ne!(
        fast.parameters().multiset_offsets()[0],
        fast.parameters().multiset_offsets()[1]
    );
    assert!(fast.parameters().update_bases().iter().all(|base| {
        use microfield::Field as _;
        !base.is_zero() && *base != Fp251V1::ONE
    }));

    assert!(matches!(
        FastGraphLabeler::<Fp251V1, _, 2>::new(encoder, RefinementProfile::Fast { rounds: 0 }),
        Err(GraphError::InvalidProfile)
    ));
}

#[test]
fn prepared_workspace_is_identical_and_allocation_free_after_reservation() {
    let graph = directed_fixture();
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 7 },
    )
    .unwrap();
    let expected = labeler.analyze(&graph).unwrap();
    let prepared = labeler.prepare(&graph).unwrap();
    let mut workspace = GraphWorkspace::new();
    workspace.reserve_for(graph.vertex_count(), 7);

    let first = labeler
        .analyze_prepared_with_workspace(&prepared, &mut workspace, GraphExecution::Sequential)
        .unwrap()
        .to_owned();
    assert_eq!(first, expected);

    let allocations = measure(|| {
        let view = labeler
            .analyze_prepared_with_workspace(&prepared, &mut workspace, GraphExecution::Sequential)
            .unwrap();
        std::hint::black_box(view.signature().lanes());
    });
    assert_eq!(allocations.count_total, 0, "{allocations:?}");
    assert_eq!(allocations.bytes_total, 0, "{allocations:?}");
}

#[test]
fn parallel_vertex_ranges_are_byte_identical_to_sequential_execution() {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = (0_usize..4096)
        .map(|index| builder.add_vertex(index.to_le_bytes().to_vec()))
        .collect();
    for index in 0..vertices.len() {
        for step in [1, 7, 31, 127] {
            builder
                .add_directed_relation(
                    vertices[index],
                    vertices[(index + step) % vertices.len()],
                    b"edge".to_vec(),
                    vec![u8::try_from(step).unwrap()],
                    1,
                )
                .unwrap();
        }
    }
    let graph = builder.build().unwrap();
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 5 },
    )
    .unwrap();
    let prepared = labeler.prepare(&graph).unwrap();
    let mut sequential_workspace = GraphWorkspace::new();
    let sequential = labeler
        .analyze_prepared_with_workspace(
            &prepared,
            &mut sequential_workspace,
            GraphExecution::Sequential,
        )
        .unwrap()
        .to_owned();
    let mut parallel_workspace = GraphWorkspace::new();
    let parallel = labeler
        .analyze_prepared_with_workspace(
            &prepared,
            &mut parallel_workspace,
            GraphExecution::Parallel {
                minimum_vertices: 1,
            },
        )
        .unwrap()
        .to_owned();
    assert_eq!(parallel, sequential);
}

#[test]
fn prepared_graph_identity_mismatch_fails_before_workspace_mutation() {
    let graph = directed_fixture();
    let first = FastGraphLabeler::<Fp251V1, _, 2>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 4 },
    )
    .unwrap();
    let second = FastGraphLabeler::<Fp251V1, _, 2>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN + 1),
        RefinementProfile::Fast { rounds: 4 },
    )
    .unwrap();
    let prepared = first.prepare(&graph).unwrap();
    let mut workspace = GraphWorkspace::new();
    assert!(matches!(
        second.analyze_prepared_with_workspace(
            &prepared,
            &mut workspace,
            GraphExecution::Sequential
        ),
        Err(GraphError::SignatureIdentityMismatch)
    ));
}

#[test]
fn f251_batched_horner_is_exact_and_allocation_free_after_reservation() {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = (0_usize..1024)
        .map(|index| builder.add_vertex((index % 11).to_le_bytes().to_vec()))
        .collect();
    for index in 0..vertices.len() {
        for step in [1_usize, 5, 17, 61] {
            builder
                .add_directed_relation(
                    vertices[index],
                    vertices[(index + step) % vertices.len()],
                    b"batch".to_vec(),
                    vec![u8::try_from(step).unwrap()],
                    u64::try_from(index % 4 + 1).unwrap(),
                )
                .unwrap();
        }
    }
    let graph = builder.build().unwrap();
    let rounds = 4;
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds },
    )
    .unwrap();
    let prepared = labeler.prepare(&graph).unwrap();
    let mut scalar_workspace = GraphWorkspace::new();
    let scalar = labeler
        .analyze_prepared_with_workspace(
            &prepared,
            &mut scalar_workspace,
            GraphExecution::Sequential,
        )
        .unwrap()
        .to_owned();
    let mut batch_workspace = F251BatchGraphWorkspace::detected(graph.vertex_count(), rounds);
    if CpuCapabilities::detect().has_x86_avx2() {
        assert_eq!(batch_workspace.backend_id(), BackendId::X86PrimeAvx2);
    }
    let batch = labeler
        .analyze_prepared_f251_batched(&prepared, &mut batch_workspace, GraphExecution::Sequential)
        .unwrap()
        .to_owned();
    assert_eq!(batch, scalar);

    let allocations = measure(|| {
        let view = labeler
            .analyze_prepared_f251_batched(
                &prepared,
                &mut batch_workspace,
                GraphExecution::Sequential,
            )
            .unwrap();
        std::hint::black_box(view.signature().lanes());
    });
    assert_eq!(allocations.count_total, 0, "{allocations:?}");
}

#[test]
fn optimized_f251_strategies_match_on_adversarial_random_multigraphs() {
    let mut rng = StdRng::seed_from_u64(0xa076_1d64_78bd_642f);
    for case in 0..24_usize {
        let vertex_count = 7 + case * 3;
        let mut builder = IncidenceGraphBuilder::new();
        let vertices: Vec<_> = (0..vertex_count)
            .map(|index| {
                let mut label = vec![0_u8; case % 19];
                rng.fill(label.as_mut_slice());
                label.extend_from_slice(&index.to_le_bytes());
                builder.add_vertex(label)
            })
            .collect();
        for source in 0..vertex_count {
            for edge in 0..6_usize {
                let target = rng.gen_range(0..vertex_count);
                let multiplicity = [1_u64, 2, 3, 4, 5, 257][edge];
                builder
                    .add_directed_relation(
                        vertices[source],
                        vertices[target],
                        vec![u8::try_from(edge).unwrap(), u8::try_from(case).unwrap()],
                        vec![rng.gen()],
                        multiplicity,
                    )
                    .unwrap();
            }
        }
        let graph = builder.build().unwrap();
        let labeler = F251GraphLabeler::<3>::f251(RefinementProfile::Fast { rounds: 5 }).unwrap();
        let prepared = labeler.prepare(&graph).unwrap();
        let mut scalar_workspace = GraphWorkspace::new();
        let scalar = labeler
            .analyze_prepared_with_workspace(
                &prepared,
                &mut scalar_workspace,
                GraphExecution::Sequential,
            )
            .unwrap()
            .to_owned();
        let mut parallel_workspace = GraphWorkspace::new();
        let parallel = rayon::ThreadPoolBuilder::new()
            .num_threads(2 + case % 3)
            .build()
            .unwrap()
            .install(|| {
                labeler
                    .analyze_prepared_with_workspace(
                        &prepared,
                        &mut parallel_workspace,
                        GraphExecution::Parallel {
                            minimum_vertices: 1,
                        },
                    )
                    .unwrap()
                    .to_owned()
            });
        let mut batch_workspace = F251BatchGraphWorkspace::detected(vertex_count, 5);
        let batch = labeler
            .analyze_prepared_f251_batched(
                &prepared,
                &mut batch_workspace,
                GraphExecution::Sequential,
            )
            .unwrap()
            .to_owned();
        assert_eq!(parallel, scalar, "parallel case {case}");
        assert_eq!(batch, scalar, "batch case {case}");
    }
}

#[test]
fn prepared_robust_and_hybrid_paths_match_owned_facades() {
    let graph = directed_fixture();
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::robust(),
    )
    .unwrap();
    let expected = labeler.analyze(&graph).unwrap();
    let expected_hybrid = labeler.analyze_hybrid(&graph).unwrap();
    let prepared = labeler.prepare(&graph).unwrap();
    let mut workspace = GraphWorkspace::new();
    let actual = labeler
        .analyze_prepared_with_workspace(&prepared, &mut workspace, GraphExecution::Sequential)
        .unwrap()
        .to_owned();
    assert_eq!(actual, expected);
    let actual_hybrid = labeler
        .analyze_prepared_hybrid_with_workspace(
            &prepared,
            &mut workspace,
            GraphExecution::Sequential,
        )
        .unwrap();
    assert_eq!(actual_hybrid, expected_hybrid);

    let mut batch_workspace = F251BatchGraphWorkspace::<3>::detected(graph.vertex_count(), 16);
    assert!(matches!(
        labeler.analyze_prepared_f251_batched(
            &prepared,
            &mut batch_workspace,
            GraphExecution::Sequential
        ),
        Err(GraphError::NonComposableProfile)
    ));
}

fn incremental_path_graph(labels: &[u64], bridges_components: bool) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = labels
        .iter()
        .map(|label| builder.add_vertex(label.to_le_bytes().to_vec()))
        .collect();
    let split = labels.len() / 2;
    for index in 0..labels.len().saturating_sub(1) {
        if index + 1 == split && !bridges_components {
            continue;
        }
        builder
            .add_directed_relation(
                vertices[index],
                vertices[index + 1],
                b"incremental-path".to_vec(),
                vec![u8::try_from(index % 3).unwrap()],
                u64::try_from(index % 5 + 1).unwrap(),
            )
            .unwrap();
    }
    builder.build().unwrap()
}

#[test]
fn incremental_label_edit_matches_full_analysis_and_stays_inside_round_radius() {
    let vertex_count = 257_usize;
    let labels: Vec<_> = (0..vertex_count).map(|index| index as u64).collect();
    let graph = incremental_path_graph(&labels, true);
    let labeler = F251GraphLabeler::<3>::f251(RefinementProfile::Fast { rounds: 4 }).unwrap();
    let mut state = labeler.incremental_state(graph.clone()).unwrap();
    assert_eq!(
        state.analysis().to_owned(),
        labeler.analyze(&graph).unwrap()
    );
    assert_eq!(state.component_count(), 1);
    assert!(state.dependency_record_count() <= graph.incidence_count() * 2);

    let no_op_revision = state.revision();
    let mut workspace = IncrementalGraphWorkspace::new();
    let mut identical = Some(graph);
    let mut no_op = None;
    let no_op_allocations = measure(|| {
        no_op = Some(
            labeler
                .update_incremental(&mut state, identical.take().unwrap(), &mut workspace)
                .unwrap(),
        );
    });
    let no_op = no_op.unwrap();
    assert_eq!(no_op_allocations.count_total, 0, "{no_op_allocations:?}");
    assert_eq!(no_op.recomputed_vertex_rounds(), 0);
    assert_eq!(no_op.revision(), no_op_revision);

    let mut edited_labels = labels.clone();
    edited_labels[vertex_count / 2] ^= 0xa5a5_5a5a;
    let edited = incremental_path_graph(&edited_labels, true);
    let expected = labeler.analyze(&edited).unwrap();
    let stats = labeler
        .update_incremental(&mut state, edited.clone(), &mut workspace)
        .unwrap();
    assert_eq!(state.analysis().to_owned(), expected);
    assert_eq!(stats.initial_seed_vertices(), 1);
    assert_eq!(stats.topology_seed_vertices(), 0);
    assert!(stats.peak_frontier_vertices() <= 9);
    assert!(stats.recomputed_vertex_rounds() < vertex_count * 4 / 16);
    assert_eq!(stats.revision(), no_op_revision + 1);

    let restored = incremental_path_graph(&labels, true);
    let expected = labeler.analyze(&restored).unwrap();
    let reverse = labeler
        .update_incremental(&mut state, restored, &mut workspace)
        .unwrap();
    assert_eq!(state.analysis().to_owned(), expected);
    assert_eq!(reverse.initial_seed_vertices(), 1);
    assert_eq!(state.revision(), no_op_revision + 2);
}

#[test]
fn incremental_topology_edits_merge_and_split_components_exactly() {
    let labels: Vec<_> = (0_u64..12).collect();
    let split = incremental_path_graph(&labels, false);
    let merged = incremental_path_graph(&labels, true);
    let labeler = F251GraphLabeler::<3>::f251(RefinementProfile::Fast { rounds: 5 }).unwrap();
    let mut state = labeler.incremental_state(split.clone()).unwrap();
    let mut workspace = IncrementalGraphWorkspace::new();
    assert_eq!(state.component_count(), 2);

    let merge = labeler
        .update_incremental(&mut state, merged.clone(), &mut workspace)
        .unwrap();
    assert_eq!(merge.previous_component_count(), 2);
    assert_eq!(merge.component_count(), 1);
    assert_eq!(merge.topology_seed_vertices(), 2);
    assert_eq!(
        state.analysis().to_owned(),
        labeler.analyze(&merged).unwrap()
    );
    assert_eq!(
        state.component_of(VertexId::new(0)),
        state.component_of(VertexId::new(11))
    );

    let separate = labeler
        .update_incremental(&mut state, split.clone(), &mut workspace)
        .unwrap();
    assert_eq!(separate.previous_component_count(), 1);
    assert_eq!(separate.component_count(), 2);
    assert_eq!(separate.topology_seed_vertices(), 2);
    assert_eq!(
        state.analysis().to_owned(),
        labeler.analyze(&split).unwrap()
    );
    assert_ne!(
        state.component_of(VertexId::new(0)),
        state.component_of(VertexId::new(11))
    );
}

#[test]
fn incremental_errors_leave_the_published_state_unchanged() {
    let labels: Vec<_> = (0_u64..10).collect();
    let graph = incremental_path_graph(&labels, true);
    let labeler = F251GraphLabeler::<3>::f251(RefinementProfile::Fast { rounds: 4 }).unwrap();
    let mut state = labeler.incremental_state(graph.clone()).unwrap();
    let before = state.clone();
    let mut workspace = IncrementalGraphWorkspace::new();

    let incompatible = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN + 99),
        RefinementProfile::Fast { rounds: 4 },
    )
    .unwrap();
    assert_eq!(
        incompatible.update_incremental(&mut state, graph.clone(), &mut workspace),
        Err(GraphError::SignatureIdentityMismatch)
    );
    assert_eq!(state.graph(), before.graph());
    assert_eq!(state.analysis().to_owned(), before.analysis().to_owned());
    assert_eq!(state.revision(), before.revision());

    let larger = incremental_path_graph(&(0_u64..11).collect::<Vec<_>>(), true);
    assert!(matches!(
        labeler.update_incremental(&mut state, larger, &mut workspace),
        Err(GraphError::IncrementalVertexCountMismatch {
            expected: 10,
            actual: 11
        })
    ));
    assert_eq!(state.graph(), before.graph());
    assert_eq!(state.analysis().to_owned(), before.analysis().to_owned());
    assert_eq!(state.revision(), before.revision());

    let robust = F251GraphLabeler::<3>::f251(RefinementProfile::robust()).unwrap();
    assert!(matches!(
        robust.incremental_state(graph),
        Err(GraphError::NonComposableProfile)
    ));
}

#[test]
fn incremental_random_edit_sequence_is_differentially_exact_across_fields() {
    fn exercise<F, E>(labeler: &FastGraphLabeler<F, E, 2>)
    where
        F: microfield::Field
            + microfield::CanonicalEncoding
            + microfield::StaticField
            + microfield::Pow
            + microfield::Invert
            + core::fmt::Debug,
        E: homomorphic_hash_rs::StructuralEncoder<F>,
    {
        let mut rng = StdRng::seed_from_u64(0xe703_7ed1_a0b4_28db);
        let mut labels: Vec<_> = (0_u64..31).collect();
        let initial = incremental_path_graph(&labels, false);
        let mut state = labeler.incremental_state(initial).unwrap();
        let mut workspace = IncrementalGraphWorkspace::new();
        for revision in 1..=48_u64 {
            let vertex = rng.gen_range(0..labels.len());
            labels[vertex] ^= rng.gen::<u64>() | 1;
            let bridge = revision % 3 != 0;
            let next = incremental_path_graph(&labels, bridge);
            let expected = labeler.analyze(&next).unwrap();
            let stats = labeler
                .update_incremental(&mut state, next, &mut workspace)
                .unwrap();
            assert_eq!(state.analysis().to_owned(), expected, "revision {revision}");
            assert_eq!(stats.revision(), revision);
            assert!(stats.dependency_records() <= state.graph().incidence_count() * 2);
        }
    }

    let f251 = FastGraphLabeler::<Fp251V1, _, 2>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 5 },
    )
    .unwrap();
    exercise(&f251);
    let goldilocks = FastGraphLabeler::<FpGoldilocks64V1, _, 2>::new(
        PrimeIntegerEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 4 },
    )
    .unwrap();
    exercise(&goldilocks);
    let binary = FastGraphLabeler::<Gf2_256HhV1, _, 2>::new(
        BinaryPolynomialEncoder::new(GRAPH_DOMAIN),
        RefinementProfile::Fast { rounds: 3 },
    )
    .unwrap();
    exercise(&binary);
}

fn incremental_adversarial_topology(case: usize) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let mut vertices: Vec<_> = (0_u64..7)
        .map(|label| builder.add_vertex(label.to_le_bytes().to_vec()))
        .collect();
    vertices.push(builder.add_typed_vertex(VertexKind::Hyperedge, b"h".to_vec()));
    for index in 0..6 {
        builder
            .add_directed_relation(
                vertices[index],
                vertices[index + 1],
                b"base".to_vec(),
                vec![u8::try_from(index % 2).unwrap()],
                1,
            )
            .unwrap();
    }
    let relation: &[u8] = if case == 1 { b"changed" } else { b"extra" };
    let role: &[u8] = if case == 2 { b"changed-role" } else { b"port" };
    let multiplicity = if case == 3 { 257 } else { 3 };
    let (source, target) = if case == 4 {
        (vertices[5], vertices[1])
    } else {
        (vertices[1], vertices[5])
    };
    builder
        .add_directed_relation(
            source,
            target,
            relation.to_vec(),
            role.to_vec(),
            multiplicity,
        )
        .unwrap();
    if case == 5 {
        builder
            .add_directed_relation(vertices[3], vertices[3], b"loop".to_vec(), Vec::new(), 11)
            .unwrap();
    }
    if case == 6 {
        for multiplicity in [2, 5, 7] {
            builder
                .add_directed_relation(
                    vertices[2],
                    vertices[7],
                    b"microfield/hyperedge-incidence-v1".to_vec(),
                    b"member".to_vec(),
                    multiplicity,
                )
                .unwrap();
        }
    }
    if case == 7 {
        builder
            .add_undirected_relation(
                vertices[0],
                vertices[7],
                b"microfield/hyperedge-incidence-v1".to_vec(),
                b"member".to_vec(),
                13,
            )
            .unwrap();
    }
    builder.build().unwrap()
}

#[test]
fn incremental_semantic_row_audit_covers_roles_direction_loops_and_multiplicity() {
    let labeler = F251GraphLabeler::<3>::f251(RefinementProfile::Fast { rounds: 6 }).unwrap();
    let initial = incremental_adversarial_topology(0);
    let mut state = labeler.incremental_state(initial).unwrap();
    let mut workspace = IncrementalGraphWorkspace::new();
    for case in 1..=7 {
        let next = incremental_adversarial_topology(case);
        let expected = labeler.analyze(&next).unwrap();
        let stats = labeler
            .update_incremental(&mut state, next, &mut workspace)
            .unwrap();
        assert_eq!(
            state.analysis().to_owned(),
            expected,
            "topology case {case}"
        );
        assert!(stats.topology_seed_vertices() > 0, "topology case {case}");
    }
}

#[test]
fn incremental_aggregate_delta_handles_zero_factor_removal_and_insertion() {
    use homomorphic_hash_rs::StructuralEncoder as _;
    use microfield::Field as _;

    let encoder = PrimeIntegerEncoder::new(GRAPH_DOMAIN);
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 1>::new(encoder, RefinementProfile::Fast { rounds: 3 })
            .unwrap();
    let offset = labeler.parameters().graph_offsets()[0];
    let salt = labeler.parameters().lane_salts()[0];
    let mut zero_label = None;
    let mut nonzero_label = None;
    for candidate in 0_u64..4096 {
        let label = candidate.to_le_bytes().to_vec();
        let mut framed = vec![1, VertexKind::Entity as u8];
        framed.extend_from_slice(&(label.len() as u64).to_le_bytes());
        framed.extend_from_slice(&label);
        framed.extend_from_slice(&0_u64.to_le_bytes());
        framed.extend_from_slice(&0_u64.to_le_bytes());
        let exact: Fp251V1 = encoder.encode(&framed).unwrap();
        let factor = exact.add(salt).add(offset);
        if factor.is_zero() {
            zero_label = Some(candidate);
        } else {
            nonzero_label = Some(candidate);
        }
        if zero_label.is_some() && nonzero_label.is_some() {
            break;
        }
    }
    let zero_label = zero_label.expect("F251 search must reach every aggregate residue");
    let nonzero_label = nonzero_label.unwrap();
    let old = incremental_path_graph(&[zero_label], true);
    let new = incremental_path_graph(&[nonzero_label], true);
    let mut state = labeler.incremental_state(old.clone()).unwrap();
    let mut workspace = IncrementalGraphWorkspace::new();
    labeler
        .update_incremental(&mut state, new.clone(), &mut workspace)
        .unwrap();
    assert_eq!(state.analysis().to_owned(), labeler.analyze(&new).unwrap());
    labeler
        .update_incremental(&mut state, old.clone(), &mut workspace)
        .unwrap();
    assert_eq!(state.analysis().to_owned(), labeler.analyze(&old).unwrap());
}
