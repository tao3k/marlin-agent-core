// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::collections::{BTreeMap, BTreeSet};
use std::future::ready;
use std::sync::{Arc, Mutex};

use crate::records::{
    AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerRecord, ArtifactPointerUpdate,
    ArtifactPutOutcome, ArtifactRecord, MemoryProposalId, MemoryProposalRecord, ProjectId,
    SessionEventKey, SessionEventRecord, SessionEventStore, StorageError, StorageFuture,
    StorageResult, TopologyEdgeId, TopologyEdgeRecord, VisibilityReceipt,
};

#[derive(Clone, Debug, Default)]
pub struct InMemoryAgentStorage {
    state: Arc<Mutex<State>>,
}

#[derive(Clone, Debug, Default)]
struct State {
    session_events: BTreeMap<SessionEventKey, SessionEventRecord>,
    session_event_counts: BTreeMap<(ProjectId, crate::records::SessionId), u64>,
    sessions: BTreeSet<(
        ProjectId,
        crate::records::AgentId,
        crate::records::SessionId,
    )>,
    artifacts: BTreeMap<(ProjectId, ArtifactHash), ArtifactRecord>,
    artifact_pointers: BTreeMap<(ProjectId, ArtifactPointerKey), ArtifactPointerRecord>,
    memory_proposals: BTreeMap<(ProjectId, MemoryProposalId), MemoryProposalRecord>,
    topology_edges: BTreeMap<(ProjectId, TopologyEdgeId), TopologyEdgeRecord>,
    visibility: Vec<VisibilityReceipt>,
}

impl InMemoryAgentStorage {
    pub fn new() -> Self {
        Self::default()
    }

    fn with_state<T>(&self, f: impl FnOnce(&mut State) -> StorageResult<T>) -> StorageResult<T> {
        let mut state = self.state.lock().map_err(|_| StorageError::Backend {
            message: "in-memory storage mutex poisoned".to_string(),
        })?;
        f(&mut state)
    }
}

