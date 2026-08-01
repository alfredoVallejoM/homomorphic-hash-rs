//! H4 portable batch contracts over every maintained field.

#![cfg(all(feature = "builtin-fields", feature = "portable"))]

use microfield::{
    BackendId, BatchError, BuiltinField, CanonicalEncoding, Engine, EngineBuildError,
    EngineBuilder, ExecutionPolicy, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, ScheduleKind,
};

trait BatchField: BuiltinField + CanonicalEncoding + core::fmt::Debug {
    const BYTES: usize;
}

impl BatchField for Gf2_128V1 {
    const BYTES: usize = 16;
}

impl BatchField for Gf2_256HhV1 {
    const BYTES: usize = 32;
}

impl BatchField for Gf2_256AltV1 {
    const BYTES: usize = 32;
}

const NORMATIVE_SIZES: &[usize] = &[
    0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 255, 256, 1024, 16_384,
];
const MIRI_SIZES: &[usize] = &[0, 1, 3];

#[test]
fn batch_matches_scalar_for_every_normative_size_and_field() {
    assert_batch_matches_scalar::<Gf2_128V1>();
    assert_batch_matches_scalar::<Gf2_256HhV1>();
    assert_batch_matches_scalar::<Gf2_256AltV1>();
}

#[test]
fn every_length_error_is_transactional_for_every_field() {
    assert_transactional_errors::<Gf2_128V1>();
    assert_transactional_errors::<Gf2_256HhV1>();
    assert_transactional_errors::<Gf2_256AltV1>();
}

#[test]
fn builder_selects_once_and_rejects_unavailable_claims() {
    assert_builder_contract::<Gf2_128V1>();
    assert_builder_contract::<Gf2_256HhV1>();
    assert_builder_contract::<Gf2_256AltV1>();
}

fn assert_batch_matches_scalar<F: BatchField>() {
    let engine = Engine::<F>::portable();
    let sizes = if cfg!(miri) {
        MIRI_SIZES
    } else {
        NORMATIVE_SIZES
    };

    for &len in sizes {
        let lhs = values::<F>(len, 0x243f_6a88_85a3_08d3);
        let rhs = values::<F>(len, 0x1319_8a2e_0370_7344);
        let sentinel = value::<F>(0xa409_3822_299f_31d0, len);

        let expected_add: Vec<_> = lhs
            .iter()
            .zip(&rhs)
            .map(|(left, right)| left.add(*right))
            .collect();
        let expected_mul: Vec<_> = lhs
            .iter()
            .zip(&rhs)
            .map(|(left, right)| left.mul(*right))
            .collect();
        let expected_square: Vec<_> = lhs.iter().map(|item| item.square()).collect();

        let mut guarded = vec![sentinel; len + 2];
        engine
            .add_into(&mut guarded[1..][..len], &lhs, &rhs)
            .expect("equal lengths are valid");
        assert_eq!(&guarded[1..][..len], expected_add);
        assert_eq!(guarded[0], sentinel);
        assert_eq!(guarded[len + 1], sentinel);

        guarded.fill(sentinel);
        engine
            .mul_into(&mut guarded[1..][..len], &lhs, &rhs)
            .expect("equal lengths are valid");
        assert_eq!(&guarded[1..][..len], expected_mul);
        assert_eq!(guarded[0], sentinel);
        assert_eq!(guarded[len + 1], sentinel);

        guarded.fill(sentinel);
        engine
            .square_into(&mut guarded[1..][..len], &lhs)
            .expect("equal lengths are valid");
        assert_eq!(&guarded[1..][..len], expected_square);
        assert_eq!(guarded[0], sentinel);
        assert_eq!(guarded[len + 1], sentinel);

        let mut assigned = Vec::with_capacity(len + 2);
        assigned.push(sentinel);
        assigned.extend_from_slice(&lhs);
        assigned.push(sentinel);
        engine
            .mul_assign(&mut assigned[1..][..len], &rhs)
            .expect("equal lengths are valid");
        assert_eq!(&assigned[1..][..len], expected_mul);
        assert_eq!(assigned[0], sentinel);
        assert_eq!(assigned[len + 1], sentinel);

        assigned[1..][..len].clone_from_slice(&lhs);
        engine.square_assign(&mut assigned[1..][..len]);
        assert_eq!(&assigned[1..][..len], expected_square);
        assert_eq!(assigned[0], sentinel);
        assert_eq!(assigned[len + 1], sentinel);
    }
}

