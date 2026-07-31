//! Generator commands expressed independently of CLI and filesystem details.

use std::path::Path;

use crate::spec::{
    ArtifactGenerator, GenerationPlanner, ValidationEngine,
    error::PipelineError,
    model::{
        FieldManifest, GeneratedArtifacts, GenerationPlan, NormalizedManifest, ReferenceVectorSet,
        ValidatedFieldSpec,
    },
    ports::{ArtifactSink, OraclePort, Publication},
};

/// Facade coordinating pure generator use cases through explicit ports.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Generator {
    validator: ValidationEngine,
    planner: GenerationPlanner,
    renderer: ArtifactGenerator,
}

impl Generator {
    /// Starts explicit generator configuration.
    #[must_use]
    pub const fn builder() -> GeneratorBuilder {
        GeneratorBuilder::new()
    }

    /// Loads and canonicalizes a manifest without mathematical certification.
    ///
    /// # Errors
    ///
    /// Returns a typed parsing or normalization failure.
    pub fn normalize(
        &self,
        manifest_path: impl AsRef<Path>,
    ) -> Result<NormalizedManifest, PipelineError> {
        Ok(FieldManifest::load(manifest_path)?.normalize()?)
    }

    /// Runs canonicalization and independent Rabin certification.
    ///
    /// # Errors
    ///
    /// Returns a typed manifest, normalization or validation failure.
    pub fn validate(
        &self,
        manifest_path: impl AsRef<Path>,
    ) -> Result<ValidatedFieldSpec, PipelineError> {
        let normalized = self.normalize(manifest_path)?;
        Ok(self.validator.validate(normalized)?)
    }

    /// Derives the complete portable generation plan.
    ///
    /// # Errors
    ///
    /// Returns a deterministic serialization failure.
    pub fn plan(&self, validated: &ValidatedFieldSpec) -> Result<GenerationPlan, PipelineError> {
        Ok(self.planner.plan(validated)?)
    }

    /// Executes the pure pipeline through immutable artifact construction.
    ///
    /// # Errors
    ///
    /// Returns any typed failure from parsing through artifact rendering.
    pub fn generate(
        &self,
        manifest_path: impl AsRef<Path>,
    ) -> Result<GeneratedArtifacts, PipelineError> {
        let validated = self.validate(manifest_path)?;
        let plan = self.plan(&validated)?;
        Ok(self.renderer.generate(&validated, &plan)?)
    }

    /// Generates and transactionally publishes one complete artifact unit.
    ///
    /// # Errors
    ///
    /// Returns a pipeline failure or an adapter error. Publication adapters
    /// must preserve the last committed unit when committing fails.
    pub fn emit<S: ArtifactSink>(
        &self,
        manifest_path: impl AsRef<Path>,
        sink: &S,
    ) -> Result<Publication, PipelineError> {
        let artifacts = self.generate(manifest_path)?;
        sink.publish(&artifacts)
            .map_err(|error| PipelineError::Adapter(error.to_string()))
    }

    /// Checks whether committed artifacts equal a clean regeneration.
    ///
    /// # Errors
    ///
    /// Returns a pipeline failure or an adapter read error.
    pub fn check<S: ArtifactSink>(
        &self,
        manifest_path: impl AsRef<Path>,
        sink: &S,
    ) -> Result<bool, PipelineError> {
        let artifacts = self.generate(manifest_path)?;
        sink.matches(&artifacts)
            .map_err(|error| PipelineError::Adapter(error.to_string()))
    }

    /// Requests vectors from an external oracle and verifies their envelope.
    ///
    /// # Errors
    ///
    /// Returns a pipeline or oracle error, an unsupported vector schema, or a
    /// field identity mismatch.
    pub fn vectors<O: OraclePort>(
        &self,
        manifest_path: impl AsRef<Path>,
        oracle: &O,
    ) -> Result<ReferenceVectorSet, PipelineError> {
        let validated = self.validate(manifest_path)?;
        let vectors = oracle
            .generate(&validated)
            .map_err(|error| PipelineError::Adapter(error.to_string()))?;
        let descriptor = validated.normalized().descriptor();
        vectors.validate_for(
            validated.field_id(),
            descriptor.degree(),
            descriptor.canonical_bytes(),
        )?;
        Ok(vectors)
    }
}

/// Builder for explicit validation safety policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratorBuilder {
    maximum_degree: usize,
}

impl GeneratorBuilder {
    /// Creates a builder with the default degree safety limit.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            maximum_degree: crate::spec::model::SCHEMA_V1_MAXIMUM_DEGREE,
        }
    }

    /// Changes the maximum degree accepted by Rabin validation.
    #[must_use]
    pub const fn maximum_degree(mut self, maximum_degree: usize) -> Self {
        self.maximum_degree = maximum_degree;
        self
    }

    /// Builds an immutable generator facade.
    #[must_use]
    pub const fn build(self) -> Generator {
        Generator {
            validator: ValidationEngine::with_maximum_degree(self.maximum_degree),
            planner: GenerationPlanner,
            renderer: ArtifactGenerator,
        }
    }
}

impl Default for GeneratorBuilder {
    fn default() -> Self {
        Self::new()
    }
}
