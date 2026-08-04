//! Differential performance probes for generated portable arithmetic.

// `criterion_group!` emits one public harness function that cannot carry docs.
#![allow(missing_docs)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use microfield::{__private, Field, Square};

const MODULUS_128: &[usize] = &[128, 7, 2, 1, 0];
const MODULUS_233: &[usize] = &[233, 74, 0];

fn portable_codegen_optimizer(criterion: &mut Criterion) {
    let lhs_128 = [0xa5a5_5a5a_f0f0_0f0f, 0x0123_4567_89ab_cdef];
    let rhs_128 = [0x3c3c_c3c3_55aa_aa55, 0xfedc_ba98_7654_3210];
    let mut aligned = criterion.benchmark_group("codegen_optimizer/gf2_128_low_tail");
    aligned.bench_function("multiply_reference_v1", |bencher| {
        bencher.iter(|| {
            __private::multiply::<2, 4>(black_box(lhs_128), black_box(rhs_128), 128, MODULUS_128)
        });
    });
    aligned.bench_function("multiply_optimized_v2", |bencher| {
        bencher.iter(|| {
            __private::multiply_low_tail::<2, 4, 0x87>(black_box(lhs_128), black_box(rhs_128))
        });
    });
    aligned.bench_function("square_reference_v1", |bencher| {
        bencher.iter(|| __private::square::<2, 4>(black_box(lhs_128), 128, MODULUS_128));
    });
    aligned.bench_function("square_optimized_v2", |bencher| {
        bencher.iter(|| __private::square_low_tail::<2, 4, 0x87>(black_box(lhs_128)));
    });
    aligned.finish();

    let lhs_233 = Bench233([
        0xa5a5_5a5a_f0f0_0f0f,
        0x0123_4567_89ab_cdef,
        0x55aa_aa55_33cc_cc33,
        0x0000_01aa_7654_3210,
    ]);
    let rhs_233 = Bench233([
        0x3c3c_c3c3_55aa_aa55,
        0xfedc_ba98_7654_3210,
        0x0f0f_f0f0_9696_6969,
        0x0000_00f1_1357_9bdf,
    ]);
    let mut unaligned = criterion.benchmark_group("codegen_optimizer/gf2_233_sparse");
    unaligned.bench_function("multiply_reference_v1", |bencher| {
        bencher.iter(|| {
            __private::multiply::<4, 8>(
                black_box(lhs_233.0),
                black_box(rhs_233.0),
                233,
                MODULUS_233,
            )
        });
    });
    unaligned.bench_function("multiply_optimized_v2", |bencher| {
        bencher.iter(|| {
            __private::multiply_sparse::<4, 8>(
                black_box(lhs_233.0),
                black_box(rhs_233.0),
                233,
                MODULUS_233,
            )
        });
    });
    unaligned.bench_function("square_reference_v1", |bencher| {
        bencher.iter(|| __private::square::<4, 8>(black_box(lhs_233.0), 233, MODULUS_233));
    });
    unaligned.bench_function("square_optimized_v2", |bencher| {
        bencher.iter(|| __private::square_sparse::<4, 8>(black_box(lhs_233.0), 233, MODULUS_233));
    });
    unaligned.bench_function("invert_reference_v1", |bencher| {
        bencher.iter(|| __private::invert::<Bench233, 233>(black_box(lhs_233)));
    });
    unaligned.bench_function("invert_itoh_tsujii_v2", |bencher| {
        bencher.iter(|| __private::invert_itoh_tsujii::<Bench233, 233>(black_box(lhs_233)));
    });
    unaligned.finish();
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Bench233([u64; 4]);

impl Field for Bench233 {
    const ZERO: Self = Self([0; 4]);
    const ONE: Self = Self([1, 0, 0, 0]);

    fn add(self, rhs: Self) -> Self {
        Self(__private::add(self.0, rhs.0))
    }

    fn sub(self, rhs: Self) -> Self {
        self.add(rhs)
    }

    fn neg(self) -> Self {
        self
    }

    fn mul(self, rhs: Self) -> Self {
        Self(__private::multiply_sparse::<4, 8>(
            self.0,
            rhs.0,
            233,
            MODULUS_233,
        ))
    }

    fn is_zero(&self) -> bool {
        __private::is_zero(&self.0)
    }
}

impl Square for Bench233 {
    fn square(self) -> Self {
        Self(__private::square_sparse::<4, 8>(self.0, 233, MODULUS_233))
    }
}

criterion_group!(benches, portable_codegen_optimizer);
criterion_main!(benches);
