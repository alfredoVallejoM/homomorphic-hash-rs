//! End-to-end acceptance of all built-in fields against maintained Sage data.

#![cfg(all(feature = "builtin-fields", feature = "generator"))]

use core::ops::{Add, Mul};
use std::path::{Path, PathBuf};

use microfield::{
    BinaryPolynomialField, CanonicalEncoding, Field, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, Invert,
    Pow, Square,
    spec::{Generator, JsonFileOracle, model::VectorOperation},
};

#[test]
fn every_maintained_sage_operation_matches_every_public_field_api() {
    verify_set::<Gf2_128V1>("gf2_128_v1");
    verify_set::<Gf2_256HhV1>("gf2_256_hh_v1");
    verify_set::<Gf2_256AltV1>("gf2_256_alt_v1");
}

fn verify_set<F>(stem: &str)
where
    F: Field
        + Square
        + Invert
        + Pow
        + CanonicalEncoding
        + BinaryPolynomialField
        + Add<Output = F>
        + Mul<Output = F>
        + core::fmt::Debug,
{
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = root.join("fields").join(format!("{stem}.toml"));
    let vectors_path = root.join("reference-vectors").join(format!("{stem}.json"));
    let vectors = Generator::default()
        .vectors(&manifest, &JsonFileOracle::new(vectors_path))
        .expect("maintained vectors have already passed strict schema validation");

    assert_eq!(vectors.oracle().name(), "SageMath");
    assert_eq!(vectors.oracle().version(), "10.7");
    assert_eq!(vectors.vectors().len(), 11);

    for vector in vectors.vectors() {
        verify_operation::<F>(&manifest, vector.case(), vector.operation());
    }
}

fn verify_operation<F>(manifest: &Path, case: &str, operation: &VectorOperation)
where
    F: Field
        + Square
        + Invert
        + Pow
        + CanonicalEncoding
        + BinaryPolynomialField
        + Add<Output = F>
        + Mul<Output = F>
        + core::fmt::Debug,
{
    let diagnostic = || format!("{}:{case}", manifest.display());
    match operation {
        VectorOperation::Canonical { element_hex_le } => {
            let bytes = decode_hex(element_hex_le);
            assert_eq!(
                element::<F>(&bytes).to_canonical().as_ref(),
                bytes,
                "{}",
                diagnostic()
            );
        }
        VectorOperation::Add {
            lhs_hex_le,
            rhs_hex_le,
            output_hex_le,
        } => assert_binary::<F>(case, lhs_hex_le, rhs_hex_le, output_hex_le, |lhs, rhs| {
            lhs + rhs
        }),
        VectorOperation::WideProduct {
            lhs_hex_le,
            rhs_hex_le,
            output_wide_hex_le,
        } => assert_eq!(
            element::<F>(&decode_hex(lhs_hex_le)) * element::<F>(&decode_hex(rhs_hex_le)),
            F::from_polynomial_bytes_mod(&decode_hex(output_wide_hex_le)),
            "{}",
            diagnostic()
        ),
        VectorOperation::Reduce {
            input_wide_hex_le,
            output_hex_le,
        } => assert_eq!(
            F::from_polynomial_bytes_mod(&decode_hex(input_wide_hex_le)),
            element::<F>(&decode_hex(output_hex_le)),
            "{}",
            diagnostic()
        ),
        VectorOperation::Multiply {
            lhs_hex_le,
            rhs_hex_le,
            output_hex_le,
        } => assert_binary::<F>(case, lhs_hex_le, rhs_hex_le, output_hex_le, |lhs, rhs| {
            lhs * rhs
        }),
        VectorOperation::Square {
            input_hex_le,
            output_hex_le,
        } => assert_unary::<F>(case, input_hex_le, output_hex_le, Square::square),
        VectorOperation::Invert {
            input_hex_le,
            output_hex_le,
        } => {
            let expected = output_hex_le
                .as_ref()
                .map(|output| element::<F>(&decode_hex(output)));
            assert_eq!(
                element::<F>(&decode_hex(input_hex_le)).invert(),
                expected,
                "{}",
                diagnostic()
            );
        }
        VectorOperation::Pow {
            base_hex_le,
            exponent_hex_le,
            output_hex_le,
        } => assert_eq!(
            element::<F>(&decode_hex(base_hex_le)).pow(&exponent_words(exponent_hex_le)),
            element::<F>(&decode_hex(output_hex_le)),
            "{}",
            diagnostic()
        ),
        VectorOperation::MulByX {
            input_hex_le,
            output_hex_le,
        } => assert_unary::<F>(
            case,
            input_hex_le,
            output_hex_le,
            BinaryPolynomialField::mul_by_x,
        ),
    }
}

fn assert_binary<F>(
    case: &str,
    lhs: &str,
    rhs: &str,
    output: &str,
    operation: impl FnOnce(F, F) -> F,
) where
    F: Field + CanonicalEncoding + core::fmt::Debug,
{
    assert_eq!(
        operation(
            element::<F>(&decode_hex(lhs)),
            element::<F>(&decode_hex(rhs))
        ),
        element::<F>(&decode_hex(output)),
        "{case}"
    );
}

fn assert_unary<F>(case: &str, input: &str, output: &str, operation: impl FnOnce(F) -> F)
where
    F: Field + CanonicalEncoding + core::fmt::Debug,
{
    assert_eq!(
        operation(element::<F>(&decode_hex(input))),
        element::<F>(&decode_hex(output)),
        "{case}"
    );
}

fn element<F: CanonicalEncoding>(bytes: &[u8]) -> F {
    F::from_canonical_slice(bytes).expect("Sage elements have canonical width")
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
