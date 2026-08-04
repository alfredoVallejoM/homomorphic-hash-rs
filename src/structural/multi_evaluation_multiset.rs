//! Commutative multiset signatures evaluated at several field points.

use microfield::{CanonicalEncoding, Field, StaticField};

use super::{
    wire::{encode_header, verify_header, HEADER_BYTES},
    CanonicalElementEncoder, SignatureAssurance, SignatureContext, SignatureError, SignatureLaw,
    StructuralEncoder,
};

/// Product signature `P(tᵢ)=∏(encode(x)+tᵢ)` at `K` distinct offsets.
///
/// Multiple evaluations reduce accidental algebraic collisions compared with
/// a single product at the cost of `K` multiplications and `K` field elements
/// per state. They do not make this a cryptographic commitment or a membership
/// proof. Every coordinate retains its own zero-factor count so partition
/// composition remains total.
#[derive(Clone, Debug)]
pub struct MultiEvaluationMultisetSignature<F, E, const K: usize>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    context: SignatureContext,
    encoder: E,
    offsets: [F; K],
    nonzero_products: [F; K],
    zero_factor_counts: [u64; K],
    cardinality: u64,
}

impl<F, E, const K: usize> PartialEq for MultiEvaluationMultisetSignature<F, E, K>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.offsets == other.offsets
            && self.nonzero_products == other.nonzero_products
            && self.zero_factor_counts == other.zero_factor_counts
            && self.cardinality == other.cardinality
    }
}

impl<F, E, const K: usize> Eq for MultiEvaluationMultisetSignature<F, E, K>
where
    F: Field,
    E: StructuralEncoder<F>,
{
}

