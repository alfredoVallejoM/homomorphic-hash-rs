//! Alternative maintained polynomial-basis implementation of GF(2²⁵⁶).

use crate::{ArtifactId, FieldId, StaticFieldSpec, binary::Polynomial256};

use super::binary_field::define_binary_field;

#[rustfmt::skip]
#[path = "../../artifacts/gf2_256_alt_v1/field.rs"]
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
    degree: 256,
    canonical_bytes: 32,
    descriptor_json: include_bytes!("../../artifacts/gf2_256_alt_v1/descriptor.json"),
    certificate_json: include_bytes!("../../artifacts/gf2_256_alt_v1/certificate.json"),
};

define_binary_field!(
    /// The maintained field `GF(2)[x] / (x^256 + x^16 + x^3 + x + 1)`.
    ///
    /// It is intentionally a different nominal type from `Gf2_256HhV1` even
    /// though both fields have the same cardinality.
    Gf2_256AltV1,
    limbs = [u64; 4],
    repr = [u8; 32],
    implementation = Polynomial256<MODULUS_TAIL>,
    modulus_tail = MODULUS_TAIL,
    catalog = crate::backend::gf2_256_alt_v1_catalog,
    spec = &SPEC,
    debug_name = "Gf2_256AltV1"
);

const _: () = {
    assert!(constants::DEGREE == 256);
    assert!(constants::CANONICAL_BYTES == 32);
    assert!(constants::MODULUS_EXPONENTS_DESC.len() == 5);
    assert!(constants::MODULUS_EXPONENTS_DESC[0] == 256);
    assert!(constants::MODULUS_EXPONENTS_DESC[1] == 16);
    assert!(constants::MODULUS_EXPONENTS_DESC[2] == 3);
    assert!(constants::MODULUS_EXPONENTS_DESC[3] == 1);
    assert!(constants::MODULUS_EXPONENTS_DESC[4] == 0);
};
