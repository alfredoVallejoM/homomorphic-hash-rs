//! Versioned, law-specific deltas for maintained structural signatures.
//!
//! Delta application checks algebraic consistency. It does not prove that a
//! removed value existed in an external source of truth.

use core::fmt;
use std::collections::BTreeSet;

use microfield::{CanonicalEncoding, Field, Invert, Pow, StaticField};
use sha2::{Digest as _, Sha256};

use super::{
    AdditiveSignature, MultisetSignature, SequenceSignature, SignatureContext, SignatureError,
    StructuralEncoder,
};

const DELTA_MAGIC: &[u8; 4] = b"MFDE";
const DELTA_SCHEMA: u16 = 1;
const DELTA_HEADER_BYTES: usize = 161;
const JOURNAL_MAGIC: &[u8; 4] = b"MFDJ";
const JOURNAL_SCHEMA: u16 = 1;
const JOURNAL_HEADER_BYTES: usize = 14;
const ADDITIVE_KIND: u8 = 1;
const MULTISET_KIND: u8 = 2;
const SEQUENCE_APPEND_KIND: u8 = 3;
const SEQUENCE_TRIM_KIND: u8 = 4;

/// Stable application boundary used to prevent cross-dataset replay.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ApplicationNamespace([u8; 32]);

impl ApplicationNamespace {
    /// Derives a namespace from an application-controlled stable descriptor.
    ///
    /// This is an identity hash, not a cryptographic authentication token.
    #[must_use]
    pub fn derive(descriptor: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"microfield-application-namespace-v1\0");
        hasher.update((descriptor.len() as u64).to_le_bytes());
        hasher.update(descriptor);
        Self(hasher.finalize().into())
    }

    /// Borrows the canonical bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Content-derived identity of one complete delta envelope and payload.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DeltaId([u8; 32]);

impl DeltaId {
    /// Borrows the canonical identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

macro_rules! impl_hex_format {
    ($type:ty, $label:literal) => {
        impl fmt::Display for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.as_bytes() {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }

        impl fmt::Debug for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str($label)?;
                formatter.write_str("(")?;
                fmt::Display::fmt(self, formatter)?;
                formatter.write_str(")")
            }
        }
    };
}

impl_hex_format!(ApplicationNamespace, "ApplicationNamespace");
impl_hex_format!(DeltaId, "DeltaId");

/// Shared immutable metadata for every maintained delta.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeltaEnvelope {
    namespace: ApplicationNamespace,
    context: SignatureContext,
    source_revision: u64,
    target_revision: u64,
    operation_count: u64,
    delta_id: DeltaId,
}

impl DeltaEnvelope {
    /// Schema version of the `MFDE` envelope.
    #[must_use]
    pub const fn schema(&self) -> u16 {
        DELTA_SCHEMA
    }

    /// Application/dataset identity.
    #[must_use]
    pub const fn namespace(&self) -> ApplicationNamespace {
        self.namespace
    }

    /// Full field, encoder and law identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Required state revision.
    #[must_use]
    pub const fn source_revision(&self) -> u64 {
        self.source_revision
    }

    /// Revision published after successful application.
    #[must_use]
    pub const fn target_revision(&self) -> u64 {
        self.target_revision
    }

    /// Number of logical additions/removals represented by the payload.
    #[must_use]
    pub const fn operation_count(&self) -> u64 {
        self.operation_count
    }

    /// Content-derived identity used for idempotent replay.
    #[must_use]
    pub const fn delta_id(&self) -> DeltaId {
        self.delta_id
    }
}

/// Honest classification of what was checked while applying a delta.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DeltaVerification {
    /// Only the typed field equation, counters, identity and revision matched.
    AlgebraicConsistency,
    /// An authoritative source separately validated the removed data.
    SourceValidated,
    /// Exact retained data was rebuilt and compared.
    ExactRebuild,
}

/// Whether a transaction changed the state or was recognized as a replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeltaApplyStatus {
    /// Candidate validation succeeded and the new revision was published.
    Applied,
    /// The exact `DeltaId` was already committed.
    AlreadyApplied,
}

