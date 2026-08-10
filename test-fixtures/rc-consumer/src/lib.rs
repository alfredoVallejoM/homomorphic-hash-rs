//! Independent RC.9 consumer using only public APIs.

use std::{fs, path::Path};

use algesum::{
    AdditiveDelta, AdditiveSignature, ApplicationNamespace, BinaryPolynomialEncoder,
    BoundedSetReconciler, CanonicalGraphDag, CanonicalGraphDagLimits, CanonicalSearchBudget,
    DatabaseApplyPolicy, DatabaseColumn, DatabaseColumnType, DatabaseRow, DatabaseSchema,
    DatabaseTransactionLimits, DatabaseTransactionLog, DatabaseValue, DeltaJournal,
    DeltaJournalLimits, FileChunkProfile, GraphDagResolveOutcome, GraphSchemaId,
    HomomorphicSummaryTree, IncidenceGraph, IncidenceGraphBuilder, Microcanon, PartitionedDatabase,
    ReconciliationLimits, RevisionedSignature, RowMutation, SummaryEditPolicy, SummaryTreeLimits,
    TransactionDelta,
};
use microfield::{BinaryPolynomialField, Engine, Field, Gf2_128V1};
use serde::Serialize;
use structural_field_fixture::Gf2_9StructuralFixture;

type Encoder = BinaryPolynomialEncoder;
type ExternalSignature = AdditiveSignature<Gf2_9StructuralFixture, Encoder>;
type ExternalDelta = AdditiveDelta<Gf2_9StructuralFixture, Encoder>;
type Tree = HomomorphicSummaryTree<Gf2_128V1, Encoder>;
type Database = PartitionedDatabase<Gf2_128V1, Encoder>;

const ROW_STORE_MAGIC: &[u8; 4] = b"RCRW";
const ROW_STORE_VERSION: u16 = 1;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ConsumerReport {
    pub schema: &'static str,
    pub passed: bool,
    pub architecture: String,
    pub operating_system: String,
    pub external_field_signature_terms: u64,
    pub signature_revision: u64,
    pub tree_revision: u64,
    pub tree_local_path: String,
    pub tree_fallback_path: String,
    pub database_revision: u64,
    pub database_fallback_path: String,
    pub migrated_database_rows: usize,
    pub graph_revision: u64,
    pub graph_reused: bool,
    pub reconciliation_left_only: Vec<u16>,
    pub reconciliation_right_only: Vec<u16>,
    pub detected_backend: String,
    pub corruption_rejected: bool,
    pub schema_drift_rejected: bool,
}

