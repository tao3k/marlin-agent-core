//! Typed graph-loop lifecycle, continuation, queue, and tool-batch receipts.

use serde::{Deserialize, Serialize};

use super::{GraphId, GraphLoopExecutionStatus, LoopGraph, NodeId, RunId};

/// Stable event identifier inside one graph-loop run.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphLoopEventId(String);

impl GraphLoopEventId {
    /// Creates a graph-loop event identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the event identifier as text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts the event identifier into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for GraphLoopEventId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GraphLoopEventId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable controller iteration identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphLoopIterationId(u64);

impl GraphLoopIterationId {
    /// Creates a graph-loop iteration identifier.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the numeric iteration identifier.
    pub fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for GraphLoopIterationId {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

/// Stable node execution identifier inside one run.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphNodeExecutionId(String);

impl GraphNodeExecutionId {
    /// Creates a graph node execution identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the node execution identifier as text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts the node execution identifier into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for GraphNodeExecutionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GraphNodeExecutionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable tool-call identifier projected from an assistant turn.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphToolCallId(String);

impl GraphToolCallId {
    /// Creates a tool-call identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the tool-call identifier as text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts the tool-call identifier into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for GraphToolCallId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GraphToolCallId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Minimal message role recorded by graph-loop events.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopMessageRole {
    System,
    User,
    Assistant,
    ToolResult,
    Custom(String),
}

/// Lifecycle event emitted by the graph-loop runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GraphLoopEvent {
    AgentStart {
        graph_id: GraphId,
    },
    TurnStart,
    MessageStart {
        role: GraphLoopMessageRole,
    },
    MessageUpdate {
        role: GraphLoopMessageRole,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content_digest: Option<String>,
    },
    MessageEnd {
        role: GraphLoopMessageRole,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content_digest: Option<String>,
    },
    ToolExecutionStart {
        tool_call_id: GraphToolCallId,
        tool_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        args_digest: Option<String>,
    },
    ToolExecutionUpdate {
        tool_call_id: GraphToolCallId,
        tool_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        update_digest: Option<String>,
    },
    ToolExecutionEnd {
        receipt: GraphToolCallReceipt,
    },
    TurnEnd {
        #[serde(default)]
        tool_result_count: usize,
    },
    AgentEnd {
        status: GraphLoopExecutionStatus,
    },
}

/// Event envelope preserving run, iteration, graph node, and trace correlation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopEventEnvelope {
    pub run_id: RunId,
    pub event_id: GraphLoopEventId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iteration_id: Option<GraphLoopIterationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    pub timestamp_ms: u64,
    pub event: GraphLoopEvent,
}

impl GraphLoopEventEnvelope {
    /// Creates a graph-loop event envelope.
    pub fn new(
        run_id: impl Into<RunId>,
        event_id: impl Into<GraphLoopEventId>,
        timestamp_ms: u64,
        event: GraphLoopEvent,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            event_id: event_id.into(),
            iteration_id: None,
            node_id: None,
            trace_id: None,
            timestamp_ms,
            event,
        }
    }

    /// Attaches the controller iteration identifier.
    pub fn with_iteration_id(mut self, iteration_id: impl Into<GraphLoopIterationId>) -> Self {
        self.iteration_id = Some(iteration_id.into());
        self
    }

    /// Attaches the graph node identifier.
    pub fn with_node_id(mut self, node_id: impl Into<NodeId>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }

    /// Attaches a trace correlation identifier.
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

/// Input queue lane used by the graph-loop controller.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopInputLane {
    Steering,
    FollowUp,
}

/// Queue drain policy for steering and follow-up input lanes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopInputDrainPolicy {
    DrainAll,
    DrainOne,
}

/// A queued input fact without transcript body ownership.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopQueuedInput {
    pub lane: GraphLoopInputLane,
    pub input_id: String,
    pub role: GraphLoopMessageRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_digest: Option<String>,
}

/// Receipt for one graph-loop queue drain point.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopInputQueueReceipt {
    pub lane: GraphLoopInputLane,
    pub drain_policy: GraphLoopInputDrainPolicy,
    pub queued_count_before: usize,
    pub drained_inputs: Vec<GraphLoopQueuedInput>,
}

impl GraphLoopInputQueueReceipt {
    /// Creates a queue drain receipt.
    pub fn new(
        lane: GraphLoopInputLane,
        drain_policy: GraphLoopInputDrainPolicy,
        queued_count_before: usize,
        drained_inputs: Vec<GraphLoopQueuedInput>,
    ) -> Self {
        Self {
            lane,
            drain_policy,
            queued_count_before,
            drained_inputs,
        }
    }
}

/// Continuation action chosen after a graph-loop iteration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum GraphLoopContinuationAction {
    Accept,
    Deny { reason: String },
    Defer { reason: String },
    Rewrite { graph: LoopGraph, reason: String },
}

