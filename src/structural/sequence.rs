//! Ordered Horner signatures with explicit length and concatenation law.

use microfield::{CanonicalEncoding, Field, Invert, Pow, StaticField};

use super::{
    snapshot::{decode_snapshot, encode_snapshot, SEQUENCE_KIND},
    wire::{encode_header, verify_header, HEADER_BYTES},
    AlgebraicResidual, CanonicalElementEncoder, SignatureAssurance, SignatureContext,
    SignatureError, SignatureLaw, StructuralEncoder, TrackedSnapshotLimits,
};

/// Ordered structural signature `H(xs) = (...(x₀·b + x₁)·b + ...)`.
#[derive(Clone, Debug)]
pub struct SequenceSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    context: SignatureContext,
    encoder: E,
    base: F,
    base_inverse: F,
    state: F,
    length: u64,
}

impl<F> SequenceSignature<F, CanonicalElementEncoder>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
{
    /// Appends an already validated field element without a byte round trip.
    ///
    /// # Errors
    ///
    /// Rejects length overflow without changing state.
    pub fn push_element(&mut self, element: F) -> Result<(), SignatureError> {
        let length = self
            .length
            .checked_add(1)
            .ok_or(SignatureError::CounterOverflow)?;
        self.state = self.state.mul(self.base).add(element);
        self.length = length;
        Ok(())
    }

    /// Appends validated elements as one transactional batch.
    ///
    /// # Errors
    ///
    /// Rejects length overflow without publishing a partial evaluation.
    pub fn push_elements<I>(&mut self, elements: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = F>,
    {
        let mut state = self.state;
        let mut length = self.length;
        for element in elements {
            length = length
                .checked_add(1)
                .ok_or(SignatureError::CounterOverflow)?;
            state = state.mul(self.base).add(element);
        }
        self.state = state;
        self.length = length;
        Ok(())
    }
}

impl<F, E> PartialEq for SequenceSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.base == other.base
            && self.state == other.state
            && self.length == other.length
    }
}

