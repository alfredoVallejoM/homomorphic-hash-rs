//! Deep functional and transactional contracts for Phase 3 algorithms.

#![cfg(all(feature = "portable", feature = "builtin-fields"))]

use core::{fmt::Debug, mem::align_of};

#[cfg(feature = "std")]
use microfield::BackendId;
#[cfg(feature = "alloc")]
use microfield::FixedBasePowers;
use microfield::{
    AlgorithmFamily, AllocationBehavior, BatchInvertError, BatchInvertPlan, BatchInvertWorkspace,
    BatchPlan, BitMaskViewMut, CanonicalEncoding, CoefficientLayout, Engine, Field, Gf2_128V1,
    Gf2_256AltV1, Gf2_256HhV1, HornerError, Invert, ManyPointsHornerPlan,
    ManyPolynomialsHornerPlan, OperationKind, ProductScanPlan, ScanDirection, ScanError, ScanMode,
    StaticField, fill_fixed_base_powers, required_mask_words,
};

#[cfg(not(miri))]
const NORMATIVE_SIZES: &[usize] = &[0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 255, 256, 1024];
#[cfg(miri)]
const NORMATIVE_SIZES: &[usize] = &[0, 4];

#[test]
fn inversion_batch_matches_scalar_for_every_static_field_and_normative_size() {
    inversion_suite::<Gf2_128V1>(NORMATIVE_SIZES);
    inversion_suite::<Gf2_256HhV1>(NORMATIVE_SIZES);
    inversion_suite::<Gf2_256AltV1>(NORMATIVE_SIZES);
}

#[test]
#[cfg_attr(
    miri,
    ignore = "large normative case is covered outside the interpreter"
)]
fn inversion_batch_large_normative_case_is_correct() {
    inversion_suite::<Gf2_256HhV1>(&[16_384]);
}

#[test]
fn inversion_batch_covers_zero_distributions_and_in_place_execution() {
    let engine = Engine::<Gf2_256HhV1>::portable();
    let len = if cfg!(miri) { 17 } else { 257 };
    let distributions = [
        vec![Gf2_256HhV1::ZERO; len],
        (0..len)
            .map(|index| deterministic_nonzero(index as u64 + 1))
            .collect(),
        (0..len)
            .map(|index| {
                if index % 2 == 0 {
                    Gf2_256HhV1::ZERO
                } else {
                    deterministic_nonzero(index as u64)
                }
            })
            .collect(),
        (0..len)
            .map(|index| {
                if index % 32 == 0 {
                    Gf2_256HhV1::ZERO
                } else {
                    deterministic_nonzero(index as u64)
                }
            })
            .collect(),
        (0..len)
            .map(|index| {
                if index == 0 || index + 1 == len {
                    Gf2_256HhV1::ZERO
                } else {
                    deterministic_nonzero(index as u64)
                }
            })
            .collect(),
    ];

    let distribution_count = if cfg!(miri) { 2 } else { distributions.len() };
    for original in distributions.into_iter().take(distribution_count) {
        let mut in_place = original.clone();
        let mut prefix_storage = vec![Gf2_256HhV1::ZERO; len];
        let mut workspace = BatchInvertWorkspace::new(&mut prefix_storage);
        let mut mask_words = vec![u64::MAX; required_mask_words(len).expect("bounded")];
        let mut mask = BitMaskViewMut::new(&mut mask_words, len).expect("exact mask");
        engine
            .invert_batch_assign(&mut in_place, &mut mask, &mut workspace)
            .expect("valid in-place inversion");
        assert_inversion_result(&original, &in_place, &mask);
        assert_padding_is_clear(&mask_words, len);
    }
}

#[test]
fn inversion_plan_publishes_identity_and_exact_storage_requirements() {
    let engine = Engine::<Gf2_128V1>::portable();
    let plan = BatchInvertPlan::new(&engine, 65).expect("bounded plan");
    let requirements = plan.requirements();

    assert_eq!(requirements.prefix_elements(), 65);
    assert_eq!(requirements.mask_words(), 2);
    assert_eq!(plan.logical_len(), 65);
    assert_eq!(plan.backend_id(), engine.backend_id());
    assert_eq!(plan.field_id(), Gf2_128V1::spec().field_id());
    assert_eq!(plan.algorithm_id().operation(), OperationKind::InvertBatch);
    assert_eq!(
        plan.algorithm_id().family(),
        AlgorithmFamily::BatchInversionMontgomery
    );
    assert_eq!(plan.algorithm_id().revision(), 1);
    let layout = plan.workspace_layout();
    assert_eq!(layout.field_elements(), 65);
    assert_eq!(layout.mask_words(), 2);
    assert_eq!(layout.alignment(), align_of::<Gf2_128V1>());
    assert!(layout.supports_in_place());
    assert_eq!(
        layout.allocation(),
        AllocationBehavior::CallerProvidedWorkspace
    );
}

