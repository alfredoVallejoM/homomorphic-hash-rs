//! Validated generated field types.
//!
//! Fields of equal cardinality remain nominally distinct:
//!
//! ```compile_fail
//! use microfield::{Field, Gf2_256AltV1, Gf2_256HhV1};
//! let _ = Gf2_256HhV1::ONE + Gf2_256AltV1::ONE;
//! ```
//!
//! Internal limbs cannot be constructed by consumers:
//!
//! ```compile_fail
//! use microfield::Gf2_128V1;
//! let _ = Gf2_128V1([0; 2]);
//! ```

#[cfg(feature = "builtin-fields")]
mod binary_field;

#[cfg(feature = "builtin-fields")]
mod gf2_128_v1;

#[cfg(feature = "builtin-fields")]
mod gf2_256_alt_v1;

#[cfg(feature = "builtin-fields")]
mod gf2_256_hh_v1;

#[cfg(feature = "builtin-fields")]
pub use {gf2_128_v1::Gf2_128V1, gf2_256_alt_v1::Gf2_256AltV1, gf2_256_hh_v1::Gf2_256HhV1};
