//! Maintained Goldilocks prime field.

// Branches establish that every narrowed accumulator is below the u64 modulus.
#![allow(clippy::cast_possible_truncation)]

use crate::prime::{PrimeFieldSpec, PrimeWideProduct};
use crate::{
    ArtifactId, BarrettPlan, CanonicalEncoding, Characteristic, DecodeError, Field, FieldId,
    Invert, PrimeExponentiationPlan, PrimeField, PrimeReductionPlan, PrimeRepresentationKind,
    RangeContract, SignedPowerOfTwo, SolinasPlan, Square, StaticFieldSpec,
};

use super::prime_field::impl_prime_field_common;

const MODULUS: u64 = 0xffff_ffff_0000_0001;
const CHARACTERISTIC: Characteristic =
    Characteristic::__from_generated("18446744069414584321", Some(MODULUS));
const BARRETT_MODULUS: [u64; 1] = [MODULUS];
const BARRETT_RECIPROCAL: [u64; 2] = [0x0000_0000_ffff_ffff, 1];
const SOLINAS_TERMS: [SignedPowerOfTwo; 3] = [
    SignedPowerOfTwo {
        positive: true,
        exponent: 64,
    },
    SignedPowerOfTwo {
        positive: false,
        exponent: 32,
    },
    SignedPowerOfTwo {
        positive: true,
        exponent: 0,
    },
];

static SPEC: StaticFieldSpec = StaticFieldSpec {
    field_id: FieldId::__from_generated_hex(
        "db27c832ee2b9e87ae66e00657a20cf705132730f5ac43e3f7031f9bb1e272ac",
    ),
    artifact_id: ArtifactId::__from_generated_hex(
        "5de756cce03a1fe635939544390b2eb169737acd8663c76eb2e929fd3bb0dc7a",
    ),
    name: "fp_goldilocks64_v1",
    characteristic: CHARACTERISTIC,
    degree: 1,
    canonical_bytes: 8,
    descriptor_json: include_bytes!("../../artifacts/fp_goldilocks64_v1/descriptor.json"),
    certificate_json: include_bytes!("../../artifacts/fp_goldilocks64_v1/certificate.json"),
};

/// The maintained prime field with modulus `2^64 - 2^32 + 1`.
///
/// The private representation is always a reduced canonical residue. Canonical
/// bytes are an eight-byte little-endian integer and never reduce implicitly.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct FpGoldilocks64V1(u64);

impl FpGoldilocks64V1 {
    /// Prime modulus.
    pub const MODULUS: u64 = MODULUS;

    /// Returns the private representation family for diagnostics.
    #[must_use]
    pub const fn representation_kind() -> PrimeRepresentationKind {
        <Self as PrimeFieldSpec>::REPRESENTATION
    }

    /// Returns the selected Barrett reduction plan.
    #[must_use]
    pub const fn reduction_plan() -> PrimeReductionPlan {
        <Self as PrimeFieldSpec>::REDUCTION
    }

    /// Returns the generated and symbolically checked `p - 2` plan.
    #[must_use]
    pub const fn inversion_plan() -> PrimeExponentiationPlan<1> {
        PrimeExponentiationPlan::__from_generated([MODULUS - 2], 64)
    }

    /// Returns the verified Solinas plan selected by the field artifact.
    #[must_use]
    pub const fn solinas_plan() -> SolinasPlan {
        SolinasPlan::__from_generated(
            64,
            &SOLINAS_TERMS,
            1,
            RangeContract::__from_generated(1, 1, 128),
        )
    }

    /// Returns the independently benchmarked Barrett comparison plan.
    #[must_use]
    pub const fn barrett_plan() -> BarrettPlan {
        BarrettPlan::__from_generated(
            64,
            1,
            &BARRETT_MODULUS,
            &BARRETT_RECIPROCAL,
            64,
            2,
            RangeContract::__from_generated(1, 1, 128),
        )
    }

    /// Reduces one machine integer explicitly.
    #[must_use]
    pub const fn from_u64_mod(value: u64) -> Self {
        Self(if value >= MODULUS {
            value - MODULUS
        } else {
            value
        })
    }

    /// Reduces a full product with the alternative remainder baseline.
    #[doc(hidden)]
    #[must_use]
    pub fn __barrett_reduce_wide(wide: u128) -> Self {
        Self(crate::prime::barrett_reduce_goldilocks(wide))
    }
}

