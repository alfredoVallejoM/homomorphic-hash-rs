//! F4.6/F4.7 AVX2 bridges and persistent lanes for external prime fields.

#![cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]

use microfield::{
    __private::{
        PortableField, PortableStrategy, VerifiedPrimeCanonical8Field,
        VerifiedPrimeCanonical16Field, VerifiedPrimeCanonical32Field, VerifiedPrimeSimd8Strategy,
        VerifiedPrimeSimd16Strategy, VerifiedPrimeSimd32Strategy,
    },
    ArtifactId, BackendId, CpuCapabilities, Engine, Field, FieldId, KernelCatalog,
    PrimeKernelMetadata, PrimeReductionKind, PrimeRepresentationKind, RangeContract, ScheduleKind,
    Square, StaticField, StaticFieldSpec,
};

#[cfg(feature = "alloc")]
use microfield::{PackedBatch, PackedLayout, pack_into_storage, required_packed_bytes};

#[cfg(feature = "alloc")]
use core::mem::MaybeUninit;

const FP17_PORTABLE_METADATA: PrimeKernelMetadata = PrimeKernelMetadata::__from_generated(
    PrimeRepresentationKind::CanonicalResidue,
    PrimeReductionKind::Native,
    RangeContract::__from_generated(1, 1, 16),
    RangeContract::__from_generated(1, 1, 16),
    1,
    false,
);
const FP17_SIMD_METADATA: PrimeKernelMetadata = PrimeKernelMetadata::__from_generated(
    PrimeRepresentationKind::CanonicalResidue,
    PrimeReductionKind::Barrett,
    RangeContract::__from_generated(1, 1, 16),
    RangeContract::__from_generated(1, 1, 16),
    32,
    false,
);

const FP65521_PORTABLE_METADATA: PrimeKernelMetadata = PrimeKernelMetadata::__from_generated(
    PrimeRepresentationKind::CanonicalResidue,
    PrimeReductionKind::Native,
    RangeContract::__from_generated(1, 1, 32),
    RangeContract::__from_generated(1, 1, 32),
    1,
    false,
);
const FP65521_SIMD_METADATA: PrimeKernelMetadata = PrimeKernelMetadata::__from_generated(
    PrimeRepresentationKind::CanonicalResidue,
    PrimeReductionKind::Barrett,
    RangeContract::__from_generated(1, 1, 32),
    RangeContract::__from_generated(1, 1, 32),
    16,
    false,
);

const FP65537_PORTABLE_METADATA: PrimeKernelMetadata = PrimeKernelMetadata::__from_generated(
    PrimeRepresentationKind::CanonicalResidue,
    PrimeReductionKind::Native,
    RangeContract::__from_generated(1, 1, 64),
    RangeContract::__from_generated(1, 1, 64),
    1,
    false,
);
const FP65537_SIMD_METADATA: PrimeKernelMetadata = PrimeKernelMetadata::__from_generated(
    PrimeRepresentationKind::CanonicalResidue,
    PrimeReductionKind::Barrett,
    RangeContract::__from_generated(1, 1, 64),
    RangeContract::__from_generated(1, 1, 64),
    8,
    false,
);

static FP17_SPEC: StaticFieldSpec = StaticFieldSpec::__from_generated_prime(
    FieldId::__from_generated_hex(
        "0000000000000000000000000000000000000000000000000000000000000011",
    ),
    ArtifactId::__from_generated_hex(
        "1111111111111111111111111111111111111111111111111111111111111111",
    ),
    "external_fp17",
    "17",
    Some(17),
    1,
    1,
    b"{}",
    b"{}",
);

static FP65521_SPEC: StaticFieldSpec = StaticFieldSpec::__from_generated_prime(
    FieldId::__from_generated_hex(
        "2222222222222222222222222222222222222222222222222222222222222222",
    ),
    ArtifactId::__from_generated_hex(
        "3333333333333333333333333333333333333333333333333333333333333333",
    ),
    "external_fp65521",
    "65521",
    Some(65_521),
    1,
    2,
    b"{}",
    b"{}",
);

static FP65537_SPEC: StaticFieldSpec = StaticFieldSpec::__from_generated_prime(
    FieldId::__from_generated_hex(
        "4444444444444444444444444444444444444444444444444444444444444444",
    ),
    ArtifactId::__from_generated_hex(
        "5555555555555555555555555555555555555555555555555555555555555555",
    ),
    "external_fp65537",
    "65537",
    Some(65_537),
    1,
    4,
    b"{}",
    b"{}",
);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
struct ExternalFp17(u8);

