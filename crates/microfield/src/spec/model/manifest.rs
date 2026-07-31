//! Strict manifest parsing and deterministic normalization.

use std::{
    collections::BTreeSet,
    fmt::Write as _,
    fs::{self, File},
    io::Read as _,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::spec::error::{ManifestError, NormalizationError};

/// Hard parser limit for schema-v1 manifest inputs.
pub const SCHEMA_V1_MAXIMUM_MANIFEST_BYTES: usize = 64 * 1024;

/// Hard mathematical degree limit for schema v1.
///
/// A validator may configure a lower policy limit, but never raise this
/// resource-safety ceiling.
pub const SCHEMA_V1_MAXIMUM_DEGREE: usize = 4096;

/// Parsed, untrusted field manifest.
#[derive(Clone, Debug)]
pub struct FieldManifest(RawManifest);

impl FieldManifest {
    /// Parses a strict schema-v1 TOML manifest.
    ///
    /// # Errors
    ///
    /// Returns a typed error for malformed TOML, unknown keys or unsupported
    /// schema versions.
    pub fn parse_toml(source: &str) -> Result<Self, ManifestError> {
        reject_oversized_input(source.len() as u64)?;
        let value = source
            .parse::<toml::Value>()
            .map_err(|error| ManifestError::Syntax(error.to_string()))?;
        reject_unknown_keys(&value)?;
        let raw = value
            .try_into::<RawManifest>()
            .map_err(|error| ManifestError::Syntax(error.to_string()))?;
        if raw.schema_version != 1 {
            return Err(ManifestError::UnsupportedSchema(raw.schema_version));
        }
        Ok(Self(raw))
    }

    /// Loads and parses a strict schema-v1 TOML manifest.
    ///
    /// # Errors
    ///
    /// Returns a typed error if the file cannot be read or the manifest is
    /// invalid.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let path = path.as_ref();
        let metadata = fs::metadata(path).map_err(|source| ManifestError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        reject_oversized_input(metadata.len())?;
        let file = File::open(path).map_err(|source| ManifestError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let mut source = String::new();
        file.take(SCHEMA_V1_MAXIMUM_MANIFEST_BYTES as u64 + 1)
            .read_to_string(&mut source)
            .map_err(|source| ManifestError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        Self::parse_toml(&source)
    }

    /// Validates structural rules and produces the unique canonical form.
    ///
    /// # Errors
    ///
    /// Returns an error when a schema-v1 invariant is violated.
    pub fn normalize(self) -> Result<NormalizedManifest, NormalizationError> {
        normalize(self.0)
    }
}

/// Canonical field identity descriptor.
///
/// Field names and build choices are deliberately absent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalFieldDescriptor {
    schema: u32,
    characteristic: String,
    degree: usize,
    basis: CanonicalBasis,
    modulus: Vec<usize>,
    encoding: CanonicalEncoding,
}

impl CanonicalFieldDescriptor {
    /// Returns the extension degree.
    #[must_use]
    pub const fn degree(&self) -> usize {
        self.degree
    }

    /// Returns non-zero modulus exponents in descending order.
    #[must_use]
    pub fn modulus_exponents(&self) -> &[usize] {
        &self.modulus
    }

    /// Returns the fixed canonical byte length.
    #[must_use]
    pub const fn canonical_bytes(&self) -> usize {
        self.encoding.bytes
    }

    /// Returns the field characteristic as a stable decimal string.
    #[must_use]
    pub fn characteristic(&self) -> &str {
        &self.characteristic
    }
}

/// Canonical, structurally valid manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedManifest {
    name: String,
    descriptor: CanonicalFieldDescriptor,
    build: NormalizedBuild,
    canonical_toml: String,
    identity_json: String,
}

impl NormalizedManifest {
    /// Returns the human-facing field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the semantic identity descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &CanonicalFieldDescriptor {
        &self.descriptor
    }

    /// Returns normalized build choices, which do not participate in
    /// [`crate::FieldId`].
    #[must_use]
    pub const fn build(&self) -> &NormalizedBuild {
        &self.build
    }

    /// Returns deterministic, human-readable normalized TOML.
    #[must_use]
    pub fn canonical_toml(&self) -> &str {
        &self.canonical_toml
    }

    /// Returns minified fixed-order JSON used as the field identity input.
    #[must_use]
    pub fn identity_json(&self) -> &str {
        &self.identity_json
    }
}

