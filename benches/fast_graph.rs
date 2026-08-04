//! Throughput comparison for bounded graph refinement over compact and wide fields.

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use homomorphic_hash_rs::{
    AdaptiveFilterPolicy, AdaptiveGraphPipeline, BinaryPolynomialEncoder, CanonicalSearchBudget,
    CellMomentProfile, ClosedWalkQueryPlan, DomainSeparatedHashToFieldEncoder,
    ExactCanonicalOutcome, F251BatchGraphWorkspace, F251GraphLabeler, FastGraphLabeler,
    GlobalGraphProfile, GraphDelta, GraphDeltaPolicy, GraphDiscriminationPolicy, GraphExecution,
    GraphWorkspace, IncidenceGraph, IncidenceGraphBuilder, IncrementalGraphWorkspace,
    LoopPatternCatalog, Microcanon, MicrocanonStrategy, PatternFieldFingerprint,
    PatternProductFingerprint, PrimeIntegerEncoder, RefinementProfile, RelationalClosedWalkProfile,
    RelationalMatrixProfile, RelationalThetaProfile, VertexId,
};
use microfield::{Fp251V1, FpGoldilocks64V1, Gf2_256HhV1};

const GRAPH_DOMAIN: u64 = 0x4752_4150_485f_0001;

fn regular_graph(vertices: usize) -> IncidenceGraph {
    regular_graph_variant(vertices, false, false)
}

fn homogeneous_cycle(vertices: usize) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let ids: Vec<_> = (0..vertices)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect();
    for index in 0..vertices {
        builder
            .add_undirected_relation(
                ids[index],
                ids[(index + 1) % vertices],
                b"edge".to_vec(),
                Vec::new(),
                1,
            )
            .unwrap();
    }
    builder.build().unwrap()
}

fn uniquely_labeled_cycle(vertices: usize) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|index| builder.add_vertex(index.to_be_bytes().to_vec()))
        .collect::<Vec<_>>();
    for index in 0..vertices {
        builder
            .add_undirected_relation(
                ids[index],
                ids[(index + 1) % vertices],
                b"edge".to_vec(),
                Vec::new(),
                1,
            )
            .unwrap();
    }
    builder.build().unwrap()
}

fn homogeneous_path(vertices: usize) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect::<Vec<_>>();
    for edge in ids.windows(2) {
        builder
            .add_undirected_relation(edge[0], edge[1], b"edge".to_vec(), Vec::new(), 1)
            .unwrap();
    }
    builder.build().unwrap()
}

fn homogeneous_star(vertices: usize) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect::<Vec<_>>();
    for leaf in 1..vertices {
        builder
            .add_undirected_relation(ids[0], ids[leaf], b"edge".to_vec(), Vec::new(), 1)
            .unwrap();
    }
    builder.build().unwrap()
}

fn path_with_first_label(vertices: usize, label: &[u8]) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|index| {
            builder.add_vertex(if index == 0 {
                label.to_vec()
            } else {
                Vec::new()
            })
        })
        .collect::<Vec<_>>();
    for edge in ids.windows(2) {
        builder
            .add_undirected_relation(edge[0], edge[1], b"edge".to_vec(), Vec::new(), 1)
            .unwrap();
    }
    builder.build().unwrap()
}

fn reverse_relabel(graph: &IncidenceGraph) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let mut old_to_new = vec![usize::MAX; graph.vertex_count()];
    for old in (0..graph.vertex_count()).rev() {
        let vertex = VertexId::new(old);
        let new = builder.add_typed_vertex(
            graph.vertex_kind(vertex),
            graph.vertex_label(vertex).to_vec(),
        );
        old_to_new[old] = new.index();
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

fn regular_graph_variant(
    vertices: usize,
    change_middle_label: bool,
    add_delta_edge: bool,
) -> IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let ids: Vec<_> = (0..vertices)
        .map(|index| {
            let mut label = vec![u8::try_from(index % 17).unwrap()];
            if change_middle_label && index == vertices / 2 {
                label.extend_from_slice(b"incremental-edit");
            }
            builder.add_vertex(label)
        })
        .collect();
    for source in 0..vertices {
        for step in [1_usize, 5, 13, 29] {
            builder
                .add_directed_relation(
                    ids[source],
                    ids[(source + step) % vertices],
                    b"edge".to_vec(),
                    Vec::new(),
                    1,
                )
                .unwrap();
        }
    }
    if add_delta_edge && vertices > 1 {
        builder
            .add_directed_relation(
                ids[vertices / 3],
                ids[(vertices * 2 / 3).min(vertices - 1)],
                b"incremental-delta".to_vec(),
                b"bridge".to_vec(),
                1,
            )
            .unwrap();
    }
    builder.build().unwrap()
}

