//! Typed external-oracle reference vector models.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{FieldId, spec::error::ReferenceVectorError};

/// Current strict reference-vector schema.
pub const REFERENCE_VECTOR_SCHEMA_VERSION: u32 = 2;

/// Deterministic input derivation required by schema v2.
pub const REFERENCE_VECTOR_GENERATION_ALGORITHM: &str = "sha256-labeled-v1";

/// Maximum number of cases accepted in one external vector set.
pub const REFERENCE_VECTOR_MAXIMUM_CASES: usize = 4096;

/// Maximum serialized JSON size accepted from an external oracle.
pub const REFERENCE_VECTOR_MAXIMUM_JSON_BYTES: usize = 8 * 1024 * 1024;

/// Maximum serialized exponent length accepted by a `pow` case.
pub const REFERENCE_VECTOR_MAXIMUM_EXPONENT_BYTES: usize = 4096;

/// Identifies the independent program that calculated a vector set.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleMetadata {
    name: String,
    version: String,
}

impl OracleMetadata {
    /// Returns the oracle implementation name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the exact oracle version reported during generation.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// Records how deterministic input elements were derived.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VectorGeneration {
    algorithm: String,
    seed_hex: String,
}

impl VectorGeneration {
    /// Returns the versioned deterministic derivation algorithm.
    #[must_use]
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// Returns the 256-bit lowercase seed.
    #[must_use]
    pub fn seed_hex(&self) -> &str {
        &self.seed_hex
    }
}

/// One fully typed operation calculated by an independent oracle.
///
/// Field elements use canonical little-endian encoding. Wide values use
/// exactly twice the canonical byte width and contain polynomial
/// coefficients before reduction. Exponents are minimally encoded
/// little-endian byte strings, with zero represented as `"00"`.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VectorOperation {
    /// Certifies that a canonical element round-trips unchanged.
    Canonical {
        /// Canonical field-element bytes.
        element_hex_le: String,
    },
    /// Addition in the field.
    Add {
        /// Left operand.
        lhs_hex_le: String,
        /// Right operand.
        rhs_hex_le: String,
        /// Canonical result.
        output_hex_le: String,
    },
    /// Unreduced polynomial product.
    WideProduct {
        /// Left canonical operand.
        lhs_hex_le: String,
        /// Right canonical operand.
        rhs_hex_le: String,
        /// Double-width polynomial coefficients.
        output_wide_hex_le: String,
    },
    /// Reduction of a double-width polynomial.
    Reduce {
        /// Double-width polynomial coefficients.
        input_wide_hex_le: String,
        /// Canonical reduced result.
        output_hex_le: String,
    },
    /// Multiplication in the field.
    Multiply {
        /// Left operand.
        lhs_hex_le: String,
        /// Right operand.
        rhs_hex_le: String,
        /// Canonical result.
        output_hex_le: String,
    },
    /// Dedicated field squaring.
    Square {
        /// Canonical operand.
        input_hex_le: String,
        /// Canonical result.
        output_hex_le: String,
    },
    /// Multiplicative inversion.
    Invert {
        /// Canonical operand.
        input_hex_le: String,
        /// Canonical inverse, or `null` exactly when the input is zero.
        output_hex_le: Option<String>,
    },
    /// Exponentiation using an explicitly encoded exponent.
    Pow {
        /// Canonical base.
        base_hex_le: String,
        /// Minimal little-endian exponent bytes.
        exponent_hex_le: String,
        /// Canonical result.
        output_hex_le: String,
    },
    /// Multiplication by the polynomial-basis element `x`.
    MulByX {
        /// Canonical operand.
        input_hex_le: String,
        /// Canonical result.
        output_hex_le: String,
    },
}

/// One named reference case.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceVector {
    case: String,
    operation: VectorOperation,
}

impl ReferenceVector {
    /// Returns the stable case name used in diagnostics.
    #[must_use]
    pub fn case(&self) -> &str {
        &self.case
    }

    /// Returns the typed operation and its operands.
    #[must_use]
    pub const fn operation(&self) -> &VectorOperation {
        &self.operation
    }
}

