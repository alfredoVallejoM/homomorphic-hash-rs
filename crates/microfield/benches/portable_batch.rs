//! Batch benchmark separating algorithms, façade validation and ISA dispatch.

#![allow(missing_docs)]

use criterion::{
    BenchmarkGroup, Criterion, black_box, criterion_group, criterion_main, measurement::WallTime,
};
use microfield::{
    BackendId, BuiltinField, CanonicalEncoding, Engine, Field, Gf2_128V1, Gf2_256AltV1,
    Gf2_256HhV1, PackedBatch, StaticField,
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
    F: BuiltinField + CanonicalEncoding + StaticField + core::fmt::Debug,
{
    let portable = Engine::<F>::portable();
    let selected = Engine::<F>::builder()
        .expected_batch(BATCH_LEN)
        .detect()
        .expect("portable fallback is always compiled");
    let forced_isa = detected_forced_isa::<F>();
    let forced_vpclmul = detected_forced_vpclmul::<F>();

    for &len in BATCH_SIZES {
        benchmark_batch(
            criterion,
            name,
            bytes,
            len,
            portable,
            selected,
            forced_isa,
            forced_vpclmul,
        );
    }
}

fn detected_forced_vpclmul<F: BuiltinField>() -> Option<Engine<F>> {
    #[cfg(target_arch = "x86_64")]
    return Engine::<F>::builder()
        .force_backend(BackendId::X86Vpclmul)
        .detect()
        .ok();
    #[cfg(not(target_arch = "x86_64"))]
    None
}

fn detected_forced_isa<F: BuiltinField>() -> Option<Engine<F>> {
    #[cfg(target_arch = "x86_64")]
    return Engine::<F>::builder()
        .force_backend(BackendId::X86Pclmul)
        .detect()
        .ok();
    #[cfg(target_arch = "aarch64")]
    return Engine::<F>::builder()
        .force_backend(BackendId::Aarch64Pmull)
        .detect()
        .ok();
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    None
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn benchmark_batch<F>(
    criterion: &mut Criterion,
    name: &str,
    bytes: usize,
    len: usize,
    portable: Engine<F>,
    selected: Engine<F>,
    forced_isa: Option<Engine<F>>,
    forced_vpclmul: Option<Engine<F>>,
) where
    F: BuiltinField + CanonicalEncoding + StaticField + core::fmt::Debug,
{
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
        BackendId::X86Vpclmul => "mul_engine_x86_vpclmul",
        BackendId::Aarch64Pmull => "mul_engine_aarch64_pmull",
        _ => "mul_engine_selected_fallback",
    };
    group.bench_function(selected_name, |bencher| {
        bencher.iter(|| {
            selected
                .mul_into(black_box(&mut output), black_box(&lhs), black_box(&rhs))
                .expect("benchmark lengths are equal");
        });
    });
    if let Some(isa) = forced_isa {
        let isa_name = match isa.backend_id() {
            BackendId::X86Pclmul => "mul_engine_forced_x86_pclmul",
            BackendId::Aarch64Pmull => "mul_engine_forced_aarch64_pmull",
            _ => "mul_engine_forced_fallback",
        };
        group.bench_function(isa_name, |bencher| {
            bencher.iter(|| {
                isa.mul_into(black_box(&mut output), black_box(&lhs), black_box(&rhs))
                    .expect("benchmark lengths are equal");
            });
        });
    }
    if let Some(vpclmul) = forced_vpclmul {
        group.bench_function("mul_engine_forced_x86_vpclmul", |bencher| {
            bencher.iter(|| {
                vpclmul
                    .mul_into(black_box(&mut output), black_box(&lhs), black_box(&rhs))
                    .expect("benchmark lengths are equal");
            });
        });
    }
    group.bench_function("square_engine_portable", |bencher| {
        bencher.iter(|| {
            portable
                .square_into(black_box(&mut output), black_box(&lhs))
                .expect("benchmark lengths are equal");
        });
    });
    let selected_square_name = match selected.backend_id() {
        BackendId::X86Pclmul => "square_engine_x86_pclmul",
        BackendId::X86Vpclmul => "square_engine_x86_vpclmul",
        BackendId::Aarch64Pmull => "square_engine_aarch64_pmull",
        _ => "square_engine_selected_fallback",
    };
    group.bench_function(selected_square_name, |bencher| {
        bencher.iter(|| {
            selected
                .square_into(black_box(&mut output), black_box(&lhs))
                .expect("benchmark lengths are equal");
        });
    });
    if let Some(isa) = forced_isa {
        let isa_name = match isa.backend_id() {
            BackendId::X86Pclmul => "square_engine_forced_x86_pclmul",
            BackendId::Aarch64Pmull => "square_engine_forced_aarch64_pmull",
            _ => "square_engine_forced_fallback",
        };
        group.bench_function(isa_name, |bencher| {
            bencher.iter(|| {
                isa.square_into(black_box(&mut output), black_box(&lhs))
                    .expect("benchmark lengths are equal");
            });
        });
    }
    if let Some(vpclmul) = forced_vpclmul {
        group.bench_function("square_engine_forced_x86_vpclmul", |bencher| {
            bencher.iter(|| {
                vpclmul
                    .square_into(black_box(&mut output), black_box(&lhs))
                    .expect("benchmark lengths are equal");
            });
        });
    }
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
    benchmark_packed(&mut group, selected, &lhs, &rhs, &mut output);
    if let Some(vpclmul) = forced_vpclmul {
        benchmark_packed_vpclmul(&mut group, vpclmul, &lhs, &rhs, &mut output);
    }
    group.finish();
}

fn benchmark_packed_vpclmul<F>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    engine: Engine<F>,
    lhs: &[F],
    rhs: &[F],
    output: &mut [F],
) where
    F: BuiltinField + StaticField,
{
    let mut packed_lhs = PackedBatch::from_aos(&engine, lhs).expect("valid packed lhs");
    let mut packed_rhs = PackedBatch::from_aos(&engine, rhs).expect("valid packed rhs");
    let mut packed_output = PackedBatch::new(&engine, lhs.len()).expect("valid packed output");

    group.bench_function("vpclmul_pack_into_reused", |bencher| {
        bencher.iter(|| {
            packed_lhs
                .pack_from(black_box(lhs))
                .expect("benchmark length is fixed");
        });
    });
    group.bench_function("vpclmul_mul_packed_reused", |bencher| {
        bencher.iter(|| {
            engine
                .mul_packed_into(
                    black_box(&mut packed_output),
                    black_box(&packed_lhs),
                    black_box(&packed_rhs),
                )
                .expect("benchmark plans are equal");
        });
    });
    group.bench_function("vpclmul_pipeline_reused_pack_mul_unpack", |bencher| {
        bencher.iter(|| {
            packed_lhs
                .pack_from(black_box(lhs))
                .expect("benchmark length is fixed");
            packed_rhs
                .pack_from(black_box(rhs))
                .expect("benchmark length is fixed");
            engine
                .mul_packed_into(&mut packed_output, &packed_lhs, &packed_rhs)
                .expect("benchmark plans are equal");
            packed_output
                .unpack_into(black_box(&mut *output))
                .expect("benchmark length is fixed");
        });
    });
}

