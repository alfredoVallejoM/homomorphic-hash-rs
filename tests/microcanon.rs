//! Contract and adversarial tests for the profile-independent exact core.

use homomorphic_hash_rs::{
    BinaryPolynomialEncoder, CanonicalGraphDocument, CanonicalSearchBudget, CanonicalizationPath,
    ExactCanonicalOutcome, FastGraphLabeler, GraphComparison, GraphError, GraphSchemaId,
    HyperedgeIncidence, IncidenceGraph, IncidenceGraphBuilder, Microcanon, MicrocanonOutcome,
    MicrocanonStrategy, MicrocanonWorkspace, PrimeIntegerEncoder, RefinementProfile,
    TryCanonicalOutcome, VerifiedGraphMapping, VertexId,
};
use microfield::{Fp251V1, Gf2_256HhV1};

const DOMAIN: u64 = 0x4d49_4352_4f43_414e;

fn exact_form(outcome: MicrocanonOutcome) -> homomorphic_hash_rs::CanonicalGraphForm {
    match outcome {
        MicrocanonOutcome::Exact { form, .. } => form,
        MicrocanonOutcome::Inconclusive { report } => {
            panic!("unexpected incomplete canonization: {report:?}")
        }
    }
}

fn relational_fixture() -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let carbon = builder.add_vertex(b"C".to_vec());
    let oxygen = builder.add_vertex(b"O".to_vec());
    let hydrogen = builder.add_vertex(b"H".to_vec());
    builder
        .add_directed_relation(carbon, oxygen, b"bond".to_vec(), b"donor".to_vec(), 2)
        .unwrap();
    builder
        .add_directed_relation(oxygen, carbon, b"bond".to_vec(), b"acceptor".to_vec(), 2)
        .unwrap();
    builder
        .add_hyperedge(
            b"constraint".to_vec(),
            &[
                HyperedgeIncidence::new(carbon, b"center".to_vec()),
                HyperedgeIncidence::new(oxygen, b"member".to_vec()),
                HyperedgeIncidence::new(hydrogen, b"member".to_vec()).with_multiplicity(3),
            ],
        )
        .unwrap();
    builder.build().unwrap()
}

