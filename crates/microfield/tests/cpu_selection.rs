//! H2.3 public CPU detection and immutable selector contracts.

#![cfg(all(feature = "std", feature = "portable", feature = "builtin-fields"))]

use std::{sync::Arc, thread};

use microfield::{
    Architecture, BackendId, CpuCapabilities, Engine, EngineBuildError, EngineBuilder,
    ExecutionPolicy, Gf2_128V1, Gf2_256HhV1,
};

fn assert_send_sync_static<T: Send + Sync + 'static>() {}

fn detected_pclmul_available() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::arch::is_x86_feature_detected!("pclmulqdq")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

#[cfg(target_arch = "x86_64")]
fn detected_vpclmul_available() -> bool {
    std::arch::is_x86_feature_detected!("pclmulqdq")
        && std::arch::is_x86_feature_detected!("avx2")
        && std::arch::is_x86_feature_detected!("vpclmulqdq")
}

#[cfg(target_arch = "aarch64")]
fn detected_pmull_available() -> bool {
    std::arch::is_aarch64_feature_detected!("neon")
        && std::arch::is_aarch64_feature_detected!("pmull")
}

fn expected_detected_backend(expected_batch: usize) -> BackendId {
    if detected_pclmul_available() && expected_batch >= 1 {
        BackendId::X86Pclmul
    } else {
        BackendId::Portable
    }
}

