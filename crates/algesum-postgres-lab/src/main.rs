//! PostgreSQL-backed correctness and bulk-scaling experiment.

use std::{env, fs::File, path::PathBuf, process::ExitCode, time::Instant};

use algesum::{
    ApplicationNamespace, BinaryPolynomialEncoder, CommittedDatabaseTransaction,
    DatabaseApplyPolicy, DatabaseChangeStreamAdapter, DatabaseColumn, DatabaseColumnType,
    DatabaseRow, DatabaseSchema, DatabaseSourceId, DatabaseTransactionLimits, DatabaseValue,
    PartitionedDatabase, RowMutation,
};
use microfield::{Field, Gf2_128V1};
use postgres::{Client, NoTls};
use serde::Serialize;

type Table = PartitionedDatabase<Gf2_128V1, BinaryPolynomialEncoder>;

#[derive(Debug)]
struct Config {
    database_url: String,
    rows: usize,
    partitions: usize,
    repetitions: usize,
    batches: Vec<usize>,
    incremental_ceiling: usize,
    partition_density_numerator: usize,
    partition_density_denominator: usize,
    output: PathBuf,
}

#[derive(Debug, Serialize)]
struct Report {
    schema: &'static str,
    rows: usize,
    partitions: usize,
    repetitions: usize,
    incremental_ceiling: usize,
    partition_density_numerator: usize,
    partition_density_denominator: usize,
    cells: Vec<Cell>,
}

#[derive(Debug, Serialize)]
struct Cell {
    batch_size: usize,
    samples: Vec<Sample>,
}

