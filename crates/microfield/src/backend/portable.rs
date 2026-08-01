//! Portable batch strategy.

use crate::{Field, Square, kernel::KernelSet};

/// Creates the allocation-free portable strategy table for a field.
pub(crate) const fn kernel_set<F>() -> KernelSet<F>
where
    F: Field + Square,
{
    KernelSet::new(
        crate::KernelMetadata::portable::<F>(),
        add::<F>,
        multiply::<F>,
        square::<F>,
        multiply_assign::<F>,
        square_assign::<F>,
    )
}

#[inline]
fn add<F: Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
        *output = left.add(*right);
    }
}

#[inline]
fn multiply<F: Field>(out: &mut [F], lhs: &[F], rhs: &[F]) {
    debug_assert_eq!(out.len(), lhs.len());
    debug_assert_eq!(lhs.len(), rhs.len());
    for ((output, left), right) in out.iter_mut().zip(lhs).zip(rhs) {
        *output = left.mul(*right);
    }
}

#[inline]
fn square<F: Square>(out: &mut [F], values: &[F]) {
    debug_assert_eq!(out.len(), values.len());
    for (output, value) in out.iter_mut().zip(values) {
        *output = value.square();
    }
}

#[inline]
fn multiply_assign<F: Field>(lhs: &mut [F], rhs: &[F]) {
    debug_assert_eq!(lhs.len(), rhs.len());
    for (left, right) in lhs.iter_mut().zip(rhs) {
        *left = left.mul(*right);
    }
}

#[inline]
fn square_assign<F: Square>(values: &mut [F]) {
    for value in values {
        *value = value.square();
    }
}
