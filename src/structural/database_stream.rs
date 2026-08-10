//! Ordered committed-change streams from transactional databases.
//!
//! Source positions (for example PostgreSQL commit LSNs) are monotonic but are
//! not required to be contiguous. They are deliberately kept separate from
//! Algesum's contiguous table revision.

use core::fmt;

use microfield::{CanonicalEncoding, Field, Invert, StaticField};

use super::{
    ApplicationNamespace, DatabaseApplyPolicy, DatabaseApplyReport, DatabaseApplyStatus,
    DatabaseError, DatabasePolicyApplyReport, DatabaseSchema, DatabaseTransactionLimits,
    PartitionedDatabase, RowMutation, StructuralEncoder, TransactionDelta, TransactionId,
};

/// Stable identity of one authoritative database/change stream.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DatabaseSourceId([u8; 32]);

impl DatabaseSourceId {
    /// Creates an application-assigned source identity.
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrows the canonical identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// One transaction observed only after its authoritative commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedDatabaseTransaction {
    source: DatabaseSourceId,
    commit_position: u64,
    mutations: Vec<RowMutation>,
}

impl CommittedDatabaseTransaction {
    /// Creates a committed batch. Position zero is reserved as "not started".
    pub fn new(
        source: DatabaseSourceId,
        commit_position: u64,
        mutations: Vec<RowMutation>,
    ) -> Result<Self, DatabaseReplicationError> {
        if commit_position == 0 {
            return Err(DatabaseReplicationError::InvalidPosition);
        }
        Ok(Self {
            source,
            commit_position,
            mutations,
        })
    }

    /// Authoritative source identity.
    #[must_use]
    pub const fn source(&self) -> DatabaseSourceId {
        self.source
    }

    /// Monotonic source position, such as a PostgreSQL commit LSN.
    #[must_use]
    pub const fn commit_position(&self) -> u64 {
        self.commit_position
    }

    /// Exact before/after images in the source transaction.
    #[must_use]
    pub fn mutations(&self) -> &[RowMutation] {
        &self.mutations
    }
}

/// Durable adapter state to store alongside the consumer checkpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DatabaseReplicationCheckpoint {
    source: DatabaseSourceId,
    commit_position: u64,
    algesum_revision: u64,
    last_transaction_id: Option<TransactionId>,
}

impl DatabaseReplicationCheckpoint {
    /// Restores application-durable checkpoint fields after validating their
    /// state transition invariants.
    pub const fn from_parts(
        source: DatabaseSourceId,
        commit_position: u64,
        algesum_revision: u64,
        last_transaction_id: Option<TransactionId>,
    ) -> Result<Self, DatabaseReplicationError> {
        let initial =
            commit_position == 0 && algesum_revision == 0 && last_transaction_id.is_none();
        let advanced = commit_position > 0 && algesum_revision > 0 && last_transaction_id.is_some();
        if !initial && !advanced {
            return Err(DatabaseReplicationError::InvalidCheckpoint);
        }
        Ok(Self {
            source,
            commit_position,
            algesum_revision,
            last_transaction_id,
        })
    }

    /// Authoritative source identity.
    #[must_use]
    pub const fn source(self) -> DatabaseSourceId {
        self.source
    }

    /// Last committed source position, or zero before the first transaction.
    #[must_use]
    pub const fn commit_position(self) -> u64 {
        self.commit_position
    }

    /// Contiguous Algesum revision corresponding to the source position.
    #[must_use]
    pub const fn algesum_revision(self) -> u64 {
        self.algesum_revision
    }

    /// Identity used to recognize exact redelivery at the current position.
    #[must_use]
    pub const fn last_transaction_id(self) -> Option<TransactionId> {
        self.last_transaction_id
    }
}

/// Stateful bridge from sparse external commit positions to contiguous revisions.
#[derive(Clone, Debug)]
pub struct DatabaseChangeStreamAdapter {
    namespace: ApplicationNamespace,
    schema: DatabaseSchema,
    checkpoint: DatabaseReplicationCheckpoint,
}

impl DatabaseChangeStreamAdapter {
    /// Starts a consumer before the first source transaction.
    #[must_use]
    pub fn new(
        namespace: ApplicationNamespace,
        schema: DatabaseSchema,
        source: DatabaseSourceId,
    ) -> Self {
        Self {
            namespace,
            schema,
            checkpoint: DatabaseReplicationCheckpoint {
                source,
                commit_position: 0,
                algesum_revision: 0,
                last_transaction_id: None,
            },
        }
    }

    /// Restores a consumer from an application-durable checkpoint.
    #[must_use]
    pub const fn from_checkpoint(
        namespace: ApplicationNamespace,
        schema: DatabaseSchema,
        checkpoint: DatabaseReplicationCheckpoint,
    ) -> Self {
        Self {
            namespace,
            schema,
            checkpoint,
        }
    }

    /// Current durable state. Persist it after a successful application.
    #[must_use]
    pub const fn checkpoint(&self) -> DatabaseReplicationCheckpoint {
        self.checkpoint
    }