/// Report returned by one atomic application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeltaApplyReport {
    status: DeltaApplyStatus,
    revision: u64,
    verification: DeltaVerification,
}

impl DeltaApplyReport {
    /// Application outcome.
    #[must_use]
    pub const fn status(self) -> DeltaApplyStatus {
        self.status
    }

    /// Revision after the call.
    #[must_use]
    pub const fn revision(self) -> u64 {
        self.revision
    }

    /// Strength of the performed verification.
    #[must_use]
    pub const fn verification(self) -> DeltaVerification {
        self.verification
    }
}

/// Typed failure produced before a revisioned state is committed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DeltaError {
    /// A structural component or encoding operation failed.
    Signature(SignatureError),
    /// A zero-operation transaction attempted to advance a revision.
    EmptyDelta,
    /// The source revision cannot advance by exactly one.
    RevisionOverflow,
    /// Optimistic concurrency control rejected the transaction.
    RevisionMismatch {
        /// Revision required by the delta.
        expected: u64,
        /// Current published revision.
        actual: u64,
    },
    /// The delta belongs to a different application dataset.
    NamespaceMismatch,
    /// The delta belongs to a different signature field/encoder/law.
    ContextMismatch,
    /// A journal contains duplicate or non-contiguous transactions.
    InvalidJournal(&'static str),
    /// A delta or journal wire is malformed.
    InvalidWire(&'static str),
    /// A defensive journal ceiling was exceeded.
    JournalLimitExceeded(&'static str),
    /// Storage allocation failed before publication.
    AllocationFailed,
}

impl fmt::Display for DeltaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Signature(error) => error.fmt(formatter),
            Self::EmptyDelta => formatter.write_str("a delta must contain at least one operation"),
            Self::RevisionOverflow => formatter.write_str("delta revision overflow"),
            Self::RevisionMismatch { expected, actual } => write!(
                formatter,
                "delta expects revision {expected}, current revision is {actual}"
            ),
            Self::NamespaceMismatch => formatter.write_str("delta application namespace mismatch"),
            Self::ContextMismatch => formatter.write_str("delta signature context mismatch"),
            Self::InvalidJournal(reason) => write!(formatter, "invalid delta journal: {reason}"),
            Self::InvalidWire(reason) => write!(formatter, "invalid delta wire: {reason}"),
            Self::JournalLimitExceeded(limit) => {
                write!(formatter, "delta journal exceeds {limit} limit")
            }
            Self::AllocationFailed => formatter.write_str("delta allocation failed"),
        }
    }
}

