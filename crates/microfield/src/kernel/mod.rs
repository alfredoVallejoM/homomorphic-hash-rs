//! Neutral batch-kernel ABI shared by engines and execution strategies.

#[cfg(all(
    feature = "portable",
    any(feature = "builtin-fields", target_arch = "aarch64", test)
))]
mod calibration;
mod catalog;
mod metadata;

#[cfg(all(
    feature = "portable",
    feature = "builtin-fields",
    target_arch = "x86_64"
))]
pub(crate) use calibration::{SelectionCalibration, X86_PCLMUL, X86_VPCLMUL_128, X86_VPCLMUL_256};
#[cfg(feature = "portable")]
pub use catalog::BuiltinField;
pub use catalog::KernelCatalog;
pub(crate) use catalog::KernelSet;
#[cfg(feature = "prime-fields")]
pub use metadata::PrimeKernelMetadata;
pub use metadata::{BackendId, KernelMetadata, ScheduleKind};

#[cfg(all(
    feature = "portable",
    any(feature = "builtin-fields", feature = "prime-fields")
))]
pub(crate) use catalog::sealed;
