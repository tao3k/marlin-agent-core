use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::records::{
    AgentId, AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerRecord,
    ArtifactPointerUpdate, ArtifactPutOutcome, ArtifactRecord, EventId, MemoryKey,
    MemoryProposalId, MemoryProposalRecord, ProjectId, SessionEventRecord, SessionId, StorageError,
    StorageFuture, StorageResult, StorageSchemaMigration, StorageSchemaSnapshot,
    StorageSchemaTable, TopologyEdgeId, TopologyEdgeRecord, TopologyNodeId, TurnId,
    VisibilityReceipt,
};

pub const STORAGE_SCHEMA_V1_MIGRATION_ID: &str = "storage-schema-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TursoAgentStorageConfig {
    pub path: PathBuf,
    pub mvcc: TursoMvccMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TursoMvccMode {
    Required,
    DisabledForCompatibility,
}

#[derive(Clone)]
pub struct TursoAgentStorage {
    database: turso::Database,
    config: TursoAgentStorageConfig,
}

impl TursoAgentStorage {
    pub async fn open_local(config: TursoAgentStorageConfig) -> StorageResult<Self> {
        let path = config.path.to_string_lossy().into_owned();
        let database = turso::Builder::new_local(&path)
            .build()
            .await
            .map_err(map_turso_error)?;
        let storage = Self { database, config };
        storage.bootstrap().await?;
        Ok(storage)
    }

    pub fn config(&self) -> &TursoAgentStorageConfig {
        &self.config
    }

    pub async fn schema_snapshot(&self) -> StorageResult<StorageSchemaSnapshot> {
        let connection = self.database.connect().map_err(map_turso_error)?;
        Ok(StorageSchemaSnapshot {
            migrations: list_schema_migrations(&connection).await?,
            tables: list_schema_tables(&connection).await?,
        })
    }

    async fn bootstrap(&self) -> StorageResult<()> {
        let connection = self.database.connect().map_err(map_turso_error)?;
        if self.config.mvcc == TursoMvccMode::Required {
            drain_rows(&connection, "PRAGMA journal_mode = 'mvcc'").await?;
        }

        for statement in STORAGE_SCHEMA {
            connection
                .execute(statement, ())
                .await
                .map_err(map_turso_error)?;
        }
        insert_schema_migration(&connection, STORAGE_SCHEMA_V1_MIGRATION_ID).await?;
        Ok(())
    }
}

