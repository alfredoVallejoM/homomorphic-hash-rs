//! Multi-evaluation sequence signatures over validated runtime fields.

use microfield::{DynElement, DynField};

use super::{
    dynamic::{check_element, checked_increment, encode_element, read_u64, require_context},
    wire::{encode_header, verify_header, HEADER_BYTES},
    CanonicalElementEncoder, DynamicStructuralEncoder, SignatureAssurance, SignatureContext,
    SignatureError, SignatureLaw,
};

/// Runtime Horner evaluations of one sequence at distinct bases.
#[derive(Clone, Debug)]
pub struct DynamicMultiEvaluationSequenceSignature<E>
where
    E: DynamicStructuralEncoder,
{
    context: SignatureContext,
    field: DynField,
    encoder: E,
    bases: Vec<DynElement>,
    states: Vec<DynElement>,
    length: u64,
}

impl<E> PartialEq for DynamicMultiEvaluationSequenceSignature<E>
where
    E: DynamicStructuralEncoder,
{
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.bases == other.bases
            && self.states == other.states
            && self.length == other.length
    }
}

impl<E> Eq for DynamicMultiEvaluationSequenceSignature<E> where E: DynamicStructuralEncoder {}

impl<E> DynamicMultiEvaluationSequenceSignature<E>
where
    E: DynamicStructuralEncoder,
{
    /// Creates an empty runtime sequence at distinct, non-degenerate bases.
    ///
    /// # Errors
    ///
    /// Rejects empty, repeated, zero, one or foreign-field bases.
    pub fn new(
        field: DynField,
        encoder: E,
        bases: impl Into<Vec<DynElement>>,
    ) -> Result<Self, SignatureError> {
        let bases = bases.into();
        validate_bases(&field, &bases)?;
        let count =
            u64::try_from(bases.len()).map_err(|_| SignatureError::InvalidEvaluationPoints)?;
        let mut parameters = Vec::with_capacity(8 + bases.len() * field.canonical_bytes());
        parameters.extend_from_slice(&count.to_le_bytes());
        for base in &bases {
            parameters.extend_from_slice(&encode_element(&field, base)?);
        }
        let context = SignatureContext::for_field_id(
            field.field_id(),
            encoder.encoder_id(),
            SignatureLaw::MultiEvaluationSequence,
            &parameters,
        );
        Ok(Self {
            context,
            states: vec![field.zero(); bases.len()],
            field,
            encoder,
            bases,
            length: 0,
        })
    }

    /// Encodes and appends one item atomically across all bases.
    ///
    /// # Errors
    ///
    /// Rejects encoding, field or counter failures without mutation.
    pub fn push(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let length = checked_increment(self.length)?;
        let element = self.encoder.encode_dynamic(&self.field, data)?;
        let states = advance(&self.field, self.states.clone(), &self.bases, &element)?;
        self.states = states;
        self.length = length;
        Ok(())
    }

    /// Appends a runtime batch transactionally.
    ///
    /// # Errors
    ///
    /// Rejects the first failure without publishing partial coordinates.
    pub fn push_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut states = self.states.clone();
        let mut length = self.length;
        for item in items {
            length = checked_increment(length)?;
            let element = self.encoder.encode_dynamic(&self.field, item.as_ref())?;
            states = advance(&self.field, states, &self.bases, &element)?;
        }
        self.states = states;
        self.length = length;
        Ok(())
    }

    /// Concatenates two compatible runtime sequence partitions.
    ///
    /// # Errors
    ///
    /// Rejects context drift, field failures and length overflow.
    pub fn concatenate(&self, suffix: &Self) -> Result<Self, SignatureError> {
        require_context(self.context, suffix.context)?;
        let length = self
            .length
            .checked_add(suffix.length)
            .ok_or(SignatureError::CounterOverflow)?;
        let mut states = Vec::with_capacity(self.states.len());
        for index in 0..self.states.len() {
            let power = pow_u64(&self.field, &self.bases[index], suffix.length)?;
            let prefix = self.field.mul(&self.states[index], &power)?;
            states.push(self.field.add(&prefix, &suffix.states[index])?);
        }
        Ok(Self {
            states,
            length,
            ..self.clone()
        })
    }

    /// Complete runtime field, encoder, law and bases identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Borrows all Horner bases.
    #[must_use]
    pub fn bases(&self) -> &[DynElement] {
        &self.bases
    }

    /// Borrows all ordered evaluations.
    #[must_use]
    pub fn states(&self) -> &[DynElement] {
        &self.states
    }

    /// Exact sequence length.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.length
    }

    /// Reports whether the sequence is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Exactness bound over already encoded runtime elements.
    #[must_use]
    pub fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::BoundedExactOverEncodedElements {
            maximum_cardinality: self.bases.len(),
        }
    }

    /// Serializes every coordinate using the common `MFSG` envelope.
    ///
    /// # Errors
    ///
    /// Rejects internally inconsistent runtime elements.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        let mut bytes =
            Vec::with_capacity(HEADER_BYTES + 8 + self.states.len() * self.field.canonical_bytes());
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.length.to_le_bytes());
        for state in &self.states {
            bytes.extend_from_slice(&encode_element(&self.field, state)?);
        }
        Ok(bytes)
    }

    /// Restores every runtime coordinate under explicit bases.
    ///
    /// # Errors
    ///
    /// Rejects malformed, non-canonical or incompatible state.
    pub fn from_canonical_bytes(
        field: DynField,
        encoder: E,
        bases: impl Into<Vec<DynElement>>,
        bytes: &[u8],
    ) -> Result<Self, SignatureError> {
        let empty = Self::new(field, encoder, bases)?;
        verify_header(bytes, empty.context)?;
        let width = empty.field.canonical_bytes();
        let expected = HEADER_BYTES
            .checked_add(8)
            .and_then(|value| value.checked_add(empty.bases.len().checked_mul(width)?))
            .ok_or(SignatureError::InvalidWireFormat(
                "dynamic multi-evaluation sequence length overflow",
            ))?;
        if bytes.len() != expected {
            return Err(SignatureError::InvalidWireFormat(
                "dynamic multi-evaluation sequence length",
            ));
        }
        let length = read_u64(bytes, HEADER_BYTES);
        let mut states = Vec::with_capacity(empty.bases.len());
        for index in 0..empty.bases.len() {
            let start = HEADER_BYTES + 8 + index * width;
            let state = empty.field.decode(&bytes[start..start + width])?;
            if length == 0 && state != empty.field.zero() {
                return Err(SignatureError::InvalidWireFormat(
                    "non-zero dynamic coordinate with zero length",
                ));
            }
            states.push(state);
        }
        Ok(Self {
            states,
            length,
            ..empty
        })
    }
}

