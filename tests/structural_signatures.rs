//! Corrected algebraic contracts for the pre-canonization Phase 6 layer.

use allocation_counter::measure;
use homomorphic_hash_rs::topology::{
    multiset::MultisetAggregator as LegacyMultiset, sequence::SequenceAggregator as LegacySequence,
    traits::HomomorphicAggregator,
};
use homomorphic_hash_rs::{
    AdditiveSignature, BidirectionalSequenceSignature, BinaryPolynomialEncoder,
    CanonicalElementEncoder, FiniteField as _, GaloisSignature256, LegacyAffineEncoderV1,
    LegacyLinearEncoderV1, MultiEvaluationMultisetSignature, MultisetSignature,
    PrimeIntegerEncoder, SequenceSignature, SignatureError, StructuralEncoder,
    SymmetricDifferenceAggregator, TrackedMultiset, TrackedSequence,
};
use microfield::{
    BinaryPolynomialField, CanonicalEncoding, Field, Fp251V1, Fp256GenericV1, FpGoldilocks64V1,
    Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, Invert, Pow, PrimeField, StaticField,
};
use structural_field_fixture::Gf2_9StructuralFixture;

#[cfg(feature = "dynamic-fields")]
use homomorphic_hash_rs::{
    DynamicAdditiveSignature, DynamicBidirectionalSequenceSignature,
    DynamicMultiEvaluationMultisetSignature, DynamicMultisetSignature, DynamicSequenceSignature,
};
#[cfg(feature = "dynamic-fields")]
use microfield::DynField;

fn binary_base() -> Gf2_256HhV1 {
    Gf2_256HhV1::ONE.mul_by_x()
}

#[test]
fn encoder_and_signature_identities_bind_every_semantic_choice() {
    let first = BinaryPolynomialEncoder::new(7);
    let same_with_stricter_limit = first.with_maximum_input_bytes(8);
    let other_domain = BinaryPolynomialEncoder::new(8);
    assert_eq!(first.id(), same_with_stricter_limit.id());
    assert_ne!(first.id(), other_domain.id());

    let additive = AdditiveSignature::<Gf2_256HhV1, _>::new(first);
    let same_additive = AdditiveSignature::<Gf2_256HhV1, _>::new(same_with_stricter_limit);
    let sequence = SequenceSignature::<Gf2_256HhV1, _>::new(first, binary_base()).unwrap();
    let other_field = AdditiveSignature::<Gf2_128V1, _>::new(BinaryPolynomialEncoder::new(7));
    let bidirectional =
        BidirectionalSequenceSignature::<Gf2_256HhV1, _>::new(first, binary_base()).unwrap();
    let multi_forward = MultiEvaluationMultisetSignature::<Gf2_256HhV1, _, 2>::new(
        first,
        [Gf2_256HhV1::ONE, binary_base()],
    )
    .unwrap();
    let multi_reordered = MultiEvaluationMultisetSignature::<Gf2_256HhV1, _, 2>::new(
        first,
        [binary_base(), Gf2_256HhV1::ONE],
    )
    .unwrap();
    assert_ne!(
        additive.context().signature_id(),
        sequence.context().signature_id()
    );
    assert_ne!(
        additive.context().signature_id(),
        other_field.context().signature_id()
    );
    assert_eq!(additive, same_additive);
    assert_ne!(
        sequence.context().signature_id(),
        bidirectional.context().signature_id()
    );
    assert_ne!(
        multi_forward.context().signature_id(),
        multi_reordered.context().signature_id()
    );
}

#[test]
fn framed_binary_encoder_distinguishes_empty_and_trailing_zero_inputs_before_reduction() {
    let encoder = BinaryPolynomialEncoder::new(0x5354_5255_4354_0001);
    let empty: Gf2_256HhV1 = encoder.encode(b"").unwrap();
    let zero: Gf2_256HhV1 = encoder.encode(&[0]).unwrap();
    let two_zeros: Gf2_256HhV1 = encoder.encode(&[0, 0]).unwrap();
    assert_ne!(empty, zero);
    assert_ne!(zero, two_zeros);

    let limited = encoder.with_maximum_input_bytes(1);
    assert!(matches!(
        <BinaryPolynomialEncoder as StructuralEncoder<Gf2_256HhV1>>::encode(&limited, &[1, 2]),
        Err(SignatureError::InputTooLarge { .. })
    ));
}

#[test]
fn additive_signature_is_partition_homomorphic_and_reports_parity_count_separately() {
    let encoder = BinaryPolynomialEncoder::new(11);
    let mut all = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
    let mut left = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
    let mut right = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
    for value in [b"alpha".as_slice(), b"beta", b"gamma", b"delta"] {
        all.absorb(value).unwrap();
    }
    for value in [b"alpha".as_slice(), b"beta"] {
        left.absorb(value).unwrap();
    }
    for value in [b"gamma".as_slice(), b"delta"] {
        right.absorb(value).unwrap();
    }
    assert_eq!(left.combine(&right).unwrap(), all);

    let mut parity = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
    parity.absorb(b"same").unwrap();
    parity.absorb(b"same").unwrap();
    assert_eq!(parity.state(), Gf2_256HhV1::ZERO);
    assert_eq!(parity.term_count(), 2);
}

#[test]
fn sequence_concatenation_length_and_checked_tracking_are_exact() {
    let encoder = BinaryPolynomialEncoder::new(12);
    let base = binary_base();
    let mut prefix = SequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    let mut suffix = SequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    let mut complete = SequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    for value in [b"a".as_slice(), b"b"] {
        prefix.push(value).unwrap();
        complete.push(value).unwrap();
    }
    for value in [b"c".as_slice(), b"d", b"e"] {
        suffix.push(value).unwrap();
        complete.push(value).unwrap();
    }
    assert_eq!(prefix.concatenate(&suffix).unwrap(), complete);
    assert_eq!(complete.len(), 5);

    let mut reversed = SequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    for value in [b"e".as_slice(), b"d", b"c", b"b", b"a"] {
        reversed.push(value).unwrap();
    }
    assert_ne!(complete.state(), reversed.state());

    let residual = complete.residual_assuming_last(b"e").unwrap();
    assert!(complete.verify_residual(b"e", &residual).unwrap());
    let fabricated = complete.residual_assuming_last(b"not-last").unwrap();
    assert!(complete.verify_residual(b"not-last", &fabricated).unwrap());

    let mut tracked = TrackedSequence::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    tracked.push(b"a").unwrap();
    tracked.push(b"b").unwrap();
    assert_eq!(tracked.pop().unwrap(), b"b");
    assert_eq!(tracked.signature().len(), 1);
}

