//! Infrastructure adapters for manifests, artifacts and external oracles.

mod filesystem;
mod oracle;

pub use filesystem::{FileSystemArtifactSink, FileSystemError};
pub use oracle::{JsonFileOracle, OracleError, SageOracle};
