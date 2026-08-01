//! Isolated allocation-counting gate for the H4 public batch API.

#![cfg(all(
    feature = "builtin-fields",
    feature = "portable",
    feature = "count-allocations"
))]

use allocation_counter::measure;
use microfield::{
    BackendId, CanonicalEncoding, CpuCapabilities, Engine, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1,
};

#[test]
fn every_portable_batch_operation_allocates_zero_times() {
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
            .expect("portable or certified PCLMUL must be available");
        #[cfg(target_arch = "x86_64")]
        let expected = if std::arch::is_x86_feature_detected!("pclmulqdq") {
            BackendId::X86Pclmul
        } else {
            BackendId::Portable
        };
        #[cfg(not(target_arch = "x86_64"))]
        let expected = BackendId::Portable;
        assert_eq!(engine.backend_id(), expected);
    });

    for allocations in [portable, detected] {
        assert_eq!(allocations.count_total, 0);
        assert_eq!(allocations.bytes_total, 0);
        assert_eq!(allocations.count_current, 0);
        assert_eq!(allocations.bytes_current, 0);
    }
}

fn assert_zero_allocations<F, const BYTES: usize>()
where
    F: microfield::BuiltinField + CanonicalEncoding + core::fmt::Debug,
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
        .expect("portable or certified PCLMUL must be available");
    for engine in [Engine::<F>::portable(), detected] {
        let allocations = measure(|| {
            engine
                .add_into(&mut output, &lhs, &rhs)
                .expect("equal lengths are valid");
            engine
                .mul_into(&mut output, &lhs, &rhs)
                .expect("equal lengths are valid");
            engine
                .square_into(&mut output, &lhs)
                .expect("equal lengths are valid");
            engine
                .mul_assign(&mut assigned, &rhs)
                .expect("equal lengths are valid");
            engine.square_assign(&mut assigned);
        });

        assert_eq!(allocations.count_total, 0);
        assert_eq!(allocations.bytes_total, 0);
        assert_eq!(allocations.count_current, 0);
        assert_eq!(allocations.bytes_current, 0);
    }
}