#[test]
fn multiset_preserves_zero_factors_and_exact_tracked_multiplicity() {
    let encoder = CanonicalElementEncoder;
    let offset = Fp251V1::ONE;
    let zero_factor_input = [250_u8];
    let mut compact = MultisetSignature::<Fp251V1, _>::new(encoder, offset);
    compact.insert(&[2]).unwrap();
    compact.insert(&zero_factor_input).unwrap();
    compact.insert(&zero_factor_input).unwrap();
    assert_eq!(compact.cardinality(), 3);
    assert_eq!(compact.zero_factor_count(), 2);
    assert_eq!(compact.evaluated_product(), Fp251V1::ZERO);
    assert_ne!(compact.nonzero_product(), Fp251V1::ZERO);

    let residual = compact
        .residual_assuming_member(&zero_factor_input)
        .unwrap();
    assert_eq!(residual.zero_factor_count(), 1);
    assert!(compact
        .verify_residual(&zero_factor_input, &residual)
        .unwrap());

    let mut tracked = TrackedMultiset::<Fp251V1, _>::new(encoder, offset);
    tracked.insert(&zero_factor_input).unwrap();
    tracked.insert(&zero_factor_input).unwrap();
    assert_eq!(tracked.multiplicity(&zero_factor_input), 2);
    tracked.remove(&zero_factor_input).unwrap();
    assert_eq!(tracked.multiplicity(&zero_factor_input), 1);
    tracked.remove(&zero_factor_input).unwrap();
    assert_eq!(tracked.multiplicity(&zero_factor_input), 0);
    assert_eq!(tracked.signature().evaluated_product(), Fp251V1::ONE);
}

#[test]
fn algebraic_multiset_residual_is_not_a_membership_proof() {
    let encoder = CanonicalElementEncoder;
    let mut compact = MultisetSignature::<Fp251V1, _>::new(encoder, Fp251V1::ONE);
    compact.insert(&[3]).unwrap();

    let never_inserted = [77_u8];
    let fabricated = compact.residual_assuming_member(&never_inserted).unwrap();
    assert!(compact
        .verify_residual(&never_inserted, &fabricated)
        .unwrap());

    let mut tracked = TrackedMultiset::<Fp251V1, _>::new(encoder, Fp251V1::ONE);
    tracked.insert(&[3]).unwrap();
    let before = tracked.clone();
    assert_eq!(
        tracked.remove(&never_inserted),
        Err(SignatureError::ItemAbsent)
    );
    assert_eq!(tracked, before);
}

#[test]
fn multiset_is_commutative_and_partition_homomorphic() {
    let encoder = CanonicalElementEncoder;
    let offset = Fp251V1::from_u64_mod(17);
    let mut all = MultisetSignature::<Fp251V1, _>::new(encoder, offset);
    let mut reversed = MultisetSignature::<Fp251V1, _>::new(encoder, offset);
    let mut left = MultisetSignature::<Fp251V1, _>::new(encoder, offset);
    let mut right = MultisetSignature::<Fp251V1, _>::new(encoder, offset);
    for value in [2_u8, 3, 5, 7] {
        all.insert(&[value]).unwrap();
    }
    for value in [7_u8, 5, 3, 2] {
        reversed.insert(&[value]).unwrap();
    }
    for value in [2_u8, 3] {
        left.insert(&[value]).unwrap();
    }
    for value in [5_u8, 7] {
        right.insert(&[value]).unwrap();
    }
    assert_eq!(all, reversed);
    assert_eq!(left.combine(&right).unwrap(), all);
}

#[test]
fn canonical_envelopes_round_trip_and_fail_closed() {
    let encoder = CanonicalElementEncoder;
    let mut additive = AdditiveSignature::<Fp251V1, _>::new(encoder);
    additive.absorb(&[7]).unwrap();
    let encoded = additive.to_canonical_bytes();
    assert_eq!(
        AdditiveSignature::<Fp251V1, _>::from_canonical_bytes(encoder, &encoded).unwrap(),
        additive
    );

    let mut identity_drift = encoded.clone();
    identity_drift[40] ^= 1;
    assert_eq!(
        AdditiveSignature::<Fp251V1, _>::from_canonical_bytes(encoder, &identity_drift),
        Err(SignatureError::IdentityMismatch)
    );

    let mut noncanonical = encoded.clone();
    *noncanonical.last_mut().unwrap() = 251;
    assert_eq!(
        AdditiveSignature::<Fp251V1, _>::from_canonical_bytes(encoder, &noncanonical),
        Err(SignatureError::NonCanonicalElement)
    );

    let mut trailing = encoded;
    trailing.push(0);
    assert!(matches!(
        AdditiveSignature::<Fp251V1, _>::from_canonical_bytes(encoder, &trailing),
        Err(SignatureError::InvalidWireFormat(_))
    ));
}

#[test]
fn every_wire_header_byte_is_semantically_bound_or_reserved() {
    const HEADER_BYTES: usize = 104;
    let encoder = CanonicalElementEncoder;
    let mut signature = AdditiveSignature::<Fp251V1, _>::new(encoder);
    signature.absorb(&[7]).unwrap();
    let encoded = signature.to_canonical_bytes();

    for index in 0..HEADER_BYTES {
        let mut changed = encoded.clone();
        changed[index] ^= 1;
        assert!(
            AdditiveSignature::<Fp251V1, _>::from_canonical_bytes(encoder, &changed).is_err(),
            "header byte {index} was not validated"
        );
    }
}

#[test]
fn counter_overflow_is_transactional_after_canonical_restore() {
    let encoder = CanonicalElementEncoder;
    let empty = AdditiveSignature::<Fp251V1, _>::new(encoder);
    let mut encoded = empty.to_canonical_bytes();
    encoded[104..112].copy_from_slice(&u64::MAX.to_le_bytes());
    let mut saturated =
        AdditiveSignature::<Fp251V1, _>::from_canonical_bytes(encoder, &encoded).unwrap();
    let before = saturated.clone();
    assert_eq!(saturated.absorb(&[1]), Err(SignatureError::CounterOverflow));
    assert_eq!(saturated, before);
}

#[test]
fn sequence_and_multiset_overflow_and_degenerate_base_fail_before_mutation() {
    let encoder = CanonicalElementEncoder;
    assert_eq!(
        SequenceSignature::<Fp251V1, _>::new(encoder, Fp251V1::ZERO),
        Err(SignatureError::DegenerateSequenceBase)
    );
    assert_eq!(
        SequenceSignature::<Fp251V1, _>::new(encoder, Fp251V1::ONE),
        Err(SignatureError::DegenerateSequenceBase)
    );

    let base = Fp251V1::from_u64_mod(7);
    let empty_sequence = SequenceSignature::<Fp251V1, _>::new(encoder, base).unwrap();
    let mut sequence_bytes = empty_sequence.to_canonical_bytes();
    sequence_bytes[104..112].copy_from_slice(&u64::MAX.to_le_bytes());
    let mut sequence =
        SequenceSignature::<Fp251V1, _>::from_canonical_bytes(encoder, base, &sequence_bytes)
            .unwrap();
    let before = sequence.clone();
    assert_eq!(sequence.push(&[1]), Err(SignatureError::CounterOverflow));
    assert_eq!(sequence, before);

    let empty_multiset = MultisetSignature::<Fp251V1, _>::new(encoder, Fp251V1::ONE);
    let mut multiset_bytes = empty_multiset.to_canonical_bytes();
    multiset_bytes[104..112].copy_from_slice(&u64::MAX.to_le_bytes());
    let mut multiset = MultisetSignature::<Fp251V1, _>::from_canonical_bytes(
        encoder,
        Fp251V1::ONE,
        &multiset_bytes,
    )
    .unwrap();
    let before = multiset.clone();
    assert_eq!(multiset.insert(&[1]), Err(SignatureError::CounterOverflow));
    assert_eq!(multiset, before);
}

