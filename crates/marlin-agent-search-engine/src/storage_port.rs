// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Marlin unified storage adapter implementing ASP's [`SearchEngineStoragePort`].
//!
//! Maps ASP's content-addressed artifact CAS, compare-and-swap generation
//! pointers, and topology edges 1:1 onto Marlin's [`AgentStorage`]
//! (`InMemoryAgentStorage` and `TursoAgentStorage`) so ASP never embeds a
//! second Turso database.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use agent_semantic_search::{
    SearchArtifactBlob, SearchArtifactHash, SearchEngineStoragePort, SearchGenerationDigest,
    SearchGenerationPointerRecord, SearchGenerationPointerSwapRequest, SearchPointerKey,
    SearchProjectId, SearchStoragePortError, SearchTopologyEdgeId, SearchTopologyEdgeRecord,
    SearchWorkspaceId,
};
use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerUpdate, ArtifactRecord,
    EventId, ProjectId, SessionId, StorageError, StoragePageLimit, TopologyEdgeId,
    TopologyEdgePageRequest, TopologyEdgeRecord, TopologyNodeId,
};

const ASP_ARTIFACT_KIND: &str = "asp.search.artifact";
const POINTER_KEY_SEPARATOR: &str = "::";
const GENERATION_EVENT_PREFIX: &str = "asp-gen:v1:";

/// Marlin provenance identifiers attached to search artifacts and topology edges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarlinSearchStorageProvenance {
    pub session_id: SessionId,
    pub agent_id: AgentId,
    pub event_id: EventId,
}

impl MarlinSearchStorageProvenance {
    /// Construct validated provenance identifiers for search storage writes.
    pub fn new(
        session_id: impl Into<String>,
        agent_id: impl Into<String>,
        event_id: impl Into<String>,
    ) -> Result<Self, StorageError> {
        Ok(Self {
            session_id: SessionId::new(session_id)?,
            agent_id: AgentId::new(agent_id)?,
            event_id: EventId::new(event_id)?,
        })
    }
}

/// Adapter exposing any Marlin [`AgentStorage`] as ASP's [`SearchEngineStoragePort`].
pub struct MarlinAgentStorageSearchPort<S> {
    storage: Arc<S>,
    provenance: MarlinSearchStorageProvenance,
}

impl<S> MarlinAgentStorageSearchPort<S> {
    #[must_use]
    pub fn new(storage: Arc<S>, provenance: MarlinSearchStorageProvenance) -> Self {
        Self {
            storage,
            provenance,
        }
    }

    #[must_use]
    pub fn storage(&self) -> &Arc<S> {
        &self.storage
    }
}

impl<S: AgentStorage> MarlinAgentStorageSearchPort<S> {
    fn append_single_topology_edge(
        &self,
        edge: &SearchTopologyEdgeRecord,
        created_at_unix_ms: i64,
    ) -> Result<bool, SearchStoragePortError> {
        let record = TopologyEdgeRecord {
            project_id: ProjectId::new(edge.project_id.as_str())
                .map_err(|error| map_storage_error(error, None))?,
            edge_id: scoped_topology_edge_id(&edge.generation_digest, &edge.edge_id)?,
            from_node_id: TopologyNodeId::new(edge.from_node.as_str())
                .map_err(|error| map_storage_error(error, None))?,
            to_node_id: TopologyNodeId::new(edge.to_node.as_str())
                .map_err(|error| map_storage_error(error, None))?,
            edge_kind: edge.relation.clone(),
            source_session_id: self.provenance.session_id.clone(),
            source_agent_id: self.provenance.agent_id.clone(),
            source_event_id: encode_edge_generation_event(&edge.generation_digest)?,
            body: edge.payload.clone(),
            created_at_unix_ms,
        };
        match futures_executor::block_on(self.storage.append_topology_edge(record)) {
            Ok(()) => Ok(true),
            Err(StorageError::DuplicateTopologyEdge { .. }) => Ok(false),
            Err(error) => Err(map_storage_error(error, None)),
        }
    }
}

