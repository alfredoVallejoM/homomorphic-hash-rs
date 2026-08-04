//! RC.3 contract for versioned, atomic and persistable signature deltas.

#![cfg(feature = "signatures")]

use homomorphic_hash_rs::{
    AdditiveDelta, AdditiveSignature, ApplicationNamespace, DeltaApplyStatus, DeltaError,
    DeltaJournal, DeltaJournalLimits, DeltaVerification, MultisetDelta, MultisetSignature,
    PrimeIntegerEncoder, RevisionedSignature, SequenceAppend, SequenceSignature, SequenceTrim,
    SignatureDelta,
};
use microfield::{Field, Fp251V1};
use rand::{rngs::StdRng, Rng, SeedableRng};
use structural_field_fixture::Gf2_9StructuralFixture;

type Encoder = PrimeIntegerEncoder;
type Additive = AdditiveSignature<Fp251V1, Encoder>;
type AddDelta = AdditiveDelta<Fp251V1, Encoder>;
type Multiset = MultisetSignature<Fp251V1, Encoder>;
type MultiDelta = MultisetDelta<Fp251V1, Encoder>;
type Sequence = SequenceSignature<Fp251V1, Encoder>;

fn encoder() -> Encoder {
    PrimeIntegerEncoder::new(0x5243_0003)
}

fn namespace() -> ApplicationNamespace {
    ApplicationNamespace::derive(b"rc3-test-dataset-v1")
}

