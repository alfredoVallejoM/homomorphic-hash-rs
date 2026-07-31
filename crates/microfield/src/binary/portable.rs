//! Allocation-free portable carry-less field arithmetic.

use super::representation::{Limbs256, Wide512};

/// Computes the carry-less product of two 64-bit polynomials.
///
/// The two return values are the low and high halves of the degree-at-most-126
/// polynomial. This implementation is intentionally portable and variable
/// time; ISA adapters may replace it for batch work without changing callers.
#[inline]
pub(crate) fn clmul64(lhs: u64, rhs: u64) -> (u64, u64) {
    let mut low = 0;
    let mut high = 0;
    let mut remaining = rhs;

    while remaining != 0 {
        let shift = remaining.trailing_zeros();
        low ^= lhs << shift;
        if shift != 0 {
            high ^= lhs >> (u64::BITS - shift);
        }
        remaining &= remaining - 1;
    }

    (low, high)
}

/// Computes an unreduced schoolbook product of two degree-256 polynomials.
#[inline]
pub(crate) fn wide_product_256(lhs: Limbs256, rhs: Limbs256) -> Wide512 {
    let mut output = [0; 8];
    for (lhs_index, lhs_limb) in lhs.into_iter().enumerate() {
        for (rhs_index, rhs_limb) in rhs.into_iter().enumerate() {
            let (low, high) = clmul64(lhs_limb, rhs_limb);
            output[lhs_index + rhs_index] ^= low;
            output[lhs_index + rhs_index + 1] ^= high;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::clmul64;

    #[test]
    fn carryless_product_crosses_the_word_boundary() {
        assert_eq!(clmul64(1 << 63, 2), (0, 1));
        assert_eq!(clmul64(u64::MAX, 1), (u64::MAX, 0));
        assert_eq!(clmul64(0b1011, 0b1101), (0b111_1111, 0));
    }
}
