//! Non-cryptographic homomorphic signatures for algebraic structure.
//!
//! These types preserve explicit composition laws and recover cheap metadata.
//! Equality is only equality of a finite-field evaluation under one identified
//! configuration; it is never a collision-free equality proof.
//!
//! ```
//! use homomorphic_hash_rs::{AdditiveSignature, BinaryPolynomialEncoder};
//! use microfield::Gf2_256HhV1;
//!
//! # fn main() -> Result<(), homomorphic_hash_rs::SignatureError> {
//! let encoder = BinaryPolynomialEncoder::new(0x4558_414d_504c_4501);
//! let mut left = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
//! let mut right = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
//! left.absorb(b"vertex-a")?;
//! right.absorb(b"vertex-b")?;
//!
//! let combined = left.combine(&right)?;
//! assert_eq!(combined.term_count(), 2);
//! # Ok(())
//! # }
//! ```

mod additive;
mod bidirectional_sequence;
#[cfg(feature = "dynamic-fields")]
mod dynamic;
#[cfg(feature = "dynamic-fields")]
mod dynamic_bidirectional_sequence;
#[cfg(feature = "dynamic-fields")]
mod dynamic_multi_evaluation_multiset;
mod encoder;
mod error;
mod id;
mod multi_evaluation_multiset;
mod multiset;
mod residual;
mod sequence;
mod wire;

pub use additive::AdditiveSignature;
pub use bidirectional_sequence::BidirectionalSequenceSignature;
#[cfg(feature = "dynamic-fields")]
pub use dynamic::{
    DynamicAdditiveSignature, DynamicAlgebraicResidual, DynamicMultisetSignature,
    DynamicSequenceSignature,
};
#[cfg(feature = "dynamic-fields")]
pub use dynamic_bidirectional_sequence::DynamicBidirectionalSequenceSignature;
#[cfg(feature = "dynamic-fields")]
pub use dynamic_multi_evaluation_multiset::DynamicMultiEvaluationMultisetSignature;
#[cfg(feature = "dynamic-fields")]
pub use encoder::DynamicStructuralEncoder;
pub use encoder::{
    BinaryPolynomialEncoder, CanonicalElementEncoder, LegacyAffineEncoderV1, LegacyLinearEncoderV1,
    PrimeIntegerEncoder, StructuralEncoder,
};
pub use error::SignatureError;
pub use id::{EncoderId, SignatureContext, SignatureId, SignatureLaw};
pub use multi_evaluation_multiset::MultiEvaluationMultisetSignature;
pub use multiset::{MultisetSignature, TrackedMultiset};
pub use residual::AlgebraicResidual;
pub use sequence::{SequenceSignature, TrackedSequence};
