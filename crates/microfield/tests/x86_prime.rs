//! Differential tests for x86 prime-field adapters.

#![cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]

use microfield::{
    BackendId, CanonicalEncoding, CpuCapabilities, Engine, ExecutionPolicy, Field, Fp251V1,
    Fp256GenericV1, FpGoldilocks64V1, PrimeField, PrimeRepresentationKind, ScheduleKind,
};

#[cfg(feature = "alloc")]
use microfield::{PackedBatch, PackedLayout};

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

#[cfg(feature = "alloc")]
#[test]
fn fp251_persistent_lanes_match_exhaustive_scalar_products_and_sums() {
    let capabilities = CpuCapabilities::detect();
    if !capabilities.has_x86_avx2() {
        return;
    }
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
    let packed_lhs = PackedBatch::from_aos(&avx2, &lhs).unwrap();
    let packed_rhs = PackedBatch::from_aos(&avx2, &rhs).unwrap();
    let mut packed_out = PackedBatch::new(&avx2, lhs.len()).unwrap();
    assert_eq!(packed_lhs.plan().layout(), PackedLayout::CanonicalU8);
    assert_eq!(packed_lhs.plan().alignment(), 32);
    assert_eq!(packed_lhs.plan().physical_element_size(), 1);

    avx2.mul_packed_into(&mut packed_out, &packed_lhs, &packed_rhs)
        .unwrap();
    let mut actual = vec![Fp251V1::ZERO; lhs.len()];
    packed_out.unpack_into(&mut actual).unwrap();
    assert_eq!(
        actual,
        lhs.iter()
            .zip(&rhs)
            .map(|(left, right)| left.mul(*right))
            .collect::<Vec<_>>()
    );

    avx2.add_packed_into(&mut packed_out, &packed_lhs, &packed_rhs)
        .unwrap();
    packed_out.unpack_into(&mut actual).unwrap();
    assert_eq!(
        actual,
        lhs.iter()
            .zip(&rhs)
            .map(|(left, right)| left.add(*right))
            .collect::<Vec<_>>()
    );
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
fn goldilocks_avx2_matches_portable_for_products_sums_tails_and_in_place_routes() {
    let capabilities = CpuCapabilities::detect();
    if !capabilities.has_x86_avx2() {
        return;
    }
    let portable = Engine::<FpGoldilocks64V1>::portable();
    let avx2 = Engine::<FpGoldilocks64V1>::builder()
        .capabilities(capabilities)
        .force_backend(BackendId::X86PrimeAvx2)
        .build()
        .unwrap();
    assert_eq!(avx2.backend_id(), BackendId::X86PrimeAvx2);
    assert!(avx2.metadata().automatic_selection());
    assert_eq!(avx2.metadata().minimum_batch(), 4);
    assert_eq!(avx2.metadata().preferred_multiple(), 4);
    assert_eq!(avx2.metadata().prime().unwrap().lanes(), 4);

    for &len in SIZES {
        let lhs = values::<FpGoldilocks64V1>(len, 0xa409_3822_299f_31d0);
        let rhs = values::<FpGoldilocks64V1>(len, 0x082e_fa98_ec4e_6c89);
        assert_goldilocks_routes(portable, avx2, &lhs, &rhs);
    }

    let boundaries = [
        0,
        1,
        2,
        0xffff_fffe,
        0xffff_ffff,
        FpGoldilocks64V1::MODULUS / 2,
        FpGoldilocks64V1::MODULUS - 2,
        FpGoldilocks64V1::MODULUS - 1,
    ];
    let lhs: Vec<_> = (0..257)
        .map(|index| FpGoldilocks64V1::from_u64_mod(boundaries[index % boundaries.len()]))
        .collect();
    let rhs: Vec<_> = (0..257)
        .map(|index| FpGoldilocks64V1::from_u64_mod(boundaries[(index * 5 + 1) % boundaries.len()]))
        .collect();
    assert_goldilocks_routes(portable, avx2, &lhs, &rhs);
}

fn assert_goldilocks_routes(
    portable: Engine<FpGoldilocks64V1>,
    avx2: Engine<FpGoldilocks64V1>,
    lhs: &[FpGoldilocks64V1],
    rhs: &[FpGoldilocks64V1],
) {
    let mut expected = vec![FpGoldilocks64V1::ZERO; lhs.len()];
    let mut actual = expected.clone();

    portable.mul_into(&mut expected, lhs, rhs).unwrap();
    avx2.mul_into(&mut actual, lhs, rhs).unwrap();
    assert_eq!(actual, expected, "multiply len={}", lhs.len());

    portable.add_into(&mut expected, lhs, rhs).unwrap();
    avx2.add_into(&mut actual, lhs, rhs).unwrap();
    assert_eq!(actual, expected, "add len={}", lhs.len());

    portable.square_into(&mut expected, lhs).unwrap();
    avx2.square_into(&mut actual, lhs).unwrap();
    assert_eq!(actual, expected, "square len={}", lhs.len());

    let mut expected_assign = lhs.to_vec();
    let mut actual_assign = lhs.to_vec();
    portable.mul_assign(&mut expected_assign, rhs).unwrap();
    avx2.mul_assign(&mut actual_assign, rhs).unwrap();
    assert_eq!(
        actual_assign,
        expected_assign,
        "mul_assign len={}",
        lhs.len()
    );

    expected_assign.copy_from_slice(lhs);
    actual_assign.copy_from_slice(lhs);
    portable.square_assign(&mut expected_assign);
    avx2.square_assign(&mut actual_assign);
    assert_eq!(
        actual_assign,
        expected_assign,
        "square_assign len={}",
        lhs.len()
    );
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
    let goldilocks_short = Engine::<FpGoldilocks64V1>::builder()
        .capabilities(capabilities)
        .expected_batch(3)
        .build()
        .unwrap();
    assert_eq!(goldilocks_short.backend_id(), BackendId::Portable);
    let goldilocks_long = Engine::<FpGoldilocks64V1>::builder()
        .capabilities(capabilities)
        .expected_batch(4)
        .build()
        .unwrap();
    assert_eq!(
        goldilocks_long.backend_id(),
        if capabilities.has_x86_avx2() {
            BackendId::X86PrimeAvx2
        } else {
            BackendId::Portable
        }
    );
}

#[test]
fn bmi2_fixed_schedule_does_not_imply_automatic_performance_promotion() {
    let capabilities = CpuCapabilities::detect();
    if !capabilities.has_x86_bmi2() {
        return;
    }

    let bmi2 = Engine::<Fp256GenericV1>::builder()
        .capabilities(capabilities)
        .force_backend(BackendId::X86PrimeBmi2)
        .build()
        .unwrap();
    let metadata = bmi2.metadata();
    assert!(!metadata.automatic_selection());
    assert_eq!(metadata.schedule(), ScheduleKind::Fixed);
    assert_eq!(metadata.preferred_multiple(), 1);
    assert_eq!(metadata.required_alignment(), 8);
    let prime = metadata.prime().unwrap();
    assert_eq!(
        prime.representation(),
        PrimeRepresentationKind::Montgomery {
            radix_bits: 64,
            limbs: 4
        }
    );

    let fixed = Engine::<Fp256GenericV1>::builder()
        .capabilities(capabilities)
        .policy(ExecutionPolicy::FixedSchedule)
        .force_backend(BackendId::X86PrimeBmi2)
        .build()
        .expect("BMI2 uses fixed carry sweeps and masked correction");
    assert_eq!(fixed.backend_id(), BackendId::X86PrimeBmi2);
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