impl std::error::Error for DeltaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Signature(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SignatureError> for DeltaError {
    fn from(error: SignatureError) -> Self {
        Self::Signature(error)
    }
}

mod private {
    pub trait StateSealed {}
    pub trait DeltaSealed {}
}

/// Sealed compact signature state accepted by [`RevisionedSignature`].
#[doc(hidden)]
pub trait RevisionedState: Clone + private::StateSealed {
    /// Returns the complete compatibility identity.
    fn revisioned_context(&self) -> SignatureContext;
}

/// A typed operation that can produce a candidate state without mutation.
pub trait SignatureDelta: private::DeltaSealed {
    /// Compact state transformed by this law-specific delta.
    type State: RevisionedState;

    /// Shared transaction metadata.
    fn envelope(&self) -> &DeltaEnvelope;

    /// Computes and validates a candidate without publishing it.
    fn candidate(&self, current: &Self::State) -> Result<Self::State, DeltaError>;

    /// Stable `MFDE` representation.
    fn to_canonical_bytes(&self) -> Vec<u8>;
}

/// One compact signature plus revision and committed delta identities.
#[derive(Clone, Debug)]
pub struct RevisionedSignature<S>
where
    S: RevisionedState,
{
    namespace: ApplicationNamespace,
    state: S,
    revision: u64,
    applied: BTreeSet<DeltaId>,
}

impl<S> RevisionedSignature<S>
where
    S: RevisionedState,
{
    /// Starts a revision-zero stream for one application namespace.
    #[must_use]
    pub fn new(namespace: ApplicationNamespace, state: S) -> Self {
        Self {
            namespace,
            state,
            revision: 0,
            applied: BTreeSet::new(),
        }
    }

    /// Current compact state.
    #[must_use]
    pub const fn state(&self) -> &S {
        &self.state
    }

    /// Current committed revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Dataset/application identity.
    #[must_use]
    pub const fn namespace(&self) -> ApplicationNamespace {
        self.namespace
    }

    /// Applies one delta using preflight/candidate/commit ordering.
    ///
    /// Every fallible algebraic operation completes on a temporary candidate.
    /// On error, state, revision and replay set remain byte-for-byte unchanged.
    pub fn apply<D>(&mut self, delta: &D) -> Result<DeltaApplyReport, DeltaError>
    where
        D: SignatureDelta<State = S>,
    {
        let envelope = delta.envelope();
        if envelope.namespace != self.namespace {
            return Err(DeltaError::NamespaceMismatch);
        }
        if envelope.context != self.state.revisioned_context() {
            return Err(DeltaError::ContextMismatch);
        }
        if self.applied.contains(&envelope.delta_id) {
            return Ok(DeltaApplyReport {
                status: DeltaApplyStatus::AlreadyApplied,
                revision: self.revision,
                verification: DeltaVerification::AlgebraicConsistency,
            });
        }
        if envelope.source_revision != self.revision {
            return Err(DeltaError::RevisionMismatch {
                expected: envelope.source_revision,
                actual: self.revision,
            });
        }
        let candidate = delta.candidate(&self.state)?;
        self.applied.insert(envelope.delta_id);
        self.state = candidate;
        self.revision = envelope.target_revision;
        Ok(DeltaApplyReport {
            status: DeltaApplyStatus::Applied,
            revision: self.revision,
            verification: DeltaVerification::AlgebraicConsistency,
        })
    }
}

impl<F, E> private::StateSealed for AdditiveSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, E> RevisionedState for AdditiveSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField,
    E: StructuralEncoder<F>,
{
    fn revisioned_context(&self) -> SignatureContext {
        self.context()
    }
}

impl<F, E> private::StateSealed for MultisetSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, E> RevisionedState for MultisetSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Invert,
    E: StructuralEncoder<F>,
{
    fn revisioned_context(&self) -> SignatureContext {
        self.context()
    }
}

impl<F, E> private::StateSealed for SequenceSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, E> RevisionedState for SequenceSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
    E: StructuralEncoder<F>,
{
    fn revisioned_context(&self) -> SignatureContext {
        self.context()
    }
}

/// Addition/removal transition under the additive law.
#[derive(Clone, Debug)]
pub struct AdditiveDelta<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    envelope: DeltaEnvelope,
    removed: AdditiveSignature<F, E>,
    added: AdditiveSignature<F, E>,
}

impl<F, E> AdditiveDelta<F, E>
where
    F: Field + CanonicalEncoding + StaticField,
    E: StructuralEncoder<F>,
{
    /// Builds one revision transition from compact removed/added partitions.
    pub fn new(
        namespace: ApplicationNamespace,
        source_revision: u64,
        removed: AdditiveSignature<F, E>,
        added: AdditiveSignature<F, E>,
    ) -> Result<Self, DeltaError> {
        ensure_same_context(removed.context(), added.context())?;
        let operation_count = removed
            .term_count()
            .checked_add(added.term_count())
            .ok_or(SignatureError::CounterOverflow)?;
        let payload = encode_pair(&removed.to_canonical_bytes(), &added.to_canonical_bytes());
        let envelope = make_envelope(
            ADDITIVE_KIND,
            namespace,
            removed.context(),
            source_revision,
            operation_count,
            &payload,
        )?;
        Ok(Self {
            envelope,
            removed,
            added,
        })
    }

    /// Parses and fully revalidates an `MFDE` additive delta.
    pub fn from_canonical_bytes(encoder: E, bytes: &[u8]) -> Result<Self, DeltaError> {
        let context = AdditiveSignature::<F, E>::new(encoder.clone()).context();
        let decoded = decode_envelope(bytes, ADDITIVE_KIND, context)?;
        let (removed, added) = decode_pair(decoded.payload)?;
        let candidate = Self::new(
            decoded.namespace,
            decoded.source_revision,
            AdditiveSignature::from_canonical_bytes(encoder.clone(), removed)?,
            AdditiveSignature::from_canonical_bytes(encoder, added)?,
        )?;
        verify_decoded(&candidate.envelope, &decoded)?;
        Ok(candidate)
    }
}

