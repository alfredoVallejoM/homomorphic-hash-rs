//! RC.2 contract for builders, profiles and compact/exact snapshots.

#![cfg(feature = "signatures")]

use homomorphic_hash_rs::{
    BinaryPolynomialEncoder, CanonicalElementEncoder, CompactSignature, PrimeIntegerEncoder,
    SignatureBuilder, SignatureError, SignatureEvaluationProfile, SignatureFieldBinding,
    SignatureLaw, TrackedMultiset, TrackedSequence, TrackedSnapshotLimits,
};
use microfield::{BinaryPolynomialField, Field, Fp251V1, Gf2_128V1, PrimeField};
use rand::{rngs::StdRng, Rng, SeedableRng};
use structural_field_fixture::Gf2_9StructuralFixture;

fn base(value: u8) -> Gf2_128V1 {
    Gf2_128V1::from_polynomial_bytes_mod(&[value])
}

#[test]
fn static_builder_keeps_laws_concrete_and_reports_field_semantics() {
    let builder = SignatureBuilder::<Gf2_128V1, _>::new(CanonicalElementEncoder);
    let field = builder.field_profile();
    assert_eq!(field.binding(), SignatureFieldBinding::Static);
    assert!(field.characteristic_is_two());
    assert_eq!(field.extension_degree(), 128);
    assert_eq!(field.canonical_bytes(), 16);

    let additive = builder.additive();
    let sequence = builder.sequence(base(2)).unwrap();
    let bidirectional = builder.bidirectional_sequence(base(2)).unwrap();
    let multiset = builder.multiset(Gf2_128V1::ONE);
    let multi_multiset = builder
        .multi_evaluation_multiset([Gf2_128V1::ZERO, Gf2_128V1::ONE])
        .unwrap();
    let multi_sequence = builder
        .multi_evaluation_sequence([base(2), base(3)])
        .unwrap();

    let profiles = [
        additive.signature_profile(),
        sequence.signature_profile(),
        bidirectional.signature_profile(),
        multiset.signature_profile(),
        multi_multiset.signature_profile(),
        multi_sequence.signature_profile(),
    ];
    assert_eq!(profiles[0].context().law(), SignatureLaw::Additive);
    assert_eq!(profiles[1].context().law(), SignatureLaw::Sequence);
    assert_eq!(
        profiles[2].context().law(),
        SignatureLaw::BidirectionalSequence
    );
    assert_eq!(profiles[3].context().law(), SignatureLaw::Multiset);
    assert_eq!(
        profiles[4].context().law(),
        SignatureLaw::MultiEvaluationMultiset
    );
    assert_eq!(
        profiles[5].context().law(),
        SignatureLaw::MultiEvaluationSequence
    );
    assert_eq!(
        profiles.map(|profile| profile.evaluation_count()),
        [1, 1, 2, 1, 2, 2]
    );
    assert_eq!(
        profiles[4].maintained_evaluation_profile(),
        Some(SignatureEvaluationProfile::K2)
    );
    assert_eq!(SignatureEvaluationProfile::K4.evaluation_count(), 4);
    for profile in profiles {
        assert_eq!(profile.item_count(), 0);
    }
}

#[test]
fn compact_snapshot_trait_is_identical_to_the_existing_mfsg_wire() {
    let builder = SignatureBuilder::<Gf2_128V1, _>::new(BinaryPolynomialEncoder::new(7));
    let mut signature = builder.sequence(base(2)).unwrap();
    signature
        .push_many([b"alpha".as_slice(), b"beta", b"gamma"])
        .unwrap();
    assert_eq!(
        signature.to_compact_snapshot().unwrap(),
        signature.to_canonical_bytes()
    );
    assert_eq!(signature.signature_profile().item_count(), 3);
}

