//! Adversarial schema, normalization and Rabin validation tests.

#![cfg(feature = "generator")]

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use microfield::spec::{
    ValidationEngine,
    error::{ManifestError, NormalizationError, ValidationError},
    model::{FieldManifest, SCHEMA_V1_MAXIMUM_DEGREE, SCHEMA_V1_MAXIMUM_MANIFEST_BYTES},
};

#[test]
fn canonical_normalization_is_idempotent() {
    let original = fs::read_to_string(fields_directory().join("gf2_256_hh_v1.toml"))
        .expect("fixture is readable");
    let first = FieldManifest::parse_toml(&original)
        .expect("fixture parses")
        .normalize()
        .expect("fixture normalizes");
    let second = FieldManifest::parse_toml(first.canonical_toml())
        .expect("canonical TOML parses")
        .normalize()
        .expect("canonical TOML normalizes");

    assert_eq!(first, second);
    assert_eq!(first.canonical_toml(), second.canonical_toml());
    assert_eq!(first.identity_json(), second.identity_json());
}

#[test]
fn comments_order_and_duplicate_build_hints_normalize_away() {
    let source = manifest_source(5, &[5, 2, 0]).replace(
        "product_strategies = [\"schoolbook\"]",
        "product_strategies = [\"schoolbook\", \"schoolbook\"]",
    );
    let normalized = FieldManifest::parse_toml(&format!("# ignored\n{source}"))
        .expect("variant parses")
        .normalize()
        .expect("variant normalizes");

    assert_eq!(
        normalized.build().product_strategies(),
        &["schoolbook".to_owned()]
    );
    assert!(
        normalized
            .canonical_toml()
            .contains("product_strategies = [\"schoolbook\"]")
    );
}

#[test]
fn parser_rejects_oversized_input_before_toml_work() {
    let oversized = " ".repeat(SCHEMA_V1_MAXIMUM_MANIFEST_BYTES + 1);
    assert!(matches!(
        FieldManifest::parse_toml(&oversized),
        Err(ManifestError::InputTooLarge {
            actual,
            maximum: SCHEMA_V1_MAXIMUM_MANIFEST_BYTES
        }) if actual == (SCHEMA_V1_MAXIMUM_MANIFEST_BYTES + 1) as u64
    ));
}

#[test]
fn file_loader_rejects_oversized_input_before_reading_it() {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time follows Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "microfield-oversized-manifest-{}-{timestamp}.toml",
        std::process::id()
    ));
    fs::write(&path, vec![b' '; SCHEMA_V1_MAXIMUM_MANIFEST_BYTES + 1])
        .expect("oversized fixture is writable");

    let result = FieldManifest::load(&path);
    fs::remove_file(path).expect("oversized fixture is removable");

    assert!(matches!(
        result,
        Err(ManifestError::InputTooLarge {
            actual,
            maximum: SCHEMA_V1_MAXIMUM_MANIFEST_BYTES
        }) if actual == (SCHEMA_V1_MAXIMUM_MANIFEST_BYTES + 1) as u64
    ));
}

#[test]
fn parser_reports_unknown_keys_at_every_schema_level() {
    let original = manifest_source(5, &[5, 2, 0]);
    let cases = [
        (
            original.replace("schema_version = 1", "schema_version = 1\nsurprise = 1"),
            "surprise",
        ),
        (
            original.replace(
                "name = \"test_field\"",
                "name = \"test_field\"\nsurprise = 1",
            ),
            "field.surprise",
        ),
        (
            original.replace(
                "kind = \"polynomial\"",
                "kind = \"polynomial\"\nsurprise = 1",
            ),
            "field.basis.surprise",
        ),
        (
            original.replace(
                "nonzero_exponents = [5, 2, 0]",
                "nonzero_exponents = [5, 2, 0]\nsurprise = 1",
            ),
            "field.modulus.surprise",
        ),
        (
            original.replace("canonical_bytes = 1", "canonical_bytes = 1\nsurprise = 1"),
            "field.encoding.surprise",
        ),
        (
            original.replace("limb_bits = 64", "limb_bits = 64\nsurprise = 1"),
            "build.surprise",
        ),
    ];

    for (source, expected_path) in cases {
        assert!(matches!(
            FieldManifest::parse_toml(&source),
            Err(ManifestError::UnknownKey(path)) if path == expected_path
        ));
    }
}