fn benchmark_packed<F>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    engine: Engine<F>,
    lhs: &[F],
    rhs: &[F],
    output: &mut [F],
) where
    F: BuiltinField + StaticField,
{
    let mut packed_lhs = PackedBatch::from_aos(&engine, lhs).expect("valid packed lhs");
    let mut packed_rhs = PackedBatch::from_aos(&engine, rhs).expect("valid packed rhs");
    let mut packed_output = PackedBatch::new(&engine, lhs.len()).expect("valid packed output");

    group.bench_function("pack_into_reused", |bencher| {
        bencher.iter(|| {
            packed_lhs
                .pack_from(black_box(lhs))
                .expect("benchmark length is fixed");
        });
    });
    group.bench_function("unpack_from_reused", |bencher| {
        bencher.iter(|| {
            packed_output
                .unpack_into(black_box(&mut *output))
                .expect("benchmark length is fixed");
        });
    });
    group.bench_function("mul_packed_reused", |bencher| {
        bencher.iter(|| {
            engine
                .mul_packed_into(
                    black_box(&mut packed_output),
                    black_box(&packed_lhs),
                    black_box(&packed_rhs),
                )
                .expect("benchmark plans are equal");
        });
    });
    group.bench_function("pipeline_reused_pack_mul_unpack", |bencher| {
        bencher.iter(|| {
            packed_lhs
                .pack_from(black_box(lhs))
                .expect("benchmark length is fixed");
            packed_rhs
                .pack_from(black_box(rhs))
                .expect("benchmark length is fixed");
            engine
                .mul_packed_into(&mut packed_output, &packed_lhs, &packed_rhs)
                .expect("benchmark plans are equal");
            packed_output
                .unpack_into(black_box(&mut *output))
                .expect("benchmark length is fixed");
        });
    });
    group.bench_function("pipeline_owned_allocate_pack_mul_unpack", |bencher| {
        bencher.iter(|| {
            let packed_left =
                PackedBatch::from_aos(&engine, black_box(lhs)).expect("valid packed lhs");
            let packed_right =
                PackedBatch::from_aos(&engine, black_box(rhs)).expect("valid packed rhs");
            let mut packed_result =
                PackedBatch::new(&engine, lhs.len()).expect("valid packed output");
            engine
                .mul_packed_into(&mut packed_result, &packed_left, &packed_right)
                .expect("benchmark plans are equal");
            packed_result
                .unpack_into(black_box(&mut *output))
                .expect("benchmark length is fixed");
            black_box(packed_result);
        });
    });
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
