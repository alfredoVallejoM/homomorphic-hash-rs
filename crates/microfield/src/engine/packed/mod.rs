//! Persistent packed batches and allocation-free borrowed views.
//!
//! Storage borrows prevent creating overlapping mutable packed views:
//!
//! ```compile_fail
//! use core::mem::MaybeUninit;
//! use microfield::{Engine, Field, Gf2_128V1, pack_into_storage};
//!
//! let engine = Engine::<Gf2_128V1>::portable();
//! let values = [Gf2_128V1::ONE; 2];
//! let mut storage = [MaybeUninit::<u8>::uninit(); 128];
//! let first = pack_into_storage(&engine, &mut storage, &values).unwrap();
//! let second = pack_into_storage(&engine, &mut storage, &values).unwrap();
//! let _ = (first, second);
//! ```
//!
//! Packing plans are runtime execution contracts, not a serialization format:
//!
//! ```compile_fail
//! use microfield::{Engine, Gf2_128V1};
//!
//! let plan = Engine::<Gf2_128V1>::portable().packing_plan(8).unwrap();
//! let _ = serde_json::to_string(&plan).unwrap();
//! ```

mod plan;
#[allow(unsafe_code)]
mod storage;
mod view;

#[cfg(feature = "alloc")]
mod owned;

use core::mem::MaybeUninit;

use crate::{__private::PortableField, StaticField, kernel::PackedKernelSet};

#[cfg(feature = "alloc")]
pub use owned::PackedBatch;
pub use plan::{PackError, PackedLayout, PackingPlan};
pub use view::{PackedBatchView, PackedBatchViewMut};
use view::{PackedStorageMut, PackedStorageRef};

use super::Engine;

impl<F: PortableField + StaticField> Engine<F> {
    /// Builds the immutable packing contract for a logical batch length.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] if size computation overflows or backend metadata
    /// contains an invalid alignment.
    pub fn packing_plan(&self, len: usize) -> Result<PackingPlan, PackError> {
        PackingPlan::build::<F>(self.kernels, len)
    }

    /// Adds persistent owned batches without repacking or allocation.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] before writing when plans are incompatible.
    #[cfg(feature = "alloc")]
    pub fn add_packed_into(
        &self,
        out: &mut PackedBatch<F>,
        lhs: &PackedBatch<F>,
        rhs: &PackedBatch<F>,
    ) -> Result<(), PackError> {
        self.add_packed_view_into(&mut out.as_view_mut(), &lhs.as_view(), &rhs.as_view())
    }

    /// Multiplies persistent owned batches without repacking or allocation.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] before writing when a batch was built for another
    /// backend, length or packing plan.
    #[cfg(feature = "alloc")]
    pub fn mul_packed_into(
        &self,
        out: &mut PackedBatch<F>,
        lhs: &PackedBatch<F>,
        rhs: &PackedBatch<F>,
    ) -> Result<(), PackError> {
        self.mul_packed_view_into(&mut out.as_view_mut(), &lhs.as_view(), &rhs.as_view())
    }

    /// Squares a persistent owned batch without repacking or allocation.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] before writing when a batch was built for another
    /// backend, length or packing plan.
    #[cfg(feature = "alloc")]
    pub fn square_packed_into(
        &self,
        out: &mut PackedBatch<F>,
        values: &PackedBatch<F>,
    ) -> Result<(), PackError> {
        self.square_packed_view_into(&mut out.as_view_mut(), &values.as_view())
    }

    /// Multiplies a persistent owned batch in place.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] before writing when plans are incompatible.
    #[cfg(feature = "alloc")]
    pub fn mul_packed_assign(
        &self,
        lhs: &mut PackedBatch<F>,
        rhs: &PackedBatch<F>,
    ) -> Result<(), PackError> {
        self.mul_packed_view_assign(&mut lhs.as_view_mut(), &rhs.as_view())
    }

    /// Squares a persistent owned batch in place.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] before writing when the batch belongs to another
    /// backend.
    #[cfg(feature = "alloc")]
    pub fn square_packed_assign(&self, values: &mut PackedBatch<F>) -> Result<(), PackError> {
        self.square_packed_view_assign(&mut values.as_view_mut())
    }

