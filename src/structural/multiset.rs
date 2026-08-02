//! Commutative product signatures with explicit zero-factor accounting.

use std::collections::BTreeMap;

use microfield::{CanonicalEncoding, Field, Invert, StaticField};

use super::{
    wire::{encode_header, verify_header, HEADER_BYTES},
    AlgebraicResidual, CanonicalElementEncoder, SignatureContext, SignatureError, SignatureLaw,
    StructuralEncoder,
};

/// Product evaluation of a multiset with exact cardinality and zero count.
///
/// A field product alone collapses permanently after a zero factor. This type
/// instead stores the product of non-zero factors and counts zero factors, so
/// combining partitions and removing a tracked zero remain well-defined.
#[derive(Clone, Debug)]
pub struct MultisetSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    context: SignatureContext,
    encoder: E,
    offset: F,
    nonzero_product: F,
    cardinality: u64,
    zero_factor_count: u64,
}

impl<F> MultisetSignature<F, CanonicalElementEncoder>
where
    F: Field + CanonicalEncoding + StaticField + Invert,
{
    /// Inserts an already validated element without a byte round trip.
    ///
    /// # Errors
    ///
    /// Rejects counter overflow without changing state.
    pub fn insert_element(&mut self, element: F) -> Result<(), SignatureError> {
        let cardinality = self
            .cardinality
            .checked_add(1)
            .ok_or(SignatureError::CounterOverflow)?;
        let factor = element.add(self.offset);
        if factor.is_zero() {
            self.zero_factor_count = self
                .zero_factor_count
                .checked_add(1)
                .ok_or(SignatureError::CounterOverflow)?;
        } else {
            self.nonzero_product = self.nonzero_product.mul(factor);
        }
        self.cardinality = cardinality;
        Ok(())
    }

    /// Inserts validated elements as one transactional batch.
    ///
    /// # Errors
    ///
    /// Rejects counter overflow without publishing partial products.
    pub fn insert_elements<I>(&mut self, elements: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = F>,
    {
        let mut nonzero_product = self.nonzero_product;
        let mut cardinality = self.cardinality;
        let mut zero_factor_count = self.zero_factor_count;
        for element in elements {
            cardinality = cardinality
                .checked_add(1)
                .ok_or(SignatureError::CounterOverflow)?;
            let factor = element.add(self.offset);
            if factor.is_zero() {
                zero_factor_count = zero_factor_count
                    .checked_add(1)
                    .ok_or(SignatureError::CounterOverflow)?;
            } else {
                nonzero_product = nonzero_product.mul(factor);
            }
        }
        self.nonzero_product = nonzero_product;
        self.cardinality = cardinality;
        self.zero_factor_count = zero_factor_count;
        Ok(())
    }
}

impl<F, E> PartialEq for MultisetSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.offset == other.offset
            && self.nonzero_product == other.nonzero_product
            && self.cardinality == other.cardinality
            && self.zero_factor_count == other.zero_factor_count
    }
}

