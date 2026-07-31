//! End-to-end contracts for the generator milestone.

#![cfg(feature = "generator")]

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use microfield::spec::{
    FileSystemArtifactSink, Generator, JsonFileOracle, ValidationEngine,
    error::{ManifestError, NormalizationError, PipelineError, ValidationError},
    model::FieldManifest,
};

const EXPECTED_FIELDS: [(&str, &str); 3] = [
    (
        "gf2_128_v1.toml",
        "4825b6d5606e34af32722a4a6a96d04a1e21337be0fb734adb9c69f9b9d77d31",
    ),
    (
        "gf2_256_alt_v1.toml",
        "5c78ea2f9ea1b2d59b88bf32e38ae33be4c2f977f0232c4441f7a16e4c9bb54d",
    ),
    (
        "gf2_256_hh_v1.toml",
        "6b62fea68b968fd4f8c39a4f69b78f714c80858b1d0f667ec5a63d4417b43ca8",
    ),
];

#[test]
fn frozen_manifests_have_golden_identities_and_valid_certificates() {
    let generator = Generator::default();
    for (file, expected_id) in EXPECTED_FIELDS {
        let validated = generator
            .validate(fields_directory().join(file))
            .expect("frozen manifest must validate");
        assert_eq!(validated.field_id().to_string(), expected_id);
        assert_eq!(
            validated.certificate().field_id(),
            validated.field_id(),
            "certificate must bind the exact semantic identity"
        );
        assert_eq!(
            validated
                .certificate()
                .irreducibility()
                .final_residue_hex_le(),
            format!(
                "02{}",
                "00".repeat(validated.normalized().descriptor().canonical_bytes() - 1)
            )
        );
    }
}

#[test]
fn identity_json_has_fixed_order_and_no_trailing_newline() {
    let normalized = FieldManifest::load(fields_directory().join("gf2_256_hh_v1.toml"))
        .expect("manifest must parse")
        .normalize()
        .expect("manifest must normalize");
    assert_eq!(
        normalized.identity_json(),
        "{\"schema\":1,\"characteristic\":\"2\",\"degree\":256,\
         \"basis\":{\"kind\":\"polynomial\",\"coefficient_order\":\"ascending\"},\
         \"modulus\":[256,10,5,2,0],\
         \"encoding\":{\"byte_order\":\"little\",\"bit_order\":\"lsb0\",\"bytes\":32}}"
    );
    assert!(!normalized.identity_json().ends_with('\n'));
}

#[test]
fn source_order_and_name_do_not_change_field_identity() {
    let original = read_manifest("gf2_256_hh_v1.toml");
    let reordered = original
        .replace("name = \"gf2_256_hh_v1\"", "name = \"renamed_field\"")
        .replace(
            "nonzero_exponents = [256, 10, 5, 2, 0]",
            "nonzero_exponents = [0, 5, 256, 2, 10]",
        );
    let validator = ValidationEngine::default();
    let original = validator
        .validate(
            FieldManifest::parse_toml(&original)
                .expect("original parses")
                .normalize()
                .expect("original normalizes"),
        )
        .expect("original validates");
    let reordered = validator
        .validate(
            FieldManifest::parse_toml(&reordered)
                .expect("variant parses")
                .normalize()
                .expect("variant normalizes"),
        )
        .expect("variant validates");
    assert_eq!(original.field_id(), reordered.field_id());
}

#[test]
fn unimplemented_build_strategy_is_rejected_before_planning() {
    let original = read_manifest("gf2_128_v1.toml");
    let alternative = original.replace(
        "product_strategies = [\"schoolbook\"]",
        "product_strategies = [\"karatsuba\", \"schoolbook\"]",
    );
    assert!(matches!(
        FieldManifest::parse_toml(&alternative)
            .expect("alternative parses")
            .normalize(),
        Err(NormalizationError::UnsupportedValue {
            path: "build.product_strategies",
            value
        }) if value == "karatsuba"
    ));
}

