//! Consumer-side proof that generated radix-64 prime types can reuse BMI2.

#![cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]

use microfield::{
    BackendId, CpuCapabilities, Engine, ExecutionPolicy, Field, PrimeKernelMetadata,
    PrimeReductionKind, PrimeRepresentationKind, RangeContract, ScheduleKind, Square,
};

const MODULUS: u64 = 17;
const PRIME_METADATA: PrimeKernelMetadata = PrimeKernelMetadata::__from_generated(
    PrimeRepresentationKind::Montgomery {
        radix_bits: 64,
        limbs: 1,
    },
    PrimeReductionKind::Montgomery,
    RangeContract::__from_generated(1, 1, 128),
    RangeContract::__from_generated(1, 1, 128),
    1,
    false,
);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
struct ExternalFp17(u64);

static PORTABLE: microfield::__private::PortableStrategy<ExternalFp17> =
    microfield::__private::PortableStrategy::new_prime(PRIME_METADATA);
static BMI2: microfield::__private::VerifiedPrimeIsaStrategy<ExternalFp17, 1, 2> =
    microfield::__private::VerifiedPrimeIsaStrategy::new(PRIME_METADATA);

impl Field for ExternalFp17 {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);

    fn add(self, rhs: Self) -> Self {
        Self((self.0 + rhs.0) % MODULUS)
    }

    fn sub(self, rhs: Self) -> Self {
        Self((self.0 + MODULUS - rhs.0) % MODULUS)
    }

    fn neg(self) -> Self {
        Self((MODULUS - self.0) % MODULUS)
    }

    fn mul(self, rhs: Self) -> Self {
        Self((self.0 * rhs.0) % MODULUS)
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

impl microfield::__private::PortableField for ExternalFp17 {
    fn __portable_strategy() -> &'static microfield::__private::PortableStrategy<Self> {
        &PORTABLE
    }

    fn __kernel_catalog() -> microfield::KernelCatalog<Self> {
        BMI2.__kernel_catalog(&PORTABLE)
    }
}

impl microfield::__private::VerifiedPrimeMontgomery64Field<1, 2> for ExternalFp17 {
    const __MODULUS: [u64; 1] = [MODULUS];
    const __NEG_INV: u64 = 0x0f0f_0f0f_0f0f_0f0f;

    fn __into_montgomery_limbs(self) -> [u64; 1] {
        [self.0]
    }

    fn __from_reduced_montgomery_limbs(limbs: [u64; 1]) -> Self {
        Self(limbs[0])
    }
}

#[test]
fn external_radix64_type_gets_a_safe_explicit_bmi2_candidate() {
    let capabilities = CpuCapabilities::detect();
    if !capabilities.has_x86_bmi2() {
        return;
    }
    let portable = Engine::<ExternalFp17>::portable();
    let bmi2 = Engine::<ExternalFp17>::builder()
        .capabilities(capabilities)
        .force_backend(BackendId::X86PrimeBmi2)
        .build()
        .unwrap();
    assert_eq!(bmi2.metadata().schedule(), ScheduleKind::Fixed);
    assert!(!bmi2.metadata().automatic_selection());

    let mut lhs = Vec::with_capacity(17 * 17);
    let mut rhs = Vec::with_capacity(17 * 17);
    for left in 0..17 {
        for right in 0..17 {
            lhs.push(ExternalFp17(left));
            rhs.push(ExternalFp17(right));
        }
    }
    let mut expected = vec![ExternalFp17::ZERO; lhs.len()];
    let mut actual = expected.clone();
    portable.mul_into(&mut expected, &lhs, &rhs).unwrap();
    bmi2.mul_into(&mut actual, &lhs, &rhs).unwrap();
    assert_eq!(actual, expected);

    portable.add_into(&mut expected, &lhs, &rhs).unwrap();
    bmi2.add_into(&mut actual, &lhs, &rhs).unwrap();
    assert_eq!(actual, expected);

    let automatic = Engine::<ExternalFp17>::builder()
        .capabilities(capabilities)
        .expected_batch(16_384)
        .build()
        .unwrap();
    assert_eq!(automatic.backend_id(), BackendId::Portable);

    let fixed = Engine::<ExternalFp17>::builder()
        .capabilities(capabilities)
        .policy(ExecutionPolicy::FixedSchedule)
        .force_backend(BackendId::X86PrimeBmi2)
        .build()
        .expect("Microfield owns the fixed carry and correction schedule");
    assert_eq!(fixed.backend_id(), BackendId::X86PrimeBmi2);
}
