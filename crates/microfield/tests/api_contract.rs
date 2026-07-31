//! Public capability and encoding contract tests.

use microfield::{CanonicalEncoding, DecodeError, F2, Field, FieldId, Invert, Pow, Square};

fn assert_field_contract<F>()
where
    F: Field + Square + Invert + Pow + Send + Sync,
{
}

#[test]
fn f2_satisfies_the_public_capabilities() {
    assert_field_contract::<F2>();

    assert_eq!(F2::ONE + F2::ONE, F2::ZERO);
    assert_eq!(F2::ONE * F2::ONE, F2::ONE);
    assert_eq!(F2::ZERO.invert(), None);
    assert_eq!(F2::ONE.invert(), Some(F2::ONE));
    assert_eq!(F2::ZERO.pow(&[]), F2::ONE);
    assert_eq!(F2::ONE.pow(&[u64::MAX]), F2::ONE);
}

#[test]
fn f2_encoding_is_strictly_canonical() {
    assert_eq!(F2::from_canonical(&[0]), Ok(F2::ZERO));
    assert_eq!(F2::from_canonical(&[1]), Ok(F2::ONE));
    assert_eq!(
        F2::from_canonical(&[2]),
        Err(DecodeError::NonCanonicalValue)
    );
    assert_eq!(
        F2::from_canonical_slice(&[]),
        Err(DecodeError::LengthMismatch {
            expected: 1,
            actual: 0,
        })
    );
    assert_eq!(F2::ONE.to_canonical(), [1]);
}

#[test]
fn field_id_formats_without_allocation_contracts() {
    let id = FieldId::from_bytes([0xab; 32]);
    assert_eq!(id.to_string(), "ab".repeat(32));
    assert_eq!(id.into_bytes(), [0xab; 32]);
}

#[test]
fn f2_exhaustively_satisfies_all_field_laws() {
    let elements = [F2::ZERO, F2::ONE];
    for a in elements {
        assert_eq!(a + F2::ZERO, a);
        assert_eq!(a * F2::ONE, a);
        assert_eq!(a * F2::ZERO, F2::ZERO);
        assert_eq!(a + a, F2::ZERO);
        assert_eq!(-a, a);
        assert_eq!(a.square(), a);
        for b in elements {
            assert_eq!(a + b, b + a);
            assert_eq!(a * b, b * a);
            for c in elements {
                assert_eq!((a + b) + c, a + (b + c));
                assert_eq!((a * b) * c, a * (b * c));
                assert_eq!(a * (b + c), a * b + a * c);
            }
        }
    }
}

#[test]
fn f2_pow_distinguishes_zero_from_positive_exponents() {
    for zero_exponent in [&[][..], &[0][..], &[0, 0][..]] {
        assert_eq!(F2::ZERO.pow(zero_exponent), F2::ONE);
        assert_eq!(F2::ONE.pow(zero_exponent), F2::ONE);
    }
    for positive_exponent in [&[1][..], &[2][..], &[0, 1][..], &[u64::MAX, u64::MAX][..]] {
        assert_eq!(F2::ZERO.pow(positive_exponent), F2::ZERO);
        assert_eq!(F2::ONE.pow(positive_exponent), F2::ONE);
    }
}

#[test]
fn f2_decoder_accepts_exactly_two_of_all_byte_values() {
    for byte in u8::MIN..=u8::MAX {
        let decoded = F2::from_canonical(&[byte]);
        match byte {
            0 => assert_eq!(decoded, Ok(F2::ZERO)),
            1 => assert_eq!(decoded, Ok(F2::ONE)),
            _ => assert_eq!(decoded, Err(DecodeError::NonCanonicalValue)),
        }
    }
}

#[test]
fn f2_value_layout_and_formatting_are_stable() {
    assert_eq!(core::mem::size_of::<F2>(), 1);
    assert_eq!(core::mem::align_of::<F2>(), 1);
    assert_eq!(F2::ZERO.to_string(), "0");
    assert_eq!(F2::ONE.to_string(), "1");
    assert_eq!(format!("{:?}", F2::ZERO), "F2(0)");
    assert_eq!(format!("{:?}", F2::ONE), "F2(1)");
}
