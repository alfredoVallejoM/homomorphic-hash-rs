//! Explicit dynamic-to-static generation bridge.

use core::fmt;

use crate::{
    DynFamilyKind, DynField, FieldId,
    spec::{
        BinaryFieldFactory, BinaryFieldFactoryError, GeneratedFieldPackage,
        GeneratedPrimeFieldPackage, PrimeFieldFactory, PrimeFieldFactoryError,
    },
};

/// Static package produced from one already validated dynamic context.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum StaticExportPackage {
    /// Certified binary polynomial source.
    Binary(Box<GeneratedFieldPackage>),
    /// Deterministically proven prime source.
    Prime(Box<GeneratedPrimeFieldPackage>),
}

impl StaticExportPackage {
    /// Returns the field identity preserved across the bridge.
    #[must_use]
    pub fn field_id(&self) -> FieldId {
        match self {
            Self::Binary(package) => package.field_id(),
            Self::Prime(package) => package.field_id(),
        }
    }
}

/// Failure while exporting a dynamic context to reviewed static source.
#[derive(Debug)]
#[non_exhaustive]
pub enum StaticExportError {
    /// Binary certification or generation failed.
    Binary(BinaryFieldFactoryError),
    /// Prime proof or generation failed.
    Prime(PrimeFieldFactoryError),
    /// Exported semantics did not preserve the dynamic identity.
    IdentityMismatch {
        /// Dynamic identity.
        dynamic: FieldId,
        /// Generated identity.
        generated: FieldId,
    },
}

impl fmt::Display for StaticExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binary(error) => error.fmt(formatter),
            Self::Prime(error) => error.fmt(formatter),
            Self::IdentityMismatch { dynamic, generated } => write!(
                formatter,
                "dynamic/static FieldId mismatch: {dynamic} != {generated}"
            ),
        }
    }
}

impl std::error::Error for StaticExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Binary(error) => Some(error),
            Self::Prime(error) => Some(error),
            Self::IdentityMismatch { .. } => None,
        }
    }
}

impl DynField {
    /// Replays the static certification pipeline over this context's exact
    /// exported manifest.
    ///
    /// # Errors
    ///
    /// Probable primes are rejected because they cannot authorize source.
    /// Any identity drift is reported after generation.
    pub fn generate_static(&self) -> Result<StaticExportPackage, StaticExportError> {
        let manifest = self.export_manifest();
        let package = match self.family() {
            DynFamilyKind::BinaryPolynomial => StaticExportPackage::Binary(Box::new(
                BinaryFieldFactory::from_manifest_toml(&manifest)
                    .map_err(StaticExportError::Binary)?
                    .generate()
                    .map_err(StaticExportError::Binary)?,
            )),
            DynFamilyKind::Prime => StaticExportPackage::Prime(Box::new(
                PrimeFieldFactory::from_manifest_toml(&manifest)
                    .map_err(StaticExportError::Prime)?
                    .generate()
                    .map_err(StaticExportError::Prime)?,
            )),
        };
        if package.field_id() != self.field_id() {
            return Err(StaticExportError::IdentityMismatch {
                dynamic: self.field_id(),
                generated: package.field_id(),
            });
        }
        Ok(package)
    }
}