impl AgentStorage for TursoAgentStorage {
    fn append_session_event<'a>(&'a self, record: SessionEventRecord) -> StorageFuture<'a, ()> {
        Box::pin(async move {
            let connection = self.database.connect().map_err(map_turso_error)?;
            let key = record.key();
            let rows_affected = connection
                .execute(
                    "INSERT OR IGNORE INTO session_events (
                        project_id,
                        session_id,
                        agent_id,
                        turn_id,
                        event_id,
                        event_kind,
                        causality_parent_event_id,
                        body,
                        created_at_unix_ms
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    vec![
                        text_value(record.project_id),
                        text_value(record.session_id),
                        text_value(record.agent_id),
                        text_value(record.turn_id),
                        text_value(record.event_id),
                        turso::Value::Text(record.event_kind),
                        optional_text_value(record.causality_parent_event_id),
                        turso::Value::Blob(record.body),
                        turso::Value::Integer(record.created_at_unix_ms),
                    ],
                )
                .await
                .map_err(map_turso_error)?;
            if rows_affected == 0 {
                return Err(StorageError::DuplicateSessionEvent { key });
            }
            Ok(())
        })
    }

    fn list_session_events<'a>(
        &'a self,
        project_id: &'a ProjectId,
        session_id: &'a SessionId,
    ) -> StorageFuture<'a, Vec<SessionEventRecord>> {
        Box::pin(async move {
            let connection = self.database.connect().map_err(map_turso_error)?;
            let mut rows = connection
                .query(
                    "SELECT
                        project_id,
                        session_id,
                        agent_id,
                        turn_id,
                        event_id,
                        event_kind,
                        causality_parent_event_id,
                        body,
                        created_at_unix_ms
                    FROM session_events
                    WHERE project_id = ?1 AND session_id = ?2
                    ORDER BY turn_id, event_id",
                    vec![
                        turso::Value::Text(project_id.as_str().to_string()),
                        turso::Value::Text(session_id.as_str().to_string()),
                    ],
                )
                .await
                .map_err(map_turso_error)?;

            let mut records = Vec::new();
            while let Some(row) = rows.next().await.map_err(map_turso_error)? {
                records.push(SessionEventRecord {
                    project_id: ProjectId::new(text_at(&row, 0)?)?,
                    session_id: SessionId::new(text_at(&row, 1)?)?,
                    agent_id: AgentId::new(text_at(&row, 2)?)?,
                    turn_id: TurnId::new(text_at(&row, 3)?)?,
                    event_id: EventId::new(text_at(&row, 4)?)?,
                    event_kind: text_at(&row, 5)?,
                    causality_parent_event_id: optional_event_id_at(&row, 6)?,
                    body: blob_at(&row, 7)?,
                    created_at_unix_ms: integer_at(&row, 8)?,
                });
            }
            Ok(records)
        })
    }

    fn put_artifact<'a>(&'a self, record: ArtifactRecord) -> StorageFuture<'a, ArtifactPutOutcome> {
        Box::pin(async move {
            let connection = self.database.connect().map_err(map_turso_error)?;
            let key_project = record.project_id.clone();
            let key_hash = record.artifact_hash.clone();
            let rows_affected = connection
                .execute(
                    "INSERT OR IGNORE INTO artifacts (
                        project_id,
                        artifact_hash,
                        artifact_kind,
                        producer_session_id,
                        producer_agent_id,
                        producer_event_id,
                        media_type,
                        size_bytes,
                        body,
                        created_at_unix_ms
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    vec![
                        text_value(record.project_id.clone()),
                        text_value(record.artifact_hash.clone()),
                        turso::Value::Text(record.artifact_kind.clone()),
                        text_value(record.producer_session_id.clone()),
                        text_value(record.producer_agent_id.clone()),
                        text_value(record.producer_event_id.clone()),
                        turso::Value::Text(record.media_type.clone()),
                        turso::Value::Integer(record.size_bytes as i64),
                        turso::Value::Blob(record.body.clone()),
                        turso::Value::Integer(record.created_at_unix_ms),
                    ],
                )
                .await
                .map_err(map_turso_error)?;

            if rows_affected > 0 {
                return Ok(ArtifactPutOutcome {
                    inserted: true,
                    artifact: record,
                });
            }

            let existing = self
                .get_artifact(&key_project, &key_hash)
                .await?
                .ok_or_else(|| StorageError::MissingArtifact {
                    project_id: key_project.clone(),
                    artifact_hash: key_hash.clone(),
                })?;

            if existing.body != record.body
                || existing.size_bytes != record.size_bytes
                || existing.media_type != record.media_type
            {
                return Err(StorageError::ArtifactHashCollision {
                    project_id: key_project,
                    artifact_hash: key_hash,
                });
            }

            Ok(ArtifactPutOutcome {
                inserted: false,
                artifact: existing,
            })
        })
    }

    fn get_artifact<'a>(
        &'a self,
        project_id: &'a ProjectId,
        artifact_hash: &'a ArtifactHash,
    ) -> StorageFuture<'a, Option<ArtifactRecord>> {
        Box::pin(async move {
            let connection = self.database.connect().map_err(map_turso_error)?;
            let mut rows = connection
                .query(
                    "SELECT
                        project_id,
                        artifact_hash,
                        artifact_kind,
                        producer_session_id,
                        producer_agent_id,
                        producer_event_id,
                        media_type,
                        size_bytes,
                        body,
                        created_at_unix_ms
                    FROM artifacts
                    WHERE project_id = ?1 AND artifact_hash = ?2",
                    vec![
                        turso::Value::Text(project_id.as_str().to_string()),
                        turso::Value::Text(artifact_hash.as_str().to_string()),
                    ],
                )
                .await
                .map_err(map_turso_error)?;

            let Some(row) = rows.next().await.map_err(map_turso_error)? else {
                return Ok(None);
            };

            Ok(Some(ArtifactRecord {
                project_id: ProjectId::new(text_at(&row, 0)?)?,
                artifact_hash: ArtifactHash::new(text_at(&row, 1)?)?,
                artifact_kind: text_at(&row, 2)?,
                producer_session_id: SessionId::new(text_at(&row, 3)?)?,
                producer_agent_id: AgentId::new(text_at(&row, 4)?)?,
                producer_event_id: EventId::new(text_at(&row, 5)?)?,
                media_type: text_at(&row, 6)?,
                size_bytes: integer_at(&row, 7)? as u64,
                body: blob_at(&row, 8)?,
                created_at_unix_ms: integer_at(&row, 9)?,
            }))
        })
    }

    fn compare_and_swap_artifact_pointer<'a>(
        &'a self,
        update: ArtifactPointerUpdate,
    ) -> StorageFuture<'a, ArtifactPointerRecord> {
        Box::pin(async move {
            if self
                .get_artifact(&update.project_id, &update.new_artifact_hash)
                .await?
                .is_none()
            {
                return Err(StorageError::MissingArtifact {
                    project_id: update.project_id,
                    artifact_hash: update.new_artifact_hash,
                });
            }

            let connection = self.database.connect().map_err(map_turso_error)?;
            let execute_outcome = match &update.expected_artifact_hash {
                None => {
                    execute_retrying(
                        &connection,
                        "INSERT OR IGNORE INTO artifact_pointers (
                            project_id,
                            pointer_key,
                            target_artifact_hash,
                            updated_by_session_id,
                            updated_by_agent_id,
                            updated_by_event_id,
                            updated_at_unix_ms
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        || {
                            vec![
                                text_value(update.project_id.clone()),
                                text_value(update.pointer_key.clone()),
                                text_value(update.new_artifact_hash.clone()),
                                text_value(update.updated_by_session_id.clone()),
                                text_value(update.updated_by_agent_id.clone()),
                                text_value(update.updated_by_event_id.clone()),
                                turso::Value::Integer(update.updated_at_unix_ms),
                            ]
                        },
                    )
                    .await?
                }
                Some(expected) => {
                    execute_retrying(
                        &connection,
                        "UPDATE artifact_pointers
                        SET
                            target_artifact_hash = ?3,
                            updated_by_session_id = ?4,
                            updated_by_agent_id = ?5,
                            updated_by_event_id = ?6,
                            updated_at_unix_ms = ?7
                        WHERE
                            project_id = ?1
                            AND pointer_key = ?2
                            AND target_artifact_hash = ?8",
                        || {
                            vec![
                                text_value(update.project_id.clone()),
                                text_value(update.pointer_key.clone()),
                                text_value(update.new_artifact_hash.clone()),
                                text_value(update.updated_by_session_id.clone()),
                                text_value(update.updated_by_agent_id.clone()),
                                text_value(update.updated_by_event_id.clone()),
                                turso::Value::Integer(update.updated_at_unix_ms),
                                text_value(expected.clone()),
                            ]
                        },
                    )
                    .await?
                }
            };
            let rows_affected = execute_outcome.rows_affected;

            if rows_affected == 0 {
                let actual = self
                    .get_artifact_pointer(&update.project_id, &update.pointer_key)
                    .await?
                    .map(|pointer| pointer.target_artifact_hash);
                insert_visibility_receipt(
                    &connection,
                    VisibilityReceipt {
                        project_id: update.project_id.clone(),
                        receipt_id: format!(
                            "turso:artifact-pointer-cas:{}:{}:conflict",
                            update.pointer_key, update.updated_by_event_id
                        ),
                        receipt_kind: "storage.turso.transaction".to_string(),
                        body: transaction_receipt_body(
                            "artifact_pointer.cas",
                            "conflict",
                            execute_outcome.retry_count,
                            rows_affected,
                        ),
                        created_at_unix_ms: update.updated_at_unix_ms,
                    },
                )
                .await?;
                return Err(StorageError::ArtifactPointerConflict {
                    project_id: update.project_id,
                    pointer_key: update.pointer_key,
                    expected: update.expected_artifact_hash,
                    actual,
                });
            }

            insert_visibility_receipt(
                &connection,
                VisibilityReceipt {
                    project_id: update.project_id.clone(),
                    receipt_id: format!(
                        "turso:artifact-pointer-cas:{}:{}:committed",
                        update.pointer_key, update.updated_by_event_id
                    ),
                    receipt_kind: "storage.turso.transaction".to_string(),
                    body: transaction_receipt_body(
                        "artifact_pointer.cas",
                        "committed",
                        execute_outcome.retry_count,
                        rows_affected,
                    ),
                    created_at_unix_ms: update.updated_at_unix_ms,
                },
            )
            .await?;

            Ok(ArtifactPointerRecord {
                project_id: update.project_id,
                pointer_key: update.pointer_key,
                target_artifact_hash: update.new_artifact_hash,
                updated_by_session_id: update.updated_by_session_id,
                updated_by_agent_id: update.updated_by_agent_id,
                updated_by_event_id: update.updated_by_event_id,
                updated_at_unix_ms: update.updated_at_unix_ms,
            })
        })
    }

    fn get_artifact_pointer<'a>(
        &'a self,
        project_id: &'a ProjectId,
        pointer_key: &'a ArtifactPointerKey,
    ) -> StorageFuture<'a, Option<ArtifactPointerRecord>> {
        Box::pin(async move {
            let connection = self.database.connect().map_err(map_turso_error)?;
            let mut rows = connection
                .query(
                    "SELECT
                        project_id,
                        pointer_key,
                        target_artifact_hash,
                        updated_by_session_id,
                        updated_by_agent_id,
                        updated_by_event_id,
                        updated_at_unix_ms
                    FROM artifact_pointers
                    WHERE project_id = ?1 AND pointer_key = ?2",
                    vec![
                        turso::Value::Text(project_id.as_str().to_string()),
                        turso::Value::Text(pointer_key.as_str().to_string()),
                    ],
                )
                .await
                .map_err(map_turso_error)?;

            let Some(row) = rows.next().await.map_err(map_turso_error)? else {
                return Ok(None);
            };

            Ok(Some(ArtifactPointerRecord {
                project_id: ProjectId::new(text_at(&row, 0)?)?,
                pointer_key: ArtifactPointerKey::new(text_at(&row, 1)?)?,
                target_artifact_hash: ArtifactHash::new(text_at(&row, 2)?)?,
                updated_by_session_id: SessionId::new(text_at(&row, 3)?)?,
                updated_by_agent_id: AgentId::new(text_at(&row, 4)?)?,
                updated_by_event_id: EventId::new(text_at(&row, 5)?)?,
                updated_at_unix_ms: integer_at(&row, 6)?,
            }))
        })
    }

    fn record_visibility<'a>(&'a self, receipt: VisibilityReceipt) -> StorageFuture<'a, ()> {
        Box::pin(async move {
            let connection = self.database.connect().map_err(map_turso_error)?;
            insert_visibility_receipt(&connection, receipt).await
        })
    }

    fn list_visibility<'a>(
        &'a self,
        project_id: &'a ProjectId,
    ) -> StorageFuture<'a, Vec<VisibilityReceipt>> {
        Box::pin(async move {
            let connection = self.database.connect().map_err(map_turso_error)?;
            let mut rows = connection
                .query(
                    "SELECT project_id, receipt_id, receipt_kind, body, created_at_unix_ms
                    FROM visibility_receipts
                    WHERE project_id = ?1
                    ORDER BY created_at_unix_ms, receipt_id",
                    vec![turso::Value::Text(project_id.as_str().to_string())],
                )
                .await
                .map_err(map_turso_error)?;

            let mut receipts = Vec::new();
            while let Some(row) = rows.next().await.map_err(map_turso_error)? {
                receipts.push(VisibilityReceipt {
                    project_id: ProjectId::new(text_at(&row, 0)?)?,
                    receipt_id: text_at(&row, 1)?,
                    receipt_kind: text_at(&row, 2)?,
                    body: blob_at(&row, 3)?,
                    created_at_unix_ms: integer_at(&row, 4)?,
                });
            }
            Ok(receipts)
        })
    }

    fn put_memory_proposal<'a>(&'a self, record: MemoryProposalRecord) -> StorageFuture<'a, ()> {
        Box::pin(async move {
            let source_project_id = record.project_id.clone();
            let source_artifact_hash = record.source_artifact_hash.clone();
            if self
                .get_artifact(&source_project_id, &source_artifact_hash)
                .await?
                .is_none()
            {
                return Err(StorageError::MissingArtifact {
                    project_id: source_project_id,
                    artifact_hash: source_artifact_hash,
                });
            }

            let connection = self.database.connect().map_err(map_turso_error)?;
            let rows_affected = connection
                .execute(
                    "INSERT OR IGNORE INTO memory_proposals (
                        project_id,
                        proposal_id,
                        memory_key,
                        proposal_status,
                        source_artifact_hash,
                        source_session_id,
                        source_agent_id,
                        source_event_id,
                        org_source_path,
                        org_source_begin,
                        org_source_end,
                        memory_kind,
                        body,
                        created_at_unix_ms
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    vec![
                        text_value(record.project_id.clone()),
                        text_value(record.proposal_id.clone()),
                        text_value(record.memory_key),
                        turso::Value::Text(record.proposal_status),
                        text_value(record.source_artifact_hash),
                        text_value(record.source_session_id),
                        text_value(record.source_agent_id),
                        text_value(record.source_event_id),
                        turso::Value::Text(record.org_source_path),
                        turso::Value::Integer(record.org_source_begin as i64),
                        turso::Value::Integer(record.org_source_end as i64),
                        turso::Value::Text(record.memory_kind),
                        turso::Value::Blob(record.body),
                        turso::Value::Integer(record.created_at_unix_ms),
                    ],
                )
                .await
                .map_err(map_turso_error)?;
            if rows_affected == 0 {
                return Err(StorageError::DuplicateMemoryProposal {
                    project_id: record.project_id,
                    proposal_id: record.proposal_id,
                });
            }
            Ok(())
        })
    }

    fn list_memory_proposals<'a>(
        &'a self,
        project_id: &'a ProjectId,
        memory_key: Option<&'a MemoryKey>,
    ) -> StorageFuture<'a, Vec<MemoryProposalRecord>> {
        Box::pin(async move {
            let connection = self.database.connect().map_err(map_turso_error)?;
            let (sql, params) = match memory_key {
                Some(memory_key) => (
                    "SELECT
                        project_id,
                        proposal_id,
                        memory_key,
                        proposal_status,
                        source_artifact_hash,
                        source_session_id,
                        source_agent_id,
                        source_event_id,
                        org_source_path,
                        org_source_begin,
                        org_source_end,
                        memory_kind,
                        body,
                        created_at_unix_ms
                    FROM memory_proposals
                    WHERE project_id = ?1 AND memory_key = ?2
                    ORDER BY created_at_unix_ms, proposal_id",
                    vec![
                        turso::Value::Text(project_id.as_str().to_string()),
                        turso::Value::Text(memory_key.as_str().to_string()),
                    ],
                ),
                None => (
                    "SELECT
                        project_id,
                        proposal_id,
                        memory_key,
                        proposal_status,
                        source_artifact_hash,
                        source_session_id,
                        source_agent_id,
                        source_event_id,
                        org_source_path,
                        org_source_begin,
                        org_source_end,
                        memory_kind,
                        body,
                        created_at_unix_ms
                    FROM memory_proposals
                    WHERE project_id = ?1
                    ORDER BY created_at_unix_ms, proposal_id",
                    vec![turso::Value::Text(project_id.as_str().to_string())],
                ),
            };
            let mut rows = connection
                .query(sql, params)
                .await
                .map_err(map_turso_error)?;

            let mut proposals = Vec::new();
            while let Some(row) = rows.next().await.map_err(map_turso_error)? {
                proposals.push(MemoryProposalRecord {
                    project_id: ProjectId::new(text_at(&row, 0)?)?,
                    proposal_id: MemoryProposalId::new(text_at(&row, 1)?)?,
                    memory_key: MemoryKey::new(text_at(&row, 2)?)?,
                    proposal_status: text_at(&row, 3)?,
                    source_artifact_hash: ArtifactHash::new(text_at(&row, 4)?)?,
                    source_session_id: SessionId::new(text_at(&row, 5)?)?,
                    source_agent_id: AgentId::new(text_at(&row, 6)?)?,
                    source_event_id: EventId::new(text_at(&row, 7)?)?,
                    org_source_path: text_at(&row, 8)?,
                    org_source_begin: u32_at(&row, 9)?,
                    org_source_end: u32_at(&row, 10)?,
                    memory_kind: text_at(&row, 11)?,
                    body: blob_at(&row, 12)?,
                    created_at_unix_ms: integer_at(&row, 13)?,
                });
            }
            Ok(proposals)
        })
    }

    fn append_topology_edge<'a>(&'a self, record: TopologyEdgeRecord) -> StorageFuture<'a, ()> {
        Box::pin(async move {
            let connection = self.database.connect().map_err(map_turso_error)?;
            let rows_affected = connection
                .execute(
                    "INSERT OR IGNORE INTO topology_edges (
                        project_id,
                        edge_id,
                        from_node_id,
                        to_node_id,
                        edge_kind,
                        source_session_id,
                        source_agent_id,
                        source_event_id,
                        body,
                        created_at_unix_ms
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    vec![
                        text_value(record.project_id.clone()),
                        text_value(record.edge_id.clone()),
                        text_value(record.from_node_id),
                        text_value(record.to_node_id),
                        turso::Value::Text(record.edge_kind),
                        text_value(record.source_session_id),
                        text_value(record.source_agent_id),
                        text_value(record.source_event_id),
                        turso::Value::Blob(record.body),
                        turso::Value::Integer(record.created_at_unix_ms),
                    ],
                )
                .await
                .map_err(map_turso_error)?;
            if rows_affected == 0 {
                return Err(StorageError::DuplicateTopologyEdge {
                    project_id: record.project_id,
                    edge_id: record.edge_id,
                });
            }
            Ok(())
        })
    }

    fn list_topology_edges<'a>(
        &'a self,
        project_id: &'a ProjectId,
    ) -> StorageFuture<'a, Vec<TopologyEdgeRecord>> {
        Box::pin(async move {
            let connection = self.database.connect().map_err(map_turso_error)?;
            let mut rows = connection
                .query(
                    "SELECT
                        project_id,
                        edge_id,
                        from_node_id,
                        to_node_id,
                        edge_kind,
                        source_session_id,
                        source_agent_id,
                        source_event_id,
                        body,
                        created_at_unix_ms
                    FROM topology_edges
                    WHERE project_id = ?1
                    ORDER BY created_at_unix_ms, edge_id",
                    vec![turso::Value::Text(project_id.as_str().to_string())],
                )
                .await
                .map_err(map_turso_error)?;

            let mut edges = Vec::new();
            while let Some(row) = rows.next().await.map_err(map_turso_error)? {
                edges.push(TopologyEdgeRecord {
                    project_id: ProjectId::new(text_at(&row, 0)?)?,
                    edge_id: TopologyEdgeId::new(text_at(&row, 1)?)?,
                    from_node_id: TopologyNodeId::new(text_at(&row, 2)?)?,
                    to_node_id: TopologyNodeId::new(text_at(&row, 3)?)?,
                    edge_kind: text_at(&row, 4)?,
                    source_session_id: SessionId::new(text_at(&row, 5)?)?,
                    source_agent_id: AgentId::new(text_at(&row, 6)?)?,
                    source_event_id: EventId::new(text_at(&row, 7)?)?,
                    body: blob_at(&row, 8)?,
                    created_at_unix_ms: integer_at(&row, 9)?,
                });
            }
            Ok(edges)
        })
    }
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

