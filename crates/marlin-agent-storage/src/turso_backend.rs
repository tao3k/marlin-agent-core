#[path = "turso_operations.rs"]
mod operations;
#[path = "turso_schema.rs"]
mod schema;
#[path = "turso_storage.rs"]
mod storage;

use schema::STORAGE_SCHEMA;

use std::path::PathBuf;
use std::time::Duration;

use crate::records::{
    AgentId, AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerRecord,
    ArtifactPointerUpdate, ArtifactPutOutcome, ArtifactRecord, EventId, MemoryKey,
    MemoryProposalId, MemoryProposalRecord, ProjectId, SessionEventRecord, SessionId, StorageError,
    StorageFuture, StorageResult, StorageSchemaSnapshot, StorageSchemaTable, TopologyEdgeId,
    TopologyEdgeRecord, TopologyNodeId, TurnId, VisibilityReceipt,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TursoAgentStorageConfig {
    pub path: PathBuf,
    pub optimization_profile: TursoOptimizationProfile,
    pub batch_transaction_mode: TursoBatchTransactionMode,
}

#[cfg(feature = "turso-sync")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TursoSyncRemoteUrl(String);

#[cfg(feature = "turso-sync")]
impl TursoSyncRemoteUrl {
    pub fn new(value: impl Into<String>) -> StorageResult<Self> {
        let value = value.into();
        if !(value.starts_with("libsql://")
            || value.starts_with("https://")
            || value.starts_with("http://"))
        {
            return Err(StorageError::InvalidSyncConfiguration {
                reason: "remote URL must use libsql, https, or http",
            });
        }
        Ok(Self(value))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "turso-sync")]
#[derive(Clone, PartialEq, Eq)]
pub struct TursoSyncAuthToken(String);

#[cfg(feature = "turso-sync")]
impl TursoSyncAuthToken {
    pub fn new(value: impl Into<String>) -> StorageResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(StorageError::InvalidSyncConfiguration {
                reason: "auth token must not be empty",
            });
        }
        Ok(Self(value))
    }

    fn expose(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "turso-sync")]
impl std::fmt::Debug for TursoSyncAuthToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("TursoSyncAuthToken([REDACTED])")
    }
}

#[cfg(feature = "turso-sync")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TursoSyncAgentStorageConfig {
    pub path: PathBuf,
    pub remote_url: TursoSyncRemoteUrl,
    pub auth_token: TursoSyncAuthToken,
    pub bootstrap_if_empty: bool,
}