impl<F, E> private::DeltaSealed for AdditiveDelta<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, E> SignatureDelta for AdditiveDelta<F, E>
where
    F: Field + CanonicalEncoding + StaticField,
    E: StructuralEncoder<F>,
{
    type State = AdditiveSignature<F, E>;

    fn envelope(&self) -> &DeltaEnvelope {
        &self.envelope
    }

    fn candidate(
        &self,
        current: &AdditiveSignature<F, E>,
    ) -> Result<AdditiveSignature<F, E>, DeltaError> {
        Ok(current.apply_delta_parts(&self.removed, &self.added)?)
    }

    fn to_canonical_bytes(&self) -> Vec<u8> {
        encode_delta(
            ADDITIVE_KIND,
            &self.envelope,
            &encode_pair(
                &self.removed.to_canonical_bytes(),
                &self.added.to_canonical_bytes(),
            ),
        )
    }
}

/// Addition/removal transition under the commutative product law.
#[derive(Clone, Debug)]
pub struct MultisetDelta<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    envelope: DeltaEnvelope,
    removed: MultisetSignature<F, E>,
    added: MultisetSignature<F, E>,
}

impl<F, E> MultisetDelta<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Invert,
    E: StructuralEncoder<F>,
{
    /// Builds one compact multiset transition.
    pub fn new(
        namespace: ApplicationNamespace,
        source_revision: u64,
        removed: MultisetSignature<F, E>,
        added: MultisetSignature<F, E>,
    ) -> Result<Self, DeltaError> {
        ensure_same_context(removed.context(), added.context())?;
        let operation_count = removed
            .cardinality()
            .checked_add(added.cardinality())
            .ok_or(SignatureError::CounterOverflow)?;
        let payload = encode_pair(&removed.to_canonical_bytes(), &added.to_canonical_bytes());
        let envelope = make_envelope(
            MULTISET_KIND,
            namespace,
            removed.context(),
            source_revision,
            operation_count,
            &payload,
        )?;
        Ok(Self {
            envelope,
            removed,
            added,
        })
    }

    /// Parses and revalidates an `MFDE` multiset delta.
    pub fn from_canonical_bytes(encoder: E, offset: F, bytes: &[u8]) -> Result<Self, DeltaError> {
        let context = MultisetSignature::<F, E>::new(encoder.clone(), offset).context();
        let decoded = decode_envelope(bytes, MULTISET_KIND, context)?;
        let (removed, added) = decode_pair(decoded.payload)?;
        let candidate = Self::new(
            decoded.namespace,
            decoded.source_revision,
            MultisetSignature::from_canonical_bytes(encoder.clone(), offset, removed)?,
            MultisetSignature::from_canonical_bytes(encoder, offset, added)?,
        )?;
        verify_decoded(&candidate.envelope, &decoded)?;
        Ok(candidate)
    }
}

impl<F, E> private::DeltaSealed for MultisetDelta<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, E> SignatureDelta for MultisetDelta<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Invert,
    E: StructuralEncoder<F>,
{
    type State = MultisetSignature<F, E>;

    fn envelope(&self) -> &DeltaEnvelope {
        &self.envelope
    }

    fn candidate(
        &self,
        current: &MultisetSignature<F, E>,
    ) -> Result<MultisetSignature<F, E>, DeltaError> {
        Ok(current.apply_delta_parts(&self.removed, &self.added)?)
    }

    fn to_canonical_bytes(&self) -> Vec<u8> {
        encode_delta(
            MULTISET_KIND,
            &self.envelope,
            &encode_pair(
                &self.removed.to_canonical_bytes(),
                &self.added.to_canonical_bytes(),
            ),
        )
    }
}

