//! Safe, reusable algorithms built from field and engine primitives.

mod horner;
mod id;
mod invert_batch;
mod mask;
mod powers;
mod scan;
mod workspace;

pub use horner::{CoefficientLayout, HornerError, ManyPointsHornerPlan, ManyPolynomialsHornerPlan};
pub use id::{
    AlgorithmFamily, AlgorithmId, AllocationBehavior, BatchPlan, OperationKind, WorkspaceLayout,
};
pub use invert_batch::{BatchInvertError, BatchInvertPlan, BatchInvertRequirements};
#[cfg(feature = "alloc")]
pub use mask::BitMask;
pub use mask::{BitMaskError, BitMaskViewMut, required_mask_words};
#[cfg(feature = "alloc")]
pub use powers::FixedBasePowers;
pub use powers::{PowerTableError, fill_fixed_base_powers};
pub use scan::{ProductScanPlan, ScanDirection, ScanError, ScanMode};
#[cfg(feature = "alloc")]
pub use workspace::OwnedBatchInvertWorkspace;
pub use workspace::{BatchInvertWorkspace, WorkspaceError};
