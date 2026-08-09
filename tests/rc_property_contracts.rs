//! RC.7 shrinkable property contracts for signatures and stateful protocols.

#![cfg(feature = "signatures")]

use std::collections::BTreeMap;

use homomorphic_hash_rs::{
    AdditiveDelta, AdditiveSignature, ApplicationNamespace, BidirectionalSequenceSignature,
    BinaryPolynomialEncoder, DatabaseApplyStatus, DatabaseColumn, DatabaseColumnType, DatabaseRow,
    DatabaseSchema, DatabaseTransactionLimits, DatabaseTransactionLog, DatabaseValue,
    DeltaApplyStatus, DeltaJournal, DeltaJournalLimits, FileChunkProfile, HomomorphicSummaryTree,
    MultiEvaluationMultisetSignature, MultiEvaluationSequenceSignature, MultisetDelta,
    MultisetSignature, PartitionedDatabase, PrimeIntegerEncoder, RevisionedSignature, RowMutation,
    SequenceAppend, SequenceSignature, SequenceTrim, SignatureBuilder, SummaryTreeLimits,
    TrackedMultiset, TrackedSequence, TransactionDelta,
};
use microfield::{BinaryPolynomialField, Field, Fp251V1, Gf2_128V1};
use proptest::prelude::*;

type BinaryEncoder = BinaryPolynomialEncoder;
type Database = PartitionedDatabase<Gf2_128V1, BinaryEncoder>;
type SummaryTree = HomomorphicSummaryTree<Gf2_128V1, BinaryEncoder>;

fn payloads(max_items: usize, max_bytes: usize) -> impl Strategy<Value = Vec<Vec<u8>>> {
    prop::collection::vec(
        prop::collection::vec(any::<u8>(), 0..=max_bytes),
        0..=max_items,
    )
}

fn signature_encoder() -> PrimeIntegerEncoder {
    PrimeIntegerEncoder::new(0x5243_7001)
}

fn sequence_base() -> Fp251V1 {
    Fp251V1::from_u64_mod(7)
}

