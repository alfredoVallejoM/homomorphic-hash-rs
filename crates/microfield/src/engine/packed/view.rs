//! Allocation-free packed views over caller-provided storage.

use crate::Field;

use super::{PackError, PackingPlan};

/// Immutable packed batch borrowed from caller-provided or owned storage.
pub struct PackedBatchView<'a, F: Field> {
    pub(super) values: &'a [F],
    pub(super) plan: PackingPlan,
}

impl<F: Field> PackedBatchView<'_, F> {
    /// Returns the immutable packing contract.
    #[must_use]
    pub const fn plan(&self) -> &PackingPlan {
        &self.plan
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

    /// Copies logical elements to a normal `AoS` slice.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::LengthMismatch`] before writing when `out` has a
    /// different logical length.
    pub fn unpack_into(&self, out: &mut [F]) -> Result<(), PackError> {
        if out.len() != self.len() {
            return Err(PackError::LengthMismatch {
                expected: self.len(),
                actual: out.len(),
            });
        }
        out.copy_from_slice(&self.values[..self.len()]);
        Ok(())
    }
}

/// Mutable packed batch borrowed exclusively from caller-provided or owned
/// storage.
pub struct PackedBatchViewMut<'a, F: Field> {
    pub(super) values: &'a mut [F],
    pub(super) plan: PackingPlan,
}

impl<F: Field> PackedBatchViewMut<'_, F> {
    /// Returns the immutable packing contract.
    #[must_use]
    pub const fn plan(&self) -> &PackingPlan {
        &self.plan
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

    /// Replaces every logical value and restores zero padding.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::LengthMismatch`] before writing when `values` has
    /// a different logical length.
    pub fn pack_from(&mut self, values: &[F]) -> Result<(), PackError> {
        if values.len() != self.len() {
            return Err(PackError::LengthMismatch {
                expected: self.len(),
                actual: values.len(),
            });
        }
        self.values[..values.len()].copy_from_slice(values);
        self.values[values.len()..].fill(F::ZERO);
        Ok(())
    }

    /// Copies logical elements to a normal `AoS` slice.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::LengthMismatch`] before writing when `out` has a
    /// different logical length.
    pub fn unpack_into(&self, out: &mut [F]) -> Result<(), PackError> {
        self.as_view().unpack_into(out)
    }

    /// Reborrows this mutable view as an immutable packed view.
    #[must_use]
    pub fn as_view(&self) -> PackedBatchView<'_, F> {
        PackedBatchView {
            values: self.values,
            plan: self.plan,
        }
    }
}
