//! Maintained polynomial-basis implementation of GF(2²⁵⁶).

use core::{
    fmt,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::{
    ArtifactId, BinaryPolynomialField, CanonicalEncoding, DecodeError, ExtensionField, F2, Field,
    FieldId, Invert, Pow, Square, StaticField, StaticFieldSpec,
    binary::{
        invert_256, mul_by_x_256, reduce_256, reduce_polynomial_bytes_256, square_256,
        wide_product_256,
    },
};

#[rustfmt::skip]
#[path = "../../artifacts/gf2_256_hh_v1/field.rs"]
mod constants;

const MODULUS_TAIL: u64 = (1 << constants::MODULUS_EXPONENTS_DESC[1])
    | (1 << constants::MODULUS_EXPONENTS_DESC[2])
    | (1 << constants::MODULUS_EXPONENTS_DESC[3])
    | (1 << constants::MODULUS_EXPONENTS_DESC[4]);

static SPEC: StaticFieldSpec = StaticFieldSpec {
    field_id: FieldId::from_bytes(constants::FIELD_ID),
    artifact_id: ArtifactId::from_bytes(constants::ARTIFACT_ID),
    name: constants::FIELD_NAME,
    characteristic: 2,
    degree: 256,
    canonical_bytes: 32,
    descriptor_json: include_bytes!("../../artifacts/gf2_256_hh_v1/descriptor.json"),
    certificate_json: include_bytes!("../../artifacts/gf2_256_hh_v1/certificate.json"),
};

/// The maintained field
/// `GF(2)[x] / (x^256 + x^10 + x^5 + x^2 + 1)`.
///
/// Values use a private polynomial-basis representation. Canonical bytes are
/// little-endian: bit `i` is the coefficient of `x^i`.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Gf2_256HhV1([u64; 4]);

impl Gf2_256HhV1 {
    fn from_limbs(limbs: [u64; 4]) -> Self {
        Self(limbs)
    }

    fn write_hex(self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for limb in self.0.into_iter().rev() {
            write!(formatter, "{limb:016x}")?;
        }
        Ok(())
    }
}

impl Field for Gf2_256HhV1 {
    const ZERO: Self = Self([0; 4]);
    const ONE: Self = Self([1, 0, 0, 0]);

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self([
            self.0[0] ^ rhs.0[0],
            self.0[1] ^ rhs.0[1],
            self.0[2] ^ rhs.0[2],
            self.0[3] ^ rhs.0[3],
        ])
    }

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Field::add(self, rhs)
    }

    #[inline]
    fn neg(self) -> Self {
        self
    }

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self::from_limbs(reduce_256::<MODULUS_TAIL>(wide_product_256(self.0, rhs.0)))
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0[0] | self.0[1] | self.0[2] | self.0[3] == 0
    }
}

impl Square for Gf2_256HhV1 {
    #[inline]
    fn square(self) -> Self {
        Self::from_limbs(square_256::<MODULUS_TAIL>(self.0))
    }
}

impl Invert for Gf2_256HhV1 {
    fn invert(self) -> Option<Self> {
        invert_256(self)
    }
}

impl Pow for Gf2_256HhV1 {}

impl CanonicalEncoding for Gf2_256HhV1 {
    type Repr = [u8; 32];

    fn from_canonical(repr: &Self::Repr) -> Result<Self, DecodeError> {
        let mut limbs = [0; 4];
        for (limb, bytes) in limbs.iter_mut().zip(repr.chunks_exact(8)) {
            *limb = u64::from_le_bytes(
                bytes
                    .try_into()
                    .expect("a 32-byte representation has four complete limbs"),
            );
        }
        Ok(Self::from_limbs(limbs))
    }

    fn from_canonical_slice(bytes: &[u8]) -> Result<Self, DecodeError> {
        let repr: [u8; 32] = bytes.try_into().map_err(|_| DecodeError::LengthMismatch {
            expected: 32,
            actual: bytes.len(),
        })?;
        Self::from_canonical(&repr)
    }

    fn to_canonical(self) -> Self::Repr {
        let mut bytes = [0; 32];
        for (limb, output) in self.0.into_iter().zip(bytes.chunks_exact_mut(8)) {
            output.copy_from_slice(&limb.to_le_bytes());
        }
        bytes
    }
}

impl ExtensionField for Gf2_256HhV1 {
    type Base = F2;
    const DEGREE: usize = 256;

    fn frobenius(self, power: usize) -> Self {
        let mut result = self;
        for _ in 0..power % Self::DEGREE {
            result = result.square();
        }
        result
    }

    fn trace(self) -> Self::Base {
        let mut conjugate = self;
        let mut result = self;
        for _ in 1..Self::DEGREE {
            conjugate = conjugate.square();
            result += conjugate;
        }
        debug_assert_eq!(result.0[1] | result.0[2] | result.0[3], 0);
        debug_assert!(result.0[0] <= 1);
        F2::from_bool(result.0[0] == 1)
    }

    fn norm(self) -> Self::Base {
        F2::from_bool(!self.is_zero())
    }
}

impl BinaryPolynomialField for Gf2_256HhV1 {
    const MODULUS_DEGREE: usize = 256;

    #[inline]
    fn mul_by_x(self) -> Self {
        Self::from_limbs(mul_by_x_256::<MODULUS_TAIL>(self.0))
    }

    fn from_polynomial_bytes_mod(bytes_le: &[u8]) -> Self {
        if bytes_le.len() <= 32 {
            let mut canonical = [0; 32];
            canonical[..bytes_le.len()].copy_from_slice(bytes_le);
            return Self::from_canonical(&canonical)
                .expect("every 256-bit polynomial is canonical");
        }
        Self::from_limbs(reduce_polynomial_bytes_256::<MODULUS_TAIL>(bytes_le))
    }
}

impl StaticField for Gf2_256HhV1 {
    fn spec() -> &'static StaticFieldSpec {
        &SPEC
    }
}

impl Add for Gf2_256HhV1 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Field::add(self, rhs)
    }
}

impl AddAssign for Gf2_256HhV1 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = Field::add(*self, rhs);
    }
}

impl Sub for Gf2_256HhV1 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Field::sub(self, rhs)
    }
}

impl SubAssign for Gf2_256HhV1 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = Field::sub(*self, rhs);
    }
}

impl Mul for Gf2_256HhV1 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Field::mul(self, rhs)
    }
}

impl MulAssign for Gf2_256HhV1 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = Field::mul(*self, rhs);
    }
}

impl Neg for Gf2_256HhV1 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Field::neg(self)
    }
}

impl fmt::Display for Gf2_256HhV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_hex(formatter)
    }
}

impl fmt::Debug for Gf2_256HhV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Gf2_256HhV1(0x")?;
        self.write_hex(formatter)?;
        formatter.write_str(")")
    }
}

const _: () = {
    assert!(constants::DEGREE == 256);
    assert!(constants::CANONICAL_BYTES == 32);
    assert!(constants::MODULUS_EXPONENTS_DESC.len() == 5);
    assert!(constants::MODULUS_EXPONENTS_DESC[0] == 256);
    assert!(constants::MODULUS_EXPONENTS_DESC[1] == 10);
    assert!(constants::MODULUS_EXPONENTS_DESC[2] == 5);
    assert!(constants::MODULUS_EXPONENTS_DESC[3] == 2);
    assert!(constants::MODULUS_EXPONENTS_DESC[4] == 0);
};