#[cfg(feature = "turso-sync")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TursoSyncStatsReceipt {
    pub network_received_bytes: u64,
    pub network_sent_bytes: u64,
    pub main_wal_size: u64,
    pub revision: Option<String>,
    pub last_pull_unix_time: Option<i64>,
    pub last_push_unix_time: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TursoAsyncIoMode {
    Enabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TursoMvccMode {
    Required,
    DisabledForCompatibility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TursoMvccCheckpointMode {
    Blocking,
    PassiveExperimental,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TursoStatementCacheMode {
    PreparedCachedPerConnection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TursoBatchTransactionMode {
    Immediate,
    Concurrent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TursoOptimizationProfile {
    AsyncIoOnlyCompatibility,
    AsyncIoWithMvcc,
    AsyncIoWithMvccAndPassiveCheckpointExperimental,
}

impl TursoOptimizationProfile {
    pub const fn receipt(self) -> TursoOptimizationReceipt {
        match self {
            Self::AsyncIoOnlyCompatibility => TursoOptimizationReceipt {
                async_io: TursoAsyncIoMode::Enabled,
                mvcc: TursoMvccMode::DisabledForCompatibility,
                mvcc_checkpoint: TursoMvccCheckpointMode::Blocking,
                connection_lanes: 1,
                statement_cache: TursoStatementCacheMode::PreparedCachedPerConnection,
            },
            Self::AsyncIoWithMvcc => TursoOptimizationReceipt {
                async_io: TursoAsyncIoMode::Enabled,
                mvcc: TursoMvccMode::Required,
                mvcc_checkpoint: TursoMvccCheckpointMode::Blocking,
                connection_lanes: 4,
                statement_cache: TursoStatementCacheMode::PreparedCachedPerConnection,
            },
            Self::AsyncIoWithMvccAndPassiveCheckpointExperimental => TursoOptimizationReceipt {
                async_io: TursoAsyncIoMode::Enabled,
                mvcc: TursoMvccMode::Required,
                mvcc_checkpoint: TursoMvccCheckpointMode::PassiveExperimental,
                connection_lanes: 4,
                statement_cache: TursoStatementCacheMode::PreparedCachedPerConnection,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TursoOptimizationReceipt {
    pub async_io: TursoAsyncIoMode,
    pub mvcc: TursoMvccMode,
    pub mvcc_checkpoint: TursoMvccCheckpointMode,
    pub connection_lanes: usize,
    pub statement_cache: TursoStatementCacheMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TursoBatchWriteReceipt {
    pub transaction_mode: TursoBatchTransactionMode,
    pub item_count: usize,
    pub rows_affected: u64,
    pub retry_count: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TursoMemoryEmbedding(Vec<f32>);

impl TursoMemoryEmbedding {
    pub fn new(values: Vec<f32>) -> StorageResult<Self> {
        if values.is_empty() {
            return Err(StorageError::InvalidEmbedding {
                reason: "vector must not be empty",
            });
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err(StorageError::InvalidEmbedding {
                reason: "all components must be finite",
            });
        }
        Ok(Self(values))
    }

    pub fn dimension(&self) -> usize {
        self.0.len()
    }

    fn json(&self) -> StorageResult<String> {
        serde_json::to_string(&self.0).map_err(|error| StorageError::Backend {
            message: format!("failed to serialize memory embedding: {error}"),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TursoMemoryEmbeddingRecord {
    pub project_id: ProjectId,
    pub memory_key: MemoryKey,
    pub embedding: TursoMemoryEmbedding,
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TursoMemorySearchLimit(u32);

impl TursoMemorySearchLimit {
    pub fn new(limit: u32) -> StorageResult<Self> {
        if !(1..=100).contains(&limit) {
            return Err(StorageError::InvalidMemorySearchLimit { limit });
        }
        Ok(Self(limit))
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TursoMemorySearchRequest {
    pub project_id: ProjectId,
    pub embedding: TursoMemoryEmbedding,
    pub limit: TursoMemorySearchLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TursoMemorySearchMatch {
    pub memory_key: MemoryKey,
    pub cosine_distance: f64,
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TursoSyncFeatureStatus {
    NotCompiled,
    Compiled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TursoSdkTelemetryStatus {
    NotExposedBySdk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TursoOperationalReceipt {
    pub optimization: TursoOptimizationReceipt,
    pub sync_feature: TursoSyncFeatureStatus,
    pub checkpoint_mode: TursoMvccCheckpointMode,
    pub checkpoint_stats: TursoSdkTelemetryStatus,
    pub database_stats: TursoSdkTelemetryStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TursoTransactionOperation {
    ArtifactPointerCompareAndSwap,
}

impl TursoTransactionOperation {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ArtifactPointerCompareAndSwap => "artifact_pointer.cas",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TursoTransactionStatus {
    Committed,
    Conflict,
}

impl TursoTransactionStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Committed => "committed",
            Self::Conflict => "conflict",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TursoTransactionReceipt {
    pub operation: TursoTransactionOperation,
    pub status: TursoTransactionStatus,
    pub retry_count: u32,
    pub rows_affected: u64,
}

impl TursoTransactionReceipt {
    pub fn visibility_body(self) -> Vec<u8> {
        format!(
            "operation={}\nstatus={}\nretry_count={}\nrows_affected={}\n",
            self.operation.as_str(),
            self.status.as_str(),
            self.retry_count,
            self.rows_affected,
        )
        .into_bytes()
    }
}

#[derive(Clone)]
pub struct TursoAgentStorage {
    _database: TursoDatabaseAuthority,
    connections: TursoConnectionPool,
    config: TursoAgentStorageConfig,
}

#[derive(Clone)]
enum TursoDatabaseAuthority {
    Local(turso::Database),
    #[cfg(feature = "turso-sync")]
    Sync(turso::sync::Database),
}

impl TursoDatabaseAuthority {
    async fn connect(&self) -> StorageResult<turso::Connection> {
        match self {
            Self::Local(database) => database.connect().map_err(map_turso_error),
            #[cfg(feature = "turso-sync")]
            Self::Sync(database) => database.connect().await.map_err(map_turso_error),
        }
    }
}

#[derive(Clone)]
struct TursoConnectionPool {
    sender: tokio::sync::mpsc::UnboundedSender<turso::Connection>,
    receiver:
        std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<turso::Connection>>>,
}

struct TursoConnectionLease {
    connection: Option<turso::Connection>,
    sender: tokio::sync::mpsc::UnboundedSender<turso::Connection>,
}

impl TursoConnectionPool {
    async fn new(database: &TursoDatabaseAuthority, lane_count: usize) -> StorageResult<Self> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        for _ in 0..lane_count {
            let connection = database.connect().await?;
            connection
                .busy_timeout(Duration::from_millis(100))
                .map_err(map_turso_error)?;
            sender.send(connection).map_err(|_| StorageError::Backend {
                message: "failed to initialize Turso connection pool".to_string(),
            })?;
        }
        Ok(Self {
            sender,
            receiver: std::sync::Arc::new(tokio::sync::Mutex::new(receiver)),
        })
    }

    async fn acquire(&self) -> StorageResult<TursoConnectionLease> {
        let connection =
            self.receiver
                .lock()
                .await
                .recv()
                .await
                .ok_or_else(|| StorageError::Backend {
                    message: "Turso connection pool closed unexpectedly".to_string(),
                })?;
        Ok(TursoConnectionLease {
            connection: Some(connection),
            sender: self.sender.clone(),
        })
    }
}

impl std::ops::Deref for TursoConnectionLease {
    type Target = turso::Connection;

    fn deref(&self) -> &Self::Target {
        self.connection
            .as_ref()
            .expect("Turso connection lease must own a connection until drop")
    }
}

impl Drop for TursoConnectionLease {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            let _ = self.sender.send(connection);
        }
    }
}

fn local_open_lock(path: &std::path::Path) -> std::sync::Arc<tokio::sync::Mutex<()>> {
    type OpenLock = tokio::sync::Mutex<()>;
    type OpenLockRegistry = std::collections::HashMap<PathBuf, std::sync::Weak<OpenLock>>;

    static OPEN_LOCKS: std::sync::OnceLock<std::sync::Mutex<OpenLockRegistry>> =
        std::sync::OnceLock::new();

    let registry = OPEN_LOCKS.get_or_init(|| std::sync::Mutex::new(OpenLockRegistry::new()));
    let mut locks = registry
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(path).and_then(std::sync::Weak::upgrade) {
        return lock;
    }

    let lock = std::sync::Arc::new(OpenLock::new(()));
    locks.insert(path.to_path_buf(), std::sync::Arc::downgrade(&lock));
    lock
}

fn map_turso_error(error: turso::Error) -> StorageError {
    StorageError::Backend {
        message: format!("turso backend error: {error}"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExecuteRetryOutcome {
    rows_affected: u64,
    retry_count: u32,
}

enum BatchAttemptError {
    Backend(turso::Error),
    Domain(StorageError),
}

async fn append_session_events_transaction(
    connection: &turso::Connection,
    transaction_mode: TursoBatchTransactionMode,
    records: &[SessionEventRecord],
) -> Result<u64, BatchAttemptError> {
    let mut unique_keys = std::collections::BTreeSet::new();
    if let Some(key) = records
        .iter()
        .map(SessionEventRecord::key)
        .find(|key| !unique_keys.insert(key.clone()))
    {
        return Err(BatchAttemptError::Domain(
            StorageError::DuplicateSessionEvent { key },
        ));
    }

    if transaction_mode == TursoBatchTransactionMode::Immediate {
        return append_session_events_native_transaction(connection, records).await;
    }
    connection
        .execute("BEGIN CONCURRENT", ())
        .await
        .map_err(BatchAttemptError::Backend)?;

    let mut statement = connection
        .prepare_cached(SESSION_EVENT_INSERT_SQL)
        .await
        .map_err(BatchAttemptError::Backend)?;
    let mut rows_affected = 0;
    for record in records {
        let affected = statement
            .execute(session_event_insert_params(record))
            .await
            .map_err(BatchAttemptError::Backend)?;
        if affected != 1 {
            connection
                .execute("ROLLBACK", ())
                .await
                .map_err(BatchAttemptError::Backend)?;
            return Err(BatchAttemptError::Domain(
                StorageError::DuplicateSessionEvent { key: record.key() },
            ));
        }
        rows_affected += affected;
    }

    connection
        .execute("COMMIT", ())
        .await
        .map_err(BatchAttemptError::Backend)?;
    Ok(rows_affected)
}

async fn append_session_events_native_transaction(
    connection: &turso::Connection,
    records: &[SessionEventRecord],
) -> Result<u64, BatchAttemptError> {
    let transaction = turso::transaction::Transaction::new_unchecked(
        connection,
        turso::transaction::TransactionBehavior::Immediate,
    )
    .await
    .map_err(BatchAttemptError::Backend)?;
    let mut statement = transaction
        .prepare(SESSION_EVENT_INSERT_SQL)
        .await
        .map_err(BatchAttemptError::Backend)?;
    let mut rows_affected = 0;
    for record in records {
        let affected = statement
            .execute(session_event_insert_params(record))
            .await
            .map_err(BatchAttemptError::Backend)?;
        if affected != 1 {
            drop(statement);
            transaction
                .rollback()
                .await
                .map_err(BatchAttemptError::Backend)?;
            return Err(BatchAttemptError::Domain(
                StorageError::DuplicateSessionEvent { key: record.key() },
            ));
        }
        rows_affected += affected;
    }
    drop(statement);
    transaction
        .commit()
        .await
        .map_err(BatchAttemptError::Backend)?;
    Ok(rows_affected)
}

const SESSION_EVENT_INSERT_SQL: &str = "INSERT OR IGNORE INTO session_events (
            project_id,
            session_id,
            agent_id,
            turn_id,
            event_id,
            event_kind,
            causality_parent_event_id,
            body,
            created_at_unix_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";

fn session_event_insert_params(record: &SessionEventRecord) -> Vec<turso::Value> {
    vec![
        text_value(record.project_id.clone()),
        text_value(record.session_id.clone()),
        text_value(record.agent_id.clone()),
        text_value(record.turn_id.clone()),
        text_value(record.event_id.clone()),
        turso::Value::Text(record.event_kind.clone()),
        optional_text_value(record.causality_parent_event_id.clone()),
        turso::Value::Blob(record.body.clone()),
        turso::Value::Integer(record.created_at_unix_ms),
    ]
}

async fn execute_retrying(
    connection: &turso::Connection,
    sql: &str,
    mut params: impl FnMut() -> Vec<turso::Value>,
) -> StorageResult<ExecuteRetryOutcome> {
    const MAX_ATTEMPTS: u32 = 8;
    let mut retry_count = 0;

    for attempt in 0..MAX_ATTEMPTS {
        let execute_result = async {
            let mut statement = connection.prepare_cached(sql).await?;
            statement.execute(params()).await
        }
        .await;
        match execute_result {
            Ok(rows_affected) => {
                return Ok(ExecuteRetryOutcome {
                    rows_affected,
                    retry_count,
                });
            }
            Err(error) if is_retryable_turso_error(&error) && attempt + 1 < MAX_ATTEMPTS => {
                retry_count += 1;
                let backoff_ms = 1_u64 << attempt.min(5);
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
            Err(error) => return Err(map_turso_error(error)),
        }
    }

    Err(StorageError::Backend {
        message: "turso retry loop exhausted unexpectedly".to_string(),
    })
}

fn is_retryable_turso_error(error: &turso::Error) -> bool {
    let message = error.to_string();
    message.contains("database is locked")
        || message.contains("busy")
        || message.contains("Busy")
        || message.contains("snapshot")
        || message.contains("Snapshot")
}

async fn list_schema_tables(
    connection: &turso::Connection,
) -> StorageResult<Vec<StorageSchemaTable>> {
    let mut rows = connection
        .query(
            "SELECT name
            FROM sqlite_schema
            WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
            ORDER BY name",
            (),
        )
        .await
        .map_err(map_turso_error)?;

    let mut tables = Vec::new();
    while let Some(row) = rows.next().await.map_err(map_turso_error)? {
        tables.push(StorageSchemaTable {
            name: text_at(&row, 0)?,
        });
    }
    Ok(tables)
}

async fn insert_visibility_receipt(
    connection: &turso::Connection,
    receipt: VisibilityReceipt,
) -> StorageResult<()> {
    execute_retrying(
        connection,
        "INSERT INTO visibility_receipts (
            project_id,
            receipt_id,
            receipt_kind,
            body,
            created_at_unix_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5)",
        || {
            vec![
                text_value(receipt.project_id.clone()),
                turso::Value::Text(receipt.receipt_id.clone()),
                turso::Value::Text(receipt.receipt_kind.clone()),
                turso::Value::Blob(receipt.body.clone()),
                turso::Value::Integer(receipt.created_at_unix_ms),
            ]
        },
    )
    .await?;
    Ok(())
}

async fn drain_rows(connection: &turso::Connection, sql: &str) -> StorageResult<()> {
    let mut rows = connection.query(sql, ()).await.map_err(map_turso_error)?;
    while rows.next().await.map_err(map_turso_error)?.is_some() {}
    Ok(())
}

fn text_value<T: ToString>(value: T) -> turso::Value {
    turso::Value::Text(value.to_string())
}

fn optional_text_value<T: ToString>(value: Option<T>) -> turso::Value {
    value.map_or(turso::Value::Null, text_value)
}

fn text_at(row: &turso::Row, index: usize) -> StorageResult<String> {
    row.get_value(index)
        .map_err(map_turso_error)?
        .as_text()
        .cloned()
        .ok_or_else(|| StorageError::Backend {
            message: format!("expected text column at index {index}"),
        })
}

fn optional_event_id_at(row: &turso::Row, index: usize) -> StorageResult<Option<EventId>> {
    let value = row.get_value(index).map_err(map_turso_error)?;
    value
        .as_text()
        .map(|value| EventId::new(value.clone()))
        .transpose()
}

fn integer_at(row: &turso::Row, index: usize) -> StorageResult<i64> {
    row.get_value(index)
        .map_err(map_turso_error)?
        .as_integer()
        .copied()
        .ok_or_else(|| StorageError::Backend {
            message: format!("expected integer column at index {index}"),
        })
}

fn u32_at(row: &turso::Row, index: usize) -> StorageResult<u32> {
    let value = integer_at(row, index)?;
    u32::try_from(value).map_err(|_| StorageError::Backend {
        message: format!("expected u32-compatible integer column at index {index}: {value}"),
    })
}

fn blob_at(row: &turso::Row, index: usize) -> StorageResult<Vec<u8>> {
    row.get_value(index)
        .map_err(map_turso_error)?
        .as_blob()
        .cloned()
        .ok_or_else(|| StorageError::Backend {
            message: format!("expected blob column at index {index}"),
        })
}
