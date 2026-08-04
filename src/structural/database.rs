//! Canonical database rows, partitioned signatures and versioned transactions.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use microfield::{CanonicalEncoding, Field, Invert, StaticField};
use sha2::{Digest as _, Sha256};

use super::{
    ApplicationNamespace, MultisetSignature, SignatureContext, SignatureError, StructuralEncoder,
};

const ROW_MAGIC: &[u8; 4] = b"MFRW";
const ROW_SCHEMA: u16 = 1;
const ROW_HEADER_BYTES: usize = 56;
const TX_MAGIC: &[u8; 4] = b"MFTX";
const TX_SCHEMA: u16 = 1;
const TX_HEADER_BYTES: usize = 96;
const LOG_MAGIC: &[u8; 4] = b"MFTL";
const LOG_SCHEMA: u16 = 1;
const LOG_HEADER_BYTES: usize = 14;

/// Canonical logical type of one row column.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum DatabaseColumnType {
    /// Boolean encoded as exactly zero or one.
    Bool = 1,
    /// Signed little-endian 64-bit integer.
    I64 = 2,
    /// Unsigned little-endian 64-bit integer.
    U64 = 3,
    /// Length-framed arbitrary bytes.
    Bytes = 4,
    /// Length-framed UTF-8 text without collation normalization.
    Text = 5,
}

/// Frozen column name, type and nullability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseColumn {
    name: String,
    column_type: DatabaseColumnType,
    nullable: bool,
}

impl DatabaseColumn {
    /// Defines one schema column.
    #[must_use]
    pub fn new(name: impl Into<String>, column_type: DatabaseColumnType, nullable: bool) -> Self {
        Self {
            name: name.into(),
            column_type,
            nullable,
        }
    }

    /// Stable column name participating in schema identity.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Logical type.
    #[must_use]
    pub const fn column_type(&self) -> DatabaseColumnType {
        self.column_type
    }

    /// Whether `DatabaseValue::Null` is accepted.
    #[must_use]
    pub const fn nullable(&self) -> bool {
        self.nullable
    }
}

/// One canonical row value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatabaseValue {
    /// SQL-like null marker, only for nullable columns.
    Null,
    /// Boolean.
    Bool(bool),
    /// Signed integer.
    I64(i64),
    /// Unsigned integer.
    U64(u64),
    /// Uninterpreted bytes.
    Bytes(Vec<u8>),
    /// Exact UTF-8 scalar sequence; no implicit collation.
    Text(String),
}

/// Stable identity of a complete row schema and primary key definition.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DatabaseSchemaId([u8; 32]);

/// Stable digest of a canonical primary-key tuple.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DatabaseRowKey([u8; 32]);

/// Content-derived transaction identity.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct TransactionId([u8; 32]);

macro_rules! impl_id {
    ($type:ty, $label:literal) => {
        impl $type {
            /// Borrows the canonical bytes.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str($label)?;
                formatter.write_str("(")?;
                for byte in self.as_bytes() {
                    write!(formatter, "{byte:02x}")?;
                }
                formatter.write_str(")")
            }
        }
    };
}

impl_id!(DatabaseSchemaId, "DatabaseSchemaId");
impl_id!(DatabaseRowKey, "DatabaseRowKey");
impl_id!(TransactionId, "TransactionId");

/// Versioned schema with an explicit unique primary key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseSchema {
    schema_version: u32,
    columns: Vec<DatabaseColumn>,
    primary_key: Vec<usize>,
    schema_id: DatabaseSchemaId,
}

impl DatabaseSchema {
    /// Validates and freezes one schema.
    pub fn new(
        schema_version: u32,
        columns: Vec<DatabaseColumn>,
        primary_key: Vec<usize>,
    ) -> Result<Self, DatabaseError> {
        if schema_version == 0 || columns.is_empty() || primary_key.is_empty() {
            return Err(DatabaseError::InvalidSchema(
                "empty schema, version or primary key",
            ));
        }
        let mut names = BTreeSet::new();
        for column in &columns {
            if column.name.is_empty() || !names.insert(column.name.clone()) {
                return Err(DatabaseError::InvalidSchema(
                    "empty or duplicate column name",
                ));
            }
        }
        let mut keys = BTreeSet::new();
        for &index in &primary_key {
            if index >= columns.len() || !keys.insert(index) || columns[index].nullable {
                return Err(DatabaseError::InvalidSchema("invalid primary-key column"));
            }
        }
        let mut descriptor = Vec::new();
        descriptor.extend_from_slice(b"microfield-database-schema-v1\0");
        descriptor.extend_from_slice(&schema_version.to_le_bytes());
        descriptor.extend_from_slice(&(columns.len() as u64).to_le_bytes());
        for column in &columns {
            descriptor.extend_from_slice(&(column.name.len() as u64).to_le_bytes());
            descriptor.extend_from_slice(column.name.as_bytes());
            descriptor.push(column.column_type as u8);
            descriptor.push(u8::from(column.nullable));
        }
        descriptor.extend_from_slice(&(primary_key.len() as u64).to_le_bytes());
        for &index in &primary_key {
            descriptor.extend_from_slice(&(index as u64).to_le_bytes());
        }
        let mut hasher = Sha256::new();
        hasher.update(&descriptor);
        Ok(Self {
            schema_version,
            columns,
            primary_key,
            schema_id: DatabaseSchemaId(hasher.finalize().into()),
        })
    }

