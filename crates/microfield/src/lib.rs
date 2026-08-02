//! Portable binary finite fields built from zero-cost abstractions.
//!
//! The crate publishes stable capability contracts, field identities, the
//! base field [`F2`], maintained portable extensions, statically selected
//! batch kernels and allocation-free derived algorithms.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[doc(hidden)]
pub mod __private;

#[cfg(feature = "portable")]
mod algorithms;
mod backend;
#[cfg(feature = "builtin-fields")]
mod binary;
#[cfg(feature = "portable")]
mod engine;
mod generated;
mod kernel;

const fn portable_kernel_set<F>() -> kernel::KernelSet<F>
where
    F: Field + Square,
{
    backend::portable::kernel_set::<F>()
}

pub mod error;
pub mod field;
pub mod id;

#[cfg(feature = "generator")]
pub mod spec;

/// Stable build-time entry points for generating certified binary field types.
#[cfg(feature = "generator")]
pub mod generator {
    pub use crate::spec::model::{
        IsaProfileBackend, IsaProfileClass, IsaProfileSchedule, IsaProfileSelection,
        PortableDegreeClass, PortableOptimizationPlan, PortableReductionStrategy,
        VerifiedIsaProfile,
    };
    pub use crate::spec::{
        BinaryFieldFactory, BinaryFieldFactoryBuilder, BinaryFieldFactoryError,
        GeneratedFieldPackage,
    };
}

#[cfg(feature = "portable")]
pub use algorithms::{
    AlgorithmFamily, AlgorithmId, AllocationBehavior, BatchInvertError, BatchInvertPlan,
    BatchInvertRequirements, BatchInvertWorkspace, BatchPlan, BitMaskError, BitMaskViewMut,
    CoefficientLayout, HornerError, ManyPointsHornerPlan, ManyPolynomialsHornerPlan, OperationKind,
    PowerTableError, ProductScanPlan, ScanDirection, ScanError, ScanMode, WorkspaceError,
    WorkspaceLayout, fill_fixed_base_powers, required_mask_words,
};
#[cfg(all(feature = "portable", feature = "alloc"))]
pub use algorithms::{BitMask, FixedBasePowers, OwnedBatchInvertWorkspace};
#[cfg(all(feature = "portable", feature = "alloc"))]
pub use engine::PackedBatch;
#[cfg(feature = "portable")]
pub use engine::{
    Architecture, CpuCapabilities, Engine, EngineBuildError, EngineBuilder, ExecutionPolicy,
    PackError, PackedBatchView, PackedBatchViewMut, PackedLayout, PackingPlan, pack_into_storage,
    required_packed_bytes,
};
pub use error::{BatchError, DecodeError};
pub use field::{
    BinaryPolynomialField, CanonicalEncoding, ExtensionField, F2, Field, Invert, Pow, Square,
    StaticField, StaticFieldSpec,
};
pub use id::{ArtifactBundleDigest, ArtifactId, FieldId};
pub use kernel::{BackendId, KernelMetadata, ScheduleKind};

#[doc(hidden)]
pub use kernel::KernelCatalog;

#[doc(hidden)]
#[cfg(feature = "portable")]
pub use kernel::BuiltinField;

#[cfg(feature = "builtin-fields")]
pub use generated::{Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1};
