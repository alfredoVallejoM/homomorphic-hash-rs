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
mod assurance;
mod bidirectional_sequence;
mod builder;
mod database;
mod delta;
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
mod dynamic;
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
mod dynamic_bidirectional_sequence;
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
mod dynamic_multi_evaluation_multiset;
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
mod dynamic_multi_evaluation_sequence;
mod encoder;
mod error;
mod id;
mod multi_evaluation_multiset;
mod multi_evaluation_sequence;
mod multiset;
mod profile;
mod reconciliation;
mod residual;
mod sequence;
mod snapshot;
mod summary_tree;
mod wire;

pub use additive::AdditiveSignature;
pub use assurance::SignatureAssurance;
pub use bidirectional_sequence::BidirectionalSequenceSignature;
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
pub use builder::DynamicSignatureBuilder;
pub use builder::SignatureBuilder;
pub use database::{
    DatabaseApplyReport, DatabaseApplyStatus, DatabaseColumn, DatabaseColumnType, DatabaseError,
    DatabaseReplayReport, DatabaseRow, DatabaseRowKey, DatabaseSchema, DatabaseSchemaId,
    DatabaseSummary, DatabaseTransactionLimits, DatabaseTransactionLog, DatabaseValue,
    PartitionedDatabase, RowMutation, TransactionDelta, TransactionId,
};
pub use delta::{
    AdditiveDelta, ApplicationNamespace, DeltaApplyReport, DeltaApplyStatus, DeltaEnvelope,
    DeltaError, DeltaId, DeltaJournal, DeltaJournalLimits, DeltaReplayReport, DeltaVerification,
    MultisetDelta, RevisionedSignature, RevisionedState, SequenceAppend, SequenceTrim,
    SignatureDelta,
};
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
pub use dynamic::{
    DynamicAdditiveSignature, DynamicAlgebraicResidual, DynamicMultisetSignature,
    DynamicSequenceSignature,
};
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
pub use dynamic_bidirectional_sequence::DynamicBidirectionalSequenceSignature;
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
pub use dynamic_multi_evaluation_multiset::DynamicMultiEvaluationMultisetSignature;
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
pub use dynamic_multi_evaluation_sequence::DynamicMultiEvaluationSequenceSignature;
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
pub use encoder::DynamicStructuralEncoder;
pub use encoder::{
    BinaryPolynomialEncoder, CanonicalElementEncoder, DomainSeparatedHashToFieldEncoder,
    LegacyAffineEncoderV1, LegacyLinearEncoderV1, PrimeIntegerEncoder, StructuralEncoder,
    StructuralLaneEncoder,
};
pub use error::SignatureError;
pub use id::{EncoderId, SignatureContext, SignatureId, SignatureLaw};
pub use multi_evaluation_multiset::MultiEvaluationMultisetSignature;
pub use multi_evaluation_sequence::MultiEvaluationSequenceSignature;
pub use multiset::{MultisetSignature, TrackedMultiset};
pub use profile::{
    CompactSignature, SignatureEvaluationProfile, SignatureFieldBinding, SignatureFieldProfile,
    SignatureProfile,
};
pub use reconciliation::{
    BoundedSetReconciler, ReconciliationError, ReconciliationLimits, ReconciliationProfileId,
    RecoveredSetDifference, SetReconciliationSketch,
};
pub use residual::AlgebraicResidual;
pub use sequence::{SequenceSignature, TrackedSequence};
pub use snapshot::TrackedSnapshotLimits;
pub use summary_tree::{
    FileChunkProfile, FileChunkProfileId, HomomorphicSummaryRoot, HomomorphicSummaryTree,
    SummaryEditPath, SummaryEditReport, SummaryTreeError, SummaryTreeLimits,
};
