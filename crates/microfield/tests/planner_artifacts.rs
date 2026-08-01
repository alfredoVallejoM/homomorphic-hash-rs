//! Structural, golden and cross-file contracts for plans and artifacts.

#![cfg(feature = "generator")]

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use microfield::spec::{
    ArtifactGenerator, Generator,
    error::GenerationError,
    model::{
        ExponentiationStep, FieldManifest, FoldStep, GeneratedArtifacts, PortableDegreeClass,
        PortableReductionStrategy,
    },
};
use sha2::{Digest, Sha256};

#[derive(serde::Deserialize, serde::Serialize)]
struct TestBundle {
    files: Vec<TestBundleFile>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct TestBundleFile {
    path: String,
    bytes: usize,
    sha256: String,
}

const GOLDEN: [(&str, &str, &str, &str); 3] = [
    (
        "gf2_128_v1.toml",
        "377b8f5402b057dae37ecdad1be47259d8e24555a794faa46535ecae298b558c",
        "07545484d4a09b1d44c25d0a0042042046396f9b5e2467bc5b6b0d7a2c327220",
        "53f5a53218beb44d45b81982eb6235b10f0ad847fa7a6f81697780594925a1bb",
    ),
    (
        "gf2_256_alt_v1.toml",
        "ef3d67b5ff9df3c21fb529aee76759f87d16290082dca22171a8d8e705ff3bc3",
        "f4a06836f946c87f3fda8f23889670e9182e3b23086cc7108a20879e3a5999e8",
        "5087cd8431c2edbe4982261147bd21328fe40358ebdd9c72e36e7d308e6d686c",
    ),
    (
        "gf2_256_hh_v1.toml",
        "b21097ca93e5e041b2059ba48a8b1017c3d77b031c485805780ecab6e3296544",
        "476cb23704fa07610dfdaad7b662c365208583f9a05e61e3e2809f96da9851f3",
        "af60eebc43cc43f96331e589acecc71d7d360df47978317c2a00ffaf4413b77f",
    ),
];

#[test]
fn product_reduction_and_identity_plans_have_frozen_shapes() {
    let generator = Generator::default();
    for (file, artifact_id, proof_digest, _) in GOLDEN {
        let validated = generator
            .validate(fields_directory().join(file))
            .expect("frozen manifest validates");
        let plan = generator.plan(&validated).expect("plan derives");
        let descriptor = validated.normalized().descriptor();
        let degree = descriptor.degree();

        assert_eq!(plan.artifact_id().to_string(), artifact_id);
        assert_eq!(plan.product().limb_bits(), 64);
        assert_eq!(plan.product().input_limbs(), degree.div_ceil(64));
        assert_eq!(plan.product().wide_limbs(), degree.div_ceil(64) * 2);
        assert_eq!(plan.product().strategies(), &["schoolbook".to_owned()]);
        let optimized = plan.portable_optimization();
        assert_eq!(
            optimized.degree_class(),
            PortableDegreeClass::PowerOfTwoLimbAligned
        );
        assert_eq!(
            optimized.reduction(),
            PortableReductionStrategy::LowTailFold
        );
        assert_eq!(optimized.multiplication(), "set-bit-schoolbook-v1");
        assert_eq!(optimized.squaring(), "bit-spread-v1");
        assert_eq!(optimized.inversion(), "itoh-tsujii-binary-v1");
        assert_eq!(
            optimized.modulus_terms(),
            descriptor.modulus_exponents().len()
        );

        let reduction = plan.reduction();
        assert_eq!(reduction.input_bits(), degree * 2);
        assert_eq!(reduction.output_bits(), degree);
        assert_eq!(reduction.steps().len(), degree - 1);
        assert_eq!(
            reduction.steps().first().map(FoldStep::source_bit),
            Some(degree * 2 - 2)
        );
        assert_eq!(
            reduction.steps().last().map(FoldStep::source_bit),
            Some(degree)
        );
        assert_eq!(reduction.proof_digest(), proof_digest);
        for step in reduction.steps() {
            assert_eq!(
                step.xor_targets().len(),
                descriptor.modulus_exponents().len() - 1
            );
            assert!(
                step.xor_targets()
                    .iter()
                    .all(|target| *target < step.source_bit())
            );
        }
    }
}

#[test]
fn inversion_schedule_reaches_exact_fermat_exponent() {
    let generator = Generator::default();
    for (file, _, _, _) in GOLDEN {
        let validated = generator
            .validate(fields_directory().join(file))
            .expect("frozen manifest validates");
        let plan = generator.plan(&validated).expect("plan derives");
        let degree = validated.normalized().descriptor().degree();
        let mut exponent_le_bits = vec![true];

        for step in plan.inversion().steps() {
            match step {
                ExponentiationStep::Square { count } => {
                    for _ in 0..*count {
                        exponent_le_bits.insert(0, false);
                    }
                }
                ExponentiationStep::MultiplyBase => add_one(&mut exponent_le_bits),
            }
        }

        assert_eq!(plan.inversion().steps().len(), degree * 2 - 3);
        assert_eq!(exponent_le_bits.len(), degree);
        assert!(!exponent_le_bits[0]);
        assert!(exponent_le_bits[1..].iter().all(|bit| *bit));
    }
}

#[test]
fn generated_repository_artifacts_equal_clean_in_memory_generation() {
    let generator = Generator::default();
    for (file, _, _, bundle_digest) in GOLDEN {
        let manifest_path = fields_directory().join(file);
        let artifacts = generator
            .generate(&manifest_path)
            .expect("generation succeeds");
        assert_eq!(artifacts.files().len(), 7);
        assert_eq!(artifacts.bundle_digest().to_string(), bundle_digest);
        assert_artifact_directory(&artifacts, &artifacts_directory());
        assert_cross_file_identity(&artifacts);
    }
}

#[test]
fn generation_is_deterministic_across_independent_runs() {
    let generator = Generator::default();
    for (file, _, _, _) in GOLDEN {
        let manifest = fields_directory().join(file);
        let first = generator.generate(&manifest).expect("first generation");
        let second = generator.generate(&manifest).expect("second generation");
        assert_eq!(first, second);
    }
}

#[test]
fn bundle_manifest_independently_authenticates_every_payload_file() {
    let generator = Generator::default();
    for (file, _, _, expected_bundle_digest) in GOLDEN {
        let artifacts = generator
            .generate(fields_directory().join(file))
            .expect("generation succeeds");
        let files = files_by_name(&artifacts);
        let bundle_value: serde_json::Value =
            serde_json::from_slice(files["bundle.json"]).expect("bundle JSON");
        let bundle: TestBundle =
            serde_json::from_slice(files["bundle.json"]).expect("typed bundle JSON");
        let entries = &bundle.files;
        assert_eq!(entries.len(), files.len() - 1);

        for entry in entries {
            let contents = files[entry.path.as_str()];
            assert_eq!(entry.bytes, contents.len());
            assert_eq!(entry.sha256, raw_sha256_hex(contents));
        }

        let descriptor =
            serde_json::to_vec(entries).expect("bundle descriptor serializes canonically");
        let mut hasher = Sha256::new();
        hasher.update(b"microfield:artifact-bundle:v1\0");
        hasher.update(descriptor);
        assert_eq!(hex(&hasher.finalize()), expected_bundle_digest);
        assert_eq!(bundle_value["bundle_digest"], expected_bundle_digest);
    }
}

#[test]
fn presentation_name_changes_files_but_not_semantic_or_representation_ids() {
    let source =
        fs::read_to_string(fields_directory().join("gf2_128_v1.toml")).expect("fixture readable");
    let renamed = source.replace("name = \"gf2_128_v1\"", "name = \"renamed_field\"");
    let generator = Generator::default();
    let original = validated_from_source(&source);
    let renamed = validated_from_source(&renamed);
    let original_plan = generator.plan(&original).expect("original plan");
    let renamed_plan = generator.plan(&renamed).expect("renamed plan");
    let original_files = ArtifactGenerator
        .generate(&original, &original_plan)
        .expect("original artifacts");
    let renamed_files = ArtifactGenerator
        .generate(&renamed, &renamed_plan)
        .expect("renamed artifacts");

    assert_eq!(original.field_id(), renamed.field_id());
    assert_eq!(original_plan.artifact_id(), renamed_plan.artifact_id());
    assert_ne!(
        original_files.bundle_digest(),
        renamed_files.bundle_digest()
    );
    assert_ne!(original_files.field_name(), renamed_files.field_name());
    assert_ne!(original_files, renamed_files);

    let original_map = files_by_name(&original_files);
    let renamed_map = files_by_name(&renamed_files);
    for unchanged in [
        "certificate.json",
        "descriptor.json",
        "generation-plan.json",
    ] {
        assert_eq!(original_map[unchanged], renamed_map[unchanged]);
    }
    for changed in [
        "bundle.json",
        "field.rs",
        "metadata.json",
        "normalized.toml",
    ] {
        assert_ne!(original_map[changed], renamed_map[changed]);
    }
}

#[test]
fn artifact_renderer_rejects_a_plan_from_another_field() {
    let generator = Generator::default();
    let field_128 = generator
        .validate(fields_directory().join("gf2_128_v1.toml"))
        .expect("128-bit manifest validates");
    let field_256 = generator
        .validate(fields_directory().join("gf2_256_hh_v1.toml"))
        .expect("256-bit manifest validates");
    let wrong_plan = generator.plan(&field_256).expect("plan derives");
    assert!(matches!(
        ArtifactGenerator.generate(&field_128, &wrong_plan),
        Err(GenerationError::MismatchedPlan)
    ));
}

fn add_one(bits: &mut Vec<bool>) {
    for bit in bits.iter_mut() {
        *bit = !*bit;
        if *bit {
            return;
        }
    }
    bits.push(true);
}

fn assert_artifact_directory(artifacts: &GeneratedArtifacts, root: &Path) {
    let directory = root.join(artifacts.field_name());
    let actual_names = fs::read_dir(&directory)
        .expect("artifact directory readable")
        .map(|entry| {
            entry
                .expect("artifact entry readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<BTreeSet<_>>();
    let expected_names = artifacts
        .files()
        .iter()
        .map(|file| file.relative_path().to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_names, expected_names);
    for file in artifacts.files() {
        assert_eq!(
            fs::read(directory.join(file.relative_path())).expect("artifact readable"),
            file.contents()
        );
    }
}

fn assert_cross_file_identity(artifacts: &GeneratedArtifacts) {
    let files = files_by_name(artifacts);
    let metadata: serde_json::Value =
        serde_json::from_slice(files["metadata.json"]).expect("metadata JSON");
    let plan: serde_json::Value =
        serde_json::from_slice(files["generation-plan.json"]).expect("plan JSON");
    let certificate: serde_json::Value =
        serde_json::from_slice(files["certificate.json"]).expect("certificate JSON");
    let descriptor: serde_json::Value =
        serde_json::from_slice(files["descriptor.json"]).expect("descriptor JSON");
    let bundle: serde_json::Value =
        serde_json::from_slice(files["bundle.json"]).expect("bundle JSON");

    assert_eq!(metadata["field_name"], artifacts.field_name());
    assert_eq!(metadata["field_id"], artifacts.field_id().to_string());
    assert_eq!(metadata["artifact_id"], artifacts.artifact_id().to_string());
    assert_eq!(plan["field_id"], metadata["field_id"]);
    assert_eq!(plan["artifact_id"], metadata["artifact_id"]);
    assert_eq!(certificate["field_id"], metadata["field_id"]);
    assert_eq!(bundle["artifact_id"], metadata["artifact_id"]);
    assert_eq!(
        bundle["bundle_digest"],
        artifacts.bundle_digest().to_string()
    );
    assert_eq!(bundle["files"].as_array().expect("file array").len(), 6);
    assert_eq!(
        descriptor["degree"].as_u64(),
        plan["reduction"]["output_bits"].as_u64()
    );
}

fn files_by_name(artifacts: &GeneratedArtifacts) -> BTreeMap<&str, &[u8]> {
    artifacts
        .files()
        .iter()
        .map(|file| (file.relative_path(), file.contents()))
        .collect()
}

fn validated_from_source(source: &str) -> microfield::spec::model::ValidatedFieldSpec {
    let normalized = FieldManifest::parse_toml(source)
        .expect("manifest parses")
        .normalize()
        .expect("manifest normalizes");
    microfield::spec::ValidationEngine::default()
        .validate(normalized)
        .expect("manifest validates")
}

fn raw_sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn fields_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fields")
}

fn artifacts_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("artifacts")
}
