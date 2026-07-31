//! Exhaustive schema-v2 reference-vector contracts.

#![cfg(feature = "generator")]

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use microfield::spec::{
    Generator, JsonFileOracle,
    error::PipelineError,
    model::{
        REFERENCE_VECTOR_MAXIMUM_CASES, REFERENCE_VECTOR_MAXIMUM_EXPONENT_BYTES,
        REFERENCE_VECTOR_MAXIMUM_JSON_BYTES, VectorOperation,
    },
};
use sha2::{Digest, Sha256};

#[test]
fn complete_typed_set_round_trips_and_exposes_every_operation() {
    let fixture = VectorFixture::frozen_128("microfield-vector-v2-complete");
    let value = valid_vector_set(&fixture.field_id, 16);
    fixture.write_json(&value);

    let vectors = fixture.load().expect("complete schema-v2 set is accepted");
    assert_eq!(vectors.schema(), 2);
    assert_eq!(vectors.oracle().name(), "independent-test");
    assert_eq!(vectors.oracle().version(), "1.0");
    assert_eq!(vectors.generation().algorithm(), "sha256-labeled-v1");
    assert_eq!(vectors.generation().seed_hex(), "11".repeat(32));
    assert_eq!(vectors.vectors().len(), 11);
    assert!(matches!(
        vectors.vectors()[2].operation(),
        VectorOperation::Add { .. }
    ));

    let serialized = serde_json::to_value(&vectors).expect("accepted model serializes");
    assert_eq!(serialized, value);
}

#[test]
fn every_normative_operation_and_both_inversion_domains_are_required() {
    let fixture = VectorFixture::frozen_128("microfield-vector-v2-coverage");
    for required_case in [
        "canonical_zero",
        "add_zero",
        "wide_product_zero",
        "reduce_zero",
        "multiply_zero",
        "square_zero",
        "invert_zero",
        "invert_one",
        "pow_zero",
        "mul_by_x_zero",
    ] {
        let mut value = valid_vector_set(&fixture.field_id, 16);
        value["vectors"]
            .as_array_mut()
            .expect("vectors array")
            .retain(|case| {
                case["case"] != required_case
                    && !(required_case == "canonical_zero" && case["case"] == "canonical_one")
            });
        fixture.write_json(&value);
        let error = fixture.load().expect_err("coverage gap must fail");
        assert_vector_error(&error, "vectors");
    }
}

#[test]
fn canonical_operands_enforce_width_lowercase_and_declared_degree() {
    let fixture = VectorFixture::degree_five("microfield-vector-v2-canonical");
    let base = valid_vector_set(&fixture.field_id, 1);
    let cases = [
        (
            "/vectors/0/operation/element_hex_le",
            serde_json::json!("0"),
        ),
        (
            "/vectors/0/operation/element_hex_le",
            serde_json::json!("00ff"),
        ),
        (
            "/vectors/0/operation/element_hex_le",
            serde_json::json!("AA"),
        ),
        (
            "/vectors/0/operation/element_hex_le",
            serde_json::json!("gg"),
        ),
        (
            "/vectors/0/operation/element_hex_le",
            serde_json::json!("20"),
        ),
    ];

    for (pointer, replacement) in cases {
        let invalid = with_value(&base, pointer, replacement);
        fixture.write_json(&invalid);
        assert_vector_error(
            &fixture.load().expect_err("invalid canonical encoding"),
            "vectors[0].operation.element_hex_le",
        );
    }
}

#[test]
fn wide_polynomials_enforce_double_width_and_unused_high_bits() {
    let fixture = VectorFixture::degree_five("microfield-vector-v2-wide");
    let base = valid_vector_set(&fixture.field_id, 1);
    for replacement in [
        serde_json::json!("00"),
        serde_json::json!("000000"),
        serde_json::json!("0002"),
        serde_json::json!("GG00"),
    ] {
        let invalid = with_value(
            &base,
            "/vectors/3/operation/output_wide_hex_le",
            replacement,
        );
        fixture.write_json(&invalid);
        assert_vector_error(
            &fixture.load().expect_err("invalid wide encoding"),
            "vectors[3].operation.output_wide_hex_le",
        );
    }
}

