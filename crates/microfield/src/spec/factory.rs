//! Public static factory for certified binary field source packages.

use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
};

use crate::{ArtifactBundleDigest, ArtifactId, FieldId};

use super::{
    ArtifactGenerator, GenerationPlanner, ValidationEngine,
    error::PipelineError,
    identity::generated_package_digest,
    model::{
        FieldManifest, GeneratedArtifacts, PortableOptimizationPlan, PortableReductionStrategy,
        ValidatedFieldSpec,
    },
};

/// Failure while configuring, certifying, rendering or emitting a field.
#[derive(Debug)]
pub enum BinaryFieldFactoryError {
    /// A required builder input was not supplied.
    MissingInput(&'static str),
    /// The existing manifest/validation/generation pipeline rejected the field.
    Pipeline(PipelineError),
    /// Atomic source emission failed.
    Io {
        /// Operation that failed.
        operation: &'static str,
        /// Affected filesystem path.
        path: PathBuf,
        /// Operating-system failure.
        source: std::io::Error,
    },
}

impl fmt::Display for BinaryFieldFactoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInput(input) => write!(formatter, "missing binary field input `{input}`"),
            Self::Pipeline(error) => error.fmt(formatter),
            Self::Io {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "{operation} `{}` failed: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for BinaryFieldFactoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pipeline(error) => Some(error),
            Self::Io { source, .. } => Some(source),
            Self::MissingInput(_) => None,
        }
    }
}

impl From<PipelineError> for BinaryFieldFactoryError {
    fn from(error: PipelineError) -> Self {
        Self::Pipeline(error)
    }
}

/// Static factory that certifies one binary polynomial field and emits Rust.
#[derive(Clone, Debug)]
pub struct BinaryFieldFactory {
    manifest: FieldManifest,
    maximum_degree: usize,
}

impl BinaryFieldFactory {
    /// Starts construction from explicit mathematical parameters.
    #[must_use]
    pub const fn builder() -> BinaryFieldFactoryBuilder {
        BinaryFieldFactoryBuilder::new()
    }

    /// Loads the same strict schema-v1 manifest accepted by `microfield-gen`.
    ///
    /// # Errors
    ///
    /// Returns a typed parsing or I/O error for an invalid manifest.
    pub fn from_manifest(path: impl AsRef<Path>) -> Result<Self, BinaryFieldFactoryError> {
        let manifest = FieldManifest::load(path).map_err(PipelineError::from)?;
        Ok(Self {
            manifest,
            maximum_degree: super::model::SCHEMA_V1_MAXIMUM_DEGREE,
        })
    }

    /// Certifies the modulus, derives immutable plans and renders one module.
    ///
    /// # Errors
    ///
    /// Returns a typed normalization, Rabin validation or generation error.
    pub fn generate(&self) -> Result<GeneratedFieldPackage, BinaryFieldFactoryError> {
        let normalized = self
            .manifest
            .clone()
            .normalize()
            .map_err(PipelineError::from)?;
        let validated = ValidationEngine::with_maximum_degree(self.maximum_degree)
            .validate(normalized)
            .map_err(PipelineError::from)?;
        let plan = GenerationPlanner
            .plan(&validated)
            .map_err(PipelineError::from)?;
        let artifacts = ArtifactGenerator
            .generate(&validated, &plan)
            .map_err(PipelineError::from)?;
        let type_name = rust_type_name(validated.normalized().name());
        let source = render_rust_module(&validated, &plan, &artifacts, &type_name);
        let source = source.into_bytes();
        let package_digest = generated_package_digest(artifacts.bundle_digest(), &source);
        Ok(GeneratedFieldPackage {
            type_name,
            source,
            package_digest,
            portable_optimization: plan.portable_optimization().clone(),
            artifacts,
        })
    }
}

/// Explicit builder for a binary polynomial field definition.
#[derive(Clone, Debug, Default)]
pub struct BinaryFieldFactoryBuilder {
    name: Option<String>,
    degree: Option<usize>,
    modulus_exponents: Option<Vec<usize>>,
    maximum_degree: Option<usize>,
}