/// Ordered append represented by one independently accumulated suffix.
#[derive(Clone, Debug)]
pub struct SequenceAppend<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    envelope: DeltaEnvelope,
    suffix: SequenceSignature<F, E>,
}

/// Ordered trim represented by an assumed exact suffix signature.
#[derive(Clone, Debug)]
pub struct SequenceTrim<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    envelope: DeltaEnvelope,
    suffix: SequenceSignature<F, E>,
}

macro_rules! impl_sequence_delta {
    ($name:ident, $kind:ident, $apply:ident, $docs:literal) => {
        impl<F, E> $name<F, E>
        where
            F: Field + CanonicalEncoding + StaticField + Pow + Invert,
            E: StructuralEncoder<F>,
        {
            #[doc = $docs]
            pub fn new(
                namespace: ApplicationNamespace,
                source_revision: u64,
                suffix: SequenceSignature<F, E>,
            ) -> Result<Self, DeltaError> {
                let payload = encode_single(&suffix.to_canonical_bytes());
                let envelope = make_envelope(
                    $kind,
                    namespace,
                    suffix.context(),
                    source_revision,
                    suffix.len(),
                    &payload,
                )?;
                Ok(Self { envelope, suffix })
            }

            /// Parses and revalidates an ordered `MFDE` delta.
            pub fn from_canonical_bytes(
                encoder: E,
                base: F,
                bytes: &[u8],
            ) -> Result<Self, DeltaError> {
                let context = SequenceSignature::<F, E>::new(encoder.clone(), base)?.context();
                let decoded = decode_envelope(bytes, $kind, context)?;
                let suffix = decode_single(decoded.payload)?;
                let candidate = Self::new(
                    decoded.namespace,
                    decoded.source_revision,
                    SequenceSignature::from_canonical_bytes(encoder, base, suffix)?,
                )?;
                verify_decoded(&candidate.envelope, &decoded)?;
                Ok(candidate)
            }
        }

        impl<F, E> private::DeltaSealed for $name<F, E>
        where
            F: Field,
            E: StructuralEncoder<F>,
        {
        }

        impl<F, E> SignatureDelta for $name<F, E>
        where
            F: Field + CanonicalEncoding + StaticField + Pow + Invert,
            E: StructuralEncoder<F>,
        {
            type State = SequenceSignature<F, E>;

            fn envelope(&self) -> &DeltaEnvelope {
                &self.envelope
            }

            fn candidate(
                &self,
                current: &SequenceSignature<F, E>,
            ) -> Result<SequenceSignature<F, E>, DeltaError> {
                Ok(current.$apply(&self.suffix)?)
            }

            fn to_canonical_bytes(&self) -> Vec<u8> {
                encode_delta(
                    $kind,
                    &self.envelope,
                    &encode_single(&self.suffix.to_canonical_bytes()),
                )
            }
        }
    };
}

impl_sequence_delta!(
    SequenceAppend,
    SEQUENCE_APPEND_KIND,
    concatenate,
    "Builds an append transaction from one non-empty suffix."
);
impl_sequence_delta!(
    SequenceTrim,
    SEQUENCE_TRIM_KIND,
    trim_assuming_suffix,
    "Builds an assumed-suffix trim transaction. Membership must be checked externally."
);

/// Defensive ceilings for decoding a persisted journal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeltaJournalLimits {
    /// Maximum transaction count.
    pub max_entries: usize,
    /// Maximum bytes occupied by one canonical delta.
    pub max_entry_bytes: usize,
    /// Maximum complete journal bytes.
    pub max_total_bytes: usize,
}

impl Default for DeltaJournalLimits {
    fn default() -> Self {
        Self {
            max_entries: 1_000_000,
            max_entry_bytes: 16 * 1024 * 1024,
            max_total_bytes: 64 * 1024 * 1024,
        }
    }
}

/// Ordered, contiguous delta journal with deterministic `MFDJ` persistence.
#[derive(Clone, Debug, Default)]
pub struct DeltaJournal<D> {
    entries: Vec<D>,
}

