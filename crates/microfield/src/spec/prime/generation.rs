//! Reproducible static packages, lock verification and immutable cache.

use std::{
    collections::BTreeMap,
    fmt,
    fmt::Write as _,
    fs::{self, File, OpenOptions},
    io::Write as _,
    path::{Component, Path, PathBuf},
};

use num_bigint::BigUint;
use num_traits::{One as _, ToPrimitive as _, Zero as _};
use serde::{Deserialize, Serialize};

use crate::{ArtifactBundleDigest, ArtifactId, FieldId, ValidationAssurance};

use super::{
    GenerationLimits, GenerationProfile, PocklingtonCertificate, PrimeFieldManifest,
    PrimeManifestError, PrimeValidationError, ValidatedPrimeField, validation,
};
use crate::spec::{
    FileSystemArtifactSink,
    identity::{artifact_bundle_digest, artifact_id, content_digest},
    model::{GeneratedArtifacts, GeneratedFile},
    ports::{ArtifactSink as _, Publication},
};

const LOCK_VERSION: u16 = 1;
const PRIME_TEMPLATE_VERSION: u16 = 1;
const GENERATOR_BUILD: &str = "microfield-prime-codegen-v1";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BundleIndex {
    schema: u16,
    field_id: String,
    artifact_id: String,
    lock_digest: String,
}

/// Private representation selected deterministically from the modulus shape.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimeRepresentationProfile {
    /// Canonical byte storage and the explicit AVX2 `u8` bridge.
    Canonical8,
    /// Canonical `u16` storage and the explicit AVX2 widening bridge.
    Canonical16,
    /// Canonical `u32` storage and the explicit AVX2 widening bridge.
    Canonical32,
    /// Radix-64 Montgomery storage and the explicit BMI2 bridge.
    Montgomery64 {
        /// Number of 64-bit limbs.
        limbs: u16,
    },
}

/// Version lock embedded in every external generated bundle.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MicrofieldLock {
    lock_version: u16,
    field_id: String,
    artifact_id: String,
    generator_version: String,
    generator_build: String,
    manifest_digest: String,
    profile: GenerationProfile,
    representation: PrimeRepresentationProfile,
    template_versions: BTreeMap<String, u16>,
    payload_digests: BTreeMap<String, String>,
}

impl MicrofieldLock {
    /// Returns the lock schema.
    #[must_use]
    pub const fn lock_version(&self) -> u16 {
        self.lock_version
    }

    /// Returns the locked semantic identity as lowercase hexadecimal.
    #[must_use]
    pub fn field_id(&self) -> &str {
        &self.field_id
    }

    /// Returns the locked artifact identity as lowercase hexadecimal.
    #[must_use]
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }

    /// Returns the canonical-manifest digest.
    #[must_use]
    pub fn manifest_digest(&self) -> &str {
        &self.manifest_digest
    }

    /// Returns the exact generation profile.
    #[must_use]
    pub const fn profile(&self) -> GenerationProfile {
        self.profile
    }

    /// Parses a strict lock JSON document.
    ///
    /// # Errors
    ///
    /// Rejects malformed JSON and unknown lock fields.
    pub fn parse_json(bytes: &[u8]) -> Result<Self, PrimeFieldFactoryError> {
        serde_json::from_slice(bytes)
            .map_err(|error| PrimeFieldFactoryError::Lock(error.to_string()))
    }

    /// Verifies identities and every payload byte covered by this lock.
    ///
    /// # Errors
    ///
    /// Reports schema, identity, missing-file or digest drift.
    pub fn verify_bundle(&self, bundle: &GeneratedArtifacts) -> Result<(), PrimeFieldFactoryError> {
        if self.lock_version != LOCK_VERSION
            || self.field_id != bundle.field_id().to_string()
            || self.artifact_id != bundle.artifact_id().to_string()
            || self.generator_build != GENERATOR_BUILD
        {
            return Err(PrimeFieldFactoryError::Lock(
                "lock identity, schema or generator build does not match the bundle".to_owned(),
            ));
        }
        for (path, digest) in &self.payload_digests {
            let file = bundle
                .files()
                .iter()
                .find(|file| file.relative_path() == path)
                .ok_or_else(|| {
                    PrimeFieldFactoryError::Lock(format!("locked payload `{path}` is missing"))
                })?;
            if content_digest(file.contents()) != *digest {
                return Err(PrimeFieldFactoryError::Lock(format!(
                    "locked payload `{path}` has drifted"
                )));
            }
        }
        let lock_bytes = bundle_file(bundle, "microfield.lock")?;
        if lock_bytes != self.to_pretty_json()? {
            return Err(lock_error(
                "serialized lock disagrees with the verified lock",
            ));
        }
        let bundle_index = bundle_file(bundle, "bundle.json")?;
        verify_bundle_index(bundle_index, lock_bytes, &self.field_id, &self.artifact_id)
    }

    /// Compares whether two locks represent the same semantic field.
    #[must_use]
    pub fn is_field_compatible_with(&self, other: &Self) -> bool {
        self.lock_version == other.lock_version && self.field_id == other.field_id
    }

    fn to_pretty_json(&self) -> Result<Vec<u8>, PrimeFieldFactoryError> {
        let mut bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| PrimeFieldFactoryError::Lock(error.to_string()))?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

/// Failure in the external prime-field factory, bundle or cache.
#[derive(Debug)]
#[non_exhaustive]
pub enum PrimeFieldFactoryError {
    /// Required builder input is absent.
    MissingInput(&'static str),
    /// Manifest parsing or normalization failed.
    Manifest(PrimeManifestError),
    /// Mathematical validation failed.
    Validation(PrimeValidationError),
    /// Artifact rendering failed.
    Generation(String),
    /// Lock contents or payload bytes disagree.
    Lock(String),
    /// Filesystem operation failed.
    Io {
        /// Stable operation name.
        operation: &'static str,
        /// Affected path.
        path: PathBuf,
        /// Operating-system error.
        source: std::io::Error,
    },
    /// Requested cache policy forbids a mutation.
    CacheReadOnly,
    /// Another writer owns the immutable cache key.
    CacheBusy(ArtifactId),
}

impl fmt::Display for PrimeFieldFactoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInput(input) => write!(formatter, "missing prime field input `{input}`"),
            Self::Manifest(error) => error.fmt(formatter),
            Self::Validation(error) => error.fmt(formatter),
            Self::Generation(reason) => write!(formatter, "generate prime field: {reason}"),
            Self::Lock(reason) => write!(formatter, "verify microfield.lock: {reason}"),
            Self::Io {
                operation,
                path,
                source,
            } => write!(formatter, "{operation} `{}`: {source}", path.display()),
            Self::CacheReadOnly => formatter.write_str("artifact cache is read-only"),
            Self::CacheBusy(id) => write!(formatter, "artifact cache key {id} is locked"),
        }
    }
}