#[test]
fn unsupported_schema_and_fixed_profile_values_are_typed() {
    let original = manifest_source(5, &[5, 2, 0]);
    assert!(matches!(
        FieldManifest::parse_toml(&original.replace("schema_version = 1", "schema_version = 2")),
        Err(ManifestError::UnsupportedSchema(2))
    ));

    let cases = [
        (
            "characteristic = 2",
            "characteristic = 3",
            "field.characteristic",
        ),
        (
            "kind = \"polynomial\"",
            "kind = \"normal\"",
            "field.basis.kind",
        ),
        (
            "coefficient_order = \"ascending\"",
            "coefficient_order = \"descending\"",
            "field.basis.coefficient_order",
        ),
        (
            "byte_order = \"little\"",
            "byte_order = \"big\"",
            "field.encoding.byte_order",
        ),
        (
            "bit_order = \"lsb0\"",
            "bit_order = \"msb0\"",
            "field.encoding.bit_order",
        ),
        ("limb_bits = 64", "limb_bits = 32", "build.limb_bits"),
        (
            "reduction_style = \"generated_fold\"",
            "reduction_style = \"division\"",
            "build.reduction_style",
        ),
        (
            "product_strategies = [\"schoolbook\"]",
            "product_strategies = [\"karatsuba\"]",
            "build.product_strategies",
        ),
        (
            "requested_backends = [\"portable\"]",
            "requested_backends = [\"pclmul\"]",
            "build.requested_backends",
        ),
    ];

    for (needle, replacement, expected_path) in cases {
        let parsed = FieldManifest::parse_toml(&original.replace(needle, replacement))
            .expect("variant is syntactically valid");
        assert!(matches!(
            parsed.normalize(),
            Err(NormalizationError::UnsupportedValue { path, .. }) if path == expected_path
        ));
    }
}

#[test]
fn invalid_names_are_rejected_exhaustively() {
    for name in [
        "",
        "_leading",
        "trailing_",
        "double__underscore",
        "Uppercase",
        "hyphen-name",
        "áccent",
    ] {
        let source = manifest_source(5, &[5, 2, 0])
            .replace("name = \"test_field\"", &format!("name = \"{name}\""));
        assert!(matches!(
            FieldManifest::parse_toml(&source)
                .expect("name remains valid TOML")
                .normalize(),
            Err(NormalizationError::InvalidName(actual)) if actual == name
        ));
    }
    let long_name = "a".repeat(65);
    let source = manifest_source(5, &[5, 2, 0])
        .replace("name = \"test_field\"", &format!("name = \"{long_name}\""));
    assert!(matches!(
        FieldManifest::parse_toml(&source)
            .expect("long name parses")
            .normalize(),
        Err(NormalizationError::InvalidName(actual)) if actual == long_name
    ));
}

#[test]
fn polynomial_shape_and_encoding_capacity_are_rejected_before_rabin() {
    let original = manifest_source(5, &[5, 2, 0]);
    let cases = [
        (original.replace("degree = 5", "degree = 1"), "field.degree"),
        (
            original.replace("canonical_bytes = 1", "canonical_bytes = 2"),
            "field.encoding.canonical_bytes",
        ),
        (
            original.replace("[5, 2, 0]", "[5, 2, 2, 0]"),
            "field.modulus.nonzero_exponents",
        ),
        (
            original.replace("[5, 2, 0]", "[6, 2, 0]"),
            "field.modulus.nonzero_exponents",
        ),
        (
            original.replace("[5, 2, 0]", "[4, 2, 0]"),
            "field.modulus.nonzero_exponents",
        ),
        (
            original.replace("[5, 2, 0]", "[5, 2, 1]"),
            "field.modulus.nonzero_exponents",
        ),
        (
            original.replace("[5, 2, 0]", "[5, 0]"),
            "field.modulus.nonzero_exponents",
        ),
    ];

    for (source, expected_path) in cases {
        assert!(matches!(
            FieldManifest::parse_toml(&source)
                .expect("shape variant parses")
                .normalize(),
            Err(NormalizationError::InvalidValue { path, .. }) if path == expected_path
        ));
    }
}

