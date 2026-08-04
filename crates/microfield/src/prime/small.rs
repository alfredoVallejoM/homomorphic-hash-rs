//! Small-prime portable helpers.

// The modular remainder is strictly smaller than the u16 modulus.
#![allow(clippy::cast_possible_truncation)]

#[must_use]
pub(crate) fn reduce_bytes_mod_u16(bytes_le: &[u8], modulus: u16) -> u16 {
    let mut residue = 0_u16;
    for byte in bytes_le.iter().rev() {
        residue =
            ((u32::from(residue) << 8) + u32::from(*byte)).wrapping_rem(u32::from(modulus)) as u16;
    }
    residue
}