async fn execute_retrying(
    connection: &turso::Connection,
    sql: &str,
    mut params: impl FnMut() -> Vec<turso::Value>,
) -> StorageResult<ExecuteRetryOutcome> {
    const MAX_ATTEMPTS: u32 = 8;
    let mut retry_count = 0;

    for attempt in 0..MAX_ATTEMPTS {
        match connection.execute(sql, params()).await {
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

async fn insert_schema_migration(
    connection: &turso::Connection,
    migration_id: &str,
) -> StorageResult<()> {
    connection
        .execute(
            "INSERT OR IGNORE INTO storage_schema_migrations (
                migration_id,
                applied_at_unix_ms
            ) VALUES (?1, ?2)",
            vec![
                turso::Value::Text(migration_id.to_string()),
                turso::Value::Integer(current_unix_ms()?),
            ],
        )
        .await
        .map_err(map_turso_error)?;
    Ok(())
}

async fn list_schema_migrations(
    connection: &turso::Connection,
) -> StorageResult<Vec<StorageSchemaMigration>> {
    let mut rows = connection
        .query(
            "SELECT migration_id, applied_at_unix_ms
            FROM storage_schema_migrations
            ORDER BY migration_id",
            (),
        )
        .await
        .map_err(map_turso_error)?;

    let mut migrations = Vec::new();
    while let Some(row) = rows.next().await.map_err(map_turso_error)? {
        migrations.push(StorageSchemaMigration {
            migration_id: text_at(&row, 0)?,
            applied_at_unix_ms: integer_at(&row, 1)?,
        });
    }
    Ok(migrations)
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

fn transaction_receipt_body(
    operation: &str,
    status: &str,
    retry_count: u32,
    rows_affected: u64,
) -> Vec<u8> {
    format!(
        "operation={operation}\nstatus={status}\nretry_count={retry_count}\nrows_affected={rows_affected}\n"
    )
    .into_bytes()
}

fn current_unix_ms() -> StorageResult<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| StorageError::Backend {
            message: format!("system time before unix epoch: {error}"),
        })?;
    Ok(duration.as_millis() as i64)
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

const STORAGE_SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS storage_schema_migrations (
        migration_id TEXT PRIMARY KEY,
        applied_at_unix_ms INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS session_events (
        project_id TEXT NOT NULL,
        session_id TEXT NOT NULL,
        agent_id TEXT NOT NULL,
        turn_id TEXT NOT NULL,
        event_id TEXT NOT NULL,
        event_kind TEXT NOT NULL,
        causality_parent_event_id TEXT,
        body BLOB NOT NULL,
        created_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, session_id, turn_id, event_id)
    )",
    "CREATE TABLE IF NOT EXISTS artifacts (
        project_id TEXT NOT NULL,
        artifact_hash TEXT NOT NULL,
        artifact_kind TEXT NOT NULL,
        producer_session_id TEXT NOT NULL,
        producer_agent_id TEXT NOT NULL,
        producer_event_id TEXT NOT NULL,
        media_type TEXT NOT NULL,
        size_bytes INTEGER NOT NULL,
        body BLOB NOT NULL,
        created_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, artifact_hash)
    )",
    "CREATE TABLE IF NOT EXISTS artifact_pointers (
        project_id TEXT NOT NULL,
        pointer_key TEXT NOT NULL,
        target_artifact_hash TEXT NOT NULL,
        updated_by_session_id TEXT NOT NULL,
        updated_by_agent_id TEXT NOT NULL,
        updated_by_event_id TEXT NOT NULL,
        updated_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, pointer_key)
    )",
    "CREATE TABLE IF NOT EXISTS visibility_receipts (
        project_id TEXT NOT NULL,
        receipt_id TEXT NOT NULL,
        receipt_kind TEXT NOT NULL,
        body BLOB NOT NULL,
        created_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, receipt_id)
    )",
    "CREATE TABLE IF NOT EXISTS memory_proposals (
        project_id TEXT NOT NULL,
        proposal_id TEXT NOT NULL,
        memory_key TEXT NOT NULL,
        proposal_status TEXT NOT NULL,
        source_artifact_hash TEXT NOT NULL,
        source_session_id TEXT NOT NULL,
        source_agent_id TEXT NOT NULL,
        source_event_id TEXT NOT NULL,
        org_source_path TEXT NOT NULL,
        org_source_begin INTEGER NOT NULL,
        org_source_end INTEGER NOT NULL,
        memory_kind TEXT NOT NULL,
        body BLOB NOT NULL,
        created_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, proposal_id)
    )",
    "CREATE INDEX IF NOT EXISTS memory_proposals_by_key
        ON memory_proposals (project_id, memory_key, created_at_unix_ms, proposal_id)",
    "CREATE TABLE IF NOT EXISTS topology_edges (
        project_id TEXT NOT NULL,
        edge_id TEXT NOT NULL,
        from_node_id TEXT NOT NULL,
        to_node_id TEXT NOT NULL,
        edge_kind TEXT NOT NULL,
        source_session_id TEXT NOT NULL,
        source_agent_id TEXT NOT NULL,
        source_event_id TEXT NOT NULL,
        body BLOB NOT NULL,
        created_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY (project_id, edge_id)
    )",
    "CREATE INDEX IF NOT EXISTS topology_edges_by_from_node
        ON topology_edges (project_id, from_node_id, created_at_unix_ms, edge_id)",
];
