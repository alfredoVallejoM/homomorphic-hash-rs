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
    full_rebuild_density_numerator: usize,
    full_rebuild_density_denominator: usize,
    max_mutations: usize,
    max_transaction_bytes: usize,
    distribution: IdDistribution,
    hotspot_rows: Option<usize>,
    output: PathBuf,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum IdDistribution {
    Clustered,
    Strided,
    Hotspot,
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
    full_rebuild_density_numerator: usize,
    full_rebuild_density_denominator: usize,
    max_mutations: usize,
    max_transaction_bytes: usize,
    distribution: IdDistribution,
    hotspot_rows: Option<usize>,
    initialize_microseconds: u128,
    initial_load_microseconds: u128,
    initial_build_microseconds: u128,
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
    verification_microseconds: u128,
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
    if config.rows == 0
        || config.partitions == 0
        || config.repetitions == 0
        || config.max_mutations == 0
        || config.max_transaction_bytes == 0
    {
        return Err("rows, partitions, repetitions and transaction limits must be nonzero".into());
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
    let initialize_started = Instant::now();
    initialize(&mut client, config.rows)?;
    let initialize_microseconds = initialize_started.elapsed().as_micros();

    let schema = account_schema()?;
    let namespace = ApplicationNamespace::derive(b"algesum-postgresql-account-lab-v1");
    let source = DatabaseSourceId::new([0x50; 32]);
    let initial_load_started = Instant::now();
    let mut exact_rows = load_rows(&mut client)?;
    let initial_load_microseconds = initial_load_started.elapsed().as_micros();
    let initial_build_started = Instant::now();
    let mut table = build_table(namespace, &schema, config.partitions, exact_rows.clone())?;
    let initial_build_microseconds = initial_build_started.elapsed().as_micros();
    let mut adapter = DatabaseChangeStreamAdapter::new(namespace, schema.clone(), source);
    let transaction_limits = DatabaseTransactionLimits {
        max_mutations: config.max_mutations,
        max_transaction_bytes: config.max_transaction_bytes,
        ..DatabaseTransactionLimits::default()
    };
    let mut cells = Vec::with_capacity(config.batches.len());

    for &batch_size in &config.batches {
        let mut samples = Vec::with_capacity(config.repetitions);
        for repetition in 0..config.repetitions {
            let ids = deterministic_ids(
                config.rows,
                batch_size,
                repetition,
                config.distribution,
                config.hotspot_rows,
            )?;
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
            let apply_report = adapter
                .apply_committed_with_policy(
                    &mut table,
                    &batch,
                    transaction_limits,
                    DatabaseApplyPolicy::adaptive(
                        config.incremental_ceiling,
                        config.partition_density_numerator,
                        config.partition_density_denominator,
                    )
                    .and_then(|policy| {
                        policy.with_full_rebuild_fraction(
                            config.full_rebuild_density_numerator,
                            config.full_rebuild_density_denominator,
                        )
                    })
                    .map_err(|error| error.to_string())?,
                    || exact_rows.clone(),
                )
                .map_err(|error| error.to_string())?;
            let algesum_apply_microseconds = apply_started.elapsed().as_micros();

            let rebuild_started = Instant::now();
            let rebuilt = build_table(namespace, &schema, config.partitions, exact_rows.clone())?;
            let algesum_rebuild_microseconds = rebuild_started.elapsed().as_micros();
            let verification_started = Instant::now();
            let persisted = load_rows(&mut client)?;
            let exact_match = persisted == exact_rows;
            let summary_match = table.summary().map_err(|error| error.to_string())?
                == rebuilt.summary().map_err(|error| error.to_string())?;
            let verification_microseconds = verification_started.elapsed().as_micros();
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
                verification_microseconds,
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
        schema: "algesum-postgresql-scaling-v3",
        rows: config.rows,
        partitions: config.partitions,
        repetitions: config.repetitions,
        incremental_ceiling: config.incremental_ceiling,
        partition_density_numerator: config.partition_density_numerator,
        partition_density_denominator: config.partition_density_denominator,
        full_rebuild_density_numerator: config.full_rebuild_density_numerator,
        full_rebuild_density_denominator: config.full_rebuild_density_denominator,
        max_mutations: config.max_mutations,
        max_transaction_bytes: config.max_transaction_bytes,
        distribution: config.distribution,
        hotspot_rows: config.hotspot_rows,
        initialize_microseconds,
        initial_load_microseconds,
        initial_build_microseconds,
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
    let mut full_rebuild_density_numerator = 1;
    let mut full_rebuild_density_denominator = 1;
    let mut max_mutations = DatabaseTransactionLimits::default().max_mutations;
    let mut max_transaction_bytes = DatabaseTransactionLimits::default().max_transaction_bytes;
    let mut distribution = IdDistribution::Clustered;
    let mut hotspot_rows = None;
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
                (partition_density_numerator, partition_density_denominator) =
                    parse_fraction(args.next(), "--partition-density")?;
            }
            "--full-rebuild-density" => {
                (
                    full_rebuild_density_numerator,
                    full_rebuild_density_denominator,
                ) = parse_fraction(args.next(), "--full-rebuild-density")?;
            }
            "--max-mutations" => {
                max_mutations = parse_usize(args.next(), "--max-mutations")?;
            }
            "--max-transaction-bytes" => {
                max_transaction_bytes = parse_usize(args.next(), "--max-transaction-bytes")?;
            }
            "--distribution" => {
                distribution = match args.next().as_deref() {
                    Some("clustered") => IdDistribution::Clustered,
                    Some("strided") => IdDistribution::Strided,
                    Some("hotspot") => IdDistribution::Hotspot,
                    Some(value) => return Err(format!("unknown distribution {value:?}")),
                    None => {
                        return Err("--distribution requires clustered, strided or hotspot".into());
                    }
                }
            }
            "--hotspot-rows" => hotspot_rows = Some(parse_usize(args.next(), "--hotspot-rows")?),
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
        full_rebuild_density_numerator,
        full_rebuild_density_denominator,
        max_mutations,
        max_transaction_bytes,
        distribution,
        hotspot_rows,
        output,
    })
}

