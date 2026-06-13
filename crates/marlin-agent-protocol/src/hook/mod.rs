//! Hook protocol contracts for runtime interception and observation.

mod context;
mod types;

pub use context::{
    HookAgentClass, HookAgentLineageNode, HookDecisionContext, HookOrgMemoryHit, HookSessionId,
    HookWorkspaceStateFact,
};
pub use types::{
    HookAgentScope, HookConfigurationReloadReceipt, HookConfigurationVersion,
    HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput, HookDispatchSelectionInput,
    HookDispatchSelectionReceipt, HookDurationMs, HookEventName, HookExecutionMode,
    HookHandlerType, HookMatcherStrategy, HookMatcherToken, HookOutputEntry, HookOutputEntryKind,
    HookPolicyDecision, HookPolicyDecisionReason, HookPolicyDecisionReceipt,
    HookPolicyDynamicAction, HookPolicyDynamicActionApplicationEffect,
    HookPolicyDynamicActionApplicationReason, HookPolicyDynamicActionApplicationReceipt,
    HookPolicyDynamicActionApplicationStatus, HookPolicyDynamicActionKind,
    HookPolicyDynamicActionReason, HookPolicyDynamicActionReplacement,
    HookPolicyDynamicActionTarget, HookPolicyExtension, HookPolicyExtensionKind, HookPolicyMode,
    HookRegistryUpdateKind, HookRegistryUpdateReceipt, HookRunId, HookRunStatus, HookRunSummary,
    HookSchemeModule, HookSchemeProcedure, HookScope, HookSelectedCandidateInput,
    HookSelectionCandidateReceipt, HookSelectionSkipReason, HookSkippedCandidateInput, HookSource,
    HookSourcePath, HookTimestampMs, HookTrustStatus,
};