impl BinaryFieldFactoryBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            name: None,
            degree: None,
            modulus_exponents: None,
            maximum_degree: None,
        }
    }

    /// Sets the stable snake-case presentation name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the extension degree `m` in `GF(2^m)`.
    #[must_use]
    pub const fn degree(mut self, degree: usize) -> Self {
        self.degree = Some(degree);
        self
    }

    /// Sets all non-zero modulus exponents, in any order.
    #[must_use]
    pub fn modulus_exponents(mut self, exponents: impl Into<Vec<usize>>) -> Self {
        self.modulus_exponents = Some(exponents.into());
        self
    }

    /// Applies a stricter validation policy than the schema-v1 ceiling.
    #[must_use]
    pub const fn maximum_degree(mut self, maximum_degree: usize) -> Self {
        self.maximum_degree = Some(maximum_degree);
        self
    }

    /// Builds the immutable factory through the canonical manifest parser.
    ///
    /// # Errors
    ///
    /// Returns a missing-input error or the same strict schema error as a file
    /// manifest. No alternate normalization path exists.
    pub fn build(self) -> Result<BinaryFieldFactory, BinaryFieldFactoryError> {
        let name = self
            .name
            .ok_or(BinaryFieldFactoryError::MissingInput("name"))?;
        let degree = self
            .degree
            .ok_or(BinaryFieldFactoryError::MissingInput("degree"))?;
        let modulus = self
            .modulus_exponents
            .ok_or(BinaryFieldFactoryError::MissingInput("modulus_exponents"))?;
        let quoted_name = toml::Value::String(name).to_string();
        let canonical_bytes = degree.div_ceil(8);
        let manifest_source = format!(
            "schema_version = 1\n\n\
             [field]\n\
             name = {quoted_name}\n\
             characteristic = 2\n\
             degree = {degree}\n\n\
             [field.basis]\n\
             kind = \"polynomial\"\n\
             coefficient_order = \"ascending\"\n\n\
             [field.modulus]\n\
             nonzero_exponents = {modulus:?}\n\n\
             [field.encoding]\n\
             byte_order = \"little\"\n\
             bit_order = \"lsb0\"\n\
             canonical_bytes = {canonical_bytes}\n\n\
             [build]\n\
             limb_bits = 64\n\
             product_strategies = [\"schoolbook\"]\n\
             reduction_style = \"generated_fold\"\n\
             requested_backends = [\"portable\"]\n"
        );
        let manifest = FieldManifest::parse_toml(&manifest_source).map_err(PipelineError::from)?;
        Ok(BinaryFieldFactory {
            manifest,
            maximum_degree: self
                .maximum_degree
                .unwrap_or(super::model::SCHEMA_V1_MAXIMUM_DEGREE),
        })
    }
}

/// Certified, deterministic Rust package ready for use from `build.rs`.
#[derive(Clone, Debug)]
pub struct GeneratedFieldPackage {
    type_name: String,
    source: Vec<u8>,
    package_digest: ArtifactBundleDigest,
    portable_optimization: PortableOptimizationPlan,
    artifacts: GeneratedArtifacts,
}

impl GeneratedFieldPackage {
    /// Returns the runtime-helper ABI required by this generated source.
    #[must_use]
    pub const fn codegen_abi_version(&self) -> u32 {
        2
    }

    /// Returns the presentation name from the manifest.
    #[must_use]
    pub fn field_name(&self) -> &str {
        self.artifacts.field_name()
    }

    /// Returns the deterministic public Rust type name.
    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Returns the semantic field identity.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.artifacts.field_id()
    }

    /// Returns the concrete generated-representation identity.
    #[must_use]
    pub const fn artifact_id(&self) -> ArtifactId {
        self.artifacts.artifact_id()
    }

    /// Authenticates the artifact bundle and exact generated Rust source.
    #[must_use]
    pub const fn package_digest(&self) -> ArtifactBundleDigest {
        self.package_digest
    }

    /// Returns the immutable portable strategy decision used by codegen.
    #[must_use]
    pub const fn portable_optimization(&self) -> &PortableOptimizationPlan {
        &self.portable_optimization
    }

    /// Returns the complete generated Rust module bytes.
    #[must_use]
    pub fn rust_source(&self) -> &[u8] {
        &self.source
    }

    /// Returns the associated certificate and planning artifacts.
    #[must_use]
    pub const fn artifacts(&self) -> &GeneratedArtifacts {
        &self.artifacts
    }

    /// Atomically writes `<field_name>.rs` below `output_directory`.
    ///
    /// # Errors
    ///
    /// Rejects non-directory output roots and non-regular existing targets;
    /// otherwise returns a contextual I/O failure without publishing a partial
    /// module.
    pub fn emit_rust(
        &self,
        output_directory: impl AsRef<Path>,
    ) -> Result<PathBuf, BinaryFieldFactoryError> {
        let output_directory = output_directory.as_ref();
        ensure_real_directory(output_directory)?;
        let target = output_directory.join(format!("{}.rs", self.field_name()));
        reject_special_target(&target)?;
        let (staging, mut file) = open_staging(output_directory, self.field_name())?;
        if let Err(source) = file.write_all(&self.source).and_then(|()| file.sync_all()) {
            let _ = fs::remove_file(&staging);
            return Err(io_error("write staged Rust module", staging, source));
        }
        drop(file);
        if let Err(source) = fs::rename(&staging, &target) {
            let _ = fs::remove_file(&staging);
            return Err(io_error("publish Rust module", target, source));
        }
        Ok(target)
    }
}