fn unix_ms_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn map_storage_error(
    error: StorageError,
    pointer_key: Option<&SearchPointerKey>,
) -> SearchStoragePortError {
    match error {
        StorageError::InvalidIdentifier { kind } => {
            SearchStoragePortError::InvalidIdentifier { kind }
        }
        StorageError::MissingArtifact {
            project_id,
            artifact_hash,
        } => {
            let project = SearchProjectId::new(project_id.as_str()).unwrap_or_else(|_| {
                SearchProjectId::new("invalid-project").expect("static project id")
            });
            let hash = SearchArtifactHash::new(artifact_hash.as_str()).unwrap_or_else(|_| {
                SearchArtifactHash::new("invalid-hash").expect("static artifact hash")
            });
            SearchStoragePortError::MissingArtifact {
                project_id: project,
                artifact_hash: hash,
            }
        }
        StorageError::ArtifactPointerConflict {
            pointer_key: raw_key,
            expected,
            actual,
            ..
        } => {
            let key = pointer_key.cloned().unwrap_or_else(|| {
                SearchPointerKey::new(raw_key.as_str())
                    .unwrap_or_else(|_| SearchPointerKey::new("pointer").expect("static key"))
            });
            SearchStoragePortError::PointerConflict {
                pointer_key: key,
                expected: expected.and_then(|hash| SearchArtifactHash::new(hash.as_str()).ok()),
                actual: actual.and_then(|hash| SearchArtifactHash::new(hash.as_str()).ok()),
            }
        }
        other => SearchStoragePortError::Backend(other.to_string()),
    }
}

fn scoped_pointer_key(
    workspace_id: &SearchWorkspaceId,
    pointer_key: &SearchPointerKey,
) -> Result<ArtifactPointerKey, SearchStoragePortError> {
    ArtifactPointerKey::new(format!(
        "{}{}{}",
        workspace_id.as_str(),
        POINTER_KEY_SEPARATOR,
        pointer_key.as_str()
    ))
    .map_err(|error| map_storage_error(error, Some(pointer_key)))
}

fn scoped_topology_edge_id(
    generation_digest: &SearchGenerationDigest,
    edge_id: &SearchTopologyEdgeId,
) -> Result<TopologyEdgeId, SearchStoragePortError> {
    TopologyEdgeId::new(format!(
        "{}{}{}",
        generation_digest.as_str(),
        POINTER_KEY_SEPARATOR,
        edge_id.as_str()
    ))
    .map_err(|error| map_storage_error(error, None))
}

fn decode_scoped_topology_edge_id(
    generation_digest: &SearchGenerationDigest,
    stored_edge_id: &TopologyEdgeId,
) -> Result<SearchTopologyEdgeId, SearchStoragePortError> {
    let prefix = format!("{}{}", generation_digest.as_str(), POINTER_KEY_SEPARATOR);
    let raw = stored_edge_id
        .as_str()
        .strip_prefix(&prefix)
        .unwrap_or(stored_edge_id.as_str());
    SearchTopologyEdgeId::new(raw)
}

fn encode_pointer_event_id(
    revision: u64,
    generation_digest: &SearchGenerationDigest,
) -> Result<EventId, SearchStoragePortError> {
    EventId::new(format!(
        "{GENERATION_EVENT_PREFIX}{revision}:{}",
        generation_digest.as_str()
    ))
    .map_err(|error| map_storage_error(error, None))
}

fn decode_pointer_event_id(
    event_id: &EventId,
) -> Result<(u64, SearchGenerationDigest), SearchStoragePortError> {
    let rest = event_id
        .as_str()
        .strip_prefix(GENERATION_EVENT_PREFIX)
        .ok_or_else(|| {
            SearchStoragePortError::Backend(format!(
                "pointer event metadata `{event_id}` does not carry ASP generation provenance"
            ))
        })?;
    let (revision_text, digest_text) = rest.split_once(':').ok_or_else(|| {
        SearchStoragePortError::Backend(format!(
            "pointer event metadata `{event_id}` is missing revision separator"
        ))
    })?;
    let revision = revision_text.parse::<u64>().map_err(|error| {
        SearchStoragePortError::Backend(format!(
            "invalid pointer revision `{revision_text}`: {error}"
        ))
    })?;
    let generation_digest = SearchGenerationDigest::new(digest_text)?;
    Ok((revision, generation_digest))
}

