//! Typed `AgentGraph` topology contracts for Marlin agent coordination.

mod ids;
mod planning;
mod receipt;
mod topology;

pub use ids::{
    AgentCapability, AgentDelegationReason, AgentEdgeId, AgentElectionReason, AgentEvidenceId,
    AgentGraphId, AgentGraphValidationError, AgentNodeId, AgentPolicyDecisionRef, AgentRole,
    AgentRoutingReason, AgentTopologyPolicyId, GerbilPolicyScopeRef, GraphLoopGraphRef,
    GraphLoopNodeRef, OrgMemoryScopeRef,
};
pub use planning::{
    AgentGraphPlanningReceipt, AgentGraphPlanningRejection, AgentGraphPlanningStatus,
    AgentGraphPlanningTarget, plan_agent_coordination, plan_agent_coordination_with_policy_receipt,
};
pub use receipt::{
    AgentCoordinationDecision, AgentCoordinationEvidenceKind, AgentCoordinationEvidenceRef,
    AgentCoordinationReceipt, AgentElectionReceipt, AgentPolicyRoutingDecision,
    AgentPolicyRoutingReceipt, AgentRoutingReceipt,
};
pub use topology::{
    AgentCoordinationPlan, AgentDelegation, AgentEdge, AgentEdgeCondition, AgentEdgeKind,
    AgentGraph, AgentGraphTopologyError, AgentNode, AgentTopologyPolicy, GraphLoopEntryRef,
};
