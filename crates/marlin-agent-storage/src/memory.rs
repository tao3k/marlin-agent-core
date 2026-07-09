use std::collections::BTreeMap;
use std::future::ready;
use std::sync::{Arc, Mutex};

use crate::records::{
    AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerRecord, ArtifactPointerUpdate,
    ArtifactPutOutcome, ArtifactRecord, MemoryKey, MemoryProposalId, MemoryProposalRecord,
    ProjectId, SessionEventKey, SessionEventRecord, SessionId, StorageError, StorageFuture,
    StorageResult, TopologyEdgeId, TopologyEdgeRecord, VisibilityReceipt,
};

#[derive(Clone, Debug, Default)]
pub struct InMemoryAgentStorage {
    state: Arc<Mutex<State>>,
}

#[derive(Clone, Debug, Default)]
struct State {
    session_events: BTreeMap<SessionEventKey, SessionEventRecord>,
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

impl AgentStorage for InMemoryAgentStorage {
    fn append_session_event<'a>(&'a self, record: SessionEventRecord) -> StorageFuture<'a, ()> {
        Box::pin(ready(self.with_state(|state| {
            let key = record.key();
            if state.session_events.contains_key(&key) {
                return Err(StorageError::DuplicateSessionEvent { key });
            }
            state.session_events.insert(key, record);
            Ok(())
        })))
    }

    fn list_session_events<'a>(
        &'a self,
        project_id: &'a ProjectId,
        session_id: &'a SessionId,
    ) -> StorageFuture<'a, Vec<SessionEventRecord>> {
        Box::pin(ready(self.with_state(|state| {
            Ok(state
                .session_events
                .values()
                .filter(|event| &event.project_id == project_id && &event.session_id == session_id)
                .cloned()
                .collect())
        })))
    }

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

    fn list_visibility<'a>(
        &'a self,
        project_id: &'a ProjectId,
    ) -> StorageFuture<'a, Vec<VisibilityReceipt>> {
        Box::pin(ready(self.with_state(|state| {
            Ok(state
                .visibility
                .iter()
                .filter(|receipt| &receipt.project_id == project_id)
                .cloned()
                .collect())
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

    fn list_memory_proposals<'a>(
        &'a self,
        project_id: &'a ProjectId,
        memory_key: Option<&'a MemoryKey>,
    ) -> StorageFuture<'a, Vec<MemoryProposalRecord>> {
        Box::pin(ready(self.with_state(|state| {
            Ok(state
                .memory_proposals
                .values()
                .filter(|proposal| {
                    &proposal.project_id == project_id
                        && memory_key
                            .map(|memory_key| &proposal.memory_key == memory_key)
                            .unwrap_or(true)
                })
                .cloned()
                .collect())
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

    fn list_topology_edges<'a>(
        &'a self,
        project_id: &'a ProjectId,
    ) -> StorageFuture<'a, Vec<TopologyEdgeRecord>> {
        Box::pin(ready(self.with_state(|state| {
            Ok(state
                .topology_edges
                .values()
                .filter(|edge| &edge.project_id == project_id)
                .cloned()
                .collect())
        })))
    }
}
