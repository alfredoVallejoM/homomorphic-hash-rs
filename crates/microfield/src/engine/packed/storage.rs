//! Audited allocation and byte-storage adapter for packed field values.

use core::{mem::MaybeUninit, slice};

#[cfg(feature = "alloc")]
use alloc::alloc::{alloc, dealloc};
#[cfg(feature = "alloc")]
use core::{alloc::Layout, marker::PhantomData, ptr::NonNull};

use crate::Field;

use super::{PackError, PackingPlan};

/// Owned allocation with a backend-selected alignment.
///
/// The generic parameter prevents safe code from reinterpreting an allocation
/// as a different element type after initialization.
#[cfg(feature = "alloc")]
pub(super) struct AlignedBuffer<F: Copy> {
    ptr: NonNull<F>,
    len: usize,
    layout: Option<Layout>,
    field: PhantomData<F>,
}

#[cfg(feature = "alloc")]
impl<F: Copy> AlignedBuffer<F> {
    pub(super) fn new(len: usize, alignment: usize, initial: F) -> Result<Self, PackError> {
        if len == 0 {
            return Ok(Self {
                ptr: NonNull::dangling(),
                len,
                layout: None,
                field: PhantomData,
            });
        }

        let bytes = len
            .checked_mul(core::mem::size_of::<F>())
            .ok_or(PackError::SizeOverflow)?;
        let layout =
            Layout::from_size_align(bytes, alignment).map_err(|_| PackError::SizeOverflow)?;
        // SAFETY: `layout` has non-zero size and valid power-of-two alignment.
        let raw = unsafe { alloc(layout) };
        let ptr = NonNull::new(raw.cast::<F>()).ok_or(PackError::AllocationFailed)?;

        let buffer = Self {
            ptr,
            len,
            layout: Some(layout),
            field: PhantomData,
        };
        for slot in 0..len {
            // SAFETY: the allocation contains `len` aligned `F` slots and each
            // slot is written exactly once before a typed reference is exposed.
            unsafe { buffer.ptr.as_ptr().add(slot).write(initial) };
        }
        Ok(buffer)
    }

