//! Phase 3 benchmarks separating scalar baselines from reusable plans.

#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use microfield::{
    BatchInvertPlan, BatchInvertWorkspace, BitMaskViewMut, CanonicalEncoding, CoefficientLayout,
    Engine, Field, Gf2_256HhV1, Invert, ManyPointsHornerPlan, ManyPolynomialsHornerPlan,
    ProductScanPlan, ScanDirection, ScanMode, fill_fixed_base_powers, required_mask_words,
};

const SIZES: &[usize] = &[1, 2, 4, 8, 16, 32, 64, 256, 1024, 4096, 16_384];

fn derived_algorithms(criterion: &mut Criterion) {
    benchmark_inversion(criterion);
    benchmark_scans(criterion);
    benchmark_horner(criterion);
    benchmark_fixed_powers(criterion);
}

fn benchmark_inversion(criterion: &mut Criterion) {
    let engine = Engine::<Gf2_256HhV1>::portable();
    let mut group = criterion.benchmark_group("phase3/inversion/gf2_256_hh_v1");
    for &len in SIZES {
        group.throughput(Throughput::Elements(len as u64));
        let values = values(len, 0x243f_6a88_85a3_08d3);
        let mut out = vec![Gf2_256HhV1::ZERO; len];
        let mut scalar = out.clone();
        let plan = BatchInvertPlan::new(&engine, len).expect("bounded plan");
        let mut prefixes = vec![Gf2_256HhV1::ZERO; len];
        let mut mask_words = vec![0_u64; required_mask_words(len).expect("bounded")];

        group.bench_with_input(
            BenchmarkId::new("scalar_independent", len),
            &len,
            |bencher, _| {
                bencher.iter(|| {
                    for (output, value) in scalar.iter_mut().zip(black_box(&values)) {
                        *output = value.invert().unwrap_or(Gf2_256HhV1::ZERO);
                    }
                    black_box(&scalar);
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("batch_borrowed", len),
            &len,
            |bencher, _| {
                bencher.iter(|| {
                    let mut workspace = BatchInvertWorkspace::new(black_box(&mut prefixes));
                    let mut mask = BitMaskViewMut::new(black_box(&mut mask_words), len)
                        .expect("exact mask storage");
                    plan.execute(
                        &engine,
                        black_box(&mut out),
                        black_box(&values),
                        &mut mask,
                        &mut workspace,
                    )
                    .expect("valid plan execution");
                    black_box(&out);
                });
            },
        );
    }
    group.finish();
}

fn benchmark_scans(criterion: &mut Criterion) {
    let engine = Engine::<Gf2_256HhV1>::portable();
    let mut group = criterion.benchmark_group("phase3/scans/gf2_256_hh_v1");
    for &len in SIZES {
        group.throughput(Throughput::Elements(len as u64));
        let values = values(len, 0x1319_8a2e_0370_7344);
        let mut out = vec![Gf2_256HhV1::ZERO; len];
        let prefix = ProductScanPlan::new(&engine, len, ScanDirection::Prefix, ScanMode::Inclusive);
        let suffix = ProductScanPlan::new(&engine, len, ScanDirection::Suffix, ScanMode::Inclusive);
        group.bench_with_input(BenchmarkId::new("prefix", len), &len, |bencher, _| {
            bencher.iter(|| {
                prefix
                    .execute(&engine, black_box(&mut out), black_box(&values))
                    .expect("valid scan");
            });
        });
        group.bench_with_input(BenchmarkId::new("suffix", len), &len, |bencher, _| {
            bencher.iter(|| {
                suffix
                    .execute(&engine, black_box(&mut out), black_box(&values))
                    .expect("valid scan");
            });
        });
    }
    group.finish();
}

fn benchmark_horner(criterion: &mut Criterion) {
    let engine = Engine::<Gf2_256HhV1>::portable();
    let coefficients = values(16, 0xa409_3822_299f_31d0);
    let mut group = criterion.benchmark_group("phase3/horner/gf2_256_hh_v1");
    for &len in &[1, 4, 16, 64, 256, 1024, 4096] {
        group.throughput(Throughput::Elements(len as u64));
        let points = values(len, 0x082e_fa98_ec4e_6c89);
        let mut out = vec![Gf2_256HhV1::ZERO; len];
        let plan = ManyPointsHornerPlan::new(&engine, len, coefficients.len())
            .expect("non-empty polynomial");
        group.bench_with_input(
            BenchmarkId::new("one_polynomial_many_points", len),
            &len,
            |b, _| {
                b.iter(|| {
                    plan.execute(
                        &engine,
                        black_box(&mut out),
                        black_box(&coefficients),
                        black_box(&points),
                    )
                    .expect("valid shape");
                });
            },
        );

        let matrix = values(len * coefficients.len(), 0x4528_21e6_38d0_1377);
        let many_plan = ManyPolynomialsHornerPlan::new(
            &engine,
            len,
            coefficients.len(),
            CoefficientLayout::PolynomialMajor,
        )
        .expect("bounded matrix");
        group.bench_with_input(
            BenchmarkId::new("many_polynomials_one_point", len),
            &len,
            |b, _| {
                b.iter(|| {
                    many_plan
                        .execute(
                            &engine,
                            black_box(&mut out),
                            black_box(&matrix),
                            black_box(points[0]),
                        )
                        .expect("valid shape");
                });
            },
        );
    }
    group.finish();
}

fn benchmark_fixed_powers(criterion: &mut Criterion) {
    let base = values(1, 0xbe54_66cf_34e9_0c6c)[0];
    let mut group = criterion.benchmark_group("phase3/fixed_powers/gf2_256_hh_v1");
    for &len in &[1, 16, 64, 256, 1024, 4096] {
        group.throughput(Throughput::Elements(len as u64));
        let mut powers = vec![Gf2_256HhV1::ZERO; len];
        group.bench_with_input(BenchmarkId::from_parameter(len), &len, |bencher, _| {
            bencher.iter(|| fill_fixed_base_powers(black_box(&mut powers), black_box(base)));
        });
    }
    group.finish();
}

fn values(len: usize, seed: u64) -> Vec<Gf2_256HhV1> {
    (0..len)
        .map(|index| {
            let mut bytes = [0_u8; 32];
            for (offset, byte) in bytes.iter_mut().enumerate() {
                let rotation = u32::try_from(offset % 64).expect("rotation is below 64");
                let mixed = seed
                    .wrapping_add(index as u64)
                    .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                    .rotate_left(rotation);
                *byte = mixed.to_le_bytes()[offset % 8];
            }
            let value = Gf2_256HhV1::from_canonical(&bytes).expect("full-width binary value");
            if index % 29 == 0 {
                Gf2_256HhV1::ZERO
            } else if value.is_zero() {
                Gf2_256HhV1::ONE
            } else {
                value
            }
        })
        .collect()
}

criterion_group!(benches, derived_algorithms);
criterion_main!(benches);