fn encode_edge_generation_event(
    generation_digest: &SearchGenerationDigest,
) -> Result<EventId, SearchStoragePortError> {
    EventId::new(format!(
        "{GENERATION_EVENT_PREFIX}{}",
        generation_digest.as_str()
    ))
    .map_err(|error| map_storage_error(error, None))
}

impl<S: AgentStorage> SearchEngineStoragePort for MarlinAgentStorageSearchPort<S> {
    fn put_artifact(&self, artifact: SearchArtifactBlob) -> Result<bool, SearchStoragePortError> {
        artifact.verify_digest()?;
        let project_id = ProjectId::new(artifact.project_id.as_str())
            .map_err(|error| map_storage_error(error, None))?;
        let artifact_hash = ArtifactHash::new(artifact.artifact_hash.as_str())
            .map_err(|error| map_storage_error(error, None))?;
        let size_bytes = u64::try_from(artifact.body.len()).unwrap_or(u64::MAX);
        let record = ArtifactRecord {
            project_id,
            artifact_hash,
            artifact_kind: ASP_ARTIFACT_KIND.to_owned(),
            producer_session_id: self.provenance.session_id.clone(),
            producer_agent_id: self.provenance.agent_id.clone(),
            producer_event_id: self.provenance.event_id.clone(),
            media_type: artifact.media_type,
            size_bytes,
            body: artifact.body,
            created_at_unix_ms: unix_ms_now(),
        };
        let outcome = futures_executor::block_on(self.storage.put_artifact(record))
            .map_err(|error| map_storage_error(error, None))?;
        Ok(outcome.inserted)
    }

    fn get_artifact(
        &self,
        project_id: &SearchProjectId,
        artifact_hash: &SearchArtifactHash,
    ) -> Result<Option<SearchArtifactBlob>, SearchStoragePortError> {
        let project =
            ProjectId::new(project_id.as_str()).map_err(|error| map_storage_error(error, None))?;
        let hash = ArtifactHash::new(artifact_hash.as_str())
            .map_err(|error| map_storage_error(error, None))?;
        let found = futures_executor::block_on(self.storage.get_artifact(&project, &hash))
            .map_err(|error| map_storage_error(error, None))?;
        Ok(found.map(|record| SearchArtifactBlob {
            project_id: project_id.clone(),
            artifact_hash: artifact_hash.clone(),
            media_type: record.media_type,
            body: record.body,
        }))
    }

    fn compare_and_swap_generation_pointer(
        &self,
        request: SearchGenerationPointerSwapRequest,
    ) -> Result<SearchGenerationPointerRecord, SearchStoragePortError> {
        let project = ProjectId::new(request.project_id.as_str())
            .map_err(|error| map_storage_error(error, Some(&request.pointer_key)))?;
        let scoped_key = scoped_pointer_key(&request.workspace_id, &request.pointer_key)?;
        let current =
            futures_executor::block_on(self.storage.get_artifact_pointer(&project, &scoped_key))
                .map_err(|error| map_storage_error(error, Some(&request.pointer_key)))?;
        let next_revision = match &current {
            Some(record) => decode_pointer_event_id(&record.updated_by_event_id)?
                .0
                .saturating_add(1),
            None => 1,
        };
        let expected_artifact_hash = request
            .expected_hash
            .as_ref()
            .map(|hash| ArtifactHash::new(hash.as_str()))
            .transpose()
            .map_err(|error| map_storage_error(error, Some(&request.pointer_key)))?;
        let new_hash = ArtifactHash::new(request.new_artifact_hash.as_str())
            .map_err(|error| map_storage_error(error, Some(&request.pointer_key)))?;
        let update = ArtifactPointerUpdate {
            project_id: project,
            pointer_key: scoped_key,
            expected_artifact_hash,
            new_artifact_hash: new_hash,
            updated_by_session_id: self.provenance.session_id.clone(),
            updated_by_agent_id: self.provenance.agent_id.clone(),
            updated_by_event_id: encode_pointer_event_id(
                next_revision,
                &request.new_generation_digest,
            )?,
            updated_at_unix_ms: unix_ms_now(),
        };
        let committed =
            futures_executor::block_on(self.storage.compare_and_swap_artifact_pointer(update))
                .map_err(|error| map_storage_error(error, Some(&request.pointer_key)))?;
        let (revision, generation_digest) =
            decode_pointer_event_id(&committed.updated_by_event_id)?;
        Ok(SearchGenerationPointerRecord {
            project_id: request.project_id,
            workspace_id: request.workspace_id,
            pointer_key: request.pointer_key,
            generation_digest,
            artifact_hash: SearchArtifactHash::new(committed.target_artifact_hash.as_str())?,
            revision,
        })
    }

