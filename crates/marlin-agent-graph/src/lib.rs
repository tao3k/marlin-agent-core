// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed `AgentGraph` topology contracts for Marlin agent coordination.

mod governed_context;
mod ids;
mod planning;
mod receipt;
mod task_actor;
mod topology;

pub use governed_context::{
    AdmittedGovernedContext, DetourExitKind, DetourMode, DetourNodeId, DetourState,
    DisclosureDepth, GovernedContextCapsule, GovernedContextRejection, KvPrefixDigest,
    admit_governed_context_capsule,
};

pub use ids::{
    AgentCapability, AgentDelegationReason, AgentEdgeId, AgentElectionReason, AgentEvidenceId,
    AgentGraphId, AgentGraphValidationError, AgentNodeId, AgentPolicyDecisionRef, AgentRole,
    AgentRoutingReason, AgentTopologyPolicyId, GerbilPolicyScopeRef, GraphLoopGraphRef,
    GraphLoopNodeRef, OrgMemoryScopeRef,
};
pub use planning::{
    AgentGraphPlanningReceipt, AgentGraphPlanningRejection, AgentGraphPlanningStatus,
    AgentGraphPlanningTarget, AgentGraphRoutingProjectionRejection, plan_agent_coordination,
    plan_agent_coordination_with_policy_receipt, project_agent_candidate_routing_receipts,
};
pub use receipt::{
    AgentCoordinationDecision, AgentCoordinationEvidenceKind, AgentCoordinationEvidenceRef,
    AgentCoordinationReceipt, AgentElectionReceipt, AgentPolicyRoutingDecision,
    AgentPolicyRoutingReceipt, AgentRoutingReceipt,
};
pub use task_actor::{
    AdmittedMarlinDispatch, DispatchRequestId, ExecutionReceiptStatus, FencingEpoch,
    GraphConvergenceNodeId, GraphDispatchOperation, GraphDispatchRejection, GraphSubstrateTarget,
    MarlinDispatchEnvelope, MarlinExecutionReceipt, ReceiptReconciliationDecision,
    ReceiptReconciliationReason, TaskActorCursor, TaskActorId, TaskActorScopeMode,
    TaskActorStatus, WorkbenchActiveTaskInput, WorkbenchTabEntry, WorkbenchTabKind,
    WorkbenchTabProjection, admit_marlin_dispatch, project_workbench_tabs,
    reconcile_marlin_execution_receipt,
};
pub use topology::{
    AgentCoordinationPlan, AgentDelegation, AgentEdge, AgentEdgeCondition, AgentEdgeKind,
    AgentGraph, AgentGraphTopologyError, AgentNode, AgentTopologyPolicy, GraphLoopEntryRef,
};