#[derive(Debug, Serialize)]
struct Sample {
    repetition: usize,
    postgres_microseconds: u128,
    algesum_apply_microseconds: u128,
    algesum_apply_path: String,
    algesum_rebuild_microseconds: u128,
    exact_match: bool,
    summary_match: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("algesum-postgres-lab: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let config = parse_config()?;
    if config.rows == 0 || config.partitions == 0 || config.repetitions == 0 {
        return Err("rows, partitions and repetitions must be nonzero".into());
    }
    if config.batches.is_empty()
        || config
            .batches
            .iter()
            .any(|&batch| batch == 0 || batch > config.rows)
    {
        return Err("every batch size must be in 1..=rows".into());
    }

    let mut client = Client::connect(&config.database_url, NoTls)
        .map_err(|error| format!("cannot connect to PostgreSQL: {error}"))?;
    initialize(&mut client, config.rows)?;

    let schema = account_schema()?;
    let namespace = ApplicationNamespace::derive(b"algesum-postgresql-account-lab-v1");
    let source = DatabaseSourceId::new([0x50; 32]);
    let mut exact_rows = load_rows(&mut client)?;
    let mut table = build_table(namespace, &schema, config.partitions, exact_rows.clone())?;
    let mut adapter = DatabaseChangeStreamAdapter::new(namespace, schema.clone(), source);
    let mut cells = Vec::with_capacity(config.batches.len());

    for &batch_size in &config.batches {
        let mut samples = Vec::with_capacity(config.repetitions);
        for repetition in 0..config.repetitions {
            let ids = deterministic_ids(config.rows, batch_size, repetition);
            let postgres_started = Instant::now();
            let (position, mutations) = update_postgres(&mut client, &ids)?;
            let postgres_microseconds = postgres_started.elapsed().as_micros();

            for mutation in &mutations {
                if let RowMutation::Update { after, .. } = mutation {
                    let id = row_id(after)?;
                    exact_rows[id] = after.clone();
                }
            }

            let batch = CommittedDatabaseTransaction::new(source, position, mutations)
                .map_err(|error| error.to_string())?;
            let apply_started = Instant::now();
            let authoritative_rows = exact_rows.clone();
            let apply_report = adapter
                .apply_committed_with_policy(
                    &mut table,
                    &batch,
                    DatabaseTransactionLimits::default(),
                    DatabaseApplyPolicy::adaptive(
                        config.incremental_ceiling,
                        config.partition_density_numerator,
                        config.partition_density_denominator,
                    )
                    .map_err(|error| error.to_string())?,
                    || authoritative_rows,
                )
                .map_err(|error| error.to_string())?;
            let algesum_apply_microseconds = apply_started.elapsed().as_micros();

            let rebuild_started = Instant::now();
            let rebuilt = build_table(namespace, &schema, config.partitions, exact_rows.clone())?;
            let algesum_rebuild_microseconds = rebuild_started.elapsed().as_micros();
            let persisted = load_rows(&mut client)?;
            let exact_match = persisted == exact_rows;
            let summary_match = table.summary().map_err(|error| error.to_string())?
                == rebuilt.summary().map_err(|error| error.to_string())?;
            if !exact_match || !summary_match {
                return Err(format!(
                    "verification failed for batch {batch_size}, repetition {repetition}"
                ));
            }
            samples.push(Sample {
                repetition,
                postgres_microseconds,
                algesum_apply_microseconds,
                algesum_apply_path: format!("{:?}", apply_report.path()),
                algesum_rebuild_microseconds,
                exact_match,
                summary_match,
            });
        }
        cells.push(Cell {
            batch_size,
            samples,
        });
    }

    let report = Report {
        schema: "algesum-postgresql-bulk-v1",
        rows: config.rows,
        partitions: config.partitions,
        repetitions: config.repetitions,
        incremental_ceiling: config.incremental_ceiling,
        partition_density_numerator: config.partition_density_numerator,
        partition_density_denominator: config.partition_density_denominator,
        cells,
    };
    let output = File::create(&config.output)
        .map_err(|error| format!("cannot create {}: {error}", config.output.display()))?;
    serde_json::to_writer_pretty(output, &report).map_err(|error| error.to_string())?;
    println!(
        "wrote verified PostgreSQL report to {}",
        config.output.display()
    );
    Ok(())
}

fn parse_config() -> Result<Config, String> {
    let mut database_url = env::var("ALGESUM_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@127.0.0.1:55432/postgres".into());
    let mut rows = 65_536;
    let mut partitions = 256;
    let mut repetitions = 3;
    let mut batches = vec![256, 4_096, 16_384, 32_768, 49_152, 65_536];
    let mut incremental_ceiling = 65_536;
    let mut partition_density_numerator = 2;
    let mut partition_density_denominator = 3;
    let mut output = PathBuf::from("/tmp/algesum-postgresql-bulk-v1.json");
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--database-url" => {
                database_url = args.next().ok_or("--database-url requires a value")?
            }
            "--rows" => rows = parse_usize(args.next(), "--rows")?,
            "--partitions" => partitions = parse_usize(args.next(), "--partitions")?,
            "--repetitions" => repetitions = parse_usize(args.next(), "--repetitions")?,
            "--batches" => {
                batches = args
                    .next()
                    .ok_or("--batches requires comma-separated integers")?
                    .split(',')
                    .map(|value| value.parse::<usize>().map_err(|error| error.to_string()))
                    .collect::<Result<Vec<_>, _>>()?;
            }
            "--incremental-ceiling" => {
                incremental_ceiling = parse_usize(args.next(), "--incremental-ceiling")?
            }
            "--partition-density" => {
                let value = args.next().ok_or("--partition-density requires N/D")?;
                let (numerator, denominator) = value
                    .split_once('/')
                    .ok_or("--partition-density requires N/D")?;
                partition_density_numerator = numerator
                    .parse::<usize>()
                    .map_err(|error| format!("invalid partition-density numerator: {error}"))?;
                partition_density_denominator = denominator
                    .parse::<usize>()
                    .map_err(|error| format!("invalid partition-density denominator: {error}"))?;
            }
            "--output" => output = PathBuf::from(args.next().ok_or("--output requires a path")?),
            _ => return Err(format!("unknown argument {argument:?}\n{}", usage())),
        }
    }
    Ok(Config {
        database_url,
        rows,
        partitions,
        repetitions,
        batches,
        incremental_ceiling,
        partition_density_numerator,
        partition_density_denominator,
        output,
    })
}

fn parse_usize(value: Option<String>, flag: &str) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("{flag} requires an integer"))?
        .parse::<usize>()
        .map_err(|error| format!("invalid {flag}: {error}"))
}

fn usage() -> &'static str {
    "usage: algesum-postgres-lab [--database-url URL] [--rows N] [--partitions N] [--repetitions N] [--batches N,N,...] [--incremental-ceiling N] [--partition-density N/D] [--output PATH]"
}