impl Field for ExternalFp17 {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);

    fn add(self, rhs: Self) -> Self {
        Self(((u16::from(self.0) + u16::from(rhs.0)) % 17) as u8)
    }

    fn sub(self, rhs: Self) -> Self {
        Self(((u16::from(self.0) + 17 - u16::from(rhs.0)) % 17) as u8)
    }

    fn neg(self) -> Self {
        Self(if self.0 == 0 { 0 } else { 17 - self.0 })
    }

    fn mul(self, rhs: Self) -> Self {
        Self(((u16::from(self.0) * u16::from(rhs.0)) % 17) as u8)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl Square for ExternalFp17 {
    fn square(self) -> Self {
        self.mul(self)
    }
}

impl StaticField for ExternalFp17 {
    fn spec() -> &'static StaticFieldSpec {
        &FP17_SPEC
    }
}

impl VerifiedPrimeCanonical8Field for ExternalFp17 {
    const __MODULUS: u16 = 17;
    const __BARRETT_RECIPROCAL: u16 = 3_855;

    fn __into_canonical_u8(self) -> u8 {
        self.0
    }

    fn __from_reduced_canonical_u8(value: u8) -> Self {
        assert!(value < 17);
        Self(value)
    }
}

static FP17_PORTABLE: PortableStrategy<ExternalFp17> =
    PortableStrategy::new_prime(FP17_PORTABLE_METADATA);
static FP17_SIMD: VerifiedPrimeSimd8Strategy<ExternalFp17> =
    VerifiedPrimeSimd8Strategy::new(FP17_SIMD_METADATA);