impl std::error::Error for PrimeFieldFactoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Manifest(error) => Some(error),
            Self::Validation(error) => Some(error),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<PrimeManifestError> for PrimeFieldFactoryError {
    fn from(error: PrimeManifestError) -> Self {
        Self::Manifest(error)
    }
}

impl From<PrimeValidationError> for PrimeFieldFactoryError {
    fn from(error: PrimeValidationError) -> Self {
        Self::Validation(error)
    }
}

/// Static factory for a replayably proven prime field.
#[derive(Clone, Debug)]
pub struct PrimeFieldFactory {
    manifest: PrimeFieldManifest,
    limits: GenerationLimits,
}

impl PrimeFieldFactory {
    /// Starts explicit construction.
    #[must_use]
    pub fn builder() -> PrimeFieldFactoryBuilder {
        PrimeFieldFactoryBuilder::default()
    }

    /// Loads the strict external-prime schema.
    ///
    /// # Errors
    ///
    /// Returns contextual manifest I/O and parsing failures.
    pub fn from_manifest(path: impl AsRef<Path>) -> Result<Self, PrimeFieldFactoryError> {
        Ok(Self {
            manifest: PrimeFieldManifest::load(path)?,
            limits: GenerationLimits::default(),
        })
    }

    /// Parses one in-memory external-prime manifest.
    ///
    /// # Errors
    ///
    /// Returns strict TOML/schema failures without publishing any state.
    pub fn from_manifest_toml(source: &str) -> Result<Self, PrimeFieldFactoryError> {
        Ok(Self {
            manifest: PrimeFieldManifest::parse_toml(source)?,
            limits: GenerationLimits::default(),
        })
    }

    /// Applies custom defensive ceilings.
    #[must_use]
    pub const fn with_limits(mut self, limits: GenerationLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Validates a proven or probable-prime definition without emitting source.
    ///
    /// # Errors
    ///
    /// Returns normalization, limit, primality or certificate failures.
    pub fn validate(&self) -> Result<ValidatedPrimeField, PrimeFieldFactoryError> {
        let normalized = self.manifest.clone().normalize()?;
        validation::validate(normalized, self.limits).map_err(Into::into)
    }

    /// Produces a complete deterministic nominal Rust bundle.
    ///
    /// Probable-prime assurance is intentionally rejected here.
    ///
    /// # Errors
    ///
    /// Returns proof, resource, rendering or lock failures.
    pub fn generate(&self) -> Result<GeneratedPrimeFieldPackage, PrimeFieldFactoryError> {
        let validated = self.validate()?;
        if !validated.permits_static_generation() {
            return Err(PrimeValidationError::ProbablePrimeCannotGenerateStatic.into());
        }
        render_package(validated, self.limits)
    }
}

/// Explicit builder for an external prime definition.
#[derive(Clone, Debug, Default)]
pub struct PrimeFieldFactoryBuilder {
    name: Option<String>,
    modulus: Option<String>,
    assurance: Option<ValidationAssurance>,
    certificate: Option<PocklingtonCertificate>,
    profile: GenerationProfile,
    limits: GenerationLimits,
}

impl PrimeFieldFactoryBuilder {
    /// Sets the normalized presentation name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the canonical unsigned decimal characteristic.
    #[must_use]
    pub fn modulus(mut self, modulus: impl Into<String>) -> Self {
        self.modulus = Some(modulus.into());
        self
    }

    /// Selects validation assurance. Defaults to [`ValidationAssurance::Proven`].
    #[must_use]
    pub const fn assurance(mut self, assurance: ValidationAssurance) -> Self {
        self.assurance = Some(assurance);
        self
    }

    /// Supplies a replayable Pocklington proof for a large proven modulus.
    #[must_use]
    pub fn certificate(mut self, certificate: PocklingtonCertificate) -> Self {
        self.certificate = Some(certificate);
        self
    }

    /// Selects the non-semantic output profile.
    #[must_use]
    pub const fn profile(mut self, profile: GenerationProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Replaces defensive ceilings.
    #[must_use]
    pub const fn limits(mut self, limits: GenerationLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Builds through the exact same strict parser as a file manifest.
    ///
    /// # Errors
    ///
    /// Returns missing inputs and strict normalization failures.
    pub fn build(self) -> Result<PrimeFieldFactory, PrimeFieldFactoryError> {
        let name = self
            .name
            .ok_or(PrimeFieldFactoryError::MissingInput("name"))?;
        let modulus = self
            .modulus
            .ok_or(PrimeFieldFactoryError::MissingInput("modulus"))?;
        let parsed = BigUint::parse_bytes(modulus.as_bytes(), 10).ok_or_else(|| {
            PrimeFieldFactoryError::Generation("invalid decimal modulus".to_owned())
        })?;
        let bytes = parsed.bits().div_ceil(8);
        let assurance = self.assurance.unwrap_or(ValidationAssurance::Proven);
        let validation = match assurance {
            ValidationAssurance::Proven => "assurance = \"proven\"\n".to_owned(),
            ValidationAssurance::ProbablePrime { rounds } => {
                format!("assurance = \"probable_prime\"\nrounds = {rounds}\n")
            }
        };
        let certificate = self
            .certificate
            .as_ref()
            .map(render_certificate_toml)
            .unwrap_or_default();
        let profile = match self.profile {
            GenerationProfile::PortableOnly => "portable_only",
            GenerationProfile::MultiBackend => "multi_backend",
            GenerationProfile::Audit => "audit",
        };
        let source = format!(
            "prime_schema_version = 1\n\n[prime]\nname = \"{name}\"\nmodulus = \"{modulus}\"\n\n[encoding]\nbyte_order = \"little\"\ninteger = \"canonical\"\ncanonical_bytes = {bytes}\n\n[validation]\n{validation}{certificate}\n[build]\nprofile = \"{profile}\"\n"
        );
        Ok(PrimeFieldFactory {
            manifest: PrimeFieldManifest::parse_toml(&source)?,
            limits: self.limits,
        })
    }
}

/// Complete verified package ready for review, publication or inclusion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedPrimeFieldPackage {
    validated: ValidatedPrimeField,
    representation: PrimeRepresentationProfile,
    type_name: String,
    lock: MicrofieldLock,
    artifacts: GeneratedArtifacts,
}

impl GeneratedPrimeFieldPackage {
    /// Semantic field identity.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.artifacts.field_id()
    }

    /// Representation and profile identity.
    #[must_use]
    pub const fn artifact_id(&self) -> ArtifactId {
        self.artifacts.artifact_id()
    }

