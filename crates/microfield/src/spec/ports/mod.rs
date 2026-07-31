//! I/O-independent ports required by generator use cases.

use crate::spec::model::{GeneratedArtifacts, ReferenceVectorSet, ValidatedFieldSpec};

/// Publication result returned by an artifact adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Publication {
    output_directory: std::path::PathBuf,
    replaced_existing: bool,
}

impl Publication {
    /// Returns the committed artifact directory.
    #[must_use]
    pub fn output_directory(&self) -> &std::path::Path {
        &self.output_directory
    }

    /// Reports whether a previous committed version was atomically replaced.
    #[must_use]
    pub const fn replaced_existing(&self) -> bool {
        self.replaced_existing
    }

    /// Creates an adapter-neutral publication result.
    #[must_use]
    pub fn new(output_directory: std::path::PathBuf, replaced_existing: bool) -> Self {
        Self {
            output_directory,
            replaced_existing,
        }
    }
}

/// Output port that atomically publishes a complete artifact unit.
pub trait ArtifactSink {
    /// Adapter-specific failure.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Publishes every file or leaves the previously committed unit intact.
    ///
    /// # Errors
    ///
    /// Returns an adapter-specific error if staging or publication fails.
    fn publish(&self, artifacts: &GeneratedArtifacts) -> Result<Publication, Self::Error>;

    /// Compares committed output byte-for-byte with an artifact set.
    ///
    /// # Errors
    ///
    /// Returns an adapter-specific error if committed output cannot be read.
    fn matches(&self, artifacts: &GeneratedArtifacts) -> Result<bool, Self::Error>;
}

/// Port implemented by independent field-arithmetic oracles.
pub trait OraclePort {
    /// Adapter-specific failure.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Produces externally computed vectors for the validated field.
    ///
    /// # Errors
    ///
    /// Returns an adapter-specific error when the oracle cannot run or its
    /// response is malformed.
    fn generate(&self, validated: &ValidatedFieldSpec) -> Result<ReferenceVectorSet, Self::Error>;
}
