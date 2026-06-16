//! Project-scoped runtime graph query contracts.

mod ids;
mod query;
mod query_projection;
mod session_content;

pub use ids::{
    GraphQueryLimit, GraphQueryScoreBasisPoints, ProjectRuntimeAgentId, ProjectRuntimeBranchRef,
    ProjectRuntimeContentId, ProjectRuntimeContextPackId, ProjectRuntimeEvidenceId,
    ProjectRuntimeMemoryId, ProjectRuntimeProjectId, ProjectRuntimeReceiptId,
    ProjectRuntimeRootSessionId, ProjectRuntimeSessionId, ProjectRuntimeSourceAnchorId,
    ProjectRuntimeSourceSpanRef, ProjectRuntimeToolCapabilityId, ProjectRuntimeWorkspaceId,
    ProjectRuntimeWorktreeId,
};
pub use query::{
    GraphQueryContext, GraphQueryExternalProjectPolicy, GraphQueryFallbackPolicy,
    GraphQueryFallbackScope, GraphQueryFamily, GraphQueryMatch, GraphQueryMatchRelationship,
    GraphQueryRelationshipFact, GraphQueryRequest, GraphQueryResponse, GraphQuerySecretVisibility,
    GraphQueryVisibility, GraphQueryVisibleSurface, ProjectMemoryContextFact,
    ProjectMemoryContextPack, ProjectMemoryRecallIntent, ProjectMemoryRecallRequest,
    ProjectMemoryRecallTerm,
};
pub use session_content::{
    AgentContentCompressionState, AgentContentNode, AgentContentNodeInput, AgentContentRole,
    AgentSessionFact, AgentSessionHistoryLimit, AgentSessionKind, ContentCompressionReceipt,
    ContentCompressionStatus, ContentTokenBudget, ContentTokenCount, ContentUsageKind,
    ContentUsageReceipt, ContentUsageReceiptInput, ContextPackReceipt, MemoryTriggerReceipt,
    MemoryTriggerStatus, ProjectRuntimeContentBodyRef,
};
