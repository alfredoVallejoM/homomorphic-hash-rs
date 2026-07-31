//! Imported-JSON and Sage process oracle adapters.

use std::{
    fmt,
    fs::{self, File},
    io::Read as _,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;

use crate::spec::{
    model::{REFERENCE_VECTOR_MAXIMUM_JSON_BYTES, ReferenceVectorSet, ValidatedFieldSpec},
    ports::OraclePort,
};

/// Failure while loading or executing an external oracle.
#[derive(Debug)]
pub struct OracleError(String);

impl fmt::Display for OracleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for OracleError {}

/// Adapter for pre-generated, independently produced vector JSON.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonFileOracle {
    path: PathBuf,
}

impl JsonFileOracle {
    /// Creates an imported-vector adapter.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the imported JSON path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl OraclePort for JsonFileOracle {
    type Error = OracleError;

    fn generate(&self, _validated: &ValidatedFieldSpec) -> Result<ReferenceVectorSet, Self::Error> {
        let metadata = fs::metadata(&self.path).map_err(|error| {
            OracleError(format!(
                "cannot inspect oracle vectors {}: {error}",
                self.path.display()
            ))
        })?;
        if metadata.len() > REFERENCE_VECTOR_MAXIMUM_JSON_BYTES as u64 {
            return Err(OracleError(format!(
                "oracle vector JSON has {} bytes; maximum is {REFERENCE_VECTOR_MAXIMUM_JSON_BYTES}",
                metadata.len()
            )));
        }
        let file = File::open(&self.path).map_err(|error| {
            OracleError(format!(
                "cannot read oracle vectors {}: {error}",
                self.path.display()
            ))
        })?;
        let mut bytes = Vec::new();
        file.take(REFERENCE_VECTOR_MAXIMUM_JSON_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                OracleError(format!(
                    "cannot read oracle vectors {}: {error}",
                    self.path.display()
                ))
            })?;
        if bytes.len() > REFERENCE_VECTOR_MAXIMUM_JSON_BYTES {
            return Err(OracleError(format!(
                "oracle vector JSON has more than {REFERENCE_VECTOR_MAXIMUM_JSON_BYTES} bytes"
            )));
        }
        serde_json::from_slice(&bytes)
            .map_err(|error| OracleError(format!("invalid oracle vector JSON: {error}")))
    }
}

/// Adapter that invokes `SageMath` as a separate non-runtime process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SageOracle {
    executable: PathBuf,
    script: PathBuf,
}

impl SageOracle {
    /// Creates a Sage adapter from executable and script paths.
    #[must_use]
    pub fn new(executable: impl Into<PathBuf>, script: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            script: script.into(),
        }
    }
}

impl OraclePort for SageOracle {
    type Error = OracleError;

    fn generate(&self, validated: &ValidatedFieldSpec) -> Result<ReferenceVectorSet, Self::Error> {
        let request = SageRequest {
            field_id: validated.field_id().to_string(),
            descriptor: validated.normalized().descriptor(),
        };
        let payload = serde_json::to_string(&request)
            .map_err(|error| OracleError(format!("cannot serialize Sage request: {error}")))?;
        let output = Command::new(&self.executable)
            .arg(&self.script)
            .arg("--payload")
            .arg(payload)
            .output()
            .map_err(|error| {
                OracleError(format!(
                    "cannot execute Sage `{}`: {error}",
                    self.executable.display()
                ))
            })?;
        if !output.status.success() {
            return Err(OracleError(format!(
                "Sage exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        if output.stdout.len() > REFERENCE_VECTOR_MAXIMUM_JSON_BYTES {
            return Err(OracleError(format!(
                "Sage vector JSON has {} bytes; maximum is {REFERENCE_VECTOR_MAXIMUM_JSON_BYTES}",
                output.stdout.len()
            )));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|error| OracleError(format!("invalid Sage vector JSON: {error}")))
    }
}

#[derive(Serialize)]
struct SageRequest<'a> {
    field_id: String,
    descriptor: &'a crate::spec::model::CanonicalFieldDescriptor,
}
