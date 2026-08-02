//! Persistent differential corpus shared by every maintained ISA boundary.

#![cfg(all(feature = "std", feature = "portable", feature = "builtin-fields"))]

use core::fmt::Debug;

use microfield::{
    BackendId, BuiltinField, CanonicalEncoding, Engine, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1,
    StaticField,
};

const CORPUS: &str = include_str!("../test-data/differential-corpus-v1.csv");

#[derive(Clone, Copy, Debug)]
struct Case<'a> {
    id: &'a str,
    len: usize,
    lhs_seed: u64,
    rhs_seed: u64,
}

#[test]
fn persistent_corpus_matches_every_available_backend_bit_for_bit() {
    let cases = parse_corpus();
    assert_eq!(cases.len(), 20);
    exercise_field::<Gf2_128V1>(&cases);
    exercise_field::<Gf2_256HhV1>(&cases);
    exercise_field::<Gf2_256AltV1>(&cases);
}

fn exercise_field<F>(cases: &[Case<'_>])
where
    F: BuiltinField + CanonicalEncoding + StaticField + Debug,
{
    let portable = Engine::<F>::portable();
    let backends = available_backends::<F>();

    for case in cases {
        if cfg!(miri) && case.len > 17 {
            continue;
        }
        let lhs = values::<F>(case.len, case.lhs_seed);
        let rhs = values::<F>(case.len, case.rhs_seed);
        let expected_mul = lhs
            .iter()
            .zip(&rhs)
            .map(|(left, right)| left.mul(*right))
            .collect::<Vec<_>>();
        let expected_square = lhs.iter().map(|value| value.square()).collect::<Vec<_>>();

        assert_engine(portable, case, &lhs, &rhs, &expected_mul, &expected_square);
        for engine in backends.iter().copied() {
            assert_engine(engine, case, &lhs, &rhs, &expected_mul, &expected_square);
        }
    }
}

fn assert_engine<F>(
    engine: Engine<F>,
    case: &Case<'_>,
    lhs: &[F],
    rhs: &[F],
    expected_mul: &[F],
    expected_square: &[F],
) where
    F: BuiltinField + Debug,
{
    let mut actual = vec![F::ZERO; case.len];
    engine
        .mul_into(&mut actual, lhs, rhs)
        .expect("corpus lengths match");
    assert_equal_with_reproducer(engine, case, "mul", &actual, expected_mul);

    engine
        .square_into(&mut actual, lhs)
        .expect("corpus lengths match");
    assert_equal_with_reproducer(engine, case, "square", &actual, expected_square);

    let mut assigned = lhs.to_vec();
    engine
        .mul_assign(&mut assigned, rhs)
        .expect("corpus lengths match");
    assert_equal_with_reproducer(engine, case, "mul_assign", &assigned, expected_mul);

    assigned.copy_from_slice(lhs);
    engine.square_assign(&mut assigned);
    assert_equal_with_reproducer(engine, case, "square_assign", &assigned, expected_square);
}

fn assert_equal_with_reproducer<F>(
    engine: Engine<F>,
    case: &Case<'_>,
    operation: &str,
    actual: &[F],
    expected: &[F],
) where
    F: Debug + Eq + microfield::__private::PortableField,
{
    if let Some(index) = actual
        .iter()
        .zip(expected)
        .position(|(actual, expected)| actual != expected)
    {
        panic!(
            "differential mismatch: field={} backend={:?} operation={operation} case={} \
             index={index}; minimized reproducer: length={} lhs_seed={:016x} rhs_seed={:016x}",
            core::any::type_name::<F>(),
            engine.backend_id(),
            case.id,
            index + 1,
            case.lhs_seed,
            case.rhs_seed,
        );
    }
}

fn available_backends<F: BuiltinField>() -> Vec<Engine<F>> {
    let mut engines = Vec::new();
    #[cfg(target_arch = "x86_64")]
    for backend in [BackendId::X86Pclmul, BackendId::X86Vpclmul] {
        if let Ok(engine) = Engine::<F>::builder().force_backend(backend).detect() {
            engines.push(engine);
        }
    }
    #[cfg(target_arch = "aarch64")]
    if let Ok(engine) = Engine::<F>::builder()
        .force_backend(BackendId::Aarch64Pmull)
        .detect()
    {
        engines.push(engine);
    }
    engines
}

fn values<F: CanonicalEncoding + StaticField>(len: usize, seed: u64) -> Vec<F> {
    let bytes = usize::from(F::spec().canonical_bytes());
    let mut state = seed;
    (0..len)
        .map(|index| {
            let mut repr = vec![0_u8; bytes];
            for (offset, byte) in repr.iter_mut().enumerate() {
                state = splitmix64(state.wrapping_add((index ^ offset) as u64));
                *byte = state.to_le_bytes()[offset % 8];
            }
            F::from_canonical_slice(&repr).expect("full-width built-in encoding is canonical")
        })
        .collect()
}

const fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn parse_corpus() -> Vec<Case<'static>> {
    CORPUS
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut columns = line.split(',');
            let id = columns.next().expect("case id");
            let len = columns
                .next()
                .expect("length")
                .parse()
                .expect("decimal length");
            let lhs_seed =
                u64::from_str_radix(columns.next().expect("lhs seed"), 16).expect("hex lhs seed");
            let rhs_seed =
                u64::from_str_radix(columns.next().expect("rhs seed"), 16).expect("hex rhs seed");
            assert!(columns.next().is_some(), "purpose is mandatory");
            assert!(columns.next().is_none(), "unexpected corpus column");
            Case {
                id,
                len,
                lhs_seed,
                rhs_seed,
            }
        })
        .collect()
}
