//! Portable binary finite fields built from zero-cost abstractions.
//!
//! The current milestone publishes the stable capability contracts, field
//! identifiers, the base field [`F2`] and three maintained portable extension
//! fields.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod backend;
#[cfg(feature = "builtin-fields")]
mod binary;
mod engine;
mod generated;
mod kernel;

pub mod error;
pub mod field;
pub mod id;

#[cfg(feature = "generator")]
pub mod spec;

pub use engine::{Engine, EngineBuildError, EngineBuilder, ExecutionPolicy};
pub use error::{BatchError, DecodeError};
pub use field::{
    BinaryPolynomialField, CanonicalEncoding, ExtensionField, F2, Field, Invert, Pow, Square,
    StaticField, StaticFieldSpec,
};
pub use id::{ArtifactBundleDigest, ArtifactId, FieldId};
pub use kernel::{BackendId, KernelMetadata, ScheduleKind};

#[doc(hidden)]
pub use kernel::{BuiltinField, KernelCatalog};

#[cfg(feature = "builtin-fields")]
pub use generated::{Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1};