    /// Schema identity.
    #[must_use]
    pub const fn schema_id(&self) -> DatabaseSchemaId {
        self.schema_id
    }

    /// Application-controlled schema revision.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Ordered columns.
    #[must_use]
    pub fn columns(&self) -> &[DatabaseColumn] {
        &self.columns
    }

    /// Ordered column indexes forming the unique primary key.
    #[must_use]
    pub fn primary_key(&self) -> &[usize] {
        &self.primary_key
    }

    /// Validates and canonically frames one complete row.
    pub fn encode_row(&self, row: &DatabaseRow) -> Result<Vec<u8>, DatabaseError> {
        self.validate_row(row)?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(ROW_MAGIC);
        bytes.extend_from_slice(&ROW_SCHEMA.to_le_bytes());
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(self.schema_id.as_bytes());
        bytes.extend_from_slice(&row.version.to_le_bytes());
        bytes.extend_from_slice(&(row.values.len() as u64).to_le_bytes());
        for value in &row.values {
            encode_value(&mut bytes, value);
        }
        Ok(bytes)
    }

    /// Parses one row and revalidates all column semantics.
    pub fn decode_row(&self, bytes: &[u8]) -> Result<DatabaseRow, DatabaseError> {
        if bytes.len() < ROW_HEADER_BYTES || &bytes[..4] != ROW_MAGIC {
            return Err(DatabaseError::InvalidWire("row header"));
        }
        if u16::from_le_bytes([bytes[4], bytes[5]]) != ROW_SCHEMA || bytes[6..8] != [0, 0] {
            return Err(DatabaseError::InvalidWire("row schema or reserved"));
        }
        if &bytes[8..40] != self.schema_id.as_bytes() {
            return Err(DatabaseError::SchemaMismatch);
        }
        let version = u64::from_le_bytes(bytes[40..48].try_into().expect("row version range"));
        let count = read_usize(&bytes[48..56], "row value count")?;
        if count != self.columns.len() {
            return Err(DatabaseError::InvalidWire("row value count"));
        }
        let mut cursor = ROW_HEADER_BYTES;
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| DatabaseError::AllocationFailed)?;
        for _ in 0..count {
            values.push(decode_value(bytes, &mut cursor)?);
        }
        if cursor != bytes.len() {
            return Err(DatabaseError::InvalidWire("trailing row bytes"));
        }
        let row = DatabaseRow { version, values };
        self.validate_row(&row)?;
        Ok(row)
    }

    /// Derives the exact canonical primary-key identity.
    pub fn row_key(&self, row: &DatabaseRow) -> Result<DatabaseRowKey, DatabaseError> {
        self.validate_row(row)?;
        let mut hasher = Sha256::new();
        hasher.update(b"microfield-database-row-key-v1\0");
        hasher.update(self.schema_id.as_bytes());
        for &index in &self.primary_key {
            let mut encoded = Vec::new();
            encode_value(&mut encoded, &row.values[index]);
            hasher.update((encoded.len() as u64).to_le_bytes());
            hasher.update(encoded);
        }
        Ok(DatabaseRowKey(hasher.finalize().into()))
    }

    fn validate_row(&self, row: &DatabaseRow) -> Result<(), DatabaseError> {
        if row.version == 0 || row.values.len() != self.columns.len() {
            return Err(DatabaseError::InvalidRow("version or column count"));
        }
        for (column, value) in self.columns.iter().zip(&row.values) {
            let accepted = matches!(value, DatabaseValue::Null) && column.nullable
                || matches!(
                    (column.column_type, value),
                    (DatabaseColumnType::Bool, DatabaseValue::Bool(_))
                        | (DatabaseColumnType::I64, DatabaseValue::I64(_))
                        | (DatabaseColumnType::U64, DatabaseValue::U64(_))
                        | (DatabaseColumnType::Bytes, DatabaseValue::Bytes(_))
                        | (DatabaseColumnType::Text, DatabaseValue::Text(_))
                );
            if !accepted {
                return Err(DatabaseError::InvalidRow("column type or nullability"));
            }
        }
        if self
            .primary_key
            .iter()
            .any(|&index| matches!(row.values[index], DatabaseValue::Null))
        {
            return Err(DatabaseError::InvalidRow("null primary key"));
        }
        Ok(())
    }
}

/// Exact row image; version participates in its canonical bytes and signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseRow {
    version: u64,
    values: Vec<DatabaseValue>,
}

impl DatabaseRow {
    /// Creates an image. Schema validation occurs at the consuming boundary.
    #[must_use]
    pub const fn new(version: u64, values: Vec<DatabaseValue>) -> Self {
        Self { version, values }
    }

    /// Exact row version/LSN supplied by the application.
    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    /// Ordered values.
    #[must_use]
    pub fn values(&self) -> &[DatabaseValue] {
        &self.values
    }
}

/// Insert/delete/update with exact before/after images.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RowMutation {
    /// Requires an absent primary key.
    Insert(DatabaseRow),
    /// Requires byte-for-byte equality with the retained row.
    Delete(DatabaseRow),
    /// Requires the same primary key and a strictly increasing row version.
    Update {
        /// Expected current image.
        before: DatabaseRow,
        /// Replacement image.
        after: DatabaseRow,
    },
}

