//! Comparative throughput for corrected structural laws.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use homomorphic_hash_rs::{
    AdditiveSignature, BidirectionalSequenceSignature, BinaryPolynomialEncoder,
    MultiEvaluationMultisetSignature, MultisetSignature, SequenceSignature,
};
use microfield::{BinaryPolynomialField, Field, Gf2_256HhV1};

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
    }
    group.finish();
}

criterion_group!(benches, structural_signatures);
criterion_main!(benches);