impl PortableField for ExternalFp17 {
    fn __portable_strategy() -> &'static PortableStrategy<Self> {
        &FP17_PORTABLE
    }

    fn __kernel_catalog() -> KernelCatalog<Self> {
        FP17_SIMD.__kernel_catalog(&FP17_PORTABLE)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
struct ExternalFp65521(u16);

impl Field for ExternalFp65521 {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);

    fn add(self, rhs: Self) -> Self {
        Self(((u32::from(self.0) + u32::from(rhs.0)) % 65_521) as u16)
    }

    fn sub(self, rhs: Self) -> Self {
        Self(((u32::from(self.0) + 65_521 - u32::from(rhs.0)) % 65_521) as u16)
    }

    fn neg(self) -> Self {
        Self(if self.0 == 0 { 0 } else { 65_521 - self.0 })
    }

    fn mul(self, rhs: Self) -> Self {
        Self(((u64::from(self.0) * u64::from(rhs.0)) % 65_521) as u16)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl Square for ExternalFp65521 {
    fn square(self) -> Self {
        self.mul(self)
    }
}

impl StaticField for ExternalFp65521 {
    fn spec() -> &'static StaticFieldSpec {
        &FP65521_SPEC
    }
}

impl VerifiedPrimeCanonical16Field for ExternalFp65521 {
    const __MODULUS: u32 = 65_521;
    const __BARRETT_RECIPROCAL: u32 = 65_551;

    fn __into_canonical_u16(self) -> u16 {
        self.0
    }

    fn __from_reduced_canonical_u16(value: u16) -> Self {
        assert!(u32::from(value) < 65_521);
        Self(value)
    }
}

static FP65521_PORTABLE: PortableStrategy<ExternalFp65521> =
    PortableStrategy::new_prime(FP65521_PORTABLE_METADATA);
static FP65521_SIMD: VerifiedPrimeSimd16Strategy<ExternalFp65521> =
    VerifiedPrimeSimd16Strategy::new(FP65521_SIMD_METADATA);

impl PortableField for ExternalFp65521 {
    fn __portable_strategy() -> &'static PortableStrategy<Self> {
        &FP65521_PORTABLE
    }

    fn __kernel_catalog() -> KernelCatalog<Self> {
        FP65521_SIMD.__kernel_catalog(&FP65521_PORTABLE)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
struct ExternalFp65537(u32);

impl Field for ExternalFp65537 {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);

    fn add(self, rhs: Self) -> Self {
        Self(((u64::from(self.0) + u64::from(rhs.0)) % 65_537) as u32)
    }

    fn sub(self, rhs: Self) -> Self {
        Self(((u64::from(self.0) + 65_537 - u64::from(rhs.0)) % 65_537) as u32)
    }

    fn neg(self) -> Self {
        Self(if self.0 == 0 { 0 } else { 65_537 - self.0 })
    }

    fn mul(self, rhs: Self) -> Self {
        Self(((u64::from(self.0) * u64::from(rhs.0)) % 65_537) as u32)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl Square for ExternalFp65537 {
    fn square(self) -> Self {
        self.mul(self)
    }
}

impl StaticField for ExternalFp65537 {
    fn spec() -> &'static StaticFieldSpec {
        &FP65537_SPEC
    }
}

impl VerifiedPrimeCanonical32Field for ExternalFp65537 {
    const __MODULUS: u64 = 65_537;
    const __BARRETT_RECIPROCAL: u64 = u64::MAX / 65_537;

    fn __into_canonical_u32(self) -> u32 {
        self.0
    }

    fn __from_reduced_canonical_u32(value: u32) -> Self {
        assert!(u64::from(value) < Self::__MODULUS);
        Self(value)
    }
}

static FP65537_PORTABLE: PortableStrategy<ExternalFp65537> =
    PortableStrategy::new_prime(FP65537_PORTABLE_METADATA);
static FP65537_SIMD: VerifiedPrimeSimd32Strategy<ExternalFp65537> =
    VerifiedPrimeSimd32Strategy::new(FP65537_SIMD_METADATA);

impl PortableField for ExternalFp65537 {
    fn __portable_strategy() -> &'static PortableStrategy<Self> {
        &FP65537_PORTABLE
    }

    fn __kernel_catalog() -> KernelCatalog<Self> {
        FP65537_SIMD.__kernel_catalog(&FP65537_PORTABLE)
    }
}

#[test]
fn byte_simd_is_exhaustive_and_covers_every_batch_route() {
    let Some((portable, avx2)) = engines::<ExternalFp17>() else {
        return;
    };
    let mut lhs = Vec::with_capacity(17 * 17);
    let mut rhs = Vec::with_capacity(17 * 17);
    for left in 0..17 {
        for right in 0..17 {
            lhs.push(ExternalFp17(left));
            rhs.push(ExternalFp17(right));
        }
    }
    compare_every_route(portable, avx2, &lhs, &rhs);
}

#[test]
fn u16_simd_covers_boundaries_random_inputs_and_every_tail() {
    let Some((portable, avx2)) = engines::<ExternalFp65521>() else {
        return;
    };
    for &len in &[0, 1, 2, 15, 16, 17, 31, 32, 33, 63, 64, 65, 255, 1024] {
        let lhs = fp65521_values(len, 0x243f_6a88_85a3_08d3);
        let rhs = fp65521_values(len, 0x1319_8a2e_0370_7344);
        compare_every_route(portable, avx2, &lhs, &rhs);
    }

    let adversarial = [
        ExternalFp65521(0),
        ExternalFp65521(1),
        ExternalFp65521(2),
        ExternalFp65521(32_760),
        ExternalFp65521(65_519),
        ExternalFp65521(65_520),
    ];
    let lhs: Vec<_> = (0..257)
        .map(|index| adversarial[index % adversarial.len()])
        .collect();
    let rhs: Vec<_> = (0..257)
        .map(|index| adversarial[(index * 5 + 1) % adversarial.len()])
        .collect();
    compare_every_route(portable, avx2, &lhs, &rhs);
}

#[test]
fn u32_candidate_covers_boundaries_tails_and_one_hundred_thousand_inputs() {
    let Some((portable, avx2)) = engines::<ExternalFp65537>() else {
        return;
    };
    for &len in &[0, 1, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 257] {
        let lhs = fp65537_values(len, 0x243f_6a88_85a3_08d3);
        let rhs = fp65537_values(len, 0x1319_8a2e_0370_7344);
        compare_every_route(portable, avx2, &lhs, &rhs);
    }
    let lhs = fp65537_values(100_003, 0xa409_3822_299f_31d0);
    let rhs = fp65537_values(100_003, 0x082e_fa98_ec4e_6c89);
    compare_every_route(portable, avx2, &lhs, &rhs);
}

#[cfg(feature = "alloc")]
#[test]
fn persistent_lanes_execute_long_chains_without_repacking() {
    let Some((_, fp17)) = engines::<ExternalFp17>() else {
        return;
    };
    let Some((_, fp65521)) = engines::<ExternalFp65521>() else {
        return;
    };
    let Some((_, fp65537)) = engines::<ExternalFp65537>() else {
        return;
    };

    for &len in &[0, 1, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 257, 1024] {
        assert_persistent(
            fp17,
            &fp17_values(len, 0x243f_6a88_85a3_08d3),
            &fp17_values(len, 0x1319_8a2e_0370_7344),
            PackedLayout::CanonicalU8,
            1,
        );
        assert_persistent(
            fp65521,
            &fp65521_values(len, 0x243f_6a88_85a3_08d3),
            &fp65521_values(len, 0x1319_8a2e_0370_7344),
            PackedLayout::CanonicalU16,
            2,
        );
        assert_persistent(
            fp65537,
            &fp65537_values(len, 0x243f_6a88_85a3_08d3),
            &fp65537_values(len, 0x1319_8a2e_0370_7344),
            PackedLayout::CanonicalU32,
            4,
        );
    }
}

#[test]
fn external_simd_profiles_are_fixed_but_never_implicitly_promoted() {
    let capabilities = CpuCapabilities::detect();
    assert_selection::<ExternalFp17>(capabilities, 32, 32);
    assert_selection::<ExternalFp65521>(capabilities, 16, 16);
    assert_selection::<ExternalFp65537>(capabilities, 8, 8);
}

#[cfg(feature = "count-allocations")]
#[test]
fn external_simd_tiles_allocate_zero_times() {
    use allocation_counter::measure;

    let Some((_, avx2_u8)) = engines::<ExternalFp17>() else {
        return;
    };
    let Some((_, avx2_u16)) = engines::<ExternalFp65521>() else {
        return;
    };
    let lhs_u8: Vec<_> = (0_usize..257)
        .map(|index| ExternalFp17(u8::try_from(index % 17).unwrap()))
        .collect();
    let rhs_u8: Vec<_> = (0_usize..257)
        .map(|index| ExternalFp17(u8::try_from((index * 7 + 3) % 17).unwrap()))
        .collect();
    let lhs_u16 = fp65521_values(257, 0x243f_6a88_85a3_08d3);
    let rhs_u16 = fp65521_values(257, 0x1319_8a2e_0370_7344);
    let mut out_u8 = vec![ExternalFp17::ZERO; 257];
    let mut out_u16 = vec![ExternalFp65521::ZERO; 257];

    let allocations = measure(|| {
        avx2_u8.mul_into(&mut out_u8, &lhs_u8, &rhs_u8).unwrap();
        avx2_u8.add_into(&mut out_u8, &lhs_u8, &rhs_u8).unwrap();
        avx2_u8.square_assign(&mut out_u8);
        avx2_u16.mul_into(&mut out_u16, &lhs_u16, &rhs_u16).unwrap();
        avx2_u16.add_into(&mut out_u16, &lhs_u16, &rhs_u16).unwrap();
        avx2_u16.square_assign(&mut out_u16);
    });
    assert_eq!(allocations.count_total, 0);
    assert_eq!(allocations.bytes_total, 0);
    assert_eq!(allocations.count_current, 0);
    assert_eq!(allocations.bytes_current, 0);
}

fn assert_selection<F: PortableField>(
    capabilities: CpuCapabilities,
    lanes: u16,
    preferred_multiple: usize,
) {
    let automatic = Engine::<F>::builder()
        .capabilities(capabilities)
        .expected_batch(16_384)
        .build()
        .unwrap();
    assert_eq!(automatic.backend_id(), BackendId::Portable);

    let forced = Engine::<F>::builder()
        .capabilities(capabilities)
        .force_backend(BackendId::X86PrimeAvx2)
        .build();
    if capabilities.has_x86_avx2() {
        let forced = forced.unwrap();
        assert_eq!(forced.backend_id(), BackendId::X86PrimeAvx2);
        assert_eq!(forced.metadata().schedule(), ScheduleKind::Fixed);
        assert!(!forced.metadata().automatic_selection());
        assert_eq!(forced.metadata().preferred_multiple(), preferred_multiple);
        assert_eq!(forced.metadata().prime().unwrap().lanes(), lanes);
    } else {
        assert!(forced.is_err());
    }
}

fn engines<F: PortableField>() -> Option<(Engine<F>, Engine<F>)> {
    let capabilities = CpuCapabilities::detect();
    if !capabilities.has_x86_avx2() {
        return None;
    }
    let portable = Engine::<F>::portable();
    let avx2 = Engine::<F>::builder()
        .capabilities(capabilities)
        .force_backend(BackendId::X86PrimeAvx2)
        .build()
        .unwrap();
    Some((portable, avx2))
}

fn compare_every_route<F: PortableField + core::fmt::Debug>(
    portable: Engine<F>,
    avx2: Engine<F>,
    lhs: &[F],
    rhs: &[F],
) {
    let mut expected = vec![F::ZERO; lhs.len()];
    let mut actual = expected.clone();

    portable.mul_into(&mut expected, lhs, rhs).unwrap();
    avx2.mul_into(&mut actual, lhs, rhs).unwrap();
    assert_eq!(actual, expected);

    portable.add_into(&mut expected, lhs, rhs).unwrap();
    avx2.add_into(&mut actual, lhs, rhs).unwrap();
    assert_eq!(actual, expected);

    portable.square_into(&mut expected, lhs).unwrap();
    avx2.square_into(&mut actual, lhs).unwrap();
    assert_eq!(actual, expected);

    let mut expected_assign = lhs.to_vec();
    let mut actual_assign = lhs.to_vec();
    portable.mul_assign(&mut expected_assign, rhs).unwrap();
    avx2.mul_assign(&mut actual_assign, rhs).unwrap();
    assert_eq!(actual_assign, expected_assign);

    expected_assign.copy_from_slice(lhs);
    actual_assign.copy_from_slice(lhs);
    portable.square_assign(&mut expected_assign);
    avx2.square_assign(&mut actual_assign);
    assert_eq!(actual_assign, expected_assign);
}

fn fp65521_values(len: usize, mut state: u64) -> Vec<ExternalFp65521> {
    (0..len)
        .map(|index| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            ExternalFp65521(((state ^ index as u64) % 65_521) as u16)
        })
        .collect()
}

