//! Homogeneous dynamic batches and their validation-once façade.

use num_bigint::BigUint;
use num_traits::{One as _, Zero as _};

use crate::FieldId;

use super::{DynBatchError, DynElement, DynField, DynLimbStorage};

/// Optimization level selected for a dynamic context.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SpecializationLevel {
    /// Shape-generic portable algorithms.
    GenericPortable,
    /// A safe parametrized ISA adapter.
    GenericIsa,
    /// A separately generated nominal Rust type.
    GeneratedStatic,
}

/// A homogeneous buffer whose element identities were checked at construction.
#[derive(Clone, Debug)]
pub struct DynBatch {
    field: DynField,
    storage: Vec<DynLimbStorage>,
}

impl DynBatch {
    /// Copies elements into homogeneous storage after one validation pass.
    ///
    /// # Errors
    ///
    /// Reports the first element carrying a different `FieldId` or storage
    /// shape. No partially built batch is returned.
    pub fn from_elements(field: &DynField, values: &[DynElement]) -> Result<Self, DynBatchError> {
        let mut storage = Vec::with_capacity(values.len());
        for (index, value) in values.iter().enumerate() {
            if value.field_id() != field.field_id() || value.storage().limb_count() != field.limbs()
            {
                return Err(DynBatchError::ElementFieldMismatch { index });
            }
            storage.push(value.storage().clone());
        }
        Ok(Self {
            field: field.clone(),
            storage,
        })
    }

    /// Decodes strided canonical records into one homogeneous batch.
    ///
    /// Bytes after the canonical width in each stride are ignored. A zero
    /// stride is accepted only for an empty byte slice.
    ///
    /// # Errors
    ///
    /// Rejects invalid stride/length pairs and non-canonical records.
    pub fn decode_many(
        field: &DynField,
        bytes: &[u8],
        stride: usize,
    ) -> Result<Self, DynBatchError> {
        if bytes.is_empty() {
            return Ok(Self::zeroed(field, 0));
        }
        if stride < field.canonical_bytes() || !bytes.len().is_multiple_of(stride) {
            return Err(DynBatchError::LengthMismatch {
                output: bytes.len(),
                lhs: stride,
                rhs: Some(field.canonical_bytes()),
            });
        }
        let mut values = Vec::with_capacity(bytes.len() / stride);
        for record in bytes.chunks_exact(stride) {
            values.push(field.decode(&record[..field.canonical_bytes()])?);
        }
        Self::from_elements(field, &values)
    }

    /// Creates a zero-filled batch for use as an explicit output buffer.
    #[must_use]
    pub fn zeroed(field: &DynField, len: usize) -> Self {
        let zero = field.storage_from_value(&BigUint::zero());
        Self {
            field: field.clone(),
            storage: vec![zero; len],
        }
    }

    /// Returns the homogeneous field identity.
    #[must_use]
    pub fn field_id(&self) -> FieldId {
        self.field.field_id()
    }

    /// Returns the logical number of elements.
    #[must_use]
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    /// Reports whether this batch has no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Materializes one nominal element for scalar inspection.
    #[must_use]
    pub fn element(&self, index: usize) -> Option<DynElement> {
        self.storage
            .get(index)
            .cloned()
            .map(|storage| self.field.element_from_storage(storage))
    }
}

/// Reusable dynamic engine that validates context and lengths once per batch.
#[derive(Clone, Debug)]
pub struct DynEngine {
    field: DynField,
}

impl DynEngine {
    /// Creates the safe generic portable strategy for one context.
    #[must_use]
    pub fn new(field: &DynField) -> Self {
        Self {
            field: field.clone(),
        }
    }

    /// Returns the context identity.
    #[must_use]
    pub fn field_id(&self) -> FieldId {
        self.field.field_id()
    }

    /// Returns the truthful current specialization level.
    #[must_use]
    pub const fn specialization_level(&self) -> SpecializationLevel {
        SpecializationLevel::GenericPortable
    }

    /// Adds two homogeneous batches atomically.
    ///
    /// # Errors
    ///
    /// Rejects field or length mismatches without changing `out`.
    pub fn add_into(
        &self,
        out: &mut DynBatch,
        lhs: &DynBatch,
        rhs: &DynBatch,
    ) -> Result<(), DynBatchError> {
        self.binary_into(out, lhs, rhs, |field, left, right| {
            field.add_values(left, right)
        })
    }

