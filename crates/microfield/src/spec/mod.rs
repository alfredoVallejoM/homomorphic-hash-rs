//! Generator-side application boundary.
//!
//! This module is available only with the `generator` feature. It keeps pure
//! domain models and use cases independent from filesystem and process I/O.

mod adapters;
mod artifact;
pub mod error;
mod factory;
mod identity;
pub mod model;
mod optimizer;
mod planner;
mod polynomial;
pub mod ports;
mod prime;
pub mod use_cases;
mod validation;

pub use adapters::{
    FileSystemArtifactSink, FileSystemError, JsonFileOracle, OracleError, SageOracle,
};
pub use artifact::ArtifactGenerator;
pub use factory::{
    BinaryFieldFactory, BinaryFieldFactoryBuilder, BinaryFieldFactoryError, GeneratedFieldPackage,
};
pub use planner::GenerationPlanner;
pub use prime::{
    GeneratedPrimeFieldPackage, GenerationLimits, GenerationProfile, MicrofieldLock,
    NormalizedPrimeManifest, PocklingtonCertificate, PocklingtonFactor, PrimeArtifactCache,
    PrimeCachePolicy, PrimeFieldFactory, PrimeFieldFactoryBuilder, PrimeFieldFactoryError,
    PrimeFieldManifest, PrimeManifestError, PrimeRepresentationProfile, PrimeValidationError,
    ValidatedPrimeField,
};
pub use use_cases::{Generator, GeneratorBuilder};
pub use validation::ValidationEngine;