impl<F, const K: usize> MultiEvaluationMultisetSignature<F, CanonicalElementEncoder, K>
where
    F: Field + CanonicalEncoding + StaticField,
{
    /// Inserts one already validated field element.
    ///
    /// # Errors
    ///
    /// Rejects counter overflow without publishing partial coordinates.
    pub fn insert_element(&mut self, element: F) -> Result<(), SignatureError> {
        let cardinality = checked_increment(self.cardinality)?;
        let (products, zero_counts) = accumulate(
            self.nonzero_products,
            self.zero_factor_counts,
            self.offsets,
            element,
        )?;
        self.nonzero_products = products;
        self.zero_factor_counts = zero_counts;
        self.cardinality = cardinality;
        Ok(())
    }

    /// Inserts validated field elements as one transactional batch.
    ///
    /// # Errors
    ///
    /// Rejects overflow without publishing any partial coordinate.
    pub fn insert_elements<I>(&mut self, elements: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = F>,
    {
        let mut products = self.nonzero_products;
        let mut zero_counts = self.zero_factor_counts;
        let mut cardinality = self.cardinality;
        for element in elements {
            cardinality = checked_increment(cardinality)?;
            (products, zero_counts) = accumulate(products, zero_counts, self.offsets, element)?;
        }
        self.nonzero_products = products;
        self.zero_factor_counts = zero_counts;
        self.cardinality = cardinality;
        Ok(())
    }
}

impl<F, E, const K: usize> MultiEvaluationMultisetSignature<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField,
    E: StructuralEncoder<F>,
{
    /// Creates an empty signature at pairwise-distinct evaluation points.
    ///
    /// # Errors
    ///
    /// Rejects `K == 0` and repeated offsets because they provide no new
    /// coordinate information.
    pub fn new(encoder: E, offsets: [F; K]) -> Result<Self, SignatureError> {
        validate_offsets(&offsets)?;
        let mut parameters = Vec::new();
        parameters.extend_from_slice(&(K as u64).to_le_bytes());
        for offset in offsets {
            parameters.extend_from_slice(offset.to_canonical().as_ref());
        }
        let context = SignatureContext::for_field::<F>(
            encoder.encoder_id(),
            SignatureLaw::MultiEvaluationMultiset,
            &parameters,
        );
        Ok(Self {
            context,
            encoder,
            offsets,
            nonzero_products: [F::ONE; K],
            zero_factor_counts: [0; K],
            cardinality: 0,
        })
    }

    /// Encodes and inserts one item atomically across every coordinate.
    ///
    /// # Errors
    ///
    /// Rejects encoding or overflow without changing the state.
    pub fn insert(&mut self, data: &[u8]) -> Result<(), SignatureError> {
        let cardinality = checked_increment(self.cardinality)?;
        let element = self.encoder.encode(data)?;
        let (products, zero_counts) = accumulate(
            self.nonzero_products,
            self.zero_factor_counts,
            self.offsets,
            element,
        )?;
        self.nonzero_products = products;
        self.zero_factor_counts = zero_counts;
        self.cardinality = cardinality;
        Ok(())
    }

    /// Inserts a batch transactionally across every evaluation point.
    ///
    /// # Errors
    ///
    /// Rejects the first encoder or counter failure without partial state.
    pub fn insert_many<I, B>(&mut self, items: I) -> Result<(), SignatureError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut products = self.nonzero_products;
        let mut zero_counts = self.zero_factor_counts;
        let mut cardinality = self.cardinality;
        for item in items {
            cardinality = checked_increment(cardinality)?;
            let element = self.encoder.encode(item.as_ref())?;
            (products, zero_counts) = accumulate(products, zero_counts, self.offsets, element)?;
        }
        self.nonzero_products = products;
        self.zero_factor_counts = zero_counts;
        self.cardinality = cardinality;
        Ok(())
    }

    /// Combines independently accumulated multiset partitions.
    ///
    /// # Errors
    ///
    /// Rejects context drift and any counter overflow.
    pub fn combine(&self, other: &Self) -> Result<Self, SignatureError> {
        if self.context != other.context {
            return Err(SignatureError::IdentityMismatch);
        }
        let cardinality = self
            .cardinality
            .checked_add(other.cardinality)
            .ok_or(SignatureError::CounterOverflow)?;
        let mut products = self.nonzero_products;
        let mut zero_counts = self.zero_factor_counts;
        for index in 0..K {
            products[index] = products[index].mul(other.nonzero_products[index]);
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

    /// Pairwise-distinct affine evaluation points.
    #[must_use]
    pub const fn offsets(&self) -> &[F; K] {
        &self.offsets
    }

    /// Products excluding zero factors, one per evaluation point.
    #[must_use]
    pub const fn nonzero_products(&self) -> &[F; K] {
        &self.nonzero_products
    }

    /// Exact zero-factor counts, one per evaluation point.
    #[must_use]
    pub const fn zero_factor_counts(&self) -> &[u64; K] {
        &self.zero_factor_counts
    }

    /// Evaluated products, with zero restored in affected coordinates.
    #[must_use]
    pub fn evaluated_products(&self) -> [F; K] {
        core::array::from_fn(|index| {
            if self.zero_factor_counts[index] == 0 {
                self.nonzero_products[index]
            } else {
                F::ZERO
            }
        })
    }

    /// Exact logical cardinality before field collisions.
    #[must_use]
    pub const fn cardinality(&self) -> u64 {
        self.cardinality
    }

    /// Exactness bound over already encoded field elements.
    ///
    /// Equality is conclusive only when both signatures have the same
    /// cardinality and that cardinality is at most `K`.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::BoundedExactOverEncodedElements {
            maximum_cardinality: K,
        }
    }

    /// Serializes all coordinates in a self-identifying envelope.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 8 + K * (8 + repr_len));
        encode_header(&mut bytes, self.context);
        bytes.extend_from_slice(&self.cardinality.to_le_bytes());
        for index in 0..K {
            bytes.extend_from_slice(&self.zero_factor_counts[index].to_le_bytes());
            bytes.extend_from_slice(self.nonzero_products[index].to_canonical().as_ref());
        }
        bytes
    }

    /// Restores every coordinate under the supplied points and encoder.
    ///
    /// # Errors
    ///
    /// Rejects malformed, impossible, non-canonical or incompatible state.
    pub fn from_canonical_bytes(
        encoder: E,
        offsets: [F; K],
        bytes: &[u8],
    ) -> Result<Self, SignatureError> {
        let empty = Self::new(encoder, offsets)?;
        verify_header(bytes, empty.context)?;
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        let stride = 8 + repr_len;
        if bytes.len() != HEADER_BYTES + 8 + K * stride {
            return Err(SignatureError::InvalidWireFormat(
                "multi-evaluation multiset length",
            ));
        }
        let cardinality = read_u64(bytes, HEADER_BYTES);
        let mut products = [F::ONE; K];
        let mut zero_counts = [0_u64; K];
        for index in 0..K {
            let cursor = HEADER_BYTES + 8 + index * stride;
            let zero_count = read_u64(bytes, cursor);
            if zero_count > cardinality {
                return Err(SignatureError::InvalidWireFormat(
                    "zero count exceeds cardinality",
                ));
            }
            let product = F::from_canonical_slice(&bytes[cursor + 8..cursor + stride])
                .map_err(|_| SignatureError::NonCanonicalElement)?;
            if product.is_zero() {
                return Err(SignatureError::InvalidWireFormat(
                    "non-zero coordinate product cannot be zero",
                ));
            }
            if (cardinality == 0 || zero_count == cardinality) && product != F::ONE {
                return Err(SignatureError::InvalidWireFormat(
                    "empty coordinate product must be one",
                ));
            }
            products[index] = product;
            zero_counts[index] = zero_count;
        }
        Ok(Self {
            nonzero_products: products,
            zero_factor_counts: zero_counts,
            cardinality,
            ..empty
        })
    }
}

fn validate_offsets<F: Field, const K: usize>(offsets: &[F; K]) -> Result<(), SignatureError> {
    if K == 0 {
        return Err(SignatureError::InvalidEvaluationPoints);
    }
    for left in 0..K {
        for right in left + 1..K {
            if offsets[left] == offsets[right] {
                return Err(SignatureError::InvalidEvaluationPoints);
            }
        }
    }
    Ok(())
}

fn accumulate<F: Field, const K: usize>(
    mut products: [F; K],
    mut zero_counts: [u64; K],
    offsets: [F; K],
    element: F,
) -> Result<([F; K], [u64; K]), SignatureError> {
    for index in 0..K {
        let factor = element.add(offsets[index]);
        if factor.is_zero() {
            zero_counts[index] = checked_increment(zero_counts[index])?;
        } else {
            products[index] = products[index].mul(factor);
        }
    }
    Ok((products, zero_counts))
}

fn checked_increment(value: u64) -> Result<u64, SignatureError> {
    value.checked_add(1).ok_or(SignatureError::CounterOverflow)
}

fn read_u64(bytes: &[u8], start: usize) -> u64 {
    u64::from_le_bytes(
        bytes[start..start + 8]
            .try_into()
            .expect("validated counter range"),
    )
}
