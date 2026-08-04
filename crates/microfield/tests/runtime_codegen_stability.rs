//! Frozen runtime/codegen and identity compatibility contracts for H2.8.

#![cfg(feature = "generator")]

use microfield::{
    __private::{
        CURRENT_CODEGEN_ABI_VERSION, MAX_CODEGEN_ABI_VERSION, MIN_CODEGEN_ABI_VERSION,
        supports_codegen_abi,
    },
    generator::BinaryFieldFactory,
};

const ABI_MATRIX: &str = include_str!("../abi/runtime-codegen-matrix-v1.csv");

#[test]
fn runtime_and_generator_share_one_versioned_abi_contract() {
    assert_eq!(MIN_CODEGEN_ABI_VERSION, 1);
    assert_eq!(MAX_CODEGEN_ABI_VERSION, 3);
    assert_eq!(CURRENT_CODEGEN_ABI_VERSION, 3);
    assert_eq!(
        ABI_MATRIX,
        "runtime_series,min_codegen_abi,max_codegen_abi,current_codegen_abi,manifest_schema,artifact_schema,compatibility\n\
         0.1.x,1,3,3,1,1,N_and_N_minus_1_or_longer\n"
    );

    for version in MIN_CODEGEN_ABI_VERSION..=MAX_CODEGEN_ABI_VERSION {
        assert!(supports_codegen_abi(version));
    }
    for version in [0, MAX_CODEGEN_ABI_VERSION + 1, u32::MAX] {
        assert!(!supports_codegen_abi(version));
    }

    let package = package("abi_contract_name");
    assert_eq!(package.codegen_abi_version(), CURRENT_CODEGEN_ABI_VERSION);
    let source = core::str::from_utf8(package.rust_source()).expect("generated Rust is UTF-8");
    assert!(source.contains("supports_codegen_abi(3)"));
    assert!(!source.contains("__CODEGEN_ABI_VERSION__"));
}

#[test]
fn presentation_name_changes_only_bundle_and_generated_source_identity() {
    let first = package("presentation_alpha");
    let renamed = package("presentation_beta");

    assert_eq!(first.field_id(), renamed.field_id());
    assert_eq!(first.artifact_id(), renamed.artifact_id());
    assert_ne!(
        first.artifacts().bundle_digest(),
        renamed.artifacts().bundle_digest()
    );
    assert_ne!(first.package_digest(), renamed.package_digest());
    assert_ne!(first.rust_source(), renamed.rust_source());
}

fn package(name: &str) -> microfield::generator::GeneratedFieldPackage {
    BinaryFieldFactory::builder()
        .name(name)
        .degree(3)
        .modulus_exponents([3, 1, 0])
        .build()
        .expect("supported definition")
        .generate()
        .expect("x^3 + x + 1 is irreducible")
}
