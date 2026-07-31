//! Deterministic generation plans and immutable artifact sets.

use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

use serde::Serialize;

use crate::{ArtifactBundleDigest, ArtifactId, FieldId, spec::error::GenerationError};

/// Static multiplication-shape plan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProductPlan {
    limb_bits: usize,
    input_limbs: usize,
    wide_limbs: usize,
    strategies: Vec<String>,
}

impl ProductPlan {
    /// Returns the selected private limb width.
    #[must_use]
    pub const fn limb_bits(&self) -> usize {
        self.limb_bits
    }

    /// Returns the number of limbs in a canonical element.
    #[must_use]
    pub const fn input_limbs(&self) -> usize {
        self.input_limbs
    }

    /// Returns the number of limbs in an unreduced product.
    #[must_use]
    pub const fn wide_limbs(&self) -> usize {
        self.wide_limbs
    }

    /// Returns accepted multiplication strategies in canonical order.
    #[must_use]
    pub fn strategies(&self) -> &[String] {
        &self.strategies
    }

    pub(crate) fn new(
        limb_bits: usize,
        input_limbs: usize,
        wide_limbs: usize,
        strategies: Vec<String>,
    ) -> Self {
        Self {
            limb_bits,
            input_limbs,
            wide_limbs,
            strategies,
        }
    }
}

/// One descending elimination step in polynomial reduction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FoldStep {
    source_bit: usize,
    xor_targets: Vec<usize>,
}

impl FoldStep {
    /// Returns the high product bit eliminated by this step.
    #[must_use]
    pub const fn source_bit(&self) -> usize {
        self.source_bit
    }

    /// Returns all bits toggled when the source bit is set.
    #[must_use]
    pub fn xor_targets(&self) -> &[usize] {
        &self.xor_targets
    }

    pub(crate) fn new(source_bit: usize, xor_targets: Vec<usize>) -> Self {
        Self {
            source_bit,
            xor_targets,
        }
    }
}

/// Auditable reduction plan derived from the modulus.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReductionPlan {
    algorithm: &'static str,
    input_bits: usize,
    output_bits: usize,
    steps: Vec<FoldStep>,
    proof_digest: String,
}

impl ReductionPlan {
    /// Returns the unreduced input width.
    #[must_use]
    pub const fn input_bits(&self) -> usize {
        self.input_bits
    }

    /// Returns the reduced output width.
    #[must_use]
    pub const fn output_bits(&self) -> usize {
        self.output_bits
    }

    /// Returns elimination steps in required descending execution order.
    #[must_use]
    pub fn steps(&self) -> &[FoldStep] {
        &self.steps
    }

    /// Returns a domain-separated digest of the canonical step list.
    #[must_use]
    pub fn proof_digest(&self) -> &str {
        &self.proof_digest
    }

    pub(crate) fn new(
        input_bits: usize,
        output_bits: usize,
        steps: Vec<FoldStep>,
        proof_digest: String,
    ) -> Self {
        Self {
            algorithm: "descending-polynomial-fold-v1",
            input_bits,
            output_bits,
            steps,
            proof_digest,
        }
    }
}

/// One statically scheduled inversion operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ExponentiationStep {
    /// Square the accumulator a fixed number of times.
    Square {
        /// Fixed repetition count.
        count: usize,
    },
    /// Multiply the accumulator by the original base.
    MultiplyBase,
}

/// Fixed schedule computing `a^(2^degree - 2)`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExponentiationPlan {
    algorithm: &'static str,
    exponent: &'static str,
    steps: Vec<ExponentiationStep>,
}

impl ExponentiationPlan {
    /// Returns the fixed, branch-free operation schedule.
    #[must_use]
    pub fn steps(&self) -> &[ExponentiationStep] {
        &self.steps
    }

    pub(crate) fn new(steps: Vec<ExponentiationStep>) -> Self {
        Self {
            algorithm: "binary-fixed-chain-v1",
            exponent: "2^degree-2",
            steps,
        }
    }
}

/// Complete immutable plan for one concrete generated representation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GenerationPlan {
    schema: u32,
    field_id: FieldId,
    artifact_id: ArtifactId,
    ir_version: u32,
    target_family: &'static str,
    product: ProductPlan,
    reduction: ReductionPlan,
    inversion: ExponentiationPlan,
}

