//! Generic algebraic, representation and nominal-type contracts for all built-ins.

#![cfg(feature = "builtin-fields")]

use core::ops::{Add, Mul, Neg, Sub};

use microfield::{
    BinaryPolynomialField, CanonicalEncoding, ExtensionField, F2, Field, Gf2_128V1, Gf2_256AltV1,
    Gf2_256HhV1, Invert, Pow, Square, StaticField,
};

const PRODUCT_SAMPLES: usize = if cfg!(miri) { 1 } else { 96 };
const LAW_SAMPLES: usize = if cfg!(miri) { 1 } else { 32 };
const INVERSION_SAMPLES: usize = if cfg!(miri) { 1 } else { 6 };

trait TestedField:
    Field
    + Square
    + Invert
    + Pow
    + CanonicalEncoding
    + ExtensionField<Base = F2>
    + BinaryPolynomialField
    + StaticField
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Neg<Output = Self>
    + core::fmt::Debug
{
    const TEST_DEGREE: usize;
    const TEST_BYTES: usize;
    const MODULUS: &'static [usize];

    fn decode(bytes: &[u8]) -> Self {
        Self::from_canonical_slice(bytes).expect("a full-width binary value is canonical")
    }

    fn encode(self) -> Vec<u8> {
        self.to_canonical().as_ref().to_vec()
    }
}

macro_rules! tested_field {
    ($field:ty, $degree:expr, $bytes:expr, $modulus:expr) => {
        impl TestedField for $field {
            const TEST_DEGREE: usize = $degree;
            const TEST_BYTES: usize = $bytes;
            const MODULUS: &'static [usize] = $modulus;
        }
    };
}

tested_field!(Gf2_128V1, 128, 16, &[128, 7, 2, 1, 0]);
tested_field!(Gf2_256HhV1, 256, 32, &[256, 10, 5, 2, 0]);
tested_field!(Gf2_256AltV1, 256, 32, &[256, 16, 3, 1, 0]);

#[test]
fn layouts_metadata_and_encodings_are_frozen() {
    assert_eq!(size_of::<Gf2_128V1>(), 16);
    assert_eq!(align_of::<Gf2_128V1>(), 8);
    assert_metadata::<Gf2_128V1>(
        "gf2_128_v1",
        "4825b6d5606e34af32722a4a6a96d04a1e21337be0fb734adb9c69f9b9d77d31",
        "cf819f1bdc3feb90b660251db0a03f0e5313bca2590e749911d7fbf7881985fc",
    );

    assert_eq!(size_of::<Gf2_256HhV1>(), 32);
    assert_eq!(align_of::<Gf2_256HhV1>(), 8);
    assert_metadata::<Gf2_256HhV1>(
        "gf2_256_hh_v1",
        "6b62fea68b968fd4f8c39a4f69b78f714c80858b1d0f667ec5a63d4417b43ca8",
        "f9752213c4cd64f851e6a9e89e4c1d1d557fe067cc6c8dbc9780c227fc8f23e4",
    );

    assert_eq!(size_of::<Gf2_256AltV1>(), 32);
    assert_eq!(align_of::<Gf2_256AltV1>(), 8);
    assert_metadata::<Gf2_256AltV1>(
        "gf2_256_alt_v1",
        "5c78ea2f9ea1b2d59b88bf32e38ae33be4c2f977f0232c4441f7a16e4c9bb54d",
        "5a7699177fffb929db93400084f9fa8495c015bd4eb0da1b247ce538ce831487",
    );

    assert_basis_encoding::<Gf2_128V1>();
    assert_basis_encoding::<Gf2_256HhV1>();
    assert_basis_encoding::<Gf2_256AltV1>();
}

#[test]
fn all_fields_match_independent_polynomial_arithmetic() {
    assert_arithmetic::<Gf2_128V1>(0x243f_6a88_85a3_08d3);
    assert_arithmetic::<Gf2_256HhV1>(0x1319_8a2e_0370_7344);
    assert_arithmetic::<Gf2_256AltV1>(0xa409_3822_299f_31d0);
}