fn parse_fraction(value: Option<String>, flag: &str) -> Result<(usize, usize), String> {
    let value = value.ok_or_else(|| format!("{flag} requires N/D"))?;
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or_else(|| format!("{flag} requires N/D"))?;
    let numerator = numerator
        .parse::<usize>()
        .map_err(|error| format!("invalid {flag} numerator: {error}"))?;
    let denominator = denominator
        .parse::<usize>()
        .map_err(|error| format!("invalid {flag} denominator: {error}"))?;
    Ok((numerator, denominator))
}

fn parse_usize(value: Option<String>, flag: &str) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("{flag} requires an integer"))?
        .parse::<usize>()
        .map_err(|error| format!("invalid {flag}: {error}"))
}

fn usage() -> &'static str {
    "usage: algesum-postgres-lab [--database-url URL] [--rows N] [--partitions N] [--repetitions N] [--batches N,N,...] [--incremental-ceiling N] [--partition-density N/D] [--full-rebuild-density N/D] [--max-mutations N] [--max-transaction-bytes N] [--distribution clustered|strided|hotspot] [--hotspot-rows N] [--output PATH]"
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

fn deterministic_ids(
    rows: usize,
    batch: usize,
    repetition: usize,
    distribution: IdDistribution,
    hotspot_rows: Option<usize>,
) -> Result<Vec<i64>, String> {
    let universe = match distribution {
        IdDistribution::Hotspot => hotspot_rows.unwrap_or_else(|| (rows / 16).max(batch)),
        IdDistribution::Clustered | IdDistribution::Strided => rows,
    };
    if universe == 0 || universe > rows || batch > universe {
        return Err(format!(
            "distribution universe must satisfy batch <= universe <= rows; batch={batch}, universe={universe}, rows={rows}"
        ));
    }
    let window_start = match distribution {
        IdDistribution::Hotspot => repetition.wrapping_mul(1_000_003) % (rows - universe + 1),
        IdDistribution::Clustered | IdDistribution::Strided => 0,
    };
    let start = repetition.wrapping_mul(1_000_003) % universe;
    let stride = match distribution {
        IdDistribution::Clustered => 1,
        IdDistribution::Strided | IdDistribution::Hotspot => coprime_stride(universe),
    };
    (0..batch)
        .map(|offset| {
            let local = (start as u128 + offset as u128 * stride as u128) % universe as u128;
            i64::try_from(window_start as u128 + local)
                .map_err(|_| "selected PostgreSQL id does not fit bigint".to_string())
        })
        .collect()
}

fn coprime_stride(modulus: usize) -> usize {
    if modulus <= 2 {
        return 1;
    }
    let mut candidate = (modulus / 2) | 1;
    while gcd(candidate, modulus) != 1 {
        candidate += 2;
        if candidate >= modulus {
            candidate = 1;
        }
    }
    candidate
}

fn gcd(mut left: usize, mut right: usize) -> usize {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{IdDistribution, deterministic_ids};

    #[test]
    fn distributions_are_deterministic_unique_and_bounded() {
        for distribution in [
            IdDistribution::Clustered,
            IdDistribution::Strided,
            IdDistribution::Hotspot,
        ] {
            let hotspot = matches!(distribution, IdDistribution::Hotspot).then_some(128);
            let first = deterministic_ids(1_024, 64, 7, distribution, hotspot).unwrap();
            let second = deterministic_ids(1_024, 64, 7, distribution, hotspot).unwrap();
            assert_eq!(first, second);
            assert_eq!(first.iter().copied().collect::<BTreeSet<_>>().len(), 64);
            assert!(first.iter().all(|id| (0..1_024).contains(id)));
        }
    }

    #[test]
    fn hotspot_rejects_batches_larger_than_the_window() {
        let error =
            deterministic_ids(1_024, 129, 0, IdDistribution::Hotspot, Some(128)).unwrap_err();
        assert!(error.contains("batch <= universe <= rows"));
    }
}
