//! Structural signatures over validated runtime field contexts.
//!
//! These adapters intentionally keep `DynField` checks and owned elements out
//! of the monomorphized static types. They are enabled by `dynamic-fields`.

use microfield::{DynElement, DynField};

use super::{
    wire::{encode_header, verify_header, HEADER_BYTES},
    CanonicalElementEncoder, DynamicStructuralEncoder, SignatureContext, SignatureError,
    SignatureId, SignatureLaw,
};

/// Runtime-field algebraic remainder without a membership claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicAlgebraicResidual {
    signature_id: SignatureId,
    law: SignatureLaw,
    state: DynElement,
    item_count: u64,
    zero_factor_count: u64,
}

impl DynamicAlgebraicResidual {
    /// Identity of the recomposition equation.
    #[must_use]
    pub const fn signature_id(&self) -> SignatureId {
        self.signature_id
    }

    /// Structural law represented by this residual.
    #[must_use]
    pub const fn law(&self) -> SignatureLaw {
        self.law
    }

    /// Remaining runtime field state.
    #[must_use]
    pub const fn state(&self) -> &DynElement {
        &self.state
    }

    /// Remaining logical item count.
    #[must_use]
    pub const fn item_count(&self) -> u64 {
        self.item_count
    }

    /// Remaining zero-factor count for a multiset law.
    #[must_use]
    pub const fn zero_factor_count(&self) -> u64 {
        self.zero_factor_count
    }
}

/// Additive signature carrying one validated runtime field context.
#[derive(Clone, Debug)]
pub struct DynamicAdditiveSignature<E>
where
    E: DynamicStructuralEncoder,
{
    context: SignatureContext,
    field: DynField,
    encoder: E,
    state: DynElement,
    term_count: u64,
}

impl<E> PartialEq for DynamicAdditiveSignature<E>
where
    E: DynamicStructuralEncoder,
{
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.state == other.state
            && self.term_count == other.term_count
    }
}

impl<E> Eq for DynamicAdditiveSignature<E> where E: DynamicStructuralEncoder {}

impl<E> DynamicAdditiveSignature<E>
where
    E: DynamicStructuralEncoder,
{
    /// Creates an empty signature bound to the runtime `FieldId`.
    #[must_use]
    pub fn new(field: DynField, encoder: E) -> Self {
        let context = SignatureContext::for_field_id(
            field.field_id(),
            encoder.encoder_id(),
            SignatureLaw::Additive,
            b"field-addition-v1",
        );
        let state = field.zero();
        Self {
            context,
            field,
            encoder,
            state,
            term_count: 0,
        }
    }

    /// Encodes and absorbs one item atomically.
    ///
    /// # Errors
    ///
    /// Rejects encoding, field and counter failures without changing state.
    pub fn absorb(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let term_count = checked_increment(self.term_count)?;
        let element = self.encoder.encode_dynamic(&self.field, data)?;
        let state = self.field.add(&self.state, &element)?;
        self.state = state;
        self.term_count = term_count;
        Ok(())
    }

    /// Encodes a batch and publishes it only after complete success.
    ///
    /// # Errors
    ///
    /// Rejects the first failure without publishing a partial sum.
    pub fn absorb_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut state = self.state.clone();
        let mut term_count = self.term_count;
        for item in items {
            term_count = checked_increment(term_count)?;
            let element = self.encoder.encode_dynamic(&self.field, item.as_ref())?;
            state = self.field.add(&state, &element)?;
        }
        self.state = state;
        self.term_count = term_count;
        Ok(())
    }

    /// Combines independently accumulated runtime partitions.
    ///
    /// # Errors
    ///
    /// Rejects identity drift, overflow and runtime field mismatch.
    pub fn combine(&self, other: &Self) -> Result<Self, SignatureError> {
        require_context(self.context, other.context)?;
        let term_count = self
            .term_count
            .checked_add(other.term_count)
            .ok_or(SignatureError::CounterOverflow)?;
        let state = self.field.add(&self.state, &other.state)?;
        Ok(Self {
            state,
            term_count,
            ..self.clone()
        })
    }

    /// Complete field/encoder/law identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Borrows the accumulated runtime element.
    #[must_use]
    pub const fn state(&self) -> &DynElement {
        &self.state
    }

    /// Exact absorbed item count.
    #[must_use]
    pub const fn term_count(&self) -> u64 {
        self.term_count
    }

    /// Serializes the same `MFSG` schema used by static signatures.
    ///
    /// # Errors
    ///
    /// Rejects an internally inconsistent runtime element.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        let state = encode_element(&self.field, &self.state)?;
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 8 + state.len());
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.term_count.to_le_bytes());
        bytes.extend_from_slice(&state);
        Ok(bytes)
    }

    /// Restores a signature under the supplied runtime context and encoder.
    ///
    /// # Errors
    ///
    /// Rejects malformed, non-canonical or incompatible bytes.
    pub fn from_canonical_bytes(
        field: DynField,
        encoder: E,
        bytes: &[u8],
    ) -> Result<Self, SignatureError> {
        let empty = Self::new(field, encoder);
        verify_header(bytes, empty.context)?;
        let expected = HEADER_BYTES + 8 + empty.field.canonical_bytes();
        if bytes.len() != expected {
            return Err(SignatureError::InvalidWireFormat("dynamic additive length"));
        }
        let term_count = read_u64(bytes, HEADER_BYTES);
        let state = empty.field.decode(&bytes[HEADER_BYTES + 8..])?;
        if term_count == 0 && state != empty.field.zero() {
            return Err(SignatureError::InvalidWireFormat(
                "non-zero dynamic additive state with zero terms",
            ));
        }
        Ok(Self {
            state,
            term_count,
            ..empty
        })
    }
}