fn benchmark_fast_graph(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("fast_graph_four_rounds");
    group.sample_size(10);
    for vertices in [1024_usize, 16_384, 131_072] {
        let graph = regular_graph(vertices);
        // Every directed record is visited through outgoing and incoming CSR
        // in each of the four rounds.
        let processed = u64::try_from(graph.incidence_count() * 2 * 4).unwrap();
        group.throughput(Throughput::Elements(processed));

        let f251 = F251GraphLabeler::<3>::f251(RefinementProfile::Fast { rounds: 4 }).unwrap();
        let prepared = f251.prepare(&graph).unwrap();
        let mut scalar_workspace = GraphWorkspace::new();
        scalar_workspace.reserve_for(vertices, 4);
        group.bench_with_input(
            BenchmarkId::new("f251_k3_end_to_end_owned", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| f251.analyze(std::hint::black_box(graph)).unwrap());
            },
        );
        group.bench_function(
            BenchmarkId::new("f251_k3_prepared_reuse_scalar", vertices),
            |bench| {
                bench.iter(|| {
                    let view = f251
                        .analyze_prepared_with_workspace(
                            std::hint::black_box(&prepared),
                            &mut scalar_workspace,
                            GraphExecution::Sequential,
                        )
                        .unwrap();
                    std::hint::black_box(view.signature().lanes());
                });
            },
        );
        let mut parallel_workspace = GraphWorkspace::new();
        parallel_workspace.reserve_for(vertices, 4);
        group.bench_function(
            BenchmarkId::new("f251_k3_prepared_reuse_parallel", vertices),
            |bench| {
                bench.iter(|| {
                    let view = f251
                        .analyze_prepared_with_workspace(
                            std::hint::black_box(&prepared),
                            &mut parallel_workspace,
                            GraphExecution::Parallel {
                                minimum_vertices: 1,
                            },
                        )
                        .unwrap();
                    std::hint::black_box(view.signature().lanes());
                });
            },
        );
        let mut batch_workspace = F251BatchGraphWorkspace::detected(vertices, 4);
        let backend = format!("{:?}", batch_workspace.backend_id()).to_lowercase();
        group.bench_function(
            BenchmarkId::new(format!("f251_k3_soa_batch_{backend}"), vertices),
            |bench| {
                bench.iter(|| {
                    let view = f251
                        .analyze_prepared_f251_batched(
                            std::hint::black_box(&prepared),
                            &mut batch_workspace,
                            GraphExecution::Sequential,
                        )
                        .unwrap();
                    std::hint::black_box(view.signature().lanes());
                });
            },
        );
        let mut parallel_batch_workspace = F251BatchGraphWorkspace::detected(vertices, 4);
        group.bench_function(
            BenchmarkId::new(format!("f251_k3_soa_batch_parallel_{backend}"), vertices),
            |bench| {
                bench.iter(|| {
                    let view = f251
                        .analyze_prepared_f251_batched(
                            std::hint::black_box(&prepared),
                            &mut parallel_batch_workspace,
                            GraphExecution::Parallel {
                                minimum_vertices: 1,
                            },
                        )
                        .unwrap();
                    std::hint::black_box(view.signature().lanes());
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("f251_k3_hybrid_sha256", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| f251.analyze_hybrid(std::hint::black_box(graph)).unwrap());
            },
        );

        if vertices <= 16_384 {
            let binary = FastGraphLabeler::<Gf2_256HhV1, _, 3>::new(
                BinaryPolynomialEncoder::new(GRAPH_DOMAIN),
                RefinementProfile::Fast { rounds: 4 },
            )
            .unwrap();
            let binary_prepared = binary.prepare(&graph).unwrap();
            let mut binary_workspace = GraphWorkspace::new();
            binary_workspace.reserve_for(vertices, 4);
            group.bench_function(
                BenchmarkId::new("gf2_256_k3_prepared_reuse", vertices),
                |bench| {
                    bench.iter(|| {
                        let view = binary
                            .analyze_prepared_with_workspace(
                                std::hint::black_box(&binary_prepared),
                                &mut binary_workspace,
                                GraphExecution::Sequential,
                            )
                            .unwrap();
                        std::hint::black_box(view.signature().lanes());
                    });
                },
            );
        }
    }
    group.finish();
}

fn benchmark_incremental_graph(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("fast_graph_incremental_four_rounds");
    group.sample_size(10);
    for vertices in [1024_usize, 16_384, 131_072] {
        let base = regular_graph_variant(vertices, false, false);
        let label_edit = regular_graph_variant(vertices, true, false);
        let topology_edit = regular_graph_variant(vertices, false, true);
        let audited = u64::try_from(vertices + base.incidence_count() * 2).unwrap();
        group.throughput(Throughput::Elements(audited));
        let labeler = F251GraphLabeler::<3>::f251(RefinementProfile::Fast { rounds: 4 }).unwrap();

        let mut full_toggle = false;
        group.bench_function(
            BenchmarkId::new("full_reanalysis_after_single_label_edit", vertices),
            |bench| {
                bench.iter_batched(
                    || {
                        full_toggle = !full_toggle;
                        if full_toggle {
                            label_edit.clone()
                        } else {
                            base.clone()
                        }
                    },
                    |graph| std::hint::black_box(labeler.analyze(&graph).unwrap()),
                    BatchSize::SmallInput,
                );
            },
        );

        let mut label_state = labeler.incremental_state(base.clone()).unwrap();
        let mut label_workspace = IncrementalGraphWorkspace::new();
        label_workspace
            .reserve_for(vertices, base.incidence_count(), 4)
            .unwrap();
        let mut label_toggle = false;
        group.bench_function(
            BenchmarkId::new("incremental_single_label_edit", vertices),
            |bench| {
                bench.iter_batched(
                    || {
                        label_toggle = !label_toggle;
                        if label_toggle {
                            label_edit.clone()
                        } else {
                            base.clone()
                        }
                    },
                    |graph| {
                        let stats = labeler
                            .update_incremental(&mut label_state, graph, &mut label_workspace)
                            .unwrap();
                        std::hint::black_box(stats.recomputed_vertex_rounds());
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        let mut topology_state = labeler.incremental_state(base.clone()).unwrap();
        let mut topology_workspace = IncrementalGraphWorkspace::new();
        topology_workspace
            .reserve_for(vertices, topology_edit.incidence_count(), 4)
            .unwrap();
        let mut topology_toggle = false;
        group.bench_function(
            BenchmarkId::new("incremental_single_edge_edit", vertices),
            |bench| {
                bench.iter_batched(
                    || {
                        topology_toggle = !topology_toggle;
                        if topology_toggle {
                            topology_edit.clone()
                        } else {
                            base.clone()
                        }
                    },
                    |graph| {
                        let stats = labeler
                            .update_incremental(&mut topology_state, graph, &mut topology_workspace)
                            .unwrap();
                        std::hint::black_box(stats.recomputed_vertex_rounds());
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

fn benchmark_degeneracy_and_exact(criterion: &mut Criterion) {
    let labeler = F251GraphLabeler::<3>::f251(RefinementProfile::Fast { rounds: 4 }).unwrap();
    let mut diagnosis = criterion.benchmark_group("graph_degeneracy_diagnosis");
    diagnosis.sample_size(10);
    for vertices in [1_024_usize, 16_384, 131_072] {
        let graph = homogeneous_cycle(vertices);
        diagnosis.throughput(Throughput::Elements(
            u64::try_from(graph.incidence_count()).unwrap(),
        ));
        diagnosis.bench_with_input(
            BenchmarkId::new("f251_fast_plus_exact_1wl", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    labeler
                        .diagnose_degeneracy(std::hint::black_box(graph))
                        .unwrap()
                });
            },
        );
    }
    diagnosis.finish();

    let mut exact = criterion.benchmark_group("graph_exact_opt_in");
    exact.sample_size(10);
    let compact = Microcanon::default();
    let reference = Microcanon::default().with_strategy(MicrocanonStrategy::Reference);
    for vertices in [6_usize, 8, 10, 12] {
        let graph = homogeneous_cycle(vertices);
        assert!(matches!(
            labeler
                .canonicalize_exact(&graph, CanonicalSearchBudget::new(10_000_000))
                .unwrap(),
            ExactCanonicalOutcome::Exact { .. }
        ));
        exact.bench_with_input(
            BenchmarkId::new("g10_compact_cycle", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    compact
                        .canonicalize(
                            std::hint::black_box(graph),
                            CanonicalSearchBudget::new(10_000_000),
                        )
                        .unwrap()
                });
            },
        );
        exact.bench_with_input(
            BenchmarkId::new("g9_reference_cycle", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    reference
                        .canonicalize(
                            std::hint::black_box(graph),
                            CanonicalSearchBudget::new(10_000_000),
                        )
                        .unwrap()
                });
            },
        );
    }
    exact.finish();

    let mut discrete = criterion.benchmark_group("graph_exact_discrete");
    discrete.sample_size(10);
    for vertices in [64_usize, 256] {
        let graph = uniquely_labeled_cycle(vertices);
        discrete.bench_with_input(
            BenchmarkId::new("g10_compact", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    compact
                        .canonicalize(std::hint::black_box(graph), CanonicalSearchBudget::new(0))
                        .unwrap()
                });
            },
        );
        discrete.bench_with_input(
            BenchmarkId::new("g9_reference", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    reference
                        .canonicalize(std::hint::black_box(graph), CanonicalSearchBudget::new(0))
                        .unwrap()
                });
            },
        );
    }
    discrete.finish();
}

fn benchmark_global_v2(criterion: &mut Criterion) {
    let labeler = F251GraphLabeler::<3>::f251(RefinementProfile::Fast { rounds: 4 }).unwrap();
    let mut group = criterion.benchmark_group("graph_global_v2");
    group.sample_size(10);
    for vertices in [1_024_usize, 16_384, 131_072] {
        let graph = homogeneous_cycle(vertices);
        group.throughput(Throughput::Elements(
            u64::try_from(graph.incidence_count()).unwrap(),
        ));
        group.bench_with_input(
            BenchmarkId::new("exact_global_profile_only", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| GlobalGraphProfile::analyze(std::hint::black_box(graph)).unwrap());
            },
        );
        group.bench_with_input(
            BenchmarkId::new("hybrid_local_baseline", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| labeler.analyze_hybrid(std::hint::black_box(graph)).unwrap());
            },
        );
        group.bench_with_input(
            BenchmarkId::new("full_discriminator_global_linear", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    labeler
                        .analyze_discriminating(
                            std::hint::black_box(graph),
                            GraphDiscriminationPolicy::GlobalLinear,
                        )
                        .unwrap()
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("full_discriminator_adaptive", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    labeler
                        .analyze_discriminating(
                            std::hint::black_box(graph),
                            GraphDiscriminationPolicy::adaptive(),
                        )
                        .unwrap()
                });
            },
        );
    }
    group.finish();
}

