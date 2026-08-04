//! Multi-evaluation multiset signatures over validated runtime fields.

use microfield::{DynElement, DynField};

use super::{
    dynamic::{check_element, checked_increment, encode_element, read_u64, require_context},
    wire::{encode_header, verify_header, HEADER_BYTES},
    CanonicalElementEncoder, DynamicStructuralEncoder, SignatureAssurance, SignatureContext,
    SignatureError, SignatureLaw,
};

/// Runtime products `P(tᵢ)=∏(encode(x)+tᵢ)` at distinct offsets.
///
/// This type is intended for fields constructed at runtime. Static generated
/// fields should prefer [`super::MultiEvaluationMultisetSignature`] so the
/// number of coordinates and arithmetic remain monomorphized.
#[derive(Clone, Debug)]
pub struct DynamicMultiEvaluationMultisetSignature<E>
where
    E: DynamicStructuralEncoder,
{
    context: SignatureContext,
    field: DynField,
    encoder: E,
    offsets: Vec<DynElement>,
    nonzero_products: Vec<DynElement>,
    zero_factor_counts: Vec<u64>,
    cardinality: u64,
}

impl<E> PartialEq for DynamicMultiEvaluationMultisetSignature<E>
where
    E: DynamicStructuralEncoder,
{
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.offsets == other.offsets
            && self.nonzero_products == other.nonzero_products
            && self.zero_factor_counts == other.zero_factor_counts
            && self.cardinality == other.cardinality
    }
}

impl<E> Eq for DynamicMultiEvaluationMultisetSignature<E> where E: DynamicStructuralEncoder {}

impl<E> DynamicMultiEvaluationMultisetSignature<E>
where
    E: DynamicStructuralEncoder,
{
    /// Creates an empty signature at distinct runtime evaluation points.
    ///
    /// # Errors
    ///
    /// Rejects empty, duplicate or foreign-field offsets.
    pub fn new(
        field: DynField,
        encoder: E,
        offsets: impl Into<Vec<DynElement>>,
    ) -> Result<Self, SignatureError> {
        let offsets = offsets.into();
        validate_offsets(&field, &offsets)?;
        let count =
            u64::try_from(offsets.len()).map_err(|_| SignatureError::InvalidEvaluationPoints)?;
        let mut parameters = Vec::with_capacity(8 + offsets.len() * field.canonical_bytes());
        parameters.extend_from_slice(&count.to_le_bytes());
        for offset in &offsets {
            parameters.extend_from_slice(&encode_element(&field, offset)?);
        }
        let context = SignatureContext::for_field_id(
            field.field_id(),
            encoder.encoder_id(),
            SignatureLaw::MultiEvaluationMultiset,
            &parameters,
        );
        Ok(Self {
            context,
            encoder,
            nonzero_products: vec![field.one(); offsets.len()],
            zero_factor_counts: vec![0; offsets.len()],
            field,
            offsets,
            cardinality: 0,
        })
    }

    /// Encodes and inserts one item atomically across all coordinates.
    ///
    /// # Errors
    ///
    /// Rejects encoding, field and counter failures without mutation.
    pub fn insert(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let cardinality = checked_increment(self.cardinality)?;
        let element = self.encoder.encode_dynamic(&self.field, data)?;
        let (products, zero_counts) = accumulate(
            &self.field,
            self.nonzero_products.clone(),
            self.zero_factor_counts.clone(),
            &self.offsets,
            &element,
        )?;
        self.nonzero_products = products;
        self.zero_factor_counts = zero_counts;
        self.cardinality = cardinality;
        Ok(())
    }

    /// Inserts a batch and publishes it only after complete success.
    ///
    /// # Errors
    ///
    /// Rejects the first failure without publishing partial coordinates.
    pub fn insert_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut products = self.nonzero_products.clone();
        let mut zero_counts = self.zero_factor_counts.clone();
        let mut cardinality = self.cardinality;
        for item in items {
            cardinality = checked_increment(cardinality)?;
            let element = self.encoder.encode_dynamic(&self.field, item.as_ref())?;
            (products, zero_counts) =
                accumulate(&self.field, products, zero_counts, &self.offsets, &element)?;
        }
        self.nonzero_products = products;
        self.zero_factor_counts = zero_counts;
        self.cardinality = cardinality;
        Ok(())
    }

    /// Combines independently accumulated runtime partitions.
    ///
    /// # Errors
    ///
    /// Rejects identity drift, field failures or counter overflow.
    pub fn combine(&self, other: &Self) -> Result<Self, SignatureError> {
        require_context(self.context, other.context)?;
        let cardinality = self
            .cardinality
            .checked_add(other.cardinality)
            .ok_or(SignatureError::CounterOverflow)?;
        let mut products = self.nonzero_products.clone();
        let mut zero_counts = self.zero_factor_counts.clone();
        for index in 0..products.len() {
            products[index] = self
                .field
                .mul(&products[index], &other.nonzero_products[index])?;
            zero_counts[index] = zero_counts[index]
                .checked_add(other.zero_factor_counts[index])
                .ok_or(SignatureError::CounterOverflow)?;
        }
        Ok(Self {
            nonzero_products: products,
            zero_factor_counts: zero_counts,
            cardinality,
            ..self.clone()
        })
    }

    /// Complete field/encoder/law/evaluation-point identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Borrows all evaluation points.
    #[must_use]
    pub fn offsets(&self) -> &[DynElement] {
        &self.offsets
    }

    /// Borrows products with zero factors excluded.
    #[must_use]
    pub fn nonzero_products(&self) -> &[DynElement] {
        &self.nonzero_products
    }

    /// Borrows exact zero-factor counts per point.
    #[must_use]
    pub fn zero_factor_counts(&self) -> &[u64] {
        &self.zero_factor_counts
    }

    /// Returns products with zero restored in affected coordinates.
    #[must_use]
    pub fn evaluated_products(&self) -> Vec<DynElement> {
        self.nonzero_products
            .iter()
            .zip(&self.zero_factor_counts)
            .map(|(product, zeros)| {
                if *zeros == 0 {
                    product.clone()
                } else {
                    self.field.zero()
                }
            })
            .collect()
    }

    /// Exact logical cardinality before field collisions.
    #[must_use]
    pub const fn cardinality(&self) -> u64 {
        self.cardinality
    }

    /// Exactness bound over already encoded runtime field elements.
    #[must_use]
    pub fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::BoundedExactOverEncodedElements {
            maximum_cardinality: self.offsets.len(),
        }
    }

    /// Serializes every coordinate using the common `MFSG` schema.
    ///
    /// # Errors
    ///
    /// Rejects internally inconsistent runtime elements.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        let stride = 8 + self.field.canonical_bytes();
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 8 + self.offsets.len() * stride);
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.cardinality.to_le_bytes());
        for index in 0..self.offsets.len() {
            bytes.extend_from_slice(&self.zero_factor_counts[index].to_le_bytes());
            bytes.extend_from_slice(&encode_element(&self.field, &self.nonzero_products[index])?);
        }
        Ok(bytes)
    }

    /// Restores every runtime coordinate under explicit offsets.
    ///
    /// # Errors
    ///
    /// Rejects malformed, impossible, non-canonical or incompatible state.
    pub fn from_canonical_bytes(
        field: DynField,
        encoder: E,
        offsets: impl Into<Vec<DynElement>>,
        bytes: &[u8],
    ) -> Result<Self, SignatureError> {
        let empty = Self::new(field, encoder, offsets)?;
        verify_header(bytes, empty.context)?;
        let width = empty.field.canonical_bytes();
        let stride = 8 + width;
        if bytes.len() != HEADER_BYTES + 8 + empty.offsets.len() * stride {
            return Err(SignatureError::InvalidWireFormat(
                "dynamic multi-evaluation multiset length",
            ));
        }
        let cardinality = read_u64(bytes, HEADER_BYTES);
        let mut products = Vec::with_capacity(empty.offsets.len());
        let mut zero_counts = Vec::with_capacity(empty.offsets.len());
        for index in 0..empty.offsets.len() {
            let cursor = HEADER_BYTES + 8 + index * stride;
            let zero_count = read_u64(bytes, cursor);
            if zero_count > cardinality {
                return Err(SignatureError::InvalidWireFormat(
                    "dynamic zero count exceeds cardinality",
                ));
            }
            let product = empty.field.decode(&bytes[cursor + 8..cursor + stride])?;
            if product == empty.field.zero() {
                return Err(SignatureError::InvalidWireFormat(
                    "dynamic non-zero coordinate product cannot be zero",
                ));
            }
            if (cardinality == 0 || zero_count == cardinality) && product != empty.field.one() {
                return Err(SignatureError::InvalidWireFormat(
                    "dynamic empty coordinate product must be one",
                ));
            }
            products.push(product);
            zero_counts.push(zero_count);
        }
        Ok(Self {
            nonzero_products: products,
            zero_factor_counts: zero_counts,
            cardinality,
            ..empty
        })
    }
}

