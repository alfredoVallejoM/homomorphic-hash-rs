//! Differential tests for x86 prime-field adapters.

#![cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]

use microfield::{
    BackendId, CanonicalEncoding, CpuCapabilities, Engine, Field, Fp251V1, Fp256GenericV1,
    PrimeField,
};

const SIZES: &[usize] = &[0, 1, 2, 15, 16, 17, 31, 32, 33, 63, 64, 255, 256, 1024];

#[test]
fn fp251_avx2_matches_portable_for_tails_and_in_place_routes() {
    let capabilities = CpuCapabilities::detect();
    if !capabilities.has_x86_avx2() {
        return;
    }
    let portable = Engine::<Fp251V1>::portable();
    let avx2 = Engine::<Fp251V1>::builder()
        .capabilities(capabilities)
        .force_backend(BackendId::X86PrimeAvx2)
        .build()
        .unwrap();
    assert_eq!(avx2.backend_id(), BackendId::X86PrimeAvx2);

    for &len in SIZES {
        let lhs = values::<Fp251V1>(len, 0x243f_6a88_85a3_08d3);
        let rhs = values::<Fp251V1>(len, 0x1319_8a2e_0370_7344);
        let mut expected = vec![Fp251V1::ZERO; len];
        let mut actual = expected.clone();

        portable.mul_into(&mut expected, &lhs, &rhs).unwrap();
        avx2.mul_into(&mut actual, &lhs, &rhs).unwrap();
        assert_eq!(actual, expected, "multiply len={len}");

        portable.add_into(&mut expected, &lhs, &rhs).unwrap();
        avx2.add_into(&mut actual, &lhs, &rhs).unwrap();
        assert_eq!(actual, expected, "add len={len}");

        portable.square_into(&mut expected, &lhs).unwrap();
        avx2.square_into(&mut actual, &lhs).unwrap();
        assert_eq!(actual, expected, "square len={len}");

        let mut expected_assign = lhs.clone();
        let mut actual_assign = lhs.clone();
        portable.mul_assign(&mut expected_assign, &rhs).unwrap();
        avx2.mul_assign(&mut actual_assign, &rhs).unwrap();
        assert_eq!(actual_assign, expected_assign, "mul_assign len={len}");

        expected_assign = lhs.clone();
        actual_assign = lhs;
        portable.square_assign(&mut expected_assign);
        avx2.square_assign(&mut actual_assign);
        assert_eq!(actual_assign, expected_assign, "square_assign len={len}");
    }
}

#[test]
fn fp251_avx2_reduction_is_exhaustive_over_every_operand_pair() {
    let capabilities = CpuCapabilities::detect();
    if !capabilities.has_x86_avx2() {
        return;
    }
    let portable = Engine::<Fp251V1>::portable();
    let avx2 = Engine::<Fp251V1>::builder()
        .capabilities(capabilities)
        .force_backend(BackendId::X86PrimeAvx2)
        .build()
        .unwrap();
    let mut lhs = Vec::with_capacity(251 * 251);
    let mut rhs = Vec::with_capacity(251 * 251);
    for left in 0_u8..251 {
        for right in 0_u8..251 {
            lhs.push(Fp251V1::from_canonical(&[left]).unwrap());
            rhs.push(Fp251V1::from_canonical(&[right]).unwrap());
        }
    }
    let mut expected = vec![Fp251V1::ZERO; lhs.len()];
    let mut actual = expected.clone();
    portable.mul_into(&mut expected, &lhs, &rhs).unwrap();
    avx2.mul_into(&mut actual, &lhs, &rhs).unwrap();
    assert_eq!(actual, expected);
    portable.add_into(&mut expected, &lhs, &rhs).unwrap();
    avx2.add_into(&mut actual, &lhs, &rhs).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn fp256_bmi2_matches_portable_for_products_carries_and_tails() {
    let capabilities = CpuCapabilities::detect();
    if !capabilities.has_x86_bmi2() {
        return;
    }
    let portable = Engine::<Fp256GenericV1>::portable();
    let bmi2 = Engine::<Fp256GenericV1>::builder()
        .capabilities(capabilities)
        .force_backend(BackendId::X86PrimeBmi2)
        .build()
        .unwrap();
    assert_eq!(bmi2.backend_id(), BackendId::X86PrimeBmi2);

    for &len in SIZES {
        let lhs = values::<Fp256GenericV1>(len, 0xa409_3822_299f_31d0);
        let rhs = values::<Fp256GenericV1>(len, 0x082e_fa98_ec4e_6c89);
        let mut expected = vec![Fp256GenericV1::ZERO; len];
        let mut actual = expected.clone();

        portable.mul_into(&mut expected, &lhs, &rhs).unwrap();
        bmi2.mul_into(&mut actual, &lhs, &rhs).unwrap();
        assert_eq!(actual, expected, "multiply len={len}");

        portable.square_into(&mut expected, &lhs).unwrap();
        bmi2.square_into(&mut actual, &lhs).unwrap();
        assert_eq!(actual, expected, "square len={len}");

        let mut expected_assign = lhs.clone();
        let mut actual_assign = lhs.clone();
        portable.mul_assign(&mut expected_assign, &rhs).unwrap();
        bmi2.mul_assign(&mut actual_assign, &rhs).unwrap();
        assert_eq!(actual_assign, expected_assign, "mul_assign len={len}");

        expected_assign = lhs.clone();
        actual_assign = lhs;
        portable.square_assign(&mut expected_assign);
        bmi2.square_assign(&mut actual_assign);
        assert_eq!(actual_assign, expected_assign, "square_assign len={len}");
    }
}

#[test]
fn selector_promotes_only_the_measured_prime_backend() {
    let capabilities = CpuCapabilities::detect();
    if capabilities.has_x86_avx2() {
        let short = Engine::<Fp251V1>::builder()
            .capabilities(capabilities)
            .expected_batch(32)
            .build()
            .unwrap();
        let long = Engine::<Fp251V1>::builder()
            .capabilities(capabilities)
            .expected_batch(64)
            .build()
            .unwrap();
        assert_eq!(short.backend_id(), BackendId::Portable);
        assert_eq!(long.backend_id(), BackendId::X86PrimeAvx2);
    }
    let generic = Engine::<Fp256GenericV1>::builder()
        .capabilities(capabilities)
        .expected_batch(16_384)
        .build()
        .unwrap();
    assert_eq!(generic.backend_id(), BackendId::Portable);
}

fn values<F: PrimeField>(len: usize, seed: u64) -> Vec<F> {
    (0..len)
        .map(|index| {
            let mut bytes = [0_u8; 48];
            for (offset, byte) in bytes.iter_mut().enumerate() {
                let mixed = seed
                    .wrapping_add(index as u64)
                    .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                    .rotate_left(u32::try_from(offset % 64).unwrap());
                *byte = mixed.to_le_bytes()[offset % 8];
            }
            F::from_bytes_mod_order(&bytes)
        })
        .collect()
}
