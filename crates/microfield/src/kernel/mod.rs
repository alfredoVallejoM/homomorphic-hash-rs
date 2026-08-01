//! Neutral batch-kernel ABI shared by engines and execution strategies.

mod catalog;
mod metadata;

pub(crate) use catalog::KernelSet;
pub use catalog::{BuiltinField, KernelCatalog};
pub use metadata::{BackendId, KernelMetadata, ScheduleKind};

#[cfg(feature = "portable")]
pub(crate) use catalog::sealed;