fn relabel(graph: &IncidenceGraph, new_to_old: &[usize]) -> IncidenceGraph {
    assert_eq!(new_to_old.len(), graph.vertex_count());
    let mut old_to_new = vec![usize::MAX; graph.vertex_count()];
    let mut builder = IncidenceGraphBuilder::new();
    for (new, &old) in new_to_old.iter().enumerate() {
        old_to_new[old] = new;
        let old_vertex = VertexId::new(old);
        builder.add_typed_vertex(
            graph.vertex_kind(old_vertex),
            graph.vertex_label(old_vertex).to_vec(),
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

fn cycles(lengths: &[usize]) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..lengths.iter().sum())
        .map(|_| builder.add_vertex(Vec::new()))
        .collect::<Vec<_>>();
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

#[test]
fn canonical_bytes_are_independent_of_field_encoder_lanes_and_profile() {
    let graph = relational_fixture();
    let budget = CanonicalSearchBudget::new(100_000);
    let prime = FastGraphLabeler::<Fp251V1, _, 1>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::Fast { rounds: 3 },
    )
    .unwrap();
    let binary = FastGraphLabeler::<Gf2_256HhV1, _, 4>::new(
        BinaryPolynomialEncoder::new(DOMAIN ^ 0x55aa),
        RefinementProfile::Robust {
            minimum_rounds: 2,
            maximum_rounds: 9,
        },
    )
    .unwrap();

    let left = match prime.canonicalize_exact(&graph, budget).unwrap() {
        ExactCanonicalOutcome::Exact { form, report } => {
            assert!(matches!(
                report.path(),
                CanonicalizationPath::FastDiscrete | CanonicalizationPath::ExactRefinementDiscrete
            ));
            form
        }
        ExactCanonicalOutcome::BudgetExhausted { report } => panic!("{report:?}"),
    };
    let right = match binary.canonicalize_exact(&graph, budget).unwrap() {
        ExactCanonicalOutcome::Exact { form, .. } => form,
        ExactCanonicalOutcome::BudgetExhausted { report } => panic!("{report:?}"),
    };
    let independent = exact_form(Microcanon::default().canonicalize(&graph, budget).unwrap());

    assert_eq!(left.bytes(), right.bytes());
    assert_eq!(left.bytes(), independent.bytes());
    assert_eq!(left.key(), right.key());
    assert_eq!(left.schema_id(), GraphSchemaId::default());

    let mut singleton = IncidenceGraphBuilder::new();
    singleton.add_vertex(b"profile-independent".to_vec());
    let singleton = singleton.build().unwrap();
    let singleton_exact = exact_form(
        Microcanon::default()
            .canonicalize(&singleton, CanonicalSearchBudget::new(0))
            .unwrap(),
    );
    let prime_fast = match prime.try_canonicalize(&singleton).unwrap() {
        TryCanonicalOutcome::Canonical(form) => form,
        TryCanonicalOutcome::SymmetryRemaining(_) => panic!("singleton must be discrete"),
    };
    let binary_fast = match binary.try_canonicalize(&singleton).unwrap() {
        TryCanonicalOutcome::Canonical(form) => form,
        TryCanonicalOutcome::SymmetryRemaining(_) => panic!("singleton must be discrete"),
    };
    assert_eq!(prime_fast.bytes(), singleton_exact.bytes());
    assert_eq!(binary_fast.bytes(), singleton_exact.bytes());
}

#[test]
fn schema_identity_changes_the_exact_envelope_but_not_graph_semantics() {
    let graph = relational_fixture();
    let budget = CanonicalSearchBudget::new(100_000);
    let chemistry = Microcanon::new(GraphSchemaId::derive(b"chemistry/v1"));
    let network = Microcanon::new(GraphSchemaId::derive(b"network/v1"));
    let left = exact_form(chemistry.canonicalize(&graph, budget).unwrap());
    let right = exact_form(network.canonicalize(&graph, budget).unwrap());

    assert_ne!(left.schema_id(), right.schema_id());
    assert_ne!(left.bytes(), right.bytes());
    assert_ne!(left.key(), right.key());
    assert_eq!(
        left.decode().unwrap().graph(),
        right.decode().unwrap().graph()
    );
}

#[test]
fn canonical_document_round_trips_and_rejects_malformed_envelopes() {
    let graph = relational_fixture();
    let budget = CanonicalSearchBudget::new(100_000);
    let form = exact_form(Microcanon::default().canonicalize(&graph, budget).unwrap());
    let document = CanonicalGraphDocument::from_bytes(form.bytes()).unwrap();
    let rebuilt = exact_form(
        Microcanon::new(document.schema_id())
            .canonicalize(document.graph(), budget)
            .unwrap(),
    );
    assert_eq!(form.bytes(), rebuilt.bytes());
    assert_eq!(document.encoding_id(), form.encoding_id());

    for length in 0..form.bytes().len() {
        assert!(CanonicalGraphDocument::from_bytes(&form.bytes()[..length]).is_err());
    }
    let mut trailing = form.bytes().to_vec();
    trailing.push(0);
    assert_eq!(
        CanonicalGraphDocument::from_bytes(&trailing),
        Err(GraphError::InvalidCanonicalEncoding)
    );
    let mut unsupported = form.bytes().to_vec();
    unsupported[4..6].copy_from_slice(&u16::MAX.to_be_bytes());
    assert_eq!(
        CanonicalGraphDocument::from_bytes(&unsupported),
        Err(GraphError::UnsupportedCanonicalEncoding { version: u16::MAX })
    );

    let mut malformed = Vec::new();
    let mut bad_magic = form.bytes().to_vec();
    bad_magic[0] ^= 0xff;
    malformed.push(bad_magic);
    let mut bad_model = form.bytes().to_vec();
    bad_model[6..8].copy_from_slice(&2_u16.to_be_bytes());
    malformed.push(bad_model);
    let mut impossible_vertices = form.bytes().to_vec();
    impossible_vertices[40..48].copy_from_slice(&u64::MAX.to_be_bytes());
    malformed.push(impossible_vertices);
    let mut impossible_arcs = form.bytes().to_vec();
    impossible_arcs[48..56].copy_from_slice(&u64::MAX.to_be_bytes());
    malformed.push(impossible_arcs);
    let mut wrong_multiplicity = form.bytes().to_vec();
    let total = u64::from_be_bytes(form.bytes()[56..64].try_into().unwrap());
    wrong_multiplicity[56..64].copy_from_slice(&(total + 1).to_be_bytes());
    malformed.push(wrong_multiplicity);
    for bytes in malformed {
        assert_eq!(
            CanonicalGraphDocument::from_bytes(&bytes),
            Err(GraphError::InvalidCanonicalEncoding)
        );
    }
}

#[test]
fn comparison_returns_only_verified_mappings_and_exact_difference_witnesses() {
    let graph = relational_fixture();
    let permuted = relabel(&graph, &[3, 1, 0, 2]);
    let canon = Microcanon::default();
    let budget = CanonicalSearchBudget::new(100_000);
    match canon.compare(&graph, &permuted, budget).unwrap() {
        GraphComparison::Isomorphic { mapping, report } => {
            assert_eq!(mapping.left_to_right().len(), graph.vertex_count());
            assert_eq!(mapping.right_to_left().len(), graph.vertex_count());
            VerifiedGraphMapping::verify(&graph, &permuted, mapping.left_to_right()).unwrap();
            assert!(report.left().is_some());
            assert!(report.right().is_some());
        }
        outcome => panic!("expected verified isomorphism, found {outcome:?}"),
    }

    let cycle = cycles(&[6]);
    let triangles = cycles(&[3, 3]);
    assert!(matches!(
        canon.compare(&cycle, &triangles, budget).unwrap(),
        GraphComparison::Different { .. }
    ));
    assert!(matches!(
        canon
            .compare(&cycle, &cycle, CanonicalSearchBudget::new(0))
            .unwrap(),
        GraphComparison::Inconclusive { .. }
    ));
}

#[test]
fn mapping_verifier_rejects_non_bijections_and_semantic_mismatches() {
    let graph = relational_fixture();
    let identity = (0..graph.vertex_count())
        .map(VertexId::new)
        .collect::<Vec<_>>();
    VerifiedGraphMapping::verify(&graph, &graph, &identity).unwrap();

    let mut duplicate = identity.clone();
    duplicate[1] = duplicate[0];
    assert_eq!(
        VerifiedGraphMapping::verify(&graph, &graph, &duplicate),
        Err(GraphError::InvalidGraphMapping)
    );
    let wrong = relabel(&graph, &[1, 0, 2, 3]);
    assert_eq!(
        VerifiedGraphMapping::verify(&graph, &wrong, &identity),
        Err(GraphError::InvalidGraphMapping)
    );
}

#[test]
fn checked_access_and_hyperedge_construction_fail_without_partial_publication() {
    let mut builder = IncidenceGraphBuilder::new();
    let entity = builder.add_vertex(b"entity".to_vec());
    let hyperedge = builder
        .add_hyperedge(
            b"first".to_vec(),
            &[HyperedgeIncidence::new(entity, Vec::new())],
        )
        .unwrap();
    assert_eq!(
        builder.add_hyperedge(
            b"invalid".to_vec(),
            &[HyperedgeIncidence::new(hyperedge, Vec::new())],
        ),
        Err(GraphError::InvalidHyperedgeEndpoint {
            index: hyperedge.index()
        })
    );
    let graph = builder.build().unwrap();
    assert_eq!(graph.vertex_count(), 2);
    assert!(graph.contains_vertex(entity));
    assert!(!graph.contains_vertex(VertexId::new(2)));
    assert!(matches!(
        graph.try_vertex_label(VertexId::new(2)),
        Err(GraphError::InvalidVertex {
            index: 2,
            vertex_count: 2
        })
    ));
    let relation = graph.outgoing(entity)[0].relation();
    assert!(graph.try_relation(relation).is_ok());
    let empty = IncidenceGraphBuilder::new().build().unwrap();
    assert_eq!(
        empty.try_relation(relation),
        Err(GraphError::InvalidRelation {
            index: relation.index(),
            relation_count: 0
        })
    );
}

#[test]
fn compact_engine_matches_reference_and_reports_certified_pruning() {
    let graph = cycles(&[32]);
    let budget = CanonicalSearchBudget::new(10_000_000);
    let (reference, reference_nodes) = match Microcanon::default()
        .with_strategy(MicrocanonStrategy::Reference)
        .canonicalize(&graph, budget)
        .unwrap()
    {
        MicrocanonOutcome::Exact { form, report } => (form, report.explored_nodes()),
        MicrocanonOutcome::Inconclusive { report } => panic!("{report:?}"),
    };
    let optimized = Microcanon::default().canonicalize(&graph, budget).unwrap();
    let (optimized, report) = match optimized {
        MicrocanonOutcome::Exact { form, report } => (form, report),
        MicrocanonOutcome::Inconclusive { report } => panic!("{report:?}"),
    };
    assert_eq!(optimized.bytes(), reference.bytes());
    assert!(report.trace_event_count() > 0);
    assert!(report.target_cell_count() > 0);
    assert!(report.verified_automorphism_count() > 0);
    assert!(report.orbit_pruned_child_count() > 0);
    assert!(report.peak_tracked_bytes() > 0);
    assert!(
        report.explored_nodes().saturating_mul(10) <= reference_nodes,
        "G10 must remove at least 90% of G9 nodes on the symmetric acceptance graph"
    );
}

#[test]
fn compact_integer_tuples_match_framed_g9_keys_on_relational_graphs() {
    let labels: [&[u8]; 4] = [b"", b"a", b"cc", b"long-label"];
    let relations: [&[u8]; 4] = [b"", b"r", b"zz", b"relation"];
    let roles: [&[u8]; 3] = [b"", b"p", b"long-role"];
    let budget = CanonicalSearchBudget::new(1_000_000);
    let compact = Microcanon::default();
    let reference = compact.with_strategy(MicrocanonStrategy::Reference);

    for seed in 0_u64..128 {
        let mut state = seed ^ 0xd1b5_4a32_d192_ed03;
        let mut builder = IncidenceGraphBuilder::new();
        let vertices = (0..5)
            .map(|vertex| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                builder.add_vertex(labels[(state as usize + vertex) % labels.len()].to_vec())
            })
            .collect::<Vec<_>>();
        for source in 0..vertices.len() {
            for target in 0..vertices.len() {
                state = state
                    .wrapping_mul(2_862_933_555_777_941_757)
                    .wrapping_add(3_037_000_493);
                if state & 3 == 0 {
                    continue;
                }
                builder
                    .add_directed_relation(
                        vertices[source],
                        vertices[target],
                        relations[(state >> 8) as usize % relations.len()].to_vec(),
                        roles[(state >> 16) as usize % roles.len()].to_vec(),
                        1 + ((state >> 24) % 3),
                    )
                    .unwrap();
            }
        }
        let graph = builder.build().unwrap();
        let permutation = [4, 2, 0, 3, 1];
        let permuted = relabel(&graph, &permutation);
        let compact_form = exact_form(compact.canonicalize(&graph, budget).unwrap());
        let reference_form = exact_form(reference.canonicalize(&graph, budget).unwrap());
        let permuted_form = exact_form(compact.canonicalize(&permuted, budget).unwrap());
        assert_eq!(
            compact_form.bytes(),
            reference_form.bytes(),
            "compact tuple ordering diverged from framed G9 keys at seed {seed}"
        );
        assert_eq!(
            compact_form.bytes(),
            permuted_form.bytes(),
            "compact relational form changed under renumbering at seed {seed}"
        );
    }
}