fn initialize(client: &mut Client, rows: usize) -> Result<(), String> {
    client
        .batch_execute(
            "DROP TABLE IF EXISTS algesum_accounts;
             DROP TABLE IF EXISTS algesum_metadata;
             CREATE TABLE algesum_metadata (
                 singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
                 revision bigint NOT NULL
             );
             INSERT INTO algesum_metadata (revision) VALUES (0);
             CREATE TABLE algesum_accounts (
                 id bigint PRIMARY KEY,
                 version bigint NOT NULL,
                 balance bigint NOT NULL,
                 label text NOT NULL
             );",
        )
        .map_err(|error| error.to_string())?;
    let rows = i64::try_from(rows).map_err(|_| "row count does not fit PostgreSQL bigint")?;
    client
        .execute(
            "INSERT INTO algesum_accounts (id, version, balance, label)
             SELECT id, 1, id * 10, 'account-' || id::text
             FROM generate_series(0::bigint, $1::bigint - 1) AS generated(id)",
            &[&rows],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn account_schema() -> Result<DatabaseSchema, String> {
    DatabaseSchema::new(
        1,
        vec![
            DatabaseColumn::new("id", DatabaseColumnType::U64, false),
            DatabaseColumn::new("balance", DatabaseColumnType::I64, false),
            DatabaseColumn::new("label", DatabaseColumnType::Text, false),
        ],
        vec![0],
    )
    .map_err(|error| error.to_string())
}

fn load_rows(client: &mut Client) -> Result<Vec<DatabaseRow>, String> {
    client
        .query(
            "SELECT id, version, balance, label FROM algesum_accounts ORDER BY id",
            &[],
        )
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(postgres_row)
        .collect()
}

fn postgres_row(row: postgres::Row) -> Result<DatabaseRow, String> {
    let id: i64 = row.get(0);
    let version: i64 = row.get(1);
    let balance: i64 = row.get(2);
    let label: String = row.get(3);
    Ok(DatabaseRow::new(
        u64::try_from(version).map_err(|_| "negative PostgreSQL row version")?,
        vec![
            DatabaseValue::U64(u64::try_from(id).map_err(|_| "negative account id")?),
            DatabaseValue::I64(balance),
            DatabaseValue::Text(label),
        ],
    ))
}

fn update_postgres(client: &mut Client, ids: &[i64]) -> Result<(u64, Vec<RowMutation>), String> {
    let mut transaction = client.transaction().map_err(|error| error.to_string())?;
    let ids = ids.to_vec();
    let before = transaction
        .query(
            "SELECT id, version, balance, label
             FROM algesum_accounts WHERE id = ANY($1::bigint[]) ORDER BY id FOR UPDATE",
            &[&ids],
        )
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(postgres_row)
        .collect::<Result<Vec<_>, _>>()?;
    let after = transaction
        .query(
            "UPDATE algesum_accounts
             SET version = version + 1, balance = balance + 1
             WHERE id = ANY($1::bigint[])
             RETURNING id, version, balance, label",
            &[&ids],
        )
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(postgres_row)
        .collect::<Result<Vec<_>, _>>()?;
    let revision: i64 = transaction
        .query_one(
            "UPDATE algesum_metadata SET revision = revision + 1 RETURNING revision",
            &[],
        )
        .map_err(|error| error.to_string())?
        .get(0);
    transaction.commit().map_err(|error| error.to_string())?;

    let mut after_by_id = after
        .into_iter()
        .map(|row| Ok((row_id(&row)?, row)))
        .collect::<Result<std::collections::BTreeMap<_, _>, String>>()?;
    let mutations = before
        .into_iter()
        .map(|before| {
            let id = row_id(&before)?;
            let after = after_by_id
                .remove(&id)
                .ok_or("missing UPDATE RETURNING row")?;
            Ok(RowMutation::Update { before, after })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if !after_by_id.is_empty() || mutations.len() != ids.len() {
        return Err("PostgreSQL update cardinality mismatch".into());
    }
    u64::try_from(revision)
        .map(|position| (position, mutations))
        .map_err(|_| "negative PostgreSQL revision".into())
}

fn deterministic_ids(rows: usize, batch: usize, repetition: usize) -> Vec<i64> {
    let start = repetition.wrapping_mul(1_000_003) % rows;
    (0..batch)
        .map(|offset| ((start + offset) % rows) as i64)
        .collect()
}

fn row_id(row: &DatabaseRow) -> Result<usize, String> {
    match row.values().first() {
        Some(DatabaseValue::U64(id)) => usize::try_from(*id).map_err(|error| error.to_string()),
        _ => Err("account row does not start with a u64 id".into()),
    }
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
        BinaryPolynomialEncoder::new(0x414c_4745_5355_4d02),
        Gf2_128V1::ONE,
        rows,
    )
    .map_err(|error| error.to_string())
}
