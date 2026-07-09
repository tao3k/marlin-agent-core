//! Agent-native storage contracts.
//!
//! This crate owns the persistence boundary for sessions, artifacts, visibility,
//! and future Turso-backed projections. Domain crates should depend on these
//! typed operations instead of embedding SQL or backend-specific conflict rules.

mod memory;
mod records;
#[cfg(feature = "turso")]
mod turso_backend;

pub use memory::InMemoryAgentStorage;
pub use records::{
    AgentId, AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerRecord,
    ArtifactPointerUpdate, ArtifactPutOutcome, ArtifactRecord, EventId, MemoryKey,
    MemoryProposalId, MemoryProposalRecord, ProjectId, SessionEventKey, SessionEventRecord,
    SessionId, StorageError, StorageFuture, StorageResult, StorageSchemaMigration,
    StorageSchemaSnapshot, StorageSchemaTable, TopologyEdgeId, TopologyEdgeRecord, TopologyNodeId,
    TurnId, VisibilityReceipt,
};
#[cfg(feature = "turso")]
pub use turso_backend::{
    STORAGE_SCHEMA_V1_MIGRATION_ID, TursoAgentStorage, TursoAgentStorageConfig, TursoMvccMode,
};