fn benchmark_g11_invariant_channels(criterion: &mut Criterion) {
    const PROFILE: [u8; 32] = [0x47; 32];
    let encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 11);
    let catalog = LoopPatternCatalog::l0_to_l3();
    let mut group = criterion.benchmark_group("graph_g11_invariant_channels");
    group.sample_size(10);
    for vertices in [8_usize, 16, 32] {
        let graph = homogeneous_cycle(vertices);
        group.throughput(Throughput::Elements(
            u64::try_from(graph.incidence_count()).unwrap(),
        ));
        group.bench_with_input(
            BenchmarkId::new("initial_cell_moments_goldilocks_k3_d4", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    CellMomentProfile::<FpGoldilocks64V1, 3, 4>::analyze_initial(
                        std::hint::black_box(graph),
                        &encoder,
                    )
                    .unwrap()
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("connected_induced_catalog_l0_l3", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    catalog
                        .analyze(std::hint::black_box(graph), u64::MAX)
                        .unwrap()
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("catalog_plus_field_compression_goldilocks_k3", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    let exact = catalog
                        .analyze(std::hint::black_box(graph), u64::MAX)
                        .unwrap();
                    PatternFieldFingerprint::<FpGoldilocks64V1, 3>::from_profile(&exact, &encoder)
                        .unwrap()
                });
            },
        );
        let exact_profile = catalog.analyze(&graph, u64::MAX).unwrap();
        group.bench_function(
            BenchmarkId::new("pattern_additive_compression_only_goldilocks_k3", vertices),
            |bench| {
                bench.iter(|| {
                    PatternFieldFingerprint::<FpGoldilocks64V1, 3>::from_profile(
                        std::hint::black_box(&exact_profile),
                        &encoder,
                    )
                    .unwrap()
                });
            },
        );
        group.bench_function(
            BenchmarkId::new("pattern_product_compression_only_goldilocks_k3", vertices),
            |bench| {
                bench.iter(|| {
                    PatternProductFingerprint::<FpGoldilocks64V1, 3>::from_profile(
                        std::hint::black_box(&exact_profile),
                        &encoder,
                    )
                    .unwrap()
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("matrix_trace4_char_eval_goldilocks_k3", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    RelationalMatrixProfile::<FpGoldilocks64V1, 3>::analyze(
                        std::hint::black_box(graph),
                        4,
                        &encoder,
                        u64::MAX,
                    )
                    .unwrap()
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("theta_rg2_goldilocks_k3", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    RelationalThetaProfile::<FpGoldilocks64V1, 3>::analyze(
                        std::hint::black_box(graph),
                        &encoder,
                        u64::MAX,
                    )
                    .unwrap()
                });
            },
        );
        let long_plan = ClosedWalkQueryPlan::new(vec![16, 64, 1_000_000_000_000]).unwrap();
        group.bench_with_input(
            BenchmarkId::new("closed_walk_recurrence_goldilocks_k3", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    RelationalClosedWalkProfile::<FpGoldilocks64V1, 3>::analyze(
                        std::hint::black_box(graph),
                        long_plan.clone(),
                        &encoder,
                        u64::MAX,
                    )
                    .unwrap()
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("non_backtracking_recurrence_goldilocks_k3", vertices),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    RelationalClosedWalkProfile::<FpGoldilocks64V1, 3>::analyze_non_backtracking(
                        std::hint::black_box(graph),
                        long_plan.clone(),
                        &encoder,
                        u64::MAX,
                    )
                    .unwrap()
                });
            },
        );
    }
    group.finish();
}