#[test]
fn strict_schema_reports_the_complete_unknown_key_path() {
    let source = read_manifest("gf2_128_v1.toml").replace(
        "canonical_bytes = 16",
        "canonical_bytes = 16\nsurprise = true",
    );
    let error = FieldManifest::parse_toml(&source).expect_err("unknown key must fail");
    assert!(matches!(
        error,
        ManifestError::UnknownKey(ref path) if path == "field.encoding.surprise"
    ));
}

#[test]
fn reducible_modulus_never_reaches_planning() {
    let source = read_manifest("gf2_128_v1.toml").replace(
        "nonzero_exponents = [128, 7, 2, 1, 0]",
        "nonzero_exponents = [128, 2, 0]",
    );
    let normalized = FieldManifest::parse_toml(&source)
        .expect("structurally valid manifest")
        .normalize()
        .expect("structurally valid normalization");
    let error = ValidationEngine::default()
        .validate(normalized)
        .expect_err("a polynomial square is reducible");
    assert!(matches!(
        error,
        ValidationError::ReduciblePolynomial { .. } | ValidationError::FrobeniusMismatch { .. }
    ));
}

#[test]
fn artifact_publication_is_reproducible_and_replaces_as_one_unit() {
    let temporary = TemporaryDirectory::new("microfield-generator-test");
    let sink = FileSystemArtifactSink::new(temporary.path());
    let manifest = fields_directory().join("gf2_256_hh_v1.toml");
    let generator = Generator::default();

    let first = generator
        .emit(&manifest, &sink)
        .expect("first publication succeeds");
    assert!(!first.replaced_existing());
    assert!(generator.check(&manifest, &sink).expect("check succeeds"));

    let metadata = first.output_directory().join("metadata.json");
    fs::write(&metadata, b"corrupted\n").expect("test can corrupt owned temporary output");
    assert!(
        !generator
            .check(&manifest, &sink)
            .expect("drift check succeeds")
    );

    let second = generator
        .emit(&manifest, &sink)
        .expect("replacement succeeds");
    assert!(second.replaced_existing());
    assert!(
        generator
            .check(&manifest, &sink)
            .expect("clean check succeeds")
    );
}

#[test]
fn pipeline_preserves_typed_manifest_errors() {
    let error = Generator::default()
        .normalize(fields_directory().join("missing.toml"))
        .expect_err("missing input must fail");
    assert!(matches!(
        error,
        PipelineError::Manifest(ManifestError::Read { .. })
    ));
}

#[test]
fn imported_oracle_vectors_are_bound_to_schema_identity_and_encoding() {
    let temporary = TemporaryDirectory::new("microfield-oracle-test");
    let manifest = fields_directory().join("gf2_128_v1.toml");
    let generator = Generator::default();
    let validated = generator.validate(&manifest).expect("manifest validates");
    let vectors_path = temporary.path().join("vectors.json");
    let valid = valid_vector_set(&validated.field_id().to_string(), 16);
    fs::write(
        &vectors_path,
        serde_json::to_vec(&valid).expect("fixture serializes"),
    )
    .expect("test vector fixture is writable");
    let vectors = generator
        .vectors(&manifest, &JsonFileOracle::new(&vectors_path))
        .expect("matching oracle envelope is accepted");
    assert_eq!(vectors.oracle().name(), "independent-test");
    assert_eq!(vectors.oracle().version(), "1.0");

    let mismatch = with_value(&valid, "/field_id", serde_json::json!("00".repeat(32)));
    fs::write(
        &vectors_path,
        serde_json::to_vec(&mismatch).expect("fixture serializes"),
    )
    .expect("fixture can be replaced");
    assert!(matches!(
        generator.vectors(&manifest, &JsonFileOracle::new(&vectors_path)),
        Err(PipelineError::ReferenceVectors(_))
    ));
}