impl DynamicMultiEvaluationSequenceSignature<CanonicalElementEncoder> {
    /// Appends a runtime element after checking its `FieldId`.
    ///
    /// # Errors
    ///
    /// Rejects a foreign element or overflow without mutation.
    pub fn push_element(&mut self, element: &DynElement) -> Result<(), SignatureError> {
        let length = checked_increment(self.length)?;
        check_element(&self.field, element)?;
        let states = advance(&self.field, self.states.clone(), &self.bases, element)?;
        self.states = states;
        self.length = length;
        Ok(())
    }
}

fn validate_bases(field: &DynField, bases: &[DynElement]) -> Result<(), SignatureError> {
    if bases.is_empty() {
        return Err(SignatureError::InvalidEvaluationPoints);
    }
    for (index, base) in bases.iter().enumerate() {
        check_element(field, base)?;
        if *base == field.zero() || *base == field.one() {
            return Err(SignatureError::DegenerateSequenceBase);
        }
        if bases[..index].contains(base) {
            return Err(SignatureError::InvalidEvaluationPoints);
        }
    }
    Ok(())
}

fn advance(
    field: &DynField,
    mut states: Vec<DynElement>,
    bases: &[DynElement],
    element: &DynElement,
) -> Result<Vec<DynElement>, SignatureError> {
    for index in 0..states.len() {
        states[index] = field.add(&field.mul(&states[index], &bases[index])?, element)?;
    }
    Ok(states)
}

fn pow_u64(
    field: &DynField,
    value: &DynElement,
    mut exponent: u64,
) -> Result<DynElement, SignatureError> {
    let mut result = field.one();
    let mut base = value.clone();
    while exponent != 0 {
        if exponent & 1 == 1 {
            result = field.mul(&result, &base)?;
        }
        exponent >>= 1;
        if exponent != 0 {
            base = field.square(&base)?;
        }
    }
    Ok(result)
}