#[test]
fn exponents_use_bounded_minimal_little_endian_hex() {
    let fixture = VectorFixture::frozen_128("microfield-vector-v2-exponents");
    let base = valid_vector_set(&fixture.field_id, 16);
    for replacement in [
        serde_json::json!(""),
        serde_json::json!("1"),
        serde_json::json!("GG"),
        serde_json::json!("AA"),
        serde_json::json!("0100"),
        serde_json::json!("01".repeat(REFERENCE_VECTOR_MAXIMUM_EXPONENT_BYTES + 1)),
    ] {
        let invalid = with_value(&base, "/vectors/9/operation/exponent_hex_le", replacement);
        fixture.write_json(&invalid);
        assert_vector_error(
            &fixture.load().expect_err("invalid exponent encoding"),
            "vectors[9].operation.exponent_hex_le",
        );
    }
}

#[test]
fn inverse_output_is_null_exactly_for_zero() {
    let fixture = VectorFixture::frozen_128("microfield-vector-v2-inverse");
    let base = valid_vector_set(&fixture.field_id, 16);
    let zero_with_output = with_value(
        &base,
        "/vectors/7/operation/output_hex_le",
        serde_json::json!("00".repeat(16)),
    );
    fixture.write_json(&zero_with_output);
    assert_vector_error(
        &fixture.load().expect_err("zero has no inverse"),
        "vectors[7].operation.output_hex_le",
    );

    let nonzero_without_output = with_value(
        &base,
        "/vectors/8/operation/output_hex_le",
        serde_json::Value::Null,
    );
    fixture.write_json(&nonzero_without_output);
    assert_vector_error(
        &fixture.load().expect_err("nonzero requires an inverse"),
        "vectors[8].operation.output_hex_le",
    );
}

#[test]
fn case_names_are_unique_stable_and_bounded() {
    let fixture = VectorFixture::frozen_128("microfield-vector-v2-cases");
    let base = valid_vector_set(&fixture.field_id, 16);
    for replacement in [
        "",
        "_leading",
        "trailing_",
        "double__gap",
        "Upper",
        "bad-name",
    ] {
        let invalid = with_value(&base, "/vectors/0/case", serde_json::json!(replacement));
        fixture.write_json(&invalid);
        assert_vector_error(
            &fixture.load().expect_err("invalid case name"),
            "vectors[0].case",
        );
    }

    let duplicate = with_value(
        &base,
        "/vectors/1/case",
        serde_json::json!("canonical_zero"),
    );
    fixture.write_json(&duplicate);
    assert_vector_error(
        &fixture.load().expect_err("duplicate case name"),
        "vectors[1].case",
    );
}

#[test]
fn envelope_metadata_identity_and_generation_are_strict() {
    let fixture = VectorFixture::frozen_128("microfield-vector-v2-envelope");
    let base = valid_vector_set(&fixture.field_id, 16);
    let cases = [
        ("/schema", serde_json::json!(1), "schema"),
        ("/field_id", serde_json::json!("AA".repeat(32)), "field_id"),
        ("/oracle/name", serde_json::json!(" Sage"), "oracle.name"),
        ("/oracle/version", serde_json::json!(""), "oracle.version"),
        (
            "/generation/algorithm",
            serde_json::json!("sha256-labeled-v2"),
            "generation.algorithm",
        ),
        (
            "/generation/seed_hex",
            serde_json::json!("00"),
            "generation.seed_hex",
        ),
    ];
    for (pointer, replacement, expected_path) in cases {
        let invalid = with_value(&base, pointer, replacement);
        fixture.write_json(&invalid);
        assert_vector_error(
            &fixture.load().expect_err("invalid envelope"),
            expected_path,
        );
    }
}

#[test]
fn unknown_fields_and_unknown_operation_kinds_fail_during_deserialization() {
    let fixture = VectorFixture::frozen_128("microfield-vector-v2-unknown");
    let mut unknown_field = valid_vector_set(&fixture.field_id, 16);
    unknown_field["vectors"][0]["operation"]["surprise"] = serde_json::json!(true);
    fixture.write_json(&unknown_field);
    assert!(matches!(fixture.load(), Err(PipelineError::Adapter(_))));

    let unknown_kind = with_value(
        &valid_vector_set(&fixture.field_id, 16),
        "/vectors/0/operation/kind",
        serde_json::json!("future_operation"),
    );
    fixture.write_json(&unknown_kind);
    assert!(matches!(fixture.load(), Err(PipelineError::Adapter(_))));
}

