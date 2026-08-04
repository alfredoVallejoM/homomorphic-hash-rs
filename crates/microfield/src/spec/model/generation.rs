//! Deterministic generation plans and immutable artifact sets.

use std::{
    collections::BTreeSet,
    fmt,
    path::{Component, Path},
};

use serde::Serialize;

use crate::{
    ArtifactBundleDigest, ArtifactId, Field, FieldId, Square, spec::error::GenerationError,
};

/// Degree shape used by the deterministic portable optimizer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortableDegreeClass {
    /// The degree is both a power of two and a multiple of the 64-bit limb size.
    PowerOfTwoLimbAligned,
    /// The degree is a multiple of the 64-bit limb size.
    LimbAligned,
    /// The most significant limb contains canonical padding bits.
    Unaligned,
}

/// Reduction family selected at generation time.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortableReductionStrategy {
    /// Word-aligned two-stage fold for a modulus tail of degree at most 32.
    LowTailFold,
    /// Descending fold that visits non-zero modulus terms directly.
    SparseTermFold,
    /// Descending fold using a packed word representation of a dense tail.
    DenseWordFold,
}

/// Structural class used by target-neutral verified ISA profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsaProfileClass {
    /// Power-of-two degree with a complete 64-bit most-significant limb.
    PowerOfTwoLimbAligned,
    /// Non-power-of-two degree with a complete 64-bit most-significant limb.
    LimbAligned,
    /// A field whose most-significant limb contains canonical padding bits.
    Unaligned,
}

/// Carry-less multiplication backend covered by a verified profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsaProfileBackend {
    /// x86-64 `PCLMULQDQ` operating on one 64-bit polynomial limb at a time.
    X86Pclmul,
    /// x86-64 `VPCLMULQDQ` operating on two independent field values per tile.
    X86Vpclmul,
    /// `AArch64` `PMULL` operating on one 64-bit polynomial limb at a time.
    Aarch64Pmull,
}

/// Selection status attached to a generated ISA profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsaProfileSelection {
    /// Correctness is certified, but automatic use awaits target measurements.
    ExplicitOnly,
}

/// Value-dependence of the complete generated ISA product plus reduction.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsaProfileSchedule {
    /// Product and generated reduction both execute a fixed schedule.
    Fixed,
    /// The generated reduction may branch on bits of the wide product.
    DataDependent,
}

/// Target-neutral profile derived exclusively from a validated binary field.
///
/// This profile certifies representation and reduction compatibility with the
/// generic 64-bit carry-less product bridge. It does not claim that a backend
/// is faster on an arbitrary CPU; generated external profiles therefore start
/// as [`IsaProfileSelection::ExplicitOnly`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VerifiedIsaProfile {
    schema: u32,
    field_id: FieldId,
    profile_class: IsaProfileClass,
    limb_bits: usize,
    input_limbs: usize,
    wide_limbs: usize,
    layout: &'static str,
    product: &'static str,
    reduction_proof_digest: String,
    backends: [IsaProfileBackend; 3],
    selection: IsaProfileSelection,
    schedule: IsaProfileSchedule,
    profile_digest: String,
}

impl VerifiedIsaProfile {
    /// Returns the semantic field identity certified by this profile.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.field_id
    }

    /// Returns the representation class used to specialize generated source.
    #[must_use]
    pub const fn profile_class(&self) -> IsaProfileClass {
        self.profile_class
    }

    /// Returns the private 64-bit element limb count.
    #[must_use]
    pub const fn input_limbs(&self) -> usize {
        self.input_limbs
    }

    /// Returns the fixed unreduced product limb count.
    #[must_use]
    pub const fn wide_limbs(&self) -> usize {
        self.wide_limbs
    }

    /// Returns the ISA families whose generic adapters match this profile.
    #[must_use]
    pub const fn backends(&self) -> &[IsaProfileBackend; 3] {
        &self.backends
    }

    /// Returns whether the profile may participate in automatic selection.
    #[must_use]
    pub const fn selection(&self) -> IsaProfileSelection {
        self.selection
    }

    /// Returns value-dependence of the complete product and reduction path.
    #[must_use]
    pub const fn schedule(&self) -> IsaProfileSchedule {
        self.schedule
    }

    /// Returns the domain-separated digest of the profile without this field.
    #[must_use]
    pub fn profile_digest(&self) -> &str {
        &self.profile_digest
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        field_id: FieldId,
        profile_class: IsaProfileClass,
        input_limbs: usize,
        wide_limbs: usize,
        reduction_proof_digest: String,
        schedule: IsaProfileSchedule,
        profile_digest: String,
    ) -> Self {
        Self {
            schema: 1,
            field_id,
            profile_class,
            limb_bits: 64,
            input_limbs,
            wide_limbs,
            layout: "polynomial-limbs-little-endian-v1",
            product: "clmul64-schoolbook-v1",
            reduction_proof_digest,
            backends: [
                IsaProfileBackend::X86Pclmul,
                IsaProfileBackend::X86Vpclmul,
                IsaProfileBackend::Aarch64Pmull,
            ],
            selection: IsaProfileSelection::ExplicitOnly,
            schedule,
            profile_digest,
        }
    }
}

