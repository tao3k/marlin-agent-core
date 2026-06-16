//! Multi-iteration graph-loop controller contracts.

use serde::{Deserialize, Serialize};

use crate::trace::AgentExecutionTrace;

use super::loop_event::{GraphLoopContinuationReceipt, GraphLoopIterationId};
use super::{
    GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopExecutionResult, LoopGraph,
    LoopPolicyProfile, NodeId, RunId,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_profile: Option<LoopPolicyProfile>,
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
            policy_profile: None,
            stop_policy: GraphLoopStopPolicy::new(),
            iteration_budget: None,
            evidence_policy: GraphLoopEvidencePolicy::default(),
        }
    }

    /// Attaches a typed configurable loop policy profile to this run request.
    pub fn with_policy_profile(mut self, policy_profile: LoopPolicyProfile) -> Self {
        self.policy_profile = Some(policy_profile);
        self
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

/// Stable human gate identifier inside one graph-loop run.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HumanGateId(String);

impl HumanGateId {
    /// Creates a human gate identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the human gate identifier as text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts the human gate identifier into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for HumanGateId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HumanGateId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable human reviewer identifier for a human decision receipt.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HumanReviewerId(String);

impl HumanReviewerId {
    /// Creates a human reviewer identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the reviewer identifier as text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts the reviewer identifier into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for HumanReviewerId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HumanReviewerId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable failure classification identifier inside one graph-loop run.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FailureClassificationId(String);

impl FailureClassificationId {
    /// Creates a failure classification identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the failure classification identifier as text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts the failure classification identifier into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for FailureClassificationId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for FailureClassificationId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Human review category required before a graph-loop continuation can resume.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HumanReviewKind {
    General,
    PolicyChange,
    PermissionEscalation,
    MemoryPromotion,
    CrossProjectReference,
    SessionVisibilityExpansion,
    DataMigration,
}

/// Human decision recorded after a human gate has been reviewed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HumanDecision {
    Approved,
    Rejected,
    NeedsQuestion,
    PatchRequested,
}

/// Typed receipt emitted when a graph loop must stop for human review.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HumanGateReceipt {
    pub gate_id: HumanGateId,
    pub run_id: RunId,
    pub iteration_id: GraphLoopIterationId,
    pub reason: String,
    pub required_review: HumanReviewKind,
    pub proposed_next_action: GraphLoopNextAction,
}

impl HumanGateReceipt {
    /// Creates a human gate receipt for the proposed continuation action.
    pub fn new(
        gate_id: impl Into<HumanGateId>,
        run_id: impl Into<RunId>,
        iteration_id: impl Into<GraphLoopIterationId>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            gate_id: gate_id.into(),
            run_id: run_id.into(),
            iteration_id: iteration_id.into(),
            reason: reason.into(),
            required_review: HumanReviewKind::General,
            proposed_next_action: GraphLoopNextAction::StopFailed,
        }
    }

    /// Sets the human review kind required by this gate.
    pub fn with_required_review(mut self, required_review: HumanReviewKind) -> Self {
        self.required_review = required_review;
        self
    }

    /// Sets the continuation action proposed before the gate was raised.
    pub fn with_proposed_next_action(mut self, proposed_next_action: GraphLoopNextAction) -> Self {
        self.proposed_next_action = proposed_next_action;
        self
    }
}

/// Typed receipt emitted when a human gate receives a decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HumanDecisionReceipt {
    pub gate_id: HumanGateId,
    pub decision: HumanDecision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<HumanReviewerId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_next_graph: Option<LoopGraph>,
}

impl HumanDecisionReceipt {
    /// Creates a human decision receipt for an existing gate.
    pub fn new(gate_id: impl Into<HumanGateId>, decision: HumanDecision) -> Self {
        Self {
            gate_id: gate_id.into(),
            decision,
            reviewer: None,
            approved_next_graph: None,
        }
    }

    /// Attaches the reviewer identity that made the decision.
    pub fn with_reviewer(mut self, reviewer: impl Into<HumanReviewerId>) -> Self {
        self.reviewer = Some(reviewer.into());
        self
    }

