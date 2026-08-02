//! Algebraic, encoding and independent-oracle tests for maintained prime fields.

#![cfg(feature = "prime-fields")]

use microfield::{
    BatchInvertWorkspace, BitMaskViewMut, CanonicalEncoding, Engine, Field, Fp251V1,
    Fp256GenericV1, FpGoldilocks64V1, Invert, Pow, PrimeField, PrimeReductionKind,
    PrimeRepresentationKind, Square, SquareRootField, StaticField,
};

#[test]
fn fp251_is_exhaustively_correct_and_rejects_noncanonical_bytes() {
    let limit = if cfg!(miri) { 17 } else { 251 };
    for left in 0_u16..limit {
        let left_byte = u8::try_from(left).unwrap();
        let a = Fp251V1::from_canonical(&[left_byte]).unwrap();
        assert_eq!(a.to_canonical(), [left_byte]);
        assert_eq!(
            a.square().to_canonical()[0],
            u8::try_from((left * left) % 251).unwrap()
        );
        if left == 0 {
            assert_eq!(a.invert(), None);
        } else {
            let inverse = a.invert().unwrap();
            assert_eq!(a * inverse, Fp251V1::ONE);
        }
        for right in 0_u16..limit {
            let b = Fp251V1::from_canonical(&[u8::try_from(right).unwrap()]).unwrap();
            assert_eq!(
                (a + b).to_canonical()[0],
                u8::try_from((left + right) % 251).unwrap()
            );
            assert_eq!(
                (a - b).to_canonical()[0],
                u8::try_from((left + 251 - right) % 251).unwrap()
            );
            assert_eq!(
                (a * b).to_canonical()[0],
                u8::try_from((left * right) % 251).unwrap()
            );
        }
    }
    for byte in 251_u8..=255 {
        assert!(Fp251V1::from_canonical(&[byte]).is_err());
    }
}

#[test]
fn fp251_square_root_uses_the_smaller_canonical_root() {
    let limit = if cfg!(miri) { 32 } else { 251 };
    for value in 0_u8..limit {
        let element = Fp251V1::from_canonical(&[value]).unwrap();
        if let Some(root) = element.sqrt() {
            assert_eq!(root.square(), element);
            assert!(root.to_canonical()[0] <= (-root).to_canonical()[0]);
        }
    }
}

#[test]
fn goldilocks_matches_u128_reference_on_seeded_values() {
    let modulus = u128::from(FpGoldilocks64V1::MODULUS);
    let mut state = 0x243f_6a88_85a3_08d3_u64;
    let iterations = if cfg!(miri) { 64 } else { 20_000 };
    for _ in 0..iterations {
        state = state
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(0xa409_3822_299f_31d0);
        let left = u128::from(state) % modulus;
        state = state
            .wrapping_mul(0xbf58_476d_1ce4_e5b9)
            .wrapping_add(0x94d0_49bb_1331_11eb);
        let right = u128::from(state) % modulus;
        let a =
            FpGoldilocks64V1::from_canonical(&u64::try_from(left).unwrap().to_le_bytes()).unwrap();
        let b =
            FpGoldilocks64V1::from_canonical(&u64::try_from(right).unwrap().to_le_bytes()).unwrap();
        assert_eq!(
            u64::from_le_bytes((a + b).to_canonical()),
            u64::try_from((left + right) % modulus).unwrap()
        );
        assert_eq!(
            u64::from_le_bytes((a - b).to_canonical()),
            u64::try_from((left + modulus - right) % modulus).unwrap()
        );
        assert_eq!(
            u64::from_le_bytes((a * b).to_canonical()),
            u64::try_from((left * right) % modulus).unwrap()
        );
        assert_eq!(
            u64::from_le_bytes(
                FpGoldilocks64V1::__barrett_reduce_wide(left * right).to_canonical()
            ),
            u64::try_from((left * right) % modulus).unwrap()
        );
        assert_eq!(a.square(), a * a);
    }
}

#[test]
fn goldilocks_encoding_boundaries_and_reduction_are_explicit() {
    let modulus = FpGoldilocks64V1::MODULUS;
    assert!(FpGoldilocks64V1::from_canonical(&modulus.to_le_bytes()).is_err());
    assert!(FpGoldilocks64V1::from_canonical(&(modulus + 1).to_le_bytes()).is_err());
    let reduced = FpGoldilocks64V1::from_bytes_mod_order(&modulus.to_le_bytes());
    assert_eq!(reduced, FpGoldilocks64V1::ZERO);
    assert_eq!(FpGoldilocks64V1::solinas_plan().verify(), Ok(()));
    assert_eq!(FpGoldilocks64V1::barrett_plan().verify(64), Ok(()));
    assert_eq!(
        FpGoldilocks64V1::reduction_plan().kind(),
        PrimeReductionKind::Barrett
    );
}