#[test]
fn oracle_envelope_rejects_all_malformed_identity_and_encoding_cases() {
    let temporary = TemporaryDirectory::new("microfield-oracle-adversarial");
    let manifest = fields_directory().join("gf2_128_v1.toml");
    let generator = Generator::default();
    let field_id = generator
        .validate(&manifest)
        .expect("manifest validates")
        .field_id()
        .to_string();
    let vectors_path = temporary.path().join("vectors.json");
    let base = valid_vector_set(&field_id, 16);

    let mut cases = vec![
        with_value(&base, "/schema", serde_json::json!(1)),
        with_value(&base, "/field_id", serde_json::json!("00".repeat(32))),
        with_value(&base, "/oracle/name", serde_json::json!(" ")),
        with_value(&base, "/oracle/version", serde_json::json!("")),
        with_value(&base, "/generation/algorithm", serde_json::json!("unknown")),
        with_value(&base, "/generation/seed_hex", serde_json::json!("00")),
        with_value(&base, "/vectors", serde_json::json!([])),
        with_value(&base, "/vectors/0/case", serde_json::json!("Bad-Name")),
        with_value(
            &base,
            "/vectors/0/operation/element_hex_le",
            serde_json::json!("00"),
        ),
        with_value(
            &base,
            "/vectors/1/operation/element_hex_le",
            serde_json::json!("AA".repeat(16)),
        ),
        with_value(
            &base,
            "/vectors/1/operation/element_hex_le",
            serde_json::json!("gg".repeat(16)),
        ),
    ];
    let mut unknown_key = base.clone();
    unknown_key["vectors"][0]["operation"]["unknown"] = serde_json::json!(true);
    cases.push(unknown_key);

    for (index, invalid) in cases.into_iter().enumerate() {
        fs::write(
            &vectors_path,
            serde_json::to_vec(&invalid).expect("fixture serializes"),
        )
        .expect("fixture writable");
        assert!(
            generator
                .vectors(&manifest, &JsonFileOracle::new(&vectors_path))
                .is_err(),
            "malformed oracle case {index} was accepted"
        );
    }
}

#[allow(clippy::too_many_lines)]
fn valid_vector_set(field_id: &str, width: usize) -> serde_json::Value {
    let zero = "00".repeat(width);
    let one = format!("01{}", "00".repeat(width - 1));
    let wide_zero = "00".repeat(width * 2);
    serde_json::json!({
        "schema": 2,
        "field_id": field_id,
        "oracle": {
            "name": "independent-test",
            "version": "1.0"
        },
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

#[test]
fn reduction_plans_match_reference_long_division() {
    let generator = Generator::default();
    for (file, _) in EXPECTED_FIELDS {
        let validated = generator
            .validate(fields_directory().join(file))
            .expect("frozen manifest validates");
        let plan = generator.plan(&validated).expect("plan derives");
        let descriptor = validated.normalized().descriptor();
        let degree = descriptor.degree();
        let modulus = descriptor.modulus_exponents();

        for pattern in 0..4 {
            let mut planned = vec![false; degree * 2];
            for (bit, value) in planned.iter_mut().enumerate().take(degree * 2 - 1) {
                *value = match pattern {
                    0 => bit == degree,
                    1 => bit == degree * 2 - 2,
                    2 => bit >= degree && bit % 3 == 0,
                    _ => bit % 5 == 1,
                };
            }
            let mut reference = planned.clone();
            apply_plan(&mut planned, plan.reduction().steps());
            reduce_reference(&mut reference, degree, modulus);
            assert_eq!(planned, reference, "{file}, pattern {pattern}");
        }
    }
}

fn apply_plan(bits: &mut [bool], steps: &[microfield::spec::model::FoldStep]) {
    for step in steps {
        let source = step.source_bit();
        if bits[source] {
            bits[source] = false;
            for &target in step.xor_targets() {
                bits[target] ^= true;
            }
        }
    }
}

fn reduce_reference(bits: &mut [bool], degree: usize, modulus: &[usize]) {
    for source in (degree..bits.len()).rev() {
        if bits[source] {
            for &exponent in modulus {
                bits[source - degree + exponent] ^= true;
            }
        }
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

fn fields_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fields")
}

fn read_manifest(file: &str) -> String {
    fs::read_to_string(fields_directory().join(file)).expect("fixture manifest must be readable")
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