/// Typed receipt attached to a graph-loop continuation decision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopContinuationReceipt {
    pub run_id: RunId,
    pub iteration_id: GraphLoopIterationId,
    pub action: GraphLoopContinuationAction,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

impl GraphLoopContinuationReceipt {
    /// Creates a continuation receipt.
    pub fn new(
        run_id: impl Into<RunId>,
        iteration_id: impl Into<GraphLoopIterationId>,
        action: GraphLoopContinuationAction,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            iteration_id: iteration_id.into(),
            action,
            diagnostics: Vec::new(),
        }
    }

    /// Appends a diagnostic entry.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostics.push(diagnostic.into());
        self
    }
}

/// Readiness-aware continuation decision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopContinuationDecision {
    pub receipt: GraphLoopContinuationReceipt,
}

impl GraphLoopContinuationDecision {
    /// Creates a continuation decision from a typed receipt.
    pub fn new(receipt: GraphLoopContinuationReceipt) -> Self {
        Self { receipt }
    }

    /// Returns true when the decision allows the loop to proceed.
    pub fn allows_progress(&self) -> bool {
        matches!(
            self.receipt.action,
            GraphLoopContinuationAction::Accept | GraphLoopContinuationAction::Rewrite { .. }
        )
    }
}

/// Tool execution mode for a graph node frontier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphToolBatchExecutionMode {
    Sequential,
    Parallel,
}

/// Terminal status for one graph tool call.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphToolCallStatus {
    Prepared,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

/// Receipt for one graph tool call.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphToolCallReceipt {
    pub node_execution_id: GraphNodeExecutionId,
    pub tool_call_id: GraphToolCallId,
    pub tool_name: String,
    pub status: GraphToolCallStatus,
    #[serde(default)]
    pub is_error: bool,
    #[serde(default)]
    pub terminate: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

impl GraphToolCallReceipt {
    /// Creates a receipt for one graph tool call.
    pub fn new(
        node_execution_id: impl Into<GraphNodeExecutionId>,
        tool_call_id: impl Into<GraphToolCallId>,
        tool_name: impl Into<String>,
        status: GraphToolCallStatus,
    ) -> Self {
        Self {
            node_execution_id: node_execution_id.into(),
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
            status,
            is_error: false,
            terminate: false,
            result_digest: None,
            diagnostics: Vec::new(),
        }
    }

    /// Marks this tool call as an error.
    pub fn with_error(mut self) -> Self {
        self.is_error = true;
        self
    }

    /// Marks this tool call as requesting batch termination.
    pub fn with_terminate(mut self) -> Self {
        self.terminate = true;
        self
    }

    /// Attaches a result digest.
    pub fn with_result_digest(mut self, result_digest: impl Into<String>) -> Self {
        self.result_digest = Some(result_digest.into());
        self
    }

    /// Appends a diagnostic entry.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostics.push(diagnostic.into());
        self
    }
}

/// Batch-level decision after finalizing graph tool calls.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphToolBatchDecision {
    Continue,
    Terminate,
}

impl GraphToolBatchDecision {
    /// Derives the batch decision from finalized tool-call receipts.
    pub fn from_tool_receipts(receipts: &[GraphToolCallReceipt]) -> Self {
        if !receipts.is_empty() && receipts.iter().all(|receipt| receipt.terminate) {
            Self::Terminate
        } else {
            Self::Continue
        }
    }
}

/// Receipt for one graph tool batch execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphToolBatchExecutionReceipt {
    pub execution_mode: GraphToolBatchExecutionMode,
    pub tool_receipts: Vec<GraphToolCallReceipt>,
    pub decision: GraphToolBatchDecision,
}

impl GraphToolBatchExecutionReceipt {
    /// Creates a tool batch receipt and derives its batch decision.
    pub fn new(
        execution_mode: GraphToolBatchExecutionMode,
        tool_receipts: Vec<GraphToolCallReceipt>,
    ) -> Self {
        let decision = GraphToolBatchDecision::from_tool_receipts(&tool_receipts);
        Self {
            execution_mode,
            tool_receipts,
            decision,
        }
    }
}

/// Graph-loop stop reason preserved independently from terminal execution status.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopStopReason {
    Completed,
    Failed,
    Cancelled,
    BudgetExhausted,
    Graceful,
    Planner,
    HumanGate,
}

/// Receipt explaining why the controller stopped.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopStopReceipt {
    pub run_id: RunId,
    pub iteration_id: GraphLoopIterationId,
    pub reason: GraphLoopStopReason,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

impl GraphLoopStopReceipt {
    /// Creates a graph-loop stop receipt.
    pub fn new(
        run_id: impl Into<RunId>,
        iteration_id: impl Into<GraphLoopIterationId>,
        reason: GraphLoopStopReason,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            iteration_id: iteration_id.into(),
            reason,
            diagnostics: Vec::new(),
        }
    }

    /// Appends a diagnostic entry.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostics.push(diagnostic.into());
        self
    }
}