fn assert_transactional_errors<F: BatchField>() {
    let engine = Engine::<F>::portable();
    let lhs = values::<F>(3, 0x082e_fa98_ec4e_6c89);
    let rhs = values::<F>(3, 0x4528_21e6_38d0_1377);
    let sentinel = value::<F>(0xbe54_66cf_34e9_0c6c, 17);

    let mut output = vec![sentinel; 3];
    let original = output.clone();
    assert_eq!(
        engine.add_into(&mut output, &lhs[..2], &rhs),
        Err(BatchError::LengthMismatch {
            out: 3,
            lhs: 2,
            rhs: Some(3),
        })
    );
    assert_eq!(output, original);

    assert_eq!(
        engine.mul_into(&mut output, &lhs, &rhs[..2]),
        Err(BatchError::LengthMismatch {
            out: 3,
            lhs: 3,
            rhs: Some(2),
        })
    );
    assert_eq!(output, original);

    assert_eq!(
        engine.square_into(&mut output[..2], &lhs),
        Err(BatchError::LengthMismatch {
            out: 2,
            lhs: 3,
            rhs: None,
        })
    );
    assert_eq!(output, original);

    let mut assigned = lhs.clone();
    let assigned_original = assigned.clone();
    assert_eq!(
        engine.mul_assign(&mut assigned, &rhs[..2]),
        Err(BatchError::LengthMismatch {
            out: 3,
            lhs: 3,
            rhs: Some(2),
        })
    );
    assert_eq!(assigned, assigned_original);

    engine
        .add_into(&mut [], &[], &[])
        .expect("empty batches are valid");
    engine
        .mul_into(&mut [], &[], &[])
        .expect("empty batches are valid");
    engine
        .square_into(&mut [], &[])
        .expect("empty batches are valid");
    engine
        .mul_assign(&mut [], &[])
        .expect("empty batches are valid");
    engine.square_assign(&mut []);
}

fn assert_builder_contract<F: BatchField>() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Engine<F>>();

    let direct = Engine::<F>::portable();
    assert_eq!(direct.backend_id(), BackendId::Portable);
    assert_eq!(direct.policy(), ExecutionPolicy::PortableOnly);
    assert_eq!(direct.metadata().minimum_batch(), 0);
    assert_eq!(direct.metadata().preferred_multiple(), 1);
    assert_eq!(direct.metadata().required_alignment(), align_of::<F>());
    assert!(direct.metadata().supports_in_place());
    assert!(!direct.metadata().requires_packing());
    assert_eq!(direct.metadata().scratch_bytes_per_element(), 0);
    assert_eq!(direct.metadata().schedule(), ScheduleKind::DataDependent);

    for policy in [
        ExecutionPolicy::Auto,
        ExecutionPolicy::LowLatency,
        ExecutionPolicy::Throughput,
        ExecutionPolicy::PortableOnly,
    ] {
        let selected = EngineBuilder::<F>::new()
            .policy(policy)
            .expected_batch(16_384)
            .build()
            .expect("portable satisfies the policy");
        assert_eq!(selected.backend_id(), BackendId::Portable);
        assert_eq!(selected.policy(), policy);
        assert_eq!(selected.expected_batch(), Some(16_384));
        assert!(core::ptr::eq(selected.metadata(), direct.metadata()));
    }

    let forced = Engine::<F>::builder()
        .force_backend(BackendId::Portable)
        .build()
        .expect("portable is available");
    assert_eq!(forced.backend_id(), BackendId::Portable);

    let pclmul = EngineBuilder::<F>::new()
        .force_backend(BackendId::X86Pclmul)
        .build();
    #[cfg(target_arch = "x86_64")]
    assert!(matches!(
        pclmul,
        Err(EngineBuildError::BackendUnsupportedByCpu(
            BackendId::X86Pclmul
        ))
    ));
    #[cfg(not(target_arch = "x86_64"))]
    assert!(matches!(
        pclmul,
        Err(EngineBuildError::BackendNotCompiled(BackendId::X86Pclmul))
    ));

    for backend in [BackendId::X86Vpclmul, BackendId::Aarch64Pmull] {
        assert!(matches!(
            EngineBuilder::<F>::new()
                .force_backend(backend)
                .build(),
            Err(EngineBuildError::BackendNotCompiled(found)) if found == backend
        ));
    }

    assert!(matches!(
        EngineBuilder::<F>::new()
            .policy(ExecutionPolicy::FixedSchedule)
            .build(),
        Err(EngineBuildError::PolicyUnsatisfied(
            ExecutionPolicy::FixedSchedule
        ))
    ));
}

fn values<F: BatchField>(len: usize, seed: u64) -> Vec<F> {
    (0..len).map(|index| value::<F>(seed, index)).collect()
}

fn value<F: BatchField>(mut state: u64, index: usize) -> F {
    state ^= (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut bytes = vec![0; F::BYTES];
    for chunk in bytes.chunks_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        chunk.copy_from_slice(&state.to_le_bytes()[..chunk.len()]);
    }
    F::from_canonical_slice(&bytes).expect("every full-width binary value is canonical")
}
