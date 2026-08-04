//! Owned packed batches backed by audited aligned storage.

use crate::{
    __private::PortableField,
    StaticField,
    kernel::{PackedKernelSet, PackedLaneKernels},
};

use super::{
    PackError, PackedBatchView, PackedBatchViewMut, PackingPlan,
    storage::AlignedBuffer,
    view::{PackedStorageMut, PackedStorageRef},
};

enum PackedStorage<F: PortableField> {
    Aos(AlignedBuffer<F>),
    CanonicalU8 {
        buffer: AlignedBuffer<u8>,
        kernels: PackedLaneKernels<F, u8>,
    },
    CanonicalU16 {
        buffer: AlignedBuffer<u16>,
        kernels: PackedLaneKernels<F, u16>,
    },
    CanonicalU32 {
        buffer: AlignedBuffer<u32>,
        kernels: PackedLaneKernels<F, u32>,
    },
}

impl<F: PortableField> PackedStorage<F> {
    fn new(engine: &crate::Engine<F>, plan: &PackingPlan) -> Result<Self, PackError> {
        match engine.kernels.packed {
            PackedKernelSet::Aos => Ok(Self::Aos(AlignedBuffer::new(
                plan.padded_len(),
                plan.alignment(),
                F::ZERO,
            )?)),
            PackedKernelSet::CanonicalU8(kernels) => Ok(Self::CanonicalU8 {
                buffer: AlignedBuffer::new(plan.padded_len(), plan.alignment(), 0_u8)?,
                kernels,
            }),
            PackedKernelSet::CanonicalU16(kernels) => Ok(Self::CanonicalU16 {
                buffer: AlignedBuffer::new(plan.padded_len(), plan.alignment(), 0_u16)?,
                kernels,
            }),
            PackedKernelSet::CanonicalU32(kernels) => Ok(Self::CanonicalU32 {
                buffer: AlignedBuffer::new(plan.padded_len(), plan.alignment(), 0_u32)?,
                kernels,
            }),
        }
    }

    fn as_ref(&self) -> PackedStorageRef<'_, F> {
        match self {
            Self::Aos(buffer) => PackedStorageRef::Aos(buffer.as_slice()),
            Self::CanonicalU8 { buffer, kernels } => PackedStorageRef::CanonicalU8 {
                values: buffer.as_slice(),
                kernels: *kernels,
            },
            Self::CanonicalU16 { buffer, kernels } => PackedStorageRef::CanonicalU16 {
                values: buffer.as_slice(),
                kernels: *kernels,
            },
            Self::CanonicalU32 { buffer, kernels } => PackedStorageRef::CanonicalU32 {
                values: buffer.as_slice(),
                kernels: *kernels,
            },
        }
    }

    fn as_mut(&mut self) -> PackedStorageMut<'_, F> {
        match self {
            Self::Aos(buffer) => PackedStorageMut::Aos(buffer.as_mut_slice()),
            Self::CanonicalU8 { buffer, kernels } => PackedStorageMut::CanonicalU8 {
                values: buffer.as_mut_slice(),
                kernels: *kernels,
            },
            Self::CanonicalU16 { buffer, kernels } => PackedStorageMut::CanonicalU16 {
                values: buffer.as_mut_slice(),
                kernels: *kernels,
            },
            Self::CanonicalU32 { buffer, kernels } => PackedStorageMut::CanonicalU32 {
                values: buffer.as_mut_slice(),
                kernels: *kernels,
            },
        }
    }
}

/// Persistent owned packed batch.
///
/// Construction and `AoS` conversion allocate, while repeated packed operations
/// do not allocate, transcode or change layout.
pub struct PackedBatch<F: PortableField + StaticField> {
    storage: PackedStorage<F>,
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
        let storage = PackedStorage::new(engine, &plan)?;
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
            storage: self.storage.as_ref(),
            plan: self.plan,
        }
    }

    /// Borrows the packed allocation mutably without conversion.
    #[must_use]
    pub fn as_view_mut(&mut self) -> PackedBatchViewMut<'_, F> {
        PackedBatchViewMut {
            storage: self.storage.as_mut(),
            plan: self.plan,
        }
    }
}