/// Defensive transaction/log ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DatabaseTransactionLimits {
    /// Maximum mutations in one transaction.
    pub max_mutations: usize,
    /// Maximum canonical bytes in one row image.
    pub max_row_bytes: usize,
    /// Maximum bytes in one transaction envelope.
    pub max_transaction_bytes: usize,
    /// Maximum transactions in one decoded log.
    pub max_log_entries: usize,
    /// Maximum complete log bytes.
    pub max_log_bytes: usize,
}

impl Default for DatabaseTransactionLimits {
    fn default() -> Self {
        Self {
            max_mutations: 100_000,
            max_row_bytes: 16 * 1024 * 1024,
            max_transaction_bytes: 64 * 1024 * 1024,
            max_log_entries: 1_000_000,
            max_log_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Typed database protocol failure.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DatabaseError {
    /// Invalid columns, key indexes or schema version.
    InvalidSchema(&'static str),
    /// Row type, nullability, version or width mismatch.
    InvalidRow(&'static str),
    /// Transaction belongs to another schema.
    SchemaMismatch,
    /// Transaction belongs to another application dataset.
    NamespaceMismatch,
    /// Expected revision differs from the current table revision.
    RevisionMismatch { expected: u64, actual: u64 },
    /// Source revision cannot advance exactly once.
    RevisionOverflow,
    /// Empty transaction or duplicate key mutation.
    InvalidTransaction(&'static str),
    /// Insert found an existing key or before image did not match.
    Conflict(&'static str),
    /// Envelope/parser failure.
    InvalidWire(&'static str),
    /// Transaction or log exceeded a defensive ceiling.
    LimitExceeded(&'static str),
    /// Underlying structural signature failed.
    Signature(SignatureError),
    /// Allocation failed before commit.
    AllocationFailed,
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSchema(reason) => write!(formatter, "invalid database schema: {reason}"),
            Self::InvalidRow(reason) => write!(formatter, "invalid database row: {reason}"),
            Self::SchemaMismatch => formatter.write_str("database schema mismatch"),
            Self::NamespaceMismatch => formatter.write_str("database namespace mismatch"),
            Self::RevisionMismatch { expected, actual } => write!(
                formatter,
                "transaction expects revision {expected}, current revision is {actual}"
            ),
            Self::RevisionOverflow => formatter.write_str("database revision overflow"),
            Self::InvalidTransaction(reason) => write!(formatter, "invalid transaction: {reason}"),
            Self::Conflict(reason) => write!(formatter, "database transaction conflict: {reason}"),
            Self::InvalidWire(reason) => write!(formatter, "invalid database wire: {reason}"),
            Self::LimitExceeded(limit) => {
                write!(formatter, "database protocol exceeds {limit} limit")
            }
            Self::Signature(error) => error.fmt(formatter),
            Self::AllocationFailed => formatter.write_str("database protocol allocation failed"),
        }
    }
}

impl std::error::Error for DatabaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Signature(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SignatureError> for DatabaseError {
    fn from(error: SignatureError) -> Self {
        Self::Signature(error)
    }
}

/// Immutable versioned transaction envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionDelta {
    namespace: ApplicationNamespace,
    schema_id: DatabaseSchemaId,
    source_revision: u64,
    target_revision: u64,
    mutations: Vec<RowMutation>,
    transaction_id: TransactionId,
}

impl TransactionDelta {
    /// Validates rows and creates a single revision transition.
    pub fn new(
        namespace: ApplicationNamespace,
        schema: &DatabaseSchema,
        source_revision: u64,
        mutations: Vec<RowMutation>,
    ) -> Result<Self, DatabaseError> {
        if mutations.is_empty() {
            return Err(DatabaseError::InvalidTransaction("empty mutation list"));
        }
        let target_revision = source_revision
            .checked_add(1)
            .ok_or(DatabaseError::RevisionOverflow)?;
        validate_mutations(schema, &mutations)?;
        let mut transaction = Self {
            namespace,
            schema_id: schema.schema_id,
            source_revision,
            target_revision,
            mutations,
            transaction_id: TransactionId([0; 32]),
        };
        transaction.transaction_id = derive_transaction_id(&transaction.to_canonical_bytes());
        Ok(transaction)
    }

    /// Application dataset identity.
    #[must_use]
    pub const fn namespace(&self) -> ApplicationNamespace {
        self.namespace
    }

    /// Row schema identity.
    #[must_use]
    pub const fn schema_id(&self) -> DatabaseSchemaId {
        self.schema_id
    }

    /// Required current revision.
    #[must_use]
    pub const fn source_revision(&self) -> u64 {
        self.source_revision
    }

    /// Published revision on success.
    #[must_use]
    pub const fn target_revision(&self) -> u64 {
        self.target_revision
    }

    /// Ordered exact mutations.
    #[must_use]
    pub fn mutations(&self) -> &[RowMutation] {
        &self.mutations
    }

    /// Content-derived replay identity.
    #[must_use]
    pub const fn transaction_id(&self) -> TransactionId {
        self.transaction_id
    }

    /// Deterministic `MFTX` schema 1 representation.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(TX_MAGIC);
        bytes.extend_from_slice(&TX_SCHEMA.to_le_bytes());
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(self.namespace.as_bytes());
        bytes.extend_from_slice(self.schema_id.as_bytes());
        bytes.extend_from_slice(&self.source_revision.to_le_bytes());
        bytes.extend_from_slice(&self.target_revision.to_le_bytes());
        bytes.extend_from_slice(&(self.mutations.len() as u64).to_le_bytes());
        for mutation in &self.mutations {
            match mutation {
                RowMutation::Insert(row) => {
                    bytes.push(1);
                    encode_framed(&mut bytes, &row_wire_unchecked(self.schema_id, row));
                }
                RowMutation::Delete(row) => {
                    bytes.push(2);
                    encode_framed(&mut bytes, &row_wire_unchecked(self.schema_id, row));
                }
                RowMutation::Update { before, after } => {
                    bytes.push(3);
                    encode_framed(&mut bytes, &row_wire_unchecked(self.schema_id, before));
                    encode_framed(&mut bytes, &row_wire_unchecked(self.schema_id, after));
                }
            }
        }
        bytes
    }

    /// Parses and validates a canonical transaction under explicit limits.
    pub fn from_canonical_bytes(
        namespace: ApplicationNamespace,
        schema: &DatabaseSchema,
        bytes: &[u8],
        limits: DatabaseTransactionLimits,
    ) -> Result<Self, DatabaseError> {
        if bytes.len() > limits.max_transaction_bytes {
            return Err(DatabaseError::LimitExceeded("transaction bytes"));
        }
        if bytes.len() < TX_HEADER_BYTES || &bytes[..4] != TX_MAGIC {
            return Err(DatabaseError::InvalidWire("transaction header"));
        }
        if u16::from_le_bytes([bytes[4], bytes[5]]) != TX_SCHEMA || bytes[6..8] != [0, 0] {
            return Err(DatabaseError::InvalidWire("transaction schema or reserved"));
        }
        if &bytes[8..40] != namespace.as_bytes() {
            return Err(DatabaseError::NamespaceMismatch);
        }
        if &bytes[40..72] != schema.schema_id.as_bytes() {
            return Err(DatabaseError::SchemaMismatch);
        }
        let source = u64::from_le_bytes(bytes[72..80].try_into().expect("source revision range"));
        let target = u64::from_le_bytes(bytes[80..88].try_into().expect("target revision range"));
        if source.checked_add(1) != Some(target) {
            return Err(DatabaseError::InvalidWire(
                "transaction revision transition",
            ));
        }
        let count = read_usize(&bytes[88..96], "mutation count")?;
        if count == 0 || count > limits.max_mutations {
            return Err(DatabaseError::LimitExceeded("mutations"));
        }
        let mut cursor = TX_HEADER_BYTES;
        let mut mutations = Vec::new();
        mutations
            .try_reserve_exact(count)
            .map_err(|_| DatabaseError::AllocationFailed)?;
        for _ in 0..count {
            let kind = *bytes
                .get(cursor)
                .ok_or(DatabaseError::InvalidWire("mutation kind"))?;
            cursor += 1;
            let before_or_after = take_framed(bytes, &mut cursor, limits.max_row_bytes)?;
            let first = schema.decode_row(before_or_after)?;
            let mutation = match kind {
                1 => RowMutation::Insert(first),
                2 => RowMutation::Delete(first),
                3 => {
                    let after = schema.decode_row(take_framed(
                        bytes,
                        &mut cursor,
                        limits.max_row_bytes,
                    )?)?;
                    RowMutation::Update {
                        before: first,
                        after,
                    }
                }
                _ => return Err(DatabaseError::InvalidWire("mutation kind")),
            };
            mutations.push(mutation);
        }
        if cursor != bytes.len() {
            return Err(DatabaseError::InvalidWire("trailing transaction bytes"));
        }
        let candidate = Self::new(namespace, schema, source, mutations)?;
        if candidate.target_revision != target
            || candidate.transaction_id != derive_transaction_id(bytes)
        {
            return Err(DatabaseError::InvalidWire("transaction identity"));
        }
        Ok(candidate)
    }
}

#[derive(Clone, Debug)]
struct DatabasePartition<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    rows: BTreeMap<DatabaseRowKey, DatabaseRow>,
    signature: MultisetSignature<F, E>,
}

/// Profile-bound commutative summary of all retained row images.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DatabaseSummary<F: Field> {
    namespace: ApplicationNamespace,
    schema_id: DatabaseSchemaId,
    context: SignatureContext,
    evaluation: F,
    nonzero_product: F,
    zero_factor_count: u64,
    row_count: u64,
    partition_count: u64,
}

impl<F: Field> DatabaseSummary<F> {
    /// Application dataset identity.
    #[must_use]
    pub const fn namespace(self) -> ApplicationNamespace {
        self.namespace
    }

    /// Frozen row schema.
    #[must_use]
    pub const fn schema_id(self) -> DatabaseSchemaId {
        self.schema_id
    }

    /// Field/encoder/multiset identity.
    #[must_use]
    pub const fn context(self) -> SignatureContext {
        self.context
    }

    /// Algebraic product evaluation.
    #[must_use]
    pub const fn evaluation(self) -> F {
        self.evaluation
    }

    /// Product excluding zero factors, retained for reversible accounting.
    #[must_use]
    pub const fn nonzero_product(self) -> F {
        self.nonzero_product
    }

    /// Exact count of factors evaluating to zero.
    #[must_use]
    pub const fn zero_factor_count(self) -> u64 {
        self.zero_factor_count
    }

    /// Exact primary-key cardinality.
    #[must_use]
    pub const fn row_count(self) -> u64 {
        self.row_count
    }

    /// Frozen number of physical partitions.
    #[must_use]
    pub const fn partition_count(self) -> u64 {
        self.partition_count
    }
}

/// Exact primary-key table with one compact multiset signature per partition.
#[derive(Clone, Debug)]
pub struct PartitionedDatabase<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    namespace: ApplicationNamespace,
    schema: DatabaseSchema,
    encoder: E,
    offset: F,
    partitions: Vec<DatabasePartition<F, E>>,
    revision: u64,
    applied: BTreeSet<TransactionId>,
}

impl<F, E> PartitionedDatabase<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Invert,
    E: StructuralEncoder<F>,
{
    /// Creates an empty table. Primary keys are unique in v1.
    pub fn new(
        namespace: ApplicationNamespace,
        schema: DatabaseSchema,
        partition_count: usize,
        encoder: E,
        offset: F,
    ) -> Result<Self, DatabaseError> {
        if partition_count == 0 {
            return Err(DatabaseError::InvalidSchema("zero partitions"));
        }
        let mut partitions = Vec::new();
        partitions
            .try_reserve_exact(partition_count)
            .map_err(|_| DatabaseError::AllocationFailed)?;
        for _ in 0..partition_count {
            partitions.push(DatabasePartition {
                rows: BTreeMap::new(),
                signature: MultisetSignature::new(encoder.clone(), offset),
            });
        }
        Ok(Self {
            namespace,
            schema,
            encoder,
            offset,
            partitions,
            revision: 0,
            applied: BTreeSet::new(),
        })
    }

    /// Rebuilds a revision-zero table from exact unique rows.
    pub fn from_rows<I>(
        namespace: ApplicationNamespace,
        schema: DatabaseSchema,
        partition_count: usize,
        encoder: E,
        offset: F,
        rows: I,
    ) -> Result<Self, DatabaseError>
    where
        I: IntoIterator<Item = DatabaseRow>,
    {
        let mut table = Self::new(namespace, schema, partition_count, encoder, offset)?;
        for row in rows {
            table.insert_rebuild(row)?;
        }
        Ok(table)
    }

    /// Current committed revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Number of unique primary keys.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.partitions
            .iter()
            .map(|partition| partition.rows.len())
            .sum()
    }

    /// Exact lookup by a validated row carrying the desired key values.
    pub fn get_by_row_key(&self, row: &DatabaseRow) -> Result<Option<&DatabaseRow>, DatabaseError> {
        let key = self.schema.row_key(row)?;
        let partition = partition_index(key, self.partitions.len());
        Ok(self.partitions[partition].rows.get(&key))
    }

    /// Combines every partition into one schema-bound summary.
    pub fn summary(&self) -> Result<DatabaseSummary<F>, DatabaseError> {
        let mut combined = MultisetSignature::new(self.encoder.clone(), self.offset);
        for partition in &self.partitions {
            combined = combined.combine(&partition.signature)?;
        }
        Ok(DatabaseSummary {
            namespace: self.namespace,
            schema_id: self.schema.schema_id,
            context: combined.context(),
            evaluation: combined.evaluated_product(),
            nonzero_product: combined.nonzero_product(),
            zero_factor_count: combined.zero_factor_count(),
            row_count: combined.cardinality(),
            partition_count: self.partitions.len() as u64,
        })
    }

    /// Exports exact rows in deterministic partition/key order for rebuilds.
    #[must_use]
    pub fn rows(&self) -> Vec<DatabaseRow> {
        self.partitions
            .iter()
            .flat_map(|partition| partition.rows.values().cloned())
            .collect()
    }

    /// Applies one exact transaction using partition candidates and one commit.
    pub fn apply_transaction(
        &mut self,
        transaction: &TransactionDelta,
        limits: DatabaseTransactionLimits,
    ) -> Result<DatabaseApplyReport, DatabaseError> {
        if transaction.namespace != self.namespace {
            return Err(DatabaseError::NamespaceMismatch);
        }
        if transaction.schema_id != self.schema.schema_id {
            return Err(DatabaseError::SchemaMismatch);
        }
        if self.applied.contains(&transaction.transaction_id) {
            return Ok(DatabaseApplyReport {
                status: DatabaseApplyStatus::AlreadyApplied,
                revision: self.revision,
                touched_partitions: 0,
            });
        }
        if transaction.source_revision != self.revision {
            return Err(DatabaseError::RevisionMismatch {
                expected: transaction.source_revision,
                actual: self.revision,
            });
        }
        if transaction.mutations.len() > limits.max_mutations {
            return Err(DatabaseError::LimitExceeded("mutations"));
        }
        if transaction.to_canonical_bytes().len() > limits.max_transaction_bytes {
            return Err(DatabaseError::LimitExceeded("transaction bytes"));
        }
        let mut candidates = BTreeMap::<usize, DatabasePartition<F, E>>::new();
        for mutation in &transaction.mutations {
            self.apply_candidate_mutation(&mut candidates, mutation, limits)?;
        }
        let touched_partitions = candidates.len();
        for (index, partition) in candidates {
            self.partitions[index] = partition;
        }
        self.applied.insert(transaction.transaction_id);
        self.revision = transaction.target_revision;
        Ok(DatabaseApplyReport {
            status: DatabaseApplyStatus::Applied,
            revision: self.revision,
            touched_partitions,
        })
    }

    fn apply_candidate_mutation(
        &self,
        candidates: &mut BTreeMap<usize, DatabasePartition<F, E>>,
        mutation: &RowMutation,
        limits: DatabaseTransactionLimits,
    ) -> Result<(), DatabaseError> {
        let (before, after) = match mutation {
            RowMutation::Insert(after) => (None, Some(after)),
            RowMutation::Delete(before) => (Some(before), None),
            RowMutation::Update { before, after } => (Some(before), Some(after)),
        };
        let reference = before.or(after).expect("a mutation always has one image");
        let key = self.schema.row_key(reference)?;
        if let Some(after) = after {
            let after_key = self.schema.row_key(after)?;
            if after_key != key {
                return Err(DatabaseError::InvalidTransaction("primary-key update"));
            }
        }
        if let (Some(before), Some(after)) = (before, after) {
            if after.version <= before.version {
                return Err(DatabaseError::InvalidTransaction(
                    "non-increasing row version",
                ));
            }
        }
        let index = partition_index(key, self.partitions.len());
        candidates
            .entry(index)
            .or_insert_with(|| self.partitions[index].clone());
        let candidate = candidates.get_mut(&index).expect("partition candidate");
        match (before, after) {
            (None, Some(after)) => {
                if candidate.rows.contains_key(&key) {
                    return Err(DatabaseError::Conflict("inserted key already exists"));
                }
                let bytes = checked_row_bytes(&self.schema, after, limits)?;
                candidate.signature.insert(&bytes)?;
                candidate.rows.insert(key, after.clone());
            }
            (Some(before), None) => {
                if candidate.rows.get(&key) != Some(before) {
                    return Err(DatabaseError::Conflict("delete before image mismatch"));
                }
                let bytes = checked_row_bytes(&self.schema, before, limits)?;
                let residual = candidate.signature.residual_assuming_member(&bytes)?;
                candidate.signature.apply_residual(residual);
                candidate.rows.remove(&key);
            }
            (Some(before), Some(after)) => {
                if candidate.rows.get(&key) != Some(before) {
                    return Err(DatabaseError::Conflict("update before image mismatch"));
                }
                let before_bytes = checked_row_bytes(&self.schema, before, limits)?;
                let after_bytes = checked_row_bytes(&self.schema, after, limits)?;
                let residual = candidate
                    .signature
                    .residual_assuming_member(&before_bytes)?;
                candidate.signature.apply_residual(residual);
                candidate.signature.insert(&after_bytes)?;
                candidate.rows.insert(key, after.clone());
            }
            (None, None) => unreachable!(),
        }
        Ok(())
    }

    fn insert_rebuild(&mut self, row: DatabaseRow) -> Result<(), DatabaseError> {
        let key = self.schema.row_key(&row)?;
        let index = partition_index(key, self.partitions.len());
        if self.partitions[index].rows.contains_key(&key) {
            return Err(DatabaseError::Conflict("duplicate primary key"));
        }
        let bytes = self.schema.encode_row(&row)?;
        self.partitions[index].signature.insert(&bytes)?;
        self.partitions[index].rows.insert(key, row);
        Ok(())
    }
}

/// Transaction application outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatabaseApplyStatus {
    /// Transaction committed.
    Applied,
    /// Exact transaction ID was already committed.
    AlreadyApplied,
}

/// Observable database commit report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DatabaseApplyReport {
    status: DatabaseApplyStatus,
    revision: u64,
    touched_partitions: usize,
}

