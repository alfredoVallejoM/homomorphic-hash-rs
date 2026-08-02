//! Isolated allocation-counting gate for the H4 public batch API.

#![cfg(all(
    feature = "builtin-fields",
    feature = "portable",
    feature = "count-allocations"
))]

use allocation_counter::{AllocationInfo, measure};
use microfield::{
    BackendId, CanonicalEncoding, CpuCapabilities, Engine, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1,
    PackedBatch, StaticField, pack_into_storage, required_packed_bytes,
};

#[test]
fn every_available_batch_operation_allocates_zero_times() {
    assert_zero_allocations::<Gf2_128V1, 16>();
    assert_zero_allocations::<Gf2_256HhV1, 32>();
    assert_zero_allocations::<Gf2_256AltV1, 32>();
}

#[test]
fn capability_detection_and_engine_selection_allocate_zero_times() {
    let portable = measure(|| {
        let engine = Engine::<Gf2_256HhV1>::builder()
            .expected_batch(4096)
            .capabilities(CpuCapabilities::portable_only())
            .build()
            .expect("portable selection is infallible");
        assert_eq!(engine.backend_id(), BackendId::Portable);
    });
    let detected = measure(|| {
        let engine = Engine::<Gf2_256HhV1>::builder()
            .expected_batch(4096)
            .detect()
            .expect("portable or a certified ISA backend must be available");
        #[cfg(target_arch = "x86_64")]
        let expected = if std::arch::is_x86_feature_detected!("pclmulqdq") {
            BackendId::X86Pclmul
        } else {
            BackendId::Portable
        };
        #[cfg(target_arch = "aarch64")]
        let expected = BackendId::Portable;
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        let expected = BackendId::Portable;
        assert_eq!(engine.backend_id(), expected);
    });

    assert_allocation_info_is_zero(portable);
    assert_allocation_info_is_zero(detected);

    #[cfg(target_arch = "x86_64")]
    if std::arch::is_x86_feature_detected!("pclmulqdq")
        && std::arch::is_x86_feature_detected!("avx2")
        && std::arch::is_x86_feature_detected!("vpclmulqdq")
    {
        let forced = measure(|| {
            let engine = Engine::<Gf2_256HhV1>::builder()
                .force_backend(BackendId::X86Vpclmul)
                .detect()
                .expect("detected VPCLMUL is compiled and certified");
            assert_eq!(engine.backend_id(), BackendId::X86Vpclmul);
        });
        assert_allocation_info_is_zero(forced);
    }

    #[cfg(target_arch = "aarch64")]
    if std::arch::is_aarch64_feature_detected!("neon")
        && std::arch::is_aarch64_feature_detected!("pmull")
    {
        let forced = measure(|| {
            let engine = Engine::<Gf2_256HhV1>::builder()
                .force_backend(BackendId::Aarch64Pmull)
                .detect()
                .expect("detected PMULL is compiled and certified");
            assert_eq!(engine.backend_id(), BackendId::Aarch64Pmull);
        });
        assert_allocation_info_is_zero(forced);
    }
}

fn assert_zero_allocations<F, const BYTES: usize>()
where
    F: microfield::BuiltinField + CanonicalEncoding + StaticField + core::fmt::Debug,
{
    const LEN: usize = 64;

    let lhs_value =
        F::from_canonical_slice(&[0xa5; BYTES]).expect("full-width binary values are canonical");
    let rhs_value =
        F::from_canonical_slice(&[0x3c; BYTES]).expect("full-width binary values are canonical");
    let lhs = vec![lhs_value; LEN];
    let rhs = vec![rhs_value; LEN];
    let mut output = vec![F::ZERO; LEN];
    let mut assigned = lhs.clone();

    let detected = Engine::<F>::builder()
        .expected_batch(LEN)
        .detect()
        .expect("portable or a certified ISA backend must be available");
    assert_engine_allocates_zero(
        Engine::<F>::portable(),
        &mut output,
        &lhs,
        &rhs,
        &mut assigned,
    );
    assert_engine_allocates_zero(detected, &mut output, &lhs, &rhs, &mut assigned);

    #[cfg(target_arch = "x86_64")]
    if std::arch::is_x86_feature_detected!("pclmulqdq")
        && std::arch::is_x86_feature_detected!("avx2")
        && std::arch::is_x86_feature_detected!("vpclmulqdq")
    {
        let forced = Engine::<F>::builder()
            .force_backend(BackendId::X86Vpclmul)
            .detect()
            .expect("the maintained field certifies VPCLMUL");
        assert_engine_allocates_zero(forced, &mut output, &lhs, &rhs, &mut assigned);
    }

    #[cfg(target_arch = "aarch64")]
    if std::arch::is_aarch64_feature_detected!("neon")
        && std::arch::is_aarch64_feature_detected!("pmull")
    {
        let forced = Engine::<F>::builder()
            .force_backend(BackendId::Aarch64Pmull)
            .detect()
            .expect("the maintained field certifies PMULL");
        assert_engine_allocates_zero(forced, &mut output, &lhs, &rhs, &mut assigned);
    }
}