impl DynamicAdditiveSignature<CanonicalElementEncoder> {
    /// Absorbs an element after checking its runtime `FieldId`.
    ///
    /// # Errors
    ///
    /// Rejects mixed fields and counter overflow atomically.
    pub fn absorb_element(&mut self, element: &DynElement) -> Result<(), SignatureError> {
        let term_count = checked_increment(self.term_count)?;
        let state = self.field.add(&self.state, element)?;
        self.state = state;
        self.term_count = term_count;
        Ok(())
    }
}

/// Ordered Horner signature over one validated runtime field.
#[derive(Clone, Debug)]
pub struct DynamicSequenceSignature<E>
where
    E: DynamicStructuralEncoder,
{
    context: SignatureContext,
    field: DynField,
    encoder: E,
    base: DynElement,
    base_inverse: DynElement,
    state: DynElement,
    length: u64,
}

impl<E> PartialEq for DynamicSequenceSignature<E>
where
    E: DynamicStructuralEncoder,
{
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.base == other.base
            && self.state == other.state
            && self.length == other.length
    }
}

impl<E> Eq for DynamicSequenceSignature<E> where E: DynamicStructuralEncoder {}

impl<E> DynamicSequenceSignature<E>
where
    E: DynamicStructuralEncoder,
{
    /// Creates an empty sequence with a checked non-zero, non-one base.
    ///
    /// # Errors
    ///
    /// Rejects a mixed-field or degenerate base.
    pub fn new(field: DynField, encoder: E, base: DynElement) -> Result<Self, SignatureError> {
        check_element(&field, &base)?;
        if base == field.zero() || base == field.one() {
            return Err(SignatureError::DegenerateSequenceBase);
        }
        let base_inverse = field.invert(&base)?;
        let parameters = encode_element(&field, &base)?;
        let context = SignatureContext::for_field_id(
            field.field_id(),
            encoder.encoder_id(),
            SignatureLaw::Sequence,
            &parameters,
        );
        let state = field.zero();
        Ok(Self {
            context,
            field,
            encoder,
            base,
            base_inverse,
            state,
            length: 0,
        })
    }

    /// Appends one encoded item atomically.
    ///
    /// # Errors
    ///
    /// Rejects encoding, field and counter failures without changing state.
    pub fn push(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let length = checked_increment(self.length)?;
        let element = self.encoder.encode_dynamic(&self.field, data)?;
        let product = self.field.mul(&self.state, &self.base)?;
        let state = self.field.add(&product, &element)?;
        self.state = state;
        self.length = length;
        Ok(())
    }

    /// Appends a batch transactionally.
    ///
    /// # Errors
    ///
    /// Rejects the first failure without publishing a partial evaluation.
    pub fn push_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut state = self.state.clone();
        let mut length = self.length;
        for item in items {
            length = checked_increment(length)?;
            let element = self.encoder.encode_dynamic(&self.field, item.as_ref())?;
            let product = self.field.mul(&state, &self.base)?;
            state = self.field.add(&product, &element)?;
        }
        self.state = state;
        self.length = length;
        Ok(())
    }

    /// Concatenates sequences using `H(A||B)=H(A)·b^len(B)+H(B)`.
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
        let power = pow_u64(&self.field, &self.base, suffix.length)?;
        let scaled = self.field.mul(&self.state, &power)?;
        let state = self.field.add(&scaled, &suffix.state)?;
        Ok(Self {
            state,
            length,
            ..self.clone()
        })
    }

    /// Derives a predecessor equation for an assumed last item.
    ///
    /// # Errors
    ///
    /// Rejects empty state, encoder or runtime arithmetic failures.
    pub fn residual_assuming_last(
        &self,
        data: &[u8],
    ) -> Result<DynamicAlgebraicResidual, SignatureError> {
        let item_count = self
            .length
            .checked_sub(1)
            .ok_or(SignatureError::EmptyState)?;
        let element = self.encoder.encode_dynamic(&self.field, data)?;
        let difference = self.field.sub(&self.state, &element)?;
        let state = self.field.mul(&difference, &self.base_inverse)?;
        Ok(DynamicAlgebraicResidual {
            signature_id: self.context.signature_id(),
            law: SignatureLaw::Sequence,
            state,
            item_count,
            zero_factor_count: 0,
        })
    }

    /// Checks only the runtime Horner equation in a residual.
    ///
    /// # Errors
    ///
    /// Rejects identity, encoder or runtime field failures.
    pub fn verify_residual(
        &self,
        data: &[u8],
        residual: &DynamicAlgebraicResidual,
    ) -> Result<bool, SignatureError> {
        require_residual(self.context, SignatureLaw::Sequence, residual)?;
        let Some(length) = residual.item_count.checked_add(1) else {
            return Ok(false);
        };
        let element = self.encoder.encode_dynamic(&self.field, data)?;
        let product = self.field.mul(&residual.state, &self.base)?;
        let state = self.field.add(&product, &element)?;
        Ok(length == self.length && state == self.state)
    }

    /// Complete field/encoder/law identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Borrows the ordered evaluation.
    #[must_use]
    pub const fn state(&self) -> &DynElement {
        &self.state
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

    /// Borrows the positional base.
    #[must_use]
    pub const fn base(&self) -> &DynElement {
        &self.base
    }

    /// Serializes the runtime sequence in `MFSG` schema 1.
    ///
    /// # Errors
    ///
    /// Rejects an internally inconsistent runtime element.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        let state = encode_element(&self.field, &self.state)?;
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 8 + state.len());
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.length.to_le_bytes());
        bytes.extend_from_slice(&state);
        Ok(bytes)
    }

    /// Restores a runtime sequence under the supplied field, encoder and base.
    ///
    /// # Errors
    ///
    /// Rejects malformed, non-canonical or incompatible bytes.
    pub fn from_canonical_bytes(
        field: DynField,
        encoder: E,
        base: DynElement,
        bytes: &[u8],
    ) -> Result<Self, SignatureError> {
        let empty = Self::new(field, encoder, base)?;
        verify_header(bytes, empty.context)?;
        let expected = HEADER_BYTES + 8 + empty.field.canonical_bytes();
        if bytes.len() != expected {
            return Err(SignatureError::InvalidWireFormat("dynamic sequence length"));
        }
        let length = read_u64(bytes, HEADER_BYTES);
        let state = empty.field.decode(&bytes[HEADER_BYTES + 8..])?;
        if length == 0 && state != empty.field.zero() {
            return Err(SignatureError::InvalidWireFormat(
                "non-zero dynamic sequence state with zero length",
            ));
        }
        Ok(Self {
            state,
            length,
            ..empty
        })
    }
}

