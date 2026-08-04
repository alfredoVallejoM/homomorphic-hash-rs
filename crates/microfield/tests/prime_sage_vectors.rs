//! Typed replay of the independent `SageMath` corpus for every Phase 4 field.

#![cfg(all(feature = "generator", feature = "prime-fields"))]

use microfield::{CanonicalEncoding, Fp251V1, Fp256GenericV1, FpGoldilocks64V1, Invert, Square};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema: u32,
    oracle: String,
    seed: String,
    seed_sha256: String,
    fields: Vec<FieldVectors>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldVectors {
    name: String,
    modulus: String,
    canonical_bytes: usize,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    a: String,
    b: String,
    sum: String,
    difference: String,
    product: String,
    square: String,
    inverse: Option<String>,
}

#[test]
fn committed_prime_corpus_matches_every_public_operation() {
    let corpus: Corpus =
        serde_json::from_slice(include_bytes!("../reference-vectors/prime-fields-v1.json"))
            .unwrap();
    assert_eq!(corpus.schema, 1);
    assert_eq!(corpus.oracle, "SageMath exact integers");
    assert_eq!(corpus.seed, "microfield:fp256-generic-v1:2026-08-02");
    assert_eq!(
        corpus.seed_sha256,
        "cf6eae7cff8f204b479357c7b75741c7d422888d8e1649d7a6db0c11ff188599"
    );
    assert_eq!(corpus.fields.len(), 3);

    validate_field::<Fp251V1>(&corpus.fields[0], "fp251_v1", "251", 1);
    validate_field::<FpGoldilocks64V1>(
        &corpus.fields[1],
        "fp_goldilocks64_v1",
        "18446744069414584321",
        8,
    );
    validate_field::<Fp256GenericV1>(
        &corpus.fields[2],
        "fp256_generic_v1",
        "71319327679048415160211920703270965766974670828100238494590001805011376932671",
        32,
    );
}

fn validate_field<F>(vectors: &FieldVectors, name: &str, modulus: &str, width: usize)
where
    F: CanonicalEncoding + Invert + Square + core::fmt::Debug,
{
    assert_eq!(vectors.name, name);
    assert_eq!(vectors.modulus, modulus);
    assert_eq!(vectors.canonical_bytes, width);
    assert_eq!(vectors.cases.len(), 8);
    for case in &vectors.cases {
        let left = F::from_canonical_slice(&decode(&case.a)).unwrap();
        let right = F::from_canonical_slice(&decode(&case.b)).unwrap();
        assert_encoding(left.add(right), &case.sum, name, &case.name);
        assert_encoding(left.sub(right), &case.difference, name, &case.name);
        assert_encoding(left.mul(right), &case.product, name, &case.name);
        assert_encoding(left.square(), &case.square, name, &case.name);
        match (&case.inverse, left.invert()) {
            (None, None) => {}
            (Some(expected), Some(actual)) => assert_encoding(actual, expected, name, &case.name),
            _ => panic!("inverse mismatch for {name}/{}", case.name),
        }
    }
}

fn assert_encoding<F: CanonicalEncoding>(value: F, expected: &str, field: &str, case: &str) {
    assert_eq!(
        value.to_canonical().as_ref(),
        decode(expected),
        "oracle mismatch for {field}/{case}"
    );
}

fn decode(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| (nibble(pair[0]) << 4) | nibble(pair[1]))
        .collect()
}

fn nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("invalid oracle hex"),
    }
}