#[test]
fn incompatible_runtime_encoder_parameters_never_combine() {
    let mut first = AdditiveSignature::<Gf2_256HhV1, _>::new(BinaryPolynomialEncoder::new(1));
    let mut second = AdditiveSignature::<Gf2_256HhV1, _>::new(BinaryPolynomialEncoder::new(2));
    first.absorb(b"x").unwrap();
    second.absorb(b"x").unwrap();
    assert_eq!(
        first.combine(&second),
        Err(SignatureError::IdentityMismatch)
    );
}

#[test]
fn tracked_multiset_keeps_exact_raw_membership_across_field_collisions() {
    let encoder = PrimeIntegerEncoder::new(99);
    let mut first_by_value = [None; 251];
    let mut collision = None;
    for raw in 0_u16..=255 {
        let bytes = [u8::try_from(raw).unwrap()];
        let encoded: Fp251V1 = encoder.encode(&bytes).unwrap();
        let value = usize::from(encoded.to_canonical()[0]);
        if let Some(previous) = first_by_value[value] {
            collision = Some((previous, bytes[0]));
            break;
        }
        first_by_value[value] = Some(bytes[0]);
    }
    let (left, right) = collision.expect("256 inputs in a 251-element field must collide");
    assert_ne!(left, right);

    let mut tracked = TrackedMultiset::<Fp251V1, _>::new(encoder, Fp251V1::ONE);
    tracked.insert(&[left]).unwrap();
    assert_eq!(tracked.multiplicity(&[left]), 1);
    assert_eq!(tracked.multiplicity(&[right]), 0);
    let before = tracked.clone();
    assert_eq!(tracked.remove(&[right]), Err(SignatureError::ItemAbsent));
    assert_eq!(tracked, before);
}

#[test]
fn sequence_and_multiset_envelopes_round_trip_and_reject_impossible_metadata() {
    let encoder = CanonicalElementEncoder;
    let base = Fp251V1::from_u64_mod(7);
    let mut sequence = SequenceSignature::<Fp251V1, _>::new(encoder, base).unwrap();
    sequence.push(&[3]).unwrap();
    sequence.push(&[5]).unwrap();
    let bytes = sequence.to_canonical_bytes();
    assert_eq!(
        SequenceSignature::<Fp251V1, _>::from_canonical_bytes(encoder, base, &bytes).unwrap(),
        sequence
    );

    let mut multiset = MultisetSignature::<Fp251V1, _>::new(encoder, Fp251V1::ONE);
    multiset.insert(&[2]).unwrap();
    let bytes = multiset.to_canonical_bytes();
    assert_eq!(
        MultisetSignature::<Fp251V1, _>::from_canonical_bytes(encoder, Fp251V1::ONE, &bytes)
            .unwrap(),
        multiset
    );

    let mut impossible = bytes;
    impossible[104..112].copy_from_slice(&1_u64.to_le_bytes());
    impossible[112..120].copy_from_slice(&2_u64.to_le_bytes());
    assert!(matches!(
        MultisetSignature::<Fp251V1, _>::from_canonical_bytes(encoder, Fp251V1::ONE, &impossible),
        Err(SignatureError::InvalidWireFormat(_))
    ));
}

#[test]
fn exhaustive_fp251_zero_factor_accounting_is_total_for_every_offset() {
    let encoder = CanonicalElementEncoder;
    for offset in 0_u16..251 {
        let offset = Fp251V1::from_u64_mod(u64::from(offset));
        let mut signature = MultisetSignature::<Fp251V1, _>::new(encoder, offset);
        for value in 0_u16..251 {
            signature.insert(&[u8::try_from(value).unwrap()]).unwrap();
        }
        assert_eq!(signature.cardinality(), 251);
        assert_eq!(signature.zero_factor_count(), 1);
        assert_eq!(signature.evaluated_product(), Fp251V1::ZERO);
        assert_ne!(signature.nonzero_product(), Fp251V1::ZERO);
    }
}

#[test]
fn corrected_laws_generalize_over_every_maintained_field_family() {
    check_binary_family::<Gf2_128V1>();
    check_binary_family::<Gf2_256HhV1>();
    check_binary_family::<Gf2_256AltV1>();
    check_prime_family::<Fp251V1>();
    check_prime_family::<FpGoldilocks64V1>();
    check_prime_family::<Fp256GenericV1>();
}

#[test]
fn generated_external_binary_field_uses_every_signature_without_adapters() {
    let element = |raw: u16| {
        Gf2_9StructuralFixture::from_canonical(&raw.to_le_bytes()).expect("value is below 2^9")
    };
    let values = [element(1), element(0x101), element(0x1ff), element(0x55)];
    let canonical = CanonicalElementEncoder;

    let mut additive = AdditiveSignature::<Gf2_9StructuralFixture, _>::new(canonical);
    additive.absorb_elements(values).unwrap();
    let mut left = AdditiveSignature::<Gf2_9StructuralFixture, _>::new(canonical);
    left.absorb_elements(values[..2].iter().copied()).unwrap();
    let mut right = AdditiveSignature::<Gf2_9StructuralFixture, _>::new(canonical);
    right.absorb_elements(values[2..].iter().copied()).unwrap();
    assert_eq!(left.combine(&right).unwrap(), additive);

    let base = Gf2_9StructuralFixture::ONE.mul_by_x();
    let mut sequence =
        SequenceSignature::<Gf2_9StructuralFixture, _>::new(canonical, base).unwrap();
    sequence.push_elements(values).unwrap();
    let mut prefix = SequenceSignature::<Gf2_9StructuralFixture, _>::new(canonical, base).unwrap();
    prefix.push_elements(values[..2].iter().copied()).unwrap();
    let mut suffix = SequenceSignature::<Gf2_9StructuralFixture, _>::new(canonical, base).unwrap();
    suffix.push_elements(values[2..].iter().copied()).unwrap();
    assert_eq!(prefix.concatenate(&suffix).unwrap(), sequence);

    let mut multiset =
        MultisetSignature::<Gf2_9StructuralFixture, _>::new(canonical, Gf2_9StructuralFixture::ONE);
    multiset.insert_elements(values).unwrap();
    let mut reversed =
        MultisetSignature::<Gf2_9StructuralFixture, _>::new(canonical, Gf2_9StructuralFixture::ONE);
    reversed.insert_elements(values.into_iter().rev()).unwrap();
    assert_eq!(multiset, reversed);

    let framed = BinaryPolynomialEncoder::new(0x4558_5445_524e_0001);
    let mut from_bytes = AdditiveSignature::<Gf2_9StructuralFixture, _>::new(framed);
    from_bytes
        .absorb_many([b"external".as_slice(), b"binary-field"])
        .unwrap();
    assert_eq!(from_bytes.term_count(), 2);
}

