//! Immutable layout planning and validation errors.

use core::{fmt, mem};

use crate::{BackendId, FieldId, KernelMetadata, StaticField};

/// Physical representation used by a packed batch.
///
/// Layout variants are introduced only with a backend that executes them.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PackedLayout {
    /// Consecutive field elements in canonical Rust object layout.
    Aos,
    /// Two consecutive `AoS` elements form one 128-bit VPCLMUL lane pair.
    ///
    /// Limbs remain private and local to each element. The plan guarantees an
    /// even padded length and a 32-byte allocation start, while the backend
    /// performs the register-level lane interleave.
    AosLanePairs,
}

/// Immutable description of one packed allocation or borrowed view.
///
/// Plans are constructed by [`crate::Engine::packing_plan`], so layout,
/// alignment and padding remain backend decisions rather than caller input.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PackingPlan {
    backend: BackendId,
    field_id: FieldId,
    layout: PackedLayout,
    logical_len: usize,
    padded_len: usize,
    tile_elements: usize,
    limb_count: usize,
    element_size: usize,
    alignment: usize,
    data_bytes: usize,
}

impl PackingPlan {
    pub(super) fn build<F: StaticField>(
        metadata: &KernelMetadata,
        logical_len: usize,
    ) -> Result<Self, PackError> {
        let element_size = mem::size_of::<F>();
        if element_size == 0 {
            return Err(PackError::ZeroSizedField);
        }

        let alignment = metadata.required_alignment().max(mem::align_of::<F>());
        if !alignment.is_power_of_two() {
            return Err(PackError::InvalidAlignment { alignment });
        }

        let tile_elements = metadata.preferred_multiple().max(1);
        let remainder = logical_len % tile_elements;
        let padded_len = if remainder == 0 {
            logical_len
        } else {
            logical_len
                .checked_add(tile_elements - remainder)
                .ok_or(PackError::SizeOverflow)?
        };
        let data_bytes = padded_len
            .checked_mul(element_size)
            .ok_or(PackError::SizeOverflow)?;
        let degree = F::spec().degree() as usize;
        let limb_count = degree.checked_add(63).ok_or(PackError::SizeOverflow)? / 64;

        Ok(Self {
            backend: metadata.backend(),
            field_id: F::spec().field_id(),
            layout: if metadata.backend() == BackendId::X86Vpclmul {
                PackedLayout::AosLanePairs
            } else {
                PackedLayout::Aos
            },
            logical_len,
            padded_len,
            tile_elements,
            limb_count,
            element_size,
            alignment,
            data_bytes,
        })
    }

    /// Returns the backend for which the layout was selected.
    #[must_use]
    pub const fn backend_id(&self) -> BackendId {
        self.backend
    }

    /// Returns the field identity embedded in the generated field type.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.field_id
    }

    /// Returns the selected physical representation.
    #[must_use]
    pub const fn layout(&self) -> PackedLayout {
        self.layout
    }

    /// Returns the number of caller-visible elements.
    #[must_use]
    pub const fn logical_len(&self) -> usize {
        self.logical_len
    }

    /// Returns the number of stored elements, including initialized padding.
    #[must_use]
    pub const fn padded_len(&self) -> usize {
        self.padded_len
    }

    /// Returns the backend's preferred element multiple.
    #[must_use]
    pub const fn tile_elements(&self) -> usize {
        self.tile_elements
    }

    /// Returns the number of 64-bit polynomial words in one field value.
    #[must_use]
    pub const fn limb_count(&self) -> usize {
        self.limb_count
    }

    /// Returns the Rust object size of one field element.
    #[must_use]
    pub const fn element_size(&self) -> usize {
        self.element_size
    }

    /// Returns the required start alignment in bytes.
    #[must_use]
    pub const fn alignment(&self) -> usize {
        self.alignment
    }

    /// Returns the bytes occupied by packed elements, excluding alignment
    /// slack required by caller-provided storage.
    #[must_use]
    pub const fn data_bytes(&self) -> usize {
        self.data_bytes
    }

    pub(super) fn is_compatible_with(&self, other: &Self) -> bool {
        self == other
    }
}

/// Failure while planning, packing or executing a persistent batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PackError {
    /// A supplied slice has a different logical length from the plan.
    LengthMismatch {
        /// Length fixed by the plan.
        expected: usize,
        /// Length supplied by the caller.
        actual: usize,
    },
    /// A size, padding or allocation-layout computation overflowed `usize`.
    SizeOverflow,
    /// A zero-sized user-defined field cannot have an `AoS` packed layout.
    ZeroSizedField,
    /// Backend metadata requested an invalid alignment.
    InvalidAlignment {
        /// Rejected alignment in bytes.
        alignment: usize,
    },
    /// The allocator could not reserve the owned packed buffer.
    AllocationFailed,
    /// Caller-provided storage cannot contain an aligned packed region.
    InsufficientStorage {
        /// Minimum bytes required for the concrete storage address.
        required: usize,
        /// Bytes supplied by the caller.
        provided: usize,
    },
    /// The packed batch was created for a different engine backend.
    WrongBackend {
        /// Backend selected by the executing engine.
        expected: BackendId,
        /// Backend recorded by the packed batch.
        actual: BackendId,
    },
    /// Operands differ in layout, field identity, length or padding contract.
    IncompatiblePlan,
}

impl fmt::Display for PackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { expected, actual } => {
                write!(
                    formatter,
                    "packed length mismatch: expected {expected}, received {actual}"
                )
            }
            Self::SizeOverflow => formatter.write_str("packed storage size overflow"),
            Self::ZeroSizedField => formatter.write_str("zero-sized fields cannot be packed"),
            Self::InvalidAlignment { alignment } => {
                write!(formatter, "invalid packed alignment: {alignment}")
            }
            Self::AllocationFailed => formatter.write_str("packed storage allocation failed"),
            Self::InsufficientStorage { required, provided } => write!(
                formatter,
                "insufficient packed storage: required {required} bytes, provided {provided}"
            ),
            Self::WrongBackend { expected, actual } => write!(
                formatter,
                "packed backend mismatch: engine={expected:?}, batch={actual:?}"
            ),
            Self::IncompatiblePlan => formatter.write_str("incompatible packing plans"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PackError {}

#[cfg(all(test, feature = "builtin-fields"))]
mod tests {
    use crate::{Gf2_128V1, KernelMetadata};

    use super::{PackError, PackedLayout, PackingPlan};

    #[test]
    fn planner_rounds_tiles_and_checks_every_size_operation() {
        let metadata = KernelMetadata::for_packing_test(4, 64);
        let plan = PackingPlan::build::<Gf2_128V1>(&metadata, 5).expect("valid plan");
        assert_eq!(plan.layout(), PackedLayout::Aos);
        assert_eq!(plan.logical_len(), 5);
        assert_eq!(plan.padded_len(), 8);
        assert_eq!(plan.tile_elements(), 4);
        assert_eq!(plan.limb_count(), 2);
        assert_eq!(plan.element_size(), 16);
        assert_eq!(plan.alignment(), 64);
        assert_eq!(plan.data_bytes(), 128);

        assert_eq!(
            PackingPlan::build::<Gf2_128V1>(&metadata, usize::MAX),
            Err(PackError::SizeOverflow)
        );
        let invalid = KernelMetadata::for_packing_test(1, 24);
        assert_eq!(
            PackingPlan::build::<Gf2_128V1>(&invalid, 1),
            Err(PackError::InvalidAlignment { alignment: 24 })
        );
    }
}
