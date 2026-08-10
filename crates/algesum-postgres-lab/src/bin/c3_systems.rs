//! C3 PostgreSQL systems preflight: logical decoding, concurrency, backlog,
//! schema migration and durable restart verification.

use std::{
    collections::BTreeMap,
    env,
    fs::File,
    path::PathBuf,
    process::ExitCode,
    sync::{Arc, Barrier},
    thread,
    time::Instant,
};

use algesum::{
    ApplicationNamespace, BinaryPolynomialEncoder, CommittedDatabaseTransaction,
    DatabaseChangeStreamAdapter, DatabaseColumn, DatabaseColumnType, DatabaseRow, DatabaseSchema,
    DatabaseSourceId, DatabaseTransactionLimits, DatabaseValue, PartitionedDatabase, RowMutation,
};
use microfield::{CanonicalEncoding, Field, Gf2_128V1};
use postgres::{Client, NoTls};
use serde::Serialize;

type Table = PartitionedDatabase<Gf2_128V1, BinaryPolynomialEncoder>;

const SLOT: &str = "algesum_c3_slot";

#[derive(Debug)]
struct Config {
    database_url: String,
    rows: usize,
    partitions: usize,
    transactions_per_client: usize,
    clients: Vec<usize>,
    phase: Phase,
    output: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Prepare,
    VerifyRestart,
}

#[derive(Debug, Serialize)]
struct PrepareReport {
    schema: &'static str,
    postgres_version: String,
    wal_level: String,
    rows: usize,
    partitions: usize,
    transactions_per_client: usize,
    concurrency: Vec<ConcurrencyResult>,
    backpressure: Vec<BackpressureResult>,
    migration: MigrationResult,
    restart_checkpoint: RestartCheckpoint,
    caveat: &'static str,
}

#[derive(Debug, Serialize)]
struct ConcurrencyResult {
    clients: usize,
    committed_transactions: usize,
    elapsed_microseconds: u128,
    transactions_per_second: f64,
    decoded_transactions: usize,
    decoded_account_updates: usize,
    lsn_strictly_monotonic: bool,
    exact_rows_match: bool,
    summary_matches_rebuild: bool,
}

#[derive(Debug, Serialize)]
struct BackpressureResult {
    burst_factor: usize,
    batch_size: usize,
    initial_backlog: usize,
    maximum_backlog: usize,
    final_backlog: usize,
    batches: usize,
    drain_microseconds: u128,
    backlog_monotonic: bool,
    revisions_contiguous: bool,
}

#[derive(Debug, Serialize)]
struct MigrationResult {
    compatible_add_column: bool,
    old_projection_stable: bool,
    migrated_summary_deterministic: bool,
    incompatible_change_detected: bool,
    rollback_preserved_schema: bool,
}

#[derive(Debug, Serialize)]
struct RestartCheckpoint {
    revision: i64,
    rows: i64,
    balance_sum: i64,
    schema_version: i32,
    summary_match_at_prepare: bool,
}

#[derive(Debug, Serialize)]
struct RestartReport {
    schema: &'static str,
    postgres_version: String,
    recovery_state: String,
    revision: i64,
    rows: i64,
    balance_sum: i64,
    schema_version: i32,
    checkpoint_fields_match: bool,
    summary_bytes_match: bool,
    summary_matches_fresh_rebuild: bool,
}

#[derive(Debug)]
struct Event {
    revision: u64,
    account_id: u64,
    before_version: u64,
    before_balance: i64,
    after_version: u64,
    after_balance: i64,
}