pub fn run_scenario(directory: &Path) -> Result<ConsumerReport, String> {
    fs::create_dir_all(directory).map_err(io_error)?;
    let encoder = Encoder::new(0x5243_3901);
    let namespace = ApplicationNamespace::derive(b"rc9-external-consumer-v1");

    let initial_items = [b"alpha".as_slice(), b"beta"];
    let mut initial_signature = ExternalSignature::new(encoder);
    initial_signature
        .absorb_many(initial_items)
        .map_err(debug_error)?;
    let added = external_signature(encoder, [b"gamma".as_slice()])?;
    let delta = ExternalDelta::new(namespace, 0, ExternalSignature::new(encoder), added)
        .map_err(debug_error)?;
    let mut journal = DeltaJournal::new();
    journal.append(delta).map_err(debug_error)?;
    atomic_write(
        &directory.join("signature.mfsg"),
        &initial_signature.to_canonical_bytes(),
    )?;
    atomic_write(
        &directory.join("signature-journal.mfdj"),
        &journal.to_canonical_bytes().map_err(debug_error)?,
    )?;

    let profile = FileChunkProfile::fixed(1_024).map_err(debug_error)?;
    let base = Gf2_128V1::from_polynomial_bytes_mod(&[2]);
    let mut exact_file = deterministic_bytes(16_384);
    let mut tree = Tree::from_bytes(profile, encoder, base, &exact_file).map_err(debug_error)?;
    exact_file[2_048..2_112].fill(0xa5);
    let local = tree
        .replace_range_with_policy(2_048..2_112, &[0xa5; 64], SummaryEditPolicy::new(4_096))
        .map_err(debug_error)?;
    exact_file.fill(0x5a);
    let fallback = tree
        .replace_range_with_policy(
            0..exact_file.len(),
            &exact_file,
            SummaryEditPolicy::new(4_096),
        )
        .map_err(debug_error)?;
    atomic_write(
        &directory.join("file-tree.mfst"),
        &tree.to_checkpoint_bytes().map_err(debug_error)?,
    )?;

    let schema = database_schema()?;
    let initial_rows = vec![database_row(0, 1, 10), database_row(1, 1, 11)];
    let target_rows = vec![
        database_row(0, 2, 100),
        initial_rows[1].clone(),
        database_row(2, 1, 12),
    ];
    let transaction = TransactionDelta::new(
        namespace,
        &schema,
        0,
        vec![
            RowMutation::Update {
                before: initial_rows[0].clone(),
                after: target_rows[0].clone(),
            },
            RowMutation::Insert(target_rows[2].clone()),
        ],
    )
    .map_err(debug_error)?;
    let mut transaction_log = DatabaseTransactionLog::new();
    transaction_log
        .append(transaction.clone())
        .map_err(debug_error)?;
    atomic_write(
        &directory.join("database-initial.rcrw"),
        &encode_rows(&schema, &initial_rows)?,
    )?;
    atomic_write(
        &directory.join("database-log.mftl"),
        &transaction_log.to_canonical_bytes().map_err(debug_error)?,
    )?;

    let graph_schema = GraphSchemaId::derive(b"rc9-external-consumer-graph-v1");
    let canonizer = Microcanon::new(graph_schema);
    let graph = path_graph()?;
    let mut dag = CanonicalGraphDag::new(graph_schema);
    dag.resolve(&graph, &canonizer, graph_budget(), &[], None)
        .map_err(debug_error)?;
    atomic_write(&directory.join("graph-dag.mfgd"), &dag.to_canonical_bytes())?;

    drop((initial_signature, journal, tree, dag, transaction_log));

    let restored_signature = ExternalSignature::from_canonical_bytes(
        encoder,
        &fs::read(directory.join("signature.mfsg")).map_err(io_error)?,
    )
    .map_err(debug_error)?;
    let restored_journal = DeltaJournal::<ExternalDelta>::from_canonical_bytes(
        &fs::read(directory.join("signature-journal.mfdj")).map_err(io_error)?,
        DeltaJournalLimits::default(),
        |bytes| ExternalDelta::from_canonical_bytes(encoder, bytes),
    )
    .map_err(debug_error)?;
    let mut revisioned = RevisionedSignature::new(namespace, restored_signature);
    restored_journal
        .replay(&mut revisioned)
        .map_err(debug_error)?;
    let exact_signature = external_signature(encoder, [b"alpha".as_slice(), b"beta", b"gamma"])?;
    if revisioned.state() != &exact_signature {
        return Err("signature replay differs from exact rebuild".into());
    }

    let tree_bytes = fs::read(directory.join("file-tree.mfst")).map_err(io_error)?;
    let restored_tree = Tree::from_checkpoint_bytes(
        profile,
        encoder,
        base,
        &tree_bytes,
        SummaryTreeLimits::default(),
    )
    .map_err(debug_error)?;
    if restored_tree.to_file_bytes().map_err(debug_error)? != exact_file {
        return Err("tree restart differs from exact file".into());
    }
    let rebuilt_tree =
        Tree::from_bytes(profile, encoder, base, &exact_file).map_err(debug_error)?;
    if restored_tree.root() != rebuilt_tree.root() {
        return Err("tree restart root differs from exact rebuild".into());
    }

    let persisted_initial = decode_rows(
        &schema,
        &fs::read(directory.join("database-initial.rcrw")).map_err(io_error)?,
    )?;
    let mut replayed_database = Database::from_rows(
        namespace,
        schema.clone(),
        4,
        encoder,
        Gf2_128V1::ONE,
        persisted_initial.clone(),
    )
    .map_err(debug_error)?;
    let log_bytes = fs::read(directory.join("database-log.mftl")).map_err(io_error)?;
    let restored_log = DatabaseTransactionLog::from_canonical_bytes(
        namespace,
        &schema,
        &log_bytes,
        DatabaseTransactionLimits::default(),
    )
    .map_err(debug_error)?;
    restored_log
        .replay(&mut replayed_database, DatabaseTransactionLimits::default())
        .map_err(debug_error)?;

    let mut policy_database = Database::from_rows(
        namespace,
        schema.clone(),
        4,
        encoder,
        Gf2_128V1::ONE,
        persisted_initial,
    )
    .map_err(debug_error)?;
    let policy_report = policy_database
        .apply_transaction_with_policy(
            &transaction,
            DatabaseTransactionLimits::default(),
            DatabaseApplyPolicy::new(1),
            || target_rows.clone(),
        )
        .map_err(debug_error)?;
    if replayed_database.summary().map_err(debug_error)?
        != policy_database.summary().map_err(debug_error)?
    {
        return Err("database replay differs from authoritative rebuild".into());
    }
    atomic_write(
        &directory.join("database-recovered.rcrw"),
        &encode_rows(&schema, &policy_database.rows())?,
    )?;
    let migrated_schema = DatabaseSchema::new(
        2,
        vec![
            DatabaseColumn::new("id", DatabaseColumnType::U64, false),
            DatabaseColumn::new("payload", DatabaseColumnType::Bytes, false),
            DatabaseColumn::new("note", DatabaseColumnType::Text, true),
        ],
        vec![0],
    )
    .map_err(debug_error)?;
    let migrated_rows = policy_database
        .rows()
        .into_iter()
        .map(|row| {
            let mut values = row.values().to_vec();
            values.push(DatabaseValue::Null);
            DatabaseRow::new(row.version(), values)
        })
        .collect::<Vec<_>>();
    let migrated_database = Database::from_rows(
        namespace,
        migrated_schema.clone(),
        4,
        encoder,
        Gf2_128V1::ONE,
        migrated_rows.clone(),
    )
    .map_err(debug_error)?;
    if migrated_database.row_count() != policy_database.row_count() {
        return Err("database migration changed row cardinality".into());
    }
    atomic_write(
        &directory.join("database-migrated.rcrw"),
        &encode_rows(&migrated_schema, &migrated_rows)?,
    )?;

    let dag_bytes = fs::read(directory.join("graph-dag.mfgd")).map_err(io_error)?;
    let mut restored_dag = CanonicalGraphDag::from_canonical_bytes(
        &dag_bytes,
        &canonizer,
        graph_budget(),
        CanonicalGraphDagLimits::default(),
    )
    .map_err(debug_error)?;
    let graph_reused = matches!(
        restored_dag
            .resolve(
                &graph,
                &canonizer,
                graph_budget(),
                &[],
                Some(restored_dag.revision()),
            )
            .map_err(debug_error)?,
        GraphDagResolveOutcome::Reused { .. }
    );

    let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(64, 4, 4, 1_024))
        .map_err(debug_error)?;
    let left = [1_u16, 2, 3, 5];
    let right = [1_u16, 3, 4, 5];
    let recovered = reconciler
        .reconcile(
            &reconciler.sketch(&left).map_err(debug_error)?,
            &reconciler.sketch(&right).map_err(debug_error)?,
            &right,
        )
        .map_err(debug_error)?;

    let mut corrupted_tree = tree_bytes;
    let last = corrupted_tree.len() - 1;
    corrupted_tree[last] ^= 1;
    let corruption_rejected = Tree::from_checkpoint_bytes(
        profile,
        encoder,
        base,
        &corrupted_tree,
        SummaryTreeLimits::default(),
    )
    .is_err();
    let schema_drift_rejected = DatabaseTransactionLog::from_canonical_bytes(
        namespace,
        &migrated_schema,
        &log_bytes,
        DatabaseTransactionLimits::default(),
    )
    .is_err();
    if !corruption_rejected || !schema_drift_rejected || !graph_reused {
        return Err("restart fail-closed checks did not hold".into());
    }

    let engine = Engine::<Gf2_128V1>::builder()
        .detect()
        .map_err(debug_error)?;
    Ok(ConsumerReport {
        schema: "microfield-rc9-consumer-report-v1",
        passed: true,
        architecture: std::env::consts::ARCH.into(),
        operating_system: std::env::consts::OS.into(),
        external_field_signature_terms: exact_signature.term_count(),
        signature_revision: revisioned.revision(),
        tree_revision: restored_tree.revision(),
        tree_local_path: format!("{:?}", local.path()),
        tree_fallback_path: format!("{:?}", fallback.path()),
        database_revision: policy_database.revision(),
        database_fallback_path: format!("{:?}", policy_report.path()),
        migrated_database_rows: migrated_database.row_count(),
        graph_revision: restored_dag.revision(),
        graph_reused,
        reconciliation_left_only: recovered.only_left().to_vec(),
        reconciliation_right_only: recovered.only_right().to_vec(),
        detected_backend: format!("{:?}", engine.backend_id()),
        corruption_rejected,
        schema_drift_rejected,
    })
}

