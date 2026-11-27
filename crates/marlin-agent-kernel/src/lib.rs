// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Tokio graph-loop kernel and node executor adapters.

mod adapters;
mod controller;
mod driver;
pub mod context_distillation;
pub mod execution_breaker;
pub mod governance_admission;
pub mod governance_cedar;
pub mod governance_plane;
pub mod model_weight_attestation;
pub mod transparency_log;
mod loop_machine;
mod loop_program_controller;
mod loop_program_execution;
mod loop_program_handoff;
mod loop_program_handoff_executor;
mod loop_program_policy_matrix;
mod loop_program_side_effects;
mod task_actor_executor;

pub use adapters::{ProviderNodeAdapter, SubAgentNodeAdapter, ToolNodeAdapter};
pub use controller::{
    GraphLoopContinuationInput, GraphLoopContinuationPlanner, GraphLoopController,
    GraphLoopHumanReviewError, GraphLoopHumanReviewHandle, TerminalGraphLoopContinuationPlanner,
    TokioGraphLoopController,
};
pub use driver::{
    GraphLoopKernel, GraphNodeExecutor, GraphPolicyProposalCompilation, TokioGraphLoopKernel,
    compile_graph_policy_proposal_with_c_abi_readiness, compile_static_graph_policy_proposal,
};
pub use execution_breaker::{
    BreakerActor, BreakerActorId, BreakerRejectionReason, BreakerTripEpoch, BreakerTripReason,
    ExecutionBreakerCommitReceipt, ExecutionBreakerRearmReceipt, ExecutionBreakerSnapshot,
    ExecutionBreakerTripReceipt, ExecutionCircuitBreaker, ToolBudgetUnits, ToolExecutionLease,
    ToolExecutionLeaseId, ToolIfcContext,
};
pub use governance_admission::{
    CedarPolicyId, DerivationId, GOVERNANCE_ADMISSION_SCHEMA_VERSION,
    GOVERNANCE_DURABLE_CHECKPOINT_VERSION, GovernanceAdmissionError, GovernanceAdmissionGate,
    GovernanceDurableCheckpoint, GovernanceGateSnapshot, GovernanceProofDigest,
    GovernanceProofEnvelope, GovernanceProofEpoch, GovernanceProofSyncReceipt, GovernedSubjectName,
    SynthesizedCedarPolicy, governance_sha256_bytes,
};
pub use governance_cedar::{
    CedarContextValue, CedarEffect, CedarEntityRef, CedarEvaluationDecision,
    CedarEvaluationRequest, CedarScopeMatcher, CedarWhenCondition, CompiledCedarPolicy,
    compile_cedar_policy, evaluate_compiled_cedar_policies, infer_actor_roles,
};
pub use governance_plane::{
    AuthorityEpoch, BackendSecurityGovernanceCoordinator, ChatroomGateStatus,
    ChatroomIngressGovernor, ChatroomIngressReceipt, DelegatedAgentAuthzLedger,
    DelegatedAgentTicket, DelegatedExecutionReceipt, DelegationScopeLevel, DelegationTicketId,
    DualControlSodLedger, ExternalIngressRequest, GovernanceActorId, GovernanceChannelId,
    GovernanceGateId, GovernancePlaneRejection, GovernancePrincipal, GovernanceProposalId,
    GovernanceTenantId, MultiTenantIsolationFence, PrivilegedHumanReceipt, SodProposalRecord,
    TenantExecutionTicket, TenantExecutionTicketId, TenantOutboxEntry, TenantOutboxEntryId,
};
pub use context_distillation::{
    ContextDistillationGate, ContextDistillationReceipt, ContextDistillationRejection,
    ContextSegment, DistilledContextPacket, EpistemicIntegrity, SecrecyClass,
};
pub use model_weight_attestation::{
    ModelInferenceAdmissionReceipt, ModelInferenceClaim, ModelWeightAttestationGate,
    ModelWeightGovernancePolicy, ModelWeightRejection, compute_shard_merkle_root,
};
pub use transparency_log::{
    GovernanceTransparencyLog, GovernanceTreeHead, MerkleConsistencyProof, MerkleInclusionProof,
    SplitViewEquivocationFraudProof, TransparencyLogRejection, verify_inclusion_proof,
    verify_tree_head_progression,
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
pub use loop_program_policy_matrix::{
    RuntimePolicyExperimentCaseId, RuntimePolicyExperimentCount, RuntimePolicyExperimentDigest,
    RuntimePolicyExperimentReceipt, RuntimePolicyImprovementRecommendation,
    RuntimePolicyRecommendationAction, RuntimePolicyRecommendationEvidence,
    RuntimePolicyRecommendationPriority, RuntimePolicyRecommendationTarget,
    has_memory_recall_intent, runtime_policy_experiment_receipt,
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
pub use task_actor_executor::{TaskActorKernelExecutor, TaskActorStepHandler};