#[test]
fn bidirectional_sequence_matches_both_orientations_and_partition_composition() {
    let encoder = CanonicalElementEncoder;
    let base = Fp251V1::from_u64_mod(7);
    let values = [3_u8, 5, 11, 19, 31];
    let mut paired = BidirectionalSequenceSignature::<Fp251V1, _>::new(encoder, base).unwrap();
    let mut forward = SequenceSignature::<Fp251V1, _>::new(encoder, base).unwrap();
    let mut reverse = SequenceSignature::<Fp251V1, _>::new(encoder, base).unwrap();
    for value in values {
        paired.push(&[value]).unwrap();
        forward.push(&[value]).unwrap();
    }
    for value in values.into_iter().rev() {
        reverse.push(&[value]).unwrap();
    }
    assert_eq!(paired.forward_state(), forward.state());
    assert_eq!(paired.reverse_state(), reverse.state());

    let encoded_values = values.map(|value| [value]);
    let mut borrowed = BidirectionalSequenceSignature::<Fp251V1, _>::new(encoder, base).unwrap();
    borrowed.push_slice(&encoded_values).unwrap();
    assert_eq!(borrowed, paired);

    let mut prefix = BidirectionalSequenceSignature::<Fp251V1, _>::new(encoder, base).unwrap();
    let mut suffix = BidirectionalSequenceSignature::<Fp251V1, _>::new(encoder, base).unwrap();
    prefix
        .push_many(values[..2].iter().copied().map(|value| [value]))
        .unwrap();
    suffix
        .push_many(values[2..].iter().copied().map(|value| [value]))
        .unwrap();
    assert_eq!(prefix.concatenate(&suffix).unwrap(), paired);

    let mut reversed = BidirectionalSequenceSignature::<Fp251V1, _>::new(encoder, base).unwrap();
    reversed
        .push_many(values.into_iter().rev().map(|value| [value]))
        .unwrap();
    assert_eq!(reversed.forward_state(), paired.reverse_state());
    assert_eq!(reversed.reverse_state(), paired.forward_state());

    let bytes = paired.to_canonical_bytes();
    assert_eq!(
        BidirectionalSequenceSignature::<Fp251V1, _>::from_canonical_bytes(encoder, base, &bytes,)
            .unwrap(),
        paired
    );
}

#[test]
fn multi_evaluation_multiset_exposes_a_collision_hidden_by_one_evaluation() {
    let encoder = CanonicalElementEncoder;
    let one = Fp251V1::ONE;
    let two = Fp251V1::from_u64_mod(2);
    let first_values = [Fp251V1::ZERO, Fp251V1::ZERO];
    let second_values = [Fp251V1::ONE, Fp251V1::from_u64_mod(125)];

    let mut first_single = MultisetSignature::<Fp251V1, _>::new(encoder, one);
    first_single.insert_elements(first_values).unwrap();
    let mut second_single = MultisetSignature::<Fp251V1, _>::new(encoder, one);
    second_single.insert_elements(second_values).unwrap();
    assert_eq!(first_single, second_single);

    let mut first_multi =
        MultiEvaluationMultisetSignature::<Fp251V1, _, 2>::new(encoder, [one, two]).unwrap();
    first_multi.insert_elements(first_values).unwrap();
    let mut second_multi =
        MultiEvaluationMultisetSignature::<Fp251V1, _, 2>::new(encoder, [one, two]).unwrap();
    second_multi.insert_elements(second_values).unwrap();
    assert_ne!(first_multi, second_multi);
    assert_eq!(
        first_multi.evaluated_products()[0],
        first_single.evaluated_product()
    );
    assert_eq!(
        second_multi.evaluated_products()[0],
        second_single.evaluated_product()
    );
    assert_ne!(
        first_multi.evaluated_products()[1],
        second_multi.evaluated_products()[1]
    );
}

#[test]
fn multi_evaluation_multiset_is_total_at_zeros_composable_and_canonical() {
    let encoder = CanonicalElementEncoder;
    let offsets = [
        Fp251V1::ONE,
        Fp251V1::from_u64_mod(2),
        Fp251V1::from_u64_mod(3),
    ];
    // -1, -2 and -3 each zero exactly one separate coordinate.
    let values = [250_u8, 249, 248, 17, 29];
    let mut all = MultiEvaluationMultisetSignature::<Fp251V1, _, 3>::new(encoder, offsets).unwrap();
    all.insert_many(values.map(|value| [value])).unwrap();
    assert_eq!(all.zero_factor_counts(), &[1, 1, 1]);
    assert_eq!(all.evaluated_products(), [Fp251V1::ZERO; 3]);

    let mut left =
        MultiEvaluationMultisetSignature::<Fp251V1, _, 3>::new(encoder, offsets).unwrap();
    let mut right =
        MultiEvaluationMultisetSignature::<Fp251V1, _, 3>::new(encoder, offsets).unwrap();
    left.insert_many(values[..2].iter().copied().map(|value| [value]))
        .unwrap();
    right
        .insert_many(values[2..].iter().copied().map(|value| [value]))
        .unwrap();
    assert_eq!(left.combine(&right).unwrap(), all);

    let mut reversed =
        MultiEvaluationMultisetSignature::<Fp251V1, _, 3>::new(encoder, offsets).unwrap();
    reversed
        .insert_many(values.into_iter().rev().map(|value| [value]))
        .unwrap();
    assert_eq!(reversed, all);

    let bytes = all.to_canonical_bytes();
    assert_eq!(
        MultiEvaluationMultisetSignature::<Fp251V1, _, 3>::from_canonical_bytes(
            encoder, offsets, &bytes,
        )
        .unwrap(),
        all
    );

    let mut impossible = bytes;
    impossible[112..120].copy_from_slice(&6_u64.to_le_bytes());
    assert!(matches!(
        MultiEvaluationMultisetSignature::<Fp251V1, _, 3>::from_canonical_bytes(
            encoder,
            offsets,
            &impossible,
        ),
        Err(SignatureError::InvalidWireFormat(_))
    ));
}

#[test]
fn enriched_signatures_reject_degenerate_parameters_and_cover_generated_binary_fields() {
    let encoder = CanonicalElementEncoder;
    assert_eq!(
        MultiEvaluationMultisetSignature::<Fp251V1, _, 0>::new(encoder, []),
        Err(SignatureError::InvalidEvaluationPoints)
    );
    assert_eq!(
        MultiEvaluationMultisetSignature::<Fp251V1, _, 2>::new(
            encoder,
            [Fp251V1::ONE, Fp251V1::ONE],
        ),
        Err(SignatureError::InvalidEvaluationPoints)
    );
    assert_eq!(
        BidirectionalSequenceSignature::<Fp251V1, _>::new(encoder, Fp251V1::ZERO),
        Err(SignatureError::DegenerateSequenceBase)
    );

    check_enriched_binary_family::<Gf2_128V1>();
    check_enriched_binary_family::<Gf2_256HhV1>();
    check_enriched_binary_family::<Gf2_256AltV1>();
    check_enriched_binary_family::<Gf2_9StructuralFixture>();
}