fn benchmark_g12_paired_comparison(criterion: &mut Criterion) {
    let canon = Microcanon::default();
    let budget = CanonicalSearchBudget::new(10_000_000);
    let mut group = criterion.benchmark_group("graph_g12_paired_vs_two_canonizations");
    group.sample_size(10);
    for vertices in [128_usize, 1_024, 4_096] {
        let left = homogeneous_path(vertices);
        let right = reverse_relabel(&left);
        group.throughput(Throughput::Elements(
            u64::try_from(left.incidence_count() + right.incidence_count()).unwrap(),
        ));
        group.bench_function(BenchmarkId::new("paired_tree_exact", vertices), |bench| {
            bench.iter(|| {
                canon
                    .compare(
                        std::hint::black_box(&left),
                        std::hint::black_box(&right),
                        budget,
                    )
                    .unwrap()
            });
        });
        if vertices <= 1_024 {
            group.bench_function(
                BenchmarkId::new("two_independent_canonizations", vertices),
                |bench| {
                    bench.iter(|| {
                        let left_form = canon
                            .canonicalize(std::hint::black_box(&left), budget)
                            .unwrap();
                        let right_form = canon
                            .canonicalize(std::hint::black_box(&right), budget)
                            .unwrap();
                        std::hint::black_box((left_form, right_form));
                    });
                },
            );
        }
    }
    group.finish();
}