    /// Attaches the approved resume graph.
    pub fn with_approved_next_graph(mut self, graph: LoopGraph) -> Self {
        self.approved_next_graph = Some(graph);
        self
    }
}

/// Failure class selected after a graph-loop iteration fails.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopFailureKind {
    TransientFailure,
    ToolUsageFailure,
    VerificationFailure,
    ContextFailure,
    PolicyFailure,
    StrategyFailure,
    Unknown,
}

/// Typed receipt that classifies a failed graph-loop iteration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FailureClassificationReceipt {
    pub classification_id: FailureClassificationId,
    pub run_id: RunId,
    pub iteration_id: GraphLoopIterationId,
    pub failure_kind: GraphLoopFailureKind,
    #[serde(default)]
    pub retryable: bool,
    #[serde(default)]
    pub requires_human: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_nodes: Vec<NodeId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_recovery_graph: Option<LoopGraph>,
}

impl FailureClassificationReceipt {
    /// Creates a failure classification receipt.
    pub fn new(
        classification_id: impl Into<FailureClassificationId>,
        run_id: impl Into<RunId>,
        iteration_id: impl Into<GraphLoopIterationId>,
        failure_kind: GraphLoopFailureKind,
    ) -> Self {
        Self {
            classification_id: classification_id.into(),
            run_id: run_id.into(),
            iteration_id: iteration_id.into(),
            failure_kind,
            retryable: false,
            requires_human: false,
            source_nodes: Vec::new(),
            diagnostics: Vec::new(),
            suggested_recovery_graph: None,
        }
    }

    /// Marks whether the failure can be retried without operator review.
    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    /// Marks whether recovery requires human review.
    pub fn with_requires_human(mut self, requires_human: bool) -> Self {
        self.requires_human = requires_human;
        self
    }

    /// Adds a source node that contributed to classification.
    pub fn with_source_node(mut self, node_id: impl Into<NodeId>) -> Self {
        self.source_nodes.push(node_id.into());
        self
    }

    /// Adds a classifier diagnostic.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostics.push(diagnostic.into());
        self
    }

    /// Attaches the proposed recovery graph.
    pub fn with_suggested_recovery_graph(mut self, graph: LoopGraph) -> Self {
        self.suggested_recovery_graph = Some(graph);
        self
    }
}

/// Report for one controller iteration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopIterationReport {
    pub iteration: u64,
    pub execution_result: GraphLoopExecutionResult,
    pub next_action: GraphLoopNextAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation_receipt: Option<GraphLoopContinuationReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub human_gate_receipt: Option<HumanGateReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub human_decision_receipt: Option<HumanDecisionReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_classification_receipt: Option<FailureClassificationReceipt>,
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
            continuation_receipt: None,
            human_gate_receipt: None,
            human_decision_receipt: None,
            failure_classification_receipt: None,
            trace: None,
        }
    }

    /// Attaches the continuation decision receipt for this iteration.
    pub fn with_continuation_receipt(mut self, receipt: GraphLoopContinuationReceipt) -> Self {
        self.continuation_receipt = Some(receipt);
        self
    }

    /// Attaches the human gate receipt for this iteration.
    pub fn with_human_gate_receipt(mut self, receipt: HumanGateReceipt) -> Self {
        self.human_gate_receipt = Some(receipt);
        self
    }

    /// Attaches the human decision receipt that resumes this iteration.
    pub fn with_human_decision_receipt(mut self, receipt: HumanDecisionReceipt) -> Self {
        self.human_decision_receipt = Some(receipt);
        self
    }

    /// Attaches the failure classification receipt for this iteration.
    pub fn with_failure_classification_receipt(
        mut self,
        receipt: FailureClassificationReceipt,
    ) -> Self {
        self.failure_classification_receipt = Some(receipt);
        self
    }

    /// Attaches a typed execution trace to the iteration report.
    pub fn with_trace(mut self, trace: AgentExecutionTrace) -> Self {
        self.trace = Some(trace);
        self
    }
}
