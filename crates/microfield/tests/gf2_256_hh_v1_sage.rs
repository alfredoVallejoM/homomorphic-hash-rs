//! End-to-end acceptance of the portable field against maintained Sage data.

#![cfg(all(feature = "builtin-fields", feature = "generator"))]

use std::path::PathBuf;

use microfield::{
    BinaryPolynomialField, CanonicalEncoding, Gf2_256HhV1, Invert, Pow, Square,
    spec::{Generator, JsonFileOracle, model::VectorOperation},
};

#[test]
fn every_maintained_sage_operation_matches_the_public_field_api() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fields")
        .join("gf2_256_hh_v1.toml");
    let vectors_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("reference-vectors")
        .join("gf2_256_hh_v1.json");
    let vectors = Generator::default()
        .vectors(&manifest, &JsonFileOracle::new(vectors_path))
        .expect("maintained vectors have already passed strict schema validation");

    assert_eq!(vectors.oracle().name(), "SageMath");
    assert_eq!(vectors.oracle().version(), "10.7");
    assert_eq!(vectors.vectors().len(), 11);

    for vector in vectors.vectors() {
        verify_operation(vector.case(), vector.operation());
    }
}

fn verify_operation(case: &str, operation: &VectorOperation) {
    match operation {
        VectorOperation::Canonical { element_hex_le } => {
            let bytes = decode_hex(element_hex_le);
            assert_eq!(element(&bytes).to_canonical().as_slice(), bytes, "{case}");
        }
        VectorOperation::Add {
            lhs_hex_le,
            rhs_hex_le,
            output_hex_le,
        } => assert_binary(case, lhs_hex_le, rhs_hex_le, output_hex_le, |lhs, rhs| {
            lhs + rhs
        }),
        VectorOperation::WideProduct {
            lhs_hex_le,
            rhs_hex_le,
            output_wide_hex_le,
        } => assert_eq!(
            element(&decode_hex(lhs_hex_le)) * element(&decode_hex(rhs_hex_le)),
            Gf2_256HhV1::from_polynomial_bytes_mod(&decode_hex(output_wide_hex_le)),
            "{case}"
        ),
        VectorOperation::Reduce {
            input_wide_hex_le,
            output_hex_le,
        } => assert_eq!(
            Gf2_256HhV1::from_polynomial_bytes_mod(&decode_hex(input_wide_hex_le)),
            element(&decode_hex(output_hex_le)),
            "{case}"
        ),
        VectorOperation::Multiply {
            lhs_hex_le,
            rhs_hex_le,
            output_hex_le,
        } => assert_binary(case, lhs_hex_le, rhs_hex_le, output_hex_le, |lhs, rhs| {
            lhs * rhs
        }),
        VectorOperation::Square {
            input_hex_le,
            output_hex_le,
        } => assert_unary(case, input_hex_le, output_hex_le, Square::square),
        VectorOperation::Invert {
            input_hex_le,
            output_hex_le,
        } => {
            let expected = output_hex_le
                .as_ref()
                .map(|output| element(&decode_hex(output)));
            assert_eq!(
                element(&decode_hex(input_hex_le)).invert(),
                expected,
                "{case}"
            );
        }
        VectorOperation::Pow {
            base_hex_le,
            exponent_hex_le,
            output_hex_le,
        } => assert_eq!(
            element(&decode_hex(base_hex_le)).pow(&exponent_words(exponent_hex_le)),
            element(&decode_hex(output_hex_le)),
            "{case}"
        ),
        VectorOperation::MulByX {
            input_hex_le,
            output_hex_le,
        } => assert_unary(
            case,
            input_hex_le,
            output_hex_le,
            BinaryPolynomialField::mul_by_x,
        ),
    }
}

fn assert_binary(
    case: &str,
    lhs: &str,
    rhs: &str,
    output: &str,
    operation: impl FnOnce(Gf2_256HhV1, Gf2_256HhV1) -> Gf2_256HhV1,
) {
    assert_eq!(
        operation(element(&decode_hex(lhs)), element(&decode_hex(rhs))),
        element(&decode_hex(output)),
        "{case}"
    );
}

fn assert_unary(
    case: &str,
    input: &str,
    output: &str,
    operation: impl FnOnce(Gf2_256HhV1) -> Gf2_256HhV1,
) {
    assert_eq!(
        operation(element(&decode_hex(input))),
        element(&decode_hex(output)),
        "{case}"
    );
}

fn element(bytes: &[u8]) -> Gf2_256HhV1 {
    Gf2_256HhV1::from_canonical_slice(bytes).expect("Sage elements have canonical width")
}

fn exponent_words(hex: &str) -> Vec<u64> {
    decode_hex(hex)
        .chunks(8)
        .map(|chunk| {
            let mut word = [0; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            u64::from_le_bytes(word)
        })
        .collect()
}

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("golden hex is UTF-8"), 16)
                .expect("strict schema already validated hexadecimal")
        })
        .collect()
}
