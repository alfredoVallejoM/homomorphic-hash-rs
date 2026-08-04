//! RC.5 contract for database transactions and bounded set reconciliation.

#![cfg(feature = "signatures")]

use std::collections::BTreeMap;

use homomorphic_hash_rs::{
    ApplicationNamespace, BinaryPolynomialEncoder, BoundedSetReconciler, DatabaseApplyStatus,
    DatabaseColumn, DatabaseColumnType, DatabaseError, DatabaseRow, DatabaseSchema,
    DatabaseTransactionLimits, DatabaseTransactionLog, DatabaseValue, PartitionedDatabase,
    ReconciliationError, ReconciliationLimits, RowMutation, TransactionDelta,
};
use microfield::{Field, Gf2_128V1};
use rand::{rngs::StdRng, seq::IteratorRandom, Rng, SeedableRng};
use structural_field_fixture::Gf2_9StructuralFixture;

type Encoder = BinaryPolynomialEncoder;
type Table = PartitionedDatabase<Gf2_128V1, Encoder>;

fn namespace() -> ApplicationNamespace {
    ApplicationNamespace::derive(b"rc5-database-v1")
}

fn encoder() -> Encoder {
    BinaryPolynomialEncoder::new(0x5243_0005)
}

fn schema() -> DatabaseSchema {
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

fn row(id: u64, version: u64, salt: u64) -> DatabaseRow {
    DatabaseRow::new(
        version,
        vec![
            DatabaseValue::U64(id),
            DatabaseValue::Bytes(salt.to_le_bytes().to_vec()),
            if salt.is_multiple_of(3) {
                DatabaseValue::Null
            } else {
                DatabaseValue::Text(format!("row-{id}-{salt}"))
            },
        ],
    )
}

fn empty_table() -> Table {
    Table::new(namespace(), schema(), 16, encoder(), Gf2_128V1::ONE).unwrap()
}

#[test]
fn row_schema_round_trips_types_nullability_identity_and_primary_key() {
    let schema = DatabaseSchema::new(
        7,
        vec![
            DatabaseColumn::new("key", DatabaseColumnType::U64, false),
            DatabaseColumn::new("active", DatabaseColumnType::Bool, false),
            DatabaseColumn::new("signed", DatabaseColumnType::I64, false),
            DatabaseColumn::new("raw", DatabaseColumnType::Bytes, false),
            DatabaseColumn::new("label", DatabaseColumnType::Text, true),
        ],
        vec![0],
    )
    .unwrap();
    let value = DatabaseRow::new(
        11,
        vec![
            DatabaseValue::U64(42),
            DatabaseValue::Bool(true),
            DatabaseValue::I64(-17),
            DatabaseValue::Bytes(vec![0, 1, 0, 2]),
            DatabaseValue::Null,
        ],
    );
    let wire = schema.encode_row(&value).unwrap();
    assert_eq!(&wire[..4], b"MFRW");
    assert_eq!(schema.decode_row(&wire).unwrap(), value);
    assert_eq!(
        schema.row_key(&value).unwrap(),
        schema.row_key(&value).unwrap()
    );

    let changed_version = DatabaseRow::new(12, value.values().to_vec());
    assert_eq!(
        schema.row_key(&value).unwrap(),
        schema.row_key(&changed_version).unwrap()
    );
    assert_ne!(
        schema.encode_row(&value).unwrap(),
        schema.encode_row(&changed_version).unwrap()
    );

    let wrong_type = DatabaseRow::new(
        1,
        vec![
            DatabaseValue::Text("not-u64".into()),
            DatabaseValue::Bool(true),
            DatabaseValue::I64(0),
            DatabaseValue::Bytes(Vec::new()),
            DatabaseValue::Null,
        ],
    );
    assert!(matches!(
        schema.encode_row(&wrong_type),
        Err(DatabaseError::InvalidRow(_))
    ));
}

#[test]
fn random_transaction_log_matches_exact_table_and_rebuild_after_every_commit() {
    let schema = schema();
    let mut rng = StdRng::seed_from_u64(0x5243_5005);
    let mut exact = BTreeMap::<u64, DatabaseRow>::new();
    let mut table = empty_table();
    let mut log = DatabaseTransactionLog::new();

    for revision in 0..400_u64 {
        let mutation = if exact.is_empty() || rng.gen_bool(0.42) {
            let mut id = rng.gen_range(0..10_000_u64);
            while exact.contains_key(&id) {
                id = rng.gen_range(0..10_000_u64);
            }
            let inserted = row(id, 1, rng.gen());
            exact.insert(id, inserted.clone());
            RowMutation::Insert(inserted)
        } else {
            let id = *exact.keys().choose(&mut rng).unwrap();
            let before = exact.get(&id).unwrap().clone();
            if rng.gen_bool(0.28) {
                exact.remove(&id);
                RowMutation::Delete(before)
            } else {
                let after = row(id, before.version() + 1, rng.gen());
                exact.insert(id, after.clone());
                RowMutation::Update { before, after }
            }
        };
        let transaction =
            TransactionDelta::new(namespace(), &schema, revision, vec![mutation]).unwrap();
        let wire = transaction.to_canonical_bytes();
        let decoded = TransactionDelta::from_canonical_bytes(
            namespace(),
            &schema,
            &wire,
            DatabaseTransactionLimits::default(),
        )
        .unwrap();
        let report = table
            .apply_transaction(&decoded, DatabaseTransactionLimits::default())
            .unwrap();
        assert_eq!(report.status(), DatabaseApplyStatus::Applied);
        assert_eq!(report.touched_partitions(), 1);
        assert_eq!(table.revision(), revision + 1);
        let rebuilt = Table::from_rows(
            namespace(),
            schema.clone(),
            16,
            encoder(),
            Gf2_128V1::ONE,
            exact.values().cloned(),
        )
        .unwrap();
        assert_eq!(table.summary().unwrap(), rebuilt.summary().unwrap());
        assert_eq!(table.row_count(), exact.len());
        for expected in exact.values() {
            assert_eq!(table.get_by_row_key(expected).unwrap(), Some(expected));
        }
        log.append(decoded).unwrap();
    }

    let log_wire = log.to_canonical_bytes().unwrap();
    let restored = DatabaseTransactionLog::from_canonical_bytes(
        namespace(),
        &schema,
        &log_wire,
        DatabaseTransactionLimits::default(),
    )
    .unwrap();
    let mut replayed = empty_table();
    let first = restored
        .replay(&mut replayed, DatabaseTransactionLimits::default())
        .unwrap();
    assert_eq!(first.applied(), 400);
    assert_eq!(replayed.summary().unwrap(), table.summary().unwrap());
    let second = restored
        .replay(&mut replayed, DatabaseTransactionLimits::default())
        .unwrap();
    assert_eq!(second.applied(), 0);
    assert_eq!(second.skipped(), 400);
    assert_eq!(second.revision(), 400);
}

#[test]
fn transaction_conflicts_limits_and_mid_log_failure_are_atomic() {
    let schema = schema();
    let original = row(1, 1, 10);
    let mut table = Table::from_rows(
        namespace(),
        schema.clone(),
        4,
        encoder(),
        Gf2_128V1::ONE,
        [original.clone()],
    )
    .unwrap();
    let before = table.summary().unwrap();
    let wrong_before = row(1, 1, 999);
    let conflict = TransactionDelta::new(
        namespace(),
        &schema,
        0,
        vec![RowMutation::Delete(wrong_before)],
    )
    .unwrap();
    assert!(matches!(
        table.apply_transaction(&conflict, DatabaseTransactionLimits::default()),
        Err(DatabaseError::Conflict(_))
    ));
    assert_eq!(table.revision(), 0);
    assert_eq!(table.summary().unwrap(), before);

    let first = TransactionDelta::new(
        namespace(),
        &schema,
        0,
        vec![RowMutation::Insert(row(2, 1, 20))],
    )
    .unwrap();
    let invalid_second = TransactionDelta::new(
        namespace(),
        &schema,
        1,
        vec![RowMutation::Delete(row(99, 1, 0))],
    )
    .unwrap();
    let mut log = DatabaseTransactionLog::new();
    log.append(first).unwrap();
    log.append(invalid_second).unwrap();
    assert!(log
        .replay(&mut table, DatabaseTransactionLimits::default())
        .is_err());
    assert_eq!(table.revision(), 0);
    assert_eq!(table.summary().unwrap(), before);
}

#[test]
fn multi_row_transaction_commits_insert_update_delete_as_one_revision() {
    let schema = schema();
    let first = row(1, 1, 10);
    let second = row(2, 1, 20);
    let mut table = Table::from_rows(
        namespace(),
        schema.clone(),
        8,
        encoder(),
        Gf2_128V1::ONE,
        [first.clone(), second.clone()],
    )
    .unwrap();
    let updated = row(1, 2, 11);
    let inserted = row(3, 1, 30);
    let transaction = TransactionDelta::new(
        namespace(),
        &schema,
        0,
        vec![
            RowMutation::Update {
                before: first,
                after: updated.clone(),
            },
            RowMutation::Delete(second),
            RowMutation::Insert(inserted.clone()),
        ],
    )
    .unwrap();
    table
        .apply_transaction(&transaction, DatabaseTransactionLimits::default())
        .unwrap();
    assert_eq!(table.revision(), 1);
    assert_eq!(table.row_count(), 2);
    assert_eq!(table.get_by_row_key(&updated).unwrap(), Some(&updated));
    assert_eq!(table.get_by_row_key(&inserted).unwrap(), Some(&inserted));
}

#[test]
fn transaction_and_log_wires_reject_every_truncated_prefix() {
    let schema = schema();
    let transaction = TransactionDelta::new(
        namespace(),
        &schema,
        0,
        vec![RowMutation::Insert(row(7, 1, 70))],
    )
    .unwrap();
    let wire = transaction.to_canonical_bytes();
    assert_eq!(&wire[..4], b"MFTX");
    for length in 0..wire.len() {
        assert!(TransactionDelta::from_canonical_bytes(
            namespace(),
            &schema,
            &wire[..length],
            DatabaseTransactionLimits::default()
        )
        .is_err());
    }
    let mut log = DatabaseTransactionLog::new();
    log.append(transaction).unwrap();
    let log_wire = log.to_canonical_bytes().unwrap();
    assert_eq!(&log_wire[..4], b"MFTL");
    for length in 0..log_wire.len() {
        assert!(DatabaseTransactionLog::from_canonical_bytes(
            namespace(),
            &schema,
            &log_wire[..length],
            DatabaseTransactionLimits::default()
        )
        .is_err());
    }
}

#[test]
fn partitioned_database_generalizes_to_external_generated_fields() {
    let schema = schema();
    let encoder = BinaryPolynomialEncoder::new(0x9005);
    let mut table = PartitionedDatabase::<Gf2_9StructuralFixture, _>::new(
        namespace(),
        schema.clone(),
        4,
        encoder,
        Gf2_9StructuralFixture::ONE,
    )
    .unwrap();
    let transaction = TransactionDelta::new(
        namespace(),
        &schema,
        0,
        vec![RowMutation::Insert(row(1, 1, 1))],
    )
    .unwrap();
    table
        .apply_transaction(&transaction, DatabaseTransactionLimits::default())
        .unwrap();
    assert_eq!(table.summary().unwrap().row_count(), 1);
}

#[test]
fn reconciliation_recovers_all_63232_bounded_pairs_and_rejects_outside_sample() {
    let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(8, 6, 6, 1_024)).unwrap();
    let mut recovered = 0_u64;
    let mut rejected = 0_u64;
    for left_mask in 0_u16..256 {
        for right_mask in 0_u16..256 {
            let left = mask_set(left_mask, 8);
            let right = mask_set(right_mask, 8);
            if (left_mask ^ right_mask).count_ones() > 6 {
                assert_eq!(
                    reconciler.reconcile(
                        &reconciler.sketch(&left).unwrap(),
                        &reconciler.sketch(&right).unwrap(),
                        &right,
                    ),
                    Err(ReconciliationError::DifferenceExceedsBound)
                );
                rejected += 1;
                continue;
            }
            let difference = reconciler
                .reconcile(
                    &reconciler.sketch(&left).unwrap(),
                    &reconciler.sketch(&right).unwrap(),
                    &right,
                )
                .unwrap();
            let expected_left = left
                .iter()
                .copied()
                .filter(|value| !right.contains(value))
                .collect::<Vec<_>>();
            let expected_right = right
                .iter()
                .copied()
                .filter(|value| !left.contains(value))
                .collect::<Vec<_>>();
            assert_eq!(difference.only_left(), expected_left);
            assert_eq!(difference.only_right(), expected_right);
            recovered += 1;
        }
    }
    assert_eq!(recovered, 63_232);
    assert_eq!(rejected, 2_304);

    let bounded = BoundedSetReconciler::new(ReconciliationLimits::new(32, 5, 5, 1_024)).unwrap();
    let left = [0, 1, 2];
    let right = [3, 4, 5];
    assert_eq!(
        bounded.reconcile(
            &bounded.sketch(&left).unwrap(),
            &bounded.sketch(&right).unwrap(),
            &right
        ),
        Err(ReconciliationError::DifferenceExceedsBound)
    );
}