fn external_signature<'a>(
    encoder: Encoder,
    items: impl IntoIterator<Item = &'a [u8]>,
) -> Result<ExternalSignature, String> {
    let mut signature = ExternalSignature::new(encoder);
    signature.absorb_many(items).map_err(debug_error)?;
    Ok(signature)
}

fn database_schema() -> Result<DatabaseSchema, String> {
    DatabaseSchema::new(
        1,
        vec![
            DatabaseColumn::new("id", DatabaseColumnType::U64, false),
            DatabaseColumn::new("payload", DatabaseColumnType::Bytes, false),
        ],
        vec![0],
    )
    .map_err(debug_error)
}

fn database_row(id: u64, version: u64, salt: u64) -> DatabaseRow {
    DatabaseRow::new(
        version,
        vec![
            DatabaseValue::U64(id),
            DatabaseValue::Bytes(salt.to_le_bytes().to_vec()),
        ],
    )
}

fn encode_rows(schema: &DatabaseSchema, rows: &[DatabaseRow]) -> Result<Vec<u8>, String> {
    let encoded = rows
        .iter()
        .map(|row| schema.encode_row(row).map_err(debug_error))
        .collect::<Result<Vec<_>, _>>()?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(ROW_STORE_MAGIC);
    bytes.extend_from_slice(&ROW_STORE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&(encoded.len() as u64).to_le_bytes());
    for row in encoded {
        bytes.extend_from_slice(&(row.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&row);
    }
    Ok(bytes)
}

fn decode_rows(schema: &DatabaseSchema, bytes: &[u8]) -> Result<Vec<DatabaseRow>, String> {
    if bytes.len() < 14 || &bytes[..4] != ROW_STORE_MAGIC {
        return Err("consumer row-store header".into());
    }
    if u16::from_le_bytes([bytes[4], bytes[5]]) != ROW_STORE_VERSION {
        return Err("consumer row-store version".into());
    }
    let count = read_u64(bytes, 6)? as usize;
    if count > 1_000_000 {
        return Err("consumer row-store row limit".into());
    }
    let mut cursor = 14_usize;
    let mut rows = Vec::with_capacity(count);
    for _ in 0..count {
        let length = read_u64(bytes, cursor)? as usize;
        cursor = cursor.checked_add(8).ok_or("consumer row-store overflow")?;
        let end = cursor
            .checked_add(length)
            .ok_or("consumer row-store overflow")?;
        let row = bytes
            .get(cursor..end)
            .ok_or("consumer row-store truncated row")?;
        rows.push(schema.decode_row(row).map_err(debug_error)?);
        cursor = end;
    }
    if cursor != bytes.len() {
        return Err("consumer row-store trailing bytes".into());
    }
    Ok(rows)
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, String> {
    let end = offset.checked_add(8).ok_or("consumer row-store overflow")?;
    let value = bytes
        .get(offset..end)
        .ok_or("consumer row-store truncated integer")?;
    Ok(u64::from_le_bytes(value.try_into().unwrap()))
}

fn path_graph() -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices = [b"alpha".as_slice(), b"beta", b"gamma", b"delta"]
        .into_iter()
        .map(|label| builder.add_vertex(label))
        .collect::<Vec<_>>();
    for pair in vertices.windows(2) {
        builder
            .add_undirected_relation(pair[0], pair[1], b"edge", b"path", 1)
            .map_err(debug_error)?;
    }
    builder.build().map_err(debug_error)
}

fn graph_budget() -> CanonicalSearchBudget {
    CanonicalSearchBudget::new(100_000)
}

fn deterministic_bytes(count: usize) -> Vec<u8> {
    (0..count)
        .map(|index| (index as u64).wrapping_mul(131).wrapping_add(17) as u8)
        .collect()
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(io_error)?;
    fs::rename(&temporary, path).map_err(io_error)
}

fn io_error(error: std::io::Error) -> String {
    error.to_string()
}

fn debug_error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use algesum::{DatabaseApplyPath, SummaryEditPath};

    #[test]
    fn clean_consumer_persists_restarts_and_rebuilds_all_verticals() {
        let directory = tempfile::tempdir().unwrap();
        let report = run_scenario(directory.path()).unwrap();
        assert_eq!(report.external_field_signature_terms, 3);
        assert_eq!(report.signature_revision, 1);
        assert_eq!(
            report.tree_local_path,
            format!("{:?}", SummaryEditPath::LocalTree)
        );
        assert_eq!(
            report.tree_fallback_path,
            format!("{:?}", SummaryEditPath::BoundaryRebuild)
        );
        assert_eq!(
            report.database_fallback_path,
            format!("{:?}", DatabaseApplyPath::AuthoritativeRebuild)
        );
        assert_eq!(report.migrated_database_rows, 3);
        assert!(report.graph_reused);
        assert!(report.corruption_rejected);
        assert!(report.schema_drift_rejected);
        assert_eq!(report.reconciliation_left_only, vec![2]);
        assert_eq!(report.reconciliation_right_only, vec![4]);
    }

    #[test]
    fn consumer_owned_row_store_rejects_truncation_and_trailing_bytes() {
        let schema = database_schema().unwrap();
        let wire = encode_rows(&schema, &[database_row(1, 1, 7)]).unwrap();
        for length in 0..wire.len() {
            assert!(decode_rows(&schema, &wire[..length]).is_err());
        }
        let mut trailing = wire;
        trailing.push(0);
        assert!(decode_rows(&schema, &trailing).is_err());
    }
}
