//! Allocation-free packed views over caller-provided storage.

use crate::{Field, kernel::PackedLaneKernels};

use super::{PackError, PackingPlan};

pub(super) enum PackedStorageRef<'a, F: Field> {
    Aos(&'a [F]),
    CanonicalU8 {
        values: &'a [u8],
        kernels: PackedLaneKernels<F, u8>,
    },
    CanonicalU16 {
        values: &'a [u16],
        kernels: PackedLaneKernels<F, u16>,
    },
    CanonicalU32 {
        values: &'a [u32],
        kernels: PackedLaneKernels<F, u32>,
    },
}

pub(super) enum PackedStorageMut<'a, F: Field> {
    Aos(&'a mut [F]),
    CanonicalU8 {
        values: &'a mut [u8],
        kernels: PackedLaneKernels<F, u8>,
    },
    CanonicalU16 {
        values: &'a mut [u16],
        kernels: PackedLaneKernels<F, u16>,
    },
    CanonicalU32 {
        values: &'a mut [u32],
        kernels: PackedLaneKernels<F, u32>,
    },
}

/// Immutable packed batch borrowed from caller-provided or owned storage.
pub struct PackedBatchView<'a, F: Field> {
    pub(super) storage: PackedStorageRef<'a, F>,
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
        validate_length(self.len(), out.len())?;
        match &self.storage {
            PackedStorageRef::Aos(values) => out.copy_from_slice(&values[..self.len()]),
            PackedStorageRef::CanonicalU8 { values, kernels } => {
                (kernels.unpack)(out, &values[..self.len()]);
            }
            PackedStorageRef::CanonicalU16 { values, kernels } => {
                (kernels.unpack)(out, &values[..self.len()]);
            }
            PackedStorageRef::CanonicalU32 { values, kernels } => {
                (kernels.unpack)(out, &values[..self.len()]);
            }
        }
        Ok(())
    }
}

/// Mutable packed batch borrowed exclusively from caller-provided or owned
/// storage.
pub struct PackedBatchViewMut<'a, F: Field> {
    pub(super) storage: PackedStorageMut<'a, F>,
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

    /// Replaces every logical value and restores canonical zero padding.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::LengthMismatch`] before writing when `values` has
    /// a different logical length.
    pub fn pack_from(&mut self, values: &[F]) -> Result<(), PackError> {
        validate_length(self.len(), values.len())?;
        match &mut self.storage {
            PackedStorageMut::Aos(storage) => {
                storage[..values.len()].copy_from_slice(values);
                storage[values.len()..].fill(F::ZERO);
            }
            PackedStorageMut::CanonicalU8 {
                values: out,
                kernels,
            } => {
                (kernels.pack)(out, values);
            }
            PackedStorageMut::CanonicalU16 {
                values: out,
                kernels,
            } => {
                (kernels.pack)(out, values);
            }
            PackedStorageMut::CanonicalU32 {
                values: out,
                kernels,
            } => {
                (kernels.pack)(out, values);
            }
        }
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
        let storage = match &self.storage {
            PackedStorageMut::Aos(values) => PackedStorageRef::Aos(values),
            PackedStorageMut::CanonicalU8 { values, kernels } => PackedStorageRef::CanonicalU8 {
                values,
                kernels: *kernels,
            },
            PackedStorageMut::CanonicalU16 { values, kernels } => PackedStorageRef::CanonicalU16 {
                values,
                kernels: *kernels,
            },
            PackedStorageMut::CanonicalU32 { values, kernels } => PackedStorageRef::CanonicalU32 {
                values,
                kernels: *kernels,
            },
        };
        PackedBatchView {
            storage,
            plan: self.plan,
        }
    }
}

fn validate_length(expected: usize, actual: usize) -> Result<(), PackError> {
    if expected != actual {
        return Err(PackError::LengthMismatch { expected, actual });
    }
    Ok(())
}