impl DatabaseApplyReport {
    /// Commit/replay outcome.
    #[must_use]
    pub const fn status(self) -> DatabaseApplyStatus {
        self.status
    }

    /// Revision after the call.
    #[must_use]
    pub const fn revision(self) -> u64 {
        self.revision
    }

    /// Number of partition candidates published.
    #[must_use]
    pub const fn touched_partitions(self) -> usize {
        self.touched_partitions
    }
}

/// Deterministic contiguous transaction log.
#[derive(Clone, Debug, Default)]
pub struct DatabaseTransactionLog {
    entries: Vec<TransactionDelta>,
}

impl DatabaseTransactionLog {
    /// Empty log.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Entry count.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Reports whether no transactions are retained.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Appends a unique contiguous transaction from one namespace/schema.
    pub fn append(&mut self, transaction: TransactionDelta) -> Result<(), DatabaseError> {
        if let Some(previous) = self.entries.last() {
            if previous.namespace != transaction.namespace
                || previous.schema_id != transaction.schema_id
                || previous.target_revision != transaction.source_revision
            {
                return Err(DatabaseError::InvalidTransaction(
                    "log identity or revision gap",
                ));
            }
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.transaction_id == transaction.transaction_id)
        {
            return Err(DatabaseError::InvalidTransaction(
                "duplicate transaction id",
            ));
        }
        self.entries
            .try_reserve(1)
            .map_err(|_| DatabaseError::AllocationFailed)?;
        self.entries.push(transaction);
        Ok(())
    }