#[test]
fn enriched_signature_counter_overflow_is_transactional_after_restore() {
    let encoder = CanonicalElementEncoder;
    let base = Fp251V1::from_u64_mod(7);
    let empty = BidirectionalSequenceSignature::<Fp251V1, _>::new(encoder, base).unwrap();
    let mut bytes = empty.to_canonical_bytes();
    bytes[104..112].copy_from_slice(&u64::MAX.to_le_bytes());
    let mut sequence =
        BidirectionalSequenceSignature::<Fp251V1, _>::from_canonical_bytes(encoder, base, &bytes)
            .unwrap();
    let before = sequence.clone();
    assert_eq!(sequence.push(&[1]), Err(SignatureError::CounterOverflow));
    assert_eq!(sequence, before);

    let offsets = [Fp251V1::ONE, Fp251V1::from_u64_mod(2)];
    let empty = MultiEvaluationMultisetSignature::<Fp251V1, _, 2>::new(encoder, offsets).unwrap();
    let mut bytes = empty.to_canonical_bytes();
    bytes[104..112].copy_from_slice(&u64::MAX.to_le_bytes());
    let mut multiset = MultiEvaluationMultisetSignature::<Fp251V1, _, 2>::from_canonical_bytes(
        encoder, offsets, &bytes,
    )
    .unwrap();
    let before = multiset.clone();
    assert_eq!(multiset.insert(&[1]), Err(SignatureError::CounterOverflow));
    assert_eq!(multiset, before);
}

#[cfg(feature = "dynamic-fields")]
#[test]
fn dynamic_binary_context_is_wire_compatible_with_the_same_generated_field() {
    let field = DynField::builder("runtime_gf2_9")
        .binary(9, vec![9, 4, 0])
        .build()
        .unwrap();
    assert_eq!(field.field_id(), Gf2_9StructuralFixture::spec().field_id());

    let raw_values = [1_u16, 0x101, 0x1ff, 0x55];
    let static_values =
        raw_values.map(|raw| Gf2_9StructuralFixture::from_canonical(&raw.to_le_bytes()).unwrap());
    let dynamic_values = raw_values.map(|raw| field.decode(&raw.to_le_bytes()).unwrap());
    let encoder = CanonicalElementEncoder;

    let mut static_additive = AdditiveSignature::<Gf2_9StructuralFixture, _>::new(encoder);
    static_additive.absorb_elements(static_values).unwrap();
    let mut dynamic_additive = DynamicAdditiveSignature::new(field.clone(), encoder);
    for element in &dynamic_values {
        dynamic_additive.absorb_element(element).unwrap();
    }
    assert_eq!(
        dynamic_additive.to_canonical_bytes().unwrap(),
        static_additive.to_canonical_bytes()
    );

    let static_base = Gf2_9StructuralFixture::ONE.mul_by_x();
    let dynamic_base = field.decode(static_base.to_canonical().as_ref()).unwrap();
    let mut static_sequence =
        SequenceSignature::<Gf2_9StructuralFixture, _>::new(encoder, static_base).unwrap();
    static_sequence.push_elements(static_values).unwrap();
    let mut dynamic_sequence =
        DynamicSequenceSignature::new(field.clone(), encoder, dynamic_base.clone()).unwrap();
    for element in &dynamic_values {
        dynamic_sequence.push_element(element).unwrap();
    }
    assert_eq!(
        dynamic_sequence.to_canonical_bytes().unwrap(),
        static_sequence.to_canonical_bytes()
    );

    let static_offset = Gf2_9StructuralFixture::ONE;
    let dynamic_offset = field.one();
    let mut static_multiset =
        MultisetSignature::<Gf2_9StructuralFixture, _>::new(encoder, static_offset);
    static_multiset.insert_elements(static_values).unwrap();
    let mut dynamic_multiset =
        DynamicMultisetSignature::new(field.clone(), encoder, dynamic_offset).unwrap();
    for element in &dynamic_values {
        dynamic_multiset.insert_element(element).unwrap();
    }
    assert_eq!(
        dynamic_multiset.to_canonical_bytes().unwrap(),
        static_multiset.to_canonical_bytes()
    );

    let mut static_bidirectional =
        BidirectionalSequenceSignature::<Gf2_9StructuralFixture, _>::new(encoder, static_base)
            .unwrap();
    static_bidirectional
        .push_elements_slice(&static_values)
        .unwrap();
    let mut dynamic_bidirectional =
        DynamicBidirectionalSequenceSignature::new(field.clone(), encoder, dynamic_base.clone())
            .unwrap();
    for element in &dynamic_values {
        dynamic_bidirectional.push_element(element).unwrap();
    }
    assert_eq!(
        dynamic_bidirectional.to_canonical_bytes().unwrap(),
        static_bidirectional.to_canonical_bytes()
    );

    let static_offsets = [
        Gf2_9StructuralFixture::ZERO,
        Gf2_9StructuralFixture::ONE,
        static_base,
    ];
    let dynamic_offsets = vec![field.zero(), field.one(), dynamic_base.clone()];
    let mut static_multi = MultiEvaluationMultisetSignature::<Gf2_9StructuralFixture, _, 3>::new(
        encoder,
        static_offsets,
    )
    .unwrap();
    static_multi.insert_elements(static_values).unwrap();
    let mut dynamic_multi = DynamicMultiEvaluationMultisetSignature::new(
        field.clone(),
        encoder,
        dynamic_offsets.clone(),
    )
    .unwrap();
    for element in &dynamic_values {
        dynamic_multi.insert_element(element).unwrap();
    }
    assert_eq!(
        dynamic_multi.to_canonical_bytes().unwrap(),
        static_multi.to_canonical_bytes()
    );

    assert_eq!(
        DynamicAdditiveSignature::from_canonical_bytes(
            field.clone(),
            encoder,
            &static_additive.to_canonical_bytes(),
        )
        .unwrap(),
        dynamic_additive
    );
    assert_eq!(
        DynamicSequenceSignature::from_canonical_bytes(
            field.clone(),
            encoder,
            dynamic_base.clone(),
            &static_sequence.to_canonical_bytes(),
        )
        .unwrap(),
        dynamic_sequence
    );
    assert_eq!(
        DynamicBidirectionalSequenceSignature::from_canonical_bytes(
            field.clone(),
            encoder,
            dynamic_base,
            &static_bidirectional.to_canonical_bytes(),
        )
        .unwrap(),
        dynamic_bidirectional
    );
    assert_eq!(
        DynamicMultiEvaluationMultisetSignature::from_canonical_bytes(
            field,
            encoder,
            dynamic_offsets,
            &static_multi.to_canonical_bytes(),
        )
        .unwrap(),
        dynamic_multi
    );
}