impl DynamicSequenceSignature<CanonicalElementEncoder> {
    /// Appends an element after checking its runtime `FieldId`.
    ///
    /// # Errors
    ///
    /// Rejects mixed fields and counter overflow atomically.
    pub fn push_element(&mut self, element: &DynElement) -> Result<(), SignatureError> {
        let length = checked_increment(self.length)?;
        let product = self.field.mul(&self.state, &self.base)?;
        let state = self.field.add(&product, element)?;
        self.state = state;
        self.length = length;
        Ok(())
    }
}

/// Commutative product signature over one validated runtime field.
#[derive(Clone, Debug)]
pub struct DynamicMultisetSignature<E>
where
    E: DynamicStructuralEncoder,
{
    context: SignatureContext,
    field: DynField,
    encoder: E,
    offset: DynElement,
    nonzero_product: DynElement,
    cardinality: u64,
    zero_factor_count: u64,
}

impl<E> PartialEq for DynamicMultisetSignature<E>
where
    E: DynamicStructuralEncoder,
{
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.offset == other.offset
            && self.nonzero_product == other.nonzero_product
            && self.cardinality == other.cardinality
            && self.zero_factor_count == other.zero_factor_count
    }
}

impl<E> Eq for DynamicMultisetSignature<E> where E: DynamicStructuralEncoder {}