    /// Replays the whole log transactionally; repeated replay is idempotent.
    pub fn replay<F, E>(
        &self,
        table: &mut PartitionedDatabase<F, E>,
        limits: DatabaseTransactionLimits,
    ) -> Result<DatabaseReplayReport, DatabaseError>
    where
        F: Field + CanonicalEncoding + StaticField + Invert,
        E: StructuralEncoder<F>,
    {
        let mut candidate = table.clone();
        let mut applied = 0_u64;
        let mut skipped = 0_u64;
        for transaction in &self.entries {
            match candidate.apply_transaction(transaction, limits)?.status() {
                DatabaseApplyStatus::Applied => applied += 1,
                DatabaseApplyStatus::AlreadyApplied => skipped += 1,
            }
        }
        *table = candidate;
        Ok(DatabaseReplayReport {
            applied,
            skipped,
            revision: table.revision,
        })
    }

    /// Persists every framed `MFTX` entry as `MFTL` schema 1.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, DatabaseError> {
        let entries = self
            .entries
            .iter()
            .map(TransactionDelta::to_canonical_bytes)
            .collect::<Vec<_>>();
        let total = entries.iter().try_fold(LOG_HEADER_BYTES, |size, entry| {
            size.checked_add(8)?.checked_add(entry.len())
        });
        let total = total.ok_or(DatabaseError::LimitExceeded("log bytes"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(total)
            .map_err(|_| DatabaseError::AllocationFailed)?;
        bytes.extend_from_slice(LOG_MAGIC);
        bytes.extend_from_slice(&LOG_SCHEMA.to_le_bytes());
        bytes.extend_from_slice(&(entries.len() as u64).to_le_bytes());
        for entry in entries {
            encode_framed(&mut bytes, &entry);
        }
        Ok(bytes)
    }

    /// Restores and revalidates a complete transaction log.
    pub fn from_canonical_bytes(
        namespace: ApplicationNamespace,
        schema: &DatabaseSchema,
        bytes: &[u8],
        limits: DatabaseTransactionLimits,
    ) -> Result<Self, DatabaseError> {
        if bytes.len() > limits.max_log_bytes {
            return Err(DatabaseError::LimitExceeded("log bytes"));
        }
        if bytes.len() < LOG_HEADER_BYTES || &bytes[..4] != LOG_MAGIC {
            return Err(DatabaseError::InvalidWire("transaction log header"));
        }
        if u16::from_le_bytes([bytes[4], bytes[5]]) != LOG_SCHEMA {
            return Err(DatabaseError::InvalidWire("transaction log schema"));
        }
        let count = read_usize(&bytes[6..14], "log entry count")?;
        if count > limits.max_log_entries {
            return Err(DatabaseError::LimitExceeded("log entries"));
        }
        let mut cursor = LOG_HEADER_BYTES;
        let mut log = Self::new();
        log.entries
            .try_reserve_exact(count)
            .map_err(|_| DatabaseError::AllocationFailed)?;
        for _ in 0..count {
            let entry = take_framed(bytes, &mut cursor, limits.max_transaction_bytes)?;
            log.append(TransactionDelta::from_canonical_bytes(
                namespace, schema, entry, limits,
            )?)?;
        }
        if cursor != bytes.len() {
            return Err(DatabaseError::InvalidWire("trailing log bytes"));
        }
        Ok(log)
    }
}

/// Aggregate log replay report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DatabaseReplayReport {
    applied: u64,
    skipped: u64,
    revision: u64,
}