impl DynamicMultiEvaluationMultisetSignature<CanonicalElementEncoder> {
    /// Inserts a runtime element after checking its `FieldId`.
    ///
    /// # Errors
    ///
    /// Rejects a foreign element or overflow without mutation.
    pub fn insert_element(&mut self, element: &DynElement) -> Result<(), SignatureError> {
        let cardinality = checked_increment(self.cardinality)?;
        check_element(&self.field, element)?;
        let (products, zero_counts) = accumulate(
            &self.field,
            self.nonzero_products.clone(),
            self.zero_factor_counts.clone(),
            &self.offsets,
            element,
        )?;
        self.nonzero_products = products;
        self.zero_factor_counts = zero_counts;
        self.cardinality = cardinality;
        Ok(())
    }
}

fn validate_offsets(field: &DynField, offsets: &[DynElement]) -> Result<(), SignatureError> {
    if offsets.is_empty() {
        return Err(SignatureError::InvalidEvaluationPoints);
    }
    for (index, offset) in offsets.iter().enumerate() {
        check_element(field, offset)?;
        if offsets[..index].contains(offset) {
            return Err(SignatureError::InvalidEvaluationPoints);
        }
    }
    Ok(())
}

fn accumulate(
    field: &DynField,
    mut products: Vec<DynElement>,
    mut zero_counts: Vec<u64>,
    offsets: &[DynElement],
    element: &DynElement,
) -> Result<(Vec<DynElement>, Vec<u64>), SignatureError> {
    for index in 0..offsets.len() {
        let factor = field.add(element, &offsets[index])?;
        if factor == field.zero() {
            zero_counts[index] = checked_increment(zero_counts[index])?;
        } else {
            products[index] = field.mul(&products[index], &factor)?;
        }
    }
    Ok((products, zero_counts))
}
