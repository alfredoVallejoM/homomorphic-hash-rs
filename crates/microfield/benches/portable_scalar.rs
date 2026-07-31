//! Reproducible scalar baselines for the first portable vertical.

// `criterion_group!` emits one public harness function that cannot carry docs.
#![allow(missing_docs)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use microfield::{BinaryPolynomialField, CanonicalEncoding, Gf2_256HhV1, Invert, Square};

fn portable_scalar(criterion: &mut Criterion) {
    let lhs = element([0xa5; 32]);
    let rhs = element([0x3c; 32]);
    let wide = [0x96; 64];

    let mut group = criterion.benchmark_group("gf2_256_hh_v1/portable_scalar");
    group.bench_function("multiply", |bencher| {
        bencher.iter(|| black_box(lhs) * black_box(rhs));
    });
    group.bench_function("square", |bencher| {
        bencher.iter(|| black_box(lhs).square());
    });
    group.bench_function("mul_by_x", |bencher| {
        bencher.iter(|| black_box(lhs).mul_by_x());
    });
    group.bench_function("reduce_64_bytes", |bencher| {
        bencher.iter(|| Gf2_256HhV1::from_polynomial_bytes_mod(black_box(&wide)));
    });
    group.bench_function("invert", |bencher| {
        bencher.iter(|| black_box(lhs).invert());
    });
    group.finish();
}

fn element(bytes: [u8; 32]) -> Gf2_256HhV1 {
    Gf2_256HhV1::from_canonical(&bytes).expect("all 256-bit values are canonical")
}

criterion_group!(benches, portable_scalar);
criterion_main!(benches);