#[test]
fn schema_degree_ceiling_cannot_be_disabled_by_builder_policy() {
    let oversized = manifest_source(
        SCHEMA_V1_MAXIMUM_DEGREE + 1,
        &[SCHEMA_V1_MAXIMUM_DEGREE + 1, 1, 0],
    );
    assert!(matches!(
        FieldManifest::parse_toml(&oversized)
            .expect("oversized degree parses")
            .normalize(),
        Err(NormalizationError::InvalidValue {
            path: "field.degree",
            ..
        })
    ));

    let validator = ValidationEngine::with_maximum_degree(usize::MAX);
    assert_eq!(validator.maximum_degree(), SCHEMA_V1_MAXIMUM_DEGREE);
}

#[test]
fn caller_can_apply_a_stricter_degree_policy() {
    let normalized = FieldManifest::parse_toml(&manifest_source(5, &[5, 2, 0]))
        .expect("manifest parses")
        .normalize()
        .expect("manifest normalizes");
    assert!(matches!(
        ValidationEngine::with_maximum_degree(4).validate(normalized),
        Err(ValidationError::DegreeLimit {
            degree: 5,
            maximum: 4
        })
    ));
}

#[test]
fn rabin_matches_independent_trial_division_for_all_small_candidates() {
    let validator = ValidationEngine::default();
    for degree in 2..=8 {
        for middle in 1_u16..(1_u16 << (degree - 1)) {
            let polynomial = (1_u16 << degree) | (middle << 1) | 1;
            let exponents = set_exponents(polynomial);
            let normalized = FieldManifest::parse_toml(&manifest_source(degree, &exponents))
                .expect("enumerated manifest parses")
                .normalize()
                .expect("enumerated manifest normalizes");
            let accepted = validator.validate(normalized).is_ok();
            assert_eq!(
                accepted,
                brute_force_irreducible(polynomial, degree),
                "degree={degree}, polynomial=0b{polynomial:b}"
            );
        }
    }
}

fn brute_force_irreducible(polynomial: u16, degree: usize) -> bool {
    for divisor_degree in 1..=degree / 2 {
        for lower in 0_u16..(1_u16 << divisor_degree) {
            let divisor = (1_u16 << divisor_degree) | lower;
            if polynomial_remainder(polynomial, divisor) == 0 {
                return false;
            }
        }
    }
    true
}

fn polynomial_remainder(mut dividend: u16, divisor: u16) -> u16 {
    let divisor_degree = polynomial_degree(divisor);
    while dividend != 0 && polynomial_degree(dividend) >= divisor_degree {
        dividend ^= divisor << (polynomial_degree(dividend) - divisor_degree);
    }
    dividend
}

fn polynomial_degree(value: u16) -> usize {
    (u16::BITS - 1 - value.leading_zeros()) as usize
}

fn set_exponents(polynomial: u16) -> Vec<usize> {
    (0..u16::BITS as usize)
        .rev()
        .filter(|bit| polynomial & (1_u16 << bit) != 0)
        .collect()
}

fn manifest_source(degree: usize, exponents: &[usize]) -> String {
    let exponents = exponents
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "schema_version = 1\n\n\
         [field]\n\
         name = \"test_field\"\n\
         characteristic = 2\n\
         degree = {degree}\n\n\
         [field.basis]\n\
         kind = \"polynomial\"\n\
         coefficient_order = \"ascending\"\n\n\
         [field.modulus]\n\
         nonzero_exponents = [{exponents}]\n\n\
         [field.encoding]\n\
         byte_order = \"little\"\n\
         bit_order = \"lsb0\"\n\
         canonical_bytes = {bytes}\n\n\
         [build]\n\
         limb_bits = 64\n\
         product_strategies = [\"schoolbook\"]\n\
         reduction_style = \"generated_fold\"\n\
         requested_backends = [\"portable\"]\n",
        bytes = degree.div_ceil(8),
    )
}

fn fields_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fields")
}
