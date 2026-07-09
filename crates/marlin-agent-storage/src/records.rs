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
    pub migrations: Vec<StorageSchemaMigration>,
    pub tables: Vec<StorageSchemaTable>,
}

impl StorageSchemaSnapshot {
    pub fn has_migration(&self, migration_id: &str) -> bool {
        self.migrations
            .iter()
            .any(|migration| migration.migration_id == migration_id)
    }

    pub fn has_table(&self, table_name: &str) -> bool {
        self.tables.iter().any(|table| table.name == table_name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageSchemaMigration {
    pub migration_id: String,
    pub applied_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageSchemaTable {
    pub name: String,
}

pub trait AgentStorage: Send + Sync {
    fn append_session_event<'a>(&'a self, record: SessionEventRecord) -> StorageFuture<'a, ()>;

    fn list_session_events<'a>(
        &'a self,
        project_id: &'a ProjectId,
        session_id: &'a SessionId,
    ) -> StorageFuture<'a, Vec<SessionEventRecord>>;

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

    fn list_visibility<'a>(
        &'a self,
        project_id: &'a ProjectId,
    ) -> StorageFuture<'a, Vec<VisibilityReceipt>>;

    fn put_memory_proposal<'a>(&'a self, record: MemoryProposalRecord) -> StorageFuture<'a, ()>;

    fn list_memory_proposals<'a>(
        &'a self,
        project_id: &'a ProjectId,
        memory_key: Option<&'a MemoryKey>,
    ) -> StorageFuture<'a, Vec<MemoryProposalRecord>>;

    fn append_topology_edge<'a>(&'a self, record: TopologyEdgeRecord) -> StorageFuture<'a, ()>;

    fn list_topology_edges<'a>(
        &'a self,
        project_id: &'a ProjectId,
    ) -> StorageFuture<'a, Vec<TopologyEdgeRecord>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageError {
    InvalidIdentifier {
        kind: &'static str,
    },
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