impl<D> DeltaJournal<D> {
    /// Creates an empty journal.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Number of retained transactions.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Reports whether the journal is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<D> DeltaJournal<D> {
    /// Appends only a unique, contiguous transaction from one context/namespace.
    pub fn append(&mut self, delta: D) -> Result<(), DeltaError>
    where
        D: SignatureDelta,
    {
        let envelope = *delta.envelope();
        if let Some(previous) = self.entries.last() {
            let prior = previous.envelope();
            if prior.namespace != envelope.namespace || prior.context != envelope.context {
                return Err(DeltaError::InvalidJournal("identity drift"));
            }
            if prior.target_revision != envelope.source_revision {
                return Err(DeltaError::InvalidJournal("revision gap or reorder"));
            }
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.envelope().delta_id == envelope.delta_id)
        {
            return Err(DeltaError::InvalidJournal("duplicate delta id"));
        }
        self.entries
            .try_reserve(1)
            .map_err(|_| DeltaError::AllocationFailed)?;
        self.entries.push(delta);
        Ok(())
    }

    /// Replays the complete journal as one transaction.
    ///
    /// Committed IDs make repeated replay a no-op. A failure in any entry
    /// discards every candidate change produced earlier in the same call.
    pub fn replay<S>(
        &self,
        state: &mut RevisionedSignature<S>,
    ) -> Result<DeltaReplayReport, DeltaError>
    where
        S: RevisionedState,
        D: SignatureDelta<State = S>,
    {
        let mut candidate = state.clone();
        let mut applied = 0_u64;
        let mut skipped = 0_u64;
        for delta in &self.entries {
            match candidate.apply(delta)?.status() {
                DeltaApplyStatus::Applied => applied += 1,
                DeltaApplyStatus::AlreadyApplied => skipped += 1,
            }
        }
        *state = candidate;
        Ok(DeltaReplayReport {
            applied,
            skipped,
            revision: state.revision,
        })
    }

    /// Serializes the journal with lengths framing every `MFDE` entry.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, DeltaError>
    where
        D: SignatureDelta,
    {
        let encoded: Vec<Vec<u8>> = self
            .entries
            .iter()
            .map(SignatureDelta::to_canonical_bytes)
            .collect();
        let total = encoded
            .iter()
            .try_fold(JOURNAL_HEADER_BYTES, |size, entry| {
                size.checked_add(8)?.checked_add(entry.len())
            });
        let total = total.ok_or(DeltaError::JournalLimitExceeded("total bytes"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(total)
            .map_err(|_| DeltaError::AllocationFailed)?;
        bytes.extend_from_slice(JOURNAL_MAGIC);
        bytes.extend_from_slice(&JOURNAL_SCHEMA.to_le_bytes());
        bytes.extend_from_slice(&(encoded.len() as u64).to_le_bytes());
        for entry in encoded {
            bytes.extend_from_slice(&(entry.len() as u64).to_le_bytes());
            bytes.extend_from_slice(&entry);
        }
        Ok(bytes)
    }

    /// Decodes entries through the law-specific canonical parser supplied by
    /// the caller, then revalidates the complete journal chain.
    pub fn from_canonical_bytes<P>(
        bytes: &[u8],
        limits: DeltaJournalLimits,
        mut parser: P,
    ) -> Result<Self, DeltaError>
    where
        D: SignatureDelta,
        P: FnMut(&[u8]) -> Result<D, DeltaError>,
    {
        if bytes.len() > limits.max_total_bytes {
            return Err(DeltaError::JournalLimitExceeded("total bytes"));
        }
        if bytes.len() < JOURNAL_HEADER_BYTES || &bytes[..4] != JOURNAL_MAGIC {
            return Err(DeltaError::InvalidWire("journal header"));
        }
        if u16::from_le_bytes([bytes[4], bytes[5]]) != JOURNAL_SCHEMA {
            return Err(DeltaError::InvalidWire("journal schema"));
        }
        let count = u64::from_le_bytes(bytes[6..14].try_into().expect("journal count range"));
        let count =
            usize::try_from(count).map_err(|_| DeltaError::JournalLimitExceeded("entries"))?;
        if count > limits.max_entries {
            return Err(DeltaError::JournalLimitExceeded("entries"));
        }
        let mut journal = Self::new();
        journal
            .entries
            .try_reserve_exact(count)
            .map_err(|_| DeltaError::AllocationFailed)?;
        let mut cursor = JOURNAL_HEADER_BYTES;
        for _ in 0..count {
            let length_end = cursor
                .checked_add(8)
                .filter(|end| *end <= bytes.len())
                .ok_or(DeltaError::InvalidWire("truncated journal entry length"))?;
            let length = u64::from_le_bytes(
                bytes[cursor..length_end]
                    .try_into()
                    .expect("entry length range"),
            );
            let length = usize::try_from(length)
                .map_err(|_| DeltaError::JournalLimitExceeded("entry bytes"))?;
            if length > limits.max_entry_bytes {
                return Err(DeltaError::JournalLimitExceeded("entry bytes"));
            }
            let end = length_end
                .checked_add(length)
                .filter(|end| *end <= bytes.len())
                .ok_or(DeltaError::InvalidWire("truncated journal entry"))?;
            journal.append(parser(&bytes[length_end..end])?)?;
            cursor = end;
        }
        if cursor != bytes.len() {
            return Err(DeltaError::InvalidWire("trailing journal bytes"));
        }
        Ok(journal)
    }
}

/// Aggregate replay outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeltaReplayReport {
    applied: u64,
    skipped: u64,
    revision: u64,
}

impl DeltaReplayReport {
    /// Newly committed entries.
    #[must_use]
    pub const fn applied(self) -> u64 {
        self.applied
    }