#[test]
fn inversion_validation_errors_are_transactional() {
    let engine = Engine::<Gf2_128V1>::portable();
    let plan = BatchInvertPlan::new(&engine, 4).expect("bounded plan");
    let inputs = (0..4).map(deterministic_nonzero).collect::<Vec<_>>();
    let sentinel = deterministic_nonzero::<Gf2_128V1>(999);

    let mut out = vec![sentinel; 3];
    let mut workspace_storage = vec![Gf2_128V1::ZERO; 4];
    let mut workspace = BatchInvertWorkspace::new(&mut workspace_storage);
    let mut mask_storage = [u64::MAX];
    let mut mask = BitMaskViewMut::new(&mut mask_storage, 4).expect("mask");
    assert!(matches!(
        plan.execute(&engine, &mut out, &inputs, &mut mask, &mut workspace),
        Err(BatchInvertError::LengthMismatch { .. })
    ));
    assert_eq!(out, vec![sentinel; 3]);
    assert_eq!(mask_storage, [u64::MAX]);

    let mut out = vec![sentinel; 4];
    let mut short_workspace_storage = vec![Gf2_128V1::ZERO; 3];
    let mut short_workspace = BatchInvertWorkspace::new(&mut short_workspace_storage);
    let mut mask_storage = [u64::MAX];
    let mut mask = BitMaskViewMut::new(&mut mask_storage, 4).expect("mask");
    assert!(matches!(
        plan.execute(&engine, &mut out, &inputs, &mut mask, &mut short_workspace),
        Err(BatchInvertError::WorkspaceTooSmall { .. })
    ));
    assert_eq!(out, vec![sentinel; 4]);
    assert_eq!(mask_storage, [u64::MAX]);

    let mut out = vec![sentinel; 4];
    let mut workspace_storage = vec![Gf2_128V1::ZERO; 4];
    let mut workspace = BatchInvertWorkspace::new(&mut workspace_storage);
    let mut mask_storage = [u64::MAX];
    let mut wrong_mask = BitMaskViewMut::new(&mut mask_storage, 3).expect("mask");
    assert!(matches!(
        plan.execute(&engine, &mut out, &inputs, &mut wrong_mask, &mut workspace),
        Err(BatchInvertError::MaskLengthMismatch { .. })
    ));
    assert_eq!(out, vec![sentinel; 4]);
    assert_eq!(mask_storage, [u64::MAX]);
}

#[test]
#[cfg(feature = "alloc")]
fn owned_inversion_convenience_matches_borrowed_route() {
    let engine = Engine::<Gf2_256AltV1>::portable();
    let values = (0_u64..129)
        .map(|index| {
            if index % 11 == 0 {
                Gf2_256AltV1::ZERO
            } else {
                deterministic_nonzero(index)
            }
        })
        .collect::<Vec<_>>();
    let (out, mask) = engine
        .invert_batch_alloc(&values)
        .expect("owned inversion succeeds");

    assert_eq!(mask.len(), values.len());
    assert_eq!(
        mask.count_ones(),
        values.iter().filter(|value| !value.is_zero()).count()
    );
    for (index, (input, output)) in values.iter().zip(&out).enumerate() {
        assert_eq!(mask.is_set(index), !input.is_zero());
        assert_eq!(*output, input.invert().unwrap_or(Gf2_256AltV1::ZERO));
    }
    assert_padding_is_clear(mask.words(), mask.len());
}