#[test]
fn reconciliation_wire_limits_and_set_semantics_are_fail_closed() {
    let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(64, 6, 8, 1_024)).unwrap();
    assert_eq!(
        reconciler.sketch(&[1, 1]),
        Err(ReconciliationError::InvalidSet)
    );
    assert_eq!(
        reconciler.sketch(&[64]),
        Err(ReconciliationError::InvalidSet)
    );
    assert!(BoundedSetReconciler::new(ReconciliationLimits::new(64, 8, 7, 1_024)).is_err());
    assert!(BoundedSetReconciler::new(ReconciliationLimits::new(64, 8, 8, 1)).is_err());

    let sketch = reconciler.sketch(&[1, 3, 7, 11]).unwrap();
    let wire = sketch.to_canonical_bytes();
    assert_eq!(&wire[..4], b"MFRS");
    assert_eq!(
        reconciler.sketch_from_canonical_bytes(&wire).unwrap(),
        sketch
    );
    for length in 0..wire.len() {
        assert!(reconciler
            .sketch_from_canonical_bytes(&wire[..length])
            .is_err());
    }
    let other = BoundedSetReconciler::new(ReconciliationLimits::new(65, 6, 8, 1_024)).unwrap();
    assert_eq!(
        other.sketch_from_canonical_bytes(&wire),
        Err(ReconciliationError::ProfileMismatch)
    );
    let mut noncanonical = wire.clone();
    *noncanonical.last_mut().unwrap() = 251;
    assert!(matches!(
        reconciler.sketch_from_canonical_bytes(&noncanonical),
        Err(ReconciliationError::InvalidWire(_))
    ));
}

fn mask_set(mask: u16, universe: u16) -> Vec<u16> {
    (0..universe)
        .filter(|bit| mask & (1_u16 << bit) != 0)
        .collect()
}
