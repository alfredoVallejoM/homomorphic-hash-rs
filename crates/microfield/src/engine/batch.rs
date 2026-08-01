//! Safe validation boundary for slice operations.

use crate::BatchError;

#[inline]
pub(super) fn validate_binary<F>(out: &[F], lhs: &[F], rhs: &[F]) -> Result<(), BatchError> {
    if out.len() != lhs.len() || lhs.len() != rhs.len() {
        return Err(BatchError::LengthMismatch {
            out: out.len(),
            lhs: lhs.len(),
            rhs: Some(rhs.len()),
        });
    }
    Ok(())
}

#[inline]
pub(super) fn validate_unary<F>(out: &[F], values: &[F]) -> Result<(), BatchError> {
    if out.len() != values.len() {
        return Err(BatchError::LengthMismatch {
            out: out.len(),
            lhs: values.len(),
            rhs: None,
        });
    }
    Ok(())
}

#[inline]
pub(super) fn validate_binary_assign<F>(lhs: &[F], rhs: &[F]) -> Result<(), BatchError> {
    if lhs.len() != rhs.len() {
        return Err(BatchError::LengthMismatch {
            out: lhs.len(),
            lhs: lhs.len(),
            rhs: Some(rhs.len()),
        });
    }
    Ok(())
}