/// Versioned set of vectors emitted by an independent oracle.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceVectorSet {
    schema: u32,
    field_id: String,
    oracle: OracleMetadata,
    generation: VectorGeneration,
    vectors: Vec<ReferenceVector>,
}

impl ReferenceVectorSet {
    /// Returns the schema version.
    #[must_use]
    pub const fn schema(&self) -> u32 {
        self.schema
    }

    /// Returns the field identity in lowercase hexadecimal.
    #[must_use]
    pub fn field_id(&self) -> &str {
        &self.field_id
    }

    /// Returns independent-oracle provenance.
    #[must_use]
    pub const fn oracle(&self) -> &OracleMetadata {
        &self.oracle
    }

    /// Returns deterministic generation metadata.
    #[must_use]
    pub const fn generation(&self) -> &VectorGeneration {
        &self.generation
    }

    /// Returns all typed operation cases.
    #[must_use]
    pub fn vectors(&self) -> &[ReferenceVector] {
        &self.vectors
    }

    pub(crate) fn validate_for(
        &self,
        field_id: FieldId,
        degree: usize,
        canonical_bytes: usize,
    ) -> Result<(), ReferenceVectorError> {
        if self.schema != REFERENCE_VECTOR_SCHEMA_VERSION {
            return Err(invalid(
                "schema",
                format!(
                    "expected {}, received {}",
                    REFERENCE_VECTOR_SCHEMA_VERSION, self.schema
                ),
            ));
        }
        if !valid_fixed_hex(&self.field_id, 32) {
            return Err(invalid(
                "field_id",
                "must be 32-byte fixed-width lowercase hexadecimal",
            ));
        }
        if self.field_id != field_id.to_string() {
            return Err(invalid(
                "field_id",
                format!("identifies {}, expected {field_id}", self.field_id),
            ));
        }
        validate_label("oracle.name", &self.oracle.name, 128)?;
        validate_label("oracle.version", &self.oracle.version, 128)?;
        if self.generation.algorithm != REFERENCE_VECTOR_GENERATION_ALGORITHM {
            return Err(invalid(
                "generation.algorithm",
                format!(
                    "expected `{REFERENCE_VECTOR_GENERATION_ALGORITHM}`, received `{}`",
                    self.generation.algorithm
                ),
            ));
        }
        if !valid_fixed_hex(&self.generation.seed_hex, 32) {
            return Err(invalid(
                "generation.seed_hex",
                "must be a 32-byte lowercase hexadecimal seed",
            ));
        }
        if self.vectors.is_empty() {
            return Err(invalid("vectors", "must contain at least one case"));
        }
        if self.vectors.len() > REFERENCE_VECTOR_MAXIMUM_CASES {
            return Err(invalid(
                "vectors",
                format!(
                    "contains {} cases; maximum is {REFERENCE_VECTOR_MAXIMUM_CASES}",
                    self.vectors.len()
                ),
            ));
        }

        let mut cases = BTreeSet::new();
        let mut coverage = Coverage::default();
        for (index, vector) in self.vectors.iter().enumerate() {
            let prefix = format!("vectors[{index}]");
            validate_case_name(&format!("{prefix}.case"), &vector.case)?;
            if !cases.insert(vector.case.as_str()) {
                return Err(invalid(
                    format!("{prefix}.case"),
                    format!("duplicate case `{}`", vector.case),
                ));
            }
            validate_operation(
                &prefix,
                &vector.operation,
                degree,
                canonical_bytes,
                &mut coverage,
            )?;
        }
        coverage.finish()
    }
}

#[derive(Default)]
struct Coverage(BTreeSet<&'static str>);

impl Coverage {
    fn mark(&mut self, operation: &'static str) {
        self.0.insert(operation);
    }

    fn finish(&self) -> Result<(), ReferenceVectorError> {
        let required = [
            "canonical",
            "add",
            "wide_product",
            "reduce",
            "multiply",
            "square",
            "invert_zero",
            "invert_nonzero",
            "pow",
            "mul_by_x",
        ];
        let missing = required
            .into_iter()
            .filter(|name| !self.0.contains(name))
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(invalid(
                "vectors",
                format!("missing required cases: {}", missing.join(", ")),
            ))
        }
    }
}