/// Auditable, static optimization decision for generated portable arithmetic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PortableOptimizationPlan {
    schema: u32,
    degree_class: PortableDegreeClass,
    multiplication: &'static str,
    squaring: &'static str,
    reduction: PortableReductionStrategy,
    inversion: &'static str,
    modulus_terms: usize,
}

impl PortableOptimizationPlan {
    /// Returns the shape of the extension degree.
    #[must_use]
    pub const fn degree_class(&self) -> PortableDegreeClass {
        self.degree_class
    }

    /// Returns the selected multiplication family.
    #[must_use]
    pub const fn multiplication(&self) -> &'static str {
        self.multiplication
    }

    /// Returns the selected dedicated squaring family.
    #[must_use]
    pub const fn squaring(&self) -> &'static str {
        self.squaring
    }

    /// Returns the selected reduction family.
    #[must_use]
    pub const fn reduction(&self) -> PortableReductionStrategy {
        self.reduction
    }

    /// Returns the selected inversion schedule family.
    #[must_use]
    pub const fn inversion(&self) -> &'static str {
        self.inversion
    }

    /// Returns the number of non-zero terms in the complete monic modulus.
    #[must_use]
    pub const fn modulus_terms(&self) -> usize {
        self.modulus_terms
    }

    pub(crate) const fn new(
        degree_class: PortableDegreeClass,
        reduction: PortableReductionStrategy,
        modulus_terms: usize,
    ) -> Self {
        Self {
            schema: 1,
            degree_class,
            multiplication: "set-bit-schoolbook-v1",
            squaring: "bit-spread-v1",
            reduction,
            inversion: "itoh-tsujii-binary-v1",
            modulus_terms,
        }
    }
}

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
    /// Copy the accumulator into the plan's single saved-value slot.
    SaveAccumulator,
    /// Square the accumulator a fixed number of times.
    Square {
        /// Fixed repetition count.
        count: usize,
    },
    /// Multiply the accumulator by the original base.
    MultiplyBase,
    /// Multiply the accumulator by the saved-value slot.
    MultiplySaved,
}

/// Exact abstract cost of a fixed exponentiation plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ExponentiationCost {
    multiplications: usize,
    squares: usize,
    saved_values: usize,
}

impl ExponentiationCost {
    /// Returns the number of field multiplications.
    #[must_use]
    pub const fn multiplications(self) -> usize {
        self.multiplications
    }

    /// Returns the number of dedicated field squares.
    #[must_use]
    pub const fn squares(self) -> usize {
        self.squares
    }

    /// Returns the maximum number of simultaneously saved intermediate values.
    #[must_use]
    pub const fn saved_values(self) -> usize {
        self.saved_values
    }
}

/// Symbolic invariant failure in a fixed exponentiation plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ExponentiationVerificationError {
    /// Binary extension inversion requires degree at least two.
    InvalidDegree {
        /// Rejected extension degree.
        degree: usize,
    },
    /// A multiplication read the saved-value slot before initialization.
    SavedValueUninitialized {
        /// Zero-based operation index.
        operation: usize,
    },
    /// Symbolic exponent storage sizing overflowed.
    SizeOverflow,
    /// The final exponent was not exactly `2^degree - 2`.
    IncorrectExponent {
        /// Expected binary extension degree.
        degree: usize,
        /// Significant bits in the exponent produced by the plan.
        actual_bits: usize,
    },
}

impl fmt::Display for ExponentiationVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDegree { degree } => {
                write!(
                    formatter,
                    "binary inversion degree must be >= 2, got {degree}"
                )
            }
            Self::SavedValueUninitialized { operation } => write!(
                formatter,
                "operation {operation} reads the saved-value slot before initialization"
            ),
            Self::SizeOverflow => formatter.write_str("symbolic exponent size overflow"),
            Self::IncorrectExponent {
                degree,
                actual_bits,
            } => write!(
                formatter,
                "chain does not compute 2^{degree}-2 (actual significant bits: {actual_bits})"
            ),
        }
    }
}

impl std::error::Error for ExponentiationVerificationError {}

