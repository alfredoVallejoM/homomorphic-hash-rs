//! Batch benchmark separating algorithms, façade validation and ISA dispatch.

#![allow(missing_docs)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use microfield::{
    BackendId, BuiltinField, CanonicalEncoding, Engine, Field, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1,
};

const BATCH_LEN: usize = 4096;
const BATCH_SIZES: &[usize] = &[1, 8, 64, BATCH_LEN];

fn portable_batch(criterion: &mut Criterion) {
    benchmark_engine_construction(criterion);
    benchmark_field::<Gf2_128V1>(criterion, "gf2_128_v1", 16);
    benchmark_field::<Gf2_256HhV1>(criterion, "gf2_256_hh_v1", 32);
    benchmark_field::<Gf2_256AltV1>(criterion, "gf2_256_alt_v1", 32);
}

fn benchmark_engine_construction(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("engine/construction");
    group.bench_function("portable_capabilities", |bencher| {
        bencher.iter(|| {
            black_box(
                Engine::<Gf2_256HhV1>::builder()
                    .expected_batch(black_box(BATCH_LEN))
                    .build()
                    .expect("portable strategy is compiled"),
            );
        });
    });
    group.bench_function("detected_capabilities", |bencher| {
        bencher.iter(|| {
            black_box(
                Engine::<Gf2_256HhV1>::builder()
                    .expected_batch(black_box(BATCH_LEN))
                    .detect()
                    .expect("portable fallback is compiled"),
            );
        });
    });
    group.finish();
}

fn benchmark_field<F>(criterion: &mut Criterion, name: &str, bytes: usize)
where
    F: BuiltinField + CanonicalEncoding + core::fmt::Debug,
{
    let portable = Engine::<F>::portable();
    let selected = Engine::<F>::builder()
        .expected_batch(BATCH_LEN)
        .detect()
        .expect("portable fallback is always compiled");

    for &len in BATCH_SIZES {
        let lhs = values::<F>(len, bytes, 0xa5);
        let rhs = values::<F>(len, bytes, 0x3c);
        let mut output = vec![F::ZERO; len];
        let mut group = criterion.benchmark_group(format!("{name}/batch/{len}"));

        group.bench_function("mul_direct_portable", |bencher| {
            bencher.iter(|| direct_mul(black_box(&mut output), black_box(&lhs), black_box(&rhs)));
        });
        group.bench_function("mul_engine_portable", |bencher| {
            bencher.iter(|| {
                portable
                    .mul_into(black_box(&mut output), black_box(&lhs), black_box(&rhs))
                    .expect("benchmark lengths are equal");
            });
        });
        let selected_name = match selected.backend_id() {
            BackendId::X86Pclmul => "mul_engine_x86_pclmul",
            _ => "mul_engine_selected_fallback",
        };
        group.bench_function(selected_name, |bencher| {
            bencher.iter(|| {
                selected
                    .mul_into(black_box(&mut output), black_box(&lhs), black_box(&rhs))
                    .expect("benchmark lengths are equal");
            });
        });
        group.bench_function("square_engine_portable", |bencher| {
            bencher.iter(|| {
                portable
                    .square_into(black_box(&mut output), black_box(&lhs))
                    .expect("benchmark lengths are equal");
            });
        });
        let selected_square_name = match selected.backend_id() {
            BackendId::X86Pclmul => "square_engine_x86_pclmul",
            _ => "square_engine_selected_fallback",
        };
        group.bench_function(selected_square_name, |bencher| {
            bencher.iter(|| {
                selected
                    .square_into(black_box(&mut output), black_box(&lhs))
                    .expect("benchmark lengths are equal");
            });
        });
        group.bench_function("add_direct", |bencher| {
            bencher.iter(|| direct_add(black_box(&mut output), black_box(&lhs), black_box(&rhs)));
        });
        group.bench_function("add_engine", |bencher| {
            bencher.iter(|| {
                selected
                    .add_into(black_box(&mut output), black_box(&lhs), black_box(&rhs))
                    .expect("benchmark lengths are equal");
            });
        });
        group.finish();
    }
}

fn direct_mul<F: Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
    for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
        *output = left.mul(*right);
    }
}

fn direct_add<F: Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
    for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
        *output = left.add(*right);
    }
}

fn values<F: CanonicalEncoding>(len: usize, bytes: usize, seed: u8) -> Vec<F> {
    (0..len)
        .map(|index| {
            let mut repr = vec![seed; bytes];
            for (offset, byte) in repr.iter_mut().enumerate() {
                *byte ^= index.wrapping_mul(offset + 1).to_le_bytes()[0];
            }
            F::from_canonical_slice(&repr).expect("full-width values are canonical")
        })
        .collect()
}

criterion_group!(benches, portable_batch);
criterion_main!(benches);
