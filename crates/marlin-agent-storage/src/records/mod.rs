// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Agent storage records and persistence contracts.

mod contract;
mod pagination;

pub use contract::{
    AgentId, AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerRecord,
    ArtifactPointerUpdate, ArtifactPutOutcome, ArtifactRecord, EventId, MemoryKey,
    MemoryProposalId, MemoryProposalRecord, ProjectId, SessionEventBatchWriteReceipt,
    SessionEventCount, SessionEventHeader, SessionEventKey, SessionEventRecord, SessionEventStore,
    SessionId, StorageError, StorageFuture, StorageResult, StorageSchemaLifecycle,
    StorageSchemaSnapshot, StorageSchemaTable, TopologyEdgeId, TopologyEdgeRecord, TopologyNodeId,
    TurnId, VisibilityReceipt,
};
pub use pagination::{
    MemoryProposalCursor, MemoryProposalPageRequest, SessionEventCursor, SessionEventPageRequest,
    SessionPageRequest, StoragePage, StoragePageLimit, TopologyEdgeCursor, TopologyEdgePageRequest,
    VisibilityCursor, VisibilityPageRequest,
};