fn fp17_values(len: usize, mut state: u64) -> Vec<ExternalFp17> {
    (0..len)
        .map(|index| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            ExternalFp17(((state ^ index as u64) % 17) as u8)
        })
        .collect()
}

fn fp65537_values(len: usize, mut state: u64) -> Vec<ExternalFp65537> {
    (0..len)
        .map(|index| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            ExternalFp65537(((state ^ index as u64) % 65_537) as u32)
        })
        .collect()
}

#[cfg(feature = "alloc")]
fn assert_persistent<F>(
    engine: Engine<F>,
    lhs_values: &[F],
    rhs_values: &[F],
    layout: PackedLayout,
    physical_element_size: usize,
) where
    F: PortableField + StaticField + core::fmt::Debug,
{
    let lhs = PackedBatch::from_aos(&engine, lhs_values).unwrap();
    let rhs = PackedBatch::from_aos(&engine, rhs_values).unwrap();
    let mut out = PackedBatch::new(&engine, lhs_values.len()).unwrap();
    assert_eq!(lhs.plan().layout(), layout);
    assert_eq!(lhs.plan().physical_element_size(), physical_element_size);
    assert_eq!(
        lhs.plan().data_bytes(),
        lhs.plan().padded_len() * physical_element_size
    );

    engine.add_packed_into(&mut out, &lhs, &rhs).unwrap();
    let mut actual = vec![F::ZERO; lhs_values.len()];
    out.unpack_into(&mut actual).unwrap();
    assert_eq!(
        actual,
        lhs_values
            .iter()
            .zip(rhs_values)
            .map(|(left, right)| left.add(*right))
            .collect::<Vec<_>>()
    );

    engine.mul_packed_into(&mut out, &lhs, &rhs).unwrap();
    let expected_product = lhs_values
        .iter()
        .zip(rhs_values)
        .map(|(left, right)| left.mul(*right))
        .collect::<Vec<_>>();
    out.unpack_into(&mut actual).unwrap();
    assert_eq!(actual, expected_product);

    let mut chained = PackedBatch::from_aos(&engine, lhs_values).unwrap();
    let mut expected_chain = lhs_values.to_vec();
    for round in 0..64 {
        if round % 2 == 0 {
            engine.mul_packed_assign(&mut chained, &rhs).unwrap();
            for (value, right) in expected_chain.iter_mut().zip(rhs_values) {
                *value = value.mul(*right);
            }
        } else {
            engine.square_packed_assign(&mut chained).unwrap();
            for value in &mut expected_chain {
                *value = value.square();
            }
        }
    }
    chained.unpack_into(&mut actual).unwrap();
    assert_eq!(actual, expected_chain);

    let plan = engine.packing_plan(lhs_values.len()).unwrap();
    let capacity = required_packed_bytes(&plan).unwrap();
    let mut storage = vec![MaybeUninit::uninit(); capacity];
    let view = pack_into_storage(&engine, &mut storage, lhs_values).unwrap();
    view.unpack_into(&mut actual).unwrap();
    assert_eq!(actual, lhs_values);
}
