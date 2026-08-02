//! Phase 5 reproducibility, lock, cache and consumer compilation tests.

#![cfg(feature = "generator")]

use std::{
    fmt::Write as _,
    fs,
    path::PathBuf,
    process::Command,
    sync::{Arc, Barrier},
};

use microfield::{ValidationAssurance, generator::*};

fn temporary_directory(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "microfield-f5-{label}-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn fp65521(profile: GenerationProfile) -> GeneratedPrimeFieldPackage {
    PrimeFieldFactory::builder()
        .name("fp65521_consumer")
        .modulus("65521")
        .profile(profile)
        .build()
        .unwrap()
        .generate()
        .unwrap()
}

#[test]
fn generation_is_reproducible_and_profile_only_changes_artifact_identity() {
    let first = fp65521(GenerationProfile::MultiBackend);
    let repeated = fp65521(GenerationProfile::MultiBackend);
    let portable = fp65521(GenerationProfile::PortableOnly);
    assert_eq!(first, repeated);
    assert_eq!(first.field_id(), portable.field_id());
    assert_ne!(first.artifact_id(), portable.artifact_id());
    assert_ne!(first.bundle_digest(), portable.bundle_digest());
    assert_eq!(
        first.representation(),
        PrimeRepresentationProfile::Canonical16
    );
}

#[test]
fn probable_prime_validates_but_cannot_emit_static_source() {
    let factory = PrimeFieldFactory::builder()
        .name("mersenne127_probable")
        .modulus("170141183460469231731687303715884105727")
        .assurance(ValidationAssurance::ProbablePrime { rounds: 32 })
        .build()
        .unwrap();
    assert_eq!(
        factory.validate().unwrap().normalized().assurance(),
        ValidationAssurance::ProbablePrime { rounds: 32 }
    );
    assert!(matches!(
        factory.generate(),
        Err(PrimeFieldFactoryError::Validation(
            PrimeValidationError::ProbablePrimeCannotGenerateStatic
        ))
    ));
}