    /// Adds borrowed persistent views without allocation or transcoding.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] before writing when plans are incompatible.
    pub fn add_packed_view_into(
        &self,
        out: &mut PackedBatchViewMut<'_, F>,
        lhs: &PackedBatchView<'_, F>,
        rhs: &PackedBatchView<'_, F>,
    ) -> Result<(), PackError> {
        self.validate_plans(&[out.plan(), lhs.plan(), rhs.plan()])?;
        match (
            &self.kernels.packed,
            &mut out.storage,
            &lhs.storage,
            &rhs.storage,
        ) {
            (
                PackedKernelSet::Aos,
                PackedStorageMut::Aos(out),
                PackedStorageRef::Aos(lhs),
                PackedStorageRef::Aos(rhs),
            ) => (self.kernels.add)(out, lhs, rhs),
            (
                PackedKernelSet::CanonicalU8(kernels),
                PackedStorageMut::CanonicalU8 { values: out, .. },
                PackedStorageRef::CanonicalU8 { values: lhs, .. },
                PackedStorageRef::CanonicalU8 { values: rhs, .. },
            ) => (kernels.add)(out, lhs, rhs),
            (
                PackedKernelSet::CanonicalU16(kernels),
                PackedStorageMut::CanonicalU16 { values: out, .. },
                PackedStorageRef::CanonicalU16 { values: lhs, .. },
                PackedStorageRef::CanonicalU16 { values: rhs, .. },
            ) => (kernels.add)(out, lhs, rhs),
            (
                PackedKernelSet::CanonicalU32(kernels),
                PackedStorageMut::CanonicalU32 { values: out, .. },
                PackedStorageRef::CanonicalU32 { values: lhs, .. },
                PackedStorageRef::CanonicalU32 { values: rhs, .. },
            ) => (kernels.add)(out, lhs, rhs),
            _ => return Err(PackError::IncompatiblePlan),
        }
        Ok(())
    }

    /// Multiplies borrowed packed views without allocation.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] before writing when plans are incompatible.
    pub fn mul_packed_view_into(
        &self,
        out: &mut PackedBatchViewMut<'_, F>,
        lhs: &PackedBatchView<'_, F>,
        rhs: &PackedBatchView<'_, F>,
    ) -> Result<(), PackError> {
        self.validate_plans(&[out.plan(), lhs.plan(), rhs.plan()])?;
        match (
            &self.kernels.packed,
            &mut out.storage,
            &lhs.storage,
            &rhs.storage,
        ) {
            (
                PackedKernelSet::Aos,
                PackedStorageMut::Aos(out),
                PackedStorageRef::Aos(lhs),
                PackedStorageRef::Aos(rhs),
            ) => (self.kernels.multiply)(out, lhs, rhs),
            (
                PackedKernelSet::CanonicalU8(kernels),
                PackedStorageMut::CanonicalU8 { values: out, .. },
                PackedStorageRef::CanonicalU8 { values: lhs, .. },
                PackedStorageRef::CanonicalU8 { values: rhs, .. },
            ) => (kernels.multiply)(out, lhs, rhs),
            (
                PackedKernelSet::CanonicalU16(kernels),
                PackedStorageMut::CanonicalU16 { values: out, .. },
                PackedStorageRef::CanonicalU16 { values: lhs, .. },
                PackedStorageRef::CanonicalU16 { values: rhs, .. },
            ) => (kernels.multiply)(out, lhs, rhs),
            (
                PackedKernelSet::CanonicalU32(kernels),
                PackedStorageMut::CanonicalU32 { values: out, .. },
                PackedStorageRef::CanonicalU32 { values: lhs, .. },
                PackedStorageRef::CanonicalU32 { values: rhs, .. },
            ) => (kernels.multiply)(out, lhs, rhs),
            _ => return Err(PackError::IncompatiblePlan),
        }
        Ok(())
    }

    /// Squares a borrowed packed view without allocation.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] before writing when plans are incompatible.
    pub fn square_packed_view_into(
        &self,
        out: &mut PackedBatchViewMut<'_, F>,
        values: &PackedBatchView<'_, F>,
    ) -> Result<(), PackError> {
        self.validate_plans(&[out.plan(), values.plan()])?;
        match (&self.kernels.packed, &mut out.storage, &values.storage) {
            (PackedKernelSet::Aos, PackedStorageMut::Aos(out), PackedStorageRef::Aos(values)) => {
                (self.kernels.square)(out, values);
            }
            (
                PackedKernelSet::CanonicalU8(kernels),
                PackedStorageMut::CanonicalU8 { values: out, .. },
                PackedStorageRef::CanonicalU8 { values, .. },
            ) => (kernels.square)(out, values),
            (
                PackedKernelSet::CanonicalU16(kernels),
                PackedStorageMut::CanonicalU16 { values: out, .. },
                PackedStorageRef::CanonicalU16 { values, .. },
            ) => (kernels.square)(out, values),
            (
                PackedKernelSet::CanonicalU32(kernels),
                PackedStorageMut::CanonicalU32 { values: out, .. },
                PackedStorageRef::CanonicalU32 { values, .. },
            ) => (kernels.square)(out, values),
            _ => return Err(PackError::IncompatiblePlan),
        }
        Ok(())
    }

