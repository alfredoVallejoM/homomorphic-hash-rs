//! Reproducibility and identity tests for maintained prime artifacts.

#![cfg(all(feature = "generator", feature = "prime-fields"))]

use microfield::{Fp251V1, Fp256GenericV1, FpGoldilocks64V1, StaticField};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

const FP251_IDENTITY: &str = "{\"schema\":2,\"characteristic\":\"251\",\"degree\":1,\"basis\":{\"kind\":\"prime\"},\"modulus\":\"251\",\"encoding\":{\"byte_order\":\"little\",\"integer\":\"canonical\",\"bytes\":1}}";
const GOLD_IDENTITY: &str = "{\"schema\":2,\"characteristic\":\"18446744069414584321\",\"degree\":1,\"basis\":{\"kind\":\"prime\"},\"modulus\":\"18446744069414584321\",\"encoding\":{\"byte_order\":\"little\",\"integer\":\"canonical\",\"bytes\":8}}";
const FP256_IDENTITY: &str = "{\"schema\":2,\"characteristic\":\"71319327679048415160211920703270965766974670828100238494590001805011376932671\",\"degree\":1,\"basis\":{\"kind\":\"prime\"},\"modulus\":\"71319327679048415160211920703270965766974670828100238494590001805011376932671\",\"encoding\":{\"byte_order\":\"little\",\"integer\":\"canonical\",\"bytes\":32}}";

const FP251_ARTIFACT: &str = "{\"schema\":1,\"field_id\":\"aef78c79e5e5e929ee046a199df8eab46633a4ea7cabf66480fe2d7909d678da\",\"representation\":{\"kind\":\"canonical\"},\"reduction\":{\"kind\":\"native\",\"word_bits\":16},\"inversion\":{\"kind\":\"fixed-exponent\",\"exponent\":\"249\"},\"codegen_abi\":3}";
const GOLD_ARTIFACT: &str = "{\"schema\":1,\"field_id\":\"db27c832ee2b9e87ae66e00657a20cf705132730f5ac43e3f7031f9bb1e272ac\",\"representation\":{\"kind\":\"canonical\"},\"reduction\":{\"kind\":\"barrett\",\"limb_bits\":64,\"limbs\":1,\"approximation_shift\":64,\"correction_steps_max\":2},\"inversion\":{\"kind\":\"fixed-exponent\",\"exponent\":\"18446744069414584319\"},\"codegen_abi\":3}";
const FP256_ARTIFACT: &str = "{\"schema\":1,\"field_id\":\"60cbdb42c3d6efbc7158144f6a42d015a708ca15ae47e5156204660f97681e8e\",\"representation\":{\"kind\":\"montgomery\",\"radix_bits\":64,\"limbs\":4},\"reduction\":{\"kind\":\"montgomery-cios\",\"neg_inv\":\"547978e477709741\"},\"inversion\":{\"kind\":\"fixed-exponent\",\"exponent\":\"71319327679048415160211920703270965766974670828100238494590001805011376932669\"},\"codegen_abi\":3}";

#[test]
fn field_ids_derive_only_from_prime_semantics_and_encoding() {
    assert_eq!(
        Fp251V1::spec().field_id().into_bytes(),
        digest(b"microfield:field-id:v1\0", FP251_IDENTITY.as_bytes())
    );
    assert_eq!(
        FpGoldilocks64V1::spec().field_id().into_bytes(),
        digest(b"microfield:field-id:v1\0", GOLD_IDENTITY.as_bytes())
    );
    assert_eq!(
        Fp256GenericV1::spec().field_id().into_bytes(),
        digest(b"microfield:field-id:v1\0", FP256_IDENTITY.as_bytes())
    );
    for identity in [FP251_IDENTITY, GOLD_IDENTITY, FP256_IDENTITY] {
        assert!(!identity.contains("name"));
        assert!(!identity.contains("montgomery"));
        assert!(!identity.contains("solinas"));
    }
}

#[test]
fn artifact_ids_bind_representation_reduction_and_inversion_plan() {
    assert_eq!(
        Fp251V1::spec().artifact_id().into_bytes(),
        digest(b"microfield:artifact-id:v1\0", FP251_ARTIFACT.as_bytes())
    );
    assert_eq!(
        FpGoldilocks64V1::spec().artifact_id().into_bytes(),
        digest(b"microfield:artifact-id:v1\0", GOLD_ARTIFACT.as_bytes())
    );
    assert_eq!(
        Fp256GenericV1::spec().artifact_id().into_bytes(),
        digest(b"microfield:artifact-id:v1\0", FP256_ARTIFACT.as_bytes())
    );
}

#[test]
fn embedded_descriptors_and_certificates_are_well_formed_and_bound() {
    for spec in [
        Fp251V1::spec(),
        FpGoldilocks64V1::spec(),
        Fp256GenericV1::spec(),
    ] {
        let descriptor: serde_json::Value = serde_json::from_slice(spec.descriptor_json()).unwrap();
        let certificate: serde_json::Value =
            serde_json::from_slice(spec.certificate_json()).unwrap();
        assert_eq!(descriptor["schema"], 2);
        assert_eq!(
            descriptor["characteristic"],
            spec.characteristic().decimal()
        );
        assert_eq!(certificate["field_id"], spec.field_id().to_string());
        assert_eq!(certificate["validator"], "microfield-prime-v1");
    }
    assert_eq!(microfield::verify_builtin_prime_certificates(), Ok(()));
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactManifest {
    schema: u32,
    artifact_id: String,
    bundle_digest: String,
    files: Vec<BundleFile>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BundleFile {
    path: String,
    bytes: usize,
    sha256: String,
}

#[test]
fn prime_bundles_authenticate_every_payload_and_their_order() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts");
    for (directory, expected_artifact) in [
        ("fp251_v1", Fp251V1::spec().artifact_id().to_string()),
        (
            "fp_goldilocks64_v1",
            FpGoldilocks64V1::spec().artifact_id().to_string(),
        ),
        (
            "fp256_generic_v1",
            Fp256GenericV1::spec().artifact_id().to_string(),
        ),
    ] {
        let directory = root.join(directory);
        let bundle: ArtifactManifest =
            serde_json::from_slice(&std::fs::read(directory.join("bundle.json")).unwrap()).unwrap();
        assert_eq!(bundle.schema, 1);
        assert_eq!(bundle.artifact_id, expected_artifact);
        assert_eq!(bundle.files.len(), 3);
        let mut previous = "";
        for file in &bundle.files {
            assert!(file.path.as_str() > previous, "bundle paths must be sorted");
            previous = &file.path;
            let payload = std::fs::read(directory.join(&file.path)).unwrap();
            assert_eq!(payload.len(), file.bytes);
            assert_eq!(hex_digest(&payload), file.sha256);
        }
        let descriptor = serde_json::to_vec(&bundle.files).unwrap();
        assert_eq!(
            hex(&digest(b"microfield:artifact-bundle:v1\0", &descriptor)),
            bundle.bundle_digest
        );
    }
}

fn digest(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(bytes);
    hasher.finalize().into()
}

fn hex_digest(bytes: &[u8]) -> String {
    hex(&digest(&[], bytes))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}
