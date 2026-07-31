//! Reproducible scalar baselines for the first portable vertical.

// `criterion_group!` emits one public harness function that cannot carry docs.
#![allow(missing_docs)]

use core::ops::Mul;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use microfield::{
    BinaryPolynomialField, CanonicalEncoding, Field, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, Invert,
    Square,
};

fn portable_scalar(criterion: &mut Criterion) {
    benchmark_field(
        criterion,
        "gf2_128_v1",
        element::<Gf2_128V1>(&[0xa5; 16]),
        element::<Gf2_128V1>(&[0x3c; 16]),
        &[0x96; 32],
    );
    benchmark_field(
        criterion,
        "gf2_256_hh_v1",
        element::<Gf2_256HhV1>(&[0xa5; 32]),
        element::<Gf2_256HhV1>(&[0x3c; 32]),
        &[0x96; 64],
    );
    benchmark_field(
        criterion,
        "gf2_256_alt_v1",
        element::<Gf2_256AltV1>(&[0xa5; 32]),
        element::<Gf2_256AltV1>(&[0x3c; 32]),
        &[0x96; 64],
    );
}

fn benchmark_field<F>(criterion: &mut Criterion, name: &str, lhs: F, rhs: F, wide: &[u8])
where
    F: Field + Square + Invert + BinaryPolynomialField + Mul<Output = F> + core::fmt::Debug,
{
    let mut group = criterion.benchmark_group(format!("{name}/portable_scalar"));
    group.bench_function("multiply", |bencher| {
        bencher.iter(|| black_box(lhs) * black_box(rhs));
    });
    group.bench_function("square", |bencher| {
        bencher.iter(|| black_box(lhs).square());
    });
    group.bench_function("mul_by_x", |bencher| {
        bencher.iter(|| black_box(lhs).mul_by_x());
    });
    group.bench_function("reduce_double_width", |bencher| {
        bencher.iter(|| F::from_polynomial_bytes_mod(black_box(wide)));
    });
    group.bench_function("invert", |bencher| {
        bencher.iter(|| black_box(lhs).invert());
    });
    group.finish();
}

fn element<F: CanonicalEncoding>(bytes: &[u8]) -> F {
    F::from_canonical_slice(bytes).expect("all full-width binary values are canonical")
}

criterion_group!(benches, portable_scalar);
criterion_main!(benches);