#[test]
fn builders_generalize_to_prime_and_external_generated_fields() {
    let prime = SignatureBuilder::<Fp251V1, _>::new(PrimeIntegerEncoder::new(19));
    let prime_base = Fp251V1::from_bytes_mod_order(&[2]);
    let mut prime_sequence = prime.sequence(prime_base).unwrap();
    prime_sequence.push_many([b"a".as_slice(), b"b"]).unwrap();
    assert_eq!(prime_sequence.signature_profile().item_count(), 2);
    assert!(!prime.field_profile().characteristic_is_two());

    let external =
        SignatureBuilder::<Gf2_9StructuralFixture, _>::new(BinaryPolynomialEncoder::new(23));
    let external_base = Gf2_9StructuralFixture::from_polynomial_bytes_mod(&[2]);
    let mut external_sequence = external.sequence(external_base).unwrap();
    external_sequence.push(b"generated").unwrap();
    let mut external_multiset = external.multiset(Gf2_9StructuralFixture::ONE);
    external_multiset
        .insert_many([b"x".as_slice(), b"y"])
        .unwrap();
    assert_eq!(external_multiset.signature_profile().item_count(), 2);
    assert_eq!(external_sequence.signature_profile().item_count(), 1);
    assert_eq!(external.field_profile().extension_degree(), 9);
}