impl DatabaseReplayReport {
    /// Newly committed transactions.
    #[must_use]
    pub const fn applied(self) -> u64 {
        self.applied
    }

    /// Transactions recognized as already committed.
    #[must_use]
    pub const fn skipped(self) -> u64 {
        self.skipped
    }

    /// Final table revision.
    #[must_use]
    pub const fn revision(self) -> u64 {
        self.revision
    }
}

fn validate_mutations(
    schema: &DatabaseSchema,
    mutations: &[RowMutation],
) -> Result<(), DatabaseError> {
    let mut keys = BTreeSet::new();
    for mutation in mutations {
        let (before, after) = match mutation {
            RowMutation::Insert(after) => (None, Some(after)),
            RowMutation::Delete(before) => (Some(before), None),
            RowMutation::Update { before, after } => (Some(before), Some(after)),
        };
        let key = schema.row_key(before.or(after).expect("mutation image"))?;
        if !keys.insert(key) {
            return Err(DatabaseError::InvalidTransaction(
                "duplicate primary key mutation",
            ));
        }
        if let Some(after) = after {
            if schema.row_key(after)? != key {
                return Err(DatabaseError::InvalidTransaction("primary-key update"));
            }
        }
        if let (Some(before), Some(after)) = (before, after) {
            if after.version <= before.version {
                return Err(DatabaseError::InvalidTransaction(
                    "non-increasing row version",
                ));
            }
        }
    }
    Ok(())
}

