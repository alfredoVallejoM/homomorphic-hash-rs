//! Generator-side application boundary.
//!
//! This module is available only with the `generator` feature. It keeps pure
//! domain models and use cases independent from filesystem and process I/O.

mod adapters;
mod artifact;
pub mod error;
mod identity;
pub mod model;
mod planner;
mod polynomial;
pub mod ports;
pub mod use_cases;
mod validation;

pub use adapters::{
    FileSystemArtifactSink, FileSystemError, JsonFileOracle, OracleError, SageOracle,
};
pub use artifact::ArtifactGenerator;
pub use planner::GenerationPlanner;
pub use use_cases::{Generator, GeneratorBuilder};
pub use validation::ValidationEngine;