    /// Applies one committed source transaction and advances the checkpoint.
    ///
    /// Gaps between source positions are valid. An exact redelivery of the
    /// latest batch is idempotent; regressions and conflicting reuse of one
    /// position fail closed.
    pub fn apply_committed<F, E>(
        &mut self,
        table: &mut PartitionedDatabase<F, E>,
        batch: &CommittedDatabaseTransaction,
        limits: DatabaseTransactionLimits,
    ) -> Result<DatabaseApplyReport, DatabaseReplicationError>
    where
        F: Field + CanonicalEncoding + StaticField + Invert,
        E: StructuralEncoder<F>,
    {
        if batch.source != self.checkpoint.source {
            return Err(DatabaseReplicationError::SourceMismatch);
        }
        if table.revision() != self.checkpoint.algesum_revision {
            return Err(DatabaseReplicationError::RevisionMismatch {
                checkpoint: self.checkpoint.algesum_revision,
                table: table.revision(),
            });
        }
        if batch.commit_position < self.checkpoint.commit_position {
            return Err(DatabaseReplicationError::PositionRegression {
                previous: self.checkpoint.commit_position,
                received: batch.commit_position,
            });
        }

        let source_revision = if batch.commit_position == self.checkpoint.commit_position {
            self.checkpoint
                .algesum_revision
                .checked_sub(1)
                .ok_or(DatabaseReplicationError::PositionConflict)?
        } else {
            self.checkpoint.algesum_revision
        };
        let transaction = TransactionDelta::new(
            self.namespace,
            &self.schema,
            source_revision,
            batch.mutations.clone(),
        )?;

        if batch.commit_position == self.checkpoint.commit_position {
            if self.checkpoint.last_transaction_id != Some(transaction.transaction_id()) {
                return Err(DatabaseReplicationError::PositionConflict);
            }
            let report = table.apply_transaction(&transaction, limits)?;
            debug_assert_eq!(report.status(), DatabaseApplyStatus::AlreadyApplied);
            return Ok(report);
        }

        let report = table.apply_transaction(&transaction, limits)?;
        self.checkpoint = DatabaseReplicationCheckpoint {
            source: self.checkpoint.source,
            commit_position: batch.commit_position,
            algesum_revision: report.revision(),
            last_transaction_id: Some(transaction.transaction_id()),
        };
        Ok(report)
    }

    /// Applies a committed batch using the measured incremental/rebuild policy.
    ///
    /// `authoritative_target_rows` must return the exact database state after
    /// this source commit. It is evaluated only when the absolute policy
    /// threshold selects the verified full-rebuild path.
    pub fn apply_committed_with_policy<F, E, I, P>(
        &mut self,
        table: &mut PartitionedDatabase<F, E>,
        batch: &CommittedDatabaseTransaction,
        limits: DatabaseTransactionLimits,
        policy: DatabaseApplyPolicy,
        authoritative_target_rows: P,
    ) -> Result<DatabasePolicyApplyReport, DatabaseReplicationError>
    where
        F: Field + CanonicalEncoding + StaticField + Invert,
        E: StructuralEncoder<F>,
        I: IntoIterator<Item = super::DatabaseRow>,
        P: FnOnce() -> I,
    {
        if batch.source != self.checkpoint.source {
            return Err(DatabaseReplicationError::SourceMismatch);
        }
        if table.revision() != self.checkpoint.algesum_revision {
            return Err(DatabaseReplicationError::RevisionMismatch {
                checkpoint: self.checkpoint.algesum_revision,
                table: table.revision(),
            });
        }
        if batch.commit_position < self.checkpoint.commit_position {
            return Err(DatabaseReplicationError::PositionRegression {
                previous: self.checkpoint.commit_position,
                received: batch.commit_position,
            });
        }
        let source_revision = if batch.commit_position == self.checkpoint.commit_position {
            self.checkpoint
                .algesum_revision
                .checked_sub(1)
                .ok_or(DatabaseReplicationError::PositionConflict)?
        } else {
            self.checkpoint.algesum_revision
        };
        let transaction = TransactionDelta::new(
            self.namespace,
            &self.schema,
            source_revision,
            batch.mutations.clone(),
        )?;
        if batch.commit_position == self.checkpoint.commit_position
            && self.checkpoint.last_transaction_id != Some(transaction.transaction_id())
        {
            return Err(DatabaseReplicationError::PositionConflict);
        }

        let report = table.apply_transaction_with_policy(
            &transaction,
            limits,
            policy,
            authoritative_target_rows,
        )?;
        if batch.commit_position > self.checkpoint.commit_position {
            self.checkpoint = DatabaseReplicationCheckpoint {
                source: self.checkpoint.source,
                commit_position: batch.commit_position,
                algesum_revision: report.revision(),
                last_transaction_id: Some(transaction.transaction_id()),
            };
        }
        Ok(report)
    }
}

/// Fail-closed replication adapter error.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DatabaseReplicationError {
    /// Position zero is reserved for the initial checkpoint.
    InvalidPosition,
    /// Restored checkpoint fields do not describe an initial or advanced state.
    InvalidCheckpoint,
    /// Batch came from another database/change stream.
    SourceMismatch,
    /// Source position moved backwards.
    PositionRegression { previous: u64, received: u64 },
    /// The latest position was redelivered with different contents.
    PositionConflict,
    /// Restored adapter and exact table do not describe the same revision.
    RevisionMismatch { checkpoint: u64, table: u64 },
    /// Core database validation or application failed.
    Database(DatabaseError),
}

impl fmt::Display for DatabaseReplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPosition => formatter.write_str("database commit position must be nonzero"),
            Self::InvalidCheckpoint => {
                formatter.write_str("invalid database replication checkpoint fields")
            }
            Self::SourceMismatch => formatter.write_str("database replication source mismatch"),
            Self::PositionRegression { previous, received } => write!(
                formatter,
                "database commit position regressed from {previous} to {received}"
            ),
            Self::PositionConflict => {
                formatter.write_str("database commit position was reused with different contents")
            }
            Self::RevisionMismatch { checkpoint, table } => write!(
                formatter,
                "database replication checkpoint revision {checkpoint} differs from table revision {table}"
            ),
            Self::Database(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for DatabaseReplicationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<DatabaseError> for DatabaseReplicationError {
    fn from(error: DatabaseError) -> Self {
        Self::Database(error)
    }
}
