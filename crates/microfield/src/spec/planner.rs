//! Pure deterministic plan construction.

use serde::Serialize;

use crate::spec::{
    error::GenerationError,
    identity::{artifact_id, proof_digest},
    model::{
        ExponentiationPlan, ExponentiationStep, FoldStep, GenerationPlan, ProductPlan,
        ReductionPlan, ValidatedFieldSpec,
    },
    optimizer::PortableOptimizer,
};

/// Stateless planner for the portable version-2 intermediate representation.
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

        let mut inversion_steps = Vec::with_capacity(2 * degree - 3);
        for _ in 0..degree - 2 {
            inversion_steps.push(ExponentiationStep::Square { count: 1 });
            inversion_steps.push(ExponentiationStep::MultiplyBase);
        }
        inversion_steps.push(ExponentiationStep::Square { count: 1 });
        let inversion = ExponentiationPlan::new(inversion_steps);
        let portable_optimization = PortableOptimizer::plan(validated);

        let artifact_descriptor = ArtifactDescriptor {
            schema: 1,
            field_id: validated.field_id(),
            generator_version: env!("CARGO_PKG_VERSION"),
            ir_version: 2,
            target_family: "portable",
            build,
            portable_optimization: &portable_optimization,
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
}