#[cfg(feature = "dynamic-fields")]
#[test]
fn dynamic_structural_contexts_fail_closed_on_field_and_encoder_drift() {
    let binary = DynField::builder("binary")
        .binary(9, vec![9, 4, 0])
        .build()
        .unwrap();
    let other_binary = DynField::builder("other_binary")
        .binary(8, vec![8, 4, 3, 1, 0])
        .build()
        .unwrap();
    let prime = DynField::builder("prime").prime("251").build().unwrap();

    let encoder = BinaryPolynomialEncoder::new(0x4459_4e41_4d49_4301);
    let mut left = DynamicAdditiveSignature::new(binary.clone(), encoder);
    let mut right = DynamicAdditiveSignature::new(other_binary.clone(), encoder);
    left.absorb(b"same").unwrap();
    right.absorb(b"same").unwrap();
    assert_eq!(left.combine(&right), Err(SignatureError::IdentityMismatch));

    let foreign = other_binary.one();
    let mut canonical = DynamicAdditiveSignature::new(binary, CanonicalElementEncoder);
    assert!(canonical.absorb_element(&foreign).is_err());
    assert_eq!(canonical.term_count(), 0);

    let mut wrong_family = DynamicAdditiveSignature::new(prime, encoder);
    assert_eq!(
        wrong_family.absorb(b"not-a-binary-field"),
        Err(SignatureError::EncoderFamilyMismatch)
    );
    assert_eq!(wrong_family.term_count(), 0);

    assert_eq!(
        DynamicMultiEvaluationMultisetSignature::new(
            other_binary.clone(),
            CanonicalElementEncoder,
            vec![other_binary.one(), other_binary.one()],
        ),
        Err(SignatureError::InvalidEvaluationPoints)
    );
    assert!(DynamicMultiEvaluationMultisetSignature::new(
        other_binary,
        CanonicalElementEncoder,
        vec![foreign, canonical.state().clone()],
    )
    .is_err());
}

#[cfg(feature = "dynamic-fields")]
#[test]
fn dynamic_prime_context_executes_all_five_partition_laws() {
    let field = DynField::builder("runtime_fp251")
        .prime("251")
        .build()
        .unwrap();
    assert_eq!(field.field_id(), Fp251V1::spec().field_id());
    let encoder = PrimeIntegerEncoder::new(0x4459_4e50_5249_4d01);
    let values = [b"north".as_slice(), b"east", b"south", b"west"];
    let base = field.decode(&[7]).unwrap();

    let mut additive = DynamicAdditiveSignature::new(field.clone(), encoder);
    let mut additive_left = DynamicAdditiveSignature::new(field.clone(), encoder);
    let mut additive_right = DynamicAdditiveSignature::new(field.clone(), encoder);
    additive.absorb_many(values).unwrap();
    additive_left.absorb_many(&values[..2]).unwrap();
    additive_right.absorb_many(&values[2..]).unwrap();
    assert_eq!(additive_left.combine(&additive_right).unwrap(), additive);

    let mut sequence = DynamicSequenceSignature::new(field.clone(), encoder, base.clone()).unwrap();
    let mut sequence_left =
        DynamicSequenceSignature::new(field.clone(), encoder, base.clone()).unwrap();
    let mut sequence_right =
        DynamicSequenceSignature::new(field.clone(), encoder, base.clone()).unwrap();
    sequence.push_many(values).unwrap();
    sequence_left.push_many(&values[..2]).unwrap();
    sequence_right.push_many(&values[2..]).unwrap();
    assert_eq!(
        sequence_left.concatenate(&sequence_right).unwrap(),
        sequence
    );

    let mut bidirectional =
        DynamicBidirectionalSequenceSignature::new(field.clone(), encoder, base.clone()).unwrap();
    let mut bidirectional_left =
        DynamicBidirectionalSequenceSignature::new(field.clone(), encoder, base.clone()).unwrap();
    let mut bidirectional_right =
        DynamicBidirectionalSequenceSignature::new(field.clone(), encoder, base.clone()).unwrap();
    bidirectional.push_many(values).unwrap();
    bidirectional_left.push_many(&values[..2]).unwrap();
    bidirectional_right.push_many(&values[2..]).unwrap();
    assert_eq!(
        bidirectional_left
            .concatenate(&bidirectional_right)
            .unwrap(),
        bidirectional
    );

    let offset = field.one();
    let mut multiset =
        DynamicMultisetSignature::new(field.clone(), encoder, offset.clone()).unwrap();
    let mut multiset_left =
        DynamicMultisetSignature::new(field.clone(), encoder, offset.clone()).unwrap();
    let mut multiset_right =
        DynamicMultisetSignature::new(field.clone(), encoder, offset.clone()).unwrap();
    multiset.insert_many(values).unwrap();
    multiset_left.insert_many(&values[..2]).unwrap();
    multiset_right.insert_many(&values[2..]).unwrap();
    assert_eq!(multiset_left.combine(&multiset_right).unwrap(), multiset);

    let offsets = vec![field.zero(), offset, base.clone()];
    let mut multi =
        DynamicMultiEvaluationMultisetSignature::new(field.clone(), encoder, offsets.clone())
            .unwrap();
    let mut multi_left =
        DynamicMultiEvaluationMultisetSignature::new(field.clone(), encoder, offsets.clone())
            .unwrap();
    let mut multi_right =
        DynamicMultiEvaluationMultisetSignature::new(field.clone(), encoder, offsets.clone())
            .unwrap();
    multi.insert_many(values).unwrap();
    multi_left.insert_many(&values[..2]).unwrap();
    multi_right.insert_many(&values[2..]).unwrap();
    assert_eq!(multi_left.combine(&multi_right).unwrap(), multi);

    assert_eq!(
        DynamicBidirectionalSequenceSignature::from_canonical_bytes(
            field.clone(),
            encoder,
            base,
            &bidirectional.to_canonical_bytes().unwrap(),
        )
        .unwrap(),
        bidirectional
    );
    assert_eq!(
        DynamicMultiEvaluationMultisetSignature::from_canonical_bytes(
            field,
            encoder,
            offsets,
            &multi.to_canonical_bytes().unwrap(),
        )
        .unwrap(),
        multi
    );
}

#[test]
fn common_inline_encoding_and_signature_updates_allocate_zero_times() {
    let encoder = BinaryPolynomialEncoder::new(123);
    let base = binary_base();
    let mut additive = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
    let mut sequence = SequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    let mut multiset = MultisetSignature::<Gf2_256HhV1, _>::new(encoder, Gf2_256HhV1::ONE);
    let mut bidirectional =
        BidirectionalSequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    let mut multi = MultiEvaluationMultisetSignature::<Gf2_256HhV1, _, 3>::new(
        encoder,
        [Gf2_256HhV1::ZERO, Gf2_256HhV1::ONE, base],
    )
    .unwrap();
    let input = [0x5a_u8; 64];
    let allocations = measure(|| {
        additive.absorb(&input).unwrap();
        sequence.push(&input).unwrap();
        multiset.insert(&input).unwrap();
        bidirectional
            .push_slice(core::slice::from_ref(&input))
            .unwrap();
        multi.insert(&input).unwrap();
    });
    assert_eq!(allocations.count_total, 0);
    assert_eq!(allocations.bytes_total, 0);
}