impl<F, E> Eq for SequenceSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, E> SequenceSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
    E: StructuralEncoder<F>,
{
    /// Creates an empty sequence using an explicit non-zero, non-one base.
    ///
    /// # Errors
    ///
    /// Rejects bases that cannot carry positional information.
    pub fn new(encoder: E, base: F) -> Result<Self, SignatureError> {
        if base.is_zero() || base == F::ONE {
            return Err(SignatureError::DegenerateSequenceBase);
        }
        let base_inverse = base
            .invert()
            .ok_or(SignatureError::DegenerateSequenceBase)?;
        let parameters = base.to_canonical();
        let context = SignatureContext::for_field::<F>(
            encoder.encoder_id(),
            SignatureLaw::Sequence,
            parameters.as_ref(),
        );
        Ok(Self {
            context,
            encoder,
            base,
            base_inverse,
            state: F::ZERO,
            length: 0,
        })
    }

    /// Appends one encoded item atomically.
    ///
    /// # Errors
    ///
    /// Rejects encoder failures and length overflow without changing state.
    pub fn push(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let next_length = self
            .length
            .checked_add(1)
            .ok_or(SignatureError::CounterOverflow)?;
        let element = self.encoder.encode(data)?;
        self.state = self.state.mul(self.base).add(element);
        self.length = next_length;
        Ok(())
    }

    /// Appends a batch and publishes it only after every item is accepted.
    ///
    /// # Errors
    ///
    /// Rejects the first encoder failure or length overflow without changing
    /// either the evaluation or its length.
    pub fn push_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut state = self.state;
        let mut length = self.length;
        for item in items {
            length = length
                .checked_add(1)
                .ok_or(SignatureError::CounterOverflow)?;
            state = state
                .mul(self.base)
                .add(self.encoder.encode(item.as_ref())?);
        }
        self.state = state;
        self.length = length;
        Ok(())
    }

    /// Concatenates two independently accumulated sequences.
    ///
    /// `H(A || B) = H(A) · base^len(B) + H(B)`.
    ///
    /// # Errors
    ///
    /// Rejects context drift and length overflow.
    pub fn concatenate(&self, suffix: &Self) -> Result<Self, SignatureError> {
        if self.context != suffix.context {
            return Err(SignatureError::IdentityMismatch);
        }
        let length = self
            .length
            .checked_add(suffix.length)
            .ok_or(SignatureError::CounterOverflow)?;
        let power = self.base.pow(&[suffix.length]);
        Ok(Self {
            state: self.state.mul(power).add(suffix.state),
            length,
            ..self.clone()
        })
    }

    pub(crate) fn trim_assuming_suffix(&self, suffix: &Self) -> Result<Self, SignatureError> {
        if self.context != suffix.context {
            return Err(SignatureError::IdentityMismatch);
        }
        let length = self
            .length
            .checked_sub(suffix.length)
            .ok_or(SignatureError::ItemAbsent)?;
        let inverse_power = self.base_inverse.pow(&[suffix.length]);
        Ok(Self {
            state: self.state.sub(suffix.state).mul(inverse_power),
            length,
            ..self.clone()
        })
    }

    /// Derives the algebraic predecessor for an assumed last item.
    ///
    /// This does not establish that `data` was actually last. Use a tracked
    /// application collection when that guarantee is required.
    ///
    /// # Errors
    ///
    /// Rejects an empty state or encoder failure.
    pub fn residual_assuming_last(
        &self,
        data: &[u8],
    ) -> Result<AlgebraicResidual<F>, SignatureError> {
        let length = self
            .length
            .checked_sub(1)
            .ok_or(SignatureError::EmptyState)?;
        let element = self.encoder.encode(data)?;
        Ok(AlgebraicResidual {
            signature_id: self.context.signature_id(),
            law: SignatureLaw::Sequence,
            state: self.state.sub(element).mul(self.base_inverse),
            item_count: length,
            zero_factor_count: 0,
        })
    }

    /// Checks only the forward Horner equation represented by a residual.
    ///
    /// # Errors
    ///
    /// Rejects identity drift and encoder failures.
    pub fn verify_residual(
        &self,
        data: &[u8],
        residual: &AlgebraicResidual<F>,
    ) -> Result<bool, SignatureError> {
        if residual.signature_id != self.context.signature_id()
            || residual.law != SignatureLaw::Sequence
        {
            return Err(SignatureError::IdentityMismatch);
        }
        let Some(recomposed_length) = residual.item_count.checked_add(1) else {
            return Ok(false);
        };
        let element = self.encoder.encode(data)?;
        Ok(recomposed_length == self.length
            && residual.state.mul(self.base).add(element) == self.state)
    }

    /// Complete compatibility identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Ordered field evaluation.
    #[must_use]
    pub const fn state(&self) -> F {
        self.state
    }

    /// Exact item count.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.length
    }

    /// Reports whether no item has been appended.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Positional Horner base.
    #[must_use]
    pub const fn base(&self) -> F {
        self.base
    }

    /// Equality remains a finite-field fingerprint for untracked sequences.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::Fingerprint
    }

    /// Serializes a stable, self-identifying little-endian envelope.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let repr = self.state.to_canonical();
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 8 + repr.as_ref().len());
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.length.to_le_bytes());
        bytes.extend_from_slice(repr.as_ref());
        bytes
    }

    /// Parses an envelope under the supplied encoder and base.
    ///
    /// # Errors
    ///
    /// Rejects malformed, non-canonical or incompatible bytes.
    pub fn from_canonical_bytes(encoder: E, base: F, bytes: &[u8]) -> Result<Self, SignatureError> {
        let empty = Self::new(encoder, base)?;
        verify_header(bytes, empty.context)?;
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        if bytes.len() != HEADER_BYTES + 8 + repr_len {
            return Err(SignatureError::InvalidWireFormat("sequence length"));
        }
        let length = u64::from_le_bytes(
            bytes[HEADER_BYTES..HEADER_BYTES + 8]
                .try_into()
                .expect("counter range"),
        );
        let state = F::from_canonical_slice(&bytes[HEADER_BYTES + 8..])
            .map_err(|_| SignatureError::NonCanonicalElement)?;
        if length == 0 && state != F::ZERO {
            return Err(SignatureError::InvalidWireFormat(
                "non-zero sequence state with zero length",
            ));
        }
        Ok(Self {
            state,
            length,
            ..empty
        })
    }
}

/// Sequence that keeps exact raw items so a pop can verify actual order.
#[derive(Clone, Debug)]
pub struct TrackedSequence<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    signature: SequenceSignature<F, E>,
    items: Vec<Vec<u8>>,
}

