//! Maintained generic 256-bit prime field in Montgomery representation.

use core::cmp::Ordering;

use crate::prime::{PrimeFieldSpec, PrimeWideProduct};
use crate::{
    ArtifactId, CanonicalEncoding, Characteristic, DecodeError, Field, FieldId, Invert,
    MontgomeryAlgorithm, MontgomeryPlan, PrimeExponentiationPlan, PrimeField, PrimeReductionPlan,
    PrimeRepresentationKind, RangeContract, Square, SquareRootField, StaticFieldSpec,
};

use super::prime_field::impl_prime_field_common;

pub(crate) const MODULUS: [u64; 4] = [
    0x60d7_67ee_a528_073f,
    0x59b0_47d9_a719_3eed,
    0xa2df_4d6d_fbec_a16e,
    0x9dad_4f18_e672_38cb,
];
const R: [u64; 4] = [
    0x9f28_9811_5ad7_f8c1,
    0xa64f_b826_58e6_c112,
    0x5d20_b292_0413_5e91,
    0x6252_b0e7_198d_c734,
];
const R2: [u64; 4] = [
    0x0dd2_f2a9_c0b6_0e80,
    0x91ef_bf81_c4cb_0056,
    0x55a3_ac4e_36a4_0349,
    0x6ba2_65a9_ee77_837f,
];
pub(crate) const NEG_INV: u64 = 0x5479_78e4_7770_9741;
const INVERSE_EXPONENT: [u64; 4] = [
    0x60d7_67ee_a528_073d,
    0x59b0_47d9_a719_3eed,
    0xa2df_4d6d_fbec_a16e,
    0x9dad_4f18_e672_38cb,
];
const SQRT_EXPONENT: [u64; 4] = [
    0x5835_d9fb_a94a_01d0,
    0x966c_11f6_69c6_4fbb,
    0xe8b7_d35b_7efb_285b,
    0x276b_53c6_399c_8e32,
];
const CHARACTERISTIC: Characteristic = Characteristic::__from_generated(
    "71319327679048415160211920703270965766974670828100238494590001805011376932671",
    None,
);

static SPEC: StaticFieldSpec = StaticFieldSpec {
    field_id: FieldId::__from_generated_hex(
        "60cbdb42c3d6efbc7158144f6a42d015a708ca15ae47e5156204660f97681e8e",
    ),
    artifact_id: ArtifactId::__from_generated_hex(
        "ada93f06fbf2d8abac8885764ff3c20a220fd0e80287f9db924ff2ca9e310940",
    ),
    name: "fp256_generic_v1",
    characteristic: CHARACTERISTIC,
    degree: 1,
    canonical_bytes: 32,
    descriptor_json: include_bytes!("../../artifacts/fp256_generic_v1/descriptor.json"),
    certificate_json: include_bytes!("../../artifacts/fp256_generic_v1/certificate.json"),
};

/// A maintained generic 256-bit prime field.
///
/// The public encoding is a canonical 32-byte little-endian integer. The
/// private four-limb value is a reduced Montgomery residue and cannot be
/// constructed or observed by consumers.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Fp256GenericV1([u64; 4]);

impl Fp256GenericV1 {
    /// Returns the 32-byte little-endian prime modulus.
    #[must_use]
    pub const fn modulus_le_bytes() -> [u8; 32] {
        encode_limbs(MODULUS)
    }

    /// Returns the private representation family without exposing residues.
    #[must_use]
    pub const fn representation_kind() -> PrimeRepresentationKind {
        <Self as PrimeFieldSpec>::REPRESENTATION
    }

    /// Returns the selected Montgomery reduction plan.
    #[must_use]
    pub const fn reduction_plan() -> PrimeReductionPlan {
        <Self as PrimeFieldSpec>::REDUCTION
    }

    /// Returns the generated and symbolically checked `p - 2` plan.
    #[must_use]
    pub const fn inversion_plan() -> PrimeExponentiationPlan<4> {
        PrimeExponentiationPlan::__from_generated(INVERSE_EXPONENT, 256)
    }

    /// Returns the verified Montgomery schedule and public shape metadata.
    #[must_use]
    pub const fn montgomery_plan() -> MontgomeryPlan {
        MontgomeryPlan::__from_generated(
            64,
            4,
            &MODULUS,
            &R,
            &R2,
            NEG_INV,
            MontgomeryAlgorithm::Cios,
            RangeContract::__from_generated(1, 1, 512),
        )
    }

    /// Converts a machine integer by explicit modular reduction.
    #[must_use]
    pub fn from_u64_mod(value: u64) -> Self {
        Self(crate::prime::to_montgomery_256(
            [value, 0, 0, 0],
            R2,
            MODULUS,
            NEG_INV,
        ))
    }

    #[cfg(all(feature = "portable", target_arch = "x86_64"))]
    pub(crate) const fn into_montgomery_limbs(self) -> [u64; 4] {
        self.0
    }

    #[cfg(all(feature = "portable", target_arch = "x86_64"))]
    pub(crate) fn reduce_isa_product(wide: [u64; 8]) -> Self {
        Self(crate::prime::montgomery_reduce_wide_256(
            wide, MODULUS, NEG_INV,
        ))
    }
}

impl Field for Fp256GenericV1 {
    const ZERO: Self = Self([0; 4]);
    const ONE: Self = Self(R);

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(crate::prime::add_mod_256(self.0, rhs.0, MODULUS))
    }

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(crate::prime::sub_mod_256(self.0, rhs.0, MODULUS))
    }

    #[inline]
    fn neg(self) -> Self {
        Self(crate::prime::neg_mod_256(self.0, MODULUS))
    }

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self(crate::prime::montgomery_reduce_wide_256(
            <Self as PrimeWideProduct>::mul_wide(self, rhs),
            MODULUS,
            NEG_INV,
        ))
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0 == [0; 4]
    }
}

