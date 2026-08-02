//! F4.7 benchmark separating the generic direct bridge from persistent lanes.

#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use microfield::{
    __private::{
        PortableField, PortableStrategy, VerifiedPrimeCanonical16Field, VerifiedPrimeSimd16Strategy,
    },
    ArtifactId, BackendId, CpuCapabilities, Engine, Field, FieldId, KernelCatalog, PackedBatch,
    PrimeKernelMetadata, PrimeReductionKind, PrimeRepresentationKind, RangeContract, Square,
    StaticField, StaticFieldSpec,
};

const PORTABLE_METADATA: PrimeKernelMetadata = PrimeKernelMetadata::__from_generated(
    PrimeRepresentationKind::CanonicalResidue,
    PrimeReductionKind::Native,
    RangeContract::__from_generated(1, 1, 32),
    RangeContract::__from_generated(1, 1, 32),
    1,
    false,
);
const SIMD_METADATA: PrimeKernelMetadata = PrimeKernelMetadata::__from_generated(
    PrimeRepresentationKind::CanonicalResidue,
    PrimeReductionKind::Barrett,
    RangeContract::__from_generated(1, 1, 32),
    RangeContract::__from_generated(1, 1, 32),
    16,
    false,
);
static SPEC: StaticFieldSpec = StaticFieldSpec::__from_generated_prime(
    FieldId::__from_generated_hex(
        "7777777777777777777777777777777777777777777777777777777777777777",
    ),
    ArtifactId::__from_generated_hex(
        "8888888888888888888888888888888888888888888888888888888888888888",
    ),
    "bench_fp65521",
    "65521",
    Some(65_521),
    1,
    2,
    b"{}",
    b"{}",
);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
struct BenchFp65521(u16);

impl Field for BenchFp65521 {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(((u32::from(self.0) + u32::from(rhs.0)) % 65_521) as u16)
    }

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(((u32::from(self.0) + 65_521 - u32::from(rhs.0)) % 65_521) as u16)
    }

    #[inline]
    fn neg(self) -> Self {
        Self(if self.0 == 0 { 0 } else { 65_521 - self.0 })
    }

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self(((u64::from(self.0) * u64::from(rhs.0)) % 65_521) as u16)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl Square for BenchFp65521 {
    #[inline]
    fn square(self) -> Self {
        self.mul(self)
    }
}

impl StaticField for BenchFp65521 {
    fn spec() -> &'static StaticFieldSpec {
        &SPEC
    }
}

impl VerifiedPrimeCanonical16Field for BenchFp65521 {
    const __MODULUS: u32 = 65_521;
    const __BARRETT_RECIPROCAL: u32 = 65_551;

    #[inline]
    fn __into_canonical_u16(self) -> u16 {
        self.0
    }

    #[inline]
    fn __from_reduced_canonical_u16(value: u16) -> Self {
        debug_assert!(u32::from(value) < Self::__MODULUS);
        Self(value)
    }
}

static PORTABLE: PortableStrategy<BenchFp65521> = PortableStrategy::new_prime(PORTABLE_METADATA);
static SIMD: VerifiedPrimeSimd16Strategy<BenchFp65521> =
    VerifiedPrimeSimd16Strategy::new(SIMD_METADATA);

impl PortableField for BenchFp65521 {
    fn __portable_strategy() -> &'static PortableStrategy<Self> {
        &PORTABLE
    }

    fn __kernel_catalog() -> KernelCatalog<Self> {
        SIMD.__kernel_catalog(&PORTABLE)
    }
}

fn persistent_prime_bridge(criterion: &mut Criterion) {
    let capabilities = CpuCapabilities::detect();
    if !capabilities.has_x86_avx2() {
        return;
    }
    let engine = Engine::<BenchFp65521>::builder()
        .capabilities(capabilities)
        .force_backend(BackendId::X86PrimeAvx2)
        .build()
        .expect("detected AVX2");
    let mut group = criterion.benchmark_group("phase47/persistent/fp65521");
    for len in [16, 64, 256, 1024, 4096, 16_384] {
        group.throughput(Throughput::Elements(len as u64));
        let lhs = values(len, 0x243f_6a88_85a3_08d3);
        let rhs = values(len, 0x1319_8a2e_0370_7344);
        let mut direct_out = vec![BenchFp65521::ZERO; len];
        let packed_lhs = PackedBatch::from_aos(&engine, &lhs).unwrap();
        let packed_rhs = PackedBatch::from_aos(&engine, &rhs).unwrap();
        let mut packed_out = PackedBatch::new(&engine, len).unwrap();
        let mut packed_chain = PackedBatch::from_aos(&engine, &lhs).unwrap();

        group.bench_with_input(BenchmarkId::new("direct_mul", len), &len, |b, _| {
            b.iter(|| {
                engine.mul_into(black_box(&mut direct_out), black_box(&lhs), black_box(&rhs))
            });
        });
        group.bench_with_input(BenchmarkId::new("packed_mul", len), &len, |b, _| {
            b.iter(|| {
                engine.mul_packed_into(
                    black_box(&mut packed_out),
                    black_box(&packed_lhs),
                    black_box(&packed_rhs),
                )
            });
        });
        group.bench_with_input(BenchmarkId::new("pack_reused", len), &len, |b, _| {
            b.iter(|| packed_out.pack_from(black_box(&lhs)));
        });
        group.bench_with_input(BenchmarkId::new("unpack_reused", len), &len, |b, _| {
            b.iter(|| packed_lhs.unpack_into(black_box(&mut direct_out)));
        });
        group.bench_with_input(BenchmarkId::new("packed_chain_8", len), &len, |b, _| {
            b.iter(|| {
                for _ in 0..4 {
                    engine
                        .mul_packed_assign(black_box(&mut packed_chain), black_box(&packed_rhs))
                        .unwrap();
                    engine
                        .square_packed_assign(black_box(&mut packed_chain))
                        .unwrap();
                }
            });
        });
    }
    group.finish();
}

fn values(len: usize, mut state: u64) -> Vec<BenchFp65521> {
    (0..len)
        .map(|index| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            BenchFp65521(((state ^ index as u64) % 65_521) as u16)
        })
        .collect()
}

criterion_group!(benches, persistent_prime_bridge);
criterion_main!(benches);