impl SessionEventStore for InMemoryAgentStorage {
    fn latest_session_event_header<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a crate::SessionId,
        from_turn: &'a crate::TurnId,
        before_turn: &'a crate::TurnId,
    ) -> StorageFuture<'a, Option<crate::SessionEventHeader>> {
        Box::pin(ready(self.with_state(|state| {
            if from_turn >= before_turn {
                return Ok(None);
            }
            let floor = crate::EventId::new("\0")?;
            let key = |turn_id: &crate::TurnId| SessionEventKey {
                project_id: project.clone(),
                session_id: session.clone(),
                turn_id: turn_id.clone(),
                event_id: floor.clone(),
            };
            Ok(state
                .session_events
                .range(key(from_turn)..key(before_turn))
                .next_back()
                .map(|(_, record)| crate::SessionEventHeader {
                    agent_id: record.agent_id.clone(),
                    cursor: crate::SessionEventCursor::from_record(record),
                    event_kind: record.event_kind.clone(),
                    causality_parent_event_id: record.causality_parent_event_id.clone(),
                }))
        })))
    }

    fn count_session_events<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a crate::records::SessionId,
    ) -> StorageFuture<'a, crate::records::SessionEventCount> {
        Box::pin(ready(self.with_state(|state| {
            let count = state
                .session_event_counts
                .get(&(project.clone(), session.clone()))
                .copied()
                .unwrap_or(0);
            Ok(crate::records::SessionEventCount::new(count))
        })))
    }

    fn list_sessions_page<'a>(
        &'a self,
        request: crate::records::SessionPageRequest,
    ) -> StorageFuture<
        'a,
        crate::records::StoragePage<crate::records::SessionId, crate::records::SessionId>,
    > {
        Box::pin(ready(self.with_state(|state| {
            use std::ops::Bound::{Excluded, Included, Unbounded};
            let project = request.project_id().clone();
            let agent = request.agent_id().clone();
            let start = match request.cursor() {
                Some(id) => Excluded((project.clone(), agent.clone(), id.clone())),
                None => Included((
                    project.clone(),
                    agent.clone(),
                    crate::records::SessionId::new("\0")?,
                )),
            };
            let mut items: Vec<_> = state
                .sessions
                .range((start, Unbounded))
                .take_while(|(p, a, _)| p == &project && a == &agent)
                .take(request.limit().get() + 1)
                .map(|(_, _, id)| id.clone())
                .collect();
            let next_cursor = if items.len() > request.limit().get() {
                items.pop();
                items.last().cloned()
            } else {
                None
            };
            Ok(crate::records::StoragePage { items, next_cursor })
        })))
    }

    fn append_session_event<'a>(&'a self, record: SessionEventRecord) -> StorageFuture<'a, ()> {
        Box::pin(ready(self.with_state(|state| {
            let key = record.key();
            if state.session_events.contains_key(&key) {
                return Err(StorageError::DuplicateSessionEvent { key });
            }
            let scope = (record.project_id.clone(), record.session_id.clone());
            let count = state
                .session_event_counts
                .get(&scope)
                .copied()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(count_overflow)?;
            state.sessions.insert((
                record.project_id.clone(),
                record.agent_id.clone(),
                record.session_id.clone(),
            ));
            state.session_events.insert(key, record);
            state.session_event_counts.insert(scope, count);
            Ok(())
        })))
    }

    fn append_session_events_atomically<'a>(
        &'a self,
        records: Vec<SessionEventRecord>,
    ) -> StorageFuture<'a, crate::records::SessionEventBatchWriteReceipt> {
        Box::pin(ready(self.with_state(|state| {
            let mut keys = std::collections::HashSet::with_capacity(records.len());
            let mut counts = BTreeMap::new();
            for record in &records {
                let key = record.key();
                if state.session_events.contains_key(&key) || !keys.insert(key.clone()) {
                    return Err(StorageError::DuplicateSessionEvent { key });
                }
                let scope = (record.project_id.clone(), record.session_id.clone());
                let count = counts.entry(scope.clone()).or_insert_with(|| {
                    state.session_event_counts.get(&scope).copied().unwrap_or(0)
                });
                *count = count.checked_add(1).ok_or_else(count_overflow)?;
            }

            let item_count = records.len();
            for record in records {
                state.sessions.insert((
                    record.project_id.clone(),
                    record.agent_id.clone(),
                    record.session_id.clone(),
                ));
                state.session_events.insert(record.key(), record);
            }
            state.session_event_counts.extend(counts);
            Ok(crate::records::SessionEventBatchWriteReceipt {
                item_count,
                rows_affected: item_count as u64,
            })
        })))
    }

    fn list_session_events_page<'a>(
        &'a self,
        request: crate::records::SessionEventPageRequest,
    ) -> StorageFuture<
        'a,
        crate::records::StoragePage<SessionEventRecord, crate::records::SessionEventCursor>,
    > {
        Box::pin(ready(self.with_state(|state| {
            use std::ops::Bound::{Excluded, Included, Unbounded};
            let limit = request.limit().get();
            let start = match request.cursor() {
                Some(cursor) => Excluded(SessionEventKey {
                    project_id: request.project_id().clone(),
                    session_id: request.session_id().clone(),
                    turn_id: cursor.turn_id().clone(),
                    event_id: cursor.event_id().clone(),
                }),
                None => Included(SessionEventKey {
                    project_id: request.project_id().clone(),
                    session_id: request.session_id().clone(),
                    turn_id: crate::records::TurnId::new("\0")?,
                    event_id: crate::records::EventId::new("\0")?,
                }),
            };
            let mut items = state
                .session_events
                .range((start, Unbounded))
                .take_while(|(key, _)| {
                    &key.project_id == request.project_id()
                        && &key.session_id == request.session_id()
                })
                .take(limit + 1)
                .map(|(_, event)| event.clone())
                .collect::<Vec<_>>();
            let next_cursor = if items.len() > limit {
                items.pop();
                items
                    .last()
                    .map(crate::records::SessionEventCursor::from_record)
            } else {
                None
            };
            Ok(crate::records::StoragePage { items, next_cursor })
        })))
    }
}

fn count_overflow() -> StorageError {
    StorageError::Backend {
        message: "session event count exceeds u64".into(),
    }
}