impl<E> DynamicMultisetSignature<E>
where
    E: DynamicStructuralEncoder,
{
    /// Creates an empty runtime multiset with a checked affine offset.
    ///
    /// # Errors
    ///
    /// Rejects an offset from another field.
    pub fn new(field: DynField, encoder: E, offset: DynElement) -> Result<Self, SignatureError> {
        check_element(&field, &offset)?;
        let parameters = encode_element(&field, &offset)?;
        let context = SignatureContext::for_field_id(
            field.field_id(),
            encoder.encoder_id(),
            SignatureLaw::Multiset,
            &parameters,
        );
        let nonzero_product = field.one();
        Ok(Self {
            context,
            field,
            encoder,
            offset,
            nonzero_product,
            cardinality: 0,
            zero_factor_count: 0,
        })
    }

    /// Encodes and inserts one factor atomically.
    ///
    /// # Errors
    ///
    /// Rejects encoding, field and counter failures without changing state.
    pub fn insert(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let cardinality = checked_increment(self.cardinality)?;
        let element = self.encoder.encode_dynamic(&self.field, data)?;
        let factor = self.field.add(&element, &self.offset)?;
        let (nonzero_product, zero_factor_count) = accumulate_factor(
            &self.field,
            &self.nonzero_product,
            self.zero_factor_count,
            &factor,
        )?;
        self.nonzero_product = nonzero_product;
        self.zero_factor_count = zero_factor_count;
        self.cardinality = cardinality;
        Ok(())
    }

    /// Inserts a batch transactionally.
    ///
    /// # Errors
    ///
    /// Rejects the first failure without publishing partial products.
    pub fn insert_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut nonzero_product = self.nonzero_product.clone();
        let mut cardinality = self.cardinality;
        let mut zero_factor_count = self.zero_factor_count;
        for item in items {
            cardinality = checked_increment(cardinality)?;
            let element = self.encoder.encode_dynamic(&self.field, item.as_ref())?;
            let factor = self.field.add(&element, &self.offset)?;
            (nonzero_product, zero_factor_count) =
                accumulate_factor(&self.field, &nonzero_product, zero_factor_count, &factor)?;
        }
        self.nonzero_product = nonzero_product;
        self.cardinality = cardinality;
        self.zero_factor_count = zero_factor_count;
        Ok(())
    }

    /// Combines independently accumulated runtime partitions.
    ///
    /// # Errors
    ///
    /// Rejects identity drift, overflow or runtime arithmetic failure.
    pub fn combine(&self, other: &Self) -> Result<Self, SignatureError> {
        require_context(self.context, other.context)?;
        let cardinality = self
            .cardinality
            .checked_add(other.cardinality)
            .ok_or(SignatureError::CounterOverflow)?;
        let zero_factor_count = self
            .zero_factor_count
            .checked_add(other.zero_factor_count)
            .ok_or(SignatureError::CounterOverflow)?;
        let nonzero_product = self
            .field
            .mul(&self.nonzero_product, &other.nonzero_product)?;
        Ok(Self {
            nonzero_product,
            cardinality,
            zero_factor_count,
            ..self.clone()
        })
    }

    /// Derives a quotient equation for an assumed member.
    ///
    /// # Errors
    ///
    /// Rejects empty state, absent zero, encoder or runtime failures.
    pub fn residual_assuming_member(
        &self,
        data: &[u8],
    ) -> Result<DynamicAlgebraicResidual, SignatureError> {
        let item_count = self
            .cardinality
            .checked_sub(1)
            .ok_or(SignatureError::EmptyState)?;
        let element = self.encoder.encode_dynamic(&self.field, data)?;
        let factor = self.field.add(&element, &self.offset)?;
        let (state, zero_factor_count) = if factor == self.field.zero() {
            (
                self.nonzero_product.clone(),
                self.zero_factor_count
                    .checked_sub(1)
                    .ok_or(SignatureError::ZeroFactorAbsent)?,
            )
        } else {
            let inverse = self.field.invert(&factor)?;
            (
                self.field.mul(&self.nonzero_product, &inverse)?,
                self.zero_factor_count,
            )
        };
        Ok(DynamicAlgebraicResidual {
            signature_id: self.context.signature_id(),
            law: SignatureLaw::Multiset,
            state,
            item_count,
            zero_factor_count,
        })
    }

    /// Checks only the runtime product equation in a residual.
    ///
    /// # Errors
    ///
    /// Rejects identity, encoder or runtime field failures.
    pub fn verify_residual(
        &self,
        data: &[u8],
        residual: &DynamicAlgebraicResidual,
    ) -> Result<bool, SignatureError> {
        require_residual(self.context, SignatureLaw::Multiset, residual)?;
        let Some(cardinality) = residual.item_count.checked_add(1) else {
            return Ok(false);
        };
        let element = self.encoder.encode_dynamic(&self.field, data)?;
        let factor = self.field.add(&element, &self.offset)?;
        let (product, zero_factor_count) = if factor == self.field.zero() {
            let Some(zeros) = residual.zero_factor_count.checked_add(1) else {
                return Ok(false);
            };
            (residual.state.clone(), zeros)
        } else {
            (
                self.field.mul(&residual.state, &factor)?,
                residual.zero_factor_count,
            )
        };
        Ok(cardinality == self.cardinality
            && zero_factor_count == self.zero_factor_count
            && product == self.nonzero_product)
    }

    /// Complete field/encoder/law identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Product of non-zero factors.
    #[must_use]
    pub const fn nonzero_product(&self) -> &DynElement {
        &self.nonzero_product
    }

    /// Product evaluation, equal to zero when any factor was zero.
    #[must_use]
    pub fn evaluated_product(&self) -> DynElement {
        if self.zero_factor_count == 0 {
            self.nonzero_product.clone()
        } else {
            self.field.zero()
        }
    }

    /// Exact logical cardinality.
    #[must_use]
    pub const fn cardinality(&self) -> u64 {
        self.cardinality
    }

    /// Exact number of zero factors.
    #[must_use]
    pub const fn zero_factor_count(&self) -> u64 {
        self.zero_factor_count
    }

    /// Borrows the affine offset.
    #[must_use]
    pub const fn offset(&self) -> &DynElement {
        &self.offset
    }

    /// Serializes the runtime multiset in `MFSG` schema 1.
    ///
    /// # Errors
    ///
    /// Rejects an internally inconsistent runtime element.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        let product = encode_element(&self.field, &self.nonzero_product)?;
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 16 + product.len());
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.cardinality.to_le_bytes());
        bytes.extend_from_slice(&self.zero_factor_count.to_le_bytes());
        bytes.extend_from_slice(&product);
        Ok(bytes)
    }

    /// Restores a runtime multiset under the supplied context and offset.
    ///
    /// # Errors
    ///
    /// Rejects malformed, inconsistent, non-canonical or incompatible bytes.
    pub fn from_canonical_bytes(
        field: DynField,
        encoder: E,
        offset: DynElement,
        bytes: &[u8],
    ) -> Result<Self, SignatureError> {
        let empty = Self::new(field, encoder, offset)?;
        verify_header(bytes, empty.context)?;
        let expected = HEADER_BYTES + 16 + empty.field.canonical_bytes();
        if bytes.len() != expected {
            return Err(SignatureError::InvalidWireFormat("dynamic multiset length"));
        }
        let cardinality = read_u64(bytes, HEADER_BYTES);
        let zero_factor_count = read_u64(bytes, HEADER_BYTES + 8);
        if zero_factor_count > cardinality {
            return Err(SignatureError::InvalidWireFormat(
                "dynamic zero count exceeds cardinality",
            ));
        }
        let nonzero_product = empty.field.decode(&bytes[HEADER_BYTES + 16..])?;
        if nonzero_product == empty.field.zero() {
            return Err(SignatureError::InvalidWireFormat(
                "dynamic non-zero product cannot be zero",
            ));
        }
        if (cardinality == 0 || zero_factor_count == cardinality)
            && nonzero_product != empty.field.one()
        {
            return Err(SignatureError::InvalidWireFormat(
                "dynamic empty non-zero product must be one",
            ));
        }
        Ok(Self {
            nonzero_product,
            cardinality,
            zero_factor_count,
            ..empty
        })
    }
}

