//! Embedded verification of maintained prime-field certificates.

// Multi-precision helpers explicitly retain the low limb after carrying the
// upper half; modular remainders are proven to fit their destination word.
#![allow(clippy::cast_possible_truncation)]

use core::{cmp::Ordering, fmt};

use crate::{CanonicalEncoding, Field, Fp256GenericV1, Pow};

const GENERIC_MODULUS: [u64; 4] = [
    0x60d7_67ee_a528_073f,
    0x59b0_47d9_a719_3eed,
    0xa2df_4d6d_fbec_a16e,
    0x9dad_4f18_e672_38cb,
];
const GENERIC_MINUS_ONE: [u64; 4] = [
    0x60d7_67ee_a528_073e,
    0x59b0_47d9_a719_3eed,
    0xa2df_4d6d_fbec_a16e,
    0x9dad_4f18_e672_38cb,
];
const KNOWN_FACTOR_PRODUCT: [u64; 4] = [0x2c90_4305_5a71_3832, 0x7c2c_fc09_81c1_216b, 0x46, 0];
const COFACTOR: [u64; 4] = [0x5175_c89a_0bcd_9477, 0x023c_adcd_d25a_c98d, 0, 0];
const GENERIC_FACTORS: [(u64, u64); 27] = [
    (2, 3),
    (3, 2),
    (5, 2),
    (7, 2),
    (11, 2),
    (13, 2),
    (17, 2),
    (19, 2),
    (23, 2),
    (29, 2),
    (31, 2),
    (37, 2),
    (41, 2),
    (43, 2),
    (47, 2),
    (53, 2),
    (59, 2),
    (61, 2),
    (67, 2),
    (71, 2),
    (73, 2),
    (79, 2),
    (83, 2),
    (89, 2),
    (97, 2),
    (101, 2),
    (103, 2),
];

/// Failure while replaying a maintained primality certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PrimeCertificateError {
    /// A declared factor is not prime.
    CompositeDeclaredFactor(u64),
    /// The factorization does not reconstruct the declared part of `p - 1`.
    FactorizationMismatch,
    /// The known factor product is not greater than the square root of `p`.
    InsufficientKnownFactor,
    /// Fermat's congruence failed for a Pocklington witness.
    FermatWitnessFailed(u64),
    /// A Pocklington witness has a non-trivial gcd with the modulus.
    PocklingtonGcdFailed(u64),
}