    /// Multiplies a borrowed packed view in place.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] before writing when plans are incompatible.
    pub fn mul_packed_view_assign(
        &self,
        lhs: &mut PackedBatchViewMut<'_, F>,
        rhs: &PackedBatchView<'_, F>,
    ) -> Result<(), PackError> {
        self.validate_plans(&[lhs.plan(), rhs.plan()])?;
        match (&self.kernels.packed, &mut lhs.storage, &rhs.storage) {
            (PackedKernelSet::Aos, PackedStorageMut::Aos(lhs), PackedStorageRef::Aos(rhs)) => {
                (self.kernels.multiply_assign)(lhs, rhs);
            }
            (
                PackedKernelSet::CanonicalU8(kernels),
                PackedStorageMut::CanonicalU8 { values: lhs, .. },
                PackedStorageRef::CanonicalU8 { values: rhs, .. },
            ) => (kernels.multiply_assign)(lhs, rhs),
            (
                PackedKernelSet::CanonicalU16(kernels),
                PackedStorageMut::CanonicalU16 { values: lhs, .. },
                PackedStorageRef::CanonicalU16 { values: rhs, .. },
            ) => (kernels.multiply_assign)(lhs, rhs),
            (
                PackedKernelSet::CanonicalU32(kernels),
                PackedStorageMut::CanonicalU32 { values: lhs, .. },
                PackedStorageRef::CanonicalU32 { values: rhs, .. },
            ) => (kernels.multiply_assign)(lhs, rhs),
            _ => return Err(PackError::IncompatiblePlan),
        }
        Ok(())
    }

    /// Squares a borrowed packed view in place.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] before writing when the plan belongs to another
    /// backend.
    pub fn square_packed_view_assign(
        &self,
        values: &mut PackedBatchViewMut<'_, F>,
    ) -> Result<(), PackError> {
        self.validate_plans(&[values.plan()])?;
        match (&self.kernels.packed, &mut values.storage) {
            (PackedKernelSet::Aos, PackedStorageMut::Aos(values)) => {
                (self.kernels.square_assign)(values);
            }
            (
                PackedKernelSet::CanonicalU8(kernels),
                PackedStorageMut::CanonicalU8 { values, .. },
            ) => (kernels.square_assign)(values),
            (
                PackedKernelSet::CanonicalU16(kernels),
                PackedStorageMut::CanonicalU16 { values, .. },
            ) => (kernels.square_assign)(values),
            (
                PackedKernelSet::CanonicalU32(kernels),
                PackedStorageMut::CanonicalU32 { values, .. },
            ) => (kernels.square_assign)(values),
            _ => return Err(PackError::IncompatiblePlan),
        }
        Ok(())
    }

    fn validate_plans(&self, plans: &[&PackingPlan]) -> Result<(), PackError> {
        let Some(reference) = plans.first() else {
            return Ok(());
        };
        for plan in plans {
            if plan.backend_id() != self.backend_id() {
                return Err(PackError::WrongBackend {
                    expected: self.backend_id(),
                    actual: plan.backend_id(),
                });
            }
            if !reference.is_compatible_with(plan) {
                return Err(PackError::IncompatiblePlan);
            }
        }
        Ok(())
    }
}

/// Returns the byte capacity that guarantees an aligned region for `plan`
/// regardless of the caller storage address.
///
/// # Errors
///
/// Returns [`PackError::SizeOverflow`] if alignment slack overflows `usize`.
pub fn required_packed_bytes(plan: &PackingPlan) -> Result<usize, PackError> {
    if plan.data_bytes() == 0 {
        return Ok(0);
    }
    plan.data_bytes()
        .checked_add(plan.alignment() - 1)
        .ok_or(PackError::SizeOverflow)
}

/// Packs `AoS` values into aligned caller-provided uninitialized byte storage.
///
/// The returned exclusive view owns the storage borrow. Dropping the view does
/// not deallocate or serialize the caller's bytes.
///
/// # Errors
///
/// Returns [`PackError`] before writing if planning fails or the concrete
/// storage region is too small.
pub fn pack_into_storage<'a, F: PortableField + StaticField>(
    engine: &Engine<F>,
    storage: &'a mut [MaybeUninit<u8>],
    values: &[F],
) -> Result<PackedBatchViewMut<'a, F>, PackError> {
    let plan = engine.packing_plan(values.len())?;
    let storage = match engine.kernels.packed {
        PackedKernelSet::Aos => {
            PackedStorageMut::Aos(storage::initialize_aos_storage(storage, &plan, values)?)
        }
        PackedKernelSet::CanonicalU8(kernels) => PackedStorageMut::CanonicalU8 {
            values: storage::initialize_lane_storage(storage, &plan, values, 0_u8, kernels.pack)?,
            kernels,
        },
        PackedKernelSet::CanonicalU16(kernels) => PackedStorageMut::CanonicalU16 {
            values: storage::initialize_lane_storage(storage, &plan, values, 0_u16, kernels.pack)?,
            kernels,
        },
        PackedKernelSet::CanonicalU32(kernels) => PackedStorageMut::CanonicalU32 {
            values: storage::initialize_lane_storage(storage, &plan, values, 0_u32, kernels.pack)?,
            kernels,
        },
    };
    Ok(PackedBatchViewMut { storage, plan })
}
