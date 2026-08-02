//! Additive structural signatures.

use microfield::{CanonicalEncoding, Field, StaticField};

use super::{
    wire::{encode_header, verify_header, HEADER_BYTES},
    CanonicalElementEncoder, SignatureContext, SignatureError, SignatureLaw, StructuralEncoder,
};

/// Commutative field sum of encoded terms with an exact absorbed-term count.
///
/// In characteristic two this captures multiplicity parity. It is not a set:
/// even multiplicities cancel and encoder/field collisions remain possible.
#[derive(Clone, Debug)]
pub struct AdditiveSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    context: SignatureContext,
    encoder: E,
    state: F,
    term_count: u64,
}

impl<F> AdditiveSignature<F, CanonicalElementEncoder>
where
    F: Field + CanonicalEncoding + StaticField,
{
    /// Absorbs an already validated field element without encoding bytes.
    ///
    /// This route preserves the canonical-element `EncoderId` and is useful
    /// for generated fields and callers that already operate in `F`.
    ///
    /// # Errors
    ///
    /// Rejects counter overflow without changing state.
    pub fn absorb_element(&mut self, element: F) -> Result<(), SignatureError> {
        let term_count = self
            .term_count
            .checked_add(1)
            .ok_or(SignatureError::CounterOverflow)?;
        self.state = self.state.add(element);
        self.term_count = term_count;
        Ok(())
    }

    /// Absorbs validated elements as one transactional batch.
    ///
    /// # Errors
    ///
    /// Rejects counter overflow without publishing a partial sum.
    pub fn absorb_elements<I>(&mut self, elements: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = F>,
    {
        let mut state = self.state;
        let mut term_count = self.term_count;
        for element in elements {
            term_count = term_count
                .checked_add(1)
                .ok_or(SignatureError::CounterOverflow)?;
            state = state.add(element);
        }
        self.state = state;
        self.term_count = term_count;
        Ok(())
    }
}

impl<F, E> PartialEq for AdditiveSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.state == other.state
            && self.term_count == other.term_count
    }
}

impl<F, E> Eq for AdditiveSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, E> AdditiveSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField,
    E: StructuralEncoder<F>,
{
    /// Creates the additive identity for one encoder and field.
    #[must_use]
    pub fn new(encoder: E) -> Self {
        let context = SignatureContext::for_field::<F>(
            encoder.encoder_id(),
            SignatureLaw::Additive,
            b"field-addition-v1",
        );
        Self {
            context,
            encoder,
            state: F::ZERO,
            term_count: 0,
        }
    }

    /// Encodes and absorbs one term atomically.
    ///
    /// # Errors
    ///
    /// Rejects encoder failures and counter overflow without changing state.
    pub fn absorb(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let next_count = self
            .term_count
            .checked_add(1)
            .ok_or(SignatureError::CounterOverflow)?;
        let element = self.encoder.encode(data)?;
        self.state = self.state.add(element);
        self.term_count = next_count;
        Ok(())
    }

    /// Encodes a batch and publishes it only if every term is accepted.
    ///
    /// This is the preferred ingestion boundary for fallible encoders: an
    /// invalid item or counter overflow leaves the complete signature intact.
    /// The iterator itself is not retained and the algebraic loop allocates no
    /// memory.
    ///
    /// # Errors
    ///
    /// Rejects the first encoder failure or counter overflow transactionally.
    pub fn absorb_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut state = self.state;
        let mut term_count = self.term_count;
        for item in items {
            term_count = term_count
                .checked_add(1)
                .ok_or(SignatureError::CounterOverflow)?;
            state = state.add(self.encoder.encode(item.as_ref())?);
        }
        self.state = state;
        self.term_count = term_count;
        Ok(())
    }

    /// Combines independently accumulated partitions.
    ///
    /// # Errors
    ///
    /// Rejects incompatible identities and counter overflow.
    pub fn combine(&self, other: &Self) -> Result<Self, SignatureError> {
        if self.context != other.context {
            return Err(SignatureError::IdentityMismatch);
        }
        let term_count = self
            .term_count
            .checked_add(other.term_count)
            .ok_or(SignatureError::CounterOverflow)?;
        Ok(Self {
            context: self.context,
            encoder: self.encoder.clone(),
            state: self.state.add(other.state),
            term_count,
        })
    }

    /// Complete compatibility identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Accumulated field value.
    #[must_use]
    pub const fn state(&self) -> F {
        self.state
    }

    /// Number of absorbed terms before characteristic-dependent cancellation.
    #[must_use]
    pub const fn term_count(&self) -> u64 {
        self.term_count
    }

    /// Serializes a stable, self-identifying little-endian envelope.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let repr = self.state.to_canonical();
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 8 + repr.as_ref().len());
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.term_count.to_le_bytes());
        bytes.extend_from_slice(repr.as_ref());
        bytes
    }

    /// Parses an envelope only under the supplied encoder's exact identity.
    ///
    /// # Errors
    ///
    /// Rejects truncation, trailing bytes, identity drift and non-canonical
    /// field state.
    pub fn from_canonical_bytes(encoder: E, bytes: &[u8]) -> Result<Self, SignatureError> {
        let empty = Self::new(encoder);
        verify_header(bytes, empty.context)?;
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        if bytes.len() != HEADER_BYTES + 8 + repr_len {
            return Err(SignatureError::InvalidWireFormat("additive length"));
        }
        let term_count = u64::from_le_bytes(
            bytes[HEADER_BYTES..HEADER_BYTES + 8]
                .try_into()
                .expect("counter range"),
        );
        let state = F::from_canonical_slice(&bytes[HEADER_BYTES + 8..])
            .map_err(|_| SignatureError::NonCanonicalElement)?;
        if term_count == 0 && state != F::ZERO {
            return Err(SignatureError::InvalidWireFormat(
                "non-zero additive state with zero terms",
            ));
        }
        Ok(Self {
            state,
            term_count,
            ..empty
        })
    }
}
