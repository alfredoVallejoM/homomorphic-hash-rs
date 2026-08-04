//! Ordered Horner signatures evaluated at several independent bases.

use microfield::{CanonicalEncoding, Field, Pow, StaticField};

use super::{
    wire::{encode_header, verify_header, HEADER_BYTES},
    CanonicalElementEncoder, SignatureAssurance, SignatureContext, SignatureError, SignatureLaw,
    StructuralEncoder,
};

/// Horner evaluations of one sequence at `K` distinct field bases.
///
/// For two sequences with the same encoded length `n`, equality at at least
/// `n` distinct bases proves equality of their polynomial coefficients over
/// the field. It does not prove equality of source bytes when the encoder can
/// collide, and it is not a cryptographic commitment.
#[derive(Clone, Debug)]
pub struct MultiEvaluationSequenceSignature<F, E, const K: usize>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    context: SignatureContext,
    encoder: E,
    bases: [F; K],
    states: [F; K],
    length: u64,
}

impl<F, E, const K: usize> PartialEq for MultiEvaluationSequenceSignature<F, E, K>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.bases == other.bases
            && self.states == other.states
            && self.length == other.length
    }
}

impl<F, E, const K: usize> Eq for MultiEvaluationSequenceSignature<F, E, K>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, const K: usize> MultiEvaluationSequenceSignature<F, CanonicalElementEncoder, K>
where
    F: Field + CanonicalEncoding + StaticField + Pow,
{
    /// Appends one already validated coefficient.
    ///
    /// # Errors
    ///
    /// Rejects length overflow without changing any coordinate.
    pub fn push_element(&mut self, element: F) -> Result<(), SignatureError> {
        let length = checked_increment(self.length)?;
        self.states = advance(self.states, self.bases, element);
        self.length = length;
        Ok(())
    }

    /// Appends validated coefficients as one transactional batch.
    ///
    /// # Errors
    ///
    /// Rejects length overflow without publishing partial coordinates.
    pub fn push_elements<I>(&mut self, elements: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = F>,
    {
        let mut states = self.states;
        let mut length = self.length;
        for element in elements {
            length = checked_increment(length)?;
            states = advance(states, self.bases, element);
        }
        self.states = states;
        self.length = length;
        Ok(())
    }
}

impl<F, E, const K: usize> MultiEvaluationSequenceSignature<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField + Pow,
    E: StructuralEncoder<F>,
{
    /// Creates an empty sequence at non-zero, non-one, distinct bases.
    ///
    /// # Errors
    ///
    /// Rejects an empty coordinate set, degenerate bases and duplicates.
    pub fn new(encoder: E, bases: [F; K]) -> Result<Self, SignatureError> {
        validate_bases(&bases)?;
        let coordinate_count =
            u64::try_from(K).map_err(|_| SignatureError::InvalidEvaluationPoints)?;
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        let mut parameters = Vec::with_capacity(8 + K.saturating_mul(repr_len));
        parameters.extend_from_slice(&coordinate_count.to_le_bytes());
        for base in bases {
            parameters.extend_from_slice(base.to_canonical().as_ref());
        }
        let context = SignatureContext::for_field::<F>(
            encoder.encoder_id(),
            SignatureLaw::MultiEvaluationSequence,
            &parameters,
        );
        Ok(Self {
            context,
            encoder,
            bases,
            states: [F::ZERO; K],
            length: 0,
        })
    }

    /// Encodes and appends one item atomically across all bases.
    ///
    /// # Errors
    ///
    /// Rejects encoding or length overflow without changing the state.
    pub fn push(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let length = checked_increment(self.length)?;
        let element = self.encoder.encode(data)?;
        self.states = advance(self.states, self.bases, element);
        self.length = length;
        Ok(())
    }

    /// Appends a batch transactionally across all evaluation bases.
    ///
    /// # Errors
    ///
    /// Rejects the first encoder or counter failure without partial state.
    pub fn push_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut states = self.states;
        let mut length = self.length;
        for item in items {
            length = checked_increment(length)?;
            let element = self.encoder.encode(item.as_ref())?;
            states = advance(states, self.bases, element);
        }
        self.states = states;
        self.length = length;
        Ok(())
    }

    /// Concatenates two independently evaluated sequence partitions.
    ///
    /// # Errors
    ///
    /// Rejects identity drift and total-length overflow.
    pub fn concatenate(&self, suffix: &Self) -> Result<Self, SignatureError> {
        if self.context != suffix.context {
            return Err(SignatureError::IdentityMismatch);
        }
        let length = self
            .length
            .checked_add(suffix.length)
            .ok_or(SignatureError::CounterOverflow)?;
        let states = core::array::from_fn(|index| {
            self.states[index]
                .mul(self.bases[index].pow(&[suffix.length]))
                .add(suffix.states[index])
        });
        Ok(Self {
            states,
            length,
            ..self.clone()
        })
    }

    /// Complete field, encoder, law and base identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Pairwise-distinct Horner bases.
    #[must_use]
    pub const fn bases(&self) -> &[F; K] {
        &self.bases
    }

    /// One ordered evaluation per base.
    #[must_use]
    pub const fn states(&self) -> &[F; K] {
        &self.states
    }

    /// Exact sequence length.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.length
    }

    /// Reports whether no coefficient has been appended.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Exactness bound over already encoded field coefficients.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::BoundedExactOverEncodedElements {
            maximum_cardinality: K,
        }
    }

    /// Serializes every coordinate in one self-identifying envelope.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 8 + K.saturating_mul(repr_len));
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.length.to_le_bytes());
        for state in self.states {
            bytes.extend_from_slice(state.to_canonical().as_ref());
        }
        bytes
    }

    /// Restores all evaluations under the supplied bases and encoder.
    ///
    /// # Errors
    ///
    /// Rejects malformed, non-canonical or incompatible bytes.
    pub fn from_canonical_bytes(
        encoder: E,
        bases: [F; K],
        bytes: &[u8],
    ) -> Result<Self, SignatureError> {
        let empty = Self::new(encoder, bases)?;
        verify_header(bytes, empty.context)?;
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        let expected = HEADER_BYTES
            .checked_add(8)
            .and_then(|length| length.checked_add(K.checked_mul(repr_len)?))
            .ok_or(SignatureError::InvalidWireFormat(
                "multi-evaluation sequence length overflow",
            ))?;
        if bytes.len() != expected {
            return Err(SignatureError::InvalidWireFormat(
                "multi-evaluation sequence length",
            ));
        }
        let counter: [u8; 8] = bytes[HEADER_BYTES..HEADER_BYTES + 8]
            .try_into()
            .map_err(|_| SignatureError::InvalidWireFormat("sequence counter"))?;
        let length = u64::from_le_bytes(counter);
        let mut states = [F::ZERO; K];
        for (index, state) in states.iter_mut().enumerate() {
            let start = HEADER_BYTES + 8 + index * repr_len;
            *state = F::from_canonical_slice(&bytes[start..start + repr_len])
                .map_err(|_| SignatureError::NonCanonicalElement)?;
            if length == 0 && *state != F::ZERO {
                return Err(SignatureError::InvalidWireFormat(
                    "non-zero coordinate with zero length",
                ));
            }
        }
        Ok(Self {
            states,
            length,
            ..empty
        })
    }
}

fn validate_bases<F: Field, const K: usize>(bases: &[F; K]) -> Result<(), SignatureError> {
    if K == 0 {
        return Err(SignatureError::InvalidEvaluationPoints);
    }
    for left in 0..K {
        if bases[left].is_zero() || bases[left] == F::ONE {
            return Err(SignatureError::DegenerateSequenceBase);
        }
        for right in left + 1..K {
            if bases[left] == bases[right] {
                return Err(SignatureError::InvalidEvaluationPoints);
            }
        }
    }
    Ok(())
}

fn advance<F: Field, const K: usize>(mut states: [F; K], bases: [F; K], element: F) -> [F; K] {
    for index in 0..K {
        states[index] = states[index].mul(bases[index]).add(element);
    }
    states
}

fn checked_increment(value: u64) -> Result<u64, SignatureError> {
    value.checked_add(1).ok_or(SignatureError::CounterOverflow)
}
