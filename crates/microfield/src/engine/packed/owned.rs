//! Owned packed batches backed by audited aligned storage.

use crate::{__private::PortableField, StaticField};

use super::{PackError, PackedBatchView, PackedBatchViewMut, PackingPlan, storage::AlignedBuffer};

/// Persistent owned packed batch.
///
/// Construction and `AoS` conversion allocate, while repeated packed operations
/// do not allocate or change layout.
pub struct PackedBatch<F: PortableField + StaticField> {
    storage: AlignedBuffer<F>,
    plan: PackingPlan,
}

impl<F: PortableField + StaticField> PackedBatch<F> {
    /// Allocates an initialized zero batch for `engine` and `len`.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] when size planning overflows, backend metadata is
    /// invalid or allocation fails.
    pub fn new(engine: &crate::Engine<F>, len: usize) -> Result<Self, PackError> {
        let plan = engine.packing_plan(len)?;
        let storage = AlignedBuffer::new(plan.padded_len(), plan.alignment(), F::ZERO)?;
        Ok(Self { storage, plan })
    }

    /// Allocates and packs a normal `AoS` slice.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] when planning or allocation fails.
    pub fn from_aos(engine: &crate::Engine<F>, values: &[F]) -> Result<Self, PackError> {
        let mut packed = Self::new(engine, values.len())?;
        packed.pack_from(values)?;
        Ok(packed)
    }

    /// Returns the caller-visible element count.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.plan.logical_len()
    }

    /// Reports whether the logical batch is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.plan.logical_len() == 0
    }

    /// Returns the backend for which this batch was packed.
    #[must_use]
    pub const fn backend_id(&self) -> crate::BackendId {
        self.plan.backend_id()
    }

    /// Returns the immutable packing contract.
    #[must_use]
    pub const fn plan(&self) -> &PackingPlan {
        &self.plan
    }

    /// Replaces every logical value and restores zero padding.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::LengthMismatch`] before writing when `values` has
    /// a different logical length.
    pub fn pack_from(&mut self, values: &[F]) -> Result<(), PackError> {
        self.as_view_mut().pack_from(values)
    }

    /// Copies logical values to a normal `AoS` slice.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::LengthMismatch`] before writing when `out` has a
    /// different logical length.
    pub fn unpack_into(&self, out: &mut [F]) -> Result<(), PackError> {
        self.as_view().unpack_into(out)
    }

    /// Borrows the packed allocation immutably without conversion.
    #[must_use]
    pub fn as_view(&self) -> PackedBatchView<'_, F> {
        PackedBatchView {
            values: self.storage.as_slice(),
            plan: self.plan,
        }
    }

    /// Borrows the packed allocation mutably without conversion.
    #[must_use]
    pub fn as_view_mut(&mut self) -> PackedBatchViewMut<'_, F> {
        PackedBatchViewMut {
            values: self.storage.as_mut_slice(),
            plan: self.plan,
        }
    }
}