    /// Integrity digest of the complete published byte set.
    #[must_use]
    pub const fn bundle_digest(&self) -> ArtifactBundleDigest {
        self.artifacts.bundle_digest()
    }

    /// Generated nominal Rust type name.
    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Selected private representation profile.
    #[must_use]
    pub const fn representation(&self) -> PrimeRepresentationProfile {
        self.representation
    }

    /// Replayable validated definition.
    #[must_use]
    pub const fn validated(&self) -> &ValidatedPrimeField {
        &self.validated
    }

    /// Version and payload lock.
    #[must_use]
    pub const fn lock(&self) -> &MicrofieldLock {
        &self.lock
    }

    /// Complete stable lexical artifact list.
    #[must_use]
    pub const fn artifacts(&self) -> &GeneratedArtifacts {
        &self.artifacts
    }

    /// Returns the generated field implementation bytes.
    #[must_use]
    pub fn rust_source(&self) -> &[u8] {
        self.artifacts
            .files()
            .iter()
            .find(|file| file.relative_path() == "field.rs")
            .map(GeneratedFile::contents)
            .unwrap_or_default()
    }

    /// Atomically publishes the complete bundle below its presentation name.
    ///
    /// # Errors
    ///
    /// Returns lock drift or transactional filesystem failures.
    pub fn publish(&self, root: impl Into<PathBuf>) -> Result<Publication, PrimeFieldFactoryError> {
        self.lock.verify_bundle(&self.artifacts)?;
        let sink = FileSystemArtifactSink::new(root);
        sink.publish(&self.artifacts)
            .map_err(|error| PrimeFieldFactoryError::Generation(error.to_string()))
    }

    /// Regenerates no state; compares a committed bundle byte-for-byte.
    ///
    /// # Errors
    ///
    /// Returns contextual filesystem inspection failures.
    pub fn matches(&self, root: impl Into<PathBuf>) -> Result<bool, PrimeFieldFactoryError> {
        let sink = FileSystemArtifactSink::new(root);
        sink.matches(&self.artifacts)
            .map_err(|error| PrimeFieldFactoryError::Generation(error.to_string()))
    }
}

/// Filesystem cache mutation policy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PrimeCachePolicy {
    /// Verify lookups but reject inserts.
    ReadOnly,
    /// Verify lookups and permit atomic immutable inserts.
    ReadWrite,
    /// Ignore cache lookups and inserts.
    Disabled,
}

/// Immutable, digest-verifying cache keyed by `ArtifactId`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimeArtifactCache {
    root: PathBuf,
    policy: PrimeCachePolicy,
}

impl PrimeArtifactCache {
    /// Creates a cache rooted at an explicitly authorized directory.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, policy: PrimeCachePolicy) -> Self {
        Self {
            root: root.into(),
            policy,
        }
    }

    /// Returns a verified immutable entry path when present.
    ///
    /// # Errors
    ///
    /// Rejects special files, malformed locks and digest drift.
    pub fn lookup(
        &self,
        artifact_id: ArtifactId,
    ) -> Result<Option<PathBuf>, PrimeFieldFactoryError> {
        if self.policy == PrimeCachePolicy::Disabled {
            return Ok(None);
        }
        let path = self.root.join(artifact_id.to_string());
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => return Err(lock_error("cache entry is not a real directory")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(io_error("inspect cache entry", path, source)),
        }
        verify_cache_directory(&path, artifact_id)?;
        Ok(Some(path))
    }

    /// Inserts a verified bundle exactly once using a sibling staging directory.
    ///
    /// # Errors
    ///
    /// Rejects policy violations, concurrent writers, digest drift and I/O
    /// failures without publishing partial entries.
    pub fn insert_verified(
        &self,
        package: &GeneratedPrimeFieldPackage,
    ) -> Result<Option<PathBuf>, PrimeFieldFactoryError> {
        match self.policy {
            PrimeCachePolicy::Disabled => return Ok(None),
            PrimeCachePolicy::ReadOnly => return Err(PrimeFieldFactoryError::CacheReadOnly),
            PrimeCachePolicy::ReadWrite => {}
        }
        package.lock.verify_bundle(package.artifacts())?;
        fs::create_dir_all(&self.root)
            .map_err(|source| io_error("create cache root", self.root.clone(), source))?;
        let id = package.artifact_id();
        if let Some(path) = self.lookup(id)? {
            return Ok(Some(path));
        }
        let lock_path = self.root.join(format!(".{id}.lock"));
        let lock_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path);
        let _lock = match lock_file {
            Ok(file) => CacheLock {
                path: lock_path,
                file,
            },
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(PrimeFieldFactoryError::CacheBusy(id));
            }
            Err(source) => return Err(io_error("acquire cache lock", lock_path, source)),
        };
        if let Some(path) = self.lookup(id)? {
            return Ok(Some(path));
        }
        let staging = self
            .root
            .join(format!(".{}.staging-{}", id, std::process::id()));
        fs::create_dir(&staging)
            .map_err(|source| io_error("create cache staging", staging.clone(), source))?;
        if let Err(error) = write_artifacts(&staging, package.artifacts()) {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        let target = self.root.join(id.to_string());
        if let Err(source) = fs::rename(&staging, &target) {
            let _ = fs::remove_dir_all(&staging);
            return Err(io_error("publish cache entry", target, source));
        }
        verify_cache_directory(&target, id)?;
        Ok(Some(target))
    }
}

