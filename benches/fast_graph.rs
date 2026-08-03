//! Throughput comparison for bounded graph refinement over compact and wide fields.

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use homomorphic_hash_rs::{
    BinaryPolynomialEncoder, CanonicalSearchBudget, ExactCanonicalOutcome, F251BatchGraphWorkspace,
    F251GraphLabeler, FastGraphLabeler, GlobalGraphProfile, GraphDiscriminationPolicy,
    GraphExecution, GraphWorkspace, IncidenceGraph, IncidenceGraphBuilder,
    IncrementalGraphWorkspace, Microcanon, MicrocanonStrategy, RefinementProfile,
};
use microfield::Gf2_256HhV1;

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

criterion_group!(
    benches,
    benchmark_fast_graph,
    benchmark_incremental_graph,
    benchmark_degeneracy_and_exact,
    benchmark_global_v2
);
criterion_main!(benches);
