//! Portable binary finite fields built from zero-cost abstractions.
//!
//! The current milestone publishes the stable capability contracts, field
//! identifiers, the base field [`F2`] and three maintained portable extension
//! fields.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[doc(hidden)]
pub mod __private;

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
    pub use crate::spec::{
        BinaryFieldFactory, BinaryFieldFactoryBuilder, BinaryFieldFactoryError,
        GeneratedFieldPackage,
    };
}

#[cfg(feature = "portable")]
pub use engine::{Engine, EngineBuildError, EngineBuilder, ExecutionPolicy};
pub use error::{BatchError, DecodeError};
pub use field::{
    BinaryPolynomialField, CanonicalEncoding, ExtensionField, F2, Field, Invert, Pow, Square,
    StaticField, StaticFieldSpec,
};
pub use id::{ArtifactBundleDigest, ArtifactId, FieldId};
pub use kernel::{BackendId, KernelMetadata, ScheduleKind};

#[doc(hidden)]
#[cfg(feature = "portable")]
pub use kernel::{BuiltinField, KernelCatalog};

#[cfg(feature = "builtin-fields")]
pub use generated::{Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1};
