//! Dedicated polynomial squaring.

use super::{
    reduction::reduce_256,
    representation::{Limbs256, Wide512},
};

/// Squares a degree-256 polynomial without evaluating cross products.
#[inline]
pub(crate) fn square_256<const MODULUS_TAIL: u64>(value: Limbs256) -> Limbs256 {
    reduce_256::<MODULUS_TAIL>(wide_square_256(value))
}

#[inline]
fn wide_square_256(value: Limbs256) -> Wide512 {
    let mut output = [0; 8];
    for (index, limb) in value.into_iter().enumerate() {
        let (low, high) = square64(limb);
        output[index * 2] = low;
        output[index * 2 + 1] = high;
    }
    output
}

#[inline]
fn square64(value: u64) -> (u64, u64) {
    (spread32(value), spread32(value >> 32))
}

#[inline]
fn spread32(mut value: u64) -> u64 {
    value &= 0x0000_0000_ffff_ffff;
    value = (value | (value << 16)) & 0x0000_ffff_0000_ffff;
    value = (value | (value << 8)) & 0x00ff_00ff_00ff_00ff;
    value = (value | (value << 4)) & 0x0f0f_0f0f_0f0f_0f0f;
    value = (value | (value << 2)) & 0x3333_3333_3333_3333;
    (value | (value << 1)) & 0x5555_5555_5555_5555
}

#[cfg(test)]
mod tests {
    use super::square64;

    #[test]
    fn bit_spreading_has_no_cross_terms() {
        assert_eq!(square64(0), (0, 0));
        assert_eq!(square64(0b11), (0b0101, 0));
        assert_eq!(square64(1 << 63), (0, 1 << 62));
    }
}
