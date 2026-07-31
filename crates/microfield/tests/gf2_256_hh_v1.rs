//! Deep algebraic and representation contracts for the first portable field.

#![cfg(feature = "builtin-fields")]

use microfield::{
    BinaryPolynomialField, CanonicalEncoding, DecodeError, ExtensionField, F2, Field, Gf2_256HhV1,
    Invert, Pow, Square, StaticField,
};

const DEGREE: usize = 256;
const BYTES: usize = 32;
const MODULUS: [usize; 5] = [256, 10, 5, 2, 0];
const PRODUCT_SAMPLES: usize = if cfg!(miri) { 1 } else { 128 };
const LAW_SAMPLES: usize = if cfg!(miri) { 1 } else { 48 };
const INVERSION_SAMPLES: usize = if cfg!(miri) { 1 } else { 8 };
const EXTENSION_SAMPLES: usize = if cfg!(miri) { 1 } else { 6 };

#[test]
fn public_contract_layout_metadata_and_formatting_are_frozen() {
    fn assert_contract<F>()
    where
        F: Field
            + Square
            + Invert
            + Pow
            + CanonicalEncoding<Repr = [u8; 32]>
            + ExtensionField<Base = F2>
            + BinaryPolynomialField
            + StaticField
            + Send
            + Sync,
    {
    }
    assert_contract::<Gf2_256HhV1>();

    assert_eq!(size_of::<Gf2_256HhV1>(), 32);
    assert_eq!(align_of::<Gf2_256HhV1>(), 8);
    assert_eq!(Gf2_256HhV1::DEGREE, 256);
    assert_eq!(Gf2_256HhV1::MODULUS_DEGREE, 256);
    assert_eq!(
        Gf2_256HhV1::spec().field_id().to_string(),
        "6b62fea68b968fd4f8c39a4f69b78f714c80858b1d0f667ec5a63d4417b43ca8"
    );
    assert_eq!(
        Gf2_256HhV1::spec().artifact_id().to_string(),
        "61116d0c70d490cb8d210d35dddff0f638d75d7e08b6e8d138197594d42334cb"
    );
    assert_eq!(Gf2_256HhV1::spec().name(), "gf2_256_hh_v1");
    assert_eq!(Gf2_256HhV1::spec().characteristic(), 2);
    assert_eq!(Gf2_256HhV1::spec().canonical_bytes(), 32);

    assert_eq!(Gf2_256HhV1::ZERO.to_string(), "0".repeat(64));
    assert_eq!(
        format!("{:?}", Gf2_256HhV1::ONE),
        format!("Gf2_256HhV1(0x{}1)", "0".repeat(63))
    );
}

#[test]
fn canonical_encoding_is_a_bijection_over_all_basis_bits() {
    for bit in 0..DEGREE {
        let mut bytes = [0; BYTES];
        bytes[bit / 8] = 1 << (bit % 8);
        let value = Gf2_256HhV1::from_canonical(&bytes).expect("all 256 bits are canonical");
        assert_eq!(value.to_canonical(), bytes);
    }

    for length in [0, 1, 31, 33, 64] {
        let bytes = vec![0; length];
        assert_eq!(
            Gf2_256HhV1::from_canonical_slice(&bytes),
            Err(DecodeError::LengthMismatch {
                expected: BYTES,
                actual: length,
            })
        );
    }
    assert_eq!(
        Gf2_256HhV1::from_canonical_slice(&[0xff; BYTES])
            .expect("the degree fills the complete representation")
            .to_canonical(),
        [0xff; BYTES]
    );
}

#[test]
fn modulus_boundary_and_mul_by_x_are_exact() {
    let mut highest = [0; BYTES];
    highest[31] = 0x80;
    let highest = decode(highest);
    let expected_tail = decode({
        let mut bytes = [0; BYTES];
        bytes[0] = 0x25;
        bytes[1] = 0x04;
        bytes
    });

    assert_eq!(highest.mul_by_x(), expected_tail);

    let mut x_128 = [0; BYTES];
    x_128[16] = 1;
    let x_128 = decode(x_128);
    assert_eq!(x_128 * x_128, expected_tail);
}

#[test]
fn portable_product_square_and_shift_match_bit_polynomial_division() {
    let dense = [0xff; BYTES];
    assert_eq!(
        (decode(dense) * decode(dense)).to_canonical(),
        reference_multiply(&dense, &dense)
    );

    let mut state = 0x9e37_79b9_7f4a_7c15;
    for _ in 0..PRODUCT_SAMPLES {
        let lhs_bytes = deterministic_bytes(&mut state);
        let rhs_bytes = deterministic_bytes(&mut state);
        let lhs = decode(lhs_bytes);
        let rhs = decode(rhs_bytes);

        assert_eq!(
            (lhs * rhs).to_canonical(),
            reference_multiply(&lhs_bytes, &rhs_bytes)
        );
        assert_eq!(
            lhs.square().to_canonical(),
            reference_multiply(&lhs_bytes, &lhs_bytes)
        );
        assert_eq!(lhs.square(), lhs * lhs);
        assert_eq!(
            lhs.mul_by_x().to_canonical(),
            reference_multiply(&lhs_bytes, &basis_x())
        );
    }
}