fn ensure_real_directory(path: &Path) -> Result<(), BinaryFieldFactoryError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => Err(io_error(
            "validate output directory",
            path.to_path_buf(),
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "must be a real directory"),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir_all(path)
            .map_err(|source| io_error("create output directory", path.to_path_buf(), source)),
        Err(source) => Err(io_error(
            "inspect output directory",
            path.to_path_buf(),
            source,
        )),
    }
}

fn reject_special_target(path: &Path) -> Result<(), BinaryFieldFactoryError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(_) => Err(io_error(
            "validate existing Rust module",
            path.to_path_buf(),
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "must be a regular file"),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error(
            "inspect existing Rust module",
            path.to_path_buf(),
            source,
        )),
    }
}

fn open_staging(
    output_directory: &Path,
    field_name: &str,
) -> Result<(PathBuf, File), BinaryFieldFactoryError> {
    for attempt in 0..100 {
        let path = output_directory.join(format!(
            ".{field_name}.microfield-staging-{}-{attempt}",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(io_error("create staged Rust module", path, source));
            }
        }
    }
    Err(io_error(
        "create staged Rust module",
        output_directory.to_path_buf(),
        std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "exhausted unique staging names",
        ),
    ))
}

fn io_error(
    operation: &'static str,
    path: PathBuf,
    source: std::io::Error,
) -> BinaryFieldFactoryError {
    BinaryFieldFactoryError::Io {
        operation,
        path,
        source,
    }
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
                bytes
                    .next()
                    .expect("normalized names contain no empty token")
                    .to_ascii_uppercase(),
            ));
            result.extend(bytes.map(char::from));
        }
    }
    result
}

fn artifact_bytes<'a>(artifacts: &'a GeneratedArtifacts, path: &str) -> &'a [u8] {
    artifacts
        .files()
        .iter()
        .find(|file| file.relative_path() == path)
        .expect("the complete artifact renderer always emits certification inputs")
        .contents()
}

fn render_rust_module(
    validated: &ValidatedFieldSpec,
    plan: &super::model::GenerationPlan,
    artifacts: &GeneratedArtifacts,
    type_name: &str,
) -> String {
    let descriptor = validated.normalized().descriptor();
    let degree = descriptor.degree();
    let limbs = degree.div_ceil(64);
    let bytes = descriptor.canonical_bytes();
    let mut modulus_tail_words = vec![0_u64; limbs];
    for exponent in &descriptor.modulus_exponents()[1..] {
        modulus_tail_words[exponent / 64] |= 1_u64 << (exponent % 64);
    }
    let low_tail = modulus_tail_words[0];
    let (multiply_body, square_body, dense_tail_constant) = match plan
        .portable_optimization()
        .reduction()
    {
        PortableReductionStrategy::LowTailFold => (
            format!(
                "Self(::microfield::__private::multiply_low_tail::<{limbs}, {}, {low_tail}>(self.0, rhs.0))",
                limbs * 2
            ),
            format!(
                "Self(::microfield::__private::square_low_tail::<{limbs}, {}, {low_tail}>(self.0))",
                limbs * 2
            ),
            String::new(),
        ),
        PortableReductionStrategy::SparseTermFold => (
            format!(
                "Self(::microfield::__private::multiply_sparse::<{limbs}, {}>(self.0, rhs.0, DEGREE, MODULUS_EXPONENTS_DESC))",
                limbs * 2
            ),
            format!(
                "Self(::microfield::__private::square_sparse::<{limbs}, {}>(self.0, DEGREE, MODULUS_EXPONENTS_DESC))",
                limbs * 2
            ),
            String::new(),
        ),
        PortableReductionStrategy::DenseWordFold => (
            format!(
                "Self(::microfield::__private::multiply_dense::<{limbs}, {}>(self.0, rhs.0, DEGREE, &MODULUS_TAIL_WORDS))",
                limbs * 2
            ),
            format!(
                "Self(::microfield::__private::square_dense::<{limbs}, {}>(self.0, DEGREE, &MODULUS_TAIL_WORDS))",
                limbs * 2
            ),
            format!("const MODULUS_TAIL_WORDS: [u64; {limbs}] = {modulus_tail_words:?};"),
        ),
    };
    let mut source = GENERATED_MODULE_TEMPLATE.to_owned();
    for (token, value) in [
        ("__TYPE__", type_name.to_owned()),
        (
            "__FIELD_NAME__",
            format!("{:?}", validated.normalized().name()),
        ),
        ("__DEGREE__", degree.to_string()),
        ("__LIMBS__", limbs.to_string()),
        ("__WIDE_LIMBS__", (limbs * 2).to_string()),
        ("__BYTES__", bytes.to_string()),
        (
            "__MODULUS__",
            format!("{:?}", descriptor.modulus_exponents()),
        ),
        ("__DENSE_TAIL_CONSTANT__", dense_tail_constant),
        ("__MULTIPLY_BODY__", multiply_body),
        ("__SQUARE_BODY__", square_body),
        (
            "__FIELD_ID__",
            format!("{:?}", artifacts.field_id().as_bytes()),
        ),
        (
            "__ARTIFACT_ID__",
            format!("{:?}", artifacts.artifact_id().as_bytes()),
        ),
        (
            "__DESCRIPTOR_JSON__",
            format!("{:?}", artifact_bytes(artifacts, "descriptor.json")),
        ),
        (
            "__CERTIFICATE_JSON__",
            format!("{:?}", artifact_bytes(artifacts, "certificate.json")),
        ),
    ] {
        source = source.replace(token, &value);
    }
    source
}

