// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Turso session event admission and ordered reads.

use super::turso;
use super::{
    AgentId, EventId, ProjectId, SessionEventRecord, SessionEventStore, SessionId, StorageError,
    StorageFuture, TurnId, TursoAgentStorage, execute_retrying, integer_at, map_turso_error,
    optional_event_id_at, optional_text_value, text_at, text_value,
};

impl SessionEventStore for TursoAgentStorage {
    fn latest_session_event_header<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a SessionId,
        from_turn: &'a TurnId,
        before_turn: &'a TurnId,
    ) -> StorageFuture<'a, Option<crate::SessionEventHeader>> {
        Box::pin(async move {
            let connection = self.connections.acquire().await?;
            let mut statement = connection
                .prepare_cached(
                    "SELECT agent_id, turn_id, event_id, event_kind, causality_parent_event_id
                 FROM session_events
                 WHERE project_id = ?1 AND session_id = ?2 AND turn_id >= ?3 AND turn_id < ?4
                 ORDER BY turn_id DESC, event_id DESC LIMIT 1",
                )
                .await
                .map_err(map_turso_error)?;
            let mut rows = statement
                .query(vec![
                    text_value(project.clone()),
                    text_value(session.clone()),
                    text_value(from_turn.clone()),
                    text_value(before_turn.clone()),
                ])
                .await
                .map_err(map_turso_error)?;
            rows.next()
                .await
                .map_err(map_turso_error)?
                .map(|row| {
                    Ok(crate::SessionEventHeader {
                        agent_id: AgentId::new(text_at(&row, 0)?)?,
                        cursor: crate::SessionEventCursor::new(
                            TurnId::new(text_at(&row, 1)?)?,
                            EventId::new(text_at(&row, 2)?)?,
                        ),
                        event_kind: text_at(&row, 3)?,
                        causality_parent_event_id: optional_event_id_at(&row, 4)?,
                    })
                })
                .transpose()
        })
    }

    fn count_session_events<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a SessionId,
    ) -> StorageFuture<'a, crate::records::SessionEventCount> {
        Box::pin(async move {
            let connection = self.connections.acquire().await?;
            let mut statement = connection
                .prepare_cached(
                    "SELECT COUNT(*) FROM session_events WHERE project_id = ?1 AND session_id = ?2",
                )
                .await
                .map_err(map_turso_error)?;
            let mut rows = statement
                .query(vec![
                    text_value(project.clone()),
                    text_value(session.clone()),
                ])
                .await
                .map_err(map_turso_error)?;
            let row = rows.next().await.map_err(map_turso_error)?.ok_or_else(|| {
                StorageError::Backend {
                    message: "session event count returned no row".into(),
                }
            })?;
            let count = u64::try_from(integer_at(&row, 0)?).map_err(|_| StorageError::Backend {
                message: "negative session event count".into(),
            })?;
            Ok(crate::records::SessionEventCount::new(count))
        })
    }

    fn list_sessions_page<'a>(
        &'a self,
        request: crate::records::SessionPageRequest,
    ) -> StorageFuture<'a, crate::records::StoragePage<SessionId, SessionId>> {
        Box::pin(async move {
            let connection = self.connections.acquire().await?;
            let (sql, params) = match request.cursor() {
                Some(cursor) => (
                    "SELECT DISTINCT session_id FROM session_events WHERE project_id = ?1 AND agent_id = ?2 AND session_id > ?3 ORDER BY session_id LIMIT ?4",
                    vec![
                        text_value(request.project_id().clone()),
                        text_value(request.agent_id().clone()),
                        text_value(cursor.clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
                None => (
                    "SELECT DISTINCT session_id FROM session_events WHERE project_id = ?1 AND agent_id = ?2 ORDER BY session_id LIMIT ?3",
                    vec![
                        text_value(request.project_id().clone()),
                        text_value(request.agent_id().clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                    ],
                ),
            };
            let mut statement = connection
                .prepare_cached(sql)
                .await
                .map_err(map_turso_error)?;
            let mut rows = statement.query(params).await.map_err(map_turso_error)?;
            let mut items = Vec::new();
            while let Some(row) = rows.next().await.map_err(map_turso_error)? {
                items.push(SessionId::new(text_at(&row, 0)?)?);
            }
            let next_cursor = if items.len() > request.limit().get() {
                items.pop();
                items.last().cloned()
            } else {
                None
            };
            Ok(crate::records::StoragePage { items, next_cursor })
        })
    }

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
            let turn_bound = request.turn_bound()?;
            let body_budget = request.body_budget();
            let connection = self.connections.acquire().await?;
            let (sql, params) = match (turn_bound, request.cursor(), body_budget) {
                (Some((cursor, budget)), _, _) => (
                    // Materialize only IDs and lengths for a limited index range.
                    // Expose bodies only for the prefix that fits the byte budget.
                    "WITH candidates AS MATERIALIZED (
                        SELECT rowid AS event_row, event_id, length(body) AS body_bytes
                        FROM session_events
                        WHERE project_id = ?1 AND session_id = ?2 AND turn_id = ?3
                          AND event_id > ?4
                        ORDER BY event_id LIMIT ?5
                    ), sizes AS (
                        SELECT event_row, event_id, row_number() OVER (ORDER BY event_id) AS ordinal,
                               sum(body_bytes) OVER (ORDER BY event_id ROWS UNBOUNDED PRECEDING)
                                   AS total_bytes
                        FROM candidates
                    )
                    SELECT e.project_id, e.session_id, e.agent_id, e.turn_id, e.event_id,
                           e.event_kind, e.causality_parent_event_id,
                           CASE WHEN s.ordinal < ?5 AND s.total_bytes <= ?6 THEN e.body END,
                           e.created_at_unix_ms
                    FROM sizes s JOIN session_events e ON e.rowid = s.event_row
                    ORDER BY s.event_id",
                    vec![
                        text_value(request.project_id().clone()),
                        text_value(request.session_id().clone()),
                        text_value(cursor.turn_id().clone()),
                        text_value(cursor.event_id().clone()),
                        turso::Value::Integer(request.limit().fetch_count()),
                        turso::Value::Integer(i64::from(budget.get())),
                    ],
                ),
                (None, cursor, Some(budget)) => {
                    let (turn, event) = match cursor {
                        Some(cursor) => (
                            text_value(cursor.turn_id().clone()),
                            text_value(cursor.event_id().clone()),
                        ),
                        // All stored identifiers are nonempty.
                        None => (turso::Value::Text(String::new()), turso::Value::Text(String::new())),
                    };
                    (
                        "WITH candidates AS MATERIALIZED (
                            SELECT rowid AS event_row, turn_id, event_id, length(body) AS body_bytes
                            FROM session_events
                            WHERE project_id = ?1 AND session_id = ?2
                              AND (turn_id, event_id) > (?3, ?4)
                            ORDER BY turn_id, event_id LIMIT ?5
                        ), sizes AS (
                            SELECT event_row, turn_id, event_id,
                                   row_number() OVER (ORDER BY turn_id, event_id) AS ordinal,
                                   sum(body_bytes) OVER (
                                       ORDER BY turn_id, event_id ROWS UNBOUNDED PRECEDING
                                   ) AS total_bytes
                            FROM candidates
                        )
                        SELECT e.project_id, e.session_id, e.agent_id, e.turn_id, e.event_id,
                               e.event_kind, e.causality_parent_event_id,
                               CASE WHEN s.ordinal < ?5 AND s.total_bytes <= ?6 THEN e.body END,
                               e.created_at_unix_ms
                        FROM sizes s JOIN session_events e ON e.rowid = s.event_row
                        ORDER BY s.turn_id, s.event_id",
                        vec![
                            text_value(request.project_id().clone()),
                            text_value(request.session_id().clone()),
                            turn,
                            event,
                            turso::Value::Integer(request.limit().fetch_count()),
                            turso::Value::Integer(i64::from(budget.get())),
                        ],
                    )
                }
                (None, Some(cursor), None) => (
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
                (None, None, None) => (
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
            let mut has_more = false;
            while let Some(row) = rows.next().await.map_err(map_turso_error)? {
                if records.len() == limit {
                    has_more = true;
                    break;
                }
                let body = match row.get_value(7).map_err(map_turso_error)? {
                    turso::Value::Blob(body) => body,
                    turso::Value::Null if let Some(budget) = body_budget => {
                        if records.is_empty() {
                            return Err(StorageError::SessionEventBodyExceedsPageBudget {
                                budget: budget.get(),
                            });
                        }
                        has_more = true;
                        break;
                    }
                    _ => {
                        return Err(StorageError::Backend {
                            message: "expected Session event body blob".into(),
                        });
                    }
                };
                records.push(SessionEventRecord {
                    project_id: ProjectId::new(text_at(&row, 0)?)?,
                    session_id: SessionId::new(text_at(&row, 1)?)?,
                    agent_id: AgentId::new(text_at(&row, 2)?)?,
                    turn_id: TurnId::new(text_at(&row, 3)?)?,
                    event_id: EventId::new(text_at(&row, 4)?)?,
                    event_kind: text_at(&row, 5)?,
                    causality_parent_event_id: optional_event_id_at(&row, 6)?,
                    body,
                    created_at_unix_ms: integer_at(&row, 8)?,
                });
            }
            let next_cursor = if has_more {
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
}
