//! Phase 4 benchmarks separating arithmetic, reduction, conversion and ISA dispatch.

#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use microfield::{
    BackendId, CanonicalEncoding, CpuCapabilities, Engine, Field, Fp251V1, Fp256GenericV1,
    FpGoldilocks64V1, Invert, PrimeField, Square,
};

const BATCH_SIZES: &[usize] = &[1, 4, 16, 32, 64, 256, 1024, 4096, 16_384];

fn prime_fields(criterion: &mut Criterion) {
    scalar(criterion);
    goldilocks_reductions(criterion);
    fp251_batch(criterion);
    fp256_batch(criterion);
    montgomery_conversion(criterion);
}

fn scalar(criterion: &mut Criterion) {
    scalar_field::<Fp251V1>(criterion, "fp251_v1", &[17], &[231]);
    scalar_field::<FpGoldilocks64V1>(
        criterion,
        "fp_goldilocks64_v1",
        &0x243f_6a88_85a3_08d3_u64.to_le_bytes(),
        &0x1319_8a2e_0370_7344_u64.to_le_bytes(),
    );
    scalar_field::<Fp256GenericV1>(criterion, "fp256_generic_v1", &[0xa5; 32], &[0x3c; 32]);
}

fn scalar_field<F: PrimeField + Square + Invert>(
    criterion: &mut Criterion,
    name: &str,
    left: &[u8],
    right: &[u8],
) {
    let left = F::from_bytes_mod_order(left);
    let right = F::from_bytes_mod_order(right);
    let mut group = criterion.benchmark_group(format!("phase4/scalar/{name}"));
    group.bench_function("add", |b| b.iter(|| black_box(left).add(black_box(right))));
    group.bench_function("mul", |b| b.iter(|| black_box(left).mul(black_box(right))));
    group.bench_function("square", |b| b.iter(|| black_box(left).square()));
    group.bench_function("invert", |b| b.iter(|| black_box(left).invert()));
    group.finish();
}

fn goldilocks_reductions(criterion: &mut Criterion) {
    let lhs = 0xfedc_ba98_7654_3210_u128;
    let rhs = 0xdead_beef_cafe_babe_u128;
    let wide = lhs * rhs;
    let left = FpGoldilocks64V1::from_u64_mod(u64::try_from(lhs).unwrap());
    let right = FpGoldilocks64V1::from_u64_mod(u64::try_from(rhs).unwrap());
    let mut group = criterion.benchmark_group("phase4/reduction/goldilocks");
    group.bench_function("solinas_field_mul", |b| {
        b.iter(|| black_box(left).mul(black_box(right)));
    });
    group.bench_function("barrett_reduce", |b| {
        b.iter(|| FpGoldilocks64V1::__barrett_reduce_wide(black_box(wide)));
    });
    group.finish();
}

fn fp251_batch(criterion: &mut Criterion) {
    let portable = Engine::<Fp251V1>::portable();
    let capabilities = CpuCapabilities::detect();
    let avx2 = capabilities.has_x86_avx2().then(|| {
        Engine::<Fp251V1>::builder()
            .capabilities(capabilities)
            .force_backend(BackendId::X86PrimeAvx2)
            .build()
            .expect("detected AVX2")
    });
    let mut group = criterion.benchmark_group("phase4/batch/fp251_v1");
    for &len in BATCH_SIZES {
        group.throughput(Throughput::Elements(len as u64));
        let lhs = values::<Fp251V1>(len, 0x243f_6a88_85a3_08d3);
        let rhs = values::<Fp251V1>(len, 0x1319_8a2e_0370_7344);
        let mut out = vec![Fp251V1::ZERO; len];
        group.bench_with_input(BenchmarkId::new("portable_mul", len), &len, |b, _| {
            b.iter(|| portable.mul_into(black_box(&mut out), black_box(&lhs), black_box(&rhs)));
        });
        if let Some(avx2) = avx2 {
            group.bench_with_input(BenchmarkId::new("avx2_mul", len), &len, |b, _| {
                b.iter(|| avx2.mul_into(black_box(&mut out), black_box(&lhs), black_box(&rhs)));
            });
        }
    }
    group.finish();
}

fn fp256_batch(criterion: &mut Criterion) {
    let portable = Engine::<Fp256GenericV1>::portable();
    let capabilities = CpuCapabilities::detect();
    let bmi2 = capabilities.has_x86_bmi2().then(|| {
        Engine::<Fp256GenericV1>::builder()
            .capabilities(capabilities)
            .force_backend(BackendId::X86PrimeBmi2)
            .build()
            .expect("detected BMI2")
    });
    let mut group = criterion.benchmark_group("phase4/batch/fp256_generic_v1");
    for &len in BATCH_SIZES {
        group.throughput(Throughput::Elements(len as u64));
        let lhs = values::<Fp256GenericV1>(len, 0xa409_3822_299f_31d0);
        let rhs = values::<Fp256GenericV1>(len, 0x082e_fa98_ec4e_6c89);
        let mut out = vec![Fp256GenericV1::ZERO; len];
        group.bench_with_input(BenchmarkId::new("portable_mul", len), &len, |b, _| {
            b.iter(|| portable.mul_into(black_box(&mut out), black_box(&lhs), black_box(&rhs)));
        });
        if let Some(bmi2) = bmi2 {
            group.bench_with_input(BenchmarkId::new("bmi2_mul", len), &len, |b, _| {
                b.iter(|| bmi2.mul_into(black_box(&mut out), black_box(&lhs), black_box(&rhs)));
            });
        }
    }
    group.finish();
}

fn montgomery_conversion(criterion: &mut Criterion) {
    let canonical = [0x42_u8; 32];
    let value = Fp256GenericV1::from_canonical(&canonical).expect("canonical fixture");
    let mut group = criterion.benchmark_group("phase4/conversion/fp256_generic_v1");
    group.bench_function("to_montgomery", |b| {
        b.iter(|| Fp256GenericV1::from_canonical(black_box(&canonical)));
    });
    group.bench_function("from_montgomery", |b| {
        b.iter(|| black_box(value).to_canonical());
    });
    group.finish();
}

fn values<F: PrimeField>(len: usize, seed: u64) -> Vec<F> {
    (0..len)
        .map(|index| {
            let mut bytes = [0_u8; 48];
            for (offset, byte) in bytes.iter_mut().enumerate() {
                let mixed = seed
                    .wrapping_add(index as u64)
                    .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                    .rotate_left(u32::try_from(offset % 64).unwrap());
                *byte = mixed.to_le_bytes()[offset % 8];
            }
            F::from_bytes_mod_order(&bytes)
        })
        .collect()
}

criterion_group!(benches, prime_fields);
criterion_main!(benches);