#[test]
fn fp256_matches_independent_sage_vectors() {
    const VECTORS: [(&str, &str, &str, &str, &str, &str, &str); 3] = [
        (
            "649655f7660f6cb7ade6e927c462b0030cdb03639cfe66160000000000000000",
            "2f34a240a4eff6209b445eb57a0b04a3fa4e893270f692ad7bfda0509e91be10",
            "93caf7370bff62d8482b48dd3e6eb4a6062a8d950cf5f9c37bfda0509e91be10",
            "7469db5bb1874cf7ffe0a419239f5cba7f2d672c9a55b30b503bd1957abdee8c",
            "94f5203aa36c382bf12f22beb2d6926e221c99db54e25123736a68258318707f",
            "82161090e745b119972cd467d3dfe80962af1cd7848dfacf4999044362e14c99",
            "06ec9b7881811b3138968664fc4a44f74e5c9fc873a9699c91f115495153e013",
        ),
        (
            "0efd9f936930ea5219fca9d868e694ef8c7b733099f35a16b562f0c496e07d0e",
            "2b7738202766be4293c914c3beae5d633be12f640ccbced831be118dc6e1409a",
            "fa6cb00ea22ed134bf86a5f44d4d42f959bbb69837714a4c1be88f6b4473110b",
            "228d8f18313203717371aebc837fe7e5bf3b30c8fa756be04edd501ee94dea11",
            "fd969b2b14500c3f4cdf8081d547e7a7063e76bcdeffee09ae8f295ab45e4b8a",
            "b67c5e9627a95cc6ea180fb207259e642993ae087d45ed7b9e51d251ef094384",
            "6f5c7ba0845bcceff740cfb8a98ffc3247d11cdd990f45c8ab97ed7258a05c09",
        ),
        (
            "ad3ff06d420326a65d7411141a21d3ef2c924195606548ce3a0b12b895937f37",
            "63f72ef15341fb22fc3cc643da719494f04342cf418710d3dfed86c59b2d0090",
            "d12ff7b9a7dc49686c72beb01a4bb72aaf349768349f79fe4ec026971872d229",
            "894fe921dd2902e44e76647719f7eeb4aaefebc18c2b179e2656fdd812b52c45",
            "ced67a59dcd06921649694a114ef54fe85bfa6e6e16e8ca28d9d08ede89f2958",
            "ad114556af3cacafdbee44aaf44b4f8f111f8f1e73770e51e264122836fc3a83",
            "e836cbb7fbe6477d305036dda2e35702953d7d6b6c19c13b8dbac97fe3317c56",
        ),
    ];
    for (a, b, sum, difference, product, square, inverse) in VECTORS {
        let a = Fp256GenericV1::from_canonical(&decode_hex_32(a)).unwrap();
        let b = Fp256GenericV1::from_canonical(&decode_hex_32(b)).unwrap();
        assert_eq!((a + b).to_canonical(), decode_hex_32(sum));
        assert_eq!((a - b).to_canonical(), decode_hex_32(difference));
        assert_eq!((a * b).to_canonical(), decode_hex_32(product));
        assert_eq!(a.square().to_canonical(), decode_hex_32(square));
        assert_eq!(a.invert().unwrap().to_canonical(), decode_hex_32(inverse));
    }
}

#[test]
fn fp256_encoding_montgomery_and_fermat_contracts_hold() {
    let modulus = Fp256GenericV1::modulus_le_bytes();
    assert!(Fp256GenericV1::from_canonical(&modulus).is_err());
    let mut modulus_plus_one = modulus;
    modulus_plus_one[0] += 1;
    assert!(Fp256GenericV1::from_canonical(&modulus_plus_one).is_err());
    let one = Fp256GenericV1::ONE;
    assert_eq!(one.to_canonical()[0], 1);
    assert!(one.to_canonical()[1..].iter().all(|byte| *byte == 0));
    assert_eq!(Fp256GenericV1::montgomery_plan().verify(256), Ok(()));
    assert_eq!(
        Fp256GenericV1::reduction_plan().kind(),
        PrimeReductionKind::Montgomery
    );

    let value = Fp256GenericV1::from_bytes_mod_order(&[0xa5; 97]);
    assert_eq!(
        value.pow(&[
            0x60d7_67ee_a528_073f,
            0x59b0_47d9_a719_3eed,
            0xa2df_4d6d_fbec_a16e,
            0x9dad_4f18_e672_38cb,
        ]),
        value
    );
    assert_eq!(value * value.invert().unwrap(), one);
    let square = value.square();
    let root = square.sqrt().unwrap();
    assert_eq!(root.square(), square);
}