    pub(super) fn as_slice(&self) -> &[F] {
        // SAFETY: construction initializes all `len` slots and the allocation
        // remains owned and live for the returned shared borrow.
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    pub(super) fn as_mut_slice(&mut self) -> &mut [F] {
        // SAFETY: construction initializes all `len` slots and `&mut self`
        // guarantees exclusive access for the returned mutable borrow.
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

#[cfg(feature = "alloc")]
impl<F: Copy> Drop for AlignedBuffer<F> {
    fn drop(&mut self) {
        if let Some(layout) = self.layout {
            // `F: Copy` cannot require drop glue. The exact allocation layout
            // is retained from construction and deallocated once.
            // SAFETY: `ptr` came from `alloc(layout)` and is still owned here.
            unsafe { dealloc(self.ptr.as_ptr().cast::<u8>(), layout) };
        }
    }
}

// The allocation has unique ownership and exposes access only through Rust
// borrows. Thread-safety therefore follows the stored element type.
#[cfg(feature = "alloc")]
unsafe impl<F: Copy + Send> Send for AlignedBuffer<F> {}
#[cfg(feature = "alloc")]
unsafe impl<F: Copy + Sync> Sync for AlignedBuffer<F> {}

/// Aligns, initializes and types a region inside caller-provided byte storage.
pub(super) fn initialize_aos_storage<'a, F: Field>(
    storage: &'a mut [MaybeUninit<u8>],
    plan: &PackingPlan,
    values: &[F],
) -> Result<&'a mut [F], PackError> {
    if values.len() != plan.logical_len() {
        return Err(PackError::LengthMismatch {
            expected: plan.logical_len(),
            actual: values.len(),
        });
    }
    if plan.padded_len() == 0 {
        return Ok(&mut []);
    }

    let base = storage.as_mut_ptr().cast::<u8>();
    let offset = base.align_offset(plan.alignment());
    if offset == usize::MAX {
        return Err(PackError::InvalidAlignment {
            alignment: plan.alignment(),
        });
    }
    let required = offset
        .checked_add(plan.data_bytes())
        .ok_or(PackError::SizeOverflow)?;
    if storage.len() < required {
        return Err(PackError::InsufficientStorage {
            required,
            provided: storage.len(),
        });
    }

    // SAFETY: `offset` aligns the pointer for `F`, `required` proves that the
    // region contains `padded_len * size_of::<F>()` bytes, and the exclusive
    // storage borrow lasts for the returned slice. Every slot is initialized
    // before the typed slice is created.
    let typed = unsafe { base.add(offset).cast::<F>() };
    for (index, value) in values.iter().copied().enumerate() {
        // SAFETY: `index < logical_len <= padded_len` and the slot is aligned.
        unsafe { typed.add(index).write(value) };
    }
    for index in values.len()..plan.padded_len() {
        // SAFETY: every padding index is within the proven packed region.
        unsafe { typed.add(index).write(F::ZERO) };
    }
    // SAFETY: all `padded_len` slots were initialized above and remain under
    // the unique lifetime of `storage`.
    Ok(unsafe { slice::from_raw_parts_mut(typed, plan.padded_len()) })
}

/// Aligns and initializes primitive lane storage before running a safe codec.
pub(super) fn initialize_lane_storage<'a, F: Field, T: Copy>(
    storage: &'a mut [MaybeUninit<u8>],
    plan: &PackingPlan,
    values: &[F],
    zero: T,
    pack: fn(&mut [T], &[F]),
) -> Result<&'a mut [T], PackError> {
    if values.len() != plan.logical_len() {
        return Err(PackError::LengthMismatch {
            expected: plan.logical_len(),
            actual: values.len(),
        });
    }
    if plan.padded_len() == 0 {
        return Ok(&mut []);
    }

    let expected_bytes = plan
        .padded_len()
        .checked_mul(core::mem::size_of::<T>())
        .ok_or(PackError::SizeOverflow)?;
    if expected_bytes != plan.data_bytes() {
        return Err(PackError::IncompatiblePlan);
    }

    let base = storage.as_mut_ptr().cast::<u8>();
    let offset = base.align_offset(plan.alignment());
    if offset == usize::MAX {
        return Err(PackError::InvalidAlignment {
            alignment: plan.alignment(),
        });
    }
    let required = offset
        .checked_add(expected_bytes)
        .ok_or(PackError::SizeOverflow)?;
    if storage.len() < required {
        return Err(PackError::InsufficientStorage {
            required,
            provided: storage.len(),
        });
    }

    // SAFETY: `plan.alignment()` is at least `align_of::<T>()`; the capacity
    // check proves every slot lies within the exclusive byte storage.
    let typed = unsafe { base.add(offset).cast::<T>() };
    for index in 0..plan.padded_len() {
        // SAFETY: every index is within the region proven above. Initializing
        // all slots precedes construction of a typed reference.
        unsafe { typed.add(index).write(zero) };
    }
    // SAFETY: all slots are initialized valid `T` values and the byte region
    // remains exclusively borrowed for the returned slice lifetime.
    let lanes = unsafe { slice::from_raw_parts_mut(typed, plan.padded_len()) };
    pack(lanes, values);
    Ok(lanes)
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use alloc::vec;
    use core::mem::MaybeUninit;

    use crate::{Field, Gf2_128V1, KernelMetadata, kernel::KernelSet};

    use super::{AlignedBuffer, initialize_lane_storage};

    #[test]
    fn owned_buffer_honors_supported_alignments_and_initializes_every_slot() {
        for alignment in [8, 16, 32, 64, 128] {
            let buffer = AlignedBuffer::new(7, alignment, 0xfeed_beef_u64)
                .expect("test allocation must succeed");
            assert_eq!(buffer.ptr.as_ptr().addr() % alignment, 0);
            assert_eq!(buffer.as_slice(), &[0xfeed_beef_u64; 7]);
        }

        let empty = AlignedBuffer::new(0, 64, 0_u64).expect("empty allocation is valid");
        assert!(empty.as_slice().is_empty());
        assert!(matches!(
            AlignedBuffer::new(usize::MAX, 8, 0_u64),
            Err(super::PackError::SizeOverflow)
        ));
    }

    #[test]
    fn owned_buffer_thread_safety_follows_its_element() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AlignedBuffer<u64>>();
    }

    #[test]
    fn primitive_lane_storage_is_aligned_initialized_and_padding_safe() {
        let kernels = KernelSet::new(
            KernelMetadata::for_packing_test(4, 32),
            field_binary,
            field_binary,
            field_unary,
            field_binary_assign,
            field_unary_assign,
        )
        .with_packed(crate::kernel::PackedKernelSet::CanonicalU16(
            crate::kernel::PackedLaneKernels::new(
                pack_u16,
                unpack_u16,
                lane_binary,
                lane_binary,
                lane_unary,
                lane_binary_assign,
                lane_unary_assign,
            ),
        ));
        let plan = crate::engine::packed::PackingPlan::build::<Gf2_128V1>(&kernels, 3).unwrap();
        assert_eq!(plan.padded_len(), 4);
        assert_eq!(plan.data_bytes(), 8);

        for offset in 0..plan.alignment() {
            let mut bytes =
                vec![MaybeUninit::new(0xa5_u8); plan.data_bytes() + plan.alignment() * 2];
            let lanes = initialize_lane_storage(
                &mut bytes[offset..],
                &plan,
                &[Gf2_128V1::ZERO; 3],
                0_u16,
                pack_u16,
            )
            .unwrap();
            assert_eq!(lanes, &[1, 2, 3, 0]);
            assert_eq!(lanes.as_ptr().addr() % 32, 0);
        }
    }

    fn field_binary(out: &mut [Gf2_128V1], lhs: &[Gf2_128V1], _rhs: &[Gf2_128V1]) {
        out.copy_from_slice(lhs);
    }

    fn field_unary(out: &mut [Gf2_128V1], values: &[Gf2_128V1]) {
        out.copy_from_slice(values);
    }

    fn field_binary_assign(_lhs: &mut [Gf2_128V1], _rhs: &[Gf2_128V1]) {}

    fn field_unary_assign(_values: &mut [Gf2_128V1]) {}

    fn pack_u16(out: &mut [u16], values: &[Gf2_128V1]) {
        for (index, output) in out.iter_mut().take(values.len()).enumerate() {
            *output = u16::try_from(index + 1).unwrap();
        }
    }

    fn unpack_u16(out: &mut [Gf2_128V1], _values: &[u16]) {
        out.fill(Gf2_128V1::ZERO);
    }

    fn lane_binary(out: &mut [u16], lhs: &[u16], _rhs: &[u16]) {
        out.copy_from_slice(lhs);
    }

    fn lane_unary(out: &mut [u16], values: &[u16]) {
        out.copy_from_slice(values);
    }

    fn lane_binary_assign(_lhs: &mut [u16], _rhs: &[u16]) {}

    fn lane_unary_assign(_values: &mut [u16]) {}
}
