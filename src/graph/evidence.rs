//! Identified, field-erased bundles of independent graph-signature evidence.
//!
//! Bundles make multi-field experiments comparable without dynamic dispatch in
//! graph arithmetic. Equality means only "indistinguishable by every listed
//! channel"; it is deliberately not named or exposed as isomorphism.

use microfield::{CanonicalEncoding, Field};
use sha2::{Digest as _, Sha256};

use super::{FastGraphSignature, GraphError, GraphSignatureId};

/// Stable identity of an ordered-independent set of graph evidence channels.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct GraphEvidenceProfileId([u8; 32]);

impl GraphEvidenceProfileId {
    /// Borrows the SHA-256 domain-separated profile identity.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for GraphEvidenceProfileId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for GraphEvidenceProfileId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "GraphEvidenceProfileId({self})")
    }
}

/// One type-erased but self-identifying finite-field signature channel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphEvidenceChannel {
    signature_id: GraphSignatureId,
    canonical_signature: Vec<u8>,
}

impl GraphEvidenceChannel {
    /// Field, encoder, lane count, recurrence parameters and round profile.
    #[must_use]
    pub const fn signature_id(&self) -> GraphSignatureId {
        self.signature_id
    }

    /// Stable bytes of the complete finite-field signature.
    #[must_use]
    pub fn canonical_signature(&self) -> &[u8] {
        &self.canonical_signature
    }
}

/// Result of comparing two compatible multi-field evidence bundles.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GraphEvidenceComparison {
    /// At least one identified channel proves the signatures are different.
    Different,
    /// Every channel collided; exact graph isomorphism remains undecided.
    Indistinguishable,
}

/// Immutable evidence from one or more independently identified field profiles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiFieldGraphEvidence {
    profile_id: GraphEvidenceProfileId,
    vertex_count: u64,
    incidence_count: u64,
    total_multiplicity: u64,
    channels: Vec<GraphEvidenceChannel>,
}

impl MultiFieldGraphEvidence {
    /// Identity of the exact channel set, independent of insertion order.
    #[must_use]
    pub const fn profile_id(&self) -> GraphEvidenceProfileId {
        self.profile_id
    }

    /// Exact cheap graph metadata shared by every channel.
    #[must_use]
    pub const fn vertex_count(&self) -> u64 {
        self.vertex_count
    }

    /// Exact normalized directed-incidence count.
    #[must_use]
    pub const fn incidence_count(&self) -> u64 {
        self.incidence_count
    }

    /// Exact sum of directed multiplicities.
    #[must_use]
    pub const fn total_multiplicity(&self) -> u64 {
        self.total_multiplicity
    }

    /// Channels sorted by `GraphSignatureId`.
    #[must_use]
    pub fn channels(&self) -> &[GraphEvidenceChannel] {
        &self.channels
    }

    /// Compares only bundles with the same fully identified profile.
    ///
    /// # Errors
    ///
    /// Rejects different channel sets before looking at signature bytes.
    pub fn compare(&self, other: &Self) -> Result<GraphEvidenceComparison, GraphError> {
        if self.profile_id != other.profile_id {
            return Err(GraphError::EvidenceProfileMismatch);
        }
        if self == other {
            Ok(GraphEvidenceComparison::Indistinguishable)
        } else {
            Ok(GraphEvidenceComparison::Different)
        }
    }
}

/// Builder for heterogeneous multi-field evidence outside the arithmetic hot path.
#[derive(Clone, Debug, Default)]
pub struct MultiFieldGraphEvidenceBuilder {
    metadata: Option<(u64, u64, u64)>,
    channels: Vec<GraphEvidenceChannel>,
}

impl MultiFieldGraphEvidenceBuilder {
    /// Creates an empty evidence transaction.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            metadata: None,
            channels: Vec::new(),
        }
    }

    /// Adds one statically produced field/lane/profile signature.
    ///
    /// # Errors
    ///
    /// Rejects signatures from different graph-size metadata and duplicate
    /// profile identities. No partially built bundle is published.
    pub fn add<F, const K: usize>(
        &mut self,
        signature: &FastGraphSignature<F, K>,
    ) -> Result<&mut Self, GraphError>
    where
        F: Field + CanonicalEncoding,
    {
        let metadata = (
            signature.vertex_count(),
            signature.incidence_count(),
            signature.total_multiplicity(),
        );
        if self.metadata.is_some_and(|current| current != metadata) {
            return Err(GraphError::EvidenceGraphMetadataMismatch);
        }
        if self
            .channels
            .iter()
            .any(|channel| channel.signature_id == signature.signature_id())
        {
            return Err(GraphError::DuplicateEvidenceChannel);
        }
        self.metadata = Some(metadata);
        self.channels.push(GraphEvidenceChannel {
            signature_id: signature.signature_id(),
            canonical_signature: signature.to_canonical_bytes(),
        });
        Ok(self)
    }

    /// Validates, sorts and publishes the complete evidence bundle.
    ///
    /// # Errors
    ///
    /// Rejects an empty profile.
    pub fn build(mut self) -> Result<MultiFieldGraphEvidence, GraphError> {
        let (vertex_count, incidence_count, total_multiplicity) =
            self.metadata.ok_or(GraphError::EmptyEvidenceProfile)?;
        self.channels
            .sort_unstable_by_key(|channel| *channel.signature_id.as_bytes());
        let mut identity = Sha256::new();
        identity.update(b"microfield-multi-field-graph-evidence-v1\0");
        let channel_count =
            u64::try_from(self.channels.len()).map_err(|_| GraphError::GraphTooLarge)?;
        identity.update(channel_count.to_le_bytes());
        for channel in &self.channels {
            identity.update(channel.signature_id.as_bytes());
        }
        Ok(MultiFieldGraphEvidence {
            profile_id: GraphEvidenceProfileId(identity.finalize().into()),
            vertex_count,
            incidence_count,
            total_multiplicity,
            channels: self.channels,
        })
    }
}