#[test]
fn deterministic_samples_satisfy_field_laws_and_fermat_inversion() {
    let mut state = 0xd1b5_4a32_d192_ed03;
    for _ in 0..LAW_SAMPLES {
        let a = decode(deterministic_bytes(&mut state));
        let b = decode(deterministic_bytes(&mut state));
        let c = decode(deterministic_bytes(&mut state));

        assert_eq!(a + Gf2_256HhV1::ZERO, a);
        assert_eq!(a + a, Gf2_256HhV1::ZERO);
        assert_eq!(a - b, a + b);
        assert_eq!(-a, a);
        assert_eq!(a * Gf2_256HhV1::ONE, a);
        assert_eq!(a * Gf2_256HhV1::ZERO, Gf2_256HhV1::ZERO);
        assert_eq!(a + b, b + a);
        assert_eq!(a * b, b * a);
        assert_eq!((a + b) + c, a + (b + c));
        assert_eq!((a * b) * c, a * (b * c));
        assert_eq!(a * (b + c), a * b + a * c);
    }

    assert_eq!(Gf2_256HhV1::ZERO.invert(), None);
    for _ in 0..INVERSION_SAMPLES {
        let value = nonzero(deterministic_bytes(&mut state));
        let inverse = value.invert().expect("sample is nonzero");
        assert_eq!(value * inverse, Gf2_256HhV1::ONE);
        if !cfg!(miri) {
            assert_eq!(inverse.invert(), Some(value));
            assert_eq!(
                value.pow(&[u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX]),
                inverse
            );
            assert_eq!(
                value.pow(&[u64::MAX, u64::MAX, u64::MAX, u64::MAX]),
                Gf2_256HhV1::ONE
            );
        }
    }
}

#[test]
fn frobenius_trace_and_norm_obey_extension_field_laws() {
    let mut state = 0xa076_1d64_78bd_642f;
    for _ in 0..EXTENSION_SAMPLES {
        let a = decode(deterministic_bytes(&mut state));
        let b = decode(deterministic_bytes(&mut state));

        assert_eq!(a.frobenius(0), a);
        assert_eq!(a.frobenius(256), a);
        assert_eq!(a.frobenius(257), a.square());
        assert_eq!((a + b).frobenius(17), a.frobenius(17) + b.frobenius(17));
        assert_eq!((a + b).trace(), a.trace() + b.trace());
        assert_eq!((a * b).norm(), a.norm() * b.norm());
    }
    assert_eq!(Gf2_256HhV1::ZERO.norm(), F2::ZERO);
    assert_eq!(Gf2_256HhV1::ONE.norm(), F2::ONE);
    assert_eq!(Gf2_256HhV1::ZERO.trace(), F2::ZERO);
    assert_eq!(Gf2_256HhV1::ONE.trace(), F2::ZERO);
}

#[test]
fn arbitrary_polynomial_bytes_reduce_without_truncation() {
    let mut state = 0xe703_7ed1_a0b4_28db;
    for length in [0, 1, 31, 32, 33, 63, 64, 97] {
        let mut bytes = vec![0; length];
        for byte in &mut bytes {
            *byte = next_u64(&mut state).to_le_bytes()[0];
        }
        assert_eq!(
            Gf2_256HhV1::from_polynomial_bytes_mod(&bytes).to_canonical(),
            reference_reduce_bytes(&bytes)
        );
    }
}

fn decode(bytes: [u8; BYTES]) -> Gf2_256HhV1 {
    Gf2_256HhV1::from_canonical(&bytes).expect("all 256-bit inputs are canonical")
}

fn nonzero(mut bytes: [u8; BYTES]) -> Gf2_256HhV1 {
    bytes[0] |= 1;
    decode(bytes)
}

fn basis_x() -> [u8; BYTES] {
    let mut bytes = [0; BYTES];
    bytes[0] = 2;
    bytes
}

fn deterministic_bytes(state: &mut u64) -> [u8; BYTES] {
    let mut output = [0; BYTES];
    for chunk in output.chunks_exact_mut(8) {
        chunk.copy_from_slice(&next_u64(state).to_le_bytes());
    }
    output
}

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    (*state).wrapping_mul(0x2545_f491_4f6c_dd1d)
}

fn reference_multiply(lhs: &[u8; BYTES], rhs: &[u8; BYTES]) -> [u8; BYTES] {
    let mut product = [false; DEGREE * 2];
    for lhs_bit in 0..DEGREE {
        if read_bit(lhs, lhs_bit) {
            for rhs_bit in 0..DEGREE {
                if read_bit(rhs, rhs_bit) {
                    product[lhs_bit + rhs_bit] ^= true;
                }
            }
        }
    }
    reference_reduce_bits(product)
}

fn reference_reduce_bytes(bytes: &[u8]) -> [u8; BYTES] {
    let mut result = [0; BYTES];
    for byte in bytes.iter().rev().copied() {
        for bit in (0..u8::BITS).rev() {
            result = reference_mul_by_x(result);
            result[0] ^= (byte >> bit) & 1;
        }
    }
    result
}

fn reference_mul_by_x(mut value: [u8; BYTES]) -> [u8; BYTES] {
    let overflow = value[BYTES - 1] >> 7;
    let mut carry = 0;
    for byte in &mut value {
        let next_carry = *byte >> 7;
        *byte = (*byte << 1) | carry;
        carry = next_carry;
    }
    if overflow == 1 {
        value[0] ^= 0x25;
        value[1] ^= 0x04;
    }
    value
}

fn reference_reduce_bits(mut bits: [bool; DEGREE * 2]) -> [u8; BYTES] {
    for source in (DEGREE..bits.len()).rev() {
        if bits[source] {
            for exponent in MODULUS {
                bits[source - DEGREE + exponent] ^= true;
            }
        }
    }

    let mut output = [0; BYTES];
    for (index, value) in bits[..DEGREE].iter().copied().enumerate() {
        if value {
            output[index / 8] |= 1 << (index % 8);
        }
    }
    output
}

fn read_bit(bytes: &[u8], bit: usize) -> bool {
    bytes[bit / 8] & (1 << (bit % 8)) != 0
}