#[test]
fn detected_snapshot_exactly_matches_rust_feature_detection() {
    let detected = CpuCapabilities::detect();
    assert_eq!(detected.architecture(), Architecture::current());

    #[cfg(target_arch = "x86_64")]
    {
        assert_eq!(detected.architecture(), Architecture::X86_64);
        assert_eq!(
            detected.has_x86_pclmulqdq(),
            std::arch::is_x86_feature_detected!("pclmulqdq")
        );
        assert_eq!(
            detected.has_x86_avx2(),
            std::arch::is_x86_feature_detected!("avx2")
        );
        assert_eq!(
            detected.has_x86_vpclmulqdq(),
            std::arch::is_x86_feature_detected!("vpclmulqdq")
        );
        assert!(!detected.has_aarch64_neon());
        assert!(!detected.has_aarch64_pmull());
    }

    #[cfg(target_arch = "aarch64")]
    {
        assert_eq!(detected.architecture(), Architecture::Aarch64);
        assert_eq!(
            detected.has_aarch64_neon(),
            std::arch::is_aarch64_feature_detected!("neon")
        );
        assert_eq!(
            detected.has_aarch64_pmull(),
            std::arch::is_aarch64_feature_detected!("pmull")
        );
        assert!(!detected.has_x86_pclmulqdq());
        assert!(!detected.has_x86_avx2());
        assert!(!detected.has_x86_vpclmulqdq());
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    assert_eq!(detected, CpuCapabilities::portable_only());
}

#[test]
fn portable_snapshot_is_an_explicit_deterministic_upper_bound() {
    let capabilities = CpuCapabilities::portable_only();
    assert_eq!(capabilities.architecture(), Architecture::current());
    assert!(!capabilities.has_x86_pclmulqdq());
    assert!(!capabilities.has_x86_avx2());
    assert!(!capabilities.has_x86_vpclmulqdq());
    assert!(!capabilities.has_aarch64_neon());
    assert!(!capabilities.has_aarch64_pmull());

    for policy in [
        ExecutionPolicy::Auto,
        ExecutionPolicy::LowLatency,
        ExecutionPolicy::Throughput,
        ExecutionPolicy::PortableOnly,
    ] {
        let engine = EngineBuilder::<Gf2_256HhV1>::new()
            .policy(policy)
            .capabilities(capabilities)
            .build()
            .expect("portable is always eligible for these policies");
        assert_eq!(engine.backend_id(), BackendId::Portable);
        assert_eq!(engine.policy(), policy);
    }
}

#[test]
#[allow(clippy::similar_names, clippy::too_many_lines)]
fn detection_selects_only_compiled_and_supported_backends() {
    for policy in [
        ExecutionPolicy::Auto,
        ExecutionPolicy::LowLatency,
        ExecutionPolicy::Throughput,
        ExecutionPolicy::PortableOnly,
    ] {
        let engine = EngineBuilder::<Gf2_128V1>::new()
            .policy(policy)
            .expected_batch(16_384)
            .detect()
            .expect("portable or a certified ISA backend must be available");
        let expected = if policy == ExecutionPolicy::PortableOnly {
            BackendId::Portable
        } else {
            expected_detected_backend(16_384)
        };
        assert_eq!(engine.backend_id(), expected);
        assert_eq!(engine.expected_batch(), Some(16_384));
    }

    let forced_pclmul = EngineBuilder::<Gf2_128V1>::new()
        .force_backend(BackendId::X86Pclmul)
        .detect();
    #[cfg(target_arch = "x86_64")]
    if detected_pclmul_available() {
        assert_eq!(
            forced_pclmul
                .expect("detected PCLMUL is compiled and certified")
                .backend_id(),
            BackendId::X86Pclmul
        );
    } else {
        assert!(matches!(
            forced_pclmul,
            Err(EngineBuildError::BackendUnsupportedByCpu(
                BackendId::X86Pclmul
            ))
        ));
    }
    #[cfg(not(target_arch = "x86_64"))]
    assert!(matches!(
        forced_pclmul,
        Err(EngineBuildError::BackendNotCompiled(BackendId::X86Pclmul))
    ));

    let forced_vpclmul = EngineBuilder::<Gf2_128V1>::new()
        .force_backend(BackendId::X86Vpclmul)
        .detect();
    #[cfg(target_arch = "x86_64")]
    if detected_vpclmul_available() {
        assert_eq!(
            forced_vpclmul
                .expect("detected VPCLMUL is compiled and certified")
                .backend_id(),
            BackendId::X86Vpclmul
        );
    } else {
        assert!(matches!(
            forced_vpclmul,
            Err(EngineBuildError::BackendUnsupportedByCpu(
                BackendId::X86Vpclmul
            ))
        ));
    }
    #[cfg(not(target_arch = "x86_64"))]
    assert!(matches!(
        forced_vpclmul,
        Err(EngineBuildError::BackendNotCompiled(BackendId::X86Vpclmul))
    ));

    let forced_pmull = EngineBuilder::<Gf2_128V1>::new()
        .force_backend(BackendId::Aarch64Pmull)
        .detect();
    #[cfg(target_arch = "aarch64")]
    if detected_pmull_available() {
        assert_eq!(
            forced_pmull
                .expect("detected PMULL is compiled and certified")
                .backend_id(),
            BackendId::Aarch64Pmull
        );
    } else {
        assert!(matches!(
            forced_pmull,
            Err(EngineBuildError::BackendUnsupportedByCpu(
                BackendId::Aarch64Pmull
            ))
        ));
    }
    #[cfg(not(target_arch = "aarch64"))]
    assert!(matches!(
        forced_pmull,
        Err(EngineBuildError::BackendNotCompiled(
            BackendId::Aarch64Pmull
        ))
    ));

    let fixed = EngineBuilder::<Gf2_128V1>::new()
        .policy(ExecutionPolicy::FixedSchedule)
        .detect();
    if detected_pclmul_available() {
        assert_eq!(
            fixed
                .expect("the detected ISA backend has a certified fixed schedule")
                .backend_id(),
            BackendId::X86Pclmul
        );
    } else {
        assert!(matches!(
            fixed,
            Err(EngineBuildError::PolicyUnsatisfied(
                ExecutionPolicy::FixedSchedule
            ))
        ));
    }
}

#[test]
fn engine_construction_is_concurrent_and_deterministic() {
    assert_send_sync_static::<Architecture>();
    assert_send_sync_static::<CpuCapabilities>();
    assert_send_sync_static::<Engine<Gf2_256HhV1>>();
    assert_send_sync_static::<EngineBuilder<Gf2_256HhV1>>();

    let start = Arc::new(std::sync::Barrier::new(16));
    let threads: Vec<_> = (0..16)
        .map(|_| {
            let start = Arc::clone(&start);
            thread::spawn(move || {
                start.wait();
                for expected in [0, 1, 8, 64, 1024, 16_384] {
                    let engine = Engine::<Gf2_256HhV1>::builder()
                        .expected_batch(expected)
                        .detect()
                        .expect("portable fallback remains available");
                    assert_eq!(engine.backend_id(), expected_detected_backend(expected));
                    assert_eq!(engine.expected_batch(), Some(expected));
                }
            })
        })
        .collect();

    for thread in threads {
        thread.join().expect("selector thread must not panic");
    }
}

#[test]
fn precise_selection_errors_have_stable_diagnostics() {
    let backend = BackendId::X86Pclmul;
    assert_eq!(
        EngineBuildError::BackendNotCompiled(backend).to_string(),
        "batch backend X86Pclmul is not compiled"
    );
    assert_eq!(
        EngineBuildError::BackendUnsupportedByField(backend).to_string(),
        "batch backend X86Pclmul does not support this field"
    );
    assert_eq!(
        EngineBuildError::BackendUnsupportedByCpu(backend).to_string(),
        "batch backend X86Pclmul is not supported by this CPU"
    );
    assert_eq!(
        EngineBuildError::PolicyUnsatisfied(ExecutionPolicy::FixedSchedule).to_string(),
        "batch policy FixedSchedule cannot be satisfied"
    );
}
