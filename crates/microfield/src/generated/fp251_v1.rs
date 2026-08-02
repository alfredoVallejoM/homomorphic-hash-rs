//! Maintained prime field with modulus 251.

// Every narrowed value is first reduced below the 8-bit modulus.
#![allow(clippy::cast_possible_truncation)]

use crate::prime::{PrimeFieldSpec, PrimeWideProduct};
use crate::{
    ArtifactId, CanonicalEncoding, Characteristic, DecodeError, Field, FieldId, Invert,
    PrimeExponentiationPlan, PrimeField, PrimeReductionPlan, PrimeRepresentationKind,
    RangeContract, Square, SquareRootField, StaticFieldSpec,
};

use super::prime_field::impl_prime_field_common;

const MODULUS: u16 = 251;
const MODULUS_LIMBS: [u64; 1] = [251];
const CHARACTERISTIC: Characteristic = Characteristic::__from_generated("251", Some(251));

static SPEC: StaticFieldSpec = StaticFieldSpec {
    field_id: FieldId::__from_generated_hex(
        "aef78c79e5e5e929ee046a199df8eab46633a4ea7cabf66480fe2d7909d678da",
    ),
    artifact_id: ArtifactId::__from_generated_hex(
        "66c93999a195a2f387b6c3b51579c91e0a484746d81e91e7741275f78a13950a",
    ),
    name: "fp251_v1",
    characteristic: CHARACTERISTIC,
    degree: 1,
    canonical_bytes: 1,
    descriptor_json: include_bytes!("../../artifacts/fp251_v1/descriptor.json"),
    certificate_json: include_bytes!("../../artifacts/fp251_v1/certificate.json"),
};

/// The maintained prime field `GF(251)`.
///
/// Canonical encodings contain one byte and reject values from 251 through
/// 255. The private representation is one canonical byte.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Fp251V1(u8);

impl Fp251V1 {
    /// Prime modulus as a machine integer.
    pub const MODULUS: u16 = MODULUS;

    /// Returns the private representation family for diagnostics.
    #[must_use]
    pub const fn representation_kind() -> PrimeRepresentationKind {
        <Self as PrimeFieldSpec>::REPRESENTATION
    }

    /// Returns the generated reduction plan.
    #[must_use]
    pub const fn reduction_plan() -> PrimeReductionPlan {
        <Self as PrimeFieldSpec>::REDUCTION
    }

    /// Returns the generated and symbolically checked `p - 2` plan.
    #[must_use]
    pub const fn inversion_plan() -> PrimeExponentiationPlan<1> {
        PrimeExponentiationPlan::__from_generated([249], 8)
    }

    /// Reduces one machine integer explicitly.
    #[must_use]
    pub const fn from_u64_mod(value: u64) -> Self {
        Self((value % MODULUS as u64) as u8)
    }
}

impl Field for Fp251V1 {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);

    #[inline]
    fn add(self, rhs: Self) -> Self {
        let sum = u16::from(self.0) + u16::from(rhs.0);
        Self(if sum >= MODULUS {
            (sum - MODULUS) as u8
        } else {
            sum as u8
        })
    }

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        if self.0 >= rhs.0 {
            Self(self.0 - rhs.0)
        } else {
            Self((MODULUS - u16::from(rhs.0 - self.0)) as u8)
        }
    }

    #[inline]
    fn neg(self) -> Self {
        if self.0 == 0 {
            self
        } else {
            Self((MODULUS - u16::from(self.0)) as u8)
        }
    }

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let wide = <Self as PrimeWideProduct>::mul_wide(self, rhs);
        Self((wide % MODULUS) as u8)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl PrimeWideProduct for Fp251V1 {
    type Wide = u16;

    #[inline]
    fn mul_wide(self, rhs: Self) -> Self::Wide {
        u16::from(self.0) * u16::from(rhs.0)
    }
}

impl PrimeFieldSpec for Fp251V1 {
    const LIMBS: usize = 1;
    const MODULUS: &'static [u64] = &MODULUS_LIMBS;
    const REPRESENTATION: PrimeRepresentationKind = PrimeRepresentationKind::CanonicalResidue;
    const REDUCTION: PrimeReductionPlan = PrimeReductionPlan::Native { word_bits: 16 };
}

impl Square for Fp251V1 {
    #[inline]
    fn square(self) -> Self {
        self.mul(self)
    }
}

impl Invert for Fp251V1 {
    fn invert(self) -> Option<Self> {
        (!self.is_zero()).then(|| Self::inversion_plan().evaluate(self))
    }
}

impl SquareRootField for Fp251V1 {
    fn sqrt(self) -> Option<Self> {
        let root = crate::Pow::pow(self, &[63]);
        if root.square() != self {
            return None;
        }
        let negated = root.neg();
        Some(if root.0 <= negated.0 { root } else { negated })
    }
}

impl CanonicalEncoding for Fp251V1 {
    type Repr = [u8; 1];

    fn from_canonical(repr: &Self::Repr) -> Result<Self, DecodeError> {
        if u16::from(repr[0]) >= MODULUS {
            Err(DecodeError::NonCanonicalValue)
        } else {
            Ok(Self(repr[0]))
        }
    }

    fn from_canonical_slice(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() != 1 {
            return Err(DecodeError::LengthMismatch {
                expected: 1,
                actual: bytes.len(),
            });
        }
        Self::from_canonical(&[bytes[0]])
    }

    fn to_canonical(self) -> Self::Repr {
        [self.0]
    }
}

impl PrimeField for Fp251V1 {
    const MODULUS_BITS: u32 = 8;
    const CAPACITY_BITS: u32 = 7;

    fn characteristic_descriptor() -> &'static Characteristic {
        &CHARACTERISTIC
    }

    fn from_bytes_mod_order(bytes_le: &[u8]) -> Self {
        Self(crate::prime::reduce_bytes_mod_u16(bytes_le, MODULUS) as u8)
    }
}

impl_prime_field_common!(
    Fp251V1,
    catalog = crate::backend::fp251_v1_catalog,
    prime_metadata = crate::PrimeKernelMetadata::__from_generated(
        PrimeRepresentationKind::CanonicalResidue,
        crate::PrimeReductionKind::Native,
        RangeContract::__from_generated(1, 1, 16),
        RangeContract::__from_generated(1, 1, 16),
        1,
        false,
    ),
    spec = &SPEC,
    debug_name = "Fp251V1"
);

const _: () = {
    assert!(<Fp251V1 as PrimeFieldSpec>::LIMBS == 1);
    assert!(<Fp251V1 as PrimeFieldSpec>::MODULUS[0] == 251);
    assert!(core::mem::size_of::<Fp251V1>() == 1);
    assert!(core::mem::align_of::<Fp251V1>() == 1);
    assert!(RangeContract::__from_generated(1, 1, 16).verify(8).is_ok());
};
