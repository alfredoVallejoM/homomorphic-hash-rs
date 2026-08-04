//! Comparative throughput for corrected structural laws.

use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use homomorphic_hash_rs::{
    AdditiveSignature, BidirectionalSequenceSignature, BinaryPolynomialEncoder,
    DegreeHistogramProfile, FileChunkProfile, HomomorphicSummaryTree, IncidenceGraphBuilder,
    MultiEvaluationMultisetSignature, MultiEvaluationSequenceSignature, MultisetSignature,
    PrimeIntegerEncoder, SequenceSignature,
};
use microfield::{BinaryPolynomialField, Field, Fp251V1, Gf2_256HhV1};

fn cycle(order: usize) -> homomorphic_hash_rs::IncidenceGraph {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = (0..order)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect::<Vec<_>>();
    for index in 0..order {
        builder
            .add_undirected_relation(
                vertices[index],
                vertices[(index + 1) % order],
                b"e".to_vec(),
                Vec::new(),
                1,
            )
            .unwrap();
    }
    builder.build().unwrap()
}

fn structural_signatures(criterion: &mut Criterion) {
    let encoder = BinaryPolynomialEncoder::new(0x4245_4e43_4800_0001);
    let base = Gf2_256HhV1::ONE.mul_by_x();
    let inputs = (0_u64..16_384).map(u64::to_le_bytes).collect::<Vec<_>>();
    let mut group = criterion.benchmark_group("phase6/structural-signatures");

    for length in [64_usize, 1_024, 16_384] {
        group.throughput(Throughput::Elements(length as u64));
        group.bench_with_input(
            BenchmarkId::new("additive", length),
            &length,
            |bench, &len| {
                bench.iter(|| {
                    let mut signature = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
                    signature.absorb_many(black_box(&inputs[..len])).unwrap();
                    black_box(signature)
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("sequence", length),
            &length,
            |bench, &len| {
                bench.iter(|| {
                    let mut signature =
                        SequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
                    signature.push_many(black_box(&inputs[..len])).unwrap();
                    black_box(signature)
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("multiset", length),
            &length,
            |bench, &len| {
                bench.iter(|| {
                    let mut signature =
                        MultisetSignature::<Gf2_256HhV1, _>::new(encoder, Gf2_256HhV1::ONE);
                    signature.insert_many(black_box(&inputs[..len])).unwrap();
                    black_box(signature)
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("bidirectional-sequence", length),
            &length,
            |bench, &len| {
                bench.iter(|| {
                    let mut signature =
                        BidirectionalSequenceSignature::<Gf2_256HhV1, _>::new(encoder, base)
                            .unwrap();
                    signature.push_slice(black_box(&inputs[..len])).unwrap();
                    black_box(signature)
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("multiset-3-evaluations", length),
            &length,
            |bench, &len| {
                bench.iter(|| {
                    let mut signature = MultiEvaluationMultisetSignature::<Gf2_256HhV1, _, 3>::new(
                        encoder,
                        [Gf2_256HhV1::ZERO, Gf2_256HhV1::ONE, base],
                    )
                    .unwrap();
                    signature.insert_many(black_box(&inputs[..len])).unwrap();
                    black_box(signature)
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("sequence-3-evaluations", length),
            &length,
            |bench, &len| {
                bench.iter(|| {
                    let base_squared = base.mul(base);
                    let mut signature = MultiEvaluationSequenceSignature::<Gf2_256HhV1, _, 3>::new(
                        encoder,
                        [base, base_squared, base_squared.mul(base)],
                    )
                    .unwrap();
                    signature.push_many(black_box(&inputs[..len])).unwrap();
                    black_box(signature)
                });
            },
        );
    }
    let degree_encoder = PrimeIntegerEncoder::new(0x4445_4752_4545_424e);
    let degree_offsets = [
        Fp251V1::ONE,
        Fp251V1::from_u64_mod(2),
        Fp251V1::from_u64_mod(3),
    ];
    for order in [64_usize, 1_024, 16_384] {
        let graph = cycle(order);
        group.throughput(Throughput::Elements(order as u64));
        group.bench_with_input(
            BenchmarkId::new("graph-degree-histogram-multiset", order),
            &graph,
            |bench, graph| {
                bench.iter(|| {
                    black_box(
                        DegreeHistogramProfile::<Fp251V1, _, 3>::analyze(
                            black_box(graph),
                            degree_encoder,
                            degree_offsets,
                        )
                        .unwrap(),
                    )
                });
            },
        );
    }

    let chunk_profile = FileChunkProfile::fixed(1_024).unwrap();
    for byte_len in [64 * 1_024_usize, 1_024 * 1_024] {
        let file = (0..byte_len).map(|index| index as u8).collect::<Vec<_>>();
        let tree = HomomorphicSummaryTree::<Gf2_256HhV1, _>::from_bytes(
            chunk_profile,
            encoder,
            base,
            &file,
        )
        .unwrap();
        let edit_start = byte_len / 2 + 17;
        let edit = [0xa5; 32];
        let mut rebuilt_file = file.clone();
        rebuilt_file[edit_start..edit_start + edit.len()].copy_from_slice(&edit);
        group.throughput(Throughput::Bytes(byte_len as u64));
        group.bench_with_input(
            BenchmarkId::new("summary-tree-local-edit", byte_len),
            &tree,
            |bench, tree| {
                bench.iter_batched(
                    || tree.clone(),
                    |mut candidate| {
                        black_box(
                            candidate
                                .replace_range(edit_start..edit_start + edit.len(), &edit)
                                .unwrap(),
                        )
                    },
                    BatchSize::SmallInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("summary-tree-full-rebuild", byte_len),
            &rebuilt_file,
            |bench, bytes| {
                bench.iter(|| {
                    black_box(
                        HomomorphicSummaryTree::<Gf2_256HhV1, _>::from_bytes(
                            chunk_profile,
                            encoder,
                            base,
                            black_box(bytes),
                        )
                        .unwrap(),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, structural_signatures);
criterion_main!(benches);
