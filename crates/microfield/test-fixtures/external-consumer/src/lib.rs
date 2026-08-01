//! External consumer proving the build-time factory contract.

#![no_std]

#[cfg(test)]
extern crate std;

/// Mixing independently generated nominal fields is a type error.
///
/// ```compile_fail
/// use microfield_external_consumer::{Gf2_9Fixture, Gf2_233Fixture};
/// let small = Gf2_9Fixture::default();
/// let large = Gf2_233Fixture::default();
/// let _: Gf2_9Fixture = small + large;
/// ```
///
/// Private limbs cannot be constructed or read by a consumer.
///
/// ```compile_fail
/// use microfield_external_consumer::Gf2_9Fixture;
/// let value = Gf2_9Fixture([1]);
/// ```
pub struct CompileFailContracts;

include!(concat!(env!("OUT_DIR"), "/gf2_9_fixture.rs"));

mod generated_233 {
    include!(concat!(env!("OUT_DIR"), "/gf2_233_fixture.rs"));
}

pub use generated_233::Gf2_233Fixture;

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use microfield::{
        BinaryPolynomialField, CanonicalEncoding, DecodeError, Engine, ExtensionField, Field,
        Invert, Pow, Square, StaticField,
    };
    use std::format;

    use super::{Gf2_9Fixture, Gf2_233Fixture};

    const MODULUS: u32 = (1 << 9) | (1 << 4) | 1;

    fn element(value: u16) -> Gf2_9Fixture {
        Gf2_9Fixture::from_canonical(&value.to_le_bytes()).expect("value is below 2^9")
    }

    fn reference_multiply(lhs: u16, rhs: u16) -> u16 {
        let mut product = 0_u32;
        for bit in 0..9 {
            if (rhs >> bit) & 1 != 0 {
                product ^= u32::from(lhs) << bit;
            }
        }
        for bit in (9..=16).rev() {
            if product & (1 << bit) != 0 {
                product ^= MODULUS << (bit - 9);
            }
        }
        product as u16
    }

    fn hex_233(source: &str) -> Gf2_233Fixture {
        assert_eq!(source.len(), 60);
        let mut bytes = [0_u8; 30];
        for (index, pair) in source.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
        }
        Gf2_233Fixture::from_canonical(&bytes).expect("committed Sage value is canonical")
    }

    fn hex_nibble(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            _ => panic!("invalid committed hexadecimal"),
        }
    }

    #[test]
    fn exhaustive_encoding_square_and_inverse_laws() {
        for raw in 0_u16..512 {
            let value = element(raw);
            assert_eq!(value.to_canonical(), raw.to_le_bytes());
            assert_eq!(value.square(), element(reference_multiply(raw, raw)));
            assert_eq!(value.frobenius(9), value);
            if raw == 0 {
                assert_eq!(value.invert(), None);
            } else {
                assert_eq!(
                    value * value.invert().expect("non-zero inverse"),
                    Gf2_9Fixture::ONE
                );
            }
        }
    }

    #[test]
    fn multiplication_matches_an_independent_model() {
        for lhs in 0_u16..512 {
            for rhs in (0_u16..512).step_by(17) {
                assert_eq!(
                    element(lhs) * element(rhs),
                    element(reference_multiply(lhs, rhs)),
                    "lhs={lhs:#x}, rhs={rhs:#x}"
                );
            }
        }
    }

    #[test]
    fn polynomial_reduction_accepts_inputs_far_wider_than_the_field() {
        let bytes = [0x35, 0xa7, 0xfe, 0x18, 0x91, 0x44, 0xff];
        let mut expected = Gf2_9Fixture::ZERO;
        for byte in bytes.iter().rev().copied() {
            for bit in (0..8).rev() {
                expected = expected.mul_by_x();
                if (byte >> bit) & 1 != 0 {
                    expected += Gf2_9Fixture::ONE;
                }
            }
        }
        assert_eq!(Gf2_9Fixture::from_polynomial_bytes_mod(&bytes), expected);
    }

    #[test]
    fn decoder_rejects_length_and_every_nonzero_padding_pattern() {
        assert_eq!(
            Gf2_9Fixture::from_canonical_slice(&[0]),
            Err(DecodeError::LengthMismatch {
                expected: 2,
                actual: 1
            })
        );
        for high in 2_u8..=u8::MAX {
            assert_eq!(
                Gf2_9Fixture::from_canonical(&[0, high]),
                Err(DecodeError::NonCanonicalValue)
            );
        }
    }

    #[test]
    fn external_field_uses_the_public_batch_facade() {
        let engine = Engine::<Gf2_9Fixture>::portable();
        let lhs = [element(3), element(0x101), element(0x1ff)];
        let rhs = [element(7), element(0x55), element(0x101)];
        let mut out = [Gf2_9Fixture::ZERO; 3];
        engine
            .mul_into(&mut out, &lhs, &rhs)
            .expect("matching lengths");
        for index in 0..out.len() {
            assert_eq!(out[index], lhs[index] * rhs[index]);
        }
    }

    #[test]
    fn layout_metadata_and_formatting_are_stable() {
        assert_eq!(size_of::<Gf2_9Fixture>(), 8);
        assert_eq!(align_of::<Gf2_9Fixture>(), 8);
        assert_eq!(Gf2_9Fixture::spec().degree(), 9);
        assert_eq!(Gf2_9Fixture::spec().canonical_bytes(), 2);
        assert_eq!(Gf2_9Fixture::spec().name(), "gf2_9_fixture");
        assert_eq!(format!("{}", element(1)), "0001");
        assert_eq!(format!("{:?}", element(1)), "Gf2_9Fixture(0x0001)");
    }

    #[test]
    fn multi_limb_degree_233_field_obeys_boundary_and_field_laws() {
        let mut left_bytes = [0_u8; 30];
        let mut right_bytes = [0_u8; 30];
        for index in 0..30 {
            left_bytes[index] = (index as u8).wrapping_mul(37).wrapping_add(11);
            right_bytes[index] = (index as u8).wrapping_mul(91).wrapping_add(7);
        }
        left_bytes[29] &= 1;
        right_bytes[29] &= 1;
        let left = Gf2_233Fixture::from_canonical(&left_bytes).expect("canonical sample");
        let right = Gf2_233Fixture::from_canonical(&right_bytes).expect("canonical sample");

        assert_eq!(left * Gf2_233Fixture::ONE, left);
        assert_eq!(left * right, right * left);
        assert_eq!(left.square(), left * left);
        assert_eq!(
            left * left.invert().expect("sample is non-zero"),
            Gf2_233Fixture::ONE
        );
        assert_eq!(left.frobenius(233), left);

        let mut highest_basis = [0_u8; 30];
        highest_basis[29] = 1;
        let wrapped = Gf2_233Fixture::from_canonical(&highest_basis)
            .expect("x^232 is canonical")
            .mul_by_x()
            .to_canonical();
        let mut expected_tail = [0_u8; 30];
        expected_tail[0] = 1;
        expected_tail[9] = 1 << 2;
        assert_eq!(wrapped, expected_tail);

        assert_eq!(size_of::<Gf2_233Fixture>(), 32);
        assert_eq!(align_of::<Gf2_233Fixture>(), 8);
        assert_eq!(Gf2_233Fixture::spec().degree(), 233);
        let mut padding = [0_u8; 30];
        padding[29] = 2;
        assert_eq!(
            Gf2_233Fixture::from_canonical(&padding),
            Err(DecodeError::NonCanonicalValue)
        );
    }

    #[test]
    fn degree_233_matches_committed_sage_10_7_vectors() {
        // Generated with tools/sage/generate_vectors.sage under laboratorio_np.
        let lhs = hex_233("cd240c95c64a5798e1632bccb09f98c5667208e4dbf6180232d0a67d2701");
        let rhs = hex_233("51b3180bb22ec32b52fee524ba63353058344218073f87761795005e1c00");
        assert_eq!(
            lhs + rhs,
            hex_233("9c97149e746494b3b39dcee80afcadf53e464afcdcc99f742545a6233b01")
        );
        assert_eq!(
            lhs * rhs,
            hex_233("94c5b35faad8c45d7bb5bd2a2434366d918c3c4f91523222ffb8677a9900")
        );
        assert_eq!(
            lhs.square(),
            hex_233("f458691a1ed30e7410cdcc5644a3279ed546213ecd2650f022efd74bca01")
        );
        assert_eq!(
            lhs.invert().expect("non-zero Sage operand"),
            hex_233("3b0152cfc484bd2414dd08e49e50590115a31db9987ff3b39dc389a0d301")
        );
        assert_eq!(
            rhs.pow(&[65_537]),
            hex_233("0a536c6545f7739f0b7e042b3a7c4c1dedc9a5fd89ca662aed2abbff5800")
        );
        assert_eq!(
            lhs.mul_by_x(),
            hex_233("9b49182a8d95ae30c3c35698613f318bcde410c8b7ed310464a04dfb4e00")
        );
    }
}