fn validate_operation(
    prefix: &str,
    operation: &VectorOperation,
    degree: usize,
    canonical_bytes: usize,
    coverage: &mut Coverage,
) -> Result<(), ReferenceVectorError> {
    let field_bits = degree;
    let (wide_bytes, wide_bits) = wide_dimensions(prefix, degree, canonical_bytes)?;
    let field = |name: &str, value: &str| {
        validate_bounded_hex(
            format!("{prefix}.operation.{name}"),
            value,
            canonical_bytes,
            field_bits,
        )
    };
    let wide = |name: &str, value: &str| {
        validate_bounded_hex(
            format!("{prefix}.operation.{name}"),
            value,
            wide_bytes,
            wide_bits,
        )
    };

    match operation {
        VectorOperation::Canonical { element_hex_le } => {
            field("element_hex_le", element_hex_le)?;
            coverage.mark("canonical");
        }
        VectorOperation::Add {
            lhs_hex_le,
            rhs_hex_le,
            output_hex_le,
        }
        | VectorOperation::Multiply {
            lhs_hex_le,
            rhs_hex_le,
            output_hex_le,
        } => {
            validate_binary(&field, lhs_hex_le, rhs_hex_le, output_hex_le)?;
            if matches!(operation, VectorOperation::Add { .. }) {
                coverage.mark("add");
            } else {
                coverage.mark("multiply");
            }
        }
        VectorOperation::WideProduct {
            lhs_hex_le,
            rhs_hex_le,
            output_wide_hex_le,
        } => {
            validate_operands(&field, lhs_hex_le, rhs_hex_le)?;
            wide("output_wide_hex_le", output_wide_hex_le)?;
            coverage.mark("wide_product");
        }
        VectorOperation::Reduce {
            input_wide_hex_le,
            output_hex_le,
        } => {
            wide("input_wide_hex_le", input_wide_hex_le)?;
            field("output_hex_le", output_hex_le)?;
            coverage.mark("reduce");
        }
        VectorOperation::Square {
            input_hex_le,
            output_hex_le,
        }
        | VectorOperation::MulByX {
            input_hex_le,
            output_hex_le,
        } => {
            validate_unary(&field, input_hex_le, output_hex_le)?;
            if matches!(operation, VectorOperation::Square { .. }) {
                coverage.mark("square");
            } else {
                coverage.mark("mul_by_x");
            }
        }
        VectorOperation::Invert {
            input_hex_le,
            output_hex_le,
        } => validate_invert(
            prefix,
            &field,
            input_hex_le,
            output_hex_le.as_deref(),
            coverage,
        )?,
        VectorOperation::Pow {
            base_hex_le,
            exponent_hex_le,
            output_hex_le,
        } => validate_pow(
            prefix,
            &field,
            base_hex_le,
            exponent_hex_le,
            output_hex_le,
            coverage,
        )?,
    }
    Ok(())
}

fn wide_dimensions(
    path: &str,
    degree: usize,
    canonical_bytes: usize,
) -> Result<(usize, usize), ReferenceVectorError> {
    let bytes = canonical_bytes
        .checked_mul(2)
        .ok_or_else(|| invalid(path, "wide encoding width overflow"))?;
    let bits = degree
        .checked_mul(2)
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| invalid(path, "wide encoding bit width overflow"))?;
    Ok((bytes, bits))
}

fn validate_operands(
    field: &impl Fn(&str, &str) -> Result<(), ReferenceVectorError>,
    lhs: &str,
    rhs: &str,
) -> Result<(), ReferenceVectorError> {
    field("lhs_hex_le", lhs)?;
    field("rhs_hex_le", rhs)
}

fn validate_binary(
    field: &impl Fn(&str, &str) -> Result<(), ReferenceVectorError>,
    lhs: &str,
    rhs: &str,
    output: &str,
) -> Result<(), ReferenceVectorError> {
    validate_operands(field, lhs, rhs)?;
    field("output_hex_le", output)
}