#[test]
fn bulk_ingestion_matches_scalar_and_is_transactional_on_encoder_failure() {
    let values = [b"a".as_slice(), b"bb", b"c"];
    let encoder = BinaryPolynomialEncoder::new(124);
    let base = binary_base();

    let mut additive_bulk = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
    additive_bulk.absorb_many(values).unwrap();
    let mut additive_scalar = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
    for value in values {
        additive_scalar.absorb(value).unwrap();
    }
    assert_eq!(additive_bulk, additive_scalar);

    let mut sequence_bulk = SequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    sequence_bulk.push_many(values).unwrap();
    let mut sequence_scalar = SequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    for value in values {
        sequence_scalar.push(value).unwrap();
    }
    assert_eq!(sequence_bulk, sequence_scalar);

    let mut multiset_bulk = MultisetSignature::<Gf2_256HhV1, _>::new(encoder, Gf2_256HhV1::ONE);
    multiset_bulk.insert_many(values).unwrap();
    let mut multiset_scalar = MultisetSignature::<Gf2_256HhV1, _>::new(encoder, Gf2_256HhV1::ONE);
    for value in values {
        multiset_scalar.insert(value).unwrap();
    }
    assert_eq!(multiset_bulk, multiset_scalar);

    let limited = encoder.with_maximum_input_bytes(1);
    let invalid_batch = [b"x".as_slice(), b"too-long"];

    let mut additive = AdditiveSignature::<Gf2_256HhV1, _>::new(limited);
    let before = additive.clone();
    assert!(matches!(
        additive.absorb_many(invalid_batch),
        Err(SignatureError::InputTooLarge { .. })
    ));
    assert_eq!(additive, before);

    let mut sequence = SequenceSignature::<Gf2_256HhV1, _>::new(limited, base).unwrap();
    let before = sequence.clone();
    assert!(matches!(
        sequence.push_many(invalid_batch),
        Err(SignatureError::InputTooLarge { .. })
    ));
    assert_eq!(sequence, before);

    let mut multiset = MultisetSignature::<Gf2_256HhV1, _>::new(limited, Gf2_256HhV1::ONE);
    let before = multiset.clone();
    assert!(matches!(
        multiset.insert_many(invalid_batch),
        Err(SignatureError::InputTooLarge { .. })
    ));
    assert_eq!(multiset, before);

    let mut bidirectional =
        BidirectionalSequenceSignature::<Gf2_256HhV1, _>::new(limited, base).unwrap();
    let before = bidirectional.clone();
    assert!(matches!(
        bidirectional.push_slice(&invalid_batch),
        Err(SignatureError::InputTooLarge { .. })
    ));
    assert_eq!(bidirectional, before);

    let mut multi = MultiEvaluationMultisetSignature::<Gf2_256HhV1, _, 3>::new(
        limited,
        [Gf2_256HhV1::ZERO, Gf2_256HhV1::ONE, base],
    )
    .unwrap();
    let before = multi.clone();
    assert!(matches!(
        multi.insert_many(invalid_batch),
        Err(SignatureError::InputTooLarge { .. })
    ));
    assert_eq!(multi, before);
}

#[test]
fn large_partition_tree_matches_single_pass_for_every_structural_law() {
    let mut seed = 0x6a09_e667_f3bc_c909_u64;
    // Native exercises a large tree; Miri exercises the same control flow with
    // a bounded corpus after the full suite has already covered field kernels.
    let item_count = if cfg!(miri) { 64 } else { 4_096 };
    let inputs = (0..item_count)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed.to_le_bytes()
        })
        .collect::<Vec<_>>();
    let encoder = BinaryPolynomialEncoder::new(125);
    let base = binary_base();

    let mut additive = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
    additive.absorb_many(&inputs).unwrap();
    let mut additive_tree = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
    for chunk in inputs.chunks(257) {
        let mut partition = AdditiveSignature::<Gf2_256HhV1, _>::new(encoder);
        partition.absorb_many(chunk).unwrap();
        additive_tree = additive_tree.combine(&partition).unwrap();
    }
    assert_eq!(additive_tree, additive);

    let mut sequence = SequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    sequence.push_many(&inputs).unwrap();
    let mut sequence_tree = SequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    for chunk in inputs.chunks(257) {
        let mut partition = SequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
        partition.push_many(chunk).unwrap();
        sequence_tree = sequence_tree.concatenate(&partition).unwrap();
    }
    assert_eq!(sequence_tree, sequence);

    let mut multiset = MultisetSignature::<Gf2_256HhV1, _>::new(encoder, Gf2_256HhV1::ONE);
    multiset.insert_many(&inputs).unwrap();
    let mut multiset_tree = MultisetSignature::<Gf2_256HhV1, _>::new(encoder, Gf2_256HhV1::ONE);
    for chunk in inputs.chunks(257) {
        let mut partition = MultisetSignature::<Gf2_256HhV1, _>::new(encoder, Gf2_256HhV1::ONE);
        partition.insert_many(chunk).unwrap();
        multiset_tree = multiset_tree.combine(&partition).unwrap();
    }
    assert_eq!(multiset_tree, multiset);

    let mut bidirectional =
        BidirectionalSequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    bidirectional.push_slice(&inputs).unwrap();
    let mut bidirectional_tree =
        BidirectionalSequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
    for chunk in inputs.chunks(257) {
        let mut partition =
            BidirectionalSequenceSignature::<Gf2_256HhV1, _>::new(encoder, base).unwrap();
        partition.push_slice(chunk).unwrap();
        bidirectional_tree = bidirectional_tree.concatenate(&partition).unwrap();
    }
    assert_eq!(bidirectional_tree, bidirectional);

    let offsets = [Gf2_256HhV1::ZERO, Gf2_256HhV1::ONE, base];
    let mut multi =
        MultiEvaluationMultisetSignature::<Gf2_256HhV1, _, 3>::new(encoder, offsets).unwrap();
    multi.insert_many(&inputs).unwrap();
    let mut multi_tree =
        MultiEvaluationMultisetSignature::<Gf2_256HhV1, _, 3>::new(encoder, offsets).unwrap();
    for chunk in inputs.chunks(257) {
        let mut partition =
            MultiEvaluationMultisetSignature::<Gf2_256HhV1, _, 3>::new(encoder, offsets).unwrap();
        partition.insert_many(chunk).unwrap();
        multi_tree = multi_tree.combine(&partition).unwrap();
    }
    assert_eq!(multi_tree, multi);
}

#[test]
fn explicit_legacy_encoders_freeze_old_bytes_and_aggregation_laws() {
    let inputs = [
        b"short".as_slice(),
        &[0_u8; 32],
        &[0xa5_u8; 33],
        &[0x5a_u8; 97],
    ];
    for input in inputs {
        let legacy_linear: GaloisSignature256 =
            SymmetricDifferenceAggregator::embed_to_field(input);
        let modern_linear: Gf2_256HhV1 = LegacyLinearEncoderV1.encode(input).unwrap();
        assert_eq!(
            legacy_linear.to_canonical_bytes(),
            modern_linear.to_canonical()
        );

        let legacy_affine: GaloisSignature256 = LegacyMultiset::embed_to_field(input);
        let modern_affine: Gf2_256HhV1 = LegacyAffineEncoderV1.encode(input).unwrap();
        assert_eq!(
            legacy_affine.to_canonical_bytes(),
            modern_affine.to_canonical()
        );
    }

    let base = Gf2_256HhV1::ONE.mul_by_x();
    let mut modern_sequence =
        SequenceSignature::<Gf2_256HhV1, _>::new(LegacyLinearEncoderV1, base).unwrap();
    let mut legacy_sequence = GaloisSignature256::zero();
    for input in inputs {
        modern_sequence.push(input).unwrap();
        let element: GaloisSignature256 = LegacySequence::embed_to_field(input);
        legacy_sequence = LegacySequence::aggregate(&legacy_sequence, &element, 0);
    }
    assert_eq!(
        modern_sequence.state().to_canonical(),
        legacy_sequence.to_canonical_bytes()
    );

    let mut generator = [0_u8; 32];
    generator[31] = 0x80;
    let offset = Gf2_256HhV1::from_canonical(&generator).unwrap();
    let mut modern_multiset =
        MultisetSignature::<Gf2_256HhV1, _>::new(LegacyAffineEncoderV1, offset);
    let mut legacy_multiset = GaloisSignature256::one();
    for input in inputs {
        modern_multiset.insert(input).unwrap();
        let element: GaloisSignature256 = LegacyMultiset::embed_to_field(input);
        legacy_multiset = LegacyMultiset::aggregate(&legacy_multiset, &element, 0);
    }
    assert_eq!(
        modern_multiset.evaluated_product().to_canonical(),
        legacy_multiset.to_canonical_bytes()
    );
}