#[derive(Debug)]
struct DecodedTransaction {
    revision: u64,
    commit_lsn: u64,
    account_updates: usize,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("algesum-postgres-c3-systems: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let config = parse_config()?;
    let report = match config.phase {
        Phase::Prepare => serde_json::to_value(run_prepare(&config)?),
        Phase::VerifyRestart => serde_json::to_value(run_verify_restart(&config)?),
    }
    .map_err(|error| error.to_string())?;
    let output = File::create(&config.output)
        .map_err(|error| format!("cannot create {}: {error}", config.output.display()))?;
    serde_json::to_writer_pretty(output, &report).map_err(|error| error.to_string())?;
    println!("wrote {}", config.output.display());
    Ok(())
}

fn run_prepare(config: &Config) -> Result<PrepareReport, String> {
    if config.rows
        < config.clients.iter().copied().max().unwrap_or(0) * config.transactions_per_client
        || config.rows == 0
        || config.partitions == 0
        || config.transactions_per_client == 0
    {
        return Err(
            "rows must cover every client transaction and all sizes must be nonzero".into(),
        );
    }
    let mut control = connect(&config.database_url)?;
    let postgres_version: String = control
        .query_one("SHOW server_version", &[])
        .map_err(string_error)?
        .get(0);
    let wal_level: String = control
        .query_one("SHOW wal_level", &[])
        .map_err(string_error)?
        .get(0);
    if wal_level != "logical" {
        return Err(format!("wal_level must be logical, found {wal_level:?}"));
    }
    initialize(&mut control, config.rows)?;

    let mut concurrency = Vec::with_capacity(config.clients.len());
    for &clients in &config.clients {
        reset_data(&mut control, config.rows)?;
        recreate_slot(&mut control)?;
        concurrency.push(run_concurrency(config, clients)?);
    }
    let last_clients = *config.clients.last().ok_or("--clients cannot be empty")?;
    let total_events = last_clients * config.transactions_per_client;
    let backpressure = [2, 5]
        .into_iter()
        .map(|factor| run_backpressure(&mut control, total_events, factor))
        .collect::<Result<Vec<_>, _>>()?;
    let migration = run_migration(&mut control, config.partitions)?;
    let restart_checkpoint = persist_restart_checkpoint(&mut control, config.partitions)?;
    drop_slot_if_present(&mut control)?;

    Ok(PrepareReport {
        schema: "algesum-c3-postgresql-systems-prepare-v1",
        postgres_version,
        wal_level,
        rows: config.rows,
        partitions: config.partitions,
        transactions_per_client: config.transactions_per_client,
        concurrency,
        backpressure,
        migration,
        restart_checkpoint,
        caveat: "preflight on a shared host; timings are diagnostic and not publication benchmarks",
    })
}

fn run_concurrency(config: &Config, clients: usize) -> Result<ConcurrencyResult, String> {
    let barrier = Arc::new(Barrier::new(clients));
    let started = Instant::now();
    let mut handles = Vec::with_capacity(clients);
    for client_index in 0..clients {
        let database_url = config.database_url.clone();
        let barrier = Arc::clone(&barrier);
        let transactions = config.transactions_per_client;
        handles.push(thread::spawn(move || -> Result<(), String> {
            let mut client = connect(&database_url)?;
            barrier.wait();
            for transaction_index in 0..transactions {
                let account_id = i64::try_from(client_index * transactions + transaction_index)
                    .map_err(|_| "account id does not fit bigint")?;
                update_one(&mut client, client_index, account_id)?;
            }
            Ok(())
        }));
    }
    for handle in handles {
        handle.join().map_err(|_| "PostgreSQL worker panicked")??;
    }
    let elapsed = started.elapsed();
    let committed_transactions = clients * config.transactions_per_client;

    let mut client = connect(&config.database_url)?;
    let decoded = decode_slot(&mut client)?;
    let events = load_events(&mut client)?;
    if decoded.len() != committed_transactions || events.len() != committed_transactions {
        return Err(format!(
            "transaction cardinality mismatch: expected {committed_transactions}, decoded {}, events {}",
            decoded.len(),
            events.len()
        ));
    }
    let positions = decoded
        .iter()
        .map(|transaction| (transaction.revision, transaction.commit_lsn))
        .collect::<BTreeMap<_, _>>();
    let lsn_strictly_monotonic = decoded
        .windows(2)
        .all(|window| window[0].commit_lsn < window[1].commit_lsn);
    let decoded_account_updates = decoded.iter().map(|item| item.account_updates).sum();

    let schema = account_schema_v1()?;
    let namespace = namespace();
    let source = source();
    let initial = initial_rows(config.rows);
    let mut exact = initial.clone();
    let mut table = build_table(namespace, &schema, config.partitions, initial)?;
    let mut adapter = DatabaseChangeStreamAdapter::new(namespace, schema.clone(), source);
    for event in events {
        let index = usize::try_from(event.account_id).map_err(|error| error.to_string())?;
        let before = account_row(event.account_id, event.before_version, event.before_balance);
        let after = account_row(event.account_id, event.after_version, event.after_balance);
        if exact.get(index) != Some(&before) {
            return Err(format!(
                "event before-image mismatch at revision {}",
                event.revision
            ));
        }
        exact[index] = after.clone();
        let position = *positions
            .get(&event.revision)
            .ok_or("decoded transaction has no matching event revision")?;
        let batch = CommittedDatabaseTransaction::new(
            source,
            position,
            vec![RowMutation::Update { before, after }],
        )
        .map_err(string_error)?;
        adapter
            .apply_committed(&mut table, &batch, DatabaseTransactionLimits::default())
            .map_err(string_error)?;
    }
    let persisted = load_rows_v1(&mut client)?;
    let rebuilt = build_table(namespace, &schema, config.partitions, exact.clone())?;
    let exact_rows_match = persisted == exact;
    let summary_matches_rebuild =
        table.summary().map_err(string_error)? == rebuilt.summary().map_err(string_error)?;
    if !lsn_strictly_monotonic
        || decoded_account_updates != committed_transactions
        || !exact_rows_match
        || !summary_matches_rebuild
    {
        return Err(format!("correctness gate failed for {clients} clients"));
    }

    Ok(ConcurrencyResult {
        clients,
        committed_transactions,
        elapsed_microseconds: elapsed.as_micros(),
        transactions_per_second: committed_transactions as f64 / elapsed.as_secs_f64(),
        decoded_transactions: decoded.len(),
        decoded_account_updates,
        lsn_strictly_monotonic,
        exact_rows_match,
        summary_matches_rebuild,
    })
}

fn run_backpressure(
    client: &mut Client,
    total_events: usize,
    burst_factor: usize,
) -> Result<BackpressureResult, String> {
    let batch_size = (total_events / burst_factor).max(1);
    let mut cursor = 0_i64;
    let mut remaining = total_events;
    let mut previous_remaining = remaining;
    let mut batches = 0;
    let mut revisions_contiguous = true;
    let started = Instant::now();
    loop {
        let limit = i64::try_from(batch_size).map_err(|_| "batch size does not fit bigint")?;
        let revisions = client
            .query(
                "SELECT revision FROM algesum_c3_events WHERE revision > $1 ORDER BY revision LIMIT $2",
                &[&cursor, &limit],
            )
            .map_err(string_error)?;
        if revisions.is_empty() {
            break;
        }
        for row in revisions {
            let revision: i64 = row.get(0);
            revisions_contiguous &= revision == cursor + 1;
            cursor = revision;
            remaining -= 1;
        }
        batches += 1;
        if remaining > previous_remaining {
            return Err("backlog increased while producer was stopped".into());
        }
        previous_remaining = remaining;
    }
    if remaining != 0 || !revisions_contiguous {
        return Err("backpressure drain did not consume every contiguous event".into());
    }
    Ok(BackpressureResult {
        burst_factor,
        batch_size,
        initial_backlog: total_events,
        maximum_backlog: total_events,
        final_backlog: remaining,
        batches,
        drain_microseconds: started.elapsed().as_micros(),
        backlog_monotonic: true,
        revisions_contiguous,
    })
}

fn run_migration(client: &mut Client, partitions: usize) -> Result<MigrationResult, String> {
    let old_rows = load_rows_v1(client)?;
    client
        .batch_execute(
            "ALTER TABLE algesum_c3_accounts
             ADD COLUMN status text NOT NULL DEFAULT 'active'",
        )
        .map_err(string_error)?;
    let compatible_add_column = client
        .query_one(
            "SELECT count(*) FROM algesum_c3_accounts WHERE status = 'active'",
            &[],
        )
        .map_err(string_error)?
        .get::<_, i64>(0)
        == i64::try_from(old_rows.len()).map_err(|_| "row count does not fit bigint")?;
    let old_projection_stable = load_rows_v1(client)? == old_rows;
    let schema = account_schema_v2()?;
    let migrated = load_rows_v2(client)?;
    let first = build_table(namespace(), &schema, partitions, migrated.clone())?;
    let second = build_table(namespace(), &schema, partitions, migrated)?;
    let migrated_summary_deterministic =
        first.summary().map_err(string_error)? == second.summary().map_err(string_error)?;

    let mut transaction = client.transaction().map_err(string_error)?;
    transaction
        .batch_execute(
            "ALTER TABLE algesum_c3_accounts
             ALTER COLUMN balance TYPE text USING balance::text",
        )
        .map_err(string_error)?;
    let incompatible_rows = transaction
        .query(
            "SELECT id, version, balance, label FROM algesum_c3_accounts ORDER BY id",
            &[],
        )
        .map_err(string_error)?;
    let incompatible_change_detected = incompatible_rows
        .first()
        .is_some_and(|row| row.try_get::<_, i64>(2).is_err());
    transaction.rollback().map_err(string_error)?;
    let balance_type: String = client
        .query_one(
            "SELECT data_type FROM information_schema.columns
             WHERE table_schema = 'public' AND table_name = 'algesum_c3_accounts'
               AND column_name = 'balance'",
            &[],
        )
        .map_err(string_error)?
        .get(0);
    let rollback_preserved_schema = balance_type == "bigint";
    if !(compatible_add_column
        && old_projection_stable
        && migrated_summary_deterministic
        && incompatible_change_detected
        && rollback_preserved_schema)
    {
        return Err("schema migration gate failed".into());
    }
    Ok(MigrationResult {
        compatible_add_column,
        old_projection_stable,
        migrated_summary_deterministic,
        incompatible_change_detected,
        rollback_preserved_schema,
    })
}

fn persist_restart_checkpoint(
    client: &mut Client,
    partitions: usize,
) -> Result<RestartCheckpoint, String> {
    let rows = load_rows_v2(client)?;
    let schema = account_schema_v2()?;
    let table = build_table(namespace(), &schema, partitions, rows)?;
    let summary = table.summary().map_err(string_error)?;
    let evaluation = summary.evaluation().to_canonical().as_ref().to_vec();
    let nonzero = summary.nonzero_product().to_canonical().as_ref().to_vec();
    let aggregate = client
        .query_one(
            "SELECT count(*)::bigint, coalesce(sum(balance), 0)::bigint FROM algesum_c3_accounts",
            &[],
        )
        .map_err(string_error)?;
    let rows: i64 = aggregate.get(0);
    let balance_sum: i64 = aggregate.get(1);
    let revision: i64 = client
        .query_one(
            "SELECT revision FROM algesum_c3_metadata WHERE singleton",
            &[],
        )
        .map_err(string_error)?
        .get(0);
    client
        .execute(
            "INSERT INTO algesum_c3_checkpoint
             (singleton, revision, rows, balance_sum, schema_version, partitions,
              evaluation, nonzero_product, zero_factor_count, summary_row_count,
              summary_partition_count)
             VALUES (true, $1, $2, $3, 2, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (singleton) DO UPDATE SET
               revision = excluded.revision, rows = excluded.rows,
               balance_sum = excluded.balance_sum, schema_version = excluded.schema_version,
               partitions = excluded.partitions, evaluation = excluded.evaluation,
               nonzero_product = excluded.nonzero_product,
               zero_factor_count = excluded.zero_factor_count,
               summary_row_count = excluded.summary_row_count,
               summary_partition_count = excluded.summary_partition_count",
            &[
                &revision,
                &rows,
                &balance_sum,
                &i64::try_from(partitions).map_err(|_| "partitions do not fit bigint")?,
                &evaluation,
                &nonzero,
                &i64::try_from(summary.zero_factor_count()).map_err(|_| "zero count too large")?,
                &i64::try_from(summary.row_count()).map_err(|_| "row count too large")?,
                &i64::try_from(summary.partition_count())
                    .map_err(|_| "partition count too large")?,
            ],
        )
        .map_err(string_error)?;
    Ok(RestartCheckpoint {
        revision,
        rows,
        balance_sum,
        schema_version: 2,
        summary_match_at_prepare: true,
    })
}

fn run_verify_restart(config: &Config) -> Result<RestartReport, String> {
    let mut client = connect(&config.database_url)?;
    let postgres_version: String = client
        .query_one("SHOW server_version", &[])
        .map_err(string_error)?
        .get(0);
    let recovery_state: bool = client
        .query_one("SELECT pg_is_in_recovery()", &[])
        .map_err(string_error)?
        .get(0);
    let checkpoint = client
        .query_one(
            "SELECT revision, rows, balance_sum, schema_version, partitions,
                    evaluation, nonzero_product, zero_factor_count,
                    summary_row_count, summary_partition_count
             FROM algesum_c3_checkpoint WHERE singleton",
            &[],
        )
        .map_err(string_error)?;
    let revision: i64 = client
        .query_one(
            "SELECT revision FROM algesum_c3_metadata WHERE singleton",
            &[],
        )
        .map_err(string_error)?
        .get(0);
    let aggregate = client
        .query_one(
            "SELECT count(*)::bigint, coalesce(sum(balance), 0)::bigint FROM algesum_c3_accounts",
            &[],
        )
        .map_err(string_error)?;
    let rows: i64 = aggregate.get(0);
    let balance_sum: i64 = aggregate.get(1);
    let schema_version: i32 = checkpoint.get(3);
    let checkpoint_fields_match = revision == checkpoint.get::<_, i64>(0)
        && rows == checkpoint.get::<_, i64>(1)
        && balance_sum == checkpoint.get::<_, i64>(2)
        && schema_version == 2;
    let partitions =
        usize::try_from(checkpoint.get::<_, i64>(4)).map_err(|error| error.to_string())?;
    let schema = account_schema_v2()?;
    let exact = load_rows_v2(&mut client)?;
    let first = build_table(namespace(), &schema, partitions, exact.clone())?;
    let second = build_table(namespace(), &schema, partitions, exact)?;
    let summary = first.summary().map_err(string_error)?;
    let summary_bytes_match = summary.evaluation().to_canonical().as_ref()
        == checkpoint.get::<_, Vec<u8>>(5).as_slice()
        && summary.nonzero_product().to_canonical().as_ref()
            == checkpoint.get::<_, Vec<u8>>(6).as_slice()
        && i64::try_from(summary.zero_factor_count()).ok() == Some(checkpoint.get(7))
        && i64::try_from(summary.row_count()).ok() == Some(checkpoint.get(8))
        && i64::try_from(summary.partition_count()).ok() == Some(checkpoint.get(9));
    let summary_matches_fresh_rebuild = summary == second.summary().map_err(string_error)?;
    if !checkpoint_fields_match || !summary_bytes_match || !summary_matches_fresh_rebuild {
        return Err("restart verification gate failed".into());
    }
    Ok(RestartReport {
        schema: "algesum-c3-postgresql-systems-restart-v1",
        postgres_version,
        recovery_state: if recovery_state { "replica" } else { "primary" }.into(),
        revision,
        rows,
        balance_sum,
        schema_version,
        checkpoint_fields_match,
        summary_bytes_match,
        summary_matches_fresh_rebuild,
    })
}

fn initialize(client: &mut Client, rows: usize) -> Result<(), String> {
    drop_slot_if_present(client)?;
    client
        .batch_execute(
            "DROP TABLE IF EXISTS algesum_c3_checkpoint;
             DROP TABLE IF EXISTS algesum_c3_events;
             DROP TABLE IF EXISTS algesum_c3_accounts;
             DROP TABLE IF EXISTS algesum_c3_metadata;
             CREATE TABLE algesum_c3_metadata (
               singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
               revision bigint NOT NULL);
             INSERT INTO algesum_c3_metadata VALUES (true, 0);
             CREATE TABLE algesum_c3_accounts (
               id bigint PRIMARY KEY, version bigint NOT NULL,
               balance bigint NOT NULL, label text NOT NULL);
             ALTER TABLE algesum_c3_accounts REPLICA IDENTITY FULL;
             CREATE TABLE algesum_c3_events (
               revision bigint PRIMARY KEY, client_index integer NOT NULL,
               account_id bigint NOT NULL, before_version bigint NOT NULL,
               before_balance bigint NOT NULL, after_version bigint NOT NULL,
               after_balance bigint NOT NULL,
               committed_at timestamptz NOT NULL DEFAULT clock_timestamp());
             CREATE TABLE algesum_c3_checkpoint (
               singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
               revision bigint NOT NULL, rows bigint NOT NULL,
               balance_sum bigint NOT NULL, schema_version integer NOT NULL,
               partitions bigint NOT NULL, evaluation bytea NOT NULL,
               nonzero_product bytea NOT NULL, zero_factor_count bigint NOT NULL,
               summary_row_count bigint NOT NULL, summary_partition_count bigint NOT NULL);",
        )
        .map_err(string_error)?;
    reset_data(client, rows)
}

fn reset_data(client: &mut Client, rows: usize) -> Result<(), String> {
    client
        .batch_execute(
            "TRUNCATE algesum_c3_events, algesum_c3_accounts;
             UPDATE algesum_c3_metadata SET revision = 0;",
        )
        .map_err(string_error)?;
    let rows = i64::try_from(rows).map_err(|_| "rows do not fit bigint")?;
    client
        .execute(
            "INSERT INTO algesum_c3_accounts(id, version, balance, label)
             SELECT id, 1, id * 10, 'account-' || id::text
             FROM generate_series(0::bigint, $1::bigint - 1) generated(id)",
            &[&rows],
        )
        .map_err(string_error)?;
    Ok(())
}

fn update_one(client: &mut Client, client_index: usize, account_id: i64) -> Result<(), String> {
    let mut transaction = client.transaction().map_err(string_error)?;
    let before = transaction
        .query_one(
            "SELECT version, balance FROM algesum_c3_accounts WHERE id = $1 FOR UPDATE",
            &[&account_id],
        )
        .map_err(string_error)?;
    let before_version: i64 = before.get(0);
    let before_balance: i64 = before.get(1);
    let after = transaction
        .query_one(
            "UPDATE algesum_c3_accounts SET version = version + 1, balance = balance + 1
             WHERE id = $1 RETURNING version, balance",
            &[&account_id],
        )
        .map_err(string_error)?;
    let after_version: i64 = after.get(0);
    let after_balance: i64 = after.get(1);
    let revision: i64 = transaction
        .query_one(
            "UPDATE algesum_c3_metadata SET revision = revision + 1 RETURNING revision",
            &[],
        )
        .map_err(string_error)?
        .get(0);
    transaction
        .execute(
            "INSERT INTO algesum_c3_events
             (revision, client_index, account_id, before_version, before_balance,
              after_version, after_balance) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[
                &revision,
                &i32::try_from(client_index).map_err(|_| "client index does not fit integer")?,
                &account_id,
                &before_version,
                &before_balance,
                &after_version,
                &after_balance,
            ],
        )
        .map_err(string_error)?;
    transaction.commit().map_err(string_error)
}

fn recreate_slot(client: &mut Client) -> Result<(), String> {
    drop_slot_if_present(client)?;
    client
        .query_one(
            "SELECT slot_name FROM pg_create_logical_replication_slot($1, 'test_decoding')",
            &[&SLOT],
        )
        .map_err(string_error)?;
    Ok(())
}

fn drop_slot_if_present(client: &mut Client) -> Result<(), String> {
    client
        .execute(
            "SELECT pg_drop_replication_slot(slot_name) FROM pg_replication_slots WHERE slot_name = $1",
            &[&SLOT],
        )
        .map_err(string_error)?;
    Ok(())
}

fn decode_slot(client: &mut Client) -> Result<Vec<DecodedTransaction>, String> {
    let rows = client
        .query(
            "SELECT lsn::text, data FROM pg_logical_slot_get_changes($1, NULL, NULL)",
            &[&SLOT],
        )
        .map_err(string_error)?;
    let mut transactions = Vec::new();
    let mut revision = None;
    let mut account_updates = 0;
    for row in rows {
        let lsn: String = row.get(0);
        let data: String = row.get(1);
        if data.starts_with("BEGIN ") {
            revision = None;
            account_updates = 0;
        } else if data.starts_with("table public.algesum_c3_accounts: UPDATE:") {
            account_updates += 1;
        } else if data.starts_with("table public.algesum_c3_events: INSERT:") {
            revision = extract_tagged_u64(&data, "revision[bigint]:");
        } else if data.starts_with("COMMIT ") {
            transactions.push(DecodedTransaction {
                revision: revision.ok_or("decoded transaction is missing event revision")?,
                commit_lsn: parse_lsn(&lsn)?,
                account_updates,
            });
        }
    }
    Ok(transactions)
}

fn extract_tagged_u64(line: &str, tag: &str) -> Option<u64> {
    let tail = line.split_once(tag)?.1;
    tail.split_whitespace().next()?.parse().ok()
}

fn parse_lsn(lsn: &str) -> Result<u64, String> {
    let (high, low) = lsn
        .split_once('/')
        .ok_or_else(|| format!("invalid PostgreSQL LSN {lsn:?}"))?;
    let high = u64::from_str_radix(high, 16).map_err(|error| error.to_string())?;
    let low = u64::from_str_radix(low, 16).map_err(|error| error.to_string())?;
    Ok((high << 32) | low)
}

fn load_events(client: &mut Client) -> Result<Vec<Event>, String> {
    client
        .query(
            "SELECT revision, account_id, before_version, before_balance,
                    after_version, after_balance
             FROM algesum_c3_events ORDER BY revision",
            &[],
        )
        .map_err(string_error)?
        .into_iter()
        .map(|row| {
            Ok(Event {
                revision: u64::try_from(row.get::<_, i64>(0)).map_err(string_error)?,
                account_id: u64::try_from(row.get::<_, i64>(1)).map_err(string_error)?,
                before_version: u64::try_from(row.get::<_, i64>(2)).map_err(string_error)?,
                before_balance: row.get(3),
                after_version: u64::try_from(row.get::<_, i64>(4)).map_err(string_error)?,
                after_balance: row.get(5),
            })
        })
        .collect()
}

fn account_schema_v1() -> Result<DatabaseSchema, String> {
    DatabaseSchema::new(
        1,
        vec![
            DatabaseColumn::new("id", DatabaseColumnType::U64, false),
            DatabaseColumn::new("balance", DatabaseColumnType::I64, false),
            DatabaseColumn::new("label", DatabaseColumnType::Text, false),
        ],
        vec![0],
    )
    .map_err(string_error)
}

fn account_schema_v2() -> Result<DatabaseSchema, String> {
    DatabaseSchema::new(
        2,
        vec![
            DatabaseColumn::new("id", DatabaseColumnType::U64, false),
            DatabaseColumn::new("balance", DatabaseColumnType::I64, false),
            DatabaseColumn::new("label", DatabaseColumnType::Text, false),
            DatabaseColumn::new("status", DatabaseColumnType::Text, false),
        ],
        vec![0],
    )
    .map_err(string_error)
}

fn initial_rows(rows: usize) -> Vec<DatabaseRow> {
    (0..rows)
        .map(|id| account_row(id as u64, 1, id as i64 * 10))
        .collect()
}

fn account_row(id: u64, version: u64, balance: i64) -> DatabaseRow {
    DatabaseRow::new(
        version,
        vec![
            DatabaseValue::U64(id),
            DatabaseValue::I64(balance),
            DatabaseValue::Text(format!("account-{id}")),
        ],
    )
}

fn load_rows_v1(client: &mut Client) -> Result<Vec<DatabaseRow>, String> {
    client
        .query(
            "SELECT id, version, balance, label FROM algesum_c3_accounts ORDER BY id",
            &[],
        )
        .map_err(string_error)?
        .into_iter()
        .map(postgres_row_v1)
        .collect()
}

fn postgres_row_v1(row: postgres::Row) -> Result<DatabaseRow, String> {
    let id: i64 = row.try_get(0).map_err(string_error)?;
    let version: i64 = row.try_get(1).map_err(string_error)?;
    let balance: i64 = row.try_get(2).map_err(string_error)?;
    let label: String = row.try_get(3).map_err(string_error)?;
    Ok(DatabaseRow::new(
        u64::try_from(version).map_err(string_error)?,
        vec![
            DatabaseValue::U64(u64::try_from(id).map_err(string_error)?),
            DatabaseValue::I64(balance),
            DatabaseValue::Text(label),
        ],
    ))
}

fn load_rows_v2(client: &mut Client) -> Result<Vec<DatabaseRow>, String> {
    client
        .query(
            "SELECT id, version, balance, label, status FROM algesum_c3_accounts ORDER BY id",
            &[],
        )
        .map_err(string_error)?
        .into_iter()
        .map(|row| {
            let mut base = postgres_row_v1(row.clone())?;
            let status: String = row.try_get(4).map_err(string_error)?;
            let mut values = base.values().to_vec();
            values.push(DatabaseValue::Text(status));
            base = DatabaseRow::new(base.version(), values);
            Ok(base)
        })
        .collect()
}

fn namespace() -> ApplicationNamespace {
    ApplicationNamespace::derive(b"algesum-postgresql-c3-systems-v1")
}

fn source() -> DatabaseSourceId {
    DatabaseSourceId::new([0x53; 32])
}

fn build_table(
    namespace: ApplicationNamespace,
    schema: &DatabaseSchema,
    partitions: usize,
    rows: Vec<DatabaseRow>,
) -> Result<Table, String> {
    Table::from_rows(
        namespace,
        schema.clone(),
        partitions,
        BinaryPolynomialEncoder::new(0x414c_4745_5355_4d03),
        Gf2_128V1::ONE,
        rows,
    )
    .map_err(string_error)
}

fn connect(url: &str) -> Result<Client, String> {
    Client::connect(url, NoTls).map_err(string_error)
}

fn string_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn parse_config() -> Result<Config, String> {
    let mut database_url = env::var("ALGESUM_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@127.0.0.1:55432/postgres".into());
    let mut rows = 8_192;
    let mut partitions = 256;
    let mut transactions_per_client = 32;
    let mut clients = vec![1, 2, 8, 16, 32];
    let mut phase = Phase::Prepare;
    let mut output = PathBuf::from("/tmp/algesum-c3-postgresql-systems-prepare-v1.json");
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--database-url" => {
                database_url = args.next().ok_or("--database-url requires a value")?
            }
            "--rows" => rows = parse_usize(args.next(), "--rows")?,
            "--partitions" => partitions = parse_usize(args.next(), "--partitions")?,
            "--transactions-per-client" => {
                transactions_per_client = parse_usize(args.next(), "--transactions-per-client")?
            }
            "--clients" => {
                clients = args
                    .next()
                    .ok_or("--clients requires comma-separated values")?
                    .split(',')
                    .map(|value| value.parse::<usize>().map_err(string_error))
                    .collect::<Result<Vec<_>, _>>()?;
            }
            "--phase" => {
                phase = match args.next().as_deref() {
                    Some("prepare") => Phase::Prepare,
                    Some("verify-restart") => Phase::VerifyRestart,
                    Some(value) => return Err(format!("unknown phase {value:?}")),
                    None => return Err("--phase requires prepare or verify-restart".into()),
                }
            }
            "--output" => output = PathBuf::from(args.next().ok_or("--output requires a path")?),
            _ => return Err(format!("unknown argument {argument:?}")),
        }
    }
    if clients.is_empty() || clients.contains(&0) {
        return Err("--clients must contain nonzero values".into());
    }
    Ok(Config {
        database_url,
        rows,
        partitions,
        transactions_per_client,
        clients,
        phase,
        output,
    })
}

fn parse_usize(value: Option<String>, flag: &str) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("{flag} requires an integer"))?
        .parse()
        .map_err(string_error)
}

#[cfg(test)]
mod tests {
    use super::{extract_tagged_u64, parse_lsn};

    #[test]
    fn parses_postgresql_lsn_as_ordered_u64() {
        assert_eq!(parse_lsn("0/1566880").unwrap(), 0x0156_6880);
        assert!(parse_lsn("invalid").is_err());
    }

    #[test]
    fn extracts_revision_from_test_decoding_record() {
        let line = "table public.algesum_c3_events: INSERT: revision[bigint]:17 label[text]:'x'";
        assert_eq!(extract_tagged_u64(line, "revision[bigint]:"), Some(17));
    }
}