fn assert_engine_allocates_zero<F: microfield::BuiltinField + StaticField>(
    engine: Engine<F>,
    output: &mut [F],
    lhs: &[F],
    rhs: &[F],
    assigned: &mut [F],
) {
    let allocations = measure(|| {
        engine
            .add_into(output, lhs, rhs)
            .expect("equal lengths are valid");
        engine
            .mul_into(output, lhs, rhs)
            .expect("equal lengths are valid");
        engine
            .square_into(output, lhs)
            .expect("equal lengths are valid");
        engine
            .mul_assign(assigned, rhs)
            .expect("equal lengths are valid");
        engine.square_assign(assigned);
    });
    assert_allocation_info_is_zero(allocations);

    let packed_lhs = PackedBatch::from_aos(&engine, lhs).expect("packed lhs");
    let packed_rhs = PackedBatch::from_aos(&engine, rhs).expect("packed rhs");
    let mut packed_out = PackedBatch::new(&engine, lhs.len()).expect("packed output");
    let mut packed_assigned = PackedBatch::from_aos(&engine, lhs).expect("packed assigned");
    let mut unpacked = output.to_vec();
    let packed_allocations = measure(|| {
        engine
            .mul_packed_into(&mut packed_out, &packed_lhs, &packed_rhs)
            .expect("matching plans");
        engine
            .square_packed_into(&mut packed_out, &packed_lhs)
            .expect("matching plans");
        engine
            .mul_packed_assign(&mut packed_assigned, &packed_rhs)
            .expect("matching plans");
        engine
            .square_packed_assign(&mut packed_assigned)
            .expect("matching backend");
        packed_assigned.pack_from(lhs).expect("matching length");
        packed_out
            .unpack_into(&mut unpacked)
            .expect("matching length");
    });
    assert_allocation_info_is_zero(packed_allocations);

    let plan = engine.packing_plan(lhs.len()).expect("valid plan");
    let bytes = required_packed_bytes(&plan).expect("bounded plan");
    let mut lhs_storage = vec![core::mem::MaybeUninit::uninit(); bytes];
    let mut rhs_storage = vec![core::mem::MaybeUninit::uninit(); bytes];
    let mut out_storage = vec![core::mem::MaybeUninit::uninit(); bytes];
    let view_allocations = measure(|| {
        let lhs_view = pack_into_storage(&engine, &mut lhs_storage, lhs).expect("view lhs storage");
        let rhs_view = pack_into_storage(&engine, &mut rhs_storage, rhs).expect("view rhs storage");
        let mut out_view =
            pack_into_storage(&engine, &mut out_storage, output).expect("view output storage");
        engine
            .mul_packed_view_into(&mut out_view, &lhs_view.as_view(), &rhs_view.as_view())
            .expect("matching plans");
        engine
            .square_packed_view_assign(&mut out_view)
            .expect("matching backend");
    });
    assert_allocation_info_is_zero(view_allocations);
}

fn assert_allocation_info_is_zero(allocations: AllocationInfo) {
    assert_eq!(allocations.count_total, 0);
    assert_eq!(allocations.bytes_total, 0);
    assert_eq!(allocations.count_current, 0);
    assert_eq!(allocations.bytes_current, 0);
}