#[test]
fn g10_physical_depth_and_time_budgets_fail_closed() {
    use std::time::Duration;

    let graph = cycles(&[10]);
    let canon = Microcanon::default();
    for (budget, expected) in [
        (
            CanonicalSearchBudget::new(1_000_000).with_max_retained_bytes(1),
            homomorphic_hash_rs::CanonicalBudgetLimit::RetainedBytes,
        ),
        (
            CanonicalSearchBudget::new(1_000_000).with_max_depth(0),
            homomorphic_hash_rs::CanonicalBudgetLimit::SearchDepth,
        ),
        (
            CanonicalSearchBudget::new(1_000_000).with_max_elapsed(Duration::ZERO),
            homomorphic_hash_rs::CanonicalBudgetLimit::ElapsedTime,
        ),
    ] {
        match canon.canonicalize(&graph, budget).unwrap() {
            MicrocanonOutcome::Exact { .. } => panic!("budget {expected:?} published a form"),
            MicrocanonOutcome::Inconclusive { report } => {
                assert_eq!(report.exhausted_limit(), Some(expected));
            }
        }
    }
}

#[test]
fn component_artifacts_participate_in_the_retained_byte_budget() {
    let mut builder = IncidenceGraphBuilder::new();
    for _ in 0..32 {
        builder.add_vertex(Vec::new());
    }
    let graph = builder.build().unwrap();
    let canon = Microcanon::default();
    let complete = canon
        .canonicalize(&graph, CanonicalSearchBudget::new(1_000_000))
        .unwrap();
    let peak = match complete {
        MicrocanonOutcome::Exact { report, .. } => report.peak_tracked_bytes(),
        MicrocanonOutcome::Inconclusive { report } => panic!("{report:?}"),
    };
    assert!(peak > 1);
    match canon
        .canonicalize(
            &graph,
            CanonicalSearchBudget::new(1_000_000).with_max_retained_bytes(peak - 1),
        )
        .unwrap()
    {
        MicrocanonOutcome::Exact { .. } => {
            panic!("component artifact peak escaped the retained-byte budget")
        }
        MicrocanonOutcome::Inconclusive { report } => assert_eq!(
            report.exhausted_limit(),
            Some(homomorphic_hash_rs::CanonicalBudgetLimit::RetainedBytes)
        ),
    }
}

