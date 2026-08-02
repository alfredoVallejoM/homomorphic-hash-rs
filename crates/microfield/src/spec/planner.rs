//! Pure deterministic plan construction.

use serde::Serialize;

use crate::spec::{
    error::GenerationError,
    identity::{artifact_id, isa_profile_digest, proof_digest},
    model::{
        ExponentiationPlan, FoldStep, GenerationPlan, IsaProfileClass, IsaProfileSchedule,
        PortableDegreeClass, PortableReductionStrategy, ProductPlan, ReductionPlan,
        ValidatedFieldSpec, VerifiedIsaProfile,
    },
    optimizer::PortableOptimizer,
};

/// Stateless planner for the portable version-4 intermediate representation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GenerationPlanner;

impl GenerationPlanner {
    /// Derives multiplication, reduction and inversion plans.
    ///
    /// # Errors
    ///
    /// Returns an error if canonical JSON serialization fails.
    pub fn plan(&self, validated: &ValidatedFieldSpec) -> Result<GenerationPlan, GenerationError> {
        let normalized = validated.normalized();
        let descriptor = normalized.descriptor();
        let build = normalized.build();
        let degree = descriptor.degree();
        let input_limbs = degree.div_ceil(build.limb_bits());

        let product = ProductPlan::new(
            build.limb_bits(),
            input_limbs,
            input_limbs * 2,
            build.product_strategies().to_vec(),
        );

        let tail = &descriptor.modulus_exponents()[1..];
        let steps = (degree..=(2 * degree - 2))
            .rev()
            .map(|source_bit| {
                FoldStep::new(
                    source_bit,
                    tail.iter()
                        .map(|exponent| source_bit - degree + exponent)
                        .collect(),
                )
            })
            .collect::<Vec<_>>();
        let step_bytes = serde_json::to_vec(&steps)
            .map_err(|error| GenerationError::Serialization(error.to_string()))?;
        let reduction = ReductionPlan::new(2 * degree, degree, steps, proof_digest(&step_bytes));

        let inversion = ExponentiationPlan::new_itoh_tsujii_binary(degree);
        inversion
            .verify_symbolically()
            .map_err(|error| GenerationError::InvalidInversionPlan(error.to_string()))?;
        let portable_optimization = PortableOptimizer::plan(validated);
        let profile_class = match portable_optimization.degree_class() {
            PortableDegreeClass::PowerOfTwoLimbAligned => IsaProfileClass::PowerOfTwoLimbAligned,
            PortableDegreeClass::LimbAligned => IsaProfileClass::LimbAligned,
            PortableDegreeClass::Unaligned => IsaProfileClass::Unaligned,
        };
        let isa_schedule = match portable_optimization.reduction() {
            PortableReductionStrategy::LowTailFold => IsaProfileSchedule::Fixed,
            PortableReductionStrategy::SparseTermFold
            | PortableReductionStrategy::DenseWordFold => IsaProfileSchedule::DataDependent,
        };
        let isa_profile_descriptor = IsaProfileDescriptor {
            schema: 1,
            field_id: validated.field_id(),
            profile_class,
            limb_bits: build.limb_bits(),
            input_limbs,
            wide_limbs: input_limbs * 2,
            layout: "polynomial-limbs-little-endian-v1",
            product: "clmul64-schoolbook-v1",
            reduction_proof_digest: reduction.proof_digest(),
            backends: ["x86_pclmul", "x86_vpclmul", "aarch64_pmull"],
            selection: "explicit_only",
            schedule: isa_schedule,
        };
        let isa_profile_bytes = serde_json::to_vec(&isa_profile_descriptor)
            .map_err(|error| GenerationError::Serialization(error.to_string()))?;
        let verified_isa_profile = VerifiedIsaProfile::new(
            validated.field_id(),
            profile_class,
            input_limbs,
            input_limbs * 2,
            reduction.proof_digest().to_owned(),
            isa_schedule,
            isa_profile_digest(&isa_profile_bytes),
        );

        let artifact_descriptor = ArtifactDescriptor {
            schema: 2,
            field_id: validated.field_id(),
            generator_version: env!("CARGO_PKG_VERSION"),
            ir_version: 4,
            target_family: "portable_with_verified_isa_profile",
            build,
            portable_optimization: &portable_optimization,
            verified_isa_profile: &verified_isa_profile,
        };
        let artifact_bytes = serde_json::to_vec(&artifact_descriptor)
            .map_err(|error| GenerationError::Serialization(error.to_string()))?;
        let artifact_id = artifact_id(&artifact_bytes);

        Ok(GenerationPlan::new(
            validated.field_id(),
            artifact_id,
            product,
            reduction,
            inversion,
            portable_optimization,
            verified_isa_profile,
        ))
    }
}

#[derive(Serialize)]
struct ArtifactDescriptor<'a> {
    schema: u32,
    field_id: crate::FieldId,
    generator_version: &'static str,
    ir_version: u32,
    target_family: &'static str,
    build: &'a crate::spec::model::NormalizedBuild,
    portable_optimization: &'a crate::spec::model::PortableOptimizationPlan,
    verified_isa_profile: &'a crate::spec::model::VerifiedIsaProfile,
}

#[derive(Serialize)]
struct IsaProfileDescriptor<'a> {
    schema: u32,
    field_id: crate::FieldId,
    profile_class: IsaProfileClass,
    limb_bits: usize,
    input_limbs: usize,
    wide_limbs: usize,
    layout: &'static str,
    product: &'static str,
    reduction_proof_digest: &'a str,
    backends: [&'static str; 3],
    selection: &'static str,
    schedule: IsaProfileSchedule,
}