impl Field for FpGoldilocks64V1 {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);

    #[inline]
    fn add(self, rhs: Self) -> Self {
        let sum = u128::from(self.0) + u128::from(rhs.0);
        Self(if sum >= u128::from(MODULUS) {
            (sum - u128::from(MODULUS)) as u64
        } else {
            sum as u64
        })
    }

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        if self.0 >= rhs.0 {
            Self(self.0 - rhs.0)
        } else {
            Self(MODULUS - (rhs.0 - self.0))
        }
    }

    #[inline]
    fn neg(self) -> Self {
        if self.0 == 0 {
            self
        } else {
            Self(MODULUS - self.0)
        }
    }

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self(crate::prime::barrett_reduce_goldilocks(
            <Self as PrimeWideProduct>::mul_wide(self, rhs),
        ))
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl PrimeWideProduct for FpGoldilocks64V1 {
    type Wide = u128;

    #[inline]
    fn mul_wide(self, rhs: Self) -> Self::Wide {
        u128::from(self.0) * u128::from(rhs.0)
    }
}

impl PrimeFieldSpec for FpGoldilocks64V1 {
    const LIMBS: usize = 1;
    const MODULUS: &'static [u64] = &BARRETT_MODULUS;
    const REPRESENTATION: PrimeRepresentationKind = PrimeRepresentationKind::CanonicalResidue;
    const REDUCTION: PrimeReductionPlan =
        PrimeReductionPlan::Barrett(FpGoldilocks64V1::barrett_plan());
}

impl Square for FpGoldilocks64V1 {
    #[inline]
    fn square(self) -> Self {
        self.mul(self)
    }
}

impl Invert for FpGoldilocks64V1 {
    fn invert(self) -> Option<Self> {
        (!self.is_zero()).then(|| Self::inversion_plan().evaluate(self))
    }
}

impl CanonicalEncoding for FpGoldilocks64V1 {
    type Repr = [u8; 8];

    fn from_canonical(repr: &Self::Repr) -> Result<Self, DecodeError> {
        let value = u64::from_le_bytes(*repr);
        if value >= MODULUS {
            Err(DecodeError::NonCanonicalValue)
        } else {
            Ok(Self(value))
        }
    }

    fn from_canonical_slice(bytes: &[u8]) -> Result<Self, DecodeError> {
        let repr: [u8; 8] = bytes.try_into().map_err(|_| DecodeError::LengthMismatch {
            expected: 8,
            actual: bytes.len(),
        })?;
        Self::from_canonical(&repr)
    }

    fn to_canonical(self) -> Self::Repr {
        self.0.to_le_bytes()
    }
}

impl PrimeField for FpGoldilocks64V1 {
    const MODULUS_BITS: u32 = 64;
    const CAPACITY_BITS: u32 = 63;

    fn characteristic_descriptor() -> &'static Characteristic {
        &CHARACTERISTIC
    }

    fn from_bytes_mod_order(bytes_le: &[u8]) -> Self {
        let mut residue = 0_u64;
        for byte in bytes_le.iter().rev() {
            residue =
                crate::prime::reduce_goldilocks((u128::from(residue) << 8) + u128::from(*byte));
        }
        Self(residue)
    }
}

impl_prime_field_common!(
    FpGoldilocks64V1,
    catalog = crate::backend::fp_goldilocks64_v1_catalog,
    prime_metadata = crate::PrimeKernelMetadata::__from_generated(
        PrimeRepresentationKind::CanonicalResidue,
        crate::PrimeReductionKind::Barrett,
        RangeContract::__from_generated(1, 1, 128),
        RangeContract::__from_generated(1, 1, 128),
        1,
        false,
    ),
    spec = &SPEC,
    debug_name = "FpGoldilocks64V1"
);

const _: () = {
    assert!(<FpGoldilocks64V1 as PrimeFieldSpec>::LIMBS == 1);
    assert!(<FpGoldilocks64V1 as PrimeFieldSpec>::MODULUS[0] == MODULUS);
    assert!(core::mem::size_of::<FpGoldilocks64V1>() == 8);
    assert!(core::mem::align_of::<FpGoldilocks64V1>() == 8);
    assert!(FpGoldilocks64V1::solinas_plan().verify().is_ok());
    assert!(FpGoldilocks64V1::barrett_plan().verify(64).is_ok());
};
