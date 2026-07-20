use std::fmt;
use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventBatchWriteReceipt {
    pub item_count: usize,
    pub rows_affected: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePageLimit(u16);

impl StoragePageLimit {
    pub const MAX: u16 = 1_000;
    pub const MAXIMUM: Self = Self(Self::MAX);

    pub fn new(limit: u16) -> StorageResult<Self> {
        if !(1..=Self::MAX).contains(&limit) {
            return Err(StorageError::InvalidPageLimit { limit });
        }
        Ok(Self(limit))
    }

    pub fn get(self) -> usize {
        usize::from(self.0)
    }

    #[cfg(feature = "turso")]
    pub(crate) fn fetch_count(self) -> i64 {
        i64::from(self.0) + 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePage<T, C> {
    pub items: Vec<T>,
    pub next_cursor: Option<C>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventCursor {
    turn_id: TurnId,
    event_id: EventId,
}

impl SessionEventCursor {
    pub fn from_record(record: &SessionEventRecord) -> Self {
        Self {
            turn_id: record.turn_id.clone(),
            event_id: record.event_id.clone(),
        }
    }

    pub fn turn_id(&self) -> &TurnId {
        &self.turn_id
    }

    pub fn event_id(&self) -> &EventId {
        &self.event_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventPageRequest {
    project_id: ProjectId,
    session_id: SessionId,
    after: Option<SessionEventCursor>,
    limit: StoragePageLimit,
}

impl SessionEventPageRequest {
    pub fn new(project_id: ProjectId, session_id: SessionId, limit: StoragePageLimit) -> Self {
        Self {
            project_id,
            session_id,
            after: None,
            limit,
        }
    }

    pub fn after(mut self, cursor: SessionEventCursor) -> Self {
        self.after = Some(cursor);
        self
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn cursor(&self) -> Option<&SessionEventCursor> {
        self.after.as_ref()
    }

    pub fn limit(&self) -> StoragePageLimit {
        self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityCursor {
    created_at_unix_ms: i64,
    receipt_id: String,
}

impl VisibilityCursor {
    pub fn from_record(record: &VisibilityReceipt) -> Self {
        Self {
            created_at_unix_ms: record.created_at_unix_ms,
            receipt_id: record.receipt_id.clone(),
        }
    }

    pub fn created_at_unix_ms(&self) -> i64 {
        self.created_at_unix_ms
    }

    pub fn receipt_id(&self) -> &str {
        &self.receipt_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityPageRequest {
    project_id: ProjectId,
    after: Option<VisibilityCursor>,
    limit: StoragePageLimit,
}

impl VisibilityPageRequest {
    pub fn new(project_id: ProjectId, limit: StoragePageLimit) -> Self {
        Self {
            project_id,
            after: None,
            limit,
        }
    }

    pub fn after(mut self, cursor: VisibilityCursor) -> Self {
        self.after = Some(cursor);
        self
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn cursor(&self) -> Option<&VisibilityCursor> {
        self.after.as_ref()
    }

    pub fn limit(&self) -> StoragePageLimit {
        self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProposalCursor {
    created_at_unix_ms: i64,
    proposal_id: MemoryProposalId,
}

impl MemoryProposalCursor {
    pub fn from_record(record: &MemoryProposalRecord) -> Self {
        Self {
            created_at_unix_ms: record.created_at_unix_ms,
            proposal_id: record.proposal_id.clone(),
        }
    }

    pub fn created_at_unix_ms(&self) -> i64 {
        self.created_at_unix_ms
    }

    pub fn proposal_id(&self) -> &MemoryProposalId {
        &self.proposal_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProposalPageRequest {
    project_id: ProjectId,
    memory_key: Option<MemoryKey>,
    after: Option<MemoryProposalCursor>,
    limit: StoragePageLimit,
}

impl MemoryProposalPageRequest {
    pub fn new(project_id: ProjectId, limit: StoragePageLimit) -> Self {
        Self {
            project_id,
            memory_key: None,
            after: None,
            limit,
        }
    }

    pub fn for_memory_key(mut self, memory_key: MemoryKey) -> Self {
        self.memory_key = Some(memory_key);
        self
    }

    pub fn after(mut self, cursor: MemoryProposalCursor) -> Self {
        self.after = Some(cursor);
        self
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn memory_key(&self) -> Option<&MemoryKey> {
        self.memory_key.as_ref()
    }

    pub fn cursor(&self) -> Option<&MemoryProposalCursor> {
        self.after.as_ref()
    }

    pub fn limit(&self) -> StoragePageLimit {
        self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyEdgeCursor {
    created_at_unix_ms: i64,
    edge_id: TopologyEdgeId,
}

impl TopologyEdgeCursor {
    pub fn from_record(record: &TopologyEdgeRecord) -> Self {
        Self {
            created_at_unix_ms: record.created_at_unix_ms,
            edge_id: record.edge_id.clone(),
        }
    }

    pub fn created_at_unix_ms(&self) -> i64 {
        self.created_at_unix_ms
    }

    pub fn edge_id(&self) -> &TopologyEdgeId {
        &self.edge_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyEdgePageRequest {
    project_id: ProjectId,
    after: Option<TopologyEdgeCursor>,
    limit: StoragePageLimit,
}

impl TopologyEdgePageRequest {
    pub fn new(project_id: ProjectId, limit: StoragePageLimit) -> Self {
        Self {
            project_id,
            after: None,
            limit,
        }
    }

    pub fn after(mut self, cursor: TopologyEdgeCursor) -> Self {
        self.after = Some(cursor);
        self
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn cursor(&self) -> Option<&TopologyEdgeCursor> {
        self.after.as_ref()
    }

    pub fn limit(&self) -> StoragePageLimit {
        self.limit
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

pub trait AgentStorage: Send + Sync {
    fn append_session_event<'a>(&'a self, record: SessionEventRecord) -> StorageFuture<'a, ()>;

    fn append_session_events_atomically<'a>(
        &'a self,
        records: Vec<SessionEventRecord>,
    ) -> StorageFuture<'a, SessionEventBatchWriteReceipt>;

    fn list_session_events_page<'a>(
        &'a self,
        request: SessionEventPageRequest,
    ) -> StorageFuture<'a, StoragePage<SessionEventRecord, SessionEventCursor>>;

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