impl AgentStorage for InMemoryAgentStorage {
    fn put_artifact<'a>(&'a self, record: ArtifactRecord) -> StorageFuture<'a, ArtifactPutOutcome> {
        Box::pin(ready(self.with_state(|state| {
            let key = (record.project_id.clone(), record.artifact_hash.clone());
            if let Some(existing) = state.artifacts.get(&key) {
                if existing.body != record.body
                    || existing.size_bytes != record.size_bytes
                    || existing.media_type != record.media_type
                {
                    return Err(StorageError::ArtifactHashCollision {
                        project_id: record.project_id,
                        artifact_hash: record.artifact_hash,
                    });
                }
                return Ok(ArtifactPutOutcome {
                    inserted: false,
                    artifact: existing.clone(),
                });
            }
            state.artifacts.insert(key, record.clone());
            Ok(ArtifactPutOutcome {
                inserted: true,
                artifact: record,
            })
        })))
    }

    fn get_artifact<'a>(
        &'a self,
        project_id: &'a ProjectId,
        artifact_hash: &'a ArtifactHash,
    ) -> StorageFuture<'a, Option<ArtifactRecord>> {
        Box::pin(ready(self.with_state(|state| {
            Ok(state
                .artifacts
                .get(&(project_id.clone(), artifact_hash.clone()))
                .cloned())
        })))
    }

    fn compare_and_swap_artifact_pointer<'a>(
        &'a self,
        update: ArtifactPointerUpdate,
    ) -> StorageFuture<'a, ArtifactPointerRecord> {
        Box::pin(ready(self.with_state(|state| {
            let artifact_key = (update.project_id.clone(), update.new_artifact_hash.clone());
            if !state.artifacts.contains_key(&artifact_key) {
                return Err(StorageError::MissingArtifact {
                    project_id: update.project_id,
                    artifact_hash: update.new_artifact_hash,
                });
            }

            let pointer_key = (update.project_id.clone(), update.pointer_key.clone());
            let actual = state
                .artifact_pointers
                .get(&pointer_key)
                .map(|pointer| pointer.target_artifact_hash.clone());

            if actual != update.expected_artifact_hash {
                return Err(StorageError::ArtifactPointerConflict {
                    project_id: update.project_id,
                    pointer_key: update.pointer_key,
                    expected: update.expected_artifact_hash,
                    actual,
                });
            }

            let record = ArtifactPointerRecord {
                project_id: update.project_id,
                pointer_key: update.pointer_key,
                target_artifact_hash: update.new_artifact_hash,
                updated_by_session_id: update.updated_by_session_id,
                updated_by_agent_id: update.updated_by_agent_id,
                updated_by_event_id: update.updated_by_event_id,
                updated_at_unix_ms: update.updated_at_unix_ms,
            };
            state.artifact_pointers.insert(pointer_key, record.clone());
            Ok(record)
        })))
    }

    fn get_artifact_pointer<'a>(
        &'a self,
        project_id: &'a ProjectId,
        pointer_key: &'a ArtifactPointerKey,
    ) -> StorageFuture<'a, Option<ArtifactPointerRecord>> {
        Box::pin(ready(self.with_state(|state| {
            Ok(state
                .artifact_pointers
                .get(&(project_id.clone(), pointer_key.clone()))
                .cloned())
        })))
    }

    fn record_visibility<'a>(&'a self, receipt: VisibilityReceipt) -> StorageFuture<'a, ()> {
        Box::pin(ready(self.with_state(|state| {
            state.visibility.push(receipt);
            Ok(())
        })))
    }

    fn list_visibility_page<'a>(
        &'a self,
        request: crate::records::VisibilityPageRequest,
    ) -> StorageFuture<
        'a,
        crate::records::StoragePage<VisibilityReceipt, crate::records::VisibilityCursor>,
    > {
        Box::pin(ready(self.with_state(|state| {
            let limit = request.limit().get();
            let mut items = state
                .visibility
                .iter()
                .filter(|receipt| &receipt.project_id == request.project_id())
                .filter(|receipt| {
                    request.cursor().is_none_or(|cursor| {
                        receipt.created_at_unix_ms > cursor.created_at_unix_ms()
                            || (receipt.created_at_unix_ms == cursor.created_at_unix_ms()
                                && receipt.receipt_id.as_str() > cursor.receipt_id())
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            items.sort_by(|left, right| {
                left.created_at_unix_ms
                    .cmp(&right.created_at_unix_ms)
                    .then_with(|| left.receipt_id.cmp(&right.receipt_id))
            });
            items.truncate(limit + 1);
            let next_cursor = if items.len() > limit {
                items.pop();
                items
                    .last()
                    .map(crate::records::VisibilityCursor::from_record)
            } else {
                None
            };
            Ok(crate::records::StoragePage { items, next_cursor })
        })))
    }

    fn put_memory_proposal<'a>(&'a self, record: MemoryProposalRecord) -> StorageFuture<'a, ()> {
        Box::pin(ready(self.with_state(|state| {
            let artifact_key = (
                record.project_id.clone(),
                record.source_artifact_hash.clone(),
            );
            if !state.artifacts.contains_key(&artifact_key) {
                return Err(StorageError::MissingArtifact {
                    project_id: record.project_id,
                    artifact_hash: record.source_artifact_hash,
                });
            }

            let key = (record.project_id.clone(), record.proposal_id.clone());
            if state.memory_proposals.contains_key(&key) {
                return Err(StorageError::DuplicateMemoryProposal {
                    project_id: record.project_id,
                    proposal_id: record.proposal_id,
                });
            }
            state.memory_proposals.insert(key, record);
            Ok(())
        })))
    }

    fn list_memory_proposals_page<'a>(
        &'a self,
        request: crate::records::MemoryProposalPageRequest,
    ) -> StorageFuture<
        'a,
        crate::records::StoragePage<MemoryProposalRecord, crate::records::MemoryProposalCursor>,
    > {
        Box::pin(ready(self.with_state(|state| {
            let limit = request.limit().get();
            let mut items = state
                .memory_proposals
                .values()
                .filter(|proposal| {
                    &proposal.project_id == request.project_id()
                        && request
                            .memory_key()
                            .map(|memory_key| &proposal.memory_key == memory_key)
                            .unwrap_or(true)
                })
                .filter(|proposal| {
                    request.cursor().is_none_or(|cursor| {
                        proposal.created_at_unix_ms > cursor.created_at_unix_ms()
                            || (proposal.created_at_unix_ms == cursor.created_at_unix_ms()
                                && proposal.proposal_id > *cursor.proposal_id())
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            items.sort_by(|left, right| {
                left.created_at_unix_ms
                    .cmp(&right.created_at_unix_ms)
                    .then_with(|| left.proposal_id.cmp(&right.proposal_id))
            });
            items.truncate(limit + 1);
            let next_cursor = if items.len() > limit {
                items.pop();
                items
                    .last()
                    .map(crate::records::MemoryProposalCursor::from_record)
            } else {
                None
            };
            Ok(crate::records::StoragePage { items, next_cursor })
        })))
    }

    fn append_topology_edge<'a>(&'a self, record: TopologyEdgeRecord) -> StorageFuture<'a, ()> {
        Box::pin(ready(self.with_state(|state| {
            let key = (record.project_id.clone(), record.edge_id.clone());
            if state.topology_edges.contains_key(&key) {
                return Err(StorageError::DuplicateTopologyEdge {
                    project_id: record.project_id,
                    edge_id: record.edge_id,
                });
            }
            state.topology_edges.insert(key, record);
            Ok(())
        })))
    }

    fn list_topology_edges_page<'a>(
        &'a self,
        request: crate::records::TopologyEdgePageRequest,
    ) -> StorageFuture<
        'a,
        crate::records::StoragePage<TopologyEdgeRecord, crate::records::TopologyEdgeCursor>,
    > {
        Box::pin(ready(self.with_state(|state| {
            let limit = request.limit().get();
            let mut items = state
                .topology_edges
                .values()
                .filter(|edge| &edge.project_id == request.project_id())
                .filter(|edge| {
                    request.cursor().is_none_or(|cursor| {
                        edge.created_at_unix_ms > cursor.created_at_unix_ms()
                            || (edge.created_at_unix_ms == cursor.created_at_unix_ms()
                                && edge.edge_id > *cursor.edge_id())
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            items.sort_by(|left, right| {
                left.created_at_unix_ms
                    .cmp(&right.created_at_unix_ms)
                    .then_with(|| left.edge_id.cmp(&right.edge_id))
            });
            items.truncate(limit + 1);
            let next_cursor = if items.len() > limit {
                items.pop();
                items
                    .last()
                    .map(crate::records::TopologyEdgeCursor::from_record)
            } else {
                None
            };
            Ok(crate::records::StoragePage { items, next_cursor })
        })))
    }
}