/// Normalized non-semantic build configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NormalizedBuild {
    limb_bits: usize,
    product_strategies: Vec<String>,
    reduction_style: String,
    requested_backends: Vec<String>,
}

impl NormalizedBuild {
    /// Returns the private representation limb width.
    #[must_use]
    pub const fn limb_bits(&self) -> usize {
        self.limb_bits
    }

    /// Returns product strategy names in canonical order.
    #[must_use]
    pub fn product_strategies(&self) -> &[String] {
        &self.product_strategies
    }

    /// Returns the requested reduction style.
    #[must_use]
    pub fn reduction_style(&self) -> &str {
        &self.reduction_style
    }

    /// Returns requested backend names in canonical order.
    #[must_use]
    pub fn requested_backends(&self) -> &[String] {
        &self.requested_backends
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    schema_version: u32,
    field: RawField,
    build: RawBuild,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawField {
    name: String,
    characteristic: usize,
    degree: usize,
    basis: RawBasis,
    modulus: RawModulus,
    encoding: RawEncoding,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBasis {
    kind: String,
    coefficient_order: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModulus {
    nonzero_exponents: Vec<usize>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEncoding {
    byte_order: String,
    bit_order: String,
    canonical_bytes: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBuild {
    limb_bits: usize,
    product_strategies: Vec<String>,
    reduction_style: String,
    requested_backends: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct CanonicalBasis {
    kind: &'static str,
    coefficient_order: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct CanonicalEncoding {
    byte_order: &'static str,
    bit_order: &'static str,
    bytes: usize,
}

fn normalize(raw: RawManifest) -> Result<NormalizedManifest, NormalizationError> {
    validate_name(&raw.field.name)?;
    validate_fixed_profile(&raw)?;
    let expected_bytes = validate_dimensions(&raw.field)?;
    let modulus = normalize_modulus(&raw.field)?;
    let build = normalize_build(raw.build)?;

    let descriptor = CanonicalFieldDescriptor {
        schema: 1,
        characteristic: "2".to_owned(),
        degree: raw.field.degree,
        basis: CanonicalBasis {
            kind: "polynomial",
            coefficient_order: "ascending",
        },
        modulus,
        encoding: CanonicalEncoding {
            byte_order: "little",
            bit_order: "lsb0",
            bytes: expected_bytes,
        },
    };
    let identity_json = serde_json::to_string(&descriptor)
        .map_err(|error| NormalizationError::Serialization(error.to_string()))?;
    let canonical_toml = canonical_toml(&raw.field.name, &descriptor, &build);

    Ok(NormalizedManifest {
        name: raw.field.name,
        descriptor,
        build,
        canonical_toml,
        identity_json,
    })
}

fn validate_fixed_profile(raw: &RawManifest) -> Result<(), NormalizationError> {
    require_supported("field.characteristic", &raw.field.characteristic, &2_usize)?;
    require_supported(
        "field.basis.kind",
        raw.field.basis.kind.as_str(),
        "polynomial",
    )?;
    require_supported(
        "field.basis.coefficient_order",
        raw.field.basis.coefficient_order.as_str(),
        "ascending",
    )?;
    require_supported(
        "field.encoding.byte_order",
        raw.field.encoding.byte_order.as_str(),
        "little",
    )?;
    require_supported(
        "field.encoding.bit_order",
        raw.field.encoding.bit_order.as_str(),
        "lsb0",
    )?;
    require_supported("build.limb_bits", &raw.build.limb_bits, &64_usize)?;
    require_supported(
        "build.reduction_style",
        raw.build.reduction_style.as_str(),
        "generated_fold",
    )
}

fn validate_dimensions(field: &RawField) -> Result<usize, NormalizationError> {
    if field.degree < 2 {
        return Err(invalid("field.degree", "must be at least 2"));
    }
    if field.degree > SCHEMA_V1_MAXIMUM_DEGREE {
        return Err(invalid(
            "field.degree",
            format!("exceeds schema-v1 safety limit {SCHEMA_V1_MAXIMUM_DEGREE}"),
        ));
    }
    let expected_bytes = field.degree.div_ceil(8);
    if field.encoding.canonical_bytes != expected_bytes {
        return Err(invalid(
            "field.encoding.canonical_bytes",
            format!(
                "must be ceil(degree / 8) = {expected_bytes}, received {}",
                field.encoding.canonical_bytes
            ),
        ));
    }
    Ok(expected_bytes)
}

fn normalize_modulus(field: &RawField) -> Result<Vec<usize>, NormalizationError> {
    let mut modulus = field.modulus.nonzero_exponents.clone();
    if modulus.len() < 3 {
        return Err(invalid(
            "field.modulus.nonzero_exponents",
            "must contain the leading term, a non-leading term and the constant term",
        ));
    }
    if modulus.len() > field.degree + 1 {
        return Err(invalid(
            "field.modulus.nonzero_exponents",
            "contains more terms than a binary polynomial of this degree",
        ));
    }
    let unique = modulus.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != modulus.len() {
        return Err(invalid(
            "field.modulus.nonzero_exponents",
            "contains duplicate exponents",
        ));
    }
    if modulus.iter().any(|exponent| *exponent > field.degree) {
        return Err(invalid(
            "field.modulus.nonzero_exponents",
            "contains an exponent greater than the degree",
        ));
    }
    modulus.sort_unstable_by(|lhs, rhs| rhs.cmp(lhs));
    if modulus.first() != Some(&field.degree) {
        return Err(invalid(
            "field.modulus.nonzero_exponents",
            "must contain the monic leading exponent equal to degree",
        ));
    }
    if modulus.last() != Some(&0) {
        return Err(invalid(
            "field.modulus.nonzero_exponents",
            "must contain a non-zero constant coefficient",
        ));
    }
    Ok(modulus)
}

fn normalize_build(raw: RawBuild) -> Result<NormalizedBuild, NormalizationError> {
    Ok(NormalizedBuild {
        limb_bits: 64,
        product_strategies: normalize_set(
            "build.product_strategies",
            raw.product_strategies,
            &["schoolbook"],
        )?,
        reduction_style: "generated_fold".to_owned(),
        requested_backends: normalize_set(
            "build.requested_backends",
            raw.requested_backends,
            &["portable"],
        )?,
    })
}

fn validate_name(name: &str) -> Result<(), NormalizationError> {
    let valid = !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('_')
        && !name.ends_with('_')
        && !name.contains("__")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(NormalizationError::InvalidName(name.to_owned()))
    }
}

fn require_supported<T>(
    path: &'static str,
    actual: &T,
    expected: &T,
) -> Result<(), NormalizationError>
where
    T: Eq + ToString + ?Sized,
{
    if actual == expected {
        Ok(())
    } else {
        Err(NormalizationError::UnsupportedValue {
            path,
            value: actual.to_string(),
        })
    }
}

fn normalize_set(
    path: &'static str,
    values: Vec<String>,
    allowed: &[&str],
) -> Result<Vec<String>, NormalizationError> {
    if values.is_empty() {
        return Err(invalid(path, "must not be empty"));
    }
    for value in &values {
        if !allowed.contains(&value.as_str()) {
            return Err(NormalizationError::UnsupportedValue {
                path,
                value: value.clone(),
            });
        }
    }
    let mut normalized = values;
    normalized.sort_unstable();
    normalized.dedup();
    Ok(normalized)
}

fn invalid(path: &'static str, reason: impl Into<String>) -> NormalizationError {
    NormalizationError::InvalidValue {
        path,
        reason: reason.into(),
    }
}

fn canonical_toml(
    name: &str,
    descriptor: &CanonicalFieldDescriptor,
    build: &NormalizedBuild,
) -> String {
    let modulus = join_usize(&descriptor.modulus);
    let products = join_quoted(&build.product_strategies);
    let backends = join_quoted(&build.requested_backends);
    format!(
        "schema_version = 1\n\n\
         [field]\n\
         name = \"{name}\"\n\
         characteristic = 2\n\
         degree = {degree}\n\n\
         [field.basis]\n\
         kind = \"polynomial\"\n\
         coefficient_order = \"ascending\"\n\n\
         [field.modulus]\n\
         nonzero_exponents = [{modulus}]\n\n\
         [field.encoding]\n\
         byte_order = \"little\"\n\
         bit_order = \"lsb0\"\n\
         canonical_bytes = {bytes}\n\n\
         [build]\n\
         limb_bits = 64\n\
         product_strategies = [{products}]\n\
         reduction_style = \"generated_fold\"\n\
         requested_backends = [{backends}]\n",
        degree = descriptor.degree,
        bytes = descriptor.encoding.bytes,
    )
}

fn join_usize(values: &[usize]) -> String {
    let mut result = String::new();
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            result.push_str(", ");
        }
        write!(result, "{value}").expect("writing to String cannot fail");
    }
    result
}

fn join_quoted(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{value}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

fn reject_unknown_keys(value: &toml::Value) -> Result<(), ManifestError> {
    check_table(value, "", &["schema_version", "field", "build"])?;
    let table = value
        .as_table()
        .ok_or_else(|| ManifestError::Syntax("document root must be a table".to_owned()))?;
    check_table(
        required(table, "field")?,
        "field",
        &[
            "name",
            "characteristic",
            "degree",
            "basis",
            "modulus",
            "encoding",
        ],
    )?;
    let field = required(table, "field")?
        .as_table()
        .ok_or_else(|| ManifestError::Syntax("`field` must be a table".to_owned()))?;
    check_table(
        required(field, "basis")?,
        "field.basis",
        &["kind", "coefficient_order"],
    )?;
    check_table(
        required(field, "modulus")?,
        "field.modulus",
        &["nonzero_exponents"],
    )?;
    check_table(
        required(field, "encoding")?,
        "field.encoding",
        &["byte_order", "bit_order", "canonical_bytes"],
    )?;
    check_table(
        required(table, "build")?,
        "build",
        &[
            "limb_bits",
            "product_strategies",
            "reduction_style",
            "requested_backends",
        ],
    )
}

fn check_table(value: &toml::Value, prefix: &str, allowed: &[&str]) -> Result<(), ManifestError> {
    let table = value.as_table().ok_or_else(|| {
        ManifestError::Syntax(format!(
            "`{}` must be a table",
            if prefix.is_empty() { "root" } else { prefix }
        ))
    })?;
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            return Err(ManifestError::UnknownKey(path));
        }
    }
    Ok(())
}

fn required<'a>(
    table: &'a toml::map::Map<String, toml::Value>,
    key: &str,
) -> Result<&'a toml::Value, ManifestError> {
    table
        .get(key)
        .ok_or_else(|| ManifestError::Syntax(format!("missing `{key}` table")))
}

fn reject_oversized_input(actual: u64) -> Result<(), ManifestError> {
    if actual > SCHEMA_V1_MAXIMUM_MANIFEST_BYTES as u64 {
        Err(ManifestError::InputTooLarge {
            actual,
            maximum: SCHEMA_V1_MAXIMUM_MANIFEST_BYTES,
        })
    } else {
        Ok(())
    }
}
