//! Typestate models for manifests, validation and generated artifacts.

mod generation;
mod manifest;
mod validation;
mod vectors;

pub use generation::{
    ExponentiationPlan, ExponentiationStep, FoldStep, GeneratedArtifacts, GeneratedFile,
    GenerationPlan, ProductPlan, ReductionPlan,
};
pub use manifest::{
    CanonicalFieldDescriptor, FieldManifest, NormalizedBuild, NormalizedManifest,
    SCHEMA_V1_MAXIMUM_DEGREE, SCHEMA_V1_MAXIMUM_MANIFEST_BYTES,
};
pub use validation::{
    CertificateBundle, IrreducibilityCertificate, RabinGcdCheck, ValidatedFieldSpec,
};
pub use vectors::{
    OracleMetadata, REFERENCE_VECTOR_GENERATION_ALGORITHM, REFERENCE_VECTOR_MAXIMUM_CASES,
    REFERENCE_VECTOR_MAXIMUM_EXPONENT_BYTES, REFERENCE_VECTOR_MAXIMUM_JSON_BYTES,
    REFERENCE_VECTOR_SCHEMA_VERSION, ReferenceVector, ReferenceVectorSet, VectorGeneration,
    VectorOperation,
};
