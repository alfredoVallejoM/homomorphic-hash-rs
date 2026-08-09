//! RC.7 property contracts for every maintained static field family.

#![cfg(all(feature = "builtin-fields", feature = "prime-fields"))]

use microfield::{
    BinaryPolynomialField, CanonicalEncoding, Field, Fp251V1, Fp256GenericV1, FpGoldilocks64V1,
    Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, Invert, PrimeField, Square,
};
use proptest::prelude::*;

macro_rules! binary_field_properties {
    ($module:ident, $field:ty, $bytes:expr) => {
        mod $module {
            use super::*;

            proptest! {
                #![proptest_config(ProptestConfig::with_cases(1_024))]

                #[test]
                fn field_laws_and_canonical_round_trip(
                    a_bytes in any::<[u8; $bytes]>(),
                    b_bytes in any::<[u8; $bytes]>(),
                    c_bytes in any::<[u8; $bytes]>(),
                ) {
                    let a = <$field>::from_canonical(&a_bytes).expect("binary encoding is total");
                    let b = <$field>::from_canonical(&b_bytes).expect("binary encoding is total");
                    let c = <$field>::from_canonical(&c_bytes).expect("binary encoding is total");

                    let canonical = a.to_canonical();
                    prop_assert_eq!(canonical, a_bytes);
                    prop_assert_eq!(a + b, b + a);
                    prop_assert_eq!((a + b) + c, a + (b + c));
                    prop_assert_eq!(a * b, b * a);
                    prop_assert_eq!((a * b) * c, a * (b * c));
                    prop_assert_eq!(a * (b + c), (a * b) + (a * c));
                    prop_assert_eq!(a + a, <$field>::ZERO);
                    prop_assert_eq!(a.square(), a * a);

                    if a == <$field>::ZERO {
                        prop_assert_eq!(a.invert(), None);
                    } else {
                        let inverse = a.invert().expect("nonzero field element is invertible");
                        prop_assert_eq!(a * inverse, <$field>::ONE);
                        prop_assert_eq!(inverse.invert(), Some(a));
                    }
                }

                #[test]
                fn arbitrary_polynomial_reduction_is_deterministic_and_canonical(
                    bytes in prop::collection::vec(any::<u8>(), 0..=3 * $bytes),
                ) {
                    let first = <$field>::from_polynomial_bytes_mod(&bytes);
                    let second = <$field>::from_polynomial_bytes_mod(&bytes);
                    prop_assert_eq!(first, second);
                    let canonical = first.to_canonical();
                    prop_assert_eq!(
                        <$field>::from_canonical(&canonical),
                        Ok(first),
                    );
                }
            }
        }
    };
}

binary_field_properties!(gf2_128, Gf2_128V1, 16);
binary_field_properties!(gf2_256_hh, Gf2_256HhV1, 32);
binary_field_properties!(gf2_256_alt, Gf2_256AltV1, 32);

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_048))]

    #[test]
    fn fp251_matches_integer_arithmetic(a in 0_u16..251, b in 0_u16..251, c in 0_u16..251) {
        let fa = Fp251V1::from_canonical(&[u8::try_from(a).unwrap()]).unwrap();
        let fb = Fp251V1::from_canonical(&[u8::try_from(b).unwrap()]).unwrap();
        let fc = Fp251V1::from_canonical(&[u8::try_from(c).unwrap()]).unwrap();

        prop_assert_eq!((fa + fb).to_canonical()[0], u8::try_from((a + b) % 251).unwrap());
        prop_assert_eq!((fa * fb).to_canonical()[0], u8::try_from((a * b) % 251).unwrap());
        prop_assert_eq!(fa * (fb + fc), (fa * fb) + (fa * fc));
        prop_assert_eq!(fa.square(), fa * fa);
        if fa == Fp251V1::ZERO {
            prop_assert_eq!(fa.invert(), None);
        } else {
            prop_assert_eq!(fa * fa.invert().unwrap(), Fp251V1::ONE);
        }
    }

    #[test]
    fn goldilocks_matches_u128_and_field_laws(a in any::<u64>(), b in any::<u64>(), c in any::<u64>()) {
        let modulus = FpGoldilocks64V1::MODULUS;
        let a = a % modulus;
        let b = b % modulus;
        let c = c % modulus;
        let fa = FpGoldilocks64V1::from_canonical(&a.to_le_bytes()).unwrap();
        let fb = FpGoldilocks64V1::from_canonical(&b.to_le_bytes()).unwrap();
        let fc = FpGoldilocks64V1::from_canonical(&c.to_le_bytes()).unwrap();
        let modulus128 = u128::from(modulus);

        prop_assert_eq!(
            u64::from_le_bytes((fa * fb).to_canonical()),
            u64::try_from((u128::from(a) * u128::from(b)) % modulus128).unwrap(),
        );
        prop_assert_eq!(fa * (fb + fc), (fa * fb) + (fa * fc));
        prop_assert_eq!(fa.square(), fa * fa);
        if fa != FpGoldilocks64V1::ZERO {
            prop_assert_eq!(fa * fa.invert().unwrap(), FpGoldilocks64V1::ONE);
        }
    }

    #[test]
    fn fp256_generic_obeys_laws_and_encoding(
        a_bytes in any::<[u8; 32]>(),
        b_bytes in any::<[u8; 32]>(),
        c_bytes in any::<[u8; 32]>(),
    ) {
        let a = Fp256GenericV1::from_bytes_mod_order(&a_bytes);
        let b = Fp256GenericV1::from_bytes_mod_order(&b_bytes);
        let c = Fp256GenericV1::from_bytes_mod_order(&c_bytes);

        prop_assert_eq!(a + b, b + a);
        prop_assert_eq!((a + b) + c, a + (b + c));
        prop_assert_eq!(a * b, b * a);
        prop_assert_eq!((a * b) * c, a * (b * c));
        prop_assert_eq!(a * (b + c), (a * b) + (a * c));
        prop_assert_eq!(a.square(), a * a);
        prop_assert_eq!(
            Fp256GenericV1::from_canonical(&a.to_canonical()),
            Ok(a),
        );
        if a != Fp256GenericV1::ZERO {
            prop_assert_eq!(a * a.invert().unwrap(), Fp256GenericV1::ONE);
        }
    }
}