impl<F, E> PartialEq for TrackedSequence<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    fn eq(&self, other: &Self) -> bool {
        self.signature == other.signature && self.items == other.items
    }
}

impl<F, E> Eq for TrackedSequence<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, E> TrackedSequence<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
    E: StructuralEncoder<F>,
{
    /// Creates an exact tracked sequence.
    ///
    /// # Errors
    ///
    /// Rejects a degenerate base.
    pub fn new(encoder: E, base: F) -> Result<Self, SignatureError> {
        Ok(Self {
            signature: SequenceSignature::new(encoder, base)?,
            items: Vec::new(),
        })
    }

    /// Appends data only after the signature accepts it.
    ///
    /// # Errors
    ///
    /// Propagates encoder and counter errors transactionally.
    pub fn push(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        self.signature.push(data)?;
        self.items.push(data.to_vec());
        Ok(())
    }

    /// Removes and returns the exact last raw item.
    ///
    /// # Errors
    ///
    /// Rejects an empty sequence.
    pub fn pop(&mut self) -> Result<Vec<u8>, SignatureError> {
        let item = self.items.last().ok_or(SignatureError::EmptyState)?;
        let residual = self.signature.residual_assuming_last(item)?;
        self.signature.state = residual.state;
        self.signature.length = residual.item_count;
        Ok(self.items.pop().expect("last item was checked"))
    }

    /// Borrows the compact structural signature.
    #[must_use]
    pub const fn signature(&self) -> &SequenceSignature<F, E> {
        &self.signature
    }

    /// Raw source items are retained and compared exactly.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::ExactTracked
    }

    /// Serializes both the compact state and every exact source item.
    ///
    /// This uses the distinct `MFTS` schema; it cannot be confused with a
    /// compact `MFSG` signature.
    ///
    /// # Errors
    ///
    /// Rejects configured limits, size overflow or allocation failure.
    pub fn to_snapshot_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        self.to_snapshot_bytes_with_limits(TrackedSnapshotLimits::default())
    }

    /// Serializes an exact snapshot under explicit defensive limits.
    ///
    /// # Errors
    ///
    /// Rejects configured limits, size overflow or allocation failure.
    pub fn to_snapshot_bytes_with_limits(
        &self,
        limits: TrackedSnapshotLimits,
    ) -> Result<Vec<u8>, SignatureError> {
        let compact = self.signature.to_canonical_bytes();
        encode_snapshot(
            SEQUENCE_KIND,
            &compact,
            self.items.iter().map(|item| (item.as_slice(), 1)),
            limits,
        )
    }

    /// Restores an exact tracked sequence and revalidates its compact state.
    ///
    /// # Errors
    ///
    /// Rejects malformed data, identity drift, resource ceilings or any
    /// disagreement between retained items and the embedded compact signature.
    pub fn from_snapshot_bytes(encoder: E, base: F, bytes: &[u8]) -> Result<Self, SignatureError> {
        Self::from_snapshot_bytes_with_limits(
            encoder,
            base,
            bytes,
            TrackedSnapshotLimits::default(),
        )
    }

    /// Restores an exact tracked sequence under explicit defensive limits.
    ///
    /// # Errors
    ///
    /// Rejects malformed, excessive or algebraically inconsistent snapshots.
    pub fn from_snapshot_bytes_with_limits(
        encoder: E,
        base: F,
        bytes: &[u8],
        limits: TrackedSnapshotLimits,
    ) -> Result<Self, SignatureError> {
        let decoded = decode_snapshot(bytes, SEQUENCE_KIND, limits)?;
        let expected =
            SequenceSignature::from_canonical_bytes(encoder.clone(), base, decoded.compact)?;
        let mut candidate = Self::new(encoder, base)?;
        candidate
            .items
            .try_reserve_exact(decoded.entries.len())
            .map_err(|_| SignatureError::AllocationFailed)?;
        for (item, multiplicity) in decoded.entries {
            if multiplicity != 1 {
                return Err(SignatureError::InvalidWireFormat(
                    "tracked sequence multiplicity",
                ));
            }
            candidate.push(&item)?;
        }
        if candidate.signature != expected {
            return Err(SignatureError::InvalidWireFormat(
                "tracked sequence compact mismatch",
            ));
        }
        Ok(candidate)
    }
}
