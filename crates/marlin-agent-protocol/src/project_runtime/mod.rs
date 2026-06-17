//! Project-scoped runtime graph query contracts.

mod ids;
mod query;
mod query_projection;
mod session_content;
mod turn_context_item_view;

pub use ids::{
    GraphQueryLimit, GraphQueryScoreBasisPoints, ProjectRuntimeAgentId,
    ProjectRuntimeBackendRequirementId, ProjectRuntimeBranchRef, ProjectRuntimeContentId,
    ProjectRuntimeContextPackId, ProjectRuntimeEvidenceId, ProjectRuntimeIsolationRequirementId,
    ProjectRuntimeMemoryCitationId, ProjectRuntimeMemoryId, ProjectRuntimeProjectId,
    ProjectRuntimeReceiptId, ProjectRuntimeRootSessionId, ProjectRuntimeSessionId,
    ProjectRuntimeSourceAnchorId, ProjectRuntimeSourceSpanRef, ProjectRuntimeSteeringItemId,
    ProjectRuntimeToolCapabilityId, ProjectRuntimeTurnId, ProjectRuntimeWorkspaceId,
    ProjectRuntimeWorktreeId,
};
pub use query::{
    GraphQueryContext, GraphQueryExternalProjectPolicy, GraphQueryFallbackPolicy,
    GraphQueryFallbackScope, GraphQueryFamily, GraphQueryMatch, GraphQueryMatchRelationship,
    GraphQueryRelationshipFact, GraphQueryRequest, GraphQueryResponse, GraphQuerySecretVisibility,
    GraphQueryVisibility, GraphQueryVisibleSurface, ProjectMemoryContextFact,
    ProjectMemoryContextPack, ProjectMemoryRecallIntent, ProjectMemoryRecallRequest,
    ProjectMemoryRecallTerm, ProjectRuntimeToolCapabilityCard,
};
pub use session_content::{
    AgentContentCompressionState, AgentContentNode, AgentContentNodeInput, AgentContentRole,
    AgentSessionFact, AgentSessionHistoryLimit, AgentSessionKind, ContentCompressionReceipt,
    ContentCompressionStatus, ContentTokenBudget, ContentTokenCount, ContentUsageKind,
    ContentUsageReceipt, ContentUsageReceiptInput, ContextPackReceipt, MemoryTriggerReceipt,
    MemoryTriggerStatus, ProjectRuntimeContentBodyRef, ProjectRuntimeMemoryCitation,
    TurnContextItemKind, TurnContextOmissionReason, TurnContextOmittedItem,
    TurnContextSelectedItem, TurnContextSteeringReceipt, TurnContextSteeringReceiptInput,
};
pub use turn_context_item_view::{TurnContextItemView, TurnContextItemViewReceipt};