/// Fixed, symbolically verified schedule computing `a^(2^degree - 2)`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExponentiationPlan {
    algorithm: &'static str,
    exponent: &'static str,
    degree: usize,
    steps: Vec<ExponentiationStep>,
    cost: ExponentiationCost,
}

impl ExponentiationPlan {
    /// Returns the stable algorithm family name.
    #[must_use]
    pub const fn algorithm(&self) -> &str {
        self.algorithm
    }

    /// Returns the binary extension degree targeted by this schedule.
    #[must_use]
    pub const fn degree(&self) -> usize {
        self.degree
    }

    /// Returns the fixed, branch-free operation schedule.
    #[must_use]
    pub fn steps(&self) -> &[ExponentiationStep] {
        &self.steps
    }

    /// Returns exact abstract operation counts for this schedule.
    #[must_use]
    pub const fn cost(&self) -> ExponentiationCost {
        self.cost
    }

    /// Verifies initialization order and the exact Fermat exponent.
    ///
    /// # Errors
    ///
    /// Returns a typed invariant error when the schedule is malformed.
    pub fn verify_symbolically(&self) -> Result<(), ExponentiationVerificationError> {
        if self.degree < 2 {
            return Err(ExponentiationVerificationError::InvalidDegree {
                degree: self.degree,
            });
        }

        let mut accumulator = vec![true];
        let mut saved = None;
        for (operation, step) in self.steps.iter().enumerate() {
            match step {
                ExponentiationStep::SaveAccumulator => saved = Some(accumulator.clone()),
                ExponentiationStep::Square { count } => {
                    let new_len = accumulator
                        .len()
                        .checked_add(*count)
                        .ok_or(ExponentiationVerificationError::SizeOverflow)?;
                    let mut shifted = Vec::with_capacity(new_len);
                    shifted.resize(*count, false);
                    shifted.extend_from_slice(&accumulator);
                    accumulator = shifted;
                }
                ExponentiationStep::MultiplyBase => add_exponents(&mut accumulator, &[true])?,
                ExponentiationStep::MultiplySaved => {
                    let saved = saved.as_deref().ok_or(
                        ExponentiationVerificationError::SavedValueUninitialized { operation },
                    )?;
                    add_exponents(&mut accumulator, saved)?;
                }
            }
        }

        trim_exponent(&mut accumulator);
        let correct = accumulator.len() == self.degree
            && !accumulator[0]
            && accumulator[1..].iter().all(|bit| *bit);
        if !correct {
            return Err(ExponentiationVerificationError::IncorrectExponent {
                degree: self.degree,
                actual_bits: accumulator.len(),
            });
        }
        Ok(())
    }

    /// Evaluates this abstract chain using field operations.
    ///
    /// Zero handling remains the inversion caller's responsibility; this
    /// method evaluates the exponent program exactly as written.
    ///
    /// # Errors
    ///
    /// Returns the same structural errors as [`Self::verify_symbolically`].
    pub fn evaluate_reference<F: Field + Square>(
        &self,
        value: F,
    ) -> Result<F, ExponentiationVerificationError> {
        self.verify_symbolically()?;
        let mut accumulator = value;
        let mut saved = None;
        for (operation, step) in self.steps.iter().enumerate() {
            match step {
                ExponentiationStep::SaveAccumulator => saved = Some(accumulator),
                ExponentiationStep::Square { count } => {
                    for _ in 0..*count {
                        accumulator = accumulator.square();
                    }
                }
                ExponentiationStep::MultiplyBase => accumulator = accumulator.mul(value),
                ExponentiationStep::MultiplySaved => {
                    let saved =
                        saved.ok_or(ExponentiationVerificationError::SavedValueUninitialized {
                            operation,
                        })?;
                    accumulator = accumulator.mul(saved);
                }
            }
        }
        Ok(accumulator)
    }

    pub(crate) fn new_itoh_tsujii_binary(degree: usize) -> Self {
        let target = degree - 1;
        let highest_bit = usize::BITS as usize - 1 - target.leading_zeros() as usize;
        let mut block = 1_usize;
        let mut steps = Vec::new();

        for bit_index in (0..highest_bit).rev() {
            steps.push(ExponentiationStep::SaveAccumulator);
            steps.push(ExponentiationStep::Square { count: block });
            steps.push(ExponentiationStep::MultiplySaved);
            block *= 2;
            if (target >> bit_index) & 1 != 0 {
                steps.push(ExponentiationStep::Square { count: 1 });
                steps.push(ExponentiationStep::MultiplyBase);
                block += 1;
            }
        }
        steps.push(ExponentiationStep::Square { count: 1 });

        let cost = exponentiation_cost(&steps);
        Self {
            algorithm: "itoh-tsujii-binary-v1",
            exponent: "2^degree-2",
            degree,
            steps,
            cost,
        }
    }
}

