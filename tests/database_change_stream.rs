//! External transactional-stream contracts, including sparse PostgreSQL-like LSNs.

#![cfg(feature = "signatures")]

use algesum::{
    ApplicationNamespace, BinaryPolynomialEncoder, CommittedDatabaseTransaction, DatabaseApplyPath,
    DatabaseApplyPolicy, DatabaseApplyStatus, DatabaseChangeStreamAdapter, DatabaseColumn,
    DatabaseColumnType, DatabaseReplicationCheckpoint, DatabaseReplicationError, DatabaseRow,
    DatabaseSchema, DatabaseSourceId, DatabaseTransactionLimits, DatabaseValue,
    PartitionedDatabase, RowMutation, TransactionId,
};
use microfield::{Field, Gf2_128V1};

type Table = PartitionedDatabase<Gf2_128V1, BinaryPolynomialEncoder>;

fn namespace() -> ApplicationNamespace {
    ApplicationNamespace::derive(b"algesum-postgresql-stream-tests-v1")
}

fn source() -> DatabaseSourceId {
    DatabaseSourceId::new([0x50; 32])
}

fn schema() -> DatabaseSchema {
    DatabaseSchema::new(
        1,
        vec![
            DatabaseColumn::new("id", DatabaseColumnType::U64, false),
            DatabaseColumn::new("balance", DatabaseColumnType::I64, false),
            DatabaseColumn::new("label", DatabaseColumnType::Text, true),
        ],
        vec![0],
    )
    .unwrap()
}

fn row(id: u64, version: u64, balance: i64) -> DatabaseRow {
    DatabaseRow::new(
        version,
        vec![
            DatabaseValue::U64(id),
            DatabaseValue::I64(balance),
            DatabaseValue::Text(format!("account-{id}")),
        ],
    )
}

fn empty(schema: &DatabaseSchema) -> Table {
    Table::new(
        namespace(),
        schema.clone(),
        64,
        BinaryPolynomialEncoder::new(0x414c_4745_5355_4d01),
        Gf2_128V1::ONE,
    )
    .unwrap()
}

#[test]
fn sparse_commit_positions_map_to_contiguous_algesum_revisions() {
    let schema = schema();
    let mut table = empty(&schema);
    let mut adapter = DatabaseChangeStreamAdapter::new(namespace(), schema, source());

    for (index, position) in [16_u64, 4096, 65_537, 9_000_000].into_iter().enumerate() {
        let batch = CommittedDatabaseTransaction::new(
            source(),
            position,
            vec![RowMutation::Insert(row(index as u64, 1, index as i64))],
        )
        .unwrap();
        let report = adapter
            .apply_committed(&mut table, &batch, DatabaseTransactionLimits::default())
            .unwrap();
        assert_eq!(report.status(), DatabaseApplyStatus::Applied);
        assert_eq!(report.revision(), index as u64 + 1);
        assert_eq!(adapter.checkpoint().commit_position(), position);
    }
    assert_eq!(table.row_count(), 4);
}

#[test]
fn latest_committed_batch_is_idempotent_but_conflicting_redelivery_fails() {
    let schema = schema();
    let mut table = empty(&schema);
    let mut adapter = DatabaseChangeStreamAdapter::new(namespace(), schema, source());
    let batch = CommittedDatabaseTransaction::new(
        source(),
        8_192,
        vec![RowMutation::Insert(row(1, 1, 10))],
    )
    .unwrap();

    adapter
        .apply_committed(&mut table, &batch, DatabaseTransactionLimits::default())
        .unwrap();
    let duplicate = adapter
        .apply_committed(&mut table, &batch, DatabaseTransactionLimits::default())
        .unwrap();
    assert_eq!(duplicate.status(), DatabaseApplyStatus::AlreadyApplied);
    assert_eq!(table.revision(), 1);
    assert_eq!(table.row_count(), 1);

    let conflict = CommittedDatabaseTransaction::new(
        source(),
        8_192,
        vec![RowMutation::Insert(row(2, 1, 20))],
    )
    .unwrap();
    assert_eq!(
        adapter.apply_committed(&mut table, &conflict, DatabaseTransactionLimits::default()),
        Err(DatabaseReplicationError::PositionConflict)
    );
}

#[test]
fn wrong_source_regression_and_checkpoint_drift_fail_closed() {
    let schema = schema();
    let mut table = empty(&schema);
    let mut adapter = DatabaseChangeStreamAdapter::new(namespace(), schema.clone(), source());
    let first =
        CommittedDatabaseTransaction::new(source(), 100, vec![RowMutation::Insert(row(1, 1, 10))])
            .unwrap();
    adapter
        .apply_committed(&mut table, &first, DatabaseTransactionLimits::default())
        .unwrap();

    let other = CommittedDatabaseTransaction::new(
        DatabaseSourceId::new([0x51; 32]),
        101,
        vec![RowMutation::Insert(row(2, 1, 20))],
    )
    .unwrap();
    assert_eq!(
        adapter.apply_committed(&mut table, &other, DatabaseTransactionLimits::default()),
        Err(DatabaseReplicationError::SourceMismatch)
    );

    let old =
        CommittedDatabaseTransaction::new(source(), 99, vec![RowMutation::Insert(row(2, 1, 20))])
            .unwrap();
    assert_eq!(
        adapter.apply_committed(&mut table, &old, DatabaseTransactionLimits::default()),
        Err(DatabaseReplicationError::PositionRegression {
            previous: 100,
            received: 99,
        })
    );

    let mut fresh_table = empty(&schema);
    assert_eq!(
        adapter.apply_committed(
            &mut fresh_table,
            &first,
            DatabaseTransactionLimits::default()
        ),
        Err(DatabaseReplicationError::RevisionMismatch {
            checkpoint: 1,
            table: 0,
        })
    );
}