impl<F, E> Eq for MultisetSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, E> MultisetSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Invert,
    E: StructuralEncoder<F>,
{
    /// Creates the empty product with an explicit affine offset.
    #[must_use]
    pub fn new(encoder: E, offset: F) -> Self {
        let parameters = offset.to_canonical();
        let context = SignatureContext::for_field::<F>(
            encoder.encoder_id(),
            SignatureLaw::Multiset,
            parameters.as_ref(),
        );
        Self {
            context,
            encoder,
            offset,
            nonzero_product: F::ONE,
            cardinality: 0,
            zero_factor_count: 0,
        }
    }

    /// Encodes and inserts one factor atomically.
    ///
    /// # Errors
    ///
    /// Rejects encoder failures and either counter overflow before mutation.
    pub fn insert(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let cardinality = self
            .cardinality
            .checked_add(1)
            .ok_or(SignatureError::CounterOverflow)?;
        let factor = self.factor(data)?;
        if factor.is_zero() {
            let zero_factor_count = self
                .zero_factor_count
                .checked_add(1)
                .ok_or(SignatureError::CounterOverflow)?;
            self.zero_factor_count = zero_factor_count;
        } else {
            self.nonzero_product = self.nonzero_product.mul(factor);
        }
        self.cardinality = cardinality;
        Ok(())
    }

    /// Inserts a batch and publishes it only after every factor is accepted.
    ///
    /// # Errors
    ///
    /// Rejects the first encoder failure or counter overflow without changing
    /// the product, cardinality or zero-factor count.
    pub fn insert_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut nonzero_product = self.nonzero_product;
        let mut cardinality = self.cardinality;
        let mut zero_factor_count = self.zero_factor_count;
        for item in items {
            cardinality = cardinality
                .checked_add(1)
                .ok_or(SignatureError::CounterOverflow)?;
            let factor = self.factor(item.as_ref())?;
            if factor.is_zero() {
                zero_factor_count = zero_factor_count
                    .checked_add(1)
                    .ok_or(SignatureError::CounterOverflow)?;
            } else {
                nonzero_product = nonzero_product.mul(factor);
            }
        }
        self.nonzero_product = nonzero_product;
        self.cardinality = cardinality;
        self.zero_factor_count = zero_factor_count;
        Ok(())
    }

    /// Combines independently accumulated multiset partitions.
    ///
    /// # Errors
    ///
    /// Rejects incompatible contexts and counter overflow.
    pub fn combine(&self, other: &Self) -> Result<Self, SignatureError> {
        if self.context != other.context {
            return Err(SignatureError::IdentityMismatch);
        }
        let cardinality = self
            .cardinality
            .checked_add(other.cardinality)
            .ok_or(SignatureError::CounterOverflow)?;
        let zero_factor_count = self
            .zero_factor_count
            .checked_add(other.zero_factor_count)
            .ok_or(SignatureError::CounterOverflow)?;
        Ok(Self {
            nonzero_product: self.nonzero_product.mul(other.nonzero_product),
            cardinality,
            zero_factor_count,
            ..self.clone()
        })
    }

    /// Derives a quotient relation for an assumed member.
    ///
    /// Every non-zero field value has an inverse, so success does not establish
    /// membership. The only rejected factor is zero when no zero was recorded.
    ///
    /// # Errors
    ///
    /// Rejects empty state, encoder failure or an absent zero factor.
    pub fn residual_assuming_member(
        &self,
        data: &[u8],
    ) -> Result<AlgebraicResidual<F>, SignatureError> {
        let cardinality = self
            .cardinality
            .checked_sub(1)
            .ok_or(SignatureError::EmptyState)?;
        let factor = self.factor(data)?;
        let (state, zero_factor_count) = if factor.is_zero() {
            (
                self.nonzero_product,
                self.zero_factor_count
                    .checked_sub(1)
                    .ok_or(SignatureError::ZeroFactorAbsent)?,
            )
        } else {
            (
                self.nonzero_product.mul(
                    factor
                        .invert()
                        .expect("a non-zero field factor is invertible"),
                ),
                self.zero_factor_count,
            )
        };
        Ok(AlgebraicResidual {
            signature_id: self.context.signature_id(),
            law: SignatureLaw::Multiset,
            state,
            item_count: cardinality,
            zero_factor_count,
        })
    }

    /// Checks only the product equation represented by a residual.
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
            || residual.law != SignatureLaw::Multiset
        {
            return Err(SignatureError::IdentityMismatch);
        }
        let Some(cardinality) = residual.item_count.checked_add(1) else {
            return Ok(false);
        };
        let factor = self.factor(data)?;
        let (product, zero_factor_count) = if factor.is_zero() {
            let Some(zeros) = residual.zero_factor_count.checked_add(1) else {
                return Ok(false);
            };
            (residual.state, zeros)
        } else {
            (residual.state.mul(factor), residual.zero_factor_count)
        };
        Ok(cardinality == self.cardinality
            && zero_factor_count == self.zero_factor_count
            && product == self.nonzero_product)
    }

    /// Complete compatibility identity.
    #[must_use]
    pub const fn context(&self) -> SignatureContext {
        self.context
    }

    /// Field evaluation, equal to zero whenever at least one factor is zero.
    #[must_use]
    pub fn evaluated_product(&self) -> F {
        if self.zero_factor_count == 0 {
            self.nonzero_product
        } else {
            F::ZERO
        }
    }

    /// Product of only non-zero factors, retained for reversible accounting.
    #[must_use]
    pub const fn nonzero_product(&self) -> F {
        self.nonzero_product
    }

    /// Exact logical multiplicity before field collisions.
    #[must_use]
    pub const fn cardinality(&self) -> u64 {
        self.cardinality
    }

    /// Number of factors that evaluated to zero.
    #[must_use]
    pub const fn zero_factor_count(&self) -> u64 {
        self.zero_factor_count
    }

    /// Affine offset added to each encoded value.
    #[must_use]
    pub const fn offset(&self) -> F {
        self.offset
    }

    /// Serializes a stable, self-identifying little-endian envelope.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let repr = self.nonzero_product.to_canonical();
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 16 + repr.as_ref().len());
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.cardinality.to_le_bytes());
        bytes.extend_from_slice(&self.zero_factor_count.to_le_bytes());
        bytes.extend_from_slice(repr.as_ref());
        bytes
    }

    /// Parses an envelope under the supplied encoder and offset.
    ///
    /// # Errors
    ///
    /// Rejects malformed, inconsistent, non-canonical or incompatible bytes.
    pub fn from_canonical_bytes(
        encoder: E,
        offset: F,
        bytes: &[u8],
    ) -> Result<Self, SignatureError> {
        let empty = Self::new(encoder, offset);
        verify_header(bytes, empty.context)?;
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        if bytes.len() != HEADER_BYTES + 16 + repr_len {
            return Err(SignatureError::InvalidWireFormat("multiset length"));
        }
        let cardinality = u64::from_le_bytes(
            bytes[HEADER_BYTES..HEADER_BYTES + 8]
                .try_into()
                .expect("counter range"),
        );
        let zero_factor_count = u64::from_le_bytes(
            bytes[HEADER_BYTES + 8..HEADER_BYTES + 16]
                .try_into()
                .expect("counter range"),
        );
        if zero_factor_count > cardinality {
            return Err(SignatureError::InvalidWireFormat(
                "zero count exceeds cardinality",
            ));
        }
        let nonzero_product = F::from_canonical_slice(&bytes[HEADER_BYTES + 16..])
            .map_err(|_| SignatureError::NonCanonicalElement)?;
        if nonzero_product.is_zero() {
            return Err(SignatureError::InvalidWireFormat(
                "non-zero product cannot be zero",
            ));
        }
        if (cardinality == 0 || zero_factor_count == cardinality) && nonzero_product != F::ONE {
            return Err(SignatureError::InvalidWireFormat(
                "empty non-zero factor product must be one",
            ));
        }
        Ok(Self {
            nonzero_product,
            cardinality,
            zero_factor_count,
            ..empty
        })
    }

    fn factor(&self, data: &[u8]) -> Result<F, SignatureError> {
        Ok(self.encoder.encode(data)?.add(self.offset))
    }

    fn apply_residual(&mut self, residual: AlgebraicResidual<F>) {
        self.nonzero_product = residual.state;
        self.cardinality = residual.item_count;
        self.zero_factor_count = residual.zero_factor_count;
    }
}