#[test]
fn all_fields_satisfy_the_same_laws_and_fermat_chain() {
    assert_laws::<Gf2_128V1>(0x082e_fa98_ec4e_6c89);
    assert_laws::<Gf2_256HhV1>(0x4528_21e6_38d0_1377);
    assert_laws::<Gf2_256AltV1>(0xbe54_66cf_34e9_0c6c);
}

#[test]
fn all_fields_reduce_arbitrary_polynomial_lengths() {
    assert_streaming_reduction::<Gf2_128V1>(0xc0ac_29b7_c97c_50dd);
    assert_streaming_reduction::<Gf2_256HhV1>(0x3f84_d5b5_b547_0917);
    assert_streaming_reduction::<Gf2_256AltV1>(0x9216_d5d9_8979_fb1b);
}

#[test]
fn equal_cardinality_does_not_imply_equal_presentation() {
    let mut highest = [0; 32];
    highest[31] = 0x80;
    let hh = Gf2_256HhV1::from_canonical(&highest).expect("full width is canonical");
    let alt = Gf2_256AltV1::from_canonical(&highest).expect("full width is canonical");

    assert_ne!(hh.mul_by_x().to_canonical(), alt.mul_by_x().to_canonical());
    assert_ne!(
        Gf2_256HhV1::spec().field_id(),
        Gf2_256AltV1::spec().field_id()
    );
}

fn assert_metadata<F: TestedField>(name: &str, field_id: &str, artifact_id: &str) {
    assert_eq!(F::DEGREE, F::TEST_DEGREE);
    assert_eq!(F::MODULUS_DEGREE, F::TEST_DEGREE);
    assert_eq!(usize::from(F::spec().canonical_bytes()), F::TEST_BYTES);
    assert_eq!(F::spec().name(), name);
    assert_eq!(F::spec().field_id().to_string(), field_id);
    assert_eq!(F::spec().artifact_id().to_string(), artifact_id);
}

fn assert_basis_encoding<F: TestedField>() {
    for bit in 0..F::TEST_DEGREE {
        let mut bytes = vec![0; F::TEST_BYTES];
        bytes[bit / 8] = 1 << (bit % 8);
        assert_eq!(F::decode(&bytes).encode(), bytes);
    }
    for invalid in [0, F::TEST_BYTES - 1, F::TEST_BYTES + 1] {
        assert!(F::from_canonical_slice(&vec![0; invalid]).is_err());
    }
}

fn assert_arithmetic<F: TestedField>(mut state: u64) {
    for _ in 0..PRODUCT_SAMPLES {
        let lhs_bytes = deterministic_bytes(&mut state, F::TEST_BYTES);
        let rhs_bytes = deterministic_bytes(&mut state, F::TEST_BYTES);
        let lhs = F::decode(&lhs_bytes);
        let rhs = F::decode(&rhs_bytes);

        assert_eq!(
            (lhs * rhs).encode(),
            reference_multiply(&lhs_bytes, &rhs_bytes, F::TEST_DEGREE, F::MODULUS)
        );
        assert_eq!(
            lhs.square().encode(),
            reference_multiply(&lhs_bytes, &lhs_bytes, F::TEST_DEGREE, F::MODULUS)
        );
        assert_eq!(lhs.square(), lhs * lhs);

        let mut x = vec![0; F::TEST_BYTES];
        x[0] = 2;
        assert_eq!(
            lhs.mul_by_x().encode(),
            reference_multiply(&lhs_bytes, &x, F::TEST_DEGREE, F::MODULUS)
        );
    }
}

