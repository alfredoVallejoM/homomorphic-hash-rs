//! Portable binary finite fields built from zero-cost abstractions.
//!
//! The current milestone publishes the stable capability contracts, field
//! identifiers and the base field [`F2`]. Larger built-in fields remain
//! private until their generated artifacts and independent vectors exist.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod backend;
mod binary;
mod engine;
mod generated;
mod kernel;

pub mod error;
pub mod field;
pub mod id;

#[cfg(feature = "generator")]
pub mod spec;

pub use error::{BatchError, DecodeError};
pub use field::{
    BinaryPolynomialField, CanonicalEncoding, ExtensionField, F2, Field, Invert, Pow, Square,
    StaticField, StaticFieldSpec,
};
pub use id::{ArtifactBundleDigest, ArtifactId, FieldId};