impl PrimeWideProduct for Fp256GenericV1 {
    type Wide = [u64; 8];

    #[inline]
    fn mul_wide(self, rhs: Self) -> Self::Wide {
        crate::prime::wide_product(self.0, rhs.0)
    }
}

impl PrimeFieldSpec for Fp256GenericV1 {
    const LIMBS: usize = 4;
    const MODULUS: &'static [u64] = &MODULUS;
    const REPRESENTATION: PrimeRepresentationKind = PrimeRepresentationKind::Montgomery {
        radix_bits: 64,
        limbs: 4,
    };
    const REDUCTION: PrimeReductionPlan =
        PrimeReductionPlan::Montgomery(Fp256GenericV1::montgomery_plan());
}

impl Square for Fp256GenericV1 {
    #[inline]
    fn square(self) -> Self {
        self.mul(self)
    }
}

impl Invert for Fp256GenericV1 {
    fn invert(self) -> Option<Self> {
        (!self.is_zero()).then(|| Self::inversion_plan().evaluate(self))
    }
}

impl SquareRootField for Fp256GenericV1 {
    fn sqrt(self) -> Option<Self> {
        let root = crate::Pow::pow(self, &SQRT_EXPONENT);
        if root.square() != self {
            return None;
        }
        let negated = root.neg();
        let root_canonical = root.canonical_limbs();
        let negated_canonical = negated.canonical_limbs();
        Some(
            if crate::prime::cmp_limbs(&root_canonical, &negated_canonical) == Ordering::Greater {
                negated
            } else {
                root
            },
        )
    }
}

impl Fp256GenericV1 {
    fn canonical_limbs(self) -> [u64; 4] {
        crate::prime::from_montgomery_256(self.0, MODULUS, NEG_INV)
    }
}

impl CanonicalEncoding for Fp256GenericV1 {
    type Repr = [u8; 32];

    fn from_canonical(repr: &Self::Repr) -> Result<Self, DecodeError> {
        let canonical = decode_limbs(*repr);
        if crate::prime::cmp_limbs(&canonical, &MODULUS) != Ordering::Less {
            return Err(DecodeError::NonCanonicalValue);
        }
        Ok(Self(crate::prime::to_montgomery_256(
            canonical, R2, MODULUS, NEG_INV,
        )))
    }

    fn from_canonical_slice(bytes: &[u8]) -> Result<Self, DecodeError> {
        let repr: [u8; 32] = bytes.try_into().map_err(|_| DecodeError::LengthMismatch {
            expected: 32,
            actual: bytes.len(),
        })?;
        Self::from_canonical(&repr)
    }

    fn to_canonical(self) -> Self::Repr {
        encode_limbs(self.canonical_limbs())
    }
}

impl PrimeField for Fp256GenericV1 {
    const MODULUS_BITS: u32 = 256;
    const CAPACITY_BITS: u32 = 255;

    fn characteristic_descriptor() -> &'static Characteristic {
        &CHARACTERISTIC
    }

    fn from_bytes_mod_order(bytes_le: &[u8]) -> Self {
        let radix = Self::from_u64_mod(256);
        let mut residue = Self::ZERO;
        for byte in bytes_le.iter().rev() {
            residue = residue.mul(radix).add(Self::from_u64_mod(u64::from(*byte)));
        }
        residue
    }
}

impl_prime_field_common!(
    Fp256GenericV1,
    catalog = crate::backend::fp256_generic_v1_catalog,
    prime_metadata = crate::PrimeKernelMetadata::__from_generated(
        PrimeRepresentationKind::Montgomery {
            radix_bits: 64,
            limbs: 4,
        },
        crate::PrimeReductionKind::Montgomery,
        RangeContract::__from_generated(1, 1, 512),
        RangeContract::__from_generated(1, 1, 512),
        1,
        false,
    ),
    spec = &SPEC,
    debug_name = "Fp256GenericV1"
);

const fn decode_limbs(bytes: [u8; 32]) -> [u64; 4] {
    let mut limbs = [0_u64; 4];
    let mut index = 0;
    while index < 4 {
        let mut limb_bytes = [0_u8; 8];
        let mut offset = 0;
        while offset < 8 {
            limb_bytes[offset] = bytes[index * 8 + offset];
            offset += 1;
        }
        limbs[index] = u64::from_le_bytes(limb_bytes);
        index += 1;
    }
    limbs
}

const fn encode_limbs(limbs: [u64; 4]) -> [u8; 32] {
    let mut bytes = [0_u8; 32];
    let mut index = 0;
    while index < 4 {
        let limb_bytes = limbs[index].to_le_bytes();
        let mut offset = 0;
        while offset < 8 {
            bytes[index * 8 + offset] = limb_bytes[offset];
            offset += 1;
        }
        index += 1;
    }
    bytes
}

const _: () = {
    assert!(<Fp256GenericV1 as PrimeFieldSpec>::LIMBS == 4);
    assert!(<Fp256GenericV1 as PrimeFieldSpec>::MODULUS[3] == MODULUS[3]);
    assert!(core::mem::size_of::<Fp256GenericV1>() == 32);
    assert!(core::mem::align_of::<Fp256GenericV1>() == 8);
    assert!(Fp256GenericV1::montgomery_plan().verify(256).is_ok());
};