fn assert_laws<F: TestedField>(mut state: u64) {
    for _ in 0..LAW_SAMPLES {
        let a = F::decode(&deterministic_bytes(&mut state, F::TEST_BYTES));
        let b = F::decode(&deterministic_bytes(&mut state, F::TEST_BYTES));
        let c = F::decode(&deterministic_bytes(&mut state, F::TEST_BYTES));

        assert_eq!(a + F::ZERO, a);
        assert_eq!(a + a, F::ZERO);
        assert_eq!(a - b, a + b);
        assert_eq!(-a, a);
        assert_eq!(a * F::ONE, a);
        assert_eq!(a * F::ZERO, F::ZERO);
        assert_eq!(a + b, b + a);
        assert_eq!(a * b, b * a);
        assert_eq!((a + b) + c, a + (b + c));
        assert_eq!((a * b) * c, a * (b * c));
        assert_eq!(a * (b + c), a * b + a * c);
        assert_eq!((a + b).trace(), a.trace() + b.trace());
        assert_eq!((a * b).norm(), a.norm() * b.norm());
        assert_eq!(a.frobenius(F::TEST_DEGREE), a);
    }

    assert_eq!(F::ZERO.invert(), None);
    for _ in 0..INVERSION_SAMPLES {
        let mut bytes = deterministic_bytes(&mut state, F::TEST_BYTES);
        bytes[0] |= 1;
        let value = F::decode(&bytes);
        let inverse = value.invert().expect("the sample is nonzero");
        assert_eq!(value * inverse, F::ONE);

        if !cfg!(miri) {
            let mut inverse_exponent = vec![u64::MAX; F::TEST_DEGREE / 64];
            inverse_exponent[0] -= 1;
            assert_eq!(value.pow(&inverse_exponent), inverse);
            assert_eq!(value.pow(&vec![u64::MAX; F::TEST_DEGREE / 64]), F::ONE);
        }
    }
}

fn assert_streaming_reduction<F: TestedField>(mut state: u64) {
    for length in [
        0,
        1,
        F::TEST_BYTES - 1,
        F::TEST_BYTES,
        F::TEST_BYTES + 1,
        F::TEST_BYTES * 2,
        F::TEST_BYTES * 2 + 17,
    ] {
        let bytes = deterministic_bytes(&mut state, length);
        assert_eq!(
            F::from_polynomial_bytes_mod(&bytes).encode(),
            reference_reduce_bytes(&bytes, F::TEST_BYTES, F::MODULUS)
        );
    }
}

fn reference_multiply(lhs: &[u8], rhs: &[u8], degree: usize, modulus: &[usize]) -> Vec<u8> {
    let mut product = vec![false; degree * 2];
    for lhs_bit in 0..degree {
        if read_bit(lhs, lhs_bit) {
            for rhs_bit in 0..degree {
                if read_bit(rhs, rhs_bit) {
                    product[lhs_bit + rhs_bit] ^= true;
                }
            }
        }
    }
    for source in (degree..product.len()).rev() {
        if product[source] {
            for &exponent in modulus {
                product[source - degree + exponent] ^= true;
            }
        }
    }

    let mut output = vec![0; degree / 8];
    for (index, coefficient) in product[..degree].iter().copied().enumerate() {
        if coefficient {
            output[index / 8] |= 1 << (index % 8);
        }
    }
    output
}

fn reference_reduce_bytes(bytes: &[u8], width: usize, modulus: &[usize]) -> Vec<u8> {
    let mut result = vec![0; width];
    for byte in bytes.iter().rev().copied() {
        for bit in (0..u8::BITS).rev() {
            let overflow = result[width - 1] >> 7;
            let mut carry = 0;
            for output in &mut result {
                let next = *output >> 7;
                *output = (*output << 1) | carry;
                carry = next;
            }
            if overflow == 1 {
                for &exponent in &modulus[1..] {
                    result[exponent / 8] ^= 1 << (exponent % 8);
                }
            }
            result[0] ^= (byte >> bit) & 1;
        }
    }
    result
}

fn read_bit(bytes: &[u8], bit: usize) -> bool {
    bytes[bit / 8] & (1 << (bit % 8)) != 0
}

fn deterministic_bytes(state: &mut u64, length: usize) -> Vec<u8> {
    let mut output = vec![0; length];
    for chunk in output.chunks_mut(8) {
        let word = next_u64(state).to_le_bytes();
        chunk.copy_from_slice(&word[..chunk.len()]);
    }
    output
}

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    (*state).wrapping_mul(0x2545_f491_4f6c_dd1d)
}
