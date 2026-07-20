use super::{
    AgentId, AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerRecord,
    ArtifactPointerUpdate, ArtifactPutOutcome, ArtifactRecord, EventId, MemoryKey,
    MemoryProposalId, MemoryProposalRecord, ProjectId, SessionEventRecord, SessionId, StorageError,
    StorageFuture, TopologyEdgeId, TopologyEdgeRecord, TopologyNodeId, TurnId, TursoAgentStorage,
    TursoTransactionOperation, TursoTransactionReceipt, TursoTransactionStatus, VisibilityReceipt,
    blob_at, execute_retrying, insert_visibility_receipt, integer_at, map_turso_error,
    optional_event_id_at, optional_text_value, text_at, text_value, u32_at,
};

impl AgentStorage for TursoAgentStorage {
    fn append_session_event<'a>(&'a self, record: SessionEventRecord) -> StorageFuture<'a, ()> {
        Box::pin(async move {
            let connection = self.connections.acquire().await?;
            let key = record.key();
            let execute_outcome = execute_retrying(
                &connection,
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
                || {
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
                },
            )
            .await?;
            if execute_outcome.rows_affected == 0 {
                return Err(StorageError::DuplicateSessionEvent { key });
            }
            Ok(())
        })
    }

    fn append_session_events_atomically<'a>(
        &'a self,
        records: Vec<SessionEventRecord>,
    ) -> StorageFuture<'a, crate::records::SessionEventBatchWriteReceipt> {
        Box::pin(async move {
            let receipt = TursoAgentStorage::append_session_events_batch(self, records).await?;
            Ok(crate::records::SessionEventBatchWriteReceipt {
                item_count: receipt.item_count,
                rows_affected: receipt.rows_affected,
            })
        })
    }

    fn list_session_events_page<'a>(
        &'a self,
        request: crate::records::SessionEventPageRequest,
    ) -> StorageFuture<
        'a,
        crate::records::StoragePage<SessionEventRecord, crate::records::SessionEventCursor>,
    > {
        Box::pin(async move {
            let limit = request.limit().get();
            let connection = self.connections.acquire().await?;
            let (sql, params) = match request.cursor() {
                Some(cursor) => (
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
                      AND (turn_id > ?3 OR (turn_id = ?3 AND event_id > ?4))
                    ORDER BY turn_id, event_id
                    LIMIT ?5",
                    vec![
                        text_value(request.project_id().clone()),
                        text_value(request.session_id().clone()),
                        text_value(cursor.turn_id().clone()),
                        text_value(cursor.event_id().clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
                None => (
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
                    ORDER BY turn_id, event_id
                    LIMIT ?3",
                    vec![
                        text_value(request.project_id().clone()),
                        text_value(request.session_id().clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
            };
            let mut statement = connection
                .prepare_cached(sql)
                .await
                .map_err(map_turso_error)?;
            let mut rows = statement.query(params).await.map_err(map_turso_error)?;

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
            let next_cursor = if records.len() > limit {
                records.pop();
                records
                    .last()
                    .map(crate::records::SessionEventCursor::from_record)
            } else {
                None
            };
            Ok(crate::records::StoragePage {
                items: records,
                next_cursor,
            })
        })
    }

    fn put_artifact<'a>(&'a self, record: ArtifactRecord) -> StorageFuture<'a, ArtifactPutOutcome> {
        Box::pin(async move {
            let connection = self.connections.acquire().await?;
            let key_project = record.project_id.clone();
            let key_hash = record.artifact_hash.clone();
            let execute_outcome = execute_retrying(
                &connection,
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
                || {
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
                    ]
                },
            )
            .await?;

            if execute_outcome.rows_affected > 0 {
                return Ok(ArtifactPutOutcome {
                    inserted: true,
                    artifact: record,
                });
            }

            drop(connection);
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
            let connection = self.connections.acquire().await?;
            let mut statement = connection
                .prepare_cached(
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
                )
                .await
                .map_err(map_turso_error)?;
            let mut rows = statement
                .query(vec![
                    turso::Value::Text(project_id.as_str().to_string()),
                    turso::Value::Text(artifact_hash.as_str().to_string()),
                ])
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

            let connection = self.connections.acquire().await?;
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
                drop(connection);
                let actual = self
                    .get_artifact_pointer(&update.project_id, &update.pointer_key)
                    .await?
                    .map(|pointer| pointer.target_artifact_hash);
                let visibility_connection = self.connections.acquire().await?;
                insert_visibility_receipt(
                    &visibility_connection,
                    VisibilityReceipt {
                        project_id: update.project_id.clone(),
                        receipt_id: format!(
                            "turso:artifact-pointer-cas:{}:{}:conflict",
                            update.pointer_key, update.updated_by_event_id
                        ),
                        receipt_kind: "storage.turso.transaction".to_string(),
                        body: TursoTransactionReceipt {
                            operation: TursoTransactionOperation::ArtifactPointerCompareAndSwap,
                            status: TursoTransactionStatus::Conflict,
                            retry_count: execute_outcome.retry_count,
                            rows_affected,
                        }
                        .visibility_body(),
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
                    body: TursoTransactionReceipt {
                        operation: TursoTransactionOperation::ArtifactPointerCompareAndSwap,
                        status: TursoTransactionStatus::Committed,
                        retry_count: execute_outcome.retry_count,
                        rows_affected,
                    }
                    .visibility_body(),
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
            let connection = self.connections.acquire().await?;
            let mut statement = connection
                .prepare_cached(
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
                )
                .await
                .map_err(map_turso_error)?;
            let mut rows = statement
                .query(vec![
                    turso::Value::Text(project_id.as_str().to_string()),
                    turso::Value::Text(pointer_key.as_str().to_string()),
                ])
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
            let connection = self.connections.acquire().await?;
            insert_visibility_receipt(&connection, receipt).await
        })
    }

    fn list_visibility_page<'a>(
        &'a self,
        request: crate::records::VisibilityPageRequest,
    ) -> StorageFuture<
        'a,
        crate::records::StoragePage<VisibilityReceipt, crate::records::VisibilityCursor>,
    > {
        Box::pin(async move {
            let limit = request.limit().get();
            let connection = self.connections.acquire().await?;
            let (sql, params) = match request.cursor() {
                Some(cursor) => (
                    "SELECT project_id, receipt_id, receipt_kind, body, created_at_unix_ms
                    FROM visibility_receipts
                    WHERE project_id = ?1
                      AND (created_at_unix_ms > ?2
                           OR (created_at_unix_ms = ?2 AND receipt_id > ?3))
                    ORDER BY created_at_unix_ms, receipt_id
                    LIMIT ?4",
                    vec![
                        text_value(request.project_id().clone()),
                        turso::Value::Integer(cursor.created_at_unix_ms()),
                        turso::Value::Text(cursor.receipt_id().to_string()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
                None => (
                    "SELECT project_id, receipt_id, receipt_kind, body, created_at_unix_ms
                    FROM visibility_receipts
                    WHERE project_id = ?1
                    ORDER BY created_at_unix_ms, receipt_id
                    LIMIT ?2",
                    vec![
                        text_value(request.project_id().clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
            };
            let mut statement = connection
                .prepare_cached(sql)
                .await
                .map_err(map_turso_error)?;
            let mut rows = statement.query(params).await.map_err(map_turso_error)?;

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
            let next_cursor = if receipts.len() > limit {
                receipts.pop();
                receipts
                    .last()
                    .map(crate::records::VisibilityCursor::from_record)
            } else {
                None
            };
            Ok(crate::records::StoragePage {
                items: receipts,
                next_cursor,
            })
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

            let connection = self.connections.acquire().await?;
            let execute_outcome = execute_retrying(
                &connection,
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
                || {
                    vec![
                        text_value(record.project_id.clone()),
                        text_value(record.proposal_id.clone()),
                        text_value(record.memory_key.clone()),
                        turso::Value::Text(record.proposal_status.clone()),
                        text_value(record.source_artifact_hash.clone()),
                        text_value(record.source_session_id.clone()),
                        text_value(record.source_agent_id.clone()),
                        text_value(record.source_event_id.clone()),
                        turso::Value::Text(record.org_source_path.clone()),
                        turso::Value::Integer(record.org_source_begin as i64),
                        turso::Value::Integer(record.org_source_end as i64),
                        turso::Value::Text(record.memory_kind.clone()),
                        turso::Value::Blob(record.body.clone()),
                        turso::Value::Integer(record.created_at_unix_ms),
                    ]
                },
            )
            .await?;
            if execute_outcome.rows_affected == 0 {
                return Err(StorageError::DuplicateMemoryProposal {
                    project_id: record.project_id,
                    proposal_id: record.proposal_id,
                });
            }
            Ok(())
        })
    }

    fn list_memory_proposals_page<'a>(
        &'a self,
        request: crate::records::MemoryProposalPageRequest,
    ) -> StorageFuture<
        'a,
        crate::records::StoragePage<MemoryProposalRecord, crate::records::MemoryProposalCursor>,
    > {
        Box::pin(async move {
            let limit = request.limit().get();
            let connection = self.connections.acquire().await?;
            let select = "SELECT
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
                    FROM memory_proposals";
            let (sql, params) = match (request.memory_key(), request.cursor()) {
                (Some(memory_key), Some(cursor)) => (
                    format!(
                        "{select}
                        WHERE project_id = ?1 AND memory_key = ?2
                          AND (created_at_unix_ms > ?3
                               OR (created_at_unix_ms = ?3 AND proposal_id > ?4))
                        ORDER BY created_at_unix_ms, proposal_id
                        LIMIT ?5"
                    ),
                    vec![
                        text_value(request.project_id().clone()),
                        text_value(memory_key.clone()),
                        turso::Value::Integer(cursor.created_at_unix_ms()),
                        text_value(cursor.proposal_id().clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
                (Some(memory_key), None) => (
                    format!(
                        "{select}
                        WHERE project_id = ?1 AND memory_key = ?2
                        ORDER BY created_at_unix_ms, proposal_id
                        LIMIT ?3"
                    ),
                    vec![
                        text_value(request.project_id().clone()),
                        text_value(memory_key.clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
                (None, Some(cursor)) => (
                    format!(
                        "{select}
                        WHERE project_id = ?1
                          AND (created_at_unix_ms > ?2
                               OR (created_at_unix_ms = ?2 AND proposal_id > ?3))
                        ORDER BY created_at_unix_ms, proposal_id
                        LIMIT ?4"
                    ),
                    vec![
                        text_value(request.project_id().clone()),
                        turso::Value::Integer(cursor.created_at_unix_ms()),
                        text_value(cursor.proposal_id().clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
                (None, None) => (
                    format!(
                        "{select}
                        WHERE project_id = ?1
                        ORDER BY created_at_unix_ms, proposal_id
                        LIMIT ?2"
                    ),
                    vec![
                        text_value(request.project_id().clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
            };
            let mut statement = connection
                .prepare_cached(&sql)
                .await
                .map_err(map_turso_error)?;
            let mut rows = statement.query(params).await.map_err(map_turso_error)?;

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
            let next_cursor = if proposals.len() > limit {
                proposals.pop();
                proposals
                    .last()
                    .map(crate::records::MemoryProposalCursor::from_record)
            } else {
                None
            };
            Ok(crate::records::StoragePage {
                items: proposals,
                next_cursor,
            })
        })
    }

    fn append_topology_edge<'a>(&'a self, record: TopologyEdgeRecord) -> StorageFuture<'a, ()> {
        Box::pin(async move {
            let connection = self.connections.acquire().await?;
            let execute_outcome = execute_retrying(
                &connection,
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
                || {
                    vec![
                        text_value(record.project_id.clone()),
                        text_value(record.edge_id.clone()),
                        text_value(record.from_node_id.clone()),
                        text_value(record.to_node_id.clone()),
                        turso::Value::Text(record.edge_kind.clone()),
                        text_value(record.source_session_id.clone()),
                        text_value(record.source_agent_id.clone()),
                        text_value(record.source_event_id.clone()),
                        turso::Value::Blob(record.body.clone()),
                        turso::Value::Integer(record.created_at_unix_ms),
                    ]
                },
            )
            .await?;
            if execute_outcome.rows_affected == 0 {
                return Err(StorageError::DuplicateTopologyEdge {
                    project_id: record.project_id,
                    edge_id: record.edge_id,
                });
            }
            Ok(())
        })
    }

    fn list_topology_edges_page<'a>(
        &'a self,
        request: crate::records::TopologyEdgePageRequest,
    ) -> StorageFuture<
        'a,
        crate::records::StoragePage<TopologyEdgeRecord, crate::records::TopologyEdgeCursor>,
    > {
        Box::pin(async move {
            let limit = request.limit().get();
            let connection = self.connections.acquire().await?;
            let (sql, params) = match request.cursor() {
                Some(cursor) => (
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
                      AND (created_at_unix_ms > ?2
                           OR (created_at_unix_ms = ?2 AND edge_id > ?3))
                    ORDER BY created_at_unix_ms, edge_id
                    LIMIT ?4",
                    vec![
                        text_value(request.project_id().clone()),
                        turso::Value::Integer(cursor.created_at_unix_ms()),
                        text_value(cursor.edge_id().clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
                None => (
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
                    ORDER BY created_at_unix_ms, edge_id
                    LIMIT ?2",
                    vec![
                        text_value(request.project_id().clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
            };
            let mut statement = connection
                .prepare_cached(sql)
                .await
                .map_err(map_turso_error)?;
            let mut rows = statement.query(params).await.map_err(map_turso_error)?;

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
            let next_cursor = if edges.len() > limit {
                edges.pop();
                edges
                    .last()
                    .map(crate::records::TopologyEdgeCursor::from_record)
            } else {
                None
            };
            Ok(crate::records::StoragePage {
                items: edges,
                next_cursor,
            })
        })
    }
}