fn additive(items: &[Vec<u8>]) -> Additive {
    let mut signature = Additive::new(encoder());
    signature
        .absorb_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

fn multiset(items: &[Vec<u8>]) -> Multiset {
    let mut signature = Multiset::new(encoder(), Fp251V1::ONE);
    signature
        .insert_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

fn sequence(items: &[Vec<u8>]) -> Sequence {
    let mut signature = Sequence::new(encoder(), Fp251V1::from_u64_mod(7)).unwrap();
    signature
        .push_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

#[test]
fn additive_random_campaign_matches_rebuild_and_replay_is_idempotent() {
    let mut rng = StdRng::seed_from_u64(0xadd1_71e5);
    let mut exact = Vec::<Vec<u8>>::new();
    let mut state = RevisionedSignature::new(namespace(), additive(&exact));
    let mut journal = DeltaJournal::<AddDelta>::new();

    for revision in 0..400_u64 {
        let remove = !exact.is_empty() && rng.gen_bool(0.45);
        let removed = if remove {
            let index = rng.gen_range(0..exact.len());
            vec![exact.swap_remove(index)]
        } else {
            Vec::new()
        };
        let added = if remove && rng.gen_bool(0.35) {
            Vec::new()
        } else {
            let value = rng.gen::<u64>().to_le_bytes().to_vec();
            exact.push(value.clone());
            vec![value]
        };
        let delta =
            AddDelta::new(namespace(), revision, additive(&removed), additive(&added)).unwrap();
        let report = state.apply(&delta).unwrap();
        assert_eq!(report.status(), DeltaApplyStatus::Applied);
        assert_eq!(
            report.verification(),
            DeltaVerification::AlgebraicConsistency
        );
        assert_eq!(state.state(), &additive(&exact), "revision {revision}");
        journal.append(delta).unwrap();
    }

    let final_bytes = state.state().to_canonical_bytes();
    let persisted = journal.to_canonical_bytes().unwrap();
    let decoded = DeltaJournal::<AddDelta>::from_canonical_bytes(
        &persisted,
        DeltaJournalLimits::default(),
        |entry| AddDelta::from_canonical_bytes(encoder(), entry),
    )
    .unwrap();
    let mut replayed = RevisionedSignature::new(namespace(), additive(&[]));
    let first = decoded.replay(&mut replayed).unwrap();
    assert_eq!(first.applied(), 400);
    assert_eq!(first.skipped(), 0);
    assert_eq!(replayed.state().to_canonical_bytes(), final_bytes);
    let second = decoded.replay(&mut replayed).unwrap();
    assert_eq!(second.applied(), 0);
    assert_eq!(second.skipped(), 400);
    assert_eq!(second.revision(), 400);
}

#[test]
fn multiset_random_campaign_handles_duplicates_and_zero_factors() {
    let mut rng = StdRng::seed_from_u64(0x5e7_5eed);
    let mut exact = vec![vec![250]]; // encode(250) + offset(1) = 0 in Fp251.
    let mut state = RevisionedSignature::new(namespace(), multiset(&exact));

    for revision in 0..300_u64 {
        let remove = rng.gen_bool(0.5);
        let removed = if remove {
            let index = rng.gen_range(0..exact.len());
            vec![exact.remove(index)]
        } else {
            Vec::new()
        };
        let added = if exact.is_empty() || !remove || rng.gen_bool(0.7) {
            let value = if rng.gen_bool(0.12) {
                vec![250]
            } else {
                vec![rng.gen_range(0..=249)]
            };
            exact.push(value.clone());
            vec![value]
        } else {
            Vec::new()
        };
        let delta =
            MultiDelta::new(namespace(), revision, multiset(&removed), multiset(&added)).unwrap();
        let wire = delta.to_canonical_bytes();
        let decoded = MultiDelta::from_canonical_bytes(encoder(), Fp251V1::ONE, &wire).unwrap();
        state.apply(&decoded).unwrap();
        assert_eq!(state.state(), &multiset(&exact), "revision {revision}");
    }
}

#[test]
fn sequence_append_and_trim_match_exact_rebuild() {
    let mut exact = Vec::<Vec<u8>>::new();
    let mut state = RevisionedSignature::new(namespace(), sequence(&exact));
    let mut revision = 0_u64;

    for batch in 0..80_u64 {
        let suffix = vec![
            batch.to_le_bytes().to_vec(),
            (batch + 1_000).to_le_bytes().to_vec(),
        ];
        let delta = SequenceAppend::new(namespace(), revision, sequence(&suffix)).unwrap();
        exact.extend(suffix);
        state.apply(&delta).unwrap();
        revision += 1;
        assert_eq!(state.state(), &sequence(&exact));
    }

    for _ in 0..40 {
        let suffix = exact.split_off(exact.len() - 2);
        let delta = SequenceTrim::new(namespace(), revision, sequence(&suffix)).unwrap();
        let wire = delta.to_canonical_bytes();
        let decoded =
            SequenceTrim::from_canonical_bytes(encoder(), Fp251V1::from_u64_mod(7), &wire).unwrap();
        state.apply(&decoded).unwrap();
        revision += 1;
        assert_eq!(state.state(), &sequence(&exact));
    }
}

#[test]
fn failed_preflight_and_candidate_leave_state_revision_and_replay_set_unchanged() {
    let initial = vec![b"present".to_vec()];
    let mut state = RevisionedSignature::new(namespace(), additive(&initial));
    let before = state.state().to_canonical_bytes();

    let wrong_namespace = AddDelta::new(
        ApplicationNamespace::derive(b"other"),
        0,
        additive(&[]),
        additive(&[b"new".to_vec()]),
    )
    .unwrap();
    assert_eq!(
        state.apply(&wrong_namespace),
        Err(DeltaError::NamespaceMismatch)
    );

    let underflow = AddDelta::new(
        namespace(),
        0,
        additive(&[b"a".to_vec(), b"b".to_vec()]),
        additive(&[]),
    )
    .unwrap();
    assert!(matches!(
        state.apply(&underflow),
        Err(DeltaError::Signature(_))
    ));
    assert_eq!(state.revision(), 0);
    assert_eq!(state.state().to_canonical_bytes(), before);

    let valid = AddDelta::new(namespace(), 0, additive(&[]), additive(&[b"new".to_vec()])).unwrap();
    state.apply(&valid).unwrap();
    let committed = state.state().to_canonical_bytes();
    assert_eq!(
        state.apply(&valid).unwrap().status(),
        DeltaApplyStatus::AlreadyApplied
    );
    assert_eq!(state.revision(), 1);
    assert_eq!(state.state().to_canonical_bytes(), committed);

    let stale = AddDelta::new(
        namespace(),
        0,
        additive(&[]),
        additive(&[b"later".to_vec()]),
    )
    .unwrap();
    assert_eq!(
        state.apply(&stale),
        Err(DeltaError::RevisionMismatch {
            expected: 0,
            actual: 1
        })
    );
    assert_eq!(state.state().to_canonical_bytes(), committed);
}

#[test]
fn delta_and_journal_parsers_reject_every_truncation_corruption_and_limit_breach() {
    let delta = AddDelta::new(
        namespace(),
        0,
        additive(&[]),
        additive(&[b"value".to_vec()]),
    )
    .unwrap();
    let wire = delta.to_canonical_bytes();
    assert_eq!(&wire[..4], b"MFDE");
    for length in 0..wire.len() {
        assert!(AddDelta::from_canonical_bytes(encoder(), &wire[..length]).is_err());
    }
    let mut trailing = wire.clone();
    trailing.push(0);
    assert!(AddDelta::from_canonical_bytes(encoder(), &trailing).is_err());
    for index in [0, 4, 6, 7, 40, 48, 56, 64, 96, 128, 160] {
        let mut corrupted = wire.clone();
        corrupted[index] ^= 0x80;
        assert!(
            AddDelta::from_canonical_bytes(encoder(), &corrupted).is_err(),
            "accepted structural corruption at byte {index}"
        );
    }
    let mut other_namespace = wire.clone();
    other_namespace[8] ^= 0x80;
    let changed = AddDelta::from_canonical_bytes(encoder(), &other_namespace).unwrap();
    assert_ne!(changed.envelope().namespace(), namespace());
    assert_ne!(changed.envelope().delta_id(), delta.envelope().delta_id());

    let mut journal = DeltaJournal::<AddDelta>::new();
    journal.append(delta).unwrap();
    let journal_wire = journal.to_canonical_bytes().unwrap();
    assert_eq!(&journal_wire[..4], b"MFDJ");
    for length in 0..journal_wire.len() {
        assert!(DeltaJournal::<AddDelta>::from_canonical_bytes(
            &journal_wire[..length],
            DeltaJournalLimits::default(),
            |entry| AddDelta::from_canonical_bytes(encoder(), entry),
        )
        .is_err());
    }
    let limits = DeltaJournalLimits {
        max_entries: 0,
        ..DeltaJournalLimits::default()
    };
    assert!(matches!(
        DeltaJournal::<AddDelta>::from_canonical_bytes(&journal_wire, limits, |entry| {
            AddDelta::from_canonical_bytes(encoder(), entry)
        }),
        Err(DeltaError::JournalLimitExceeded("entries"))
    ));
}

#[test]
fn journal_rejects_reorder_gap_duplicate_and_context_drift() {
    let first = AddDelta::new(namespace(), 0, additive(&[]), additive(&[b"one".to_vec()])).unwrap();
    let duplicate = first.clone();
    let gap = AddDelta::new(namespace(), 2, additive(&[]), additive(&[b"two".to_vec()])).unwrap();
    let mut journal = DeltaJournal::new();
    journal.append(first).unwrap();
    assert!(matches!(
        journal.append(duplicate),
        Err(DeltaError::InvalidJournal(_))
    ));
    assert!(matches!(
        journal.append(gap),
        Err(DeltaError::InvalidJournal(_))
    ));

    let other_encoder = PrimeIntegerEncoder::new(0xdead_beef);
    let mut other_added = AdditiveSignature::<Fp251V1, _>::new(other_encoder);
    other_added.absorb(b"other-context").unwrap();
    let context_drift = AddDelta::new(
        namespace(),
        1,
        AdditiveSignature::new(other_encoder),
        other_added,
    )
    .unwrap();
    assert!(matches!(
        journal.append(context_drift),
        Err(DeltaError::InvalidJournal("identity drift"))
    ));
}

#[test]
fn journal_replay_is_atomic_when_a_later_candidate_fails() {
    let first = AddDelta::new(namespace(), 0, additive(&[]), additive(&[b"one".to_vec()])).unwrap();
    let invalid_second = AddDelta::new(
        namespace(),
        1,
        additive(&[b"x".to_vec(), b"y".to_vec()]),
        additive(&[]),
    )
    .unwrap();
    let mut journal = DeltaJournal::new();
    journal.append(first).unwrap();
    journal.append(invalid_second).unwrap();

    let initial = additive(&[]);
    let initial_bytes = initial.to_canonical_bytes();
    let mut state = RevisionedSignature::new(namespace(), initial);
    assert!(matches!(
        journal.replay(&mut state),
        Err(DeltaError::Signature(_))
    ));
    assert_eq!(state.revision(), 0);
    assert_eq!(state.state().to_canonical_bytes(), initial_bytes);
}

#[test]
fn delta_contract_is_monomorphized_for_external_generated_fields() {
    let encoder = homomorphic_hash_rs::BinaryPolynomialEncoder::new(0x9003);
    let empty = AdditiveSignature::<Gf2_9StructuralFixture, _>::new(encoder);
    let mut added = AdditiveSignature::<Gf2_9StructuralFixture, _>::new(encoder);
    added
        .absorb_many([b"external".as_slice(), b"field"])
        .unwrap();
    let delta = AdditiveDelta::new(namespace(), 0, empty.clone(), added.clone()).unwrap();
    let wire = delta.to_canonical_bytes();
    let decoded = AdditiveDelta::from_canonical_bytes(encoder, &wire).unwrap();
    let mut state = RevisionedSignature::new(namespace(), empty);
    state.apply(&decoded).unwrap();
    assert_eq!(state.state(), &added);
}
