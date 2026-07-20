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
    MemoryProposalCursor, MemoryProposalId, MemoryProposalPageRequest, MemoryProposalRecord,
    ProjectId, SessionEventBatchWriteReceipt, SessionEventCursor, SessionEventKey,
    SessionEventPageRequest, SessionEventRecord, SessionId, StorageError, StorageFuture,
    StoragePage, StoragePageLimit, StorageResult, StorageSchemaLifecycle, StorageSchemaSnapshot,
    StorageSchemaTable, TopologyEdgeCursor, TopologyEdgeId, TopologyEdgePageRequest,
    TopologyEdgeRecord, TopologyNodeId, TurnId, VisibilityCursor, VisibilityPageRequest,
    VisibilityReceipt,
};
#[cfg(feature = "turso")]
pub use turso_backend::{
    TursoAgentStorage, TursoAgentStorageConfig, TursoAsyncIoMode, TursoBatchTransactionMode,
    TursoBatchWriteReceipt, TursoMemoryEmbedding, TursoMemoryEmbeddingRecord,
    TursoMemorySearchLimit, TursoMemorySearchMatch, TursoMemorySearchRequest,
    TursoMvccCheckpointMode, TursoMvccMode, TursoOperationalReceipt, TursoOptimizationProfile,
    TursoOptimizationReceipt, TursoSdkTelemetryStatus, TursoStatementCacheMode,
    TursoSyncFeatureStatus, TursoTransactionOperation, TursoTransactionReceipt,
    TursoTransactionStatus,
};

#[cfg(feature = "turso-sync")]
pub use turso_backend::{
    TursoSyncAgentStorageConfig, TursoSyncAuthToken, TursoSyncRemoteUrl, TursoSyncStatsReceipt,
};