#[test]
#[cfg(feature = "std")]
fn derived_algorithms_are_backend_independent_and_plans_are_backend_bound() {
    let portable = Engine::<Gf2_256HhV1>::portable();
    let values = (0_u64..257)
        .map(|index| {
            if index % 17 == 0 {
                Gf2_256HhV1::ZERO
            } else {
                deterministic_nonzero(index)
            }
        })
        .collect::<Vec<_>>();
    let portable_result = borrowed_inversion(&portable, &values);

    let mut candidates = Vec::new();
    let selected = Engine::<Gf2_256HhV1>::builder()
        .expected_batch(values.len())
        .detect()
        .expect("portable fallback");
    if selected.backend_id() != BackendId::Portable {
        candidates.push(selected);
    }
    #[cfg(target_arch = "x86_64")]
    for backend in [BackendId::X86Pclmul, BackendId::X86Vpclmul] {
        if let Ok(engine) = Engine::<Gf2_256HhV1>::builder()
            .force_backend(backend)
            .detect()
            && candidates
                .iter()
                .all(|candidate| candidate.backend_id() != engine.backend_id())
        {
            candidates.push(engine);
        }
    }
    #[cfg(target_arch = "aarch64")]
    if let Ok(engine) = Engine::<Gf2_256HhV1>::builder()
        .force_backend(BackendId::Aarch64Pmull)
        .detect()
    {
        candidates.push(engine);
    }

    for engine in candidates {
        let candidate_result = borrowed_inversion(&engine, &values);
        assert_eq!(candidate_result, portable_result);

        let mut portable_scan = vec![Gf2_256HhV1::ZERO; values.len()];
        let mut candidate_scan = portable_scan.clone();
        portable
            .prefix_products_into(&mut portable_scan, &values)
            .expect("portable scan");
        engine
            .prefix_products_into(&mut candidate_scan, &values)
            .expect("candidate scan");
        assert_eq!(candidate_scan, portable_scan);

        let coefficients = (0..9).map(deterministic_value).collect::<Vec<_>>();
        portable
            .horner_many_points_into(&mut portable_scan, &coefficients, &values)
            .expect("portable Horner");
        engine
            .horner_many_points_into(&mut candidate_scan, &coefficients, &values)
            .expect("candidate Horner");
        assert_eq!(candidate_scan, portable_scan);

        let portable_plan = BatchInvertPlan::new(&portable, values.len()).expect("plan");
        let sentinel = deterministic_nonzero(9999);
        let mut out = vec![sentinel; values.len()];
        let mut prefixes = vec![Gf2_256HhV1::ZERO; values.len()];
        let mut workspace = BatchInvertWorkspace::new(&mut prefixes);
        let mut words = vec![u64::MAX; required_mask_words(values.len()).expect("mask")];
        let mut mask = BitMaskViewMut::new(&mut words, values.len()).expect("mask");
        assert!(matches!(
            portable_plan.execute(&engine, &mut out, &values, &mut mask, &mut workspace),
            Err(BatchInvertError::BackendMismatch { .. })
        ));
        assert!(out.iter().all(|value| *value == sentinel));
        assert!(words.iter().all(|word| *word == u64::MAX));
    }
}

#[test]
fn product_scans_cover_all_directions_modes_empty_and_in_place() {
    let engine = Engine::<Gf2_128V1>::portable();
    for len in [0, 1, 2, 7, 64, 257] {
        let inputs = (0..len)
            .map(|index| deterministic_value(index as u64))
            .collect::<Vec<_>>();
        for direction in [ScanDirection::Prefix, ScanDirection::Suffix] {
            for mode in [ScanMode::Inclusive, ScanMode::Exclusive] {
                let plan = ProductScanPlan::new(&engine, len, direction, mode);
                let expected = reference_scan(&inputs, direction, mode);
                let mut out = vec![Gf2_128V1::ZERO; len];
                plan.execute(&engine, &mut out, &inputs)
                    .expect("valid scan");
                assert_eq!(out, expected);

                let mut in_place = inputs.clone();
                plan.execute_assign(&engine, &mut in_place)
                    .expect("valid in-place scan");
                assert_eq!(in_place, expected);
                assert_eq!(plan.direction(), direction);
                assert_eq!(plan.mode(), mode);
            }
        }
    }
}