impl DynamicMultisetSignature<CanonicalElementEncoder> {
    /// Inserts an element after checking its runtime `FieldId`.
    ///
    /// # Errors
    ///
    /// Rejects mixed fields and counter overflow atomically.
    pub fn insert_element(&mut self, element: &DynElement) -> Result<(), SignatureError> {
        let cardinality = checked_increment(self.cardinality)?;
        let factor = self.field.add(element, &self.offset)?;
        let (nonzero_product, zero_factor_count) = accumulate_factor(
            &self.field,
            &self.nonzero_product,
            self.zero_factor_count,
            &factor,
        )?;
        self.nonzero_product = nonzero_product;
        self.zero_factor_count = zero_factor_count;
        self.cardinality = cardinality;
        Ok(())
    }
}

pub(super) fn checked_increment(value: u64) -> Result<u64, SignatureError> {
    value.checked_add(1).ok_or(SignatureError::CounterOverflow)
}

pub(super) fn require_context(
    expected: SignatureContext,
    actual: SignatureContext,
) -> Result<(), SignatureError> {
    if expected == actual {
        Ok(())
    } else {
        Err(SignatureError::IdentityMismatch)
    }
}

fn require_residual(
    context: SignatureContext,
    law: SignatureLaw,
    residual: &DynamicAlgebraicResidual,
) -> Result<(), SignatureError> {
    if residual.signature_id == context.signature_id() && residual.law == law {
        Ok(())
    } else {
        Err(SignatureError::IdentityMismatch)
    }
}

