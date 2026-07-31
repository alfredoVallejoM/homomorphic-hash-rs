//! Algebraic capability contracts and representation-independent metadata.

mod base;
mod encoding;
mod metadata;
mod pow;
mod traits;

pub use base::F2;
pub use metadata::StaticFieldSpec;
pub use traits::{
    BinaryPolynomialField, CanonicalEncoding, ExtensionField, Field, Invert, Pow, Square,
    StaticField,
};