#[test]
fn scan_convenience_methods_and_transactional_error_match_reference() {
    let engine = Engine::<Gf2_256HhV1>::portable();
    let inputs = (0..19).map(deterministic_value).collect::<Vec<_>>();
    let routes = [
        (ScanDirection::Prefix, ScanMode::Inclusive),
        (ScanDirection::Prefix, ScanMode::Exclusive),
        (ScanDirection::Suffix, ScanMode::Inclusive),
        (ScanDirection::Suffix, ScanMode::Exclusive),
    ];
    for (direction, mode) in routes {
        let mut out = vec![Gf2_256HhV1::ZERO; inputs.len()];
        match (direction, mode) {
            (ScanDirection::Prefix, ScanMode::Inclusive) => {
                engine.prefix_products_into(&mut out, &inputs)
            }
            (ScanDirection::Prefix, ScanMode::Exclusive) => {
                engine.exclusive_prefix_products_into(&mut out, &inputs)
            }
            (ScanDirection::Suffix, ScanMode::Inclusive) => {
                engine.suffix_products_into(&mut out, &inputs)
            }
            (ScanDirection::Suffix, ScanMode::Exclusive) => {
                engine.exclusive_suffix_products_into(&mut out, &inputs)
            }
        }
        .expect("valid convenience scan");
        assert_eq!(out, reference_scan(&inputs, direction, mode));
    }

    let plan = ProductScanPlan::new(
        &engine,
        inputs.len(),
        ScanDirection::Prefix,
        ScanMode::Inclusive,
    );
    let sentinel = deterministic_nonzero(999);
    let mut short_out = vec![sentinel; inputs.len() - 1];
    assert!(matches!(
        plan.execute(&engine, &mut short_out, &inputs),
        Err(ScanError::LengthMismatch { .. })
    ));
    assert!(short_out.iter().all(|value| *value == sentinel));
}

#[test]
fn horner_many_points_matches_scalar_for_edge_points_and_dense_polynomial() {
    let engine = Engine::<Gf2_256HhV1>::portable();
    let coefficients = (0..17).map(deterministic_value).collect::<Vec<_>>();
    let mut points = vec![Gf2_256HhV1::ZERO, Gf2_256HhV1::ONE];
    points.extend((0..67).map(deterministic_nonzero::<Gf2_256HhV1>));
    let plan = ManyPointsHornerPlan::new(&engine, points.len(), coefficients.len())
        .expect("non-empty polynomial");
    let mut out = vec![Gf2_256HhV1::ZERO; points.len()];
    plan.execute(&engine, &mut out, &coefficients, &points)
        .expect("valid Horner shape");

    for (output, point) in out.iter().zip(&points) {
        assert_eq!(*output, evaluate_polynomial(&coefficients, *point));
    }
    assert_eq!(plan.point_count(), points.len());
    assert_eq!(plan.coefficient_count(), coefficients.len());
    assert_eq!(
        plan.algorithm_id().operation(),
        OperationKind::HornerManyPoints
    );
}

#[test]
fn constant_horner_and_empty_points_have_explicit_semantics() {
    let engine = Engine::<Gf2_128V1>::portable();
    let constant = deterministic_nonzero(17);
    let points = (0..8).map(deterministic_value).collect::<Vec<_>>();
    let mut out = vec![Gf2_128V1::ZERO; points.len()];
    engine
        .horner_many_points_into(&mut out, &[constant], &points)
        .expect("constant polynomial");
    assert_eq!(out, vec![constant; points.len()]);

    let mut empty_out = Vec::new();
    engine
        .horner_many_points_into(&mut empty_out, &[constant], &[])
        .expect("empty point set");
    assert!(empty_out.is_empty());

    assert_eq!(
        ManyPointsHornerPlan::new(&engine, 1, 0),
        Err(HornerError::EmptyCoefficientShape)
    );
}