fn additive(items: &[Vec<u8>]) -> AdditiveSignature<Fp251V1, PrimeIntegerEncoder> {
    let mut signature = AdditiveSignature::new(signature_encoder());
    signature
        .absorb_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

fn sequence(items: &[Vec<u8>]) -> SequenceSignature<Fp251V1, PrimeIntegerEncoder> {
    let mut signature = SequenceSignature::new(signature_encoder(), sequence_base()).unwrap();
    signature
        .push_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

fn multiset(items: &[Vec<u8>]) -> MultisetSignature<Fp251V1, PrimeIntegerEncoder> {
    let mut signature = MultisetSignature::new(signature_encoder(), Fp251V1::ONE);
    signature
        .insert_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

fn bidirectional(
    items: &[Vec<u8>],
) -> BidirectionalSequenceSignature<Fp251V1, PrimeIntegerEncoder> {
    let mut signature =
        BidirectionalSequenceSignature::new(signature_encoder(), sequence_base()).unwrap();
    signature
        .push_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

fn evaluation_bases() -> [Fp251V1; 2] {
    [Fp251V1::from_u64_mod(7), Fp251V1::from_u64_mod(11)]
}

fn evaluation_offsets() -> [Fp251V1; 2] {
    [Fp251V1::ZERO, Fp251V1::ONE]
}

fn multi_sequence(
    items: &[Vec<u8>],
) -> MultiEvaluationSequenceSignature<Fp251V1, PrimeIntegerEncoder, 2> {
    let mut signature =
        MultiEvaluationSequenceSignature::new(signature_encoder(), evaluation_bases()).unwrap();
    signature
        .push_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

fn multi_multiset(
    items: &[Vec<u8>],
) -> MultiEvaluationMultisetSignature<Fp251V1, PrimeIntegerEncoder, 2> {
    let mut signature =
        MultiEvaluationMultisetSignature::new(signature_encoder(), evaluation_offsets()).unwrap();
    signature
        .insert_many(items.iter().map(Vec::as_slice))
        .unwrap();
    signature
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn every_static_signature_law_composes_and_round_trips(
        items in payloads(64, 64),
        split_seed in any::<usize>(),
    ) {
        let split = split_seed % (items.len() + 1);
        let (left, right) = items.split_at(split);

        let additive_all = additive(&items);
        let additive_composed = additive(left).combine(&additive(right)).unwrap();
        prop_assert_eq!(&additive_composed, &additive_all);
        prop_assert_eq!(
            AdditiveSignature::from_canonical_bytes(
                signature_encoder(),
                &additive_all.to_canonical_bytes(),
            ),
            Ok(additive_all.clone()),
        );

        let sequence_all = sequence(&items);
        let sequence_composed = sequence(left).concatenate(&sequence(right)).unwrap();
        prop_assert_eq!(&sequence_composed, &sequence_all);
        prop_assert_eq!(
            SequenceSignature::from_canonical_bytes(
                signature_encoder(),
                sequence_base(),
                &sequence_all.to_canonical_bytes(),
            ),
            Ok(sequence_all.clone()),
        );

        let multiset_all = multiset(&items);
        let multiset_composed = multiset(left).combine(&multiset(right)).unwrap();
        prop_assert_eq!(&multiset_composed, &multiset_all);
        let mut reversed = items.clone();
        reversed.reverse();
        prop_assert_eq!(multiset(&reversed), multiset_all.clone());
        prop_assert_eq!(
            MultisetSignature::from_canonical_bytes(
                signature_encoder(),
                Fp251V1::ONE,
                &multiset_all.to_canonical_bytes(),
            ),
            Ok(multiset_all),
        );

        let bidirectional_all = bidirectional(&items);
        let bidirectional_composed = bidirectional(left).concatenate(&bidirectional(right)).unwrap();
        prop_assert_eq!(&bidirectional_composed, &bidirectional_all);
        prop_assert_eq!(
            BidirectionalSequenceSignature::from_canonical_bytes(
                signature_encoder(), sequence_base(), &bidirectional_all.to_canonical_bytes(),
            ),
            Ok(bidirectional_all),
        );

        let multi_sequence_all = multi_sequence(&items);
        let multi_sequence_composed = multi_sequence(left).concatenate(&multi_sequence(right)).unwrap();
        prop_assert_eq!(&multi_sequence_composed, &multi_sequence_all);
        prop_assert_eq!(
            MultiEvaluationSequenceSignature::from_canonical_bytes(
                signature_encoder(), evaluation_bases(), &multi_sequence_all.to_canonical_bytes(),
            ),
            Ok(multi_sequence_all),
        );

        let multi_multiset_all = multi_multiset(&items);
        let multi_multiset_composed = multi_multiset(left).combine(&multi_multiset(right)).unwrap();
        prop_assert_eq!(&multi_multiset_composed, &multi_multiset_all);
        prop_assert_eq!(multi_multiset(&reversed), multi_multiset_all.clone());
        prop_assert_eq!(
            MultiEvaluationMultisetSignature::from_canonical_bytes(
                signature_encoder(), evaluation_offsets(), &multi_multiset_all.to_canonical_bytes(),
            ),
            Ok(multi_multiset_all),
        );
    }

    #[test]
    fn tracked_signatures_preserve_exact_content_and_multiplicity(
        items in payloads(48, 48),
    ) {
        let mut tracked_sequence =
            TrackedSequence::<Fp251V1, _>::new(signature_encoder(), sequence_base()).unwrap();
        let mut tracked_multiset =
            TrackedMultiset::<Fp251V1, _>::new(signature_encoder(), Fp251V1::ONE);
        for item in &items {
            tracked_sequence.push(item).unwrap();
            tracked_multiset.insert(item).unwrap();
        }

        let sequence_wire = tracked_sequence.to_snapshot_bytes().unwrap();
        let restored_sequence = TrackedSequence::from_snapshot_bytes(
            signature_encoder(),
            sequence_base(),
            &sequence_wire,
        ).unwrap();
        prop_assert_eq!(restored_sequence, tracked_sequence);

        let multiset_wire = tracked_multiset.to_snapshot_bytes().unwrap();
        let restored_multiset = TrackedMultiset::from_snapshot_bytes(
            signature_encoder(),
            Fp251V1::ONE,
            &multiset_wire,
        ).unwrap();
        prop_assert_eq!(&restored_multiset, &tracked_multiset);
        for item in &items {
            let expected = items.iter().filter(|candidate| *candidate == item).count();
            prop_assert_eq!(restored_multiset.multiplicity(item), expected as u64);
        }
    }
}

#[derive(Clone, Debug)]
struct SignatureCommand {
    remove: bool,
    index: usize,
    value: Vec<u8>,
}

fn signature_commands() -> impl Strategy<Value = Vec<SignatureCommand>> {
    prop::collection::vec(
        (
            any::<bool>(),
            any::<usize>(),
            prop::collection::vec(any::<u8>(), 0..=48),
        )
            .prop_map(|(remove, index, value)| SignatureCommand {
                remove,
                index,
                value,
            }),
        0..=128,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn additive_and_multiset_deltas_match_rebuild_and_journal_replay(
        commands in signature_commands(),
    ) {
        let mut exact = Vec::<Vec<u8>>::new();
        let mut additive_state = RevisionedSignature::new(database_namespace(), additive(&exact));
        let mut multiset_state = RevisionedSignature::new(database_namespace(), multiset(&exact));
        let mut additive_journal = DeltaJournal::<AdditiveDelta<Fp251V1, PrimeIntegerEncoder>>::new();
        let mut multiset_journal = DeltaJournal::<MultisetDelta<Fp251V1, PrimeIntegerEncoder>>::new();

        for (revision, command) in commands.iter().enumerate() {
            let (removed, added) = if command.remove && !exact.is_empty() {
                let index = command.index % exact.len();
                (vec![exact.swap_remove(index)], Vec::new())
            } else {
                exact.push(command.value.clone());
                (Vec::new(), vec![command.value.clone()])
            };

            let additive_delta = AdditiveDelta::new(
                database_namespace(),
                revision as u64,
                additive(&removed),
                additive(&added),
            ).unwrap();
            let multiset_delta = MultisetDelta::new(
                database_namespace(),
                revision as u64,
                multiset(&removed),
                multiset(&added),
            ).unwrap();
            prop_assert_eq!(
                additive_state.apply(&additive_delta).unwrap().status(),
                DeltaApplyStatus::Applied,
            );
            prop_assert_eq!(
                multiset_state.apply(&multiset_delta).unwrap().status(),
                DeltaApplyStatus::Applied,
            );
            prop_assert_eq!(additive_state.state(), &additive(&exact));
            prop_assert_eq!(multiset_state.state(), &multiset(&exact));
            additive_journal.append(additive_delta).unwrap();
            multiset_journal.append(multiset_delta).unwrap();
        }

        let additive_journal = DeltaJournal::from_canonical_bytes(
            &additive_journal.to_canonical_bytes().unwrap(),
            DeltaJournalLimits::default(),
            |entry| AdditiveDelta::from_canonical_bytes(signature_encoder(), entry),
        ).unwrap();
        let multiset_journal = DeltaJournal::from_canonical_bytes(
            &multiset_journal.to_canonical_bytes().unwrap(),
            DeltaJournalLimits::default(),
            |entry| MultisetDelta::from_canonical_bytes(
                signature_encoder(), Fp251V1::ONE, entry,
            ),
        ).unwrap();
        let mut additive_replay = RevisionedSignature::new(database_namespace(), additive(&[]));
        let mut multiset_replay = RevisionedSignature::new(database_namespace(), multiset(&[]));
        additive_journal.replay(&mut additive_replay).unwrap();
        multiset_journal.replay(&mut multiset_replay).unwrap();
        prop_assert_eq!(additive_replay.revision(), additive_state.revision());
        prop_assert_eq!(additive_replay.state(), additive_state.state());
        prop_assert_eq!(multiset_replay.revision(), multiset_state.revision());
        prop_assert_eq!(multiset_replay.state(), multiset_state.state());
    }

    #[test]
    fn sequence_append_and_trim_deltas_match_exact_rebuild(
        commands in signature_commands(),
    ) {
        let mut exact = Vec::<Vec<u8>>::new();
        let mut state = RevisionedSignature::new(database_namespace(), sequence(&exact));

        for (revision, command) in commands.iter().enumerate() {
            if command.remove && !exact.is_empty() {
                let count = command.index % exact.len() + 1;
                let suffix = exact.split_off(exact.len() - count);
                let delta = SequenceTrim::new(
                    database_namespace(), revision as u64, sequence(&suffix),
                ).unwrap();
                state.apply(&delta).unwrap();
            } else {
                let suffix = vec![command.value.clone()];
                exact.extend(suffix.clone());
                let delta = SequenceAppend::new(
                    database_namespace(), revision as u64, sequence(&suffix),
                ).unwrap();
                state.apply(&delta).unwrap();
            }
            prop_assert_eq!(state.state(), &sequence(&exact));
        }
    }

    #[test]
    fn rejected_delta_underflow_is_atomic(items in payloads(48, 48)) {
        let mut state = RevisionedSignature::new(database_namespace(), additive(&items));
        let before_revision = state.revision();
        let before_state = state.state().clone();
        let mut removed = items.clone();
        removed.push(b"one-more-than-present".to_vec());
        let delta = AdditiveDelta::new(
            database_namespace(), 0, additive(&removed), additive(&[]),
        ).unwrap();
        prop_assert!(state.apply(&delta).is_err());
        prop_assert_eq!(state.revision(), before_revision);
        prop_assert_eq!(state.state(), &before_state);
    }
}

#[derive(Clone, Debug)]
struct DatabaseCommand {
    kind: u8,
    id: u8,
    salt: u64,
}

fn database_commands() -> impl Strategy<Value = Vec<DatabaseCommand>> {
    prop::collection::vec(
        (any::<u8>(), 0_u8..32, any::<u64>()).prop_map(|(kind, id, salt)| DatabaseCommand {
            kind,
            id,
            salt,
        }),
        1..=128,
    )
}

fn database_namespace() -> ApplicationNamespace {
    ApplicationNamespace::derive(b"rc7-property-database-v1")
}

fn binary_encoder() -> BinaryEncoder {
    BinaryEncoder::new(0x5243_7002)
}

fn database_schema() -> DatabaseSchema {
    DatabaseSchema::new(
        1,
        vec![
            DatabaseColumn::new("id", DatabaseColumnType::U64, false),
            DatabaseColumn::new("payload", DatabaseColumnType::Bytes, false),
            DatabaseColumn::new("note", DatabaseColumnType::Text, true),
        ],
        vec![0],
    )
    .unwrap()
}

fn database_row(id: u8, version: u64, salt: u64) -> DatabaseRow {
    DatabaseRow::new(
        version,
        vec![
            DatabaseValue::U64(u64::from(id)),
            DatabaseValue::Bytes(salt.to_le_bytes().to_vec()),
            if salt.is_multiple_of(3) {
                DatabaseValue::Null
            } else {
                DatabaseValue::Text(format!("row-{id}-{salt}"))
            },
        ],
    )
}

fn empty_database() -> Database {
    Database::new(
        database_namespace(),
        database_schema(),
        8,
        binary_encoder(),
        Gf2_128V1::ONE,
    )
    .unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn partitioned_database_matches_an_exact_state_machine(commands in database_commands()) {
        let schema = database_schema();
        let limits = DatabaseTransactionLimits::default();
        let mut model = BTreeMap::<u8, DatabaseRow>::new();
        let mut database = empty_database();
        let mut log = DatabaseTransactionLog::new();

        for (revision, command) in commands.iter().enumerate() {
            let current = model.get(&command.id).cloned();
            let mutation = match (command.kind % 3, current) {
                (1, Some(before)) => {
                    model.remove(&command.id);
                    RowMutation::Delete(before)
                }
                (_, Some(before)) => {
                    let after = database_row(command.id, before.version() + 1, command.salt);
                    model.insert(command.id, after.clone());
                    RowMutation::Update { before, after }
                }
                (_, None) => {
                    let inserted = database_row(command.id, 1, command.salt);
                    model.insert(command.id, inserted.clone());
                    RowMutation::Insert(inserted)
                }
            };

            let transaction = TransactionDelta::new(
                database_namespace(),
                &schema,
                revision as u64,
                vec![mutation],
            ).unwrap();
            let transaction = TransactionDelta::from_canonical_bytes(
                database_namespace(),
                &schema,
                &transaction.to_canonical_bytes(),
                limits,
            ).unwrap();
            let report = database.apply_transaction(&transaction, limits).unwrap();
            prop_assert_eq!(report.status(), DatabaseApplyStatus::Applied);
            prop_assert_eq!(database.revision(), revision as u64 + 1);
            prop_assert_eq!(database.row_count(), model.len());

            let rebuilt = Database::from_rows(
                database_namespace(),
                schema.clone(),
                8,
                binary_encoder(),
                Gf2_128V1::ONE,
                model.values().cloned(),
            ).unwrap();
            prop_assert_eq!(database.summary().unwrap(), rebuilt.summary().unwrap());
            for expected in model.values() {
                prop_assert_eq!(database.get_by_row_key(expected).unwrap(), Some(expected));
            }
            log.append(transaction).unwrap();
        }

        let log = DatabaseTransactionLog::from_canonical_bytes(
            database_namespace(),
            &schema,
            &log.to_canonical_bytes().unwrap(),
            limits,
        ).unwrap();
        let mut replayed = empty_database();
        let report = log.replay(&mut replayed, limits).unwrap();
        prop_assert_eq!(report.applied(), commands.len() as u64);
        prop_assert_eq!(replayed.summary().unwrap(), database.summary().unwrap());
    }
}

#[derive(Clone, Debug)]
enum FileEdit {
    Replace {
        first: usize,
        second: usize,
        byte: u8,
    },
    Insert {
        offset: usize,
        bytes: Vec<u8>,
    },
    Remove {
        first: usize,
        second: usize,
    },
    Append(Vec<u8>),
    Truncate(usize),
}

fn file_edits() -> impl Strategy<Value = Vec<FileEdit>> {
    prop::collection::vec(
        prop_oneof![
            (any::<usize>(), any::<usize>(), any::<u8>()).prop_map(|(first, second, byte)| {
                FileEdit::Replace {
                    first,
                    second,
                    byte,
                }
            }),
            (any::<usize>(), prop::collection::vec(any::<u8>(), 0..=24))
                .prop_map(|(offset, bytes)| FileEdit::Insert { offset, bytes }),
            (any::<usize>(), any::<usize>())
                .prop_map(|(first, second)| FileEdit::Remove { first, second }),
            prop::collection::vec(any::<u8>(), 0..=24).prop_map(FileEdit::Append),
            any::<usize>().prop_map(FileEdit::Truncate),
        ],
        0..=64,
    )
}

fn summary_base() -> Gf2_128V1 {
    Gf2_128V1::from_polynomial_bytes_mod(&[2])
}

fn rebuild_summary(profile: FileChunkProfile, bytes: &[u8]) -> SummaryTree {
    SummaryTree::from_bytes(profile, binary_encoder(), summary_base(), bytes).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn summary_tree_matches_exact_bytes_after_every_edit(
        initial in prop::collection::vec(any::<u8>(), 0..=256),
        edits in file_edits(),
    ) {
        let profile = FileChunkProfile::fixed(23).unwrap();
        let mut exact = initial;
        let mut tree = rebuild_summary(profile, &exact);

        for edit in edits {
            match edit {
                FileEdit::Replace { first, second, byte } if !exact.is_empty() => {
                    let mut start = first % exact.len();
                    let mut end = second % exact.len();
                    if start > end {
                        std::mem::swap(&mut start, &mut end);
                    }
                    end += 1;
                    let replacement = vec![byte; end - start];
                    exact[start..end].copy_from_slice(&replacement);
                    tree.replace_range(start..end, &replacement).unwrap();
                }
                FileEdit::Insert { offset, bytes } => {
                    let offset = offset % (exact.len() + 1);
                    exact.splice(offset..offset, bytes.iter().copied());
                    tree.insert_range(offset, &bytes).unwrap();
                }
                FileEdit::Remove { first, second } if !exact.is_empty() => {
                    let mut start = first % exact.len();
                    let mut end = second % exact.len();
                    if start > end {
                        std::mem::swap(&mut start, &mut end);
                    }
                    end += 1;
                    exact.drain(start..end);
                    tree.remove_range(start..end).unwrap();
                }
                FileEdit::Append(bytes) => {
                    exact.extend_from_slice(&bytes);
                    tree.append(&bytes).unwrap();
                }
                FileEdit::Truncate(seed) => {
                    let new_len = seed % (exact.len() + 1);
                    exact.truncate(new_len);
                    tree.truncate(new_len).unwrap();
                }
                FileEdit::Replace { .. } | FileEdit::Remove { .. } => {}
            }

            prop_assert_eq!(tree.to_file_bytes().unwrap(), exact.clone());
            prop_assert_eq!(tree.root(), rebuild_summary(profile, &exact).root());
            let checkpoint = tree.to_checkpoint_bytes().unwrap();
            let restored = SummaryTree::from_checkpoint_bytes(
                profile,
                binary_encoder(),
                summary_base(),
                &checkpoint,
                SummaryTreeLimits::default(),
            ).unwrap();
            prop_assert_eq!(restored.root(), tree.root());
            prop_assert_eq!(restored.to_file_bytes().unwrap(), exact.clone());
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_024))]

    #[test]
    fn public_wire_parsers_never_panic_on_bounded_arbitrary_bytes(
        bytes in prop::collection::vec(any::<u8>(), 0..=2_048),
    ) {
        let _ = AdditiveSignature::<Fp251V1, _>::from_canonical_bytes(signature_encoder(), &bytes);
        let _ = SequenceSignature::<Fp251V1, _>::from_canonical_bytes(
            signature_encoder(), sequence_base(), &bytes,
        );
        let _ = MultisetSignature::<Fp251V1, _>::from_canonical_bytes(
            signature_encoder(), Fp251V1::ONE, &bytes,
        );
        let _ = TrackedSequence::<Fp251V1, _>::from_snapshot_bytes(
            signature_encoder(), sequence_base(), &bytes,
        );
        let _ = TrackedMultiset::<Fp251V1, _>::from_snapshot_bytes(
            signature_encoder(), Fp251V1::ONE, &bytes,
        );
        let _ = AdditiveDelta::<Fp251V1, _>::from_canonical_bytes(signature_encoder(), &bytes);
        let _ = MultisetDelta::<Fp251V1, _>::from_canonical_bytes(
            signature_encoder(), Fp251V1::ONE, &bytes,
        );
        let _ = SequenceAppend::<Fp251V1, _>::from_canonical_bytes(
            signature_encoder(), sequence_base(), &bytes,
        );
        let _ = SequenceTrim::<Fp251V1, _>::from_canonical_bytes(
            signature_encoder(), sequence_base(), &bytes,
        );
        let _ = DeltaJournal::<AdditiveDelta<Fp251V1, PrimeIntegerEncoder>>::from_canonical_bytes(
            &bytes,
            DeltaJournalLimits::default(),
            |entry| AdditiveDelta::from_canonical_bytes(signature_encoder(), entry),
        );

        let schema = database_schema();
        let _ = schema.decode_row(&bytes);
        let _ = TransactionDelta::from_canonical_bytes(
            database_namespace(), &schema, &bytes, DatabaseTransactionLimits::default(),
        );
        let _ = DatabaseTransactionLog::from_canonical_bytes(
            database_namespace(), &schema, &bytes, DatabaseTransactionLimits::default(),
        );
        let profile = FileChunkProfile::fixed(23).unwrap();
        let _ = SummaryTree::from_checkpoint_bytes(
            profile,
            binary_encoder(),
            summary_base(),
            &bytes,
            SummaryTreeLimits::default(),
        );
    }
}

#[test]
fn signature_builder_remains_available_to_external_property_consumers() {
    let builder = SignatureBuilder::<Fp251V1, _>::new(signature_encoder());
    assert_eq!(builder.additive(), additive(&[]));
    let state = RevisionedSignature::new(
        ApplicationNamespace::derive(b"rc7-builder-smoke"),
        builder.additive(),
    );
    assert_eq!(state.revision(), 0);
}
