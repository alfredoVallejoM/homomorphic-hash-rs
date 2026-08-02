//! H2.7 contracts for the paired-lane x86-64 VPCLMUL strategy.

#![cfg(all(
    feature = "std",
    feature = "portable",
    feature = "builtin-fields",
    target_arch = "x86_64"
))]

use core::mem::MaybeUninit;

use microfield::{
    BackendId, BatchError, BuiltinField, CanonicalEncoding, CpuCapabilities, Engine,
    EngineBuildError, ExecutionPolicy, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, PackError,
    PackedBatch, PackedLayout, ScheduleKind, StaticField, pack_into_storage, required_packed_bytes,
};

trait VpclmulField: BuiltinField + CanonicalEncoding + StaticField + core::fmt::Debug {
    const BYTES: usize;
}

impl VpclmulField for Gf2_128V1 {
    const BYTES: usize = 16;
}

impl VpclmulField for Gf2_256HhV1 {
    const BYTES: usize = 32;
}

impl VpclmulField for Gf2_256AltV1 {
    const BYTES: usize = 32;
}

const NORMATIVE_SIZES: &[usize] = &[
    0, 1, 2, 3, 4, 5, 7, 8, 15, 16, 31, 32, 33, 63, 64, 255, 256, 257, 1024, 16_384,
];

fn vpclmul_is_available() -> bool {
    std::arch::is_x86_feature_detected!("pclmulqdq")
        && std::arch::is_x86_feature_detected!("avx2")
        && std::arch::is_x86_feature_detected!("vpclmulqdq")
}

fn vpclmul_engine<F: VpclmulField>() -> Option<Engine<F>> {
    if !vpclmul_is_available() {
        return None;
    }
    Some(
        Engine::<F>::builder()
            .force_backend(BackendId::X86Vpclmul)
            .detect()
            .expect("the private catalog certifies every maintained field"),
    )
}

#[test]
fn selector_requires_every_feature_and_exposes_lane_pair_metadata() {
    let without_detection = Engine::<Gf2_256HhV1>::builder()
        .force_backend(BackendId::X86Vpclmul)
        .capabilities(CpuCapabilities::portable_only())
        .build();
    assert!(matches!(
        without_detection,
        Err(EngineBuildError::BackendUnsupportedByCpu(
            BackendId::X86Vpclmul
        ))
    ));

    let Some(engine) = vpclmul_engine::<Gf2_256HhV1>() else {
        return;
    };
    let metadata = engine.metadata();
    assert_eq!(engine.backend_id(), BackendId::X86Vpclmul);
    assert_eq!(metadata.backend(), BackendId::X86Vpclmul);
    assert_eq!(metadata.minimum_batch(), usize::MAX);
    assert_eq!(metadata.preferred_multiple(), 2);
    assert_eq!(metadata.required_alignment(), 32);
    assert!(metadata.supports_in_place());
    assert!(metadata.requires_packing());
    assert_eq!(metadata.scratch_bytes_per_element(), 0);
    assert_eq!(metadata.schedule(), ScheduleKind::Fixed);
    assert!(
        !metadata.automatic_selection(),
        "automatic selection needs measured end-to-end thresholds"
    );
    assert_eq!(
        vpclmul_engine::<Gf2_128V1>()
            .expect("the same CPU supports the 128-bit strategy")
            .metadata()
            .minimum_batch(),
        64
    );

    let fixed = Engine::<Gf2_256HhV1>::builder()
        .policy(ExecutionPolicy::FixedSchedule)
        .force_backend(BackendId::X86Vpclmul)
        .detect()
        .expect("forced VPCLMUL satisfies the fixed-schedule policy");
    assert_eq!(fixed.backend_id(), BackendId::X86Vpclmul);
}