#[test]
fn restored_checkpoint_accepts_next_large_update_batch_and_matches_rebuild() {
    let schema = schema();
    let initial = (0..4_096)
        .map(|id| row(id, 1, id as i64))
        .collect::<Vec<_>>();
    let mut table = Table::from_rows(
        namespace(),
        schema.clone(),
        256,
        BinaryPolynomialEncoder::new(0x414c_4745_5355_4d01),
        Gf2_128V1::ONE,
        initial.clone(),
    )
    .unwrap();
    let mut adapter = DatabaseChangeStreamAdapter::new(namespace(), schema.clone(), source());
    let seed = CommittedDatabaseTransaction::new(
        source(),
        10,
        vec![RowMutation::Update {
            before: initial[0].clone(),
            after: row(0, 2, 1),
        }],
    )
    .unwrap();
    adapter
        .apply_committed(&mut table, &seed, DatabaseTransactionLimits::default())
        .unwrap();
    let checkpoint = adapter.checkpoint();
    let persisted_transaction_id = checkpoint
        .last_transaction_id()
        .map(|identity| TransactionId::from_canonical_bytes(*identity.as_bytes()));
    let persisted = DatabaseReplicationCheckpoint::from_parts(
        checkpoint.source(),
        checkpoint.commit_position(),
        checkpoint.algesum_revision(),
        persisted_transaction_id,
    )
    .unwrap();
    let mut restored =
        DatabaseChangeStreamAdapter::from_checkpoint(namespace(), schema.clone(), persisted);

    let mutations = (1..=2_048)
        .map(|id| RowMutation::Update {
            before: initial[id].clone(),
            after: row(id as u64, 2, initial[id].values()[1].clone().as_i64() + 7),
        })
        .collect::<Vec<_>>();
    let batch = CommittedDatabaseTransaction::new(source(), 8_000_000, mutations).unwrap();
    restored
        .apply_committed(&mut table, &batch, DatabaseTransactionLimits::default())
        .unwrap();

    let mut expected = initial;
    expected[0] = row(0, 2, 1);
    for (id, slot) in expected.iter_mut().enumerate().take(2_049).skip(1) {
        *slot = row(id as u64, 2, id as i64 + 7);
    }
    let rebuilt = Table::from_rows(
        namespace(),
        schema,
        256,
        BinaryPolynomialEncoder::new(0x414c_4745_5355_4d01),
        Gf2_128V1::ONE,
        expected,
    )
    .unwrap();
    assert_eq!(table.summary().unwrap(), rebuilt.summary().unwrap());
}

#[test]
fn committed_stream_uses_verified_authoritative_rebuild_above_policy_ceiling() {
    let schema = schema();
    let initial = (0..1_024)
        .map(|id| row(id, 1, id as i64))
        .collect::<Vec<_>>();
    let mut target = initial.clone();
    let mutations = (0..768)
        .map(|id| {
            let after = row(id, 2, id as i64 + 10);
            target[id as usize] = after.clone();
            RowMutation::Update {
                before: initial[id as usize].clone(),
                after,
            }
        })
        .collect();
    let mut table = Table::from_rows(
        namespace(),
        schema.clone(),
        64,
        BinaryPolynomialEncoder::new(0x414c_4745_5355_4d01),
        Gf2_128V1::ONE,
        initial,
    )
    .unwrap();
    let mut adapter = DatabaseChangeStreamAdapter::new(namespace(), schema, source());
    let batch = CommittedDatabaseTransaction::new(source(), 1_000, mutations).unwrap();
    let report = adapter
        .apply_committed_with_policy(
            &mut table,
            &batch,
            DatabaseTransactionLimits::default(),
            DatabaseApplyPolicy::new(256),
            || target.clone(),
        )
        .unwrap();
    assert_eq!(report.path(), DatabaseApplyPath::AuthoritativeRebuild);
    assert_eq!(table.row_count(), target.len());
    for expected in &target {
        assert_eq!(table.get_by_row_key(expected).unwrap(), Some(expected));
    }
}

#[test]
fn checkpoint_parts_reject_torn_or_incomplete_persistence() {
    assert_eq!(
        DatabaseReplicationCheckpoint::from_parts(source(), 10, 0, None),
        Err(DatabaseReplicationError::InvalidCheckpoint)
    );
    assert_eq!(
        DatabaseReplicationCheckpoint::from_parts(source(), 0, 1, None),
        Err(DatabaseReplicationError::InvalidCheckpoint)
    );
}

trait ValueExt {
    fn as_i64(&self) -> i64;
}

impl ValueExt for DatabaseValue {
    fn as_i64(&self) -> i64 {
        match self {
            DatabaseValue::I64(value) => *value,
            _ => panic!("test balance must be i64"),
        }
    }
}