#[test]
fn tracked_sequence_snapshot_round_trips_order_and_rejects_tampering() {
    let encoder = BinaryPolynomialEncoder::new(11);
    let mut tracked = TrackedSequence::<Gf2_128V1, _>::new(encoder, base(2)).unwrap();
    for item in [b"first".as_slice(), b"second", b"third"] {
        tracked.push(item).unwrap();
    }

    let bytes = tracked.to_snapshot_bytes().unwrap();
    assert_eq!(&bytes[..4], b"MFTS");
    assert_ne!(&bytes[..4], b"MFSG");
    let mut restored = TrackedSequence::from_snapshot_bytes(encoder, base(2), &bytes).unwrap();
    assert_eq!(restored, tracked);
    assert_eq!(restored.pop().unwrap(), b"third");
    assert_eq!(restored.pop().unwrap(), b"second");
    assert_eq!(restored.pop().unwrap(), b"first");

    let mut tampered = bytes.clone();
    *tampered.last_mut().unwrap() ^= 1;
    assert!(TrackedSequence::from_snapshot_bytes(encoder, base(2), &tampered).is_err());
    assert!(TrackedSequence::from_snapshot_bytes(encoder, base(3), &bytes).is_err());
    for length in 0..bytes.len() {
        assert!(
            TrackedSequence::from_snapshot_bytes(encoder, base(2), &bytes[..length]).is_err(),
            "accepted truncated snapshot at byte {length}"
        );
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(TrackedSequence::from_snapshot_bytes(encoder, base(2), &trailing).is_err());
}

#[test]
fn tracked_multiset_snapshot_round_trips_multiplicity_and_is_fail_closed() {
    let encoder = BinaryPolynomialEncoder::new(13);
    let mut tracked = TrackedMultiset::<Gf2_128V1, _>::new(encoder, Gf2_128V1::ONE);
    for item in [b"row-a".as_slice(), b"row-b", b"row-a", b"row-c"] {
        tracked.insert(item).unwrap();
    }

    let bytes = tracked.to_snapshot_bytes().unwrap();
    let restored = TrackedMultiset::from_snapshot_bytes(encoder, Gf2_128V1::ONE, &bytes).unwrap();
    assert_eq!(restored, tracked);
    assert_eq!(restored.multiplicity(b"row-a"), 2);
    assert_eq!(restored.multiplicity(b"row-b"), 1);
    assert_eq!(restored.multiplicity(b"absent"), 0);

    let mut reordered = TrackedMultiset::<Gf2_128V1, _>::new(encoder, Gf2_128V1::ONE);
    for item in [b"row-c".as_slice(), b"row-a", b"row-b", b"row-a"] {
        reordered.insert(item).unwrap();
    }
    assert_eq!(reordered.to_snapshot_bytes().unwrap(), bytes);

    let strict = TrackedSnapshotLimits::new(3, 3, 16, 4096);
    assert_eq!(
        tracked.to_snapshot_bytes_with_limits(strict),
        Err(SignatureError::SnapshotLimitExceeded("logical items"))
    );
    assert_eq!(tracked.multiplicity(b"row-a"), 2);

    let wrong_kind = TrackedSequence::<Gf2_128V1, _>::from_snapshot_bytes(encoder, base(2), &bytes);
    assert!(matches!(
        wrong_kind,
        Err(SignatureError::InvalidWireFormat(
            "tracked snapshot identity"
        ))
    ));
}

#[test]
fn long_random_tracked_histories_round_trip_after_every_checkpoint() {
    let encoder = BinaryPolynomialEncoder::new(29);
    let mut sequence = TrackedSequence::<Gf2_128V1, _>::new(encoder, base(2)).unwrap();
    let mut multiset = TrackedMultiset::<Gf2_128V1, _>::new(encoder, Gf2_128V1::ONE);
    let mut exact_sequence = Vec::<Vec<u8>>::new();
    let mut exact_multiset = Vec::<Vec<u8>>::new();
    let mut rng = StdRng::seed_from_u64(0x5243_3253_4e41_5053);

    for step in 0..500_u64 {
        let item = rng.gen::<u64>().to_le_bytes().to_vec();
        if !exact_sequence.is_empty() && rng.gen_ratio(1, 4) {
            let expected = exact_sequence.pop().unwrap();
            assert_eq!(sequence.pop().unwrap(), expected);
        } else {
            sequence.push(&item).unwrap();
            exact_sequence.push(item.clone());
        }

        if !exact_multiset.is_empty() && rng.gen_ratio(1, 3) {
            let index = rng.gen_range(0..exact_multiset.len());
            let removed = exact_multiset.swap_remove(index);
            multiset.remove(&removed).unwrap();
        } else {
            multiset.insert(&item).unwrap();
            exact_multiset.push(item);
        }

        if step % 17 == 0 {
            let sequence_bytes = sequence.to_snapshot_bytes().unwrap();
            let restored_sequence =
                TrackedSequence::from_snapshot_bytes(encoder, base(2), &sequence_bytes).unwrap();
            assert_eq!(restored_sequence, sequence);

            let multiset_bytes = multiset.to_snapshot_bytes().unwrap();
            let restored_multiset =
                TrackedMultiset::from_snapshot_bytes(encoder, Gf2_128V1::ONE, &multiset_bytes)
                    .unwrap();
            assert_eq!(restored_multiset, multiset);
        }
    }
}

#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
#[test]
fn dynamic_builder_matches_static_field_profile_and_compact_contract() {
    use homomorphic_hash_rs::DynamicSignatureBuilder;
    use microfield::StaticField;
    let field = microfield::DynField::builder("rc2_runtime_gf2_9")
        .binary(9, vec![9, 4, 0])
        .build()
        .unwrap();
    let builder = DynamicSignatureBuilder::new(field.clone(), BinaryPolynomialEncoder::new(17));
    let profile = builder.field_profile();
    assert_eq!(profile.binding(), SignatureFieldBinding::Runtime);
    assert_eq!(
        profile.field_id(),
        Gf2_9StructuralFixture::spec().field_id()
    );
    assert!(profile.characteristic_is_two());
    assert_eq!(profile.extension_degree(), 9);
    assert_eq!(profile.canonical_bytes(), 2);

    let dynamic_base = field.decode(&[2, 0]).unwrap();
    let mut sequence = builder.sequence(dynamic_base).unwrap();
    sequence
        .push_many([b"chunk-a".as_slice(), b"chunk-b"])
        .unwrap();
    assert_eq!(sequence.signature_profile().item_count(), 2);
    assert_eq!(sequence.signature_profile().evaluation_count(), 1);
    assert!(sequence.to_compact_snapshot().unwrap().starts_with(b"MFSG"));
}
