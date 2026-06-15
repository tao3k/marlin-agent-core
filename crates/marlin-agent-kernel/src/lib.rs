//! Tokio graph-loop kernel and node executor adapters.

mod adapters;
mod controller;
mod driver;

pub use adapters::{ProviderNodeAdapter, SubAgentNodeAdapter, ToolNodeAdapter};
pub use controller::{
    GraphLoopContinuationInput, GraphLoopContinuationPlanner, GraphLoopController,
    TerminalGraphLoopContinuationPlanner, TokioGraphLoopController,
};
pub use driver::{
    GraphLoopKernel, GraphNodeExecutor, GraphPolicyProposalCompilation, TokioGraphLoopKernel,
    compile_graph_policy_proposal, compile_graph_policy_proposal_with_native_abi_readiness,
};
pub use marlin_agent_protocol::{
    ExecutorName, GraphId, GraphLoopEvidencePolicy, GraphLoopExecutionBudget,
    GraphLoopExecutionRequest, GraphLoopExecutionResult, GraphLoopExecutionStatus,
    GraphLoopIterationReport, GraphLoopNextAction, GraphLoopRunRequest, GraphLoopStopPolicy,
    GraphLoopStrategy, GraphLoopStrategyId, GraphLoopStrategyRuntime, GraphLoopStrategyVersion,
    GraphNodeExecutionReceipt, GraphNodeExecutionStatus, GraphNodeInvocation, GraphPolicyProposal,
    GraphPolicyProposalReceipt, GraphPolicyProposalStatus, LoopEdgeSpec, LoopGraph, LoopNodeSpec,
    NodeId, RunId, RuntimePlanSnapshot,
};