#[test]
fn many_polynomial_horner_supports_both_explicit_layouts() {
    let engine = Engine::<Gf2_256AltV1>::portable();
    let polynomial_count = 11;
    let coefficient_count = 9;
    let point = deterministic_nonzero(777);
    let polynomial_major = (0..polynomial_count)
        .flat_map(|polynomial| {
            (0..coefficient_count)
                .map(move |degree| deterministic_value((polynomial * 31 + degree) as u64))
        })
        .collect::<Vec<_>>();
    let coefficient_major = (0..coefficient_count)
        .flat_map(|degree| {
            (0..polynomial_count)
                .map(move |polynomial| deterministic_value((polynomial * 31 + degree) as u64))
        })
        .collect::<Vec<_>>();

    let mut expected = Vec::with_capacity(polynomial_count);
    for polynomial in 0..polynomial_count {
        let start = polynomial * coefficient_count;
        expected.push(evaluate_polynomial(
            &polynomial_major[start..start + coefficient_count],
            point,
        ));
    }

    for (layout, coefficients) in [
        (CoefficientLayout::PolynomialMajor, &polynomial_major),
        (CoefficientLayout::CoefficientMajor, &coefficient_major),
    ] {
        let plan =
            ManyPolynomialsHornerPlan::new(&engine, polynomial_count, coefficient_count, layout)
                .expect("valid rectangle");
        let mut out = vec![Gf2_256AltV1::ZERO; polynomial_count];
        plan.execute(&engine, &mut out, coefficients, point)
            .expect("valid Horner rectangle");
        assert_eq!(out, expected);
        assert_eq!(plan.layout(), layout);
    }
}

#[test]
fn horner_and_mul_add_validation_leave_output_untouched() {
    let engine = Engine::<Gf2_128V1>::portable();
    let coefficients = (0..4).map(deterministic_value).collect::<Vec<_>>();
    let points = (0..4).map(deterministic_value).collect::<Vec<_>>();
    let plan = ManyPointsHornerPlan::new(&engine, 4, 4).expect("valid shape");
    let sentinel = deterministic_nonzero(333);
    let mut out = vec![sentinel; 3];
    assert!(matches!(
        plan.execute(&engine, &mut out, &coefficients, &points),
        Err(HornerError::LengthMismatch { .. })
    ));
    assert_eq!(out, vec![sentinel; 3]);

    let lhs = vec![deterministic_nonzero(1); 4];
    let rhs = vec![deterministic_nonzero(2); 4];
    let addend = vec![deterministic_nonzero(3); 3];
    let mut fused = vec![sentinel; 4];
    assert!(
        engine
            .mul_add_into(&mut fused, &lhs, &rhs, &addend)
            .is_err()
    );
    assert_eq!(fused, vec![sentinel; 4]);

    let addend = vec![deterministic_nonzero(3); 4];
    engine
        .mul_add_into(&mut fused, &lhs, &rhs, &addend)
        .expect("equal lengths");
    for index in 0..4 {
        assert_eq!(fused[index], lhs[index].mul(rhs[index]).add(addend[index]));
    }
}

#[test]
fn fixed_base_powers_borrowed_match_repeated_multiplication() {
    let base = deterministic_nonzero::<Gf2_256HhV1>(42);
    let mut borrowed = vec![Gf2_256HhV1::ZERO; 130];
    fill_fixed_base_powers(&mut borrowed, base);
    let mut expected = Gf2_256HhV1::ONE;
    for power in &borrowed {
        assert_eq!(*power, expected);
        expected = expected.mul(base);
    }

    let mut empty = Vec::<Gf2_256HhV1>::new();
    fill_fixed_base_powers(&mut empty, base);
    assert!(empty.is_empty());
}

#[test]
#[cfg(feature = "alloc")]
fn owned_fixed_base_power_table_matches_borrowed_route() {
    let base = deterministic_nonzero::<Gf2_256HhV1>(42);
    let mut borrowed = vec![Gf2_256HhV1::ZERO; 130];
    fill_fixed_base_powers(&mut borrowed, base);
    let owned = FixedBasePowers::new(base, 129).expect("bounded table");

    assert_eq!(owned.base(), base);
    assert_eq!(owned.max_exponent(), 129);
    assert_eq!(owned.as_slice(), borrowed);
    for (exponent, expected) in borrowed.iter().enumerate() {
        assert_eq!(owned.power(exponent), Some(*expected));
    }
    assert_eq!(owned.power(130), None);
}

fn inversion_suite<F>(sizes: &[usize])
where
    F: microfield::BuiltinField + CanonicalEncoding + StaticField + Invert + Debug,
{
    let engine = Engine::<F>::portable();
    for &len in sizes {
        let values = (0..len)
            .map(|index| {
                if index % 13 == 0 {
                    F::ZERO
                } else {
                    deterministic_nonzero(index as u64 + len as u64)
                }
            })
            .collect::<Vec<_>>();
        let plan = BatchInvertPlan::new(&engine, len).expect("bounded plan");
        let mut prefix_storage = vec![F::ZERO; plan.requirements().prefix_elements()];
        let mut workspace = BatchInvertWorkspace::new(&mut prefix_storage);
        let mut mask_words = vec![u64::MAX; plan.requirements().mask_words()];
        let mut mask = BitMaskViewMut::new(&mut mask_words, len).expect("exact mask");
        let mut out = vec![F::ZERO; len];
        plan.execute(&engine, &mut out, &values, &mut mask, &mut workspace)
            .expect("valid inversion");
        assert_inversion_result(&values, &out, &mask);
        assert_padding_is_clear(&mask_words, len);
    }
}

