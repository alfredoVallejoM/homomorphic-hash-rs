//! Ordered signatures evaluated from both ends of a sequence.

use microfield::{CanonicalEncoding, Field, Pow, StaticField};

use super::{
    wire::{encode_header, verify_header, HEADER_BYTES},
    CanonicalElementEncoder, SignatureAssurance, SignatureContext, SignatureError, SignatureLaw,
    StructuralEncoder,
};

/// Paired Horner evaluation of a sequence and its reversal.
///
/// For a sequence `xs`, this stores `H(xs)` and `H(reverse(xs))` under the
/// same base. The second coordinate rejects many directional collisions and
/// is useful when a later graph layer must distinguish incoming from outgoing
/// order. It remains a compact finite-field fingerprint, not an equality or
/// collision-resistance proof.
#[derive(Clone, Debug)]
pub struct BidirectionalSequenceSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    context: SignatureContext,
    encoder: E,
    base: F,
    forward: F,
    reverse: F,
    next_power: F,
    length: u64,
}

impl<F, E> PartialEq for BidirectionalSequenceSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.base == other.base
            && self.forward == other.forward
            && self.reverse == other.reverse
            && self.next_power == other.next_power
            && self.length == other.length
    }
}

impl<F, E> Eq for BidirectionalSequenceSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F> BidirectionalSequenceSignature<F, CanonicalElementEncoder>
where
    F: Field + CanonicalEncoding + StaticField + Pow,
{
    /// Appends one already validated element without encoding bytes.
    ///
    /// # Errors
    ///
    /// Rejects length overflow without changing either evaluation.
    pub fn push_element(&mut self, element: F) -> Result<(), SignatureError> {
        let length = checked_increment(self.length)?;
        let forward = self.forward.mul(self.base).add(element);
        let reverse = element.mul(self.next_power).add(self.reverse);
        let next_power = self.next_power.mul(self.base);
        self.forward = forward;
        self.reverse = reverse;
        self.next_power = next_power;
        self.length = length;
        Ok(())
    }

    /// Appends validated elements as one transactional batch.
    ///
    /// # Errors
    ///
    /// Rejects length overflow without publishing partial state.
    pub fn push_elements<I>(&mut self, elements: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = F>,
    {
        let mut candidate = self.clone();
        for element in elements {
            candidate.push_element(element)?;
        }
        *self = candidate;
        Ok(())
    }

    /// Appends a borrowed element slice using two Horner passes.
    ///
    /// This avoids one general multiplication per element in the incremental
    /// reverse update. It performs no allocation and is the preferred bulk
    /// route when the caller already owns a contiguous field-element slice.
    ///
    /// # Errors
    ///
    /// Rejects length overflow without publishing partial state.
    pub fn push_elements_slice(&mut self, elements: &[F]) -> Result<(), SignatureError> {
        let additional =
            u64::try_from(elements.len()).map_err(|_| SignatureError::CounterOverflow)?;
        let length = self
            .length
            .checked_add(additional)
            .ok_or(SignatureError::CounterOverflow)?;
        let mut forward = self.forward;
        for &element in elements {
            forward = forward.mul(self.base).add(element);
        }
        let mut reverse_suffix = F::ZERO;
        for &element in elements.iter().rev() {
            reverse_suffix = reverse_suffix.mul(self.base).add(element);
        }
        let reverse = reverse_suffix.mul(self.next_power).add(self.reverse);
        let next_power = self.next_power.mul(self.base.pow(&[additional]));
        self.forward = forward;
        self.reverse = reverse;
        self.next_power = next_power;
        self.length = length;
        Ok(())
    }
}

impl<F, E> BidirectionalSequenceSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Pow,
    E: StructuralEncoder<F>,
{
    /// Creates an empty paired evaluation with a non-zero, non-one base.
    ///
    /// # Errors
    ///
    /// Rejects a base that carries no useful positional information.
    pub fn new(encoder: E, base: F) -> Result<Self, SignatureError> {
        if base.is_zero() || base == F::ONE {
            return Err(SignatureError::DegenerateSequenceBase);
        }
        let parameters = base.to_canonical();
        let context = SignatureContext::for_field::<F>(
            encoder.encoder_id(),
            SignatureLaw::BidirectionalSequence,
            parameters.as_ref(),
        );
        Ok(Self {
            context,
            encoder,
            base,
            forward: F::ZERO,
            reverse: F::ZERO,
            next_power: F::ONE,
            length: 0,
        })
    }

    /// Appends one encoded item atomically.
    ///
    /// # Errors
    ///
    /// Rejects encoding and counter failures without mutation.
    pub fn push(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let length = checked_increment(self.length)?;
        let element = self.encoder.encode(data)?;
        let forward = self.forward.mul(self.base).add(element);
        let reverse = element.mul(self.next_power).add(self.reverse);
        let next_power = self.next_power.mul(self.base);
        self.forward = forward;
        self.reverse = reverse;
        self.next_power = next_power;
        self.length = length;
        Ok(())
    }

    /// Appends a batch transactionally.
    ///
    /// # Errors
    ///
    /// Rejects the first encoder failure or overflow without partial state.
    pub fn push_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut forward = self.forward;
        let mut reverse = self.reverse;
        let mut next_power = self.next_power;
        let mut length = self.length;
        for item in items {
            length = checked_increment(length)?;
            let element = self.encoder.encode(item.as_ref())?;
            forward = forward.mul(self.base).add(element);
            reverse = element.mul(next_power).add(reverse);
            next_power = next_power.mul(self.base);
        }
        self.forward = forward;
        self.reverse = reverse;
        self.next_power = next_power;
        self.length = length;
        Ok(())
    }

    /// Appends a borrowed item slice using two Horner passes.
    ///
    /// The encoder contract is deterministic, so encoding each item in both
    /// orientations is semantically identical. This route trades a second
    /// encoding pass for eliminating a general field multiplication from each
    /// reverse update. It allocates no intermediate element buffer.
    ///
    /// # Errors
    ///
    /// Rejects the first encoder failure or overflow transactionally.
    pub fn push_slice<B>(&mut self, items: &[B]) -> Result<(), SignatureError>
    where
        B: AsRef<[u8]>,
    {
        let additional = u64::try_from(items.len()).map_err(|_| SignatureError::CounterOverflow)?;
        let length = self
            .length
            .checked_add(additional)
            .ok_or(SignatureError::CounterOverflow)?;
        let mut forward = self.forward;
        for item in items {
            forward = forward
                .mul(self.base)
                .add(self.encoder.encode(item.as_ref())?);
        }
        let mut reverse_suffix = F::ZERO;
        for item in items.iter().rev() {
            reverse_suffix = reverse_suffix
                .mul(self.base)
                .add(self.encoder.encode(item.as_ref())?);
        }
        let reverse = reverse_suffix.mul(self.next_power).add(self.reverse);
        let next_power = self.next_power.mul(self.base.pow(&[additional]));
        self.forward = forward;
        self.reverse = reverse;
        self.next_power = next_power;
        self.length = length;
        Ok(())
    }

    /// Concatenates two partitions while preserving both directions.
    ///
    /// `forward(A||B)=forward(A)·b^|B|+forward(B)` and
    /// `reverse(A||B)=reverse(B)·b^|A|+reverse(A)`.
    ///
    /// # Errors
    ///
    /// Rejects incompatible contexts and length overflow.
    pub fn concatenate(&self, suffix: &Self) -> Result<Self, SignatureError> {
        if self.context != suffix.context {
            return Err(SignatureError::IdentityMismatch);
        }
        let length = self
            .length
            .checked_add(suffix.length)
            .ok_or(SignatureError::CounterOverflow)?;
        Ok(Self {
            forward: self.forward.mul(suffix.next_power).add(suffix.forward),
            reverse: suffix.reverse.mul(self.next_power).add(self.reverse),
            next_power: self.next_power.mul(suffix.next_power),
            length,
            ..self.clone()
        })
    }

    /// Complete compatibility identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Two directions reduce collisions but remain finite fingerprints.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::Fingerprint
    }

    /// Evaluation in insertion order.
    #[must_use]
    pub const fn forward_state(&self) -> F {
        self.forward
    }

    /// Evaluation in reverse insertion order.
    #[must_use]
    pub const fn reverse_state(&self) -> F {
        self.reverse
    }

    /// Exact sequence length.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.length
    }

    /// Reports whether no item has been appended.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Positional base used by both evaluations.
    #[must_use]
    pub const fn base(&self) -> F {
        self.base
    }

    /// Serializes both states in one self-identifying canonical envelope.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let forward = self.forward.to_canonical();
        let reverse = self.reverse.to_canonical();
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 8 + 2 * forward.as_ref().len());
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.length.to_le_bytes());
        bytes.extend_from_slice(forward.as_ref());
        bytes.extend_from_slice(reverse.as_ref());
        bytes
    }

    /// Restores both evaluations under the supplied encoder and base.
    ///
    /// # Errors
    ///
    /// Rejects malformed, incompatible or non-canonical state.
    pub fn from_canonical_bytes(encoder: E, base: F, bytes: &[u8]) -> Result<Self, SignatureError> {
        let empty = Self::new(encoder, base)?;
        verify_header(bytes, empty.context)?;
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        if bytes.len() != HEADER_BYTES + 8 + 2 * repr_len {
            return Err(SignatureError::InvalidWireFormat(
                "bidirectional sequence length",
            ));
        }
        let length = u64::from_le_bytes(
            bytes[HEADER_BYTES..HEADER_BYTES + 8]
                .try_into()
                .expect("counter range"),
        );
        let states = &bytes[HEADER_BYTES + 8..];
        let forward = F::from_canonical_slice(&states[..repr_len])
            .map_err(|_| SignatureError::NonCanonicalElement)?;
        let reverse = F::from_canonical_slice(&states[repr_len..])
            .map_err(|_| SignatureError::NonCanonicalElement)?;
        if (length == 0 && (forward != F::ZERO || reverse != F::ZERO))
            || (length == 1 && forward != reverse)
        {
            return Err(SignatureError::InvalidWireFormat(
                "inconsistent bidirectional state",
            ));
        }
        Ok(Self {
            forward,
            reverse,
            next_power: base.pow(&[length]),
            length,
            ..empty
        })
    }
}

fn checked_increment(value: u64) -> Result<u64, SignatureError> {
    value.checked_add(1).ok_or(SignatureError::CounterOverflow)
}
