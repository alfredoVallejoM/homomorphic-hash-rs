//! H2.5 public contracts for the `AArch64` PMULL batch strategy.

#![cfg(all(
    feature = "std",
    feature = "portable",
    feature = "builtin-fields",
    target_arch = "aarch64"
))]

use core::mem::align_of;

use microfield::{
    BackendId, BatchError, BuiltinField, CanonicalEncoding, CpuCapabilities, Engine,
    EngineBuildError, ExecutionPolicy, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, ScheduleKind,
};

trait PmullField: BuiltinField + CanonicalEncoding + core::fmt::Debug {
    const BYTES: usize;
}

impl PmullField for Gf2_128V1 {
    const BYTES: usize = 16;
}

impl PmullField for Gf2_256HhV1 {
    const BYTES: usize = 32;
}

impl PmullField for Gf2_256AltV1 {
    const BYTES: usize = 32;
}

const NORMATIVE_SIZES: &[usize] = &[
    0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 255, 256, 1024, 16_384,
];

fn pmull_is_available() -> bool {
    std::arch::is_aarch64_feature_detected!("neon")
        && std::arch::is_aarch64_feature_detected!("pmull")
}

fn pmull_engine<F: PmullField>() -> Option<Engine<F>> {
    if !pmull_is_available() {
        return None;
    }
    Some(
        Engine::<F>::builder()
            .force_backend(BackendId::Aarch64Pmull)
            .detect()
            .expect("the private catalog certifies every maintained field"),
    )
}

#[test]
fn selector_requires_trusted_capabilities_and_exposes_fixed_metadata() {
    let without_detection = Engine::<Gf2_256HhV1>::builder()
        .force_backend(BackendId::Aarch64Pmull)
        .capabilities(CpuCapabilities::portable_only())
        .build();
    assert!(matches!(
        without_detection,
        Err(EngineBuildError::BackendUnsupportedByCpu(
            BackendId::Aarch64Pmull
        ))
    ));

    let Some(engine) = pmull_engine::<Gf2_256HhV1>() else {
        return;
    };
    let metadata = engine.metadata();
    assert_eq!(engine.backend_id(), BackendId::Aarch64Pmull);
    assert_eq!(metadata.backend(), BackendId::Aarch64Pmull);
    assert_eq!(metadata.minimum_batch(), 1);
    assert_eq!(metadata.preferred_multiple(), 1);
    assert_eq!(metadata.required_alignment(), align_of::<Gf2_256HhV1>());
    assert!(metadata.supports_in_place());
    assert!(!metadata.requires_packing());
    assert_eq!(metadata.scratch_bytes_per_element(), 0);
    assert_eq!(metadata.schedule(), ScheduleKind::Fixed);
    assert!(!metadata.automatic_selection());

    let fixed = Engine::<Gf2_256HhV1>::builder()
        .policy(ExecutionPolicy::FixedSchedule)
        .force_backend(BackendId::Aarch64Pmull)
        .detect()
        .expect("forced PMULL satisfies the fixed-schedule policy");
    assert_eq!(fixed.backend_id(), BackendId::Aarch64Pmull);

    let empty_auto = Engine::<Gf2_256HhV1>::builder()
        .expected_batch(0)
        .detect()
        .expect("portable remains eligible while PMULL awaits calibration");
    assert_eq!(empty_auto.backend_id(), BackendId::Portable);
    let non_empty_auto = Engine::<Gf2_256HhV1>::builder()
        .expected_batch(1)
        .detect()
        .expect("uncalibrated PMULL cannot enter automatic selection");
    assert_eq!(non_empty_auto.backend_id(), BackendId::Portable);
}

#[test]
fn pmull_matches_portable_for_every_field_basis_bit_and_normative_size() {
    assert_every_basis_bit::<Gf2_128V1>();
    assert_every_basis_bit::<Gf2_256HhV1>();
    assert_every_basis_bit::<Gf2_256AltV1>();
    assert_differential::<Gf2_128V1>();
    assert_differential::<Gf2_256HhV1>();
    assert_differential::<Gf2_256AltV1>();
}

fn assert_every_basis_bit<F: PmullField>() {
    let Some(pmull) = pmull_engine::<F>() else {
        return;
    };
    let bits = F::BYTES * 8;
    let mut lhs = Vec::with_capacity(bits);
    let mut rhs = Vec::with_capacity(bits);
    for bit in 0..bits {
        lhs.push(basis::<F>(bit));
        rhs.push(basis::<F>((bit * 73 + 63) % bits));
    }
    let mut expected = vec![F::ZERO; bits];
    let mut actual = vec![F::ZERO; bits];
    Engine::<F>::portable()
        .mul_into(&mut expected, &lhs, &rhs)
        .expect("equal lengths");
    pmull
        .mul_into(&mut actual, &lhs, &rhs)
        .expect("equal lengths");
    assert_eq!(actual, expected);
}