#[test]
fn case_count_and_json_size_have_hard_resource_limits() {
    let fixture = VectorFixture::frozen_128("microfield-vector-v2-limits");
    let mut too_many = valid_vector_set(&fixture.field_id, 16);
    let template = too_many["vectors"][0].clone();
    let cases = (0..=REFERENCE_VECTOR_MAXIMUM_CASES)
        .map(|index| {
            let mut case = template.clone();
            case["case"] = serde_json::json!(format!("case_{index}"));
            case
        })
        .collect();
    too_many["vectors"] = serde_json::Value::Array(cases);
    fixture.write_json(&too_many);
    assert_vector_error(&fixture.load().expect_err("too many cases"), "vectors");

    fs::write(
        &fixture.vectors,
        vec![b' '; REFERENCE_VECTOR_MAXIMUM_JSON_BYTES + 1],
    )
    .expect("oversized fixture writable");
    assert!(matches!(fixture.load(), Err(PipelineError::Adapter(_))));
}

#[test]
fn committed_sage_goldens_match_an_independent_slow_polynomial_model() {
    let fields = [
        ("gf2_128_v1", 128, &[128, 7, 2, 1, 0][..]),
        ("gf2_256_alt_v1", 256, &[256, 16, 3, 1, 0][..]),
        ("gf2_256_hh_v1", 256, &[256, 10, 5, 2, 0][..]),
    ];

    for (field, degree, modulus) in fields {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fields")
            .join(format!("{field}.toml"));
        let vectors_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("reference-vectors")
            .join(format!("{field}.json"));
        let vectors = Generator::default()
            .vectors(&manifest, &JsonFileOracle::new(vectors_path))
            .expect("committed Sage vectors satisfy schema v2");
        assert_eq!(vectors.oracle().name(), "SageMath");
        assert_eq!(vectors.oracle().version(), "10.7");
        let expected_seed = Sha256::new()
            .chain_update(b"microfield:sage-vector-seed:v2\0")
            .chain_update(decode_hex(vectors.field_id()))
            .finalize();
        assert_eq!(
            vectors.generation().seed_hex(),
            encode_hex(expected_seed.as_slice())
        );

        for vector in vectors.vectors() {
            verify_operation(vector.operation(), degree, modulus);
        }
    }
}

fn verify_operation(operation: &VectorOperation, degree: usize, modulus: &[usize]) {
    match operation {
        VectorOperation::Canonical { element_hex_le } => {
            assert_eq!(encode_hex(&decode_hex(element_hex_le)), *element_hex_le);
        }
        VectorOperation::Add {
            lhs_hex_le,
            rhs_hex_le,
            output_hex_le,
        } => {
            let output = xor(&decode_hex(lhs_hex_le), &decode_hex(rhs_hex_le));
            assert_eq!(encode_hex(&output), *output_hex_le);
        }
        VectorOperation::WideProduct {
            lhs_hex_le,
            rhs_hex_le,
            output_wide_hex_le,
        } => {
            let output = slow_wide_product(&decode_hex(lhs_hex_le), &decode_hex(rhs_hex_le));
            assert_eq!(encode_hex(&output), *output_wide_hex_le);
        }
        VectorOperation::Reduce {
            input_wide_hex_le,
            output_hex_le,
        } => {
            let output = slow_reduce(&decode_hex(input_wide_hex_le), degree, modulus);
            assert_eq!(encode_hex(&output), *output_hex_le);
        }
        VectorOperation::Multiply {
            lhs_hex_le,
            rhs_hex_le,
            output_hex_le,
        } => {
            let output = slow_field_multiply(
                &decode_hex(lhs_hex_le),
                &decode_hex(rhs_hex_le),
                degree,
                modulus,
            );
            assert_eq!(encode_hex(&output), *output_hex_le);
        }
        VectorOperation::Square {
            input_hex_le,
            output_hex_le,
        } => {
            let input = decode_hex(input_hex_le);
            let output = slow_field_multiply(&input, &input, degree, modulus);
            assert_eq!(encode_hex(&output), *output_hex_le);
        }
        VectorOperation::Invert {
            input_hex_le,
            output_hex_le,
        } => {
            let input = decode_hex(input_hex_le);
            if input.iter().all(|byte| *byte == 0) {
                assert!(output_hex_le.is_none());
            } else {
                let inverse = decode_hex(output_hex_le.as_ref().expect("nonzero inverse"));
                let product = slow_field_multiply(&input, &inverse, degree, modulus);
                assert_eq!(product[0], 1);
                assert!(product[1..].iter().all(|byte| *byte == 0));
            }
        }
        VectorOperation::Pow {
            base_hex_le,
            exponent_hex_le,
            output_hex_le,
        } => {
            let output = slow_pow(
                &decode_hex(base_hex_le),
                &decode_hex(exponent_hex_le),
                degree,
                modulus,
            );
            assert_eq!(encode_hex(&output), *output_hex_le);
        }
        VectorOperation::MulByX {
            input_hex_le,
            output_hex_le,
        } => {
            let mut basis_x = vec![0; degree.div_ceil(8)];
            basis_x[0] = 2;
            let output = slow_field_multiply(&decode_hex(input_hex_le), &basis_x, degree, modulus);
            assert_eq!(encode_hex(&output), *output_hex_le);
        }
    }
}