fn benchmark_g13_g14_adaptive_execution(criterion: &mut Criterion) {
    let encoder = PrimeIntegerEncoder::new(GRAPH_DOMAIN ^ 0x1314);
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 3>::new(encoder, RefinementProfile::Fast { rounds: 6 })
            .unwrap();
    let pipeline = AdaptiveGraphPipeline::new(
        labeler.clone(),
        Microcanon::default(),
        AdaptiveFilterPolicy::default(),
    )
    .unwrap();
    let mut group = criterion.benchmark_group("graph_g13_g14_adaptive");
    group.sample_size(20);
    for vertices in [128_usize, 1_024, 4_096] {
        let path = homogeneous_path(vertices);
        let star = homogeneous_star(vertices);
        group.bench_function(
            BenchmarkId::new("pipeline_degree_reject", vertices),
            |bench| {
                bench.iter(|| {
                    pipeline
                        .compare(std::hint::black_box(&path), std::hint::black_box(&star))
                        .unwrap()
                });
            },
        );

        let mut delta = GraphDelta::new();
        delta
            .set_vertex_label(VertexId::new(0), b"delta".to_vec())
            .unwrap();
        let base_state = labeler.incremental_state(path.clone()).unwrap();
        group.bench_function(
            BenchmarkId::new("transactional_local_delta", vertices),
            |bench| {
                bench.iter_batched(
                    || (base_state.clone(), IncrementalGraphWorkspace::new()),
                    |(mut state, mut workspace)| {
                        labeler
                            .apply_delta(
                                &mut state,
                                std::hint::black_box(&delta),
                                GraphDeltaPolicy::new(8, 1_000).unwrap(),
                                &mut workspace,
                            )
                            .unwrap()
                    },
                    BatchSize::SmallInput,
                );
            },
        );
        let changed = path_with_first_label(vertices, b"delta");
        group.bench_function(
            BenchmarkId::new("complete_state_rebuild", vertices),
            |bench| {
                bench.iter(|| {
                    labeler
                        .incremental_state(std::hint::black_box(changed.clone()))
                        .unwrap()
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    benchmark_fast_graph,
    benchmark_incremental_graph,
    benchmark_degeneracy_and_exact,
    benchmark_global_v2,
    benchmark_g11_invariant_channels,
    benchmark_g12_paired_comparison,
    benchmark_g13_g14_adaptive_execution
);
criterion_main!(benches);
