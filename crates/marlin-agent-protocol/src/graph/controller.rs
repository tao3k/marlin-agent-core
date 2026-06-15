//! Multi-iteration graph-loop controller contracts.

use serde::{Deserialize, Serialize};

use crate::trace::AgentExecutionTrace;

use super::{
    GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopExecutionResult, LoopGraph,
};

/// Policy that bounds a multi-iteration graph-loop run.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopStopPolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_duration_ms: Option<u64>,
    #[serde(default)]
    pub stop_on_failed_execution: bool,
    #[serde(default)]
    pub human_gate_required: bool,
}

impl GraphLoopStopPolicy {
    /// Creates an unbounded stop policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stops after at most `max_iterations` controller iterations.
    pub fn max_iterations(max_iterations: u64) -> Self {
        Self::new().with_max_iterations(max_iterations)
    }

    /// Sets the maximum controller iterations.
    pub fn with_max_iterations(mut self, max_iterations: u64) -> Self {
        self.max_iterations = Some(max_iterations);
        self
    }

    /// Sets the maximum wall-clock duration in milliseconds.
    pub fn with_max_duration_ms(mut self, max_duration_ms: u64) -> Self {
        self.max_duration_ms = Some(max_duration_ms);
        self
    }

    /// Stops the loop when an iteration execution fails.
    pub fn stop_on_failed_execution(mut self) -> Self {
        self.stop_on_failed_execution = true;
        self
    }

    /// Requires a human gate before continuation.
    pub fn require_human_gate(mut self) -> Self {
        self.human_gate_required = true;
        self
    }
}

/// Evidence capture policy for a multi-iteration graph-loop run.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopEvidencePolicy {
    #[serde(default)]
    pub capture_runtime_events: bool,
    #[serde(default)]
    pub capture_node_receipts: bool,
    #[serde(default)]
    pub capture_trace: bool,
    #[serde(default)]
    pub replayable: bool,
}

impl GraphLoopEvidencePolicy {
    /// Captures the evidence needed for deterministic no-live-LLM replay.
    pub fn replayable_runtime() -> Self {
        Self {
            capture_runtime_events: true,
            capture_node_receipts: true,
            capture_trace: true,
            replayable: true,
        }
    }
}

/// Request to run a bounded multi-iteration graph loop.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopRunRequest {
    pub initial_request: GraphLoopExecutionRequest,
    pub stop_policy: GraphLoopStopPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iteration_budget: Option<GraphLoopExecutionBudget>,
    pub evidence_policy: GraphLoopEvidencePolicy,
}

impl GraphLoopRunRequest {
    /// Creates a controller request from the first executable graph request.
    pub fn new(initial_request: GraphLoopExecutionRequest) -> Self {
        Self {
            initial_request,
            stop_policy: GraphLoopStopPolicy::new(),
            iteration_budget: None,
            evidence_policy: GraphLoopEvidencePolicy::default(),
        }
    }

    /// Sets the controller stop policy.
    pub fn with_stop_policy(mut self, stop_policy: GraphLoopStopPolicy) -> Self {
        self.stop_policy = stop_policy;
        self
    }

    /// Sets the per-iteration execution budget.
    pub fn with_iteration_budget(mut self, iteration_budget: GraphLoopExecutionBudget) -> Self {
        self.iteration_budget = Some(iteration_budget);
        self
    }

    /// Sets the evidence capture policy.
    pub fn with_evidence_policy(mut self, evidence_policy: GraphLoopEvidencePolicy) -> Self {
        self.evidence_policy = evidence_policy;
        self
    }
}

/// Controller decision after one graph-loop iteration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopNextAction {
    StopCompleted,
    StopFailed,
    ContinueWithGraph(LoopGraph),
    EscalateToHuman { reason: String },
}

/// Report for one controller iteration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopIterationReport {
    pub iteration: u64,
    pub execution_result: GraphLoopExecutionResult,
    pub next_action: GraphLoopNextAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<AgentExecutionTrace>,
}

impl GraphLoopIterationReport {
    /// Creates one iteration report from the terminal execution result and next action.
    pub fn new(
        iteration: u64,
        execution_result: GraphLoopExecutionResult,
        next_action: GraphLoopNextAction,
    ) -> Self {
        Self {
            iteration,
            execution_result,
            next_action,
            trace: None,
        }
    }

    /// Attaches a typed execution trace to the iteration report.
    pub fn with_trace(mut self, trace: AgentExecutionTrace) -> Self {
        self.trace = Some(trace);
        self
    }
}