    /// Entries recognized by `DeltaId` as already committed.
    #[must_use]
    pub const fn skipped(self) -> u64 {
        self.skipped
    }

    /// Final state revision.
    #[must_use]
    pub const fn revision(self) -> u64 {
        self.revision
    }
}

struct DecodedEnvelope<'a> {
    namespace: ApplicationNamespace,
    source_revision: u64,
    target_revision: u64,
    operation_count: u64,
    delta_id: DeltaId,
    payload: &'a [u8],
}

fn ensure_same_context(left: SignatureContext, right: SignatureContext) -> Result<(), DeltaError> {
    if left == right {
        Ok(())
    } else {
        Err(DeltaError::ContextMismatch)
    }
}

fn make_envelope(
    kind: u8,
    namespace: ApplicationNamespace,
    context: SignatureContext,
    source_revision: u64,
    operation_count: u64,
    payload: &[u8],
) -> Result<DeltaEnvelope, DeltaError> {
    if operation_count == 0 {
        return Err(DeltaError::EmptyDelta);
    }
    let target_revision = source_revision
        .checked_add(1)
        .ok_or(DeltaError::RevisionOverflow)?;
    let mut envelope = DeltaEnvelope {
        namespace,
        context,
        source_revision,
        target_revision,
        operation_count,
        delta_id: DeltaId([0; 32]),
    };
    envelope.delta_id = derive_delta_id(&encode_delta(kind, &envelope, payload));
    Ok(envelope)
}

fn encode_delta(kind: u8, envelope: &DeltaEnvelope, payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(DELTA_HEADER_BYTES + payload.len());
    bytes.extend_from_slice(DELTA_MAGIC);
    bytes.extend_from_slice(&DELTA_SCHEMA.to_le_bytes());
    bytes.push(kind);
    bytes.push(0);
    bytes.extend_from_slice(envelope.namespace.as_bytes());
    bytes.extend_from_slice(&envelope.source_revision.to_le_bytes());
    bytes.extend_from_slice(&envelope.target_revision.to_le_bytes());
    bytes.extend_from_slice(&envelope.operation_count.to_le_bytes());
    bytes.extend_from_slice(envelope.context.field_id().as_bytes());
    bytes.extend_from_slice(envelope.context.encoder_id().as_bytes());
    bytes.extend_from_slice(envelope.context.signature_id().as_bytes());
    bytes.push(envelope.context.law() as u8);
    bytes.extend_from_slice(payload);
    bytes
}