#[test]
fn lock_and_check_detect_one_modified_byte() {
    let package = fp65521(GenerationProfile::MultiBackend);
    let root = temporary_directory("lock-drift");
    let publication = package.publish(&root).unwrap();
    assert!(package.matches(&root).unwrap());
    let source = publication.output_directory().join("field.rs");
    let mut bytes = fs::read(&source).unwrap();
    bytes[0] ^= 1;
    fs::write(&source, bytes).unwrap();
    assert!(!package.matches(&root).unwrap());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cache_verifies_payloads_and_rejects_tampering() {
    let package = fp65521(GenerationProfile::MultiBackend);
    let root = temporary_directory("cache");
    let cache = PrimeArtifactCache::new(&root, PrimeCachePolicy::ReadWrite);
    let entry = cache.insert_verified(&package).unwrap().unwrap();
    assert_eq!(
        cache.lookup(package.artifact_id()).unwrap(),
        Some(entry.clone())
    );
    let source = entry.join("field.rs");
    fs::write(&source, b"tampered").unwrap();
    assert!(matches!(
        cache.lookup(package.artifact_id()),
        Err(PrimeFieldFactoryError::Lock(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn lock_index_and_cache_paths_fail_closed() {
    let package = fp65521(GenerationProfile::MultiBackend);
    let root = temporary_directory("cache-lock-drift");
    let cache = PrimeArtifactCache::new(&root, PrimeCachePolicy::ReadWrite);
    let entry = cache.insert_verified(&package).unwrap().unwrap();

    let lock_path = entry.join("microfield.lock");
    let mut lock = fs::read(&lock_path).unwrap();
    let position = lock.iter().position(|byte| *byte == b'{').unwrap();
    lock.insert(position + 1, b' ');
    fs::write(&lock_path, lock).unwrap();
    assert!(matches!(
        cache.lookup(package.artifact_id()),
        Err(PrimeFieldFactoryError::Lock(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_cache_publication_has_one_complete_immutable_result() {
    const WORKERS: usize = 8;
    let package = Arc::new(fp65521(GenerationProfile::MultiBackend));
    let root = temporary_directory("cache-concurrent");
    let cache = PrimeArtifactCache::new(&root, PrimeCachePolicy::ReadWrite);
    let barrier = Arc::new(Barrier::new(WORKERS));
    let workers = (0..WORKERS)
        .map(|_| {
            let package = Arc::clone(&package);
            let cache = cache.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                cache.insert_verified(&package)
            })
        })
        .collect::<Vec<_>>();

    let mut completed = 0;
    for worker in workers {
        match worker.join().unwrap() {
            Ok(Some(_)) => completed += 1,
            Err(PrimeFieldFactoryError::CacheBusy(id)) => {
                assert_eq!(id, package.artifact_id());
            }
            result => panic!("unexpected concurrent cache result: {result:?}"),
        }
    }
    assert!(completed >= 1);
    assert!(cache.lookup(package.artifact_id()).unwrap().is_some());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn generated_profiles_compile_and_execute_in_an_external_consumer() {
    let root = temporary_directory("consumer");
    let generated = root.join("generated");
    fs::create_dir_all(&generated).unwrap();
    let profiles = [
        ("fp251_ext", "251"),
        ("fp65521_ext", "65521"),
        ("fp4294967291_ext", "4294967291"),
        ("fp_goldilocks_ext", "18446744069414584321"),
    ];
    let mut modules = String::new();
    let mut checks = String::new();
    for (index, (name, modulus)) in profiles.iter().enumerate() {
        let package = PrimeFieldFactory::builder()
            .name(*name)
            .modulus(*modulus)
            .build()
            .unwrap()
            .generate()
            .unwrap();
        package.publish(&generated).unwrap();
        let _ = writeln!(
            modules,
            "#[path = {:?}] mod f{index};",
            generated.join(name).join("mod.rs")
        );
        let _ = writeln!(
            checks,
            "let a = f{index}::{}::from_u64_mod(123); assert_eq!(a.mul(a.invert().unwrap()), f{index}::{}::ONE);",
            package.type_name(),
            package.type_name()
        );
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let consumer = root.join("consumer");
    fs::create_dir_all(consumer.join("src")).unwrap();
    fs::write(
        consumer.join("Cargo.toml"),
        format!(
            "[package]\nname='microfield-phase5-consumer'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nmicrofield={{path={manifest_dir:?}}}\n"
        ),
    )
    .unwrap();
    fs::write(
        consumer.join("src/main.rs"),
        format!("{modules}\nuse microfield::{{Field, Invert}};\nfn main() {{ {checks} }}\n"),
    )
    .unwrap();
    let status = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--offline"])
        .current_dir(&consumer)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .status()
        .unwrap();
    assert!(status.success());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn strict_prime_manifest_rejects_unknown_fields_and_wrong_width() {
    let unknown = "prime_schema_version=1\nunknown=true\n[prime]\nname='fp251'\nmodulus='251'\n[encoding]\nbyte_order='little'\ninteger='canonical'\ncanonical_bytes=1\n[validation]\nassurance='proven'\n";
    assert!(PrimeFieldManifest::parse_toml(unknown).is_err());
    let wrong_width = "prime_schema_version=1\n[prime]\nname='fp251'\nmodulus='251'\n[encoding]\nbyte_order='little'\ninteger='canonical'\ncanonical_bytes=2\n[validation]\nassurance='proven'\n";
    assert!(
        PrimeFieldManifest::parse_toml(wrong_width)
            .unwrap()
            .normalize()
            .is_err()
    );
}

#[test]
fn pocklington_witness_and_resource_limits_fail_closed() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fields")
        .join("fp256_generic_external_v1.toml");
    let normalized = PrimeFieldManifest::load(path).unwrap().normalize().unwrap();
    let mut certificate = normalized.certificate().unwrap().clone();
    certificate.factors[0].witness = 1;
    let invalid = PrimeFieldFactory::builder()
        .name("fp256_invalid_proof")
        .modulus(normalized.modulus_decimal())
        .certificate(certificate)
        .build()
        .unwrap();
    assert!(matches!(
        invalid.validate(),
        Err(PrimeFieldFactoryError::Validation(
            PrimeValidationError::InvalidCertificate(_)
        ))
    ));

    let limited = GenerationLimits {
        maximum_characteristic_bits: 8,
        ..GenerationLimits::default()
    };
    let factory = PrimeFieldFactory::builder()
        .name("fp65521_limited")
        .modulus("65521")
        .limits(limited)
        .build()
        .unwrap();
    assert!(matches!(
        factory.validate(),
        Err(PrimeFieldFactoryError::Validation(
            PrimeValidationError::LimitExceeded { .. }
        ))
    ));
}

#[test]
fn prime_cli_generate_check_inspect_and_drift_exit_codes_are_stable() {
    let root = temporary_directory("cli");
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fields")
        .join("fp65521_external_v1.toml");
    let cli = env!("CARGO_BIN_EXE_microfield-gen");
    let generate = Command::new(cli)
        .args(["prime-generate", manifest.to_str().unwrap(), "--out"])
        .arg(&root)
        .status()
        .unwrap();
    assert!(generate.success());
    let check = Command::new(cli)
        .args(["prime-check", manifest.to_str().unwrap(), "--out"])
        .arg(&root)
        .status()
        .unwrap();
    assert!(check.success());
    let bundle = root.join("fp65521_external_v1");
    let inspect = Command::new(cli)
        .arg("prime-inspect")
        .arg(bundle.join("microfield.lock"))
        .status()
        .unwrap();
    assert!(inspect.success());
    fs::write(bundle.join("field.rs"), b"drift").unwrap();
    let drift = Command::new(cli)
        .args(["prime-check", manifest.to_str().unwrap(), "--out"])
        .arg(&root)
        .status()
        .unwrap();
    assert_eq!(drift.code(), Some(1));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(feature = "dynamic")]
#[test]
fn dynamic_to_static_bridge_preserves_identity_and_proof_policy() {
    use microfield::{DynField, StaticExportError};

    let prime = DynField::builder("fp65521_bridge")
        .prime("65521")
        .build()
        .unwrap();
    assert_eq!(
        prime.generate_static().unwrap().field_id(),
        prime.field_id()
    );

    let binary = DynField::builder("gf2_8_bridge")
        .binary(8, vec![8, 4, 3, 1, 0])
        .build()
        .unwrap();
    assert_eq!(
        binary.generate_static().unwrap().field_id(),
        binary.field_id()
    );

    let probable = DynField::builder("mersenne127_bridge")
        .prime("170141183460469231731687303715884105727")
        .assurance(ValidationAssurance::ProbablePrime { rounds: 32 })
        .build()
        .unwrap();
    assert!(matches!(
        probable.generate_static(),
        Err(StaticExportError::Prime(_))
    ));

    let large_manifest = PrimeFieldManifest::load(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fields")
            .join("fp256_generic_external_v1.toml"),
    )
    .unwrap()
    .normalize()
    .unwrap();
    let large = DynField::builder("fp256_dynamic_bridge")
        .prime(large_manifest.modulus_decimal())
        .pocklington_certificate(large_manifest.certificate().unwrap().clone())
        .build()
        .unwrap();
    assert_eq!(
        large.generate_static().unwrap().field_id(),
        large.field_id()
    );
}
