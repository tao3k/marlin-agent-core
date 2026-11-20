// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

use super::pagination::{
    MemoryProposalCursor, MemoryProposalPageRequest, SessionEventCursor, SessionEventPageRequest,
    SessionPageRequest, StoragePage, TopologyEdgeCursor, TopologyEdgePageRequest, VisibilityCursor,
    VisibilityPageRequest,
};

pub type StorageResult<T> = Result<T, StorageError>;
pub type StorageFuture<'a, T> = Pin<Box<dyn Future<Output = StorageResult<T>> + Send + 'a>>;

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> StorageResult<Self> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(StorageError::InvalidIdentifier {
                        kind: stringify!($name),
                    });
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(ProjectId);
string_id!(SessionId);
string_id!(AgentId);
string_id!(TurnId);
string_id!(EventId);
string_id!(ArtifactHash);
string_id!(ArtifactPointerKey);
string_id!(MemoryKey);
string_id!(MemoryProposalId);
string_id!(TopologyEdgeId);
string_id!(TopologyNodeId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionEventKey {
    pub project_id: ProjectId,
    pub session_id: SessionId,
    pub turn_id: TurnId,
    pub event_id: EventId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventRecord {
    pub project_id: ProjectId,
    pub session_id: SessionId,
    pub agent_id: AgentId,
    pub turn_id: TurnId,
    pub event_id: EventId,
    pub event_kind: String,
    pub causality_parent_event_id: Option<EventId>,
    pub body: Vec<u8>,
    pub created_at_unix_ms: i64,
}

/// Identity and kind of a committed event without its body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionEventHeader {
    pub agent_id: AgentId,
    pub cursor: SessionEventCursor,
    pub event_kind: String,
    pub causality_parent_event_id: Option<EventId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventBatchWriteReceipt {
    pub item_count: usize,
    pub rows_affected: u64,
}

/// Number of committed events in a project-scoped session.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionEventCount(u64);

impl SessionEventCount {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
    pub fn get(self) -> u64 {
        self.0
    }
}

impl SessionEventRecord {
    pub fn key(&self) -> SessionEventKey {
        SessionEventKey {
            project_id: self.project_id.clone(),
            session_id: self.session_id.clone(),
            turn_id: self.turn_id.clone(),
            event_id: self.event_id.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub project_id: ProjectId,
    pub artifact_hash: ArtifactHash,
    pub artifact_kind: String,
    pub producer_session_id: SessionId,
    pub producer_agent_id: AgentId,
    pub producer_event_id: EventId,
    pub media_type: String,
    pub size_bytes: u64,
    pub body: Vec<u8>,
    pub created_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPutOutcome {
    pub inserted: bool,
    pub artifact: ArtifactRecord,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPointerRecord {
    pub project_id: ProjectId,
    pub pointer_key: ArtifactPointerKey,
    pub target_artifact_hash: ArtifactHash,
    pub updated_by_session_id: SessionId,
    pub updated_by_agent_id: AgentId,
    pub updated_by_event_id: EventId,
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPointerUpdate {
    pub project_id: ProjectId,
    pub pointer_key: ArtifactPointerKey,
    pub expected_artifact_hash: Option<ArtifactHash>,
    pub new_artifact_hash: ArtifactHash,
    pub updated_by_session_id: SessionId,
    pub updated_by_agent_id: AgentId,
    pub updated_by_event_id: EventId,
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityReceipt {
    pub project_id: ProjectId,
    pub receipt_id: String,
    pub receipt_kind: String,
    pub body: Vec<u8>,
    pub created_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProposalRecord {
    pub project_id: ProjectId,
    pub proposal_id: MemoryProposalId,
    pub memory_key: MemoryKey,
    pub proposal_status: String,
    pub source_artifact_hash: ArtifactHash,
    pub source_session_id: SessionId,
    pub source_agent_id: AgentId,
    pub source_event_id: EventId,
    pub org_source_path: String,
    pub org_source_begin: u32,
    pub org_source_end: u32,
    pub memory_kind: String,
    pub body: Vec<u8>,
    pub created_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyEdgeRecord {
    pub project_id: ProjectId,
    pub edge_id: TopologyEdgeId,
    pub from_node_id: TopologyNodeId,
    pub to_node_id: TopologyNodeId,
    pub edge_kind: String,
    pub source_session_id: SessionId,
    pub source_agent_id: AgentId,
    pub source_event_id: EventId,
    pub body: Vec<u8>,
    pub created_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageSchemaSnapshot {
    pub lifecycle: StorageSchemaLifecycle,
    pub tables: Vec<StorageSchemaTable>,
}

impl StorageSchemaSnapshot {
    pub fn has_table(&self, table_name: &str) -> bool {
        self.tables.iter().any(|table| table.name == table_name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageSchemaLifecycle {
    DevelopmentBaseline,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageSchemaTable {
    pub name: String,
}

/// Ordered, atomic session event persistence. Implementations may own only
/// session records without providing artifact, memory or topology storage.
pub trait SessionEventStore: Send + Sync {
    /// Seek to the last event in the half-open turn range for one Session.
    fn latest_session_event_header<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a SessionId,
        from_turn: &'a TurnId,
        before_turn: &'a TurnId,
    ) -> StorageFuture<'a, Option<SessionEventHeader>>;

    /// Counts committed session events. Under the append-only storage contract,
    /// a changed count reveals an append between two completed reads. This is
    /// not a transaction, a lease, or a cross-replica consistency guarantee.
    fn count_session_events<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a SessionId,
    ) -> StorageFuture<'a, SessionEventCount>;

    fn list_sessions_page<'a>(
        &'a self,
        request: SessionPageRequest,
    ) -> StorageFuture<'a, StoragePage<SessionId, SessionId>>;

    fn append_session_event<'a>(&'a self, record: SessionEventRecord) -> StorageFuture<'a, ()>;

    fn append_session_events_atomically<'a>(
        &'a self,
        records: Vec<SessionEventRecord>,
    ) -> StorageFuture<'a, SessionEventBatchWriteReceipt>;

    /// For a byte-bounded request, report the last returned cursor when
    /// another record remains. An oversized first body returns an error.
    fn list_session_events_page<'a>(
        &'a self,
        request: SessionEventPageRequest,
    ) -> StorageFuture<'a, StoragePage<SessionEventRecord, SessionEventCursor>>;
}

/// Complete agent storage, composed from the session event store and the
/// separate artifact, visibility, memory and topology operations.
pub trait AgentStorage: SessionEventStore {
    fn put_artifact<'a>(&'a self, record: ArtifactRecord) -> StorageFuture<'a, ArtifactPutOutcome>;

    fn get_artifact<'a>(
        &'a self,
        project_id: &'a ProjectId,
        artifact_hash: &'a ArtifactHash,
    ) -> StorageFuture<'a, Option<ArtifactRecord>>;

    fn compare_and_swap_artifact_pointer<'a>(
        &'a self,
        update: ArtifactPointerUpdate,
    ) -> StorageFuture<'a, ArtifactPointerRecord>;

    fn get_artifact_pointer<'a>(
        &'a self,
        project_id: &'a ProjectId,
        pointer_key: &'a ArtifactPointerKey,
    ) -> StorageFuture<'a, Option<ArtifactPointerRecord>>;

    fn record_visibility<'a>(&'a self, receipt: VisibilityReceipt) -> StorageFuture<'a, ()>;

    fn list_visibility_page<'a>(
        &'a self,
        request: VisibilityPageRequest,
    ) -> StorageFuture<'a, StoragePage<VisibilityReceipt, VisibilityCursor>>;

    fn put_memory_proposal<'a>(&'a self, record: MemoryProposalRecord) -> StorageFuture<'a, ()>;

    fn list_memory_proposals_page<'a>(
        &'a self,
        request: MemoryProposalPageRequest,
    ) -> StorageFuture<'a, StoragePage<MemoryProposalRecord, MemoryProposalCursor>>;

    fn append_topology_edge<'a>(&'a self, record: TopologyEdgeRecord) -> StorageFuture<'a, ()>;

    fn list_topology_edges_page<'a>(
        &'a self,
        request: TopologyEdgePageRequest,
    ) -> StorageFuture<'a, StoragePage<TopologyEdgeRecord, TopologyEdgeCursor>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageError {
    InvalidIdentifier {
        kind: &'static str,
    },
    InvalidEmbedding {
        reason: &'static str,
    },
    InvalidMemorySearchLimit {
        limit: u32,
    },
    InvalidPageLimit {
        limit: u16,
    },
    MissingTurnPageCursor,
    SessionEventBodyExceedsPageBudget {
        budget: u32,
    },
    InvalidSyncConfiguration {
        reason: &'static str,
    },
    SyncNotConfigured,
    DuplicateSessionEvent {
        key: SessionEventKey,
    },
    ArtifactHashCollision {
        project_id: ProjectId,
        artifact_hash: ArtifactHash,
    },
    MissingArtifact {
        project_id: ProjectId,
        artifact_hash: ArtifactHash,
    },
    ArtifactPointerConflict {
        project_id: ProjectId,
        pointer_key: ArtifactPointerKey,
        expected: Option<ArtifactHash>,
        actual: Option<ArtifactHash>,
    },
    Backend {
        message: String,
    },
    DuplicateMemoryProposal {
        project_id: ProjectId,
        proposal_id: MemoryProposalId,
    },
    DuplicateTopologyEdge {
        project_id: ProjectId,
        edge_id: TopologyEdgeId,
    },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::InvalidIdentifier { kind } => {
                write!(f, "invalid empty storage identifier: {kind}")
            }
            StorageError::InvalidEmbedding { reason } => {
                write!(f, "invalid memory embedding: {reason}")
            }
            StorageError::InvalidMemorySearchLimit { limit } => {
                write!(
                    f,
                    "memory vector search limit must be between 1 and 100: {limit}"
                )
            }
            StorageError::InvalidPageLimit { limit } => {
                write!(f, "storage page limit must be between 1 and 1000: {limit}")
            }
            StorageError::MissingTurnPageCursor => {
                f.write_str("turn-scoped Session event page requires an exclusive cursor")
            }
            StorageError::SessionEventBodyExceedsPageBudget { budget } => {
                write!(
                    f,
                    "Session event body exceeds the {budget}-byte page budget"
                )
            }
            StorageError::InvalidSyncConfiguration { reason } => {
                write!(f, "invalid Turso sync configuration: {reason}")
            }
            StorageError::SyncNotConfigured => {
                f.write_str("Turso sync operation requires a sync-backed storage authority")
            }
            StorageError::DuplicateSessionEvent { key } => {
                write!(
                    f,
                    "duplicate session event {}/{}/{}/{}",
                    key.project_id, key.session_id, key.turn_id, key.event_id
                )
            }
            StorageError::ArtifactHashCollision {
                project_id,
                artifact_hash,
            } => {
                write!(
                    f,
                    "artifact hash collision in project {project_id}: {artifact_hash}"
                )
            }
            StorageError::MissingArtifact {
                project_id,
                artifact_hash,
            } => {
                write!(
                    f,
                    "artifact {artifact_hash} is missing in project {project_id}"
                )
            }
            StorageError::ArtifactPointerConflict {
                project_id,
                pointer_key,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "artifact pointer conflict in project {project_id} for {pointer_key}: expected {:?}, actual {:?}",
                    expected.as_ref().map(ArtifactHash::as_str),
                    actual.as_ref().map(ArtifactHash::as_str)
                )
            }
            StorageError::Backend { message } => f.write_str(message),
            StorageError::DuplicateMemoryProposal {
                project_id,
                proposal_id,
            } => {
                write!(
                    f,
                    "duplicate memory proposal in project {project_id}: {proposal_id}"
                )
            }
            StorageError::DuplicateTopologyEdge {
                project_id,
                edge_id,
            } => {
                write!(
                    f,
                    "duplicate topology edge in project {project_id}: {edge_id}"
                )
            }
        }
    }
}

impl std::error::Error for StorageError {}