#[cfg(feature = "std")]
fn borrowed_inversion<F>(engine: &Engine<F>, values: &[F]) -> (Vec<F>, Vec<u64>)
where
    F: microfield::BuiltinField + StaticField + Invert,
{
    let mut out = vec![F::ZERO; values.len()];
    let mut prefixes = vec![F::ZERO; values.len()];
    let mut workspace = BatchInvertWorkspace::new(&mut prefixes);
    let mut words = vec![0_u64; required_mask_words(values.len()).expect("bounded mask")];
    let mut mask = BitMaskViewMut::new(&mut words, values.len()).expect("exact mask");
    engine
        .invert_batch_into(&mut out, values, &mut mask, &mut workspace)
        .expect("valid inversion");
    (out, words)
}

fn assert_inversion_result<F>(inputs: &[F], outputs: &[F], mask: &BitMaskViewMut<'_>)
where
    F: Field + Invert + Debug,
{
    assert_eq!(inputs.len(), outputs.len());
    assert_eq!(mask.len(), inputs.len());
    for (index, (input, output)) in inputs.iter().zip(outputs).enumerate() {
        if input.is_zero() {
            assert!(!mask.is_set(index));
            assert_eq!(*output, F::ZERO);
        } else {
            assert!(mask.is_set(index));
            assert_eq!(output.mul(*input), F::ONE);
            if !cfg!(miri) {
                assert_eq!(Some(*output), input.invert());
            }
        }
    }
}

fn assert_padding_is_clear(words: &[u64], len: usize) {
    if len == 0 || len.is_multiple_of(64) {
        return;
    }
    let padding_mask = !((1_u64 << (len % 64)) - 1);
    assert_eq!(words[words.len() - 1] & padding_mask, 0);
}

fn reference_scan<F: Field>(values: &[F], direction: ScanDirection, mode: ScanMode) -> Vec<F> {
    let mut out = vec![F::ZERO; values.len()];
    let mut accumulator = F::ONE;
    match direction {
        ScanDirection::Prefix => {
            for (index, value) in values.iter().enumerate() {
                if mode == ScanMode::Exclusive {
                    out[index] = accumulator;
                }
                accumulator = accumulator.mul(*value);
                if mode == ScanMode::Inclusive {
                    out[index] = accumulator;
                }
            }
        }
        ScanDirection::Suffix => {
            for index in (0..values.len()).rev() {
                if mode == ScanMode::Exclusive {
                    out[index] = accumulator;
                }
                accumulator = accumulator.mul(values[index]);
                if mode == ScanMode::Inclusive {
                    out[index] = accumulator;
                }
            }
        }
    }
    out
}

fn evaluate_polynomial<F: Field>(coefficients: &[F], point: F) -> F {
    coefficients
        .iter()
        .rev()
        .fold(F::ZERO, |value, coefficient| {
            value.mul(point).add(*coefficient)
        })
}

fn deterministic_nonzero<F: CanonicalEncoding>(seed: u64) -> F {
    let value: F = deterministic_value(seed);
    if value.is_zero() { F::ONE } else { value }
}

fn deterministic_value<F: CanonicalEncoding>(seed: u64) -> F {
    let mut repr = F::Repr::default();
    for (index, byte) in repr.as_mut().iter_mut().enumerate() {
        let rotation = u32::try_from(index % 64).expect("rotation is below 64");
        let index_u64 = u64::try_from(index).expect("field representation fits u64");
        let mixed = seed
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .rotate_left(rotation)
            .wrapping_add(index_u64.wrapping_mul(0xa5a5_5a5a_1f1f_e0e0));
        *byte = mixed.to_le_bytes()[index % 8];
    }
    F::from_canonical(&repr).expect("maintained binary encodings accept all fixed-width bits")
}