struct CacheLock {
    path: PathBuf,
    #[allow(dead_code)]
    file: File,
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[allow(clippy::too_many_lines)]
fn render_package(
    validated: ValidatedPrimeField,
    limits: GenerationLimits,
) -> Result<GeneratedPrimeFieldPackage, PrimeFieldFactoryError> {
    let normalized = validated.normalized();
    let representation = select_representation(normalized.modulus());
    let type_name = rust_type_name(normalized.name());
    let plan = serde_json::json!({
        "schema": 1,
        "field_id": validated.field_id(),
        "codegen_abi": crate::__private::CURRENT_CODEGEN_ABI_VERSION,
        "generator_build": GENERATOR_BUILD,
        "profile": normalized.profile(),
        "representation": representation,
        "portable": "fixed-shape-generated-v1",
        "isa_selection": "explicit_only_without_field_calibration",
    });
    let plan_bytes = serde_json::to_vec(&plan)
        .map_err(|error| PrimeFieldFactoryError::Generation(error.to_string()))?;
    let artifact_id = artifact_id(&plan_bytes);
    let descriptor: serde_json::Value = serde_json::from_str(normalized.identity_json())
        .map_err(|error| PrimeFieldFactoryError::Generation(error.to_string()))?;
    let mut descriptor_bytes = serde_json::to_vec_pretty(&descriptor)
        .map_err(|error| PrimeFieldFactoryError::Generation(error.to_string()))?;
    descriptor_bytes.push(b'\n');
    let mut plan_pretty = serde_json::to_vec_pretty(&plan)
        .map_err(|error| PrimeFieldFactoryError::Generation(error.to_string()))?;
    plan_pretty.push(b'\n');
    let source = render_source(&validated, artifact_id, representation, &type_name);
    let vectors = render_vectors(&validated, normalized.profile())?;
    let manifest_digest = content_digest(normalized.canonical_toml().as_bytes());
    let readme = format!(
        "# {}\n\nCampo primo externo generado de forma determinista por Microfield.\n\n- `FieldId`: `{}`\n- `ArtifactId`: `{artifact_id}`\n- assurance: `Proven`\n- representación: `{representation:?}`\n- ISA externa: candidata explícita hasta calibración del campo concreto.\n",
        normalized.name(),
        validated.field_id()
    );
    let mut files = vec![
        generated_file("README.generated.md", readme.into_bytes())?,
        generated_file("certificate.json", validated.certificate_json().to_vec())?,
        generated_file("descriptor.json", descriptor_bytes)?,
        generated_file("field.rs", source.into_bytes())?,
        generated_file(
            "mod.rs",
            format!("mod field;\npub use field::{type_name};\n").into_bytes(),
        )?,
        generated_file(
            "normalized.toml",
            normalized.canonical_toml().as_bytes().to_vec(),
        )?,
        generated_file("plans.json", plan_pretty)?,
        generated_file("vectors.json", vectors)?,
    ];
    let payload_digests = files
        .iter()
        .map(|file| {
            (
                file.relative_path().to_owned(),
                content_digest(file.contents()),
            )
        })
        .collect();
    let mut template_versions = BTreeMap::new();
    template_versions.insert("prime-field".to_owned(), PRIME_TEMPLATE_VERSION);
    let lock = MicrofieldLock {
        lock_version: LOCK_VERSION,
        field_id: validated.field_id().to_string(),
        artifact_id: artifact_id.to_string(),
        generator_version: env!("CARGO_PKG_VERSION").to_owned(),
        generator_build: GENERATOR_BUILD.to_owned(),
        manifest_digest,
        profile: normalized.profile(),
        representation,
        template_versions,
        payload_digests,
    };
    let lock_bytes = lock.to_pretty_json()?;
    let lock_digest = content_digest(&lock_bytes);
    files.push(generated_file("microfield.lock", lock_bytes)?);
    files.push(generated_file(
        "bundle.json",
        format!(
            "{{\n  \"schema\": 1,\n  \"field_id\": \"{}\",\n  \"artifact_id\": \"{}\",\n  \"lock_digest\": \"{}\"\n}}\n",
            validated.field_id(), artifact_id, lock_digest
        )
        .into_bytes(),
    )?);
    let total_bytes = files.iter().try_fold(0_u64, |total, file| {
        total.checked_add(file.contents().len() as u64)
    });
    if total_bytes.is_none_or(|bytes| bytes > limits.maximum_generated_bytes) {
        return Err(PrimeValidationError::LimitExceeded {
            limit: "maximum_generated_bytes",
            maximum: limits.maximum_generated_bytes,
        }
        .into());
    }
    let digest_descriptor = bundle_descriptor(&files);
    let bundle_digest = artifact_bundle_digest(&digest_descriptor);
    let artifacts = GeneratedArtifacts::new(
        normalized.name().to_owned(),
        validated.field_id(),
        artifact_id,
        bundle_digest,
        files,
    )
    .map_err(|error| PrimeFieldFactoryError::Generation(error.to_string()))?;
    lock.verify_bundle(&artifacts)?;
    Ok(GeneratedPrimeFieldPackage {
        validated,
        representation,
        type_name,
        lock,
        artifacts,
    })
}

fn render_vectors(
    validated: &ValidatedPrimeField,
    profile: GenerationProfile,
) -> Result<Vec<u8>, PrimeFieldFactoryError> {
    let modulus = validated.normalized().modulus();
    let count = if profile == GenerationProfile::Audit {
        64
    } else {
        16
    };
    let mut vectors = Vec::with_capacity(count);
    for index in 0..count {
        let lhs = (BigUint::from(index * 17 + 3)) % modulus;
        let rhs = (BigUint::from(index * 29 + 5)) % modulus;
        let inverse = if lhs.is_zero() {
            None
        } else {
            Some(little_hex(
                &lhs.modpow(&(modulus - 2_u8), modulus),
                validated.normalized().canonical_bytes(),
            ))
        };
        vectors.push(serde_json::json!({
            "lhs_le_hex": little_hex(&lhs, validated.normalized().canonical_bytes()),
            "rhs_le_hex": little_hex(&rhs, validated.normalized().canonical_bytes()),
            "add_le_hex": little_hex(&((&lhs + &rhs) % modulus), validated.normalized().canonical_bytes()),
            "mul_le_hex": little_hex(&((&lhs * &rhs) % modulus), validated.normalized().canonical_bytes()),
            "square_le_hex": little_hex(&((&lhs * &lhs) % modulus), validated.normalized().canonical_bytes()),
            "inverse_le_hex": inverse,
        }));
    }
    let document = serde_json::json!({
        "schema": 1,
        "field_id": validated.field_id(),
        "oracle": "microfield-num-bigint-reference-v1",
        "encoding": "canonical-little-endian-hex",
        "vectors": vectors,
    });
    let mut bytes = serde_json::to_vec_pretty(&document)
        .map_err(|error| PrimeFieldFactoryError::Generation(error.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn little_hex(value: &BigUint, bytes: usize) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = value.to_bytes_le();
    encoded.resize(bytes, 0);
    let mut result = String::with_capacity(bytes * 2);
    for byte in encoded {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    result
}

fn select_representation(modulus: &BigUint) -> PrimeRepresentationProfile {
    if modulus <= &BigUint::from(251_u16) {
        PrimeRepresentationProfile::Canonical8
    } else if modulus <= &BigUint::from(65_521_u32) {
        PrimeRepresentationProfile::Canonical16
    } else if modulus <= &BigUint::from(4_294_967_291_u64) {
        PrimeRepresentationProfile::Canonical32
    } else {
        PrimeRepresentationProfile::Montgomery64 {
            limbs: u16::try_from(modulus.bits().div_ceil(64))
                .expect("generation limits keep limb count within u16"),
        }
    }
}

fn render_source(
    validated: &ValidatedPrimeField,
    artifact_id: ArtifactId,
    representation: PrimeRepresentationProfile,
    type_name: &str,
) -> String {
    match representation {
        PrimeRepresentationProfile::Canonical8 => {
            render_small_source(validated, artifact_id, type_name, "u8", 16, 32)
        }
        PrimeRepresentationProfile::Canonical16 => {
            render_small_source(validated, artifact_id, type_name, "u16", 32, 16)
        }
        PrimeRepresentationProfile::Canonical32 => {
            render_small_source(validated, artifact_id, type_name, "u32", 64, 8)
        }
        PrimeRepresentationProfile::Montgomery64 { limbs } => {
            render_montgomery_source(validated, artifact_id, type_name, usize::from(limbs))
        }
    }
}

#[allow(clippy::too_many_lines)]
fn render_small_source(
    validated: &ValidatedPrimeField,
    artifact_id: ArtifactId,
    type_name: &str,
    storage: &str,
    accumulator_bits: u16,
    lanes: u16,
) -> String {
    let normalized = validated.normalized();
    let modulus = normalized
        .modulus()
        .to_u64()
        .expect("small profiles fit u64");
    let bytes = normalized.canonical_bytes();
    let bits = validated.modulus_bits();
    let capacity = bits - 1;
    let reciprocal = match storage {
        "u8" => (65_536_u64 / modulus).to_string(),
        "u16" => ((1_u128 << 32) / u128::from(modulus)).to_string(),
        "u32" => (u64::MAX / modulus).to_string(),
        _ => unreachable!(),
    };
    let verified_trait = match storage {
        "u8" => "VerifiedPrimeCanonical8Field",
        "u16" => "VerifiedPrimeCanonical16Field",
        "u32" => "VerifiedPrimeCanonical32Field",
        _ => unreachable!(),
    };
    let strategy = match storage {
        "u8" => "VerifiedPrimeSimd8Strategy",
        "u16" => "VerifiedPrimeSimd16Strategy",
        "u32" => "VerifiedPrimeSimd32Strategy",
        _ => unreachable!(),
    };
    let trait_modulus_type = match storage {
        "u8" => "u16",
        "u16" => "u32",
        "u32" => "u64",
        _ => unreachable!(),
    };
    let profile_has_isa = normalized.profile() != GenerationProfile::PortableOnly;
    let isa_static = if profile_has_isa {
        format!(
            "static VERIFIED_ISA_STRATEGY: ::microfield::__private::{strategy}<{type_name}> = ::microfield::__private::{strategy}::new(PRIME_METADATA);"
        )
    } else {
        String::new()
    };
    let catalog = if profile_has_isa {
        "fn __kernel_catalog() -> ::microfield::KernelCatalog<Self> { VERIFIED_ISA_STRATEGY.__kernel_catalog(&PORTABLE_STRATEGY) }"
    } else {
        ""
    };
    let descriptor = format!("{:?}", normalized.identity_json().as_bytes());
    let certificate = format!("{:?}", validated.certificate_json());
    format!(
        r#"// Proven prime field generated by `microfield`.
const _: () = assert!(::microfield::__private::supports_codegen_abi({abi}));
const MODULUS: u64 = {modulus};
const CHARACTERISTIC: ::microfield::Characteristic = ::microfield::Characteristic::__from_generated("{modulus}", Some({modulus}));
const DESCRIPTOR_JSON: &[u8] = &{descriptor};
const CERTIFICATE_JSON: &[u8] = &{certificate};
const PRIME_METADATA: ::microfield::PrimeKernelMetadata = ::microfield::PrimeKernelMetadata::__from_generated(
    ::microfield::PrimeRepresentationKind::CanonicalResidue,
    ::microfield::PrimeReductionKind::Barrett,
    ::microfield::RangeContract::__from_generated(1, 1, {accumulator_bits}),
    ::microfield::RangeContract::__from_generated(1, 1, {accumulator_bits}),
    {lanes}, false,
);
static SPEC: ::microfield::StaticFieldSpec = ::microfield::StaticFieldSpec::__from_generated_prime(
    ::microfield::FieldId::from_bytes({field_id:?}),
    ::microfield::ArtifactId::from_bytes({artifact_id:?}),
    "{name}", "{modulus}", Some({modulus}), 1, {bytes}, DESCRIPTOR_JSON, CERTIFICATE_JSON,
);

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct {type_name}({storage});

static PORTABLE_STRATEGY: ::microfield::__private::PortableStrategy<{type_name}> = ::microfield::__private::PortableStrategy::new_prime(PRIME_METADATA);
{isa_static}

impl {type_name} {{
    #[must_use]
    pub const fn from_u64_mod(value: u64) -> Self {{ Self((value % MODULUS) as {storage}) }}
    fn write_hex(self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{
        for byte in <Self as ::microfield::CanonicalEncoding>::to_canonical(self).iter().rev() {{ write!(f, "{{byte:02x}}")?; }}
        Ok(())
    }}
}}

impl ::microfield::Field for {type_name} {{
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);
    #[inline] fn add(self, rhs: Self) -> Self {{ Self(((self.0 as u64 + rhs.0 as u64) % MODULUS) as {storage}) }}
    #[inline] fn sub(self, rhs: Self) -> Self {{ Self(((self.0 as u64 + MODULUS - rhs.0 as u64) % MODULUS) as {storage}) }}
    #[inline] fn neg(self) -> Self {{ Self(((MODULUS - self.0 as u64) % MODULUS) as {storage}) }}
    #[inline] fn mul(self, rhs: Self) -> Self {{ Self(((self.0 as u64 * rhs.0 as u64) % MODULUS) as {storage}) }}
    #[inline] fn is_zero(&self) -> bool {{ self.0 == 0 }}
}}
impl ::microfield::Square for {type_name} {{ #[inline] fn square(self) -> Self {{ <Self as ::microfield::Field>::mul(self, self) }} }}
impl ::microfield::Invert for {type_name} {{ fn invert(self) -> Option<Self> {{ (!<Self as ::microfield::Field>::is_zero(&self)).then(|| <Self as ::microfield::Pow>::pow(self, &[MODULUS - 2])) }} }}
impl ::microfield::Pow for {type_name} {{}}
impl ::microfield::CanonicalEncoding for {type_name} {{
    type Repr = [u8; {bytes}];
    fn from_canonical(repr: &Self::Repr) -> Result<Self, ::microfield::DecodeError> {{
        let mut value = 0_u64;
        let mut index = 0; while index < {bytes} {{ value |= u64::from(repr[index]) << (index * 8); index += 1; }}
        if value >= MODULUS {{ Err(::microfield::DecodeError::NonCanonicalValue) }} else {{ Ok(Self(value as {storage})) }}
    }}
    fn from_canonical_slice(bytes: &[u8]) -> Result<Self, ::microfield::DecodeError> {{
        let repr: [u8; {bytes}] = bytes.try_into().map_err(|_| ::microfield::DecodeError::LengthMismatch {{ expected: {bytes}, actual: bytes.len() }})?;
        Self::from_canonical(&repr)
    }}
    fn to_canonical(self) -> Self::Repr {{
        let mut repr = [0_u8; {bytes}]; let raw = (self.0 as u64).to_le_bytes(); repr.copy_from_slice(&raw[..{bytes}]); repr
    }}
}}
impl ::microfield::PrimeField for {type_name} {{
    const MODULUS_BITS: u32 = {bits}; const CAPACITY_BITS: u32 = {capacity};
    fn characteristic_descriptor() -> &'static ::microfield::Characteristic {{ &CHARACTERISTIC }}
    fn from_bytes_mod_order(bytes: &[u8]) -> Self {{
        let mut residue = 0_u64; for byte in bytes.iter().rev() {{ residue = ((u128::from(residue) * 256 + u128::from(*byte)) % u128::from(MODULUS)) as u64; }} Self(residue as {storage})
    }}
}}
impl ::microfield::StaticField for {type_name} {{ fn spec() -> &'static ::microfield::StaticFieldSpec {{ &SPEC }} }}
impl ::microfield::__private::PortableField for {type_name} {{
    fn __portable_strategy() -> &'static ::microfield::__private::PortableStrategy<Self> {{ &PORTABLE_STRATEGY }}
    {catalog}
}}
impl ::microfield::__private::{verified_trait} for {type_name} {{
    const __MODULUS: {trait_modulus_type} = {modulus};
    const __BARRETT_RECIPROCAL: {trait_modulus_type} = {reciprocal};
    fn __into_canonical_{storage}(self) -> {storage} {{ self.0 }}
    fn __from_reduced_canonical_{storage}(value: {storage}) -> Self {{ Self(value) }}
}}
{operators}
"#,
        abi = crate::__private::CURRENT_CODEGEN_ABI_VERSION,
        field_id = validated.field_id().into_bytes(),
        artifact_id = artifact_id.into_bytes(),
        name = normalized.name(),
        bytes = bytes,
        operators = render_operators(type_name),
    )
}

#[allow(clippy::too_many_lines)]
fn render_montgomery_source(
    validated: &ValidatedPrimeField,
    artifact_id: ArtifactId,
    type_name: &str,
    limbs: usize,
) -> String {
    let normalized = validated.normalized();
    let modulus = normalized.modulus();
    let wide = limbs * 2;
    let radix = BigUint::one() << (limbs * 64);
    let r = &radix % modulus;
    let r2 = (&r * &r) % modulus;
    let modulus_limbs = fixed_limbs(modulus, limbs);
    let radix_limbs = fixed_limbs(&r, limbs);
    let radix_squared_limbs = fixed_limbs(&r2, limbs);
    let neg_inv = montgomery_neg_inverse(modulus_limbs[0]);
    let exponent_limbs = fixed_limbs(&(modulus - 2_u8), limbs);
    let bytes = normalized.canonical_bytes();
    let bits = validated.modulus_bits();
    let capacity = bits - 1;
    let accumulator_bits = u16::try_from(wide * 64).expect("bounded profile fits u16");
    let characteristic_small = modulus.to_u64();
    let characteristic_small_source =
        characteristic_small.map_or_else(|| "None".to_owned(), |value| format!("Some({value})"));
    let from_u64_value =
        characteristic_small.map_or_else(|| "value".to_owned(), |value| format!("value % {value}"));
    let profile_has_isa = normalized.profile() != GenerationProfile::PortableOnly;
    let isa_static = if profile_has_isa {
        format!(
            "static VERIFIED_ISA_STRATEGY: ::microfield::__private::VerifiedPrimeIsaStrategy<{type_name}, {limbs}, {wide}> = ::microfield::__private::VerifiedPrimeIsaStrategy::new(PRIME_METADATA);"
        )
    } else {
        String::new()
    };
    let catalog = if profile_has_isa {
        "fn __kernel_catalog() -> ::microfield::KernelCatalog<Self> { VERIFIED_ISA_STRATEGY.__kernel_catalog(&PORTABLE_STRATEGY) }"
    } else {
        ""
    };
    format!(
        r#"// Proven prime field generated by `microfield`.
const _: () = assert!(::microfield::__private::supports_codegen_abi({abi}));
const MODULUS: [u64; {limbs}] = {modulus_limbs:?};
const R: [u64; {limbs}] = {radix_limbs:?};
const R2: [u64; {limbs}] = {radix_squared_limbs:?};
const NEG_INV: u64 = {neg_inv};
const INVERSE_EXPONENT: [u64; {limbs}] = {exponent_limbs:?};
const CHARACTERISTIC: ::microfield::Characteristic = ::microfield::Characteristic::__from_generated("{modulus}", {characteristic_small_source});
const DESCRIPTOR_JSON: &[u8] = &{descriptor:?};
const CERTIFICATE_JSON: &[u8] = &{certificate:?};
const PRIME_METADATA: ::microfield::PrimeKernelMetadata = ::microfield::PrimeKernelMetadata::__from_generated(
    ::microfield::PrimeRepresentationKind::Montgomery {{ radix_bits: 64, limbs: {limbs} }},
    ::microfield::PrimeReductionKind::Montgomery,
    ::microfield::RangeContract::__from_generated(1, 1, {accumulator_bits}),
    ::microfield::RangeContract::__from_generated(1, 1, {accumulator_bits}),
    1, false,
);
static SPEC: ::microfield::StaticFieldSpec = ::microfield::StaticFieldSpec::__from_generated_prime(
    ::microfield::FieldId::from_bytes({field_id:?}), ::microfield::ArtifactId::from_bytes({artifact_id:?}),
    "{name}", "{modulus}", {characteristic_small_source}, 1, {bytes}, DESCRIPTOR_JSON, CERTIFICATE_JSON,
);
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct {type_name}([u64; {limbs}]);
static PORTABLE_STRATEGY: ::microfield::__private::PortableStrategy<{type_name}> = ::microfield::__private::PortableStrategy::new_prime(PRIME_METADATA);
{isa_static}
impl {type_name} {{
    #[must_use] pub fn from_u64_mod(value: u64) -> Self {{
        let value = {from_u64_value}; let mut canonical = [0_u64; {limbs}]; canonical[0] = value;
        Self(::microfield::__private::prime_to_montgomery::<{limbs}, {wide}>(canonical, R2, MODULUS, NEG_INV))
    }}
    fn canonical_limbs(self) -> [u64; {limbs}] {{ ::microfield::__private::prime_from_montgomery::<{limbs}, {wide}>(self.0, MODULUS, NEG_INV) }}
    fn write_hex(self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{ for byte in <Self as ::microfield::CanonicalEncoding>::to_canonical(self).iter().rev() {{ write!(f, "{{byte:02x}}")?; }} Ok(()) }}
}}
impl ::microfield::Field for {type_name} {{
    const ZERO: Self = Self([0; {limbs}]); const ONE: Self = Self(R);
    #[inline] fn add(self, rhs: Self) -> Self {{ Self(::microfield::__private::prime_add_mod(self.0, rhs.0, MODULUS)) }}
    #[inline] fn sub(self, rhs: Self) -> Self {{ Self(::microfield::__private::prime_sub_mod(self.0, rhs.0, MODULUS)) }}
    #[inline] fn neg(self) -> Self {{ Self(::microfield::__private::prime_neg_mod(self.0, MODULUS)) }}
    #[inline] fn mul(self, rhs: Self) -> Self {{ Self(::microfield::__private::prime_montgomery_mul::<{limbs}, {wide}>(self.0, rhs.0, MODULUS, NEG_INV)) }}
    #[inline] fn is_zero(&self) -> bool {{ ::microfield::__private::is_zero(&self.0) }}
}}
impl ::microfield::Square for {type_name} {{ #[inline] fn square(self) -> Self {{ <Self as ::microfield::Field>::mul(self, self) }} }}
impl ::microfield::Invert for {type_name} {{ fn invert(self) -> Option<Self> {{ (!<Self as ::microfield::Field>::is_zero(&self)).then(|| <Self as ::microfield::Pow>::pow(self, &INVERSE_EXPONENT)) }} }}
impl ::microfield::Pow for {type_name} {{}}
impl ::microfield::CanonicalEncoding for {type_name} {{
    type Repr = [u8; {bytes}];
    fn from_canonical(repr: &Self::Repr) -> Result<Self, ::microfield::DecodeError> {{
        let canonical = ::microfield::__private::prime_decode_limbs::<{limbs}>(repr);
        if !::microfield::__private::prime_limbs_less_than(&canonical, &MODULUS) {{ return Err(::microfield::DecodeError::NonCanonicalValue); }}
        Ok(Self(::microfield::__private::prime_to_montgomery::<{limbs}, {wide}>(canonical, R2, MODULUS, NEG_INV)))
    }}
    fn from_canonical_slice(bytes: &[u8]) -> Result<Self, ::microfield::DecodeError> {{
        let repr: [u8; {bytes}] = bytes.try_into().map_err(|_| ::microfield::DecodeError::LengthMismatch {{ expected: {bytes}, actual: bytes.len() }})?; Self::from_canonical(&repr)
    }}
    fn to_canonical(self) -> Self::Repr {{ let mut repr = [0_u8; {bytes}]; ::microfield::__private::prime_encode_limbs(self.canonical_limbs(), &mut repr); repr }}
}}
impl ::microfield::PrimeField for {type_name} {{
    const MODULUS_BITS: u32 = {bits}; const CAPACITY_BITS: u32 = {capacity};
    fn characteristic_descriptor() -> &'static ::microfield::Characteristic {{ &CHARACTERISTIC }}
    fn from_bytes_mod_order(bytes: &[u8]) -> Self {{ let radix = Self::from_u64_mod(256); let mut residue = <Self as ::microfield::Field>::ZERO; for byte in bytes.iter().rev() {{ residue = <Self as ::microfield::Field>::add(<Self as ::microfield::Field>::mul(residue, radix), Self::from_u64_mod(u64::from(*byte))); }} residue }}
}}
impl ::microfield::StaticField for {type_name} {{ fn spec() -> &'static ::microfield::StaticFieldSpec {{ &SPEC }} }}
impl ::microfield::__private::PortableField for {type_name} {{ fn __portable_strategy() -> &'static ::microfield::__private::PortableStrategy<Self> {{ &PORTABLE_STRATEGY }} {catalog} }}
impl ::microfield::__private::VerifiedPrimeMontgomery64Field<{limbs}, {wide}> for {type_name} {{
    const __MODULUS: [u64; {limbs}] = MODULUS; const __NEG_INV: u64 = NEG_INV;
    fn __into_montgomery_limbs(self) -> [u64; {limbs}] {{ self.0 }}
    fn __from_reduced_montgomery_limbs(limbs: [u64; {limbs}]) -> Self {{ Self(limbs) }}
}}
{operators}
"#,
        abi = crate::__private::CURRENT_CODEGEN_ABI_VERSION,
        modulus = modulus,
        descriptor = normalized.identity_json().as_bytes(),
        certificate = validated.certificate_json(),
        field_id = validated.field_id().into_bytes(),
        artifact_id = artifact_id.into_bytes(),
        name = normalized.name(),
        operators = render_operators(type_name),
    )
}

fn render_operators(type_name: &str) -> String {
    format!(
        r#"impl core::ops::Add for {type_name} {{ type Output = Self; fn add(self, rhs: Self) -> Self {{ <Self as ::microfield::Field>::add(self, rhs) }} }}
impl core::ops::AddAssign for {type_name} {{ fn add_assign(&mut self, rhs: Self) {{ *self = <Self as ::microfield::Field>::add(*self, rhs); }} }}
impl core::ops::Sub for {type_name} {{ type Output = Self; fn sub(self, rhs: Self) -> Self {{ <Self as ::microfield::Field>::sub(self, rhs) }} }}
impl core::ops::SubAssign for {type_name} {{ fn sub_assign(&mut self, rhs: Self) {{ *self = <Self as ::microfield::Field>::sub(*self, rhs); }} }}
impl core::ops::Mul for {type_name} {{ type Output = Self; fn mul(self, rhs: Self) -> Self {{ <Self as ::microfield::Field>::mul(self, rhs) }} }}
impl core::ops::MulAssign for {type_name} {{ fn mul_assign(&mut self, rhs: Self) {{ *self = <Self as ::microfield::Field>::mul(*self, rhs); }} }}
impl core::ops::Neg for {type_name} {{ type Output = Self; fn neg(self) -> Self {{ <Self as ::microfield::Field>::neg(self) }} }}
impl core::fmt::Display for {type_name} {{ fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{ self.write_hex(f) }} }}
impl core::fmt::Debug for {type_name} {{ fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{ f.write_str(concat!(stringify!({type_name}), "(0x"))?; self.write_hex(f)?; f.write_str(")") }} }}"#
    )
}

fn fixed_limbs(value: &BigUint, limbs: usize) -> Vec<u64> {
    let mut result = value.to_u64_digits();
    result.resize(limbs, 0);
    result
}

fn montgomery_neg_inverse(modulus_low: u64) -> u64 {
    let mut inverse = 1_u64;
    for _ in 0..6 {
        inverse = inverse.wrapping_mul(2_u64.wrapping_sub(modulus_low.wrapping_mul(inverse)));
    }
    inverse.wrapping_neg()
}

fn rust_type_name(field_name: &str) -> String {
    let mut result = String::new();
    for token in field_name.split('_') {
        if token.as_bytes()[0].is_ascii_digit() {
            if result.is_empty() {
                result.push_str("Field");
            }
            result.push('_');
            result.push_str(token);
        } else {
            let mut bytes = token.bytes();
            result.push(char::from(
                bytes.next().expect("normalized token").to_ascii_uppercase(),
            ));
            result.extend(bytes.map(char::from));
        }
    }
    result
}

fn render_certificate_toml(certificate: &PocklingtonCertificate) -> String {
    let mut factors = certificate.factors.clone();
    factors.sort_unstable_by_key(|factor| factor.prime);
    let mut result = format!(
        "\n[certificate]\nalgorithm = \"{}\"\nknown_factor_product = \"{}\"\ncofactor = \"{}\"\n",
        certificate.algorithm, certificate.known_factor_product, certificate.cofactor
    );
    for factor in factors {
        let _ = write!(
            result,
            "\n[[certificate.factors]]\nprime = {}\nexponent = {}\nwitness = {}\n",
            factor.prime, factor.exponent, factor.witness
        );
    }
    result
}

fn generated_file(path: &str, bytes: Vec<u8>) -> Result<GeneratedFile, PrimeFieldFactoryError> {
    GeneratedFile::new(path, bytes)
        .map_err(|error| PrimeFieldFactoryError::Generation(error.to_string()))
}

fn bundle_descriptor(files: &[GeneratedFile]) -> Vec<u8> {
    let mut entries = files
        .iter()
        .map(|file| {
            (
                file.relative_path(),
                file.contents().len(),
                content_digest(file.contents()),
            )
        })
        .collect::<Vec<_>>();
    entries.sort_unstable_by_key(|entry| entry.0);
    serde_json::to_vec(&entries).expect("bundle descriptor contains only serializable scalars")
}

fn write_artifacts(
    directory: &Path,
    artifacts: &GeneratedArtifacts,
) -> Result<(), PrimeFieldFactoryError> {
    for generated in artifacts.files() {
        let path = directory.join(generated.relative_path());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| {
                io_error("create cache directory", parent.to_path_buf(), source)
            })?;
        }
        let mut file = File::create(&path)
            .map_err(|source| io_error("create cache payload", path.clone(), source))?;
        file.write_all(generated.contents())
            .and_then(|()| file.sync_all())
            .map_err(|source| io_error("write cache payload", path, source))?;
    }
    Ok(())
}

fn verify_cache_directory(
    directory: &Path,
    artifact_id: ArtifactId,
) -> Result<(), PrimeFieldFactoryError> {
    let lock_path = directory.join("microfield.lock");
    let lock_bytes = read_regular_cache_file(&lock_path, "read cached lock")?;
    let lock = MicrofieldLock::parse_json(&lock_bytes)?;
    if lock.artifact_id != artifact_id.to_string() {
        return Err(lock_error("cache directory key and lock ArtifactId differ"));
    }
    let bundle_path = directory.join("bundle.json");
    let bundle_bytes = read_regular_cache_file(&bundle_path, "read cached bundle index")?;
    verify_bundle_index(
        &bundle_bytes,
        &lock_bytes,
        &lock.field_id,
        &lock.artifact_id,
    )?;
    for (path, expected) in &lock.payload_digests {
        if !is_safe_relative_payload(path) {
            return Err(lock_error(
                "cached payload path is not a safe relative path",
            ));
        }
        let payload = directory.join(path);
        let bytes = read_regular_cache_file(&payload, "read cached payload")?;
        if content_digest(&bytes) != *expected {
            return Err(lock_error("cached payload digest mismatch"));
        }
    }
    Ok(())
}

fn bundle_file<'a>(
    bundle: &'a GeneratedArtifacts,
    path: &str,
) -> Result<&'a [u8], PrimeFieldFactoryError> {
    bundle
        .files()
        .iter()
        .find(|file| file.relative_path() == path)
        .map(GeneratedFile::contents)
        .ok_or_else(|| lock_error(format!("required bundle file `{path}` is missing")))
}

fn verify_bundle_index(
    bundle_bytes: &[u8],
    lock_bytes: &[u8],
    field_id: &str,
    artifact_id: &str,
) -> Result<(), PrimeFieldFactoryError> {
    let index: BundleIndex = serde_json::from_slice(bundle_bytes)
        .map_err(|error| lock_error(format!("invalid bundle index: {error}")))?;
    if index.schema != 1
        || index.field_id != field_id
        || index.artifact_id != artifact_id
        || index.lock_digest != content_digest(lock_bytes)
    {
        return Err(lock_error(
            "bundle index schema, identities or lock digest do not match",
        ));
    }
    Ok(())
}

fn read_regular_cache_file(
    path: &Path,
    operation: &'static str,
) -> Result<Vec<u8>, PrimeFieldFactoryError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|source| io_error("inspect cached file", path.to_path_buf(), source))?;
    if !metadata.file_type().is_file() {
        return Err(lock_error("cached file is not a regular file"));
    }
    fs::read(path).map_err(|source| io_error(operation, path.to_path_buf(), source))
}

fn is_safe_relative_payload(path: &str) -> bool {
    let mut components = Path::new(path).components();
    let Some(Component::Normal(_)) = components.next() else {
        return false;
    };
    components.all(|component| matches!(component, Component::Normal(_)))
}

fn lock_error(reason: impl Into<String>) -> PrimeFieldFactoryError {
    PrimeFieldFactoryError::Lock(reason.into())
}

fn io_error(
    operation: &'static str,
    path: PathBuf,
    source: std::io::Error,
) -> PrimeFieldFactoryError {
    PrimeFieldFactoryError::Io {
        operation,
        path,
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn representation_boundaries_follow_verified_isa_contracts() {
        assert_eq!(
            select_representation(&BigUint::from(251_u16)),
            PrimeRepresentationProfile::Canonical8
        );
        assert_eq!(
            select_representation(&BigUint::from(65_521_u32)),
            PrimeRepresentationProfile::Canonical16
        );
        assert_eq!(
            select_representation(&BigUint::from(4_294_967_291_u64)),
            PrimeRepresentationProfile::Canonical32
        );
        assert_eq!(
            select_representation(&BigUint::from(4_294_967_311_u64)),
            PrimeRepresentationProfile::Montgomery64 { limbs: 1 }
        );
    }

    #[test]
    fn montgomery_inverse_cancels_low_word() {
        for modulus in [3_u64, 4_294_967_311, u64::MAX - 58] {
            let inverse = montgomery_neg_inverse(modulus);
            assert_eq!(modulus.wrapping_mul(inverse).wrapping_add(1), 0);
        }
    }
}