    /// Multiplies two homogeneous batches atomically.
    ///
    /// # Errors
    ///
    /// Rejects field or length mismatches without changing `out`.
    pub fn mul_into(
        &self,
        out: &mut DynBatch,
        lhs: &DynBatch,
        rhs: &DynBatch,
    ) -> Result<(), DynBatchError> {
        self.binary_into(out, lhs, rhs, |field, left, right| {
            field.mul_values(left, right)
        })
    }

    /// Squares one homogeneous batch atomically.
    ///
    /// # Errors
    ///
    /// Rejects field or length mismatches without changing `out`.
    pub fn square_into(&self, out: &mut DynBatch, values: &DynBatch) -> Result<(), DynBatchError> {
        self.validate_unary(out, values)?;
        for (target, value) in out.storage.iter_mut().zip(&values.storage) {
            let squared = self.field.square_value(&value.to_biguint());
            target.assign_biguint(&squared);
        }
        Ok(())
    }

    /// Inverts every non-zero value using one field inversion and prefix/suffix
    /// products. Output remains unchanged if zero is encountered.
    ///
    /// # Errors
    ///
    /// Rejects field/length mismatches and any batch containing zero.
    pub fn invert_batch_into(
        &self,
        out: &mut DynBatch,
        values: &DynBatch,
    ) -> Result<(), DynBatchError> {
        self.validate_unary(out, values)?;
        if values.is_empty() {
            return Ok(());
        }
        let source = values
            .storage
            .iter()
            .map(DynLimbStorage::to_biguint)
            .collect::<Vec<_>>();
        let mut prefixes = Vec::with_capacity(source.len());
        let mut accumulator = BigUint::one();
        for value in &source {
            prefixes.push(accumulator.clone());
            accumulator = self.field.mul_values(&accumulator, value);
        }
        let mut inverse = self.field.invert_value(&accumulator)?;
        let mut result = vec![BigUint::zero(); source.len()];
        for index in (0..source.len()).rev() {
            result[index] = self.field.mul_values(&inverse, &prefixes[index]);
            inverse = self.field.mul_values(&inverse, &source[index]);
        }
        for (target, value) in out.storage.iter_mut().zip(result) {
            target.assign_biguint(&value);
        }
        Ok(())
    }

    fn binary_into(
        &self,
        out: &mut DynBatch,
        lhs: &DynBatch,
        rhs: &DynBatch,
        operation: impl Fn(&DynField, &BigUint, &BigUint) -> BigUint,
    ) -> Result<(), DynBatchError> {
        self.validate_binary(out, lhs, rhs)?;
        for ((target, left), right) in out.storage.iter_mut().zip(&lhs.storage).zip(&rhs.storage) {
            let value = operation(&self.field, &left.to_biguint(), &right.to_biguint());
            target.assign_biguint(&value);
        }
        Ok(())
    }

    fn validate_binary(
        &self,
        out: &DynBatch,
        lhs: &DynBatch,
        rhs: &DynBatch,
    ) -> Result<(), DynBatchError> {
        if !self.field.same_field(&out.field)
            || !self.field.same_field(&lhs.field)
            || !self.field.same_field(&rhs.field)
        {
            return Err(DynBatchError::FieldMismatch);
        }
        if out.len() != lhs.len() || lhs.len() != rhs.len() {
            return Err(DynBatchError::LengthMismatch {
                output: out.len(),
                lhs: lhs.len(),
                rhs: Some(rhs.len()),
            });
        }
        Ok(())
    }

    fn validate_unary(&self, out: &DynBatch, values: &DynBatch) -> Result<(), DynBatchError> {
        if !self.field.same_field(&out.field) || !self.field.same_field(&values.field) {
            return Err(DynBatchError::FieldMismatch);
        }
        if out.len() != values.len() {
            return Err(DynBatchError::LengthMismatch {
                output: out.len(),
                lhs: values.len(),
                rhs: None,
            });
        }
        Ok(())
    }
}

impl DynField {
    /// Creates a reusable validation-once batch façade.
    #[must_use]
    pub fn engine(&self) -> DynEngine {
        DynEngine::new(self)
    }
}