fn checked_row_bytes(
    schema: &DatabaseSchema,
    row: &DatabaseRow,
    limits: DatabaseTransactionLimits,
) -> Result<Vec<u8>, DatabaseError> {
    let bytes = schema.encode_row(row)?;
    if bytes.len() > limits.max_row_bytes {
        Err(DatabaseError::LimitExceeded("row bytes"))
    } else {
        Ok(bytes)
    }
}

fn partition_index(key: DatabaseRowKey, count: usize) -> usize {
    let value = u64::from_le_bytes(key.0[..8].try_into().expect("partition key range"));
    (value % count as u64) as usize
}

fn derive_transaction_id(bytes: &[u8]) -> TransactionId {
    let mut hasher = Sha256::new();
    hasher.update(b"microfield-database-transaction-v1\0");
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    TransactionId(hasher.finalize().into())
}

fn row_wire_unchecked(schema_id: DatabaseSchemaId, row: &DatabaseRow) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(ROW_MAGIC);
    bytes.extend_from_slice(&ROW_SCHEMA.to_le_bytes());
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(schema_id.as_bytes());
    bytes.extend_from_slice(&row.version.to_le_bytes());
    bytes.extend_from_slice(&(row.values.len() as u64).to_le_bytes());
    for value in &row.values {
        encode_value(&mut bytes, value);
    }
    bytes
}