fn validate_unary(
    field: &impl Fn(&str, &str) -> Result<(), ReferenceVectorError>,
    input: &str,
    output: &str,
) -> Result<(), ReferenceVectorError> {
    field("input_hex_le", input)?;
    field("output_hex_le", output)
}

fn validate_invert(
    prefix: &str,
    field: &impl Fn(&str, &str) -> Result<(), ReferenceVectorError>,
    input: &str,
    output: Option<&str>,
    coverage: &mut Coverage,
) -> Result<(), ReferenceVectorError> {
    field("input_hex_le", input)?;
    if let Some(output) = output {
        field("output_hex_le", output)?;
    }
    let input_is_zero = input.bytes().all(|byte| byte == b'0');
    if input_is_zero != output.is_none() {
        return Err(invalid(
            format!("{prefix}.operation.output_hex_le"),
            "must be null exactly when the inversion input is zero",
        ));
    }
    coverage.mark(if input_is_zero {
        "invert_zero"
    } else {
        "invert_nonzero"
    });
    Ok(())
}

fn validate_pow(
    prefix: &str,
    field: &impl Fn(&str, &str) -> Result<(), ReferenceVectorError>,
    base: &str,
    exponent: &str,
    output: &str,
    coverage: &mut Coverage,
) -> Result<(), ReferenceVectorError> {
    field("base_hex_le", base)?;
    validate_exponent(format!("{prefix}.operation.exponent_hex_le"), exponent)?;
    field("output_hex_le", output)?;
    coverage.mark("pow");
    Ok(())
}

fn validate_bounded_hex(
    path: String,
    value: &str,
    bytes: usize,
    meaningful_bits: usize,
) -> Result<(), ReferenceVectorError> {
    if !valid_fixed_hex(value, bytes) {
        return Err(invalid(
            path,
            format!("must be {bytes}-byte fixed-width lowercase hexadecimal"),
        ));
    }
    let complete_bytes = meaningful_bits / 8;
    let remaining_bits = meaningful_bits % 8;
    for index in complete_bytes..bytes {
        let byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .expect("fixed lowercase hexadecimal was already validated");
        let valid = if index == complete_bytes && remaining_bits != 0 {
            byte >> remaining_bits == 0
        } else {
            byte == 0
        };
        if !valid {
            return Err(invalid(path, "sets bits outside the declared width"));
        }
    }
    Ok(())
}

fn validate_exponent(path: String, value: &str) -> Result<(), ReferenceVectorError> {
    if value.is_empty()
        || !value.len().is_multiple_of(2)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid(
            path,
            "must be non-empty even-length lowercase hexadecimal",
        ));
    }
    let bytes = value.len() / 2;
    if bytes > REFERENCE_VECTOR_MAXIMUM_EXPONENT_BYTES {
        return Err(invalid(
            path,
            format!("contains {bytes} bytes; maximum is {REFERENCE_VECTOR_MAXIMUM_EXPONENT_BYTES}"),
        ));
    }
    if bytes > 1 && value.ends_with("00") {
        return Err(invalid(
            path,
            "must use minimal little-endian encoding without a zero high byte",
        ));
    }
    Ok(())
}

fn validate_label(
    path: &'static str,
    value: &str,
    maximum: usize,
) -> Result<(), ReferenceVectorError> {
    if value.is_empty()
        || value.len() > maximum
        || value.trim() != value
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
    {
        Err(invalid(
            path,
            format!("must be 1..={maximum} printable ASCII characters without edge whitespace"),
        ))
    } else {
        Ok(())
    }
}

fn validate_case_name(path: &str, value: &str) -> Result<(), ReferenceVectorError> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(invalid(
            path,
            "must be 1..=64 stable lowercase ASCII letters, digits or underscores",
        ))
    }
}

fn valid_fixed_hex(value: &str, bytes: usize) -> bool {
    value.len() == bytes * 2
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn invalid(path: impl Into<String>, reason: impl Into<String>) -> ReferenceVectorError {
    ReferenceVectorError::new(path.into(), reason.into())
}