#[test]
fn pmull_length_errors_are_transactional() {
    let Some(engine) = pmull_engine::<Gf2_128V1>() else {
        return;
    };
    let lhs = values::<Gf2_128V1>(3, 0x243f_6a88_85a3_08d3);
    let rhs = values::<Gf2_128V1>(3, 0x1319_8a2e_0370_7344);
    let sentinel = element::<Gf2_128V1>(17, 0xa409_3822_299f_31d0);

    let mut out = vec![sentinel; 3];
    let original = out.clone();
    assert_eq!(
        engine.mul_into(&mut out, &lhs[..2], &rhs),
        Err(BatchError::LengthMismatch {
            out: 3,
            lhs: 2,
            rhs: Some(3),
        })
    );
    assert_eq!(out, original);

    let mut assigned = lhs;
    let original = assigned.clone();
    assert_eq!(
        engine.mul_assign(&mut assigned, &rhs[..2]),
        Err(BatchError::LengthMismatch {
            out: 3,
            lhs: 3,
            rhs: Some(2),
        })
    );
    assert_eq!(assigned, original);
}

fn assert_differential<F: PmullField>() {
    let Some(pmull) = pmull_engine::<F>() else {
        return;
    };
    let portable = Engine::<F>::portable();

    for &len in NORMATIVE_SIZES {
        let lhs = values::<F>(len, 0x243f_6a88_85a3_08d3);
        let rhs = values::<F>(len, 0x1319_8a2e_0370_7344);
        let sentinel = element::<F>(len + 37, 0xa409_3822_299f_31d0);
        let mut expected = vec![F::ZERO; len];
        let mut guarded = vec![sentinel; len + 2];

        portable
            .add_into(&mut expected, &lhs, &rhs)
            .expect("equal lengths");
        pmull
            .add_into(&mut guarded[1..][..len], &lhs, &rhs)
            .expect("equal lengths");
        assert_eq!(&guarded[1..][..len], expected);
        assert_canaries(&guarded, &sentinel);

        guarded.fill(sentinel);
        portable
            .mul_into(&mut expected, &lhs, &rhs)
            .expect("equal lengths");
        pmull
            .mul_into(&mut guarded[1..][..len], &lhs, &rhs)
            .expect("equal lengths");
        assert_eq!(&guarded[1..][..len], expected);
        assert_canaries(&guarded, &sentinel);

        guarded.fill(sentinel);
        portable
            .square_into(&mut expected, &lhs)
            .expect("equal lengths");
        pmull
            .square_into(&mut guarded[1..][..len], &lhs)
            .expect("equal lengths");
        assert_eq!(&guarded[1..][..len], expected);
        assert_canaries(&guarded, &sentinel);

        let mut expected_assign = lhs.clone();
        let mut actual_assign = lhs.clone();
        portable
            .mul_assign(&mut expected_assign, &rhs)
            .expect("equal lengths");
        pmull
            .mul_assign(&mut actual_assign, &rhs)
            .expect("equal lengths");
        assert_eq!(actual_assign, expected_assign);

        expected_assign.clone_from(&lhs);
        actual_assign.clone_from(&lhs);
        portable.square_assign(&mut expected_assign);
        pmull.square_assign(&mut actual_assign);
        assert_eq!(actual_assign, expected_assign);
    }
}

fn assert_canaries<F: Eq + core::fmt::Debug>(guarded: &[F], sentinel: &F) {
    assert_eq!(guarded.first(), Some(sentinel));
    assert_eq!(guarded.last(), Some(sentinel));
}

fn values<F: PmullField>(len: usize, seed: u64) -> Vec<F> {
    (0..len).map(|index| element::<F>(index, seed)).collect()
}

fn element<F: PmullField>(index: usize, mut state: u64) -> F {
    let mut bytes = vec![0; F::BYTES];
    match index {
        0 => {}
        1 => bytes[0] = 1,
        2 => bytes.fill(u8::MAX),
        3 => bytes[F::BYTES - 1] = 0x80,
        4 => bytes.fill(0xaa),
        5 => bytes.fill(0x55),
        _ => {
            state ^= (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
            for chunk in bytes.chunks_mut(8) {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                chunk.copy_from_slice(&state.to_le_bytes()[..chunk.len()]);
            }
        }
    }
    F::from_canonical_slice(&bytes).expect("maintained fields use their complete bit width")
}

fn basis<F: PmullField>(bit: usize) -> F {
    let mut bytes = vec![0; F::BYTES];
    bytes[bit / 8] = 1 << (bit % 8);
    F::from_canonical_slice(&bytes).expect("every basis bit is canonical")
}