fn check_binary_family<F>()
where
    F: Field
        + CanonicalEncoding
        + StaticField
        + BinaryPolynomialField
        + Pow
        + Invert
        + core::fmt::Debug,
{
    let encoder = BinaryPolynomialEncoder::new(0x4656_4249_4e00_0001);
    check_composition_laws::<F, _>(encoder, F::ONE.mul_by_x());
}

fn check_enriched_binary_family<F>()
where
    F: Field + CanonicalEncoding + StaticField + BinaryPolynomialField + Pow + core::fmt::Debug,
{
    let encoder = BinaryPolynomialEncoder::new(0x454e_5249_4348_0001);
    let base = F::ONE.mul_by_x();
    let offsets = [F::ZERO, F::ONE, base];
    let values = [b"alpha".as_slice(), b"beta", b"gamma", b"delta"];

    let mut complete = BidirectionalSequenceSignature::<F, _>::new(encoder, base).unwrap();
    complete.push_slice(&values).unwrap();
    let mut prefix = BidirectionalSequenceSignature::<F, _>::new(encoder, base).unwrap();
    let mut suffix = BidirectionalSequenceSignature::<F, _>::new(encoder, base).unwrap();
    prefix.push_slice(&values[..2]).unwrap();
    suffix.push_slice(&values[2..]).unwrap();
    assert_eq!(prefix.concatenate(&suffix).unwrap(), complete);

    let mut all = MultiEvaluationMultisetSignature::<F, _, 3>::new(encoder, offsets).unwrap();
    let mut left = MultiEvaluationMultisetSignature::<F, _, 3>::new(encoder, offsets).unwrap();
    let mut right = MultiEvaluationMultisetSignature::<F, _, 3>::new(encoder, offsets).unwrap();
    all.insert_many(values).unwrap();
    left.insert_many(&values[..2]).unwrap();
    right.insert_many(&values[2..]).unwrap();
    assert_eq!(left.combine(&right).unwrap(), all);

    assert_eq!(
        BidirectionalSequenceSignature::<F, _>::from_canonical_bytes(
            encoder,
            base,
            &complete.to_canonical_bytes(),
        )
        .unwrap(),
        complete
    );
    assert_eq!(
        MultiEvaluationMultisetSignature::<F, _, 3>::from_canonical_bytes(
            encoder,
            offsets,
            &all.to_canonical_bytes(),
        )
        .unwrap(),
        all
    );
}

fn check_prime_family<F>()
where
    F: Field + CanonicalEncoding + StaticField + PrimeField + Pow + Invert + core::fmt::Debug,
{
    let encoder = PrimeIntegerEncoder::new(0x4656_5052_494d_0001);
    check_composition_laws::<F, _>(encoder, F::from_bytes_mod_order(&[7]));
}

fn check_composition_laws<F, E>(encoder: E, base: F)
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert + core::fmt::Debug,
    E: StructuralEncoder<F> + Eq + core::fmt::Debug,
{
    let values = [b"north".as_slice(), b"east", b"south", b"west"];

    let mut all_additive = AdditiveSignature::<F, _>::new(encoder.clone());
    let mut left_additive = AdditiveSignature::<F, _>::new(encoder.clone());
    let mut right_additive = AdditiveSignature::<F, _>::new(encoder.clone());
    for value in values {
        all_additive.absorb(value).unwrap();
    }
    for value in &values[..2] {
        left_additive.absorb(value).unwrap();
    }
    for value in &values[2..] {
        right_additive.absorb(value).unwrap();
    }
    assert_eq!(
        left_additive.combine(&right_additive).unwrap(),
        all_additive
    );

    let mut complete = SequenceSignature::<F, _>::new(encoder.clone(), base).unwrap();
    let mut prefix = SequenceSignature::<F, _>::new(encoder.clone(), base).unwrap();
    let mut suffix = SequenceSignature::<F, _>::new(encoder.clone(), base).unwrap();
    for value in values {
        complete.push(value).unwrap();
    }
    for value in &values[..2] {
        prefix.push(value).unwrap();
    }
    for value in &values[2..] {
        suffix.push(value).unwrap();
    }
    assert_eq!(prefix.concatenate(&suffix).unwrap(), complete);

    let mut all_multiset = MultisetSignature::<F, _>::new(encoder.clone(), F::ONE);
    let mut reversed = MultisetSignature::<F, _>::new(encoder.clone(), F::ONE);
    for value in values {
        all_multiset.insert(value).unwrap();
    }
    for value in values.into_iter().rev() {
        reversed.insert(value).unwrap();
    }
    assert_eq!(all_multiset, reversed);

    let mut all_bidirectional =
        BidirectionalSequenceSignature::<F, _>::new(encoder.clone(), base).unwrap();
    let mut left_bidirectional =
        BidirectionalSequenceSignature::<F, _>::new(encoder.clone(), base).unwrap();
    let mut right_bidirectional =
        BidirectionalSequenceSignature::<F, _>::new(encoder.clone(), base).unwrap();
    all_bidirectional.push_slice(&values).unwrap();
    left_bidirectional.push_slice(&values[..2]).unwrap();
    right_bidirectional.push_slice(&values[2..]).unwrap();
    assert_eq!(
        left_bidirectional
            .concatenate(&right_bidirectional)
            .unwrap(),
        all_bidirectional
    );

    let offsets = [F::ZERO, F::ONE, base];
    let mut all_multi =
        MultiEvaluationMultisetSignature::<F, _, 3>::new(encoder.clone(), offsets).unwrap();
    let mut left_multi =
        MultiEvaluationMultisetSignature::<F, _, 3>::new(encoder.clone(), offsets).unwrap();
    let mut right_multi =
        MultiEvaluationMultisetSignature::<F, _, 3>::new(encoder, offsets).unwrap();
    all_multi.insert_many(values).unwrap();
    left_multi.insert_many(&values[..2]).unwrap();
    right_multi.insert_many(&values[2..]).unwrap();
    assert_eq!(left_multi.combine(&right_multi).unwrap(), all_multi);
}
