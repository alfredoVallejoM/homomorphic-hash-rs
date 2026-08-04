//! Stable identities for encoders and structural laws.

use core::fmt;

use microfield::{FieldId, StaticField};
use sha2::{Digest as _, Sha256};

/// Stable identity of byte-to-field semantics.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct EncoderId([u8; 32]);

/// Stable identity of field, encoder, law and law parameters.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct SignatureId([u8; 32]);

/// Structural combination law encoded in a signature.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u8)]
pub enum SignatureLaw {
    /// Addition in the field; parity when the characteristic is two.
    Additive = 1,
    /// Ordered Horner evaluation with explicit length.
    Sequence = 2,
    /// Commutative product with explicit zero-factor count.
    Multiset = 3,
    /// Two ordered Horner evaluations, one in each direction.
    BidirectionalSequence = 4,
    /// Commutative products evaluated at multiple independent offsets.
    MultiEvaluationMultiset = 5,
    /// Ordered Horner evaluations at several pairwise-distinct bases.
    MultiEvaluationSequence = 6,
}

/// Complete compatibility identity attached to every state.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SignatureContext {
    field_id: FieldId,
    encoder_id: EncoderId,
    signature_id: SignatureId,
    law: SignatureLaw,
}

impl EncoderId {
    /// Borrows the serialized digest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn derive(descriptor: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"microfield-structural-encoder-v1\0");
        hasher.update(descriptor);
        Self(hasher.finalize().into())
    }

    pub(crate) fn derive_tagged(descriptor: &[u8], domain_tag: u64) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"microfield-structural-encoder-v1\0");
        hasher.update(descriptor);
        hasher.update(domain_tag.to_le_bytes());
        Self(hasher.finalize().into())
    }
}

impl SignatureId {
    /// Borrows the serialized digest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    fn derive(
        field_id: FieldId,
        encoder_id: EncoderId,
        law: SignatureLaw,
        parameters: &[u8],
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"microfield-structural-signature-v1\0");
        hasher.update(field_id.as_bytes());
        hasher.update(encoder_id.as_bytes());
        hasher.update([law as u8]);
        hasher.update((parameters.len() as u64).to_le_bytes());
        hasher.update(parameters);
        Self(hasher.finalize().into())
    }
}

impl SignatureContext {
    pub(crate) fn for_field<F: StaticField>(
        encoder_id: EncoderId,
        law: SignatureLaw,
        parameters: &[u8],
    ) -> Self {
        Self::for_field_id(F::spec().field_id(), encoder_id, law, parameters)
    }

    pub(crate) fn for_field_id(
        field_id: FieldId,
        encoder_id: EncoderId,
        law: SignatureLaw,
        parameters: &[u8],
    ) -> Self {
        Self {
            field_id,
            encoder_id,
            signature_id: SignatureId::derive(field_id, encoder_id, law, parameters),
            law,
        }
    }

    /// Field presentation used by the state.
    #[must_use]
    pub const fn field_id(self) -> FieldId {
        self.field_id
    }

    /// Encoder semantics used by the state.
    #[must_use]
    pub const fn encoder_id(self) -> EncoderId {
        self.encoder_id
    }

    /// Complete compatibility identity.
    #[must_use]
    pub const fn signature_id(self) -> SignatureId {
        self.signature_id
    }

    /// Combination law used by the state.
    #[must_use]
    pub const fn law(self) -> SignatureLaw {
        self.law
    }
}

macro_rules! impl_digest_format {
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

impl_digest_format!(EncoderId, "EncoderId");
impl_digest_format!(SignatureId, "SignatureId");
