//! Graph-loop owner interface.

mod contract;
mod controller;
mod execution_budget;
mod gerbil_policy;
mod loop_event;
mod native_abi;
mod policy_profile;

pub use contract::{
    ExecutorName, GRAPH_POLICY_PROPOSAL_SCHEMA_ID, GraphId, GraphLoopExecutionRequest,
    GraphLoopExecutionResult, GraphLoopExecutionStatus, GraphLoopStrategy, GraphLoopStrategyId,
    GraphLoopStrategyRuntime, GraphLoopStrategyVersion, GraphNodeExecutionReceipt,
    GraphNodeExecutionStatus, GraphNodeInvocation, GraphPolicyDigest, GraphPolicyProposal,
    GraphPolicyProposalReceipt, GraphPolicyProposalStatus, GraphPolicyProposalValidationReport,
    LoopEdgeSpec, LoopGraph, LoopNodeSpec, NodeId, RunId, RuntimePlanSnapshot,
    validate_graph_policy_proposal,
};
pub use controller::{
    FailureClassificationId, FailureClassificationReceipt, GraphLoopEvidencePolicy,
    GraphLoopFailureKind, GraphLoopGovernancePolicy, GraphLoopGovernedContextNamespace,
    GraphLoopGovernedSessionKind, GraphLoopIterationReport, GraphLoopNextAction,
    GraphLoopRunRequest, GraphLoopSandboxBackend, GraphLoopSandboxPolicy, GraphLoopSessionPolicy,
    GraphLoopStatePolicy, GraphLoopStopPolicy, GraphLoopVerifierPolicy, HumanDecision,
    HumanDecisionReceipt, HumanGateId, HumanGateReceipt, HumanReviewKind, HumanReviewerId,
};
pub use execution_budget::GraphLoopExecutionBudget;
pub use gerbil_policy::{
    GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID, GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID,
    GerbilLoopGraphContinuationAction, GerbilLoopGraphContinuationCompileError,
    GerbilLoopGraphContinuationRequest, GerbilLoopGraphPolicyCompilationRequest,
    compile_gerbil_loop_graph, compile_gerbil_loop_graph_continuation,
    compile_gerbil_loop_graph_policy,
};
pub use loop_event::{
    GraphLoopContinuationAction, GraphLoopContinuationDecision, GraphLoopContinuationReceipt,
    GraphLoopEvent, GraphLoopEventEnvelope, GraphLoopEventId, GraphLoopInputDrainPolicy,
    GraphLoopInputLane, GraphLoopInputQueueReceipt, GraphLoopIterationId, GraphLoopMessageRole,
    GraphLoopQueuedInput, GraphLoopStopReason, GraphLoopStopReceipt, GraphNodeExecutionId,
    GraphToolBatchDecision, GraphToolBatchExecutionMode, GraphToolBatchExecutionReceipt,
    GraphToolCallId, GraphToolCallReceipt, GraphToolCallStatus,
};
pub use native_abi::{
    GraphNativeAbiId, GraphNativeAbiReadinessReceipt, GraphNativeAbiReadinessStatus,
    GraphNativeAbiRequirement, GraphNativeSymbol,
};
pub use policy_profile::{
    LoopContinuationCapability, LoopContinuationPolicy, LoopEvidenceCapturePolicy,
    LoopFailurePolicy, LoopHumanGatePolicy, LoopMemoryPolicy, LoopModelRoutePolicy,
    LoopPolicyProfile, LoopPolicyProfileId, LoopQueuePolicy, LoopSelfEvolutionPolicy,
    LoopToolBatchPolicy,
};
