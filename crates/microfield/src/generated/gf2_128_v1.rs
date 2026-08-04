//! Maintained polynomial-basis implementation of GF(2¹²⁸).

use crate::{ArtifactId, FieldId, StaticFieldSpec, binary::Polynomial128};

use super::binary_field::define_binary_field;

#[rustfmt::skip]
#[path = "../../artifacts/gf2_128_v1/field.rs"]
mod constants;

const MODULUS_TAIL: u64 = (1 << constants::MODULUS_EXPONENTS_DESC[1])
    | (1 << constants::MODULUS_EXPONENTS_DESC[2])
    | (1 << constants::MODULUS_EXPONENTS_DESC[3])
    | (1 << constants::MODULUS_EXPONENTS_DESC[4]);

static SPEC: StaticFieldSpec = StaticFieldSpec {
    field_id: FieldId::from_bytes(constants::FIELD_ID),
    artifact_id: ArtifactId::from_bytes(constants::ARTIFACT_ID),
    name: constants::FIELD_NAME,
    characteristic: crate::Characteristic::__from_generated("2", Some(2)),
    degree: 128,
    canonical_bytes: 16,
    descriptor_json: include_bytes!("../../artifacts/gf2_128_v1/descriptor.json"),
    certificate_json: include_bytes!("../../artifacts/gf2_128_v1/certificate.json"),
};

define_binary_field!(
    /// The maintained field `GF(2)[x] / (x^128 + x^7 + x^2 + x + 1)`.
    ///
    /// Canonical bytes are little-endian and the private representation has
    /// two naturally aligned limbs.
    Gf2_128V1,
    limbs = [u64; 2],
    repr = [u8; 16],
    implementation = Polynomial128<MODULUS_TAIL>,
    modulus_tail = MODULUS_TAIL,
    catalog = crate::backend::gf2_128_v1_catalog,
    spec = &SPEC,
    debug_name = "Gf2_128V1"
);

const _: () = {
    assert!(constants::DEGREE == 128);
    assert!(constants::CANONICAL_BYTES == 16);
    assert!(constants::MODULUS_EXPONENTS_DESC.len() == 5);
    assert!(constants::MODULUS_EXPONENTS_DESC[0] == 128);
    assert!(constants::MODULUS_EXPONENTS_DESC[1] == 7);
    assert!(constants::MODULUS_EXPONENTS_DESC[2] == 2);
    assert!(constants::MODULUS_EXPONENTS_DESC[3] == 1);
    assert!(constants::MODULUS_EXPONENTS_DESC[4] == 0);
};
