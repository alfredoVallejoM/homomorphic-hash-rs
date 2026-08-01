//! Neutral batch-kernel ABI shared by engines and execution strategies.

mod catalog;
mod metadata;

pub(crate) use catalog::KernelSet;
#[cfg(feature = "portable")]
pub use catalog::{BuiltinField, KernelCatalog};
pub use metadata::{BackendId, KernelMetadata, ScheduleKind};

#[cfg(all(feature = "portable", feature = "builtin-fields"))]
pub(crate) use catalog::sealed;