impl fmt::Display for PrimeCertificateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CompositeDeclaredFactor(factor) => {
                write!(formatter, "certificate factor {factor} is composite")
            }
            Self::FactorizationMismatch => {
                formatter.write_str("certificate factors do not reconstruct p - 1")
            }
            Self::InsufficientKnownFactor => {
                formatter.write_str("known Pocklington factor does not exceed sqrt(p)")
            }
            Self::FermatWitnessFailed(factor) => {
                write!(formatter, "Fermat witness failed for factor {factor}")
            }
            Self::PocklingtonGcdFailed(factor) => {
                write!(formatter, "Pocklington gcd failed for factor {factor}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PrimeCertificateError {}

/// Replays every maintained prime-field certificate without trusting Sage.
///
/// The 256-bit certificate uses Pocklington with a completely factored
/// 135-bit divisor of `p - 1`, which is larger than `sqrt(p)`.
///
/// # Errors
///
/// Returns the first failed certificate invariant.
pub fn verify_builtin_prime_certificates() -> Result<(), PrimeCertificateError> {
    verify_fp251()?;
    verify_goldilocks()?;
    verify_generic_256()
}

fn verify_fp251() -> Result<(), PrimeCertificateError> {
    for divisor in 2..=15 {
        if 251 % divisor == 0 {
            return Err(PrimeCertificateError::CompositeDeclaredFactor(251));
        }
    }
    Ok(())
}

fn verify_goldilocks() -> Result<(), PrimeCertificateError> {
    const MODULUS: u64 = 0xffff_ffff_0000_0001;
    const FACTORS: [(u64, u32, u64); 6] = [
        (2, 32, 7),
        (3, 1, 2),
        (5, 1, 3),
        (17, 1, 3),
        (257, 1, 3),
        (65_537, 1, 3),
    ];
    let mut product = 1_u128;
    for (factor, exponent, witness) in FACTORS {
        if !is_prime_u64(factor) {
            return Err(PrimeCertificateError::CompositeDeclaredFactor(factor));
        }
        product *= u128::from(factor).pow(exponent);
        if mod_pow_u64(witness, MODULUS - 1, MODULUS) != 1 {
            return Err(PrimeCertificateError::FermatWitnessFailed(factor));
        }
        let residue = mod_pow_u64(witness, (MODULUS - 1) / factor, MODULUS);
        if gcd_u64(residue.wrapping_sub(1), MODULUS) != 1 {
            return Err(PrimeCertificateError::PocklingtonGcdFailed(factor));
        }
    }
    if product != u128::from(MODULUS - 1) {
        return Err(PrimeCertificateError::FactorizationMismatch);
    }
    Ok(())
}

fn verify_generic_256() -> Result<(), PrimeCertificateError> {
    let mut product = [1_u64, 0, 0, 0];
    for (factor, _) in GENERIC_FACTORS {
        if !is_prime_u64(factor) {
            return Err(PrimeCertificateError::CompositeDeclaredFactor(factor));
        }
        product =
            multiply_small(product, factor).ok_or(PrimeCertificateError::FactorizationMismatch)?;
    }
    if product != KNOWN_FACTOR_PRODUCT {
        return Err(PrimeCertificateError::FactorizationMismatch);
    }
    if bit_length(product) <= 128 {
        return Err(PrimeCertificateError::InsufficientKnownFactor);
    }
    if multiply_256(product, COFACTOR)
        != [
            GENERIC_MINUS_ONE[0],
            GENERIC_MINUS_ONE[1],
            GENERIC_MINUS_ONE[2],
            GENERIC_MINUS_ONE[3],
            0,
            0,
            0,
            0,
        ]
    {
        return Err(PrimeCertificateError::FactorizationMismatch);
    }

    for (factor, witness) in GENERIC_FACTORS {
        let base = Fp256GenericV1::from_u64_mod(witness);
        if base.pow(&GENERIC_MINUS_ONE) != Fp256GenericV1::ONE {
            return Err(PrimeCertificateError::FermatWitnessFailed(factor));
        }
        let exponent = divide_small(GENERIC_MINUS_ONE, factor).0;
        let residue = base.pow(&exponent).sub(Fp256GenericV1::ONE);
        let canonical = decode_limbs(residue.to_canonical());
        if !gcd_is_one(canonical, GENERIC_MODULUS) {
            return Err(PrimeCertificateError::PocklingtonGcdFailed(factor));
        }
    }
    Ok(())
}

fn mod_pow_u64(mut base: u64, mut exponent: u64, modulus: u64) -> u64 {
    let mut result = 1_u64;
    while exponent != 0 {
        if exponent & 1 == 1 {
            result = ((u128::from(result) * u128::from(base)) % u128::from(modulus)) as u64;
        }
        base = ((u128::from(base) * u128::from(base)) % u128::from(modulus)) as u64;
        exponent >>= 1;
    }
    result
}

fn gcd_u64(mut lhs: u64, mut rhs: u64) -> u64 {
    while rhs != 0 {
        (lhs, rhs) = (rhs, lhs % rhs);
    }
    lhs
}

fn is_prime_u64(value: u64) -> bool {
    if value < 2 {
        return false;
    }
    let mut divisor = 2_u64;
    while divisor * divisor <= value {
        if value / divisor * divisor == value {
            return value == divisor;
        }
        divisor += if divisor == 2 { 1 } else { 2 };
    }
    true
}

fn multiply_small(mut value: [u64; 4], factor: u64) -> Option<[u64; 4]> {
    let mut carry = 0_u64;
    for limb in &mut value {
        let product = u128::from(*limb) * u128::from(factor) + u128::from(carry);
        *limb = product as u64;
        carry = (product >> 64) as u64;
    }
    (carry == 0).then_some(value)
}

fn multiply_256(lhs: [u64; 4], rhs: [u64; 4]) -> [u64; 8] {
    let mut result = [0_u64; 8];
    for (left_index, left_limb) in lhs.iter().copied().enumerate() {
        let mut carry = 0_u64;
        for (right_index, right_limb) in rhs.iter().copied().enumerate() {
            let index = left_index + right_index;
            let product = u128::from(left_limb) * u128::from(right_limb)
                + u128::from(result[index])
                + u128::from(carry);
            result[index] = product as u64;
            carry = (product >> 64) as u64;
        }
        let mut index = left_index + 4;
        while carry != 0 {
            let sum = u128::from(result[index]) + u128::from(carry);
            result[index] = sum as u64;
            carry = (sum >> 64) as u64;
            index += 1;
        }
    }
    result
}

fn divide_small(value: [u64; 4], divisor: u64) -> ([u64; 4], u64) {
    let mut quotient = [0_u64; 4];
    let mut remainder = 0_u128;
    for index in (0..4).rev() {
        let dividend = (remainder << 64) | u128::from(value[index]);
        quotient[index] = (dividend / u128::from(divisor)) as u64;
        remainder = dividend % u128::from(divisor);
    }
    (quotient, remainder as u64)
}

fn gcd_is_one(mut lhs: [u64; 4], mut rhs: [u64; 4]) -> bool {
    if is_zero(lhs) {
        return rhs == [1, 0, 0, 0];
    }
    loop {
        while lhs[0] & 1 == 0 {
            shift_right_one(&mut lhs);
        }
        while rhs[0] & 1 == 0 {
            shift_right_one(&mut rhs);
        }
        match super::cmp_limbs(&lhs, &rhs) {
            Ordering::Equal => return lhs == [1, 0, 0, 0],
            Ordering::Greater => subtract_assign(&mut lhs, rhs),
            Ordering::Less => subtract_assign(&mut rhs, lhs),
        }
    }
}

fn subtract_assign(lhs: &mut [u64; 4], rhs: [u64; 4]) {
    let mut borrow = false;
    for index in 0..4 {
        let (difference, borrow_a) = lhs[index].overflowing_sub(rhs[index]);
        let (difference, borrow_b) = difference.overflowing_sub(u64::from(borrow));
        lhs[index] = difference;
        borrow = borrow_a || borrow_b;
    }
    debug_assert!(!borrow);
}

fn shift_right_one(value: &mut [u64; 4]) {
    let mut carry = 0_u64;
    for limb in value.iter_mut().rev() {
        let next = *limb << 63;
        *limb = (*limb >> 1) | carry;
        carry = next;
    }
}

fn is_zero(value: [u64; 4]) -> bool {
    value == [0; 4]
}

fn bit_length(value: [u64; 4]) -> u32 {
    for index in (0..4).rev() {
        if value[index] != 0 {
            return index as u32 * 64 + (64 - value[index].leading_zeros());
        }
    }
    0
}

fn decode_limbs(bytes: [u8; 32]) -> [u64; 4] {
    let mut out = [0_u64; 4];
    for (index, chunk) in bytes.chunks_exact(8).enumerate() {
        out[index] = u64::from_le_bytes(chunk.try_into().expect("fixed chunk"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maintained_certificates_replay_without_external_oracle() {
        assert_eq!(verify_builtin_prime_certificates(), Ok(()));
    }

    #[test]
    fn exact_factorization_and_small_division_are_consistent() {
        let product = multiply_256(KNOWN_FACTOR_PRODUCT, COFACTOR);
        assert_eq!(&product[..4], &GENERIC_MINUS_ONE);
        assert!(product[4..].iter().all(|limb| *limb == 0));
        for (factor, _) in GENERIC_FACTORS {
            assert_eq!(divide_small(GENERIC_MINUS_ONE, factor).1, 0);
        }
    }
}