fn slow_pow(base: &[u8], exponent_le: &[u8], degree: usize, modulus: &[usize]) -> Vec<u8> {
    let mut result = vec![0; degree.div_ceil(8)];
    result[0] = 1;
    let mut power = base.to_vec();
    for byte in exponent_le {
        for bit in 0..8 {
            if byte & (1 << bit) != 0 {
                result = slow_field_multiply(&result, &power, degree, modulus);
            }
            power = slow_field_multiply(&power, &power, degree, modulus);
        }
    }
    result
}

fn slow_field_multiply(lhs: &[u8], rhs: &[u8], degree: usize, modulus: &[usize]) -> Vec<u8> {
    slow_reduce(&slow_wide_product(lhs, rhs), degree, modulus)
}

fn slow_wide_product(lhs: &[u8], rhs: &[u8]) -> Vec<u8> {
    let mut output = vec![false; (lhs.len() + rhs.len()) * 8];
    for lhs_bit in 0..lhs.len() * 8 {
        if bit(lhs, lhs_bit) {
            for rhs_bit in 0..rhs.len() * 8 {
                if bit(rhs, rhs_bit) {
                    output[lhs_bit + rhs_bit] ^= true;
                }
            }
        }
    }
    pack_bits(&output, lhs.len() + rhs.len())
}

fn slow_reduce(input: &[u8], degree: usize, modulus: &[usize]) -> Vec<u8> {
    let mut bits = unpack_bits(input);
    for source in (degree..bits.len()).rev() {
        if bits[source] {
            for exponent in modulus {
                bits[source - degree + exponent] ^= true;
            }
        }
    }
    pack_bits(&bits[..degree], degree.div_ceil(8))
}

fn bit(bytes: &[u8], index: usize) -> bool {
    bytes[index / 8] & (1 << (index % 8)) != 0
}

fn unpack_bits(bytes: &[u8]) -> Vec<bool> {
    (0..bytes.len() * 8)
        .map(|index| bit(bytes, index))
        .collect()
}

fn pack_bits(bits: &[bool], bytes: usize) -> Vec<u8> {
    let mut output = vec![0; bytes];
    for (index, value) in bits.iter().copied().enumerate() {
        if value {
            output[index / 8] |= 1 << (index % 8);
        }
    }
    output
}

fn xor(lhs: &[u8], rhs: &[u8]) -> Vec<u8> {
    lhs.iter().zip(rhs).map(|(lhs, rhs)| lhs ^ rhs).collect()
}

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("golden hex is UTF-8"), 16)
                .expect("golden is validated hexadecimal")
        })
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn assert_vector_error(error: &PipelineError, expected_path: &str) {
    match error {
        PipelineError::ReferenceVectors(error) => assert_eq!(error.path(), expected_path),
        other => panic!("expected typed reference-vector error, received {other}"),
    }
}

fn with_value(
    original: &serde_json::Value,
    pointer: &str,
    replacement: serde_json::Value,
) -> serde_json::Value {
    let mut changed = original.clone();
    *changed
        .pointer_mut(pointer)
        .expect("test JSON pointer exists") = replacement;
    changed
}