    fn get_generation_pointer(
        &self,
        project_id: &SearchProjectId,
        workspace_id: &SearchWorkspaceId,
        pointer_key: &SearchPointerKey,
    ) -> Result<Option<SearchGenerationPointerRecord>, SearchStoragePortError> {
        let project = ProjectId::new(project_id.as_str())
            .map_err(|error| map_storage_error(error, Some(pointer_key)))?;
        let scoped_key = scoped_pointer_key(workspace_id, pointer_key)?;
        let Some(record) =
            futures_executor::block_on(self.storage.get_artifact_pointer(&project, &scoped_key))
                .map_err(|error| map_storage_error(error, Some(pointer_key)))?
        else {
            return Ok(None);
        };
        let (revision, generation_digest) = decode_pointer_event_id(&record.updated_by_event_id)?;
        Ok(Some(SearchGenerationPointerRecord {
            project_id: project_id.clone(),
            workspace_id: workspace_id.clone(),
            pointer_key: pointer_key.clone(),
            generation_digest,
            artifact_hash: SearchArtifactHash::new(record.target_artifact_hash.as_str())?,
            revision,
        }))
    }

    fn append_topology_edges(
        &self,
        edges: &[SearchTopologyEdgeRecord],
    ) -> Result<usize, SearchStoragePortError> {
        let now = unix_ms_now();
        edges.iter().try_fold(0usize, |inserted, edge| {
            self.append_single_topology_edge(edge, now)
                .map(|added| inserted + usize::from(added))
        })
    }

    fn list_topology_edges(
        &self,
        project_id: &SearchProjectId,
        generation_digest: &SearchGenerationDigest,
    ) -> Result<Vec<SearchTopologyEdgeRecord>, SearchStoragePortError> {
        let project =
            ProjectId::new(project_id.as_str()).map_err(|error| map_storage_error(error, None))?;
        let expected_event = encode_edge_generation_event(generation_digest)?;
        let mut edges = Vec::new();
        let mut cursor = None;
        loop {
            let mut request =
                TopologyEdgePageRequest::new(project.clone(), StoragePageLimit::MAXIMUM);
            if let Some(next_cursor) = cursor {
                request = request.after(next_cursor);
            }
            let page = futures_executor::block_on(self.storage.list_topology_edges_page(request))
                .map_err(|error| map_storage_error(error, None))?;
            for item in page.items {
                if item.source_event_id == expected_event {
                    edges.push(SearchTopologyEdgeRecord {
                        project_id: project_id.clone(),
                        generation_digest: generation_digest.clone(),
                        edge_id: decode_scoped_topology_edge_id(generation_digest, &item.edge_id)?,
                        from_node: item.from_node_id.as_str().to_owned(),
                        to_node: item.to_node_id.as_str().to_owned(),
                        relation: item.edge_kind,
                        payload: item.body,
                    });
                }
            }
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        edges.sort_by(|left, right| left.edge_id.cmp(&right.edge_id));
        Ok(edges)
    }
}