impl GenerationPlan {
    /// Returns the semantic field identity.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.field_id
    }

    /// Returns the concrete artifact identity.
    #[must_use]
    pub const fn artifact_id(&self) -> ArtifactId {
        self.artifact_id
    }

    /// Returns the multiplication plan.
    #[must_use]
    pub const fn product(&self) -> &ProductPlan {
        &self.product
    }

    /// Returns the reduction plan.
    #[must_use]
    pub const fn reduction(&self) -> &ReductionPlan {
        &self.reduction
    }

    /// Returns the inversion plan.
    #[must_use]
    pub const fn inversion(&self) -> &ExponentiationPlan {
        &self.inversion
    }

    pub(crate) fn new(
        field_id: FieldId,
        artifact_id: ArtifactId,
        product: ProductPlan,
        reduction: ReductionPlan,
        inversion: ExponentiationPlan,
    ) -> Self {
        Self {
            schema: 1,
            field_id,
            artifact_id,
            ir_version: 1,
            target_family: "portable",
            product,
            reduction,
            inversion,
        }
    }
}

/// One immutable generated file with a safe relative path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedFile {
    relative_path: String,
    contents: Vec<u8>,
}

impl GeneratedFile {
    /// Returns the validated relative output path.
    #[must_use]
    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    /// Returns the complete file bytes.
    #[must_use]
    pub fn contents(&self) -> &[u8] {
        &self.contents
    }

    pub(crate) fn new(
        relative_path: impl Into<String>,
        contents: Vec<u8>,
    ) -> Result<Self, GenerationError> {
        let relative_path = relative_path.into();
        let path = Path::new(&relative_path);
        let safe = !relative_path.is_empty()
            && !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !safe {
            return Err(GenerationError::InvalidArtifactPath(relative_path));
        }
        Ok(Self {
            relative_path,
            contents,
        })
    }
}

/// Complete validated output of the pure generation use case.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedArtifacts {
    field_name: String,
    field_id: FieldId,
    artifact_id: ArtifactId,
    bundle_digest: ArtifactBundleDigest,
    files: Vec<GeneratedFile>,
}

impl GeneratedArtifacts {
    /// Returns the human-facing field name used as the publication directory.
    #[must_use]
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    /// Returns the semantic field identity.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.field_id
    }

    /// Returns the representation artifact identity.
    #[must_use]
    pub const fn artifact_id(&self) -> ArtifactId {
        self.artifact_id
    }

    /// Returns the integrity digest of the exact generated payload files.
    #[must_use]
    pub const fn bundle_digest(&self) -> ArtifactBundleDigest {
        self.bundle_digest
    }

    /// Returns generated files in stable lexical order.
    #[must_use]
    pub fn files(&self) -> &[GeneratedFile] {
        &self.files
    }

    pub(crate) fn new(
        field_name: String,
        field_id: FieldId,
        artifact_id: ArtifactId,
        bundle_digest: ArtifactBundleDigest,
        mut files: Vec<GeneratedFile>,
    ) -> Result<Self, GenerationError> {
        files.sort_unstable_by(|lhs, rhs| lhs.relative_path.cmp(&rhs.relative_path));
        let mut paths = BTreeSet::new();
        for file in &files {
            if !paths.insert(file.relative_path.clone()) {
                return Err(GenerationError::DuplicateArtifactPath(
                    file.relative_path.clone(),
                ));
            }
        }
        Ok(Self {
            field_name,
            field_id,
            artifact_id,
            bundle_digest,
            files,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{GeneratedArtifacts, GeneratedFile};
    use crate::{ArtifactBundleDigest, ArtifactId, FieldId, spec::error::GenerationError};

    #[test]
    fn generated_paths_cannot_escape_the_transaction_root() {
        for path in ["", "/absolute", "../escape", "nested/../../escape", "."] {
            assert!(matches!(
                GeneratedFile::new(path, Vec::new()),
                Err(GenerationError::InvalidArtifactPath(_))
            ));
        }
        assert!(GeneratedFile::new("nested/file.json", Vec::new()).is_ok());
    }

    #[test]
    fn duplicate_generated_paths_are_rejected() {
        let files = vec![
            GeneratedFile::new("same", vec![1]).expect("safe path"),
            GeneratedFile::new("same", vec![2]).expect("safe path"),
        ];
        assert!(matches!(
            GeneratedArtifacts::new(
                "field".to_owned(),
                FieldId::from_bytes([0; 32]),
                ArtifactId::from_bytes([0; 32]),
                ArtifactBundleDigest::from_bytes([0; 32]),
                files
            ),
            Err(GenerationError::DuplicateArtifactPath(path)) if path == "same"
        ));
    }
}