#[test]
fn generated_inversion_plans_and_kernel_ranges_are_field_specific() {
    assert!(Fp251V1::inversion_plan().verifies_target([249]));
    assert!(FpGoldilocks64V1::inversion_plan().verifies_target([FpGoldilocks64V1::MODULUS - 2]));
    assert!(Fp256GenericV1::inversion_plan().verifies_target([
        0x60d7_67ee_a528_073d,
        0x59b0_47d9_a719_3eed,
        0xa2df_4d6d_fbec_a16e,
        0x9dad_4f18_e672_38cb,
    ]));

    let fp251 = *Engine::<Fp251V1>::portable().metadata().prime().unwrap();
    assert_eq!(
        fp251.representation(),
        PrimeRepresentationKind::CanonicalResidue
    );
    assert_eq!(fp251.reduction(), PrimeReductionKind::Native);
    assert_eq!(fp251.output_range().output_multiple(), 1);
    assert_eq!(fp251.output_range().verify(8), Ok(()));

    let gold = *Engine::<FpGoldilocks64V1>::portable()
        .metadata()
        .prime()
        .unwrap();
    assert_eq!(gold.reduction(), PrimeReductionKind::Barrett);
    assert_eq!(gold.output_range().verify(64), Ok(()));

    let generic = *Engine::<Fp256GenericV1>::portable()
        .metadata()
        .prime()
        .unwrap();
    assert_eq!(
        generic.representation(),
        PrimeRepresentationKind::Montgomery {
            radix_bits: 64,
            limbs: 4,
        }
    );
    assert_eq!(generic.reduction(), PrimeReductionKind::Montgomery);
    assert_eq!(generic.output_range().verify(256), Ok(()));
}

#[test]
fn arbitrary_byte_reduction_covers_every_input_bit_through_double_width() {
    check_reduced_input_bits::<Fp251V1>(2);
    check_reduced_input_bits::<FpGoldilocks64V1>(16);
    check_reduced_input_bits::<Fp256GenericV1>(64);
}

#[test]
fn phase3_batch_inversion_is_reused_without_prime_specific_semantics() {
    batch_inversion::<Fp251V1>();
    batch_inversion::<FpGoldilocks64V1>();
    batch_inversion::<Fp256GenericV1>();
}

fn batch_inversion<F>()
where
    F: microfield::BuiltinField + StaticField + PrimeField + Invert + core::fmt::Debug,
{
    let engine = Engine::<F>::portable();
    let sizes: &[usize] = if cfg!(miri) {
        &[0, 1, 2, 7, 16]
    } else {
        &[0, 1, 2, 3, 7, 8, 15, 16, 31, 32, 63, 64, 255, 256, 1024]
    };
    for &len in sizes {
        let values = (0..len)
            .map(|index| {
                if index % 11 == 0 {
                    F::ZERO
                } else {
                    F::from_bytes_mod_order(&index.wrapping_mul(73).to_le_bytes())
                }
            })
            .collect::<Vec<_>>();
        let mut out = vec![F::ZERO; len];
        let mut prefixes = vec![F::ZERO; len];
        let mut mask_words =
            vec![u64::MAX; microfield::required_mask_words(len).expect("bounded mask")];
        let mut workspace = BatchInvertWorkspace::new(&mut prefixes);
        let mut mask = BitMaskViewMut::new(&mut mask_words, values.len()).unwrap();
        engine
            .invert_batch_into(&mut out, &values, &mut mask, &mut workspace)
            .unwrap();
        for index in 0..len {
            if values[index].is_zero() {
                assert!(!mask.is_set(index));
                assert_eq!(out[index], F::ZERO);
            } else {
                assert!(mask.is_set(index));
                assert_eq!(out[index].mul(values[index]), F::ONE);
            }
        }
    }
}

fn check_reduced_input_bits<F: PrimeField + core::fmt::Debug>(bytes: usize) {
    let mut expected = F::ONE;
    for bit in 0..bytes * 8 {
        if !cfg!(miri) || bit < 9 || bit % 63 == 0 || bit + 1 == bytes * 8 {
            let mut input = vec![0_u8; bytes];
            input[bit / 8] = 1 << (bit % 8);
            assert_eq!(
                F::from_bytes_mod_order(&input),
                expected,
                "explicit reduction failed at bit {bit}"
            );
        }
        expected = expected.add(expected);
    }
}

fn decode_hex_32(hex: &str) -> [u8; 32] {
    assert_eq!(hex.len(), 64);
    let mut bytes = [0_u8; 32];
    for (index, output) in bytes.iter_mut().enumerate() {
        *output = (nibble(hex.as_bytes()[index * 2]) << 4) | nibble(hex.as_bytes()[index * 2 + 1]);
    }
    bytes
}

fn nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("invalid hex vector"),
    }
}