fn encode_value(bytes: &mut Vec<u8>, value: &DatabaseValue) {
    match value {
        DatabaseValue::Null => bytes.push(0),
        DatabaseValue::Bool(value) => {
            bytes.push(1);
            bytes.push(u8::from(*value));
        }
        DatabaseValue::I64(value) => {
            bytes.push(2);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        DatabaseValue::U64(value) => {
            bytes.push(3);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        DatabaseValue::Bytes(value) => {
            bytes.push(4);
            encode_framed(bytes, value);
        }
        DatabaseValue::Text(value) => {
            bytes.push(5);
            encode_framed(bytes, value.as_bytes());
        }
    }
}

fn decode_value(bytes: &[u8], cursor: &mut usize) -> Result<DatabaseValue, DatabaseError> {
    let tag = *bytes
        .get(*cursor)
        .ok_or(DatabaseError::InvalidWire("row value tag"))?;
    *cursor += 1;
    match tag {
        0 => Ok(DatabaseValue::Null),
        1 => {
            let value = *bytes
                .get(*cursor)
                .ok_or(DatabaseError::InvalidWire("boolean value"))?;
            *cursor += 1;
            match value {
                0 => Ok(DatabaseValue::Bool(false)),
                1 => Ok(DatabaseValue::Bool(true)),
                _ => Err(DatabaseError::InvalidWire("non-canonical boolean")),
            }
        }
        2 => Ok(DatabaseValue::I64(i64::from_le_bytes(take_fixed::<8>(
            bytes,
            cursor,
            "i64 value",
        )?))),
        3 => Ok(DatabaseValue::U64(u64::from_le_bytes(take_fixed::<8>(
            bytes,
            cursor,
            "u64 value",
        )?))),
        4 => Ok(DatabaseValue::Bytes(
            take_framed(bytes, cursor, usize::MAX)?.to_vec(),
        )),
        5 => {
            let value = take_framed(bytes, cursor, usize::MAX)?;
            Ok(DatabaseValue::Text(
                core::str::from_utf8(value)
                    .map_err(|_| DatabaseError::InvalidWire("invalid UTF-8"))?
                    .to_owned(),
            ))
        }
        _ => Err(DatabaseError::InvalidWire("row value tag")),
    }
}

fn encode_framed(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value);
}

fn take_framed<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    maximum: usize,
) -> Result<&'a [u8], DatabaseError> {
    let length = read_usize(
        bytes
            .get(*cursor..(*cursor).saturating_add(8))
            .ok_or(DatabaseError::InvalidWire("framed length"))?,
        "framed length",
    )?;
    if length > maximum {
        return Err(DatabaseError::LimitExceeded("framed bytes"));
    }
    *cursor = cursor
        .checked_add(8)
        .ok_or(DatabaseError::InvalidWire("framed cursor"))?;
    let end = cursor
        .checked_add(length)
        .filter(|end| *end <= bytes.len())
        .ok_or(DatabaseError::InvalidWire("framed boundary"))?;
    let value = &bytes[*cursor..end];
    *cursor = end;
    Ok(value)
}

fn take_fixed<const N: usize>(
    bytes: &[u8],
    cursor: &mut usize,
    label: &'static str,
) -> Result<[u8; N], DatabaseError> {
    let end = cursor
        .checked_add(N)
        .filter(|end| *end <= bytes.len())
        .ok_or(DatabaseError::InvalidWire(label))?;
    let value = bytes[*cursor..end].try_into().expect("fixed range");
    *cursor = end;
    Ok(value)
}

fn read_usize(bytes: &[u8], label: &'static str) -> Result<usize, DatabaseError> {
    let value = u64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| DatabaseError::InvalidWire(label))?,
    );
    usize::try_from(value).map_err(|_| DatabaseError::LimitExceeded(label))
}
