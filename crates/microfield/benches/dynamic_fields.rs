//! Phase 5 static/dynamic and scalar/batch comparison harness.

#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use microfield::{DynBatch, DynField, Engine, Field, Fp251V1};

fn dynamic_fields(criterion: &mut Criterion) {
    let field = DynField::builder("fp251_benchmark")
        .prime("251")
        .build()
        .expect("benchmark modulus is proven");
    let left = field.decode(&[137]).expect("canonical");
    let right = field.decode(&[211]).expect("canonical");

    let mut scalar = criterion.benchmark_group("phase5/dynamic-scalar");
    scalar.bench_function("fp251/mul", |bencher| {
        bencher.iter(|| field.mul(black_box(&left), black_box(&right)).unwrap());
    });
    scalar.bench_function("fp251/static-mul", |bencher| {
        let left = Fp251V1::from_u64_mod(137);
        let right = Fp251V1::from_u64_mod(211);
        bencher.iter(|| left.mul(black_box(right)));
    });
    scalar.finish();

    let mut batch = criterion.benchmark_group("phase5/dynamic-batch");
    for len in [64_usize, 1_024, 16_384] {
        batch.throughput(Throughput::Elements(
            u64::try_from(len).expect("benchmark length"),
        ));
        let values = (0..len)
            .map(|index| {
                field
                    .decode(&[u8::try_from(index % 251).expect("reduced")])
                    .expect("canonical")
            })
            .collect::<Vec<_>>();
        let lhs = DynBatch::from_elements(&field, &values).unwrap();
        let rhs = lhs.clone();
        let mut out = DynBatch::zeroed(&field, len);
        let engine = field.engine();
        batch.bench_with_input(BenchmarkId::new("fp251/mul", len), &len, |bencher, _| {
            bencher.iter(|| engine.mul_into(black_box(&mut out), &lhs, &rhs).unwrap());
        });

        let static_values = (0..len)
            .map(|index| Fp251V1::from_u64_mod(u64::try_from(index).expect("benchmark index")))
            .collect::<Vec<_>>();
        let mut static_out = vec![Fp251V1::ZERO; len];
        let static_engine = Engine::<Fp251V1>::portable();
        batch.bench_with_input(
            BenchmarkId::new("fp251/static-mul", len),
            &len,
            |bencher, _| {
                bencher.iter(|| {
                    static_engine
                        .mul_into(black_box(&mut static_out), &static_values, &static_values)
                        .unwrap();
                });
            },
        );
    }
    batch.finish();
}

criterion_group!(benches, dynamic_fields);
criterion_main!(benches);
