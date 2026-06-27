//! Tokio graph-loop kernel and node executor adapters.

mod adapters;
mod controller;
mod driver;
mod loop_machine;
mod loop_program_controller;
mod loop_program_execution;
mod loop_program_handoff;
mod loop_program_handoff_executor;
mod loop_program_side_effects;

pub use adapters::{ProviderNodeAdapter, SubAgentNodeAdapter, ToolNodeAdapter};
pub use controller::{
    GraphLoopContinuationInput, GraphLoopContinuationPlanner, GraphLoopController,
    TerminalGraphLoopContinuationPlanner, TokioGraphLoopController,
};
pub use driver::{
    GraphLoopKernel, GraphNodeExecutor, GraphPolicyProposalCompilation, TokioGraphLoopKernel,
    compile_graph_policy_proposal, compile_graph_policy_proposal_with_native_abi_readiness,
};
pub use loop_machine::{
    GenericLoopMachine, GenericLoopMachineError, GenericLoopMachineReceipt, GenericLoopMachineStep,
    GenericLoopMachineStepIndex,
};
pub use loop_program_controller::{
    LoopProgramRunReceipt, LoopProgramRunRequest, LoopProgramRunStatus,
};
pub use loop_program_execution::{
    LoopProgramEventMapper, LoopProgramExecutionDriver, LoopProgramExecutionReceipt,
    LoopProgramExecutionRequest, LoopProgramExecutionStatus, LoopProgramExecutionStepReceipt,
    ReceiptDrivenLoopProgramEventMapper, ScriptedLoopProgramEventMapper,
    TerminalLoopProgramEventMapper,
};
pub use loop_program_handoff::{
    LoopProgramRuntimeHandoff, LoopProgramRuntimeHandoffKind, LoopProgramRuntimeHandoffPlan,
};
pub use loop_program_handoff_executor::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, DeferredLoopProgramRuntimeHandoffHandler,
    DenylistedLoopProgramToolDispatchHandler, HybridLoopProgramRuntimeHandoffExecutor,
    LoopProgramAgentFlowRuntimeHandoffRequest, LoopProgramMemoryProjectionReceipt,
    LoopProgramRuntimeHandoffExecution, LoopProgramRuntimeHandoffExecutionReceipt,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffExecutionStatus,
    LoopProgramRuntimeHandoffExecutor, LoopProgramRuntimeHandoffHandler,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, LoopProgramToolProcessProgram,
    LoopProgramToolProcessProjectionReceipt, LoopProgramToolProcessSpawnReceipt,
    LoopProgramToolProcessSpawnRequest, PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor,
    RetryBudgetToolHandler, StaticLoopProgramRuntimeHandoffHandler,
    spawn_loop_program_tool_process,
};
pub use loop_program_side_effects::{
    LoopProgramDerivedSessionPolicyStatus, LoopProgramExecutionReplayBundleReceipt,
    LoopProgramFileSandbox, LoopProgramFileWriteReceipt, LoopProgramFileWriteRequest,
    LoopProgramFileWriteResolver, LoopProgramFileWriteSideEffectReceipt,
    LoopProgramFileWriteSideEffectStatus, LoopProgramFileWriteTemplate,
    LoopProgramRuntimeReplayBundleReceipt, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramRuntimeSideEffectReceipt, LoopProgramRuntimeSideEffectStatus,
    LoopProgramToolProcessCommandTemplate, LoopProgramToolProcessResolver,
    LoopProgramToolProcessSideEffectReceipt, LoopProgramToolProcessSideEffectStatus,
    StaticLoopProgramFileWriteResolver, StaticLoopProgramToolProcessResolver,
};
pub use marlin_agent_protocol::{
    ExecutorName, FailureClassificationId, FailureClassificationReceipt, GraphId,
    GraphLoopContinuationAction, GraphLoopContinuationDecision, GraphLoopContinuationReceipt,
    GraphLoopEvidencePolicy, GraphLoopExecutionBudget, GraphLoopExecutionRequest,
    GraphLoopExecutionResult, GraphLoopExecutionStatus, GraphLoopFailureKind,
    GraphLoopGovernancePolicy, GraphLoopGovernedContextNamespace, GraphLoopGovernedSessionKind,
    GraphLoopIterationReport, GraphLoopNextAction, GraphLoopRunRequest, GraphLoopSandboxBackend,
    GraphLoopSandboxPolicy, GraphLoopSessionPolicy, GraphLoopStatePolicy, GraphLoopStopPolicy,
    GraphLoopStrategy, GraphLoopStrategyId, GraphLoopStrategyRuntime, GraphLoopStrategyVersion,
    GraphLoopVerifierPolicy, GraphNodeExecutionReceipt, GraphNodeExecutionStatus,
    GraphNodeInvocation, GraphPolicyProposal, GraphPolicyProposalReceipt,
    GraphPolicyProposalStatus, HumanDecision, HumanDecisionReceipt, HumanGateId, HumanGateReceipt,
    HumanReviewKind, HumanReviewerId, LoopContinuationCapability, LoopContinuationPolicy,
    LoopEdgeSpec, LoopEvidenceCapturePolicy, LoopFailurePolicy, LoopGraph, LoopHumanGatePolicy,
    LoopMemoryPolicy, LoopModelRoutePolicy, LoopNodeSpec, LoopPolicyProfile, LoopPolicyProfileId,
    LoopQueuePolicy, LoopSelfEvolutionPolicy, LoopToolBatchPolicy, NodeId, RunId,
    RuntimePlanSnapshot,
};