pub(super) fn check_element(field: &DynField, element: &DynElement) -> Result<(), SignatureError> {
    let _ = field.add(element, &field.zero())?;
    Ok(())
}

pub(super) fn encode_element(
    field: &DynField,
    element: &DynElement,
) -> Result<Vec<u8>, SignatureError> {
    let mut bytes = vec![0_u8; field.canonical_bytes()];
    field.encode(element, &mut bytes)?;
    Ok(bytes)
}

pub(super) fn read_u64(bytes: &[u8], start: usize) -> u64 {
    u64::from_le_bytes(
        bytes[start..start + 8]
            .try_into()
            .expect("validated wire counter range"),
    )
}

pub(super) fn pow_u64(
    field: &DynField,
    base: &DynElement,
    mut exponent: u64,
) -> Result<DynElement, SignatureError> {
    let mut result = field.one();
    let mut power = base.clone();
    while exponent != 0 {
        if exponent & 1 == 1 {
            result = field.mul(&result, &power)?;
        }
        exponent >>= 1;
        if exponent != 0 {
            power = field.square(&power)?;
        }
    }
    Ok(result)
}

fn accumulate_factor(
    field: &DynField,
    product: &DynElement,
    zero_count: u64,
    factor: &DynElement,
) -> Result<(DynElement, u64), SignatureError> {
    if *factor == field.zero() {
        Ok((product.clone(), checked_increment(zero_count)?))
    } else {
        Ok((field.mul(product, factor)?, zero_count))
    }
}