#[test]
fn vpclmul_matches_portable_and_pclmul_for_every_field_size_and_tail() {
    assert_every_basis_bit::<Gf2_128V1>();
    assert_every_basis_bit::<Gf2_256HhV1>();
    assert_every_basis_bit::<Gf2_256AltV1>();
    assert_differential::<Gf2_128V1>();
    assert_differential::<Gf2_256HhV1>();
    assert_differential::<Gf2_256AltV1>();
}

#[test]
fn packed_lane_pair_layout_is_aligned_padded_and_field_correct() {
    assert_packed::<Gf2_128V1>();
    assert_packed::<Gf2_256HhV1>();
    assert_packed::<Gf2_256AltV1>();
}

#[test]
fn vpclmul_length_validation_is_transactional() {
    let Some(engine) = vpclmul_engine::<Gf2_128V1>() else {
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
}

fn assert_every_basis_bit<F: VpclmulField>() {
    let Some(vpclmul) = vpclmul_engine::<F>() else {
        return;
    };
    let bits = F::BYTES * 8;
    let lhs: Vec<_> = (0..bits).map(basis::<F>).collect();
    let rhs: Vec<_> = (0..bits)
        .map(|bit| basis::<F>((bit * 73 + 63) % bits))
        .collect();
    let mut expected = vec![F::ZERO; bits];
    let mut actual = vec![F::ZERO; bits];
    Engine::<F>::portable()
        .mul_into(&mut expected, &lhs, &rhs)
        .expect("equal lengths");
    vpclmul
        .mul_into(&mut actual, &lhs, &rhs)
        .expect("equal lengths");
    assert_eq!(actual, expected);
}

fn assert_differential<F: VpclmulField>() {
    let Some(vpclmul) = vpclmul_engine::<F>() else {
        return;
    };
    let portable = Engine::<F>::portable();
    let pclmul = Engine::<F>::builder()
        .force_backend(BackendId::X86Pclmul)
        .detect()
        .expect("VPCLMUL-capable x86 also has PCLMUL");

    for &len in NORMATIVE_SIZES {
        let lhs = values::<F>(len, 0x243f_6a88_85a3_08d3);
        let rhs = values::<F>(len, 0x1319_8a2e_0370_7344);
        let sentinel = element::<F>(len + 37, 0xa409_3822_299f_31d0);
        let mut expected = vec![F::ZERO; len];
        let mut pclmul_result = vec![F::ZERO; len];
        let mut guarded = vec![sentinel; len + 2];

        portable
            .mul_into(&mut expected, &lhs, &rhs)
            .expect("equal lengths");
        pclmul
            .mul_into(&mut pclmul_result, &lhs, &rhs)
            .expect("equal lengths");
        vpclmul
            .mul_into(&mut guarded[1..][..len], &lhs, &rhs)
            .expect("equal lengths");
        assert_eq!(pclmul_result, expected);
        assert_eq!(&guarded[1..][..len], expected);
        assert_canaries(&guarded, &sentinel);

        guarded.fill(sentinel);
        portable
            .square_into(&mut expected, &lhs)
            .expect("equal lengths");
        vpclmul
            .square_into(&mut guarded[1..][..len], &lhs)
            .expect("equal lengths");
        assert_eq!(&guarded[1..][..len], expected);
        assert_canaries(&guarded, &sentinel);

        let mut expected_assign = lhs.clone();
        let mut actual_assign = lhs.clone();
        portable
            .mul_assign(&mut expected_assign, &rhs)
            .expect("equal lengths");
        vpclmul
            .mul_assign(&mut actual_assign, &rhs)
            .expect("equal lengths");
        assert_eq!(actual_assign, expected_assign);

        expected_assign.clone_from(&lhs);
        actual_assign.clone_from(&lhs);
        portable.square_assign(&mut expected_assign);
        vpclmul.square_assign(&mut actual_assign);
        assert_eq!(actual_assign, expected_assign);
    }
}

fn assert_packed<F: VpclmulField>() {
    let Some(engine) = vpclmul_engine::<F>() else {
        return;
    };
    for &len in NORMATIVE_SIZES {
        let lhs_values = values::<F>(len, 0x6a09_e667_f3bc_c909);
        let rhs_values = values::<F>(len, 0xbb67_ae85_84ca_a73b);
        let lhs = PackedBatch::from_aos(&engine, &lhs_values).expect("packed lhs");
        let rhs = PackedBatch::from_aos(&engine, &rhs_values).expect("packed rhs");
        let mut out = PackedBatch::new(&engine, len).expect("packed output");
        let plan = *out.plan();
        assert_eq!(plan.layout(), PackedLayout::AosLanePairs);
        assert_eq!(plan.alignment(), 32);
        assert_eq!(plan.tile_elements(), 2);
        assert_eq!(plan.padded_len(), len.next_multiple_of(2));

        engine
            .mul_packed_into(&mut out, &lhs, &rhs)
            .expect("matching plans");
        let mut actual = vec![F::ZERO; len];
        out.unpack_into(&mut actual).expect("matching length");
        let expected: Vec<_> = lhs_values
            .iter()
            .zip(&rhs_values)
            .map(|(left, right)| left.mul(*right))
            .collect();
        assert_eq!(actual, expected);

        let storage_bytes = required_packed_bytes(&plan).expect("bounded lane-pair plan");
        let mut lhs_storage = vec![MaybeUninit::uninit(); storage_bytes];
        let mut rhs_storage = vec![MaybeUninit::uninit(); storage_bytes];
        let mut out_storage = vec![MaybeUninit::uninit(); storage_bytes];
        let lhs_view = pack_into_storage(&engine, &mut lhs_storage, &lhs_values)
            .expect("borrowed lane-pair lhs");
        let rhs_view = pack_into_storage(&engine, &mut rhs_storage, &rhs_values)
            .expect("borrowed lane-pair rhs");
        let mut out_view = pack_into_storage(&engine, &mut out_storage, &vec![F::ZERO; len])
            .expect("borrowed lane-pair output");
        engine
            .mul_packed_view_into(&mut out_view, &lhs_view.as_view(), &rhs_view.as_view())
            .expect("matching borrowed lane-pair plans");
        out_view
            .unpack_into(&mut actual)
            .expect("matching borrowed output");
        assert_eq!(actual, expected);

        if len > 0 {
            let before = actual.clone();
            assert_eq!(
                out.unpack_into(&mut actual[..len - 1]),
                Err(PackError::LengthMismatch {
                    expected: len,
                    actual: len - 1,
                })
            );
            assert_eq!(actual, before);
        }
    }

    let values = values::<F>(5, 0xdead_beef_cafe_babe);
    let plan = engine
        .packing_plan(values.len())
        .expect("valid lane-pair plan");
    let required = required_packed_bytes(&plan).expect("bounded lane-pair plan");
    for offset in 0..plan.alignment() {
        let mut storage = vec![MaybeUninit::uninit(); required + plan.alignment()];
        let view = pack_into_storage(&engine, &mut storage[offset..offset + required], &values)
            .expect("worst-case capacity supports every input alignment");
        let mut actual = vec![F::ZERO; values.len()];
        view.unpack_into(&mut actual).expect("matching output");
        assert_eq!(actual, values);
    }
}

fn assert_canaries<F: Eq + core::fmt::Debug>(guarded: &[F], sentinel: &F) {
    assert_eq!(guarded.first(), Some(sentinel));
    assert_eq!(guarded.last(), Some(sentinel));
}

fn values<F: VpclmulField>(len: usize, seed: u64) -> Vec<F> {
    (0..len).map(|index| element::<F>(index, seed)).collect()
}

fn element<F: VpclmulField>(index: usize, mut state: u64) -> F {
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
    F::from_canonical_slice(&bytes).expect("built-in binary fields use their complete bit width")
}

fn basis<F: VpclmulField>(bit: usize) -> F {
    let mut bytes = vec![0; F::BYTES];
    bytes[bit / 8] = 1 << (bit % 8);
    F::from_canonical_slice(&bytes).expect("every basis bit is canonical")
}