/// Exact raw-membership adapter around a compact multiset signature.
///
/// The map is intentionally outside the compact algebraic state. It is the
/// source of truth for checked deletion; the field product alone cannot be.
#[derive(Clone, Debug)]
pub struct TrackedMultiset<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    signature: MultisetSignature<F, E>,
    multiplicities: BTreeMap<Vec<u8>, u64>,
}

impl<F, E> PartialEq for TrackedMultiset<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    fn eq(&self, other: &Self) -> bool {
        self.signature == other.signature && self.multiplicities == other.multiplicities
    }
}

impl<F, E> Eq for TrackedMultiset<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, E> TrackedMultiset<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Invert,
    E: StructuralEncoder<F>,
{
    /// Creates an empty tracked collection.
    #[must_use]
    pub fn new(encoder: E, offset: F) -> Self {
        Self {
            signature: MultisetSignature::new(encoder, offset),
            multiplicities: BTreeMap::new(),
        }
    }

    /// Inserts an exact raw item transactionally.
    ///
    /// # Errors
    ///
    /// Rejects encoder or counter failures before publishing either state.
    pub fn insert(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let current = self.multiplicities.get(data).copied().unwrap_or(0);
        let next = current
            .checked_add(1)
            .ok_or(SignatureError::CounterOverflow)?;
        self.signature.insert(data)?;
        self.multiplicities.insert(data.to_vec(), next);
        Ok(())
    }

    /// Removes one occurrence only when exact raw membership is known.
    ///
    /// # Errors
    ///
    /// Rejects absent items without changing either the map or signature.
    pub fn remove(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let current = self
            .multiplicities
            .get(data)
            .copied()
            .ok_or(SignatureError::ItemAbsent)?;
        let residual = self.signature.residual_assuming_member(data)?;
        self.signature.apply_residual(residual);
        if current == 1 {
            self.multiplicities.remove(data);
        } else {
            self.multiplicities.insert(data.to_vec(), current - 1);
        }
        Ok(())
    }

    /// Returns the exact raw multiplicity known to the adapter.
    #[must_use]
    pub fn multiplicity(&self, data: &[u8]) -> u64 {
        self.multiplicities.get(data).copied().unwrap_or(0)
    }

    /// Borrows the compact structural signature.
    #[must_use]
    pub const fn signature(&self) -> &MultisetSignature<F, E> {
        &self.signature
    }
}