const GENERATED_MODULE_TEMPLATE: &str = r#"// Certified binary field generated by `microfield`.

const _: () = assert!(::microfield::__private::supports_codegen_abi(2));

const DEGREE: usize = __DEGREE__;
const MODULUS_EXPONENTS_DESC: &[usize] = &__MODULUS__;
__DENSE_TAIL_CONSTANT__
const DESCRIPTOR_JSON: &[u8] = &__DESCRIPTOR_JSON__;
const CERTIFICATE_JSON: &[u8] = &__CERTIFICATE_JSON__;

static SPEC: ::microfield::StaticFieldSpec = ::microfield::StaticFieldSpec::__from_generated(
    ::microfield::FieldId::from_bytes(__FIELD_ID__),
    ::microfield::ArtifactId::from_bytes(__ARTIFACT_ID__),
    __FIELD_NAME__,
    DEGREE as u32,
    __BYTES___u16,
    DESCRIPTOR_JSON,
    CERTIFICATE_JSON,
);

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct __TYPE__([u64; __LIMBS__]);

static PORTABLE_STRATEGY: ::microfield::__private::PortableStrategy<__TYPE__> =
    ::microfield::__private::PortableStrategy::new();

impl __TYPE__ {
    fn write_hex(self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let bytes = <Self as ::microfield::CanonicalEncoding>::to_canonical(self);
        for byte in bytes.iter().rev() {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl ::microfield::Field for __TYPE__ {
    const ZERO: Self = Self([0; __LIMBS__]);
    const ONE: Self = {
        let mut limbs = [0; __LIMBS__];
        limbs[0] = 1;
        Self(limbs)
    };

    #[inline]
    fn add(self, rhs: Self) -> Self { Self(::microfield::__private::add(self.0, rhs.0)) }
    #[inline]
    fn sub(self, rhs: Self) -> Self { <Self as ::microfield::Field>::add(self, rhs) }
    #[inline]
    fn neg(self) -> Self { self }
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        __MULTIPLY_BODY__
    }
    #[inline]
    fn is_zero(&self) -> bool { ::microfield::__private::is_zero(&self.0) }
}

impl ::microfield::Square for __TYPE__ {
    #[inline]
    fn square(self) -> Self {
        __SQUARE_BODY__
    }
}

impl ::microfield::Invert for __TYPE__ {
    fn invert(self) -> Option<Self> {
        ::microfield::__private::invert_itoh_tsujii::<Self, DEGREE>(self)
    }
}

impl ::microfield::Pow for __TYPE__ {}

impl ::microfield::CanonicalEncoding for __TYPE__ {
    type Repr = [u8; __BYTES__];

    fn from_canonical(repr: &Self::Repr) -> Result<Self, ::microfield::DecodeError> {
        if !::microfield::__private::canonical_padding_is_zero(repr, DEGREE) {
            return Err(::microfield::DecodeError::NonCanonicalValue);
        }
        Ok(Self(::microfield::__private::decode(repr)))
    }

    fn from_canonical_slice(bytes: &[u8]) -> Result<Self, ::microfield::DecodeError> {
        if bytes.len() != __BYTES__ {
            return Err(::microfield::DecodeError::LengthMismatch {
                expected: __BYTES__, actual: bytes.len(),
            });
        }
        let mut repr = [0_u8; __BYTES__];
        repr.copy_from_slice(bytes);
        <Self as ::microfield::CanonicalEncoding>::from_canonical(&repr)
    }

    fn to_canonical(self) -> Self::Repr {
        let mut repr = [0_u8; __BYTES__];
        ::microfield::__private::encode(self.0, &mut repr);
        repr
    }
}

impl ::microfield::ExtensionField for __TYPE__ {
    type Base = ::microfield::F2;
    const DEGREE: usize = DEGREE;

    fn frobenius(self, power: usize) -> Self {
        ::microfield::__private::frobenius::<Self, DEGREE>(self, power)
    }
    fn trace(self) -> Self::Base {
        ::microfield::__private::trace::<Self, DEGREE>(self)
    }
    fn norm(self) -> Self::Base {
        ::microfield::F2::from_bool(!<Self as ::microfield::Field>::is_zero(&self))
    }
}

impl ::microfield::BinaryPolynomialField for __TYPE__ {
    const MODULUS_DEGREE: usize = DEGREE;
    fn mul_by_x(self) -> Self {
        Self(::microfield::__private::mul_by_x(self.0, DEGREE, MODULUS_EXPONENTS_DESC))
    }
    fn from_polynomial_bytes_mod(bytes_le: &[u8]) -> Self {
        Self(::microfield::__private::reduce_polynomial_bytes(
            bytes_le, DEGREE, MODULUS_EXPONENTS_DESC,
        ))
    }
}

impl ::microfield::StaticField for __TYPE__ {
    fn spec() -> &'static ::microfield::StaticFieldSpec { &SPEC }
}

impl ::microfield::__private::PortableField for __TYPE__ {
    fn __portable_strategy() -> &'static ::microfield::__private::PortableStrategy<Self> {
        &PORTABLE_STRATEGY
    }
}

impl core::ops::Add for __TYPE__ {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { <Self as ::microfield::Field>::add(self, rhs) }
}
impl core::ops::AddAssign for __TYPE__ {
    fn add_assign(&mut self, rhs: Self) { *self = <Self as ::microfield::Field>::add(*self, rhs); }
}
impl core::ops::Sub for __TYPE__ {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { <Self as ::microfield::Field>::sub(self, rhs) }
}
impl core::ops::SubAssign for __TYPE__ {
    fn sub_assign(&mut self, rhs: Self) { *self = <Self as ::microfield::Field>::sub(*self, rhs); }
}
impl core::ops::Mul for __TYPE__ {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self { <Self as ::microfield::Field>::mul(self, rhs) }
}
impl core::ops::MulAssign for __TYPE__ {
    fn mul_assign(&mut self, rhs: Self) { *self = <Self as ::microfield::Field>::mul(*self, rhs); }
}
impl core::ops::Neg for __TYPE__ {
    type Output = Self;
    fn neg(self) -> Self { <Self as ::microfield::Field>::neg(self) }
}
impl core::fmt::Display for __TYPE__ {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { self.write_hex(formatter) }
}
impl core::fmt::Debug for __TYPE__ {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(concat!(stringify!(__TYPE__), "(0x"))?;
        self.write_hex(formatter)?;
        formatter.write_str(")")
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::{BinaryFieldFactory, BinaryFieldFactoryError, rust_type_name};

    #[test]
    fn type_names_are_deterministic_and_valid() {
        assert_eq!(rust_type_name("gf2_233_custom"), "Gf2_233Custom");
        assert_eq!(rust_type_name("9_bit"), "Field_9Bit");
    }

    #[test]
    fn builder_reports_each_required_input() {
        assert!(matches!(
            BinaryFieldFactory::builder().build(),
            Err(BinaryFieldFactoryError::MissingInput("name"))
        ));
        assert!(matches!(
            BinaryFieldFactory::builder().name("gf8").build(),
            Err(BinaryFieldFactoryError::MissingInput("degree"))
        ));
        assert!(matches!(
            BinaryFieldFactory::builder().name("gf8").degree(3).build(),
            Err(BinaryFieldFactoryError::MissingInput("modulus_exponents"))
        ));
    }
}