fn exponentiation_cost(steps: &[ExponentiationStep]) -> ExponentiationCost {
    let mut multiplications = 0;
    let mut squares = 0;
    let mut saved_values = 0;
    for step in steps {
        match step {
            ExponentiationStep::SaveAccumulator => saved_values = 1,
            ExponentiationStep::Square { count } => squares += count,
            ExponentiationStep::MultiplyBase | ExponentiationStep::MultiplySaved => {
                multiplications += 1;
            }
        }
    }
    ExponentiationCost {
        multiplications,
        squares,
        saved_values,
    }
}

fn add_exponents(
    accumulator: &mut Vec<bool>,
    rhs: &[bool],
) -> Result<(), ExponentiationVerificationError> {
    let required = accumulator
        .len()
        .max(rhs.len())
        .checked_add(1)
        .ok_or(ExponentiationVerificationError::SizeOverflow)?;
    accumulator.resize(required, false);
    let mut carry = false;
    for (index, output) in accumulator.iter_mut().enumerate() {
        let left = *output;
        let right = rhs.get(index).copied().unwrap_or(false);
        let sum = u8::from(left) + u8::from(right) + u8::from(carry);
        *output = sum & 1 == 1;
        carry = sum >= 2;
    }
    trim_exponent(accumulator);
    Ok(())
}

fn trim_exponent(exponent: &mut Vec<bool>) {
    while exponent.len() > 1 && exponent.last() == Some(&false) {
        exponent.pop();
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
    portable_optimization: PortableOptimizationPlan,
    verified_isa_profile: VerifiedIsaProfile,
}

impl GenerationPlan {
    /// Returns the serialized generation-plan schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema
    }

    /// Returns the intermediate-representation version.
    #[must_use]
    pub const fn ir_version(&self) -> u32 {
        self.ir_version
    }

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

    /// Returns the deterministic portable code-generation decision.
    #[must_use]
    pub const fn portable_optimization(&self) -> &PortableOptimizationPlan {
        &self.portable_optimization
    }

    /// Returns the generated target-neutral ISA compatibility profile.
    #[must_use]
    pub const fn verified_isa_profile(&self) -> &VerifiedIsaProfile {
        &self.verified_isa_profile
    }

    pub(crate) fn new(
        field_id: FieldId,
        artifact_id: ArtifactId,
        product: ProductPlan,
        reduction: ReductionPlan,
        inversion: ExponentiationPlan,
        portable_optimization: PortableOptimizationPlan,
        verified_isa_profile: VerifiedIsaProfile,
    ) -> Self {
        Self {
            schema: 2,
            field_id,
            artifact_id,
            ir_version: 4,
            target_family: "portable_with_verified_isa_profile",
            product,
            reduction,
            inversion,
            portable_optimization,
            verified_isa_profile,
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
    use super::{
        ExponentiationCost, ExponentiationPlan, ExponentiationStep,
        ExponentiationVerificationError, GeneratedArtifacts, GeneratedFile,
    };
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

    #[test]
    fn itoh_tsujii_chains_verify_across_schema_degree_range() {
        for degree in [2, 3, 4, 5, 7, 8, 31, 64, 127, 128, 255, 256, 4096] {
            let plan = ExponentiationPlan::new_itoh_tsujii_binary(degree);
            plan.verify_symbolically().expect("chain must verify");
            assert_eq!(plan.cost().squares(), degree - 1);
        }
    }

    #[test]
    fn symbolic_verifier_rejects_uninitialized_saved_slot() {
        let plan = ExponentiationPlan {
            algorithm: "invalid-test",
            exponent: "2^degree-2",
            degree: 3,
            steps: vec![ExponentiationStep::MultiplySaved],
            cost: ExponentiationCost {
                multiplications: 1,
                squares: 0,
                saved_values: 0,
            },
        };
        assert_eq!(
            plan.verify_symbolically(),
            Err(ExponentiationVerificationError::SavedValueUninitialized { operation: 0 })
        );
    }

    #[test]
    fn symbolic_verifier_rejects_incorrect_exponent() {
        let plan = ExponentiationPlan {
            algorithm: "invalid-test",
            exponent: "2^degree-2",
            degree: 3,
            steps: vec![ExponentiationStep::Square { count: 1 }],
            cost: ExponentiationCost {
                multiplications: 0,
                squares: 1,
                saved_values: 0,
            },
        };
        assert!(matches!(
            plan.verify_symbolically(),
            Err(ExponentiationVerificationError::IncorrectExponent { degree: 3, .. })
        ));
    }
}