fn valid_vector_set(field_id: &str, width: usize) -> serde_json::Value {
    let zero = "00".repeat(width);
    let one = format!("01{}", "00".repeat(width - 1));
    let wide_zero = "00".repeat(width * 2);
    serde_json::json!({
        "schema": 2,
        "field_id": field_id,
        "oracle": {"name": "independent-test", "version": "1.0"},
        "generation": {
            "algorithm": "sha256-labeled-v1",
            "seed_hex": "11".repeat(32)
        },
        "vectors": [
            {
                "case": "canonical_zero",
                "operation": {"kind": "canonical", "element_hex_le": zero}
            },
            {
                "case": "canonical_one",
                "operation": {"kind": "canonical", "element_hex_le": one}
            },
            {
                "case": "add_zero",
                "operation": {
                    "kind": "add",
                    "lhs_hex_le": "00".repeat(width),
                    "rhs_hex_le": "00".repeat(width),
                    "output_hex_le": "00".repeat(width)
                }
            },
            {
                "case": "wide_product_zero",
                "operation": {
                    "kind": "wide_product",
                    "lhs_hex_le": "00".repeat(width),
                    "rhs_hex_le": "00".repeat(width),
                    "output_wide_hex_le": wide_zero
                }
            },
            {
                "case": "reduce_zero",
                "operation": {
                    "kind": "reduce",
                    "input_wide_hex_le": "00".repeat(width * 2),
                    "output_hex_le": "00".repeat(width)
                }
            },
            {
                "case": "multiply_zero",
                "operation": {
                    "kind": "multiply",
                    "lhs_hex_le": "00".repeat(width),
                    "rhs_hex_le": "00".repeat(width),
                    "output_hex_le": "00".repeat(width)
                }
            },
            {
                "case": "square_zero",
                "operation": {
                    "kind": "square",
                    "input_hex_le": "00".repeat(width),
                    "output_hex_le": "00".repeat(width)
                }
            },
            {
                "case": "invert_zero",
                "operation": {
                    "kind": "invert",
                    "input_hex_le": "00".repeat(width),
                    "output_hex_le": null
                }
            },
            {
                "case": "invert_one",
                "operation": {
                    "kind": "invert",
                    "input_hex_le": format!("01{}", "00".repeat(width - 1)),
                    "output_hex_le": format!("01{}", "00".repeat(width - 1))
                }
            },
            {
                "case": "pow_zero",
                "operation": {
                    "kind": "pow",
                    "base_hex_le": "00".repeat(width),
                    "exponent_hex_le": "01",
                    "output_hex_le": "00".repeat(width)
                }
            },
            {
                "case": "mul_by_x_zero",
                "operation": {
                    "kind": "mul_by_x",
                    "input_hex_le": "00".repeat(width),
                    "output_hex_le": "00".repeat(width)
                }
            }
        ]
    })
}

struct VectorFixture {
    _temporary: TemporaryDirectory,
    manifest: PathBuf,
    vectors: PathBuf,
    field_id: String,
}

impl VectorFixture {
    fn frozen_128(prefix: &str) -> Self {
        let temporary = TemporaryDirectory::new(prefix);
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fields")
            .join("gf2_128_v1.toml");
        Self::new(temporary, manifest)
    }

    fn degree_five(prefix: &str) -> Self {
        let temporary = TemporaryDirectory::new(prefix);
        let manifest = temporary.path().join("gf2_5.toml");
        fs::write(&manifest, degree_five_manifest()).expect("small manifest writable");
        Self::new(temporary, manifest)
    }

    fn new(temporary: TemporaryDirectory, manifest: PathBuf) -> Self {
        let field_id = Generator::default()
            .validate(&manifest)
            .expect("fixture manifest validates")
            .field_id()
            .to_string();
        let vectors = temporary.path().join("vectors.json");
        Self {
            _temporary: temporary,
            manifest,
            vectors,
            field_id,
        }
    }

    fn write_json(&self, value: &serde_json::Value) {
        fs::write(
            &self.vectors,
            serde_json::to_vec(value).expect("fixture serializes"),
        )
        .expect("vector fixture writable");
    }

    fn load(&self) -> Result<microfield::spec::model::ReferenceVectorSet, PipelineError> {
        Generator::default().vectors(&self.manifest, &JsonFileOracle::new(&self.vectors))
    }
}

fn degree_five_manifest() -> &'static str {
    "schema_version = 1\n\n\
     [field]\n\
     name = \"gf2_5\"\n\
     characteristic = 2\n\
     degree = 5\n\n\
     [field.basis]\n\
     kind = \"polynomial\"\n\
     coefficient_order = \"ascending\"\n\n\
     [field.modulus]\n\
     nonzero_exponents = [5, 2, 0]\n\n\
     [field.encoding]\n\
     byte_order = \"little\"\n\
     bit_order = \"lsb0\"\n\
     canonical_bytes = 1\n\n\
     [build]\n\
     limb_bits = 64\n\
     product_strategies = [\"schoolbook\"]\n\
     reduction_style = \"generated_fold\"\n\
     requested_backends = [\"portable\"]\n"
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new(prefix: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("{prefix}-{}-{timestamp}", std::process::id()));
        fs::create_dir(&path).expect("unique temporary directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
