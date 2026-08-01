//! Static binary-field factory integration and adversarial tests.

#![cfg(feature = "generator")]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use microfield::{
    StaticField,
    generator::{BinaryFieldFactory, BinaryFieldFactoryError},
    spec::{
        Generator, JsonFileOracle,
        error::{PipelineError, ValidationError},
    },
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(test_name: &str) -> Self {
        let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "microfield-factory-{test_name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("unique test directory");
        Self(path)
    }
}

impl AsRef<Path> for TempDirectory {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn gf8_factory() -> BinaryFieldFactory {
    BinaryFieldFactory::builder()
        .name("gf2_3_test")
        .degree(3)
        .modulus_exponents(vec![0, 3, 1])
        .build()
        .expect("structurally valid factory")
}

#[test]
fn builder_is_deterministic_and_normalizes_exponent_order() {
    let first = gf8_factory().generate().expect("irreducible polynomial");
    let second = BinaryFieldFactory::builder()
        .name("gf2_3_test")
        .degree(3)
        .modulus_exponents(vec![3, 1, 0])
        .build()
        .expect("valid factory")
        .generate()
        .expect("irreducible polynomial");

    assert_eq!(first.field_id(), second.field_id());
    assert_eq!(first.artifact_id(), second.artifact_id());
    assert_eq!(first.package_digest(), second.package_digest());
    assert_eq!(first.rust_source(), second.rust_source());
    assert_eq!(first.type_name(), "Gf2_3Test");
    assert_eq!(first.codegen_abi_version(), 1);
    assert!(
        !first
            .rust_source()
            .windows(6)
            .any(|window| window == b"unsafe")
    );
    assert!(
        !first
            .rust_source()
            .windows(7)
            .any(|window| window == b"dyn Tra")
    );
}

#[test]
fn adversarial_names_never_become_rust_tokens_or_paths() {
    for name in [
        "escape/path",
        "field\"\nconst INJECTED: bool = true;\n\"",
        "../field",
        "field-name",
        "_private",
    ] {
        let factory = BinaryFieldFactory::builder()
            .name(name)
            .degree(3)
            .modulus_exponents(vec![3, 1, 0])
            .build()
            .expect("TOML quoting keeps builder input data-only");
        assert!(factory.generate().is_err(), "name `{name}` was accepted");
    }
}

#[test]
fn reducible_modulus_is_rejected_before_source_exists() {
    let error = BinaryFieldFactory::builder()
        .name("reducible")
        .degree(4)
        .modulus_exponents(vec![4, 2, 0])
        .build()
        .expect("structurally valid")
        .generate()
        .expect_err("(x^2 + x + 1)^2 is reducible");
    assert!(matches!(
        error,
        BinaryFieldFactoryError::Pipeline(PipelineError::Validation(
            ValidationError::ReduciblePolynomial { .. } | ValidationError::FrobeniusMismatch { .. }
        ))
    ));
}

#[test]
fn configured_degree_limit_is_enforced_by_the_shared_validator() {
    let error = BinaryFieldFactory::builder()
        .name("gf2_9_limited")
        .degree(9)
        .modulus_exponents(vec![9, 4, 0])
        .maximum_degree(8)
        .build()
        .expect("structurally valid")
        .generate()
        .expect_err("policy must reject degree nine");
    assert!(matches!(
        error,
        BinaryFieldFactoryError::Pipeline(PipelineError::Validation(
            ValidationError::DegreeLimit {
                degree: 9,
                maximum: 8
            }
        ))
    ));
}

#[test]
fn emit_is_repeatable_and_publishes_only_complete_source() {
    let directory = TempDirectory::new("atomic");
    let package = gf8_factory().generate().expect("valid package");
    let target = package.emit_rust(&directory).expect("first publication");
    assert_eq!(
        fs::read(&target).expect("published source"),
        package.rust_source()
    );

    fs::write(&target, b"obsolete").expect("replace fixture target");
    let repeated = package.emit_rust(&directory).expect("atomic replacement");
    assert_eq!(repeated, target);
    assert_eq!(
        fs::read(&target).expect("replaced source"),
        package.rust_source()
    );
    let entries = fs::read_dir(&directory)
        .expect("read output")
        .collect::<Result<Vec<_>, _>>()
        .expect("valid entries");
    assert_eq!(entries.len(), 1, "no staging residue may remain");
}

#[cfg(unix)]
#[test]
fn emit_rejects_symlink_targets_without_touching_their_destination() {
    use std::os::unix::fs::symlink;

    let directory = TempDirectory::new("symlink");
    let victim = directory.as_ref().join("victim.rs");
    fs::write(&victim, b"keep me").expect("victim fixture");
    let target = directory.as_ref().join("gf2_3_test.rs");
    symlink(&victim, &target).expect("symlink fixture");

    assert!(
        gf8_factory()
            .generate()
            .expect("valid package")
            .emit_rust(&directory)
            .is_err()
    );
    assert_eq!(fs::read(victim).expect("victim remains"), b"keep me");
}

#[cfg(unix)]
#[test]
fn emit_rejects_a_symlink_as_the_output_root() {
    use std::os::unix::fs::symlink;

    let directory = TempDirectory::new("symlink-root");
    let real_output = directory.as_ref().join("real");
    fs::create_dir(&real_output).expect("real output fixture");
    let linked_output = directory.as_ref().join("linked");
    symlink(&real_output, &linked_output).expect("linked output fixture");

    assert!(
        gf8_factory()
            .generate()
            .expect("valid package")
            .emit_rust(linked_output)
            .is_err()
    );
    assert_eq!(
        fs::read_dir(real_output)
            .expect("real output remains readable")
            .count(),
        0
    );
}

#[cfg(feature = "builtin-fields")]
#[test]
fn maintained_presets_have_the_same_identity_through_the_factory() {
    use microfield::{Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1};

    let manifest_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fields");
    for (file, expected) in [
        ("gf2_128_v1.toml", Gf2_128V1::spec().field_id()),
        ("gf2_256_hh_v1.toml", Gf2_256HhV1::spec().field_id()),
        ("gf2_256_alt_v1.toml", Gf2_256AltV1::spec().field_id()),
    ] {
        let package = BinaryFieldFactory::from_manifest(manifest_root.join(file))
            .expect("maintained manifest")
            .generate()
            .expect("maintained modulus is certified");
        assert_eq!(package.field_id(), expected, "identity drift in {file}");
    }
}

#[test]
fn external_degree_233_sage_vectors_match_the_strict_oracle_contract() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-fixtures/external-consumer");
    let vectors = Generator::default()
        .vectors(
            fixture.join("field_233.toml"),
            &JsonFileOracle::new(fixture.join("reference-vectors/gf2_233_fixture.json")),
        )
        .expect("committed external vectors pass the strict schema");
    assert_eq!(vectors.oracle().name(), "SageMath");
    assert_eq!(vectors.oracle().version(), "10.7");
    assert_eq!(vectors.vectors().len(), 11);
}
