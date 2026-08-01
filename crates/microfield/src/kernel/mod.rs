//! Neutral batch-kernel ABI shared by engines and execution strategies.

mod catalog;
mod metadata;

#[cfg(feature = "portable")]
pub use catalog::BuiltinField;
pub use catalog::KernelCatalog;
pub(crate) use catalog::KernelSet;
pub use metadata::{BackendId, KernelMetadata, ScheduleKind};

#[cfg(all(feature = "portable", feature = "builtin-fields"))]
pub(crate) use catalog::sealed;