fn decode_envelope<'a>(
    bytes: &'a [u8],
    expected_kind: u8,
    expected_context: SignatureContext,
) -> Result<DecodedEnvelope<'a>, DeltaError> {
    if bytes.len() < DELTA_HEADER_BYTES || &bytes[..4] != DELTA_MAGIC {
        return Err(DeltaError::InvalidWire("delta header"));
    }
    if u16::from_le_bytes([bytes[4], bytes[5]]) != DELTA_SCHEMA {
        return Err(DeltaError::InvalidWire("delta schema"));
    }
    if bytes[6] != expected_kind || bytes[7] != 0 {
        return Err(DeltaError::InvalidWire("delta kind or reserved byte"));
    }
    if &bytes[64..96] != expected_context.field_id().as_bytes()
        || &bytes[96..128] != expected_context.encoder_id().as_bytes()
        || &bytes[128..160] != expected_context.signature_id().as_bytes()
        || bytes[160] != expected_context.law() as u8
    {
        return Err(DeltaError::ContextMismatch);
    }
    let source_revision = u64::from_le_bytes(bytes[40..48].try_into().expect("source range"));
    let target_revision = u64::from_le_bytes(bytes[48..56].try_into().expect("target range"));
    if source_revision.checked_add(1) != Some(target_revision) {
        return Err(DeltaError::InvalidWire("non-contiguous revisions"));
    }
    let operation_count = u64::from_le_bytes(bytes[56..64].try_into().expect("operation range"));
    if operation_count == 0 {
        return Err(DeltaError::EmptyDelta);
    }
    Ok(DecodedEnvelope {
        namespace: ApplicationNamespace(bytes[8..40].try_into().expect("namespace range")),
        source_revision,
        target_revision,
        operation_count,
        delta_id: derive_delta_id(bytes),
        payload: &bytes[DELTA_HEADER_BYTES..],
    })
}

fn verify_decoded(
    envelope: &DeltaEnvelope,
    decoded: &DecodedEnvelope<'_>,
) -> Result<(), DeltaError> {
    if envelope.target_revision != decoded.target_revision
        || envelope.operation_count != decoded.operation_count
        || envelope.delta_id != decoded.delta_id
    {
        return Err(DeltaError::InvalidWire("delta envelope/payload mismatch"));
    }
    Ok(())
}

fn derive_delta_id(bytes: &[u8]) -> DeltaId {
    let mut hasher = Sha256::new();
    hasher.update(b"microfield-signature-delta-v1\0");
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    DeltaId(hasher.finalize().into())
}

fn encode_single(value: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(8 + value.len());
    payload.extend_from_slice(&(value.len() as u64).to_le_bytes());
    payload.extend_from_slice(value);
    payload
}

fn encode_pair(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut payload = encode_single(left);
    payload.extend_from_slice(&(right.len() as u64).to_le_bytes());
    payload.extend_from_slice(right);
    payload
}

fn take_framed<'a>(bytes: &'a [u8], cursor: &mut usize) -> Result<&'a [u8], DeltaError> {
    let length_end = cursor
        .checked_add(8)
        .filter(|end| *end <= bytes.len())
        .ok_or(DeltaError::InvalidWire("truncated payload length"))?;
    let length = u64::from_le_bytes(
        bytes[*cursor..length_end]
            .try_into()
            .expect("payload length range"),
    );
    let length =
        usize::try_from(length).map_err(|_| DeltaError::InvalidWire("payload length overflow"))?;
    let end = length_end
        .checked_add(length)
        .filter(|end| *end <= bytes.len())
        .ok_or(DeltaError::InvalidWire("truncated payload"))?;
    *cursor = end;
    Ok(&bytes[length_end..end])
}

fn decode_single(bytes: &[u8]) -> Result<&[u8], DeltaError> {
    let mut cursor = 0;
    let value = take_framed(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(DeltaError::InvalidWire("trailing payload bytes"));
    }
    Ok(value)
}

fn decode_pair(bytes: &[u8]) -> Result<(&[u8], &[u8]), DeltaError> {
    let mut cursor = 0;
    let left = take_framed(bytes, &mut cursor)?;
    let right = take_framed(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(DeltaError::InvalidWire("trailing payload bytes"));
    }
    Ok((left, right))
}
