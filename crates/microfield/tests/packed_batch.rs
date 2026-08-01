//! Owned H2.6 `PackedBatch` contracts.

#![cfg(all(feature = "alloc", feature = "builtin-fields", feature = "portable"))]

#[cfg(feature = "std")]
use microfield::BackendId;
use microfield::{
    BuiltinField, CanonicalEncoding, Engine, Field, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1,
    PackError, PackedBatch, StaticField,
};

trait PackedField: BuiltinField + CanonicalEncoding + StaticField + core::fmt::Debug {
    const BYTES: usize;
}

impl PackedField for Gf2_128V1 {
    const BYTES: usize = 16;
}

impl PackedField for Gf2_256HhV1 {
    const BYTES: usize = 32;
}

impl PackedField for Gf2_256AltV1 {
    const BYTES: usize = 32;
}

const SIZES: &[usize] = &[0, 1, 2, 3, 7, 8, 31, 32, 33, 257];

#[test]
fn owned_packed_operations_match_scalar_for_all_fields() {
    assert_owned::<Gf2_128V1>();
    assert_owned::<Gf2_256HhV1>();
    assert_owned::<Gf2_256AltV1>();
}

#[test]
fn owned_batches_are_send_sync_when_the_field_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PackedBatch<Gf2_128V1>>();
    assert_send_sync::<PackedBatch<Gf2_256HhV1>>();
    assert_send_sync::<PackedBatch<Gf2_256AltV1>>();
}

#[test]
fn conversion_errors_and_incompatible_plans_are_transactional() {
    let engine = Engine::<Gf2_128V1>::portable();
    let original = values::<Gf2_128V1>(3, 1);
    let mut packed = PackedBatch::from_aos(&engine, &original).expect("packed batch");
    assert_eq!(
        packed.pack_from(&original[..2]),
        Err(PackError::LengthMismatch {
            expected: 3,
            actual: 2,
        })
    );
    let mut actual = vec![Gf2_128V1::ZERO; 3];
    packed.unpack_into(&mut actual).expect("matching output");
    assert_eq!(actual, original);

    let mut short = vec![Gf2_128V1::ONE; 2];
    let short_before = short.clone();
    assert_eq!(
        packed.unpack_into(&mut short),
        Err(PackError::LengthMismatch {
            expected: 3,
            actual: 2,
        })
    );
    assert_eq!(short, short_before);

    let lhs = PackedBatch::from_aos(&engine, &original).expect("lhs");
    let rhs = PackedBatch::from_aos(&engine, &values::<Gf2_128V1>(4, 2)).expect("rhs");
    let sentinel = values::<Gf2_128V1>(3, 3);
    let mut out = PackedBatch::from_aos(&engine, &sentinel).expect("out");
    assert_eq!(
        engine.mul_packed_into(&mut out, &lhs, &rhs),
        Err(PackError::IncompatiblePlan)
    );
    out.unpack_into(&mut actual).expect("matching output");
    assert_eq!(actual, sentinel);
}

#[test]
#[cfg(feature = "std")]
fn a_batch_cannot_be_executed_by_an_engine_with_another_backend() {
    let portable = Engine::<Gf2_128V1>::portable();
    let Some(isa) = detected_isa::<Gf2_128V1>() else {
        return;
    };
    let operands = values::<Gf2_128V1>(8, 7);
    let lhs = PackedBatch::from_aos(&portable, &operands).expect("lhs");
    let rhs = PackedBatch::from_aos(&portable, &operands).expect("rhs");
    let sentinel = values::<Gf2_128V1>(8, 11);
    let mut out = PackedBatch::from_aos(&portable, &sentinel).expect("out");
    assert_eq!(
        isa.mul_packed_into(&mut out, &lhs, &rhs),
        Err(PackError::WrongBackend {
            expected: isa.backend_id(),
            actual: BackendId::Portable,
        })
    );
    let mut actual = vec![Gf2_128V1::ZERO; sentinel.len()];
    out.unpack_into(&mut actual).expect("matching output");
    assert_eq!(actual, sentinel);
}

fn assert_owned<F: PackedField>() {
    let portable = Engine::<F>::portable();
    assert_owned_engine(portable);
    #[cfg(feature = "std")]
    if let Some(isa) = detected_isa::<F>() {
        assert_owned_engine(isa);
    }
}

fn assert_owned_engine<F: PackedField>(engine: Engine<F>) {
    for &len in SIZES {
        let lhs_values = values::<F>(len, 0x243f_6a88_85a3_08d3);
        let rhs_values = values::<F>(len, 0x1319_8a2e_0370_7344);
        let lhs = PackedBatch::from_aos(&engine, &lhs_values).expect("lhs");
        let rhs = PackedBatch::from_aos(&engine, &rhs_values).expect("rhs");
        let mut out = PackedBatch::new(&engine, len).expect("output");
        assert_eq!(out.backend_id(), engine.backend_id());
        assert_eq!(out.len(), len);
        assert_eq!(out.is_empty(), len == 0);

        engine
            .mul_packed_into(&mut out, &lhs, &rhs)
            .expect("compatible plans");
        let expected_product: Vec<_> = lhs_values
            .iter()
            .zip(&rhs_values)
            .map(|(left, right)| left.mul(*right))
            .collect();
        let mut actual = vec![F::ZERO; len];
        out.unpack_into(&mut actual).expect("matching output");
        assert_eq!(actual, expected_product);

        engine
            .square_packed_into(&mut out, &lhs)
            .expect("compatible plans");
        out.unpack_into(&mut actual).expect("matching output");
        assert_eq!(
            actual,
            lhs_values
                .iter()
                .map(|value| value.square())
                .collect::<Vec<_>>()
        );

        let mut assigned = PackedBatch::from_aos(&engine, &lhs_values).expect("assigned");
        engine
            .mul_packed_assign(&mut assigned, &rhs)
            .expect("compatible plans");
        assigned.unpack_into(&mut actual).expect("matching output");
        assert_eq!(actual, expected_product);
        engine
            .square_packed_assign(&mut assigned)
            .expect("matching backend");
        assigned.unpack_into(&mut actual).expect("matching output");
        assert_eq!(
            actual,
            expected_product
                .iter()
                .map(|value| value.square())
                .collect::<Vec<_>>()
        );
    }
}

#[cfg(feature = "std")]
fn detected_isa<F: PackedField>() -> Option<Engine<F>> {
    #[cfg(target_arch = "x86_64")]
    let backend = BackendId::X86Pclmul;
    #[cfg(target_arch = "aarch64")]
    let backend = BackendId::Aarch64Pmull;
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    return None;

    Engine::<F>::builder().force_backend(backend).detect().ok()
}

fn values<F: PackedField>(len: usize, seed: u64) -> Vec<F> {
    (0..len).map(|index| value::<F>(seed, index)).collect()
}

fn value<F: PackedField>(mut state: u64, index: usize) -> F {
    state ^= (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut bytes = vec![0; F::BYTES];
    for chunk in bytes.chunks_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        chunk.copy_from_slice(&state.to_le_bytes()[..chunk.len()]);
    }
    F::from_canonical_slice(&bytes).expect("full-width binary values are canonical")
}
