//! Neutral ABI for persistent packed lanes.

use crate::Field;

pub(crate) type PackKernel<F, T> = fn(out: &mut [T], values: &[F]);
pub(crate) type UnpackKernel<F, T> = fn(out: &mut [F], values: &[T]);
pub(crate) type PackedBinaryKernel<T> = fn(out: &mut [T], lhs: &[T], rhs: &[T]);
pub(crate) type PackedUnaryKernel<T> = fn(out: &mut [T], values: &[T]);
pub(crate) type PackedBinaryAssignKernel<T> = fn(lhs: &mut [T], rhs: &[T]);
pub(crate) type PackedUnaryAssignKernel<T> = fn(values: &mut [T]);

/// Physical scalar stored in one persistent packed lane.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(not(feature = "portable"), allow(dead_code))]
pub(crate) enum PackedStorageKind {
    Aos,
    CanonicalU8,
    CanonicalU16,
    CanonicalU32,
}

/// Immutable codecs and arithmetic entry points for one lane width.
#[derive(Clone, Copy)]
#[cfg_attr(
    not(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64")),
    allow(dead_code)
)]
pub(crate) struct PackedLaneKernels<F: Field, T: Copy> {
    pub(crate) pack: PackKernel<F, T>,
    pub(crate) unpack: UnpackKernel<F, T>,
    pub(crate) add: PackedBinaryKernel<T>,
    pub(crate) multiply: PackedBinaryKernel<T>,
    pub(crate) square: PackedUnaryKernel<T>,
    pub(crate) multiply_assign: PackedBinaryAssignKernel<T>,
    pub(crate) square_assign: PackedUnaryAssignKernel<T>,
}

impl<F: Field, T: Copy> PackedLaneKernels<F, T> {
    #[cfg_attr(
        not(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64")),
        allow(dead_code)
    )]
    pub(crate) const fn new(
        pack: PackKernel<F, T>,
        unpack: UnpackKernel<F, T>,
        add: PackedBinaryKernel<T>,
        multiply: PackedBinaryKernel<T>,
        square: PackedUnaryKernel<T>,
        multiply_assign: PackedBinaryAssignKernel<T>,
        square_assign: PackedUnaryAssignKernel<T>,
    ) -> Self {
        Self {
            pack,
            unpack,
            add,
            multiply,
            square,
            multiply_assign,
            square_assign,
        }
    }
}

/// Optional physical strategy attached to a normal field kernel table.
///
/// The enum keeps lane dispatch static and exhaustively checked. It is matched
/// once by the packed facade, never inside an arithmetic loop.
#[derive(Clone, Copy)]
#[cfg_attr(
    not(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64")),
    allow(dead_code)
)]
pub(crate) enum PackedKernelSet<F: Field> {
    Aos,
    CanonicalU8(PackedLaneKernels<F, u8>),
    CanonicalU16(PackedLaneKernels<F, u16>),
    CanonicalU32(PackedLaneKernels<F, u32>),
}

impl<F: Field> PackedKernelSet<F> {
    #[cfg_attr(not(feature = "portable"), allow(dead_code))]
    pub(crate) const fn storage_kind(&self) -> PackedStorageKind {
        match self {
            Self::Aos => PackedStorageKind::Aos,
            Self::CanonicalU8(_) => PackedStorageKind::CanonicalU8,
            Self::CanonicalU16(_) => PackedStorageKind::CanonicalU16,
            Self::CanonicalU32(_) => PackedStorageKind::CanonicalU32,
        }
    }
}
