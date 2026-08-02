//! Allocation-free H2.6 packed-view contracts.

#![cfg(all(feature = "builtin-fields", feature = "portable"))]

use core::mem::MaybeUninit;

use microfield::{
    BuiltinField, CanonicalEncoding, Engine, Field, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1,
    PackError, PackedLayout, StaticField, pack_into_storage, required_packed_bytes,
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

const NORMATIVE_SIZES: &[usize] = &[0, 1, 2, 3, 7, 8, 31, 32, 33, 255, 256, 1025];
const MIRI_SIZES: &[usize] = &[0, 1, 3, 8];

#[test]
fn borrowed_views_match_scalar_on_every_field_and_boundary_size() {
    assert_views::<Gf2_128V1>();
    assert_views::<Gf2_256HhV1>();
    assert_views::<Gf2_256AltV1>();
}

#[test]
fn every_possible_storage_offset_can_produce_an_aligned_view() {
    assert_offsets::<Gf2_128V1>();
    assert_offsets::<Gf2_256HhV1>();
    assert_offsets::<Gf2_256AltV1>();
}

#[test]
fn view_errors_are_transactional_and_empty_storage_is_valid() {
    let engine = Engine::<Gf2_128V1>::portable();
    let values = values::<Gf2_128V1>(3, 0x1234);
    let plan = engine.packing_plan(values.len()).expect("valid plan");

    let mut too_small = vec![MaybeUninit::new(0xa5); plan.data_bytes() - 1];
    assert!(matches!(
        pack_into_storage(&engine, &mut too_small, &values),
        Err(PackError::InsufficientStorage { .. })
    ));

    let capacity = required_packed_bytes(&plan).expect("small plan");
    let mut storage = vec![MaybeUninit::uninit(); capacity];
    let mut view = pack_into_storage(&engine, &mut storage, &values).expect("enough storage");
    let before = values.clone();
    assert_eq!(
        view.pack_from(&values[..2]),
        Err(PackError::LengthMismatch {
            expected: 3,
            actual: 2,
        })
    );
    let mut actual = vec![Gf2_128V1::ZERO; 3];
    view.unpack_into(&mut actual).expect("matching output");
    assert_eq!(actual, before);

    let mut short = vec![Gf2_128V1::ONE; 2];
    let short_before = short.clone();
    assert_eq!(
        view.unpack_into(&mut short),
        Err(PackError::LengthMismatch {
            expected: 3,
            actual: 2,
        })
    );
    assert_eq!(short, short_before);

    let empty_plan = engine.packing_plan(0).expect("empty plan");
    assert_eq!(required_packed_bytes(&empty_plan), Ok(0));
    let empty = pack_into_storage(&engine, &mut [], &[]).expect("empty storage is valid");
    assert!(empty.is_empty());
}

#[test]
fn incompatible_view_lengths_leave_output_untouched() {
    let engine = Engine::<Gf2_256HhV1>::portable();
    let lhs_values = values::<Gf2_256HhV1>(3, 1);
    let rhs_values = values::<Gf2_256HhV1>(4, 2);
    let sentinel = values::<Gf2_256HhV1>(3, 3);
    let mut lhs_storage = storage_for(&engine, lhs_values.len());
    let mut rhs_storage = storage_for(&engine, rhs_values.len());
    let mut out_storage = storage_for(&engine, sentinel.len());
    let lhs = pack_into_storage(&engine, &mut lhs_storage, &lhs_values).expect("lhs");
    let rhs = pack_into_storage(&engine, &mut rhs_storage, &rhs_values).expect("rhs");
    let mut out = pack_into_storage(&engine, &mut out_storage, &sentinel).expect("out");

    assert_eq!(
        engine.mul_packed_view_into(&mut out, &lhs.as_view(), &rhs.as_view()),
        Err(PackError::IncompatiblePlan)
    );
    let mut actual = vec![Gf2_256HhV1::ZERO; sentinel.len()];
    out.unpack_into(&mut actual).expect("matching output");
    assert_eq!(actual, sentinel);
}

#[test]
fn plan_identity_and_overflow_are_explicit() {
    let engine = Engine::<Gf2_128V1>::portable();
    let plan = engine.packing_plan(7).expect("valid plan");
    assert_eq!(plan.backend_id(), engine.backend_id());
    assert_eq!(plan.field_id(), Gf2_128V1::spec().field_id());
    assert_eq!(plan.layout(), PackedLayout::Aos);
    assert_eq!(plan.logical_len(), 7);
    assert_eq!(plan.padded_len(), 7);
    assert_eq!(plan.tile_elements(), 1);
    assert_eq!(plan.limb_count(), 2);
    assert_eq!(plan.element_size(), core::mem::size_of::<Gf2_128V1>());
    assert_eq!(plan.alignment(), core::mem::align_of::<Gf2_128V1>());
    assert_eq!(plan.data_bytes(), 7 * core::mem::size_of::<Gf2_128V1>());
    assert_eq!(
        engine.packing_plan(usize::MAX),
        Err(PackError::SizeOverflow)
    );
}

fn assert_views<F: PackedField>() {
    let engine = Engine::<F>::portable();
    let sizes = if cfg!(miri) {
        MIRI_SIZES
    } else {
        NORMATIVE_SIZES
    };
    for &len in sizes {
        let lhs_values = values::<F>(len, 0x243f_6a88_85a3_08d3);
        let rhs_values = values::<F>(len, 0x1319_8a2e_0370_7344);
        let mut lhs_storage = storage_for(&engine, len);
        let mut rhs_storage = storage_for(&engine, len);
        let mut out_storage = storage_for(&engine, len);
        let mut assign_storage = storage_for(&engine, len);
        let lhs = pack_into_storage(&engine, &mut lhs_storage, &lhs_values).expect("lhs pack");
        let rhs = pack_into_storage(&engine, &mut rhs_storage, &rhs_values).expect("rhs pack");
        let mut out =
            pack_into_storage(&engine, &mut out_storage, &vec![F::ZERO; len]).expect("out pack");
        let mut assigned =
            pack_into_storage(&engine, &mut assign_storage, &lhs_values).expect("assign pack");

        engine
            .add_packed_view_into(&mut out, &lhs.as_view(), &rhs.as_view())
            .expect("compatible plans");
        let mut actual = vec![F::ZERO; len];
        out.unpack_into(&mut actual).expect("matching output");
        assert_eq!(
            actual,
            lhs_values
                .iter()
                .zip(&rhs_values)
                .map(|(left, right)| left.add(*right))
                .collect::<Vec<_>>()
        );

        engine
            .mul_packed_view_into(&mut out, &lhs.as_view(), &rhs.as_view())
            .expect("compatible plans");
        out.unpack_into(&mut actual).expect("matching output");
        let expected: Vec<_> = lhs_values
            .iter()
            .zip(&rhs_values)
            .map(|(left, right)| left.mul(*right))
            .collect();
        assert_eq!(actual, expected);

        engine
            .square_packed_view_into(&mut out, &lhs.as_view())
            .expect("compatible plans");
        out.unpack_into(&mut actual).expect("matching output");
        assert_eq!(
            actual,
            lhs_values
                .iter()
                .map(|value| value.square())
                .collect::<Vec<_>>()
        );

        engine
            .mul_packed_view_assign(&mut assigned, &rhs.as_view())
            .expect("compatible plans");
        assigned.unpack_into(&mut actual).expect("matching output");
        assert_eq!(actual, expected);
        engine
            .square_packed_view_assign(&mut assigned)
            .expect("matching backend");
        assigned.unpack_into(&mut actual).expect("matching output");
        assert_eq!(
            actual,
            expected
                .iter()
                .map(|value| value.square())
                .collect::<Vec<_>>()
        );
    }
}

fn assert_offsets<F: PackedField>() {
    let engine = Engine::<F>::portable();
    let values = values::<F>(5, 0xdead_beef);
    let plan = engine.packing_plan(values.len()).expect("valid plan");
    let required = required_packed_bytes(&plan).expect("bounded plan");
    for offset in 0..plan.alignment() {
        let mut storage = vec![MaybeUninit::uninit(); required + plan.alignment()];
        let view = pack_into_storage(&engine, &mut storage[offset..offset + required], &values)
            .expect("worst-case capacity handles every base offset");
        let mut actual = vec![F::ZERO; values.len()];
        view.unpack_into(&mut actual).expect("matching output");
        assert_eq!(actual, values);
    }
}

fn storage_for<F: PackedField>(engine: &Engine<F>, len: usize) -> Vec<MaybeUninit<u8>> {
    let plan = engine.packing_plan(len).expect("valid plan");
    vec![MaybeUninit::uninit(); required_packed_bytes(&plan).expect("bounded plan")]
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