#[test]
fn differential_reference_never_publishes_past_new_g10_limits() {
    let graph = cycles(&[8]);
    let reference = Microcanon::default().with_strategy(MicrocanonStrategy::Reference);
    for (budget, expected) in [
        (
            CanonicalSearchBudget::new(1_000_000).with_max_depth(0),
            homomorphic_hash_rs::CanonicalBudgetLimit::SearchDepth,
        ),
        (
            CanonicalSearchBudget::new(1_000_000).with_max_retained_bytes(1),
            homomorphic_hash_rs::CanonicalBudgetLimit::RetainedBytes,
        ),
    ] {
        match reference.canonicalize(&graph, budget).unwrap() {
            MicrocanonOutcome::Exact { .. } => {
                panic!("G9 differential strategy published past {expected:?}")
            }
            MicrocanonOutcome::Inconclusive { report } => {
                assert_eq!(report.exhausted_limit(), Some(expected));
            }
        }
    }
}

#[test]
fn compact_workspace_reuses_incidence_scaled_storage_and_rejects_reference_mode() {
    let graph = relational_fixture();
    let permuted = relabel(&graph, &[3, 1, 0, 2]);
    let budget = CanonicalSearchBudget::new(100_000);
    let mut workspace = MicrocanonWorkspace::new();
    workspace
        .reserve_for(graph.vertex_count(), graph.incidence_count())
        .unwrap();
    let reserved = workspace.retained_bytes().unwrap();
    let left = exact_form(
        Microcanon::default()
            .canonicalize_with_workspace(&graph, budget, &mut workspace)
            .unwrap(),
    );
    let after_left = workspace.retained_bytes().unwrap();
    let right = exact_form(
        Microcanon::default()
            .canonicalize_with_workspace(&permuted, budget, &mut workspace)
            .unwrap(),
    );
    assert_eq!(left.bytes(), right.bytes());
    assert_eq!(reserved, after_left);
    assert_eq!(after_left, workspace.retained_bytes().unwrap());
    assert_eq!(
        Microcanon::default()
            .with_strategy(MicrocanonStrategy::Reference)
            .canonicalize_with_workspace(&graph, budget, &mut workspace),
        Err(GraphError::IncompatibleCanonicalWorkspace)
    );
}
