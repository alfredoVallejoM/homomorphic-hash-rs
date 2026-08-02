//! Bidirectional sequence signatures over validated runtime fields.

use microfield::{DynElement, DynField};

use super::{
    dynamic::{
        check_element, checked_increment, encode_element, pow_u64, read_u64, require_context,
    },
    wire::{encode_header, verify_header, HEADER_BYTES},
    CanonicalElementEncoder, DynamicStructuralEncoder, SignatureContext, SignatureError,
    SignatureLaw,
};

/// Runtime-field counterpart of [`super::BidirectionalSequenceSignature`].
///
/// Runtime validation and allocation are deliberately confined to this
/// opt-in type; generated static fields keep their monomorphized hot path.
#[derive(Clone, Debug)]
pub struct DynamicBidirectionalSequenceSignature<E>
where
    E: DynamicStructuralEncoder,
{
    context: SignatureContext,
    field: DynField,
    encoder: E,
    base: DynElement,
    forward: DynElement,
    reverse: DynElement,
    next_power: DynElement,
    length: u64,
}

impl<E> PartialEq for DynamicBidirectionalSequenceSignature<E>
where
    E: DynamicStructuralEncoder,
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

impl<E> Eq for DynamicBidirectionalSequenceSignature<E> where E: DynamicStructuralEncoder {}

impl<E> DynamicBidirectionalSequenceSignature<E>
where
    E: DynamicStructuralEncoder,
{
    /// Creates an empty paired evaluation in one runtime context.
    ///
    /// # Errors
    ///
    /// Rejects a mixed-field, zero or one base.
    pub fn new(field: DynField, encoder: E, base: DynElement) -> Result<Self, SignatureError> {
        check_element(&field, &base)?;
        if base == field.zero() || base == field.one() {
            return Err(SignatureError::DegenerateSequenceBase);
        }
        let parameters = encode_element(&field, &base)?;
        let context = SignatureContext::for_field_id(
            field.field_id(),
            encoder.encoder_id(),
            SignatureLaw::BidirectionalSequence,
            &parameters,
        );
        Ok(Self {
            context,
            encoder,
            forward: field.zero(),
            reverse: field.zero(),
            next_power: field.one(),
            field,
            base,
            length: 0,
        })
    }

    /// Encodes and appends one item atomically.
    ///
    /// # Errors
    ///
    /// Rejects encoder, field and counter failures without mutation.
    pub fn push(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let length = checked_increment(self.length)?;
        let element = self.encoder.encode_dynamic(&self.field, data)?;
        let forward_product = self.field.mul(&self.forward, &self.base)?;
        let forward = self.field.add(&forward_product, &element)?;
        let reverse_product = self.field.mul(&element, &self.next_power)?;
        let reverse = self.field.add(&reverse_product, &self.reverse)?;
        let next_power = self.field.mul(&self.next_power, &self.base)?;
        self.forward = forward;
        self.reverse = reverse;
        self.next_power = next_power;
        self.length = length;
        Ok(())
    }

    /// Appends a batch and publishes it only after complete success.
    ///
    /// # Errors
    ///
    /// Rejects the first failure without publishing partial state.
    pub fn push_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut candidate = self.clone();
        for item in items {
            candidate.push(item.as_ref())?;
        }
        *self = candidate;
        Ok(())
    }

    /// Concatenates independently accumulated runtime partitions.
    ///
    /// # Errors
    ///
    /// Rejects identity drift, overflow or runtime arithmetic failure.
    pub fn concatenate(&self, suffix: &Self) -> Result<Self, SignatureError> {
        require_context(self.context, suffix.context)?;
        let length = self
            .length
            .checked_add(suffix.length)
            .ok_or(SignatureError::CounterOverflow)?;
        let forward_prefix = self.field.mul(&self.forward, &suffix.next_power)?;
        let forward = self.field.add(&forward_prefix, &suffix.forward)?;
        let reverse_suffix = self.field.mul(&suffix.reverse, &self.next_power)?;
        let reverse = self.field.add(&reverse_suffix, &self.reverse)?;
        let next_power = self.field.mul(&self.next_power, &suffix.next_power)?;
        Ok(Self {
            forward,
            reverse,
            next_power,
            length,
            ..self.clone()
        })
    }

    /// Complete compatibility identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Borrows the insertion-order evaluation.
    #[must_use]
    pub const fn forward_state(&self) -> &DynElement {
        &self.forward
    }

    /// Borrows the reverse-order evaluation.
    #[must_use]
    pub const fn reverse_state(&self) -> &DynElement {
        &self.reverse
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

    /// Serializes both runtime states using the common `MFSG` schema.
    ///
    /// # Errors
    ///
    /// Rejects internally inconsistent runtime elements.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        let forward = encode_element(&self.field, &self.forward)?;
        let reverse = encode_element(&self.field, &self.reverse)?;
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 8 + forward.len() + reverse.len());
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.length.to_le_bytes());
        bytes.extend_from_slice(&forward);
        bytes.extend_from_slice(&reverse);
        Ok(bytes)
    }

    /// Restores a paired runtime evaluation.
    ///
    /// # Errors
    ///
    /// Rejects malformed, incompatible or non-canonical state.
    pub fn from_canonical_bytes(
        field: DynField,
        encoder: E,
        base: DynElement,
        bytes: &[u8],
    ) -> Result<Self, SignatureError> {
        let empty = Self::new(field, encoder, base)?;
        verify_header(bytes, empty.context)?;
        let width = empty.field.canonical_bytes();
        if bytes.len() != HEADER_BYTES + 8 + 2 * width {
            return Err(SignatureError::InvalidWireFormat(
                "dynamic bidirectional sequence length",
            ));
        }
        let length = read_u64(bytes, HEADER_BYTES);
        let states = &bytes[HEADER_BYTES + 8..];
        let forward = empty.field.decode(&states[..width])?;
        let reverse = empty.field.decode(&states[width..])?;
        if (length == 0 && (forward != empty.field.zero() || reverse != empty.field.zero()))
            || (length == 1 && forward != reverse)
        {
            return Err(SignatureError::InvalidWireFormat(
                "inconsistent dynamic bidirectional state",
            ));
        }
        let next_power = pow_u64(&empty.field, &empty.base, length)?;
        Ok(Self {
            forward,
            reverse,
            next_power,
            length,
            ..empty
        })
    }
}

impl DynamicBidirectionalSequenceSignature<CanonicalElementEncoder> {
    /// Appends an element after validating its runtime field identity.
    ///
    /// # Errors
    ///
    /// Rejects a foreign element or counter overflow atomically.
    pub fn push_element(&mut self, element: &DynElement) -> Result<(), SignatureError> {
        let length = checked_increment(self.length)?;
        check_element(&self.field, element)?;
        let forward_product = self.field.mul(&self.forward, &self.base)?;
        let forward = self.field.add(&forward_product, element)?;
        let reverse_product = self.field.mul(element, &self.next_power)?;
        let reverse = self.field.add(&reverse_product, &self.reverse)?;
        let next_power = self.field.mul(&self.next_power, &self.base)?;
        self.forward = forward;
        self.reverse = reverse;
        self.next_power = next_power;
        self.length = length;
        Ok(())
    }
}
