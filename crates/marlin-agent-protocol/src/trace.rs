//! Serializable tracing contracts emitted by agent-owned execution.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::event::AgentEvent;
use crate::graph::{
    GraphId, GraphLoopExecutionStatus, GraphLoopStrategyId, GraphPolicyProposalReceipt,
    GraphPolicyProposalStatus, RunId,
};

/// Span name for Rust-side graph policy proposal validation and compilation.
pub const GRAPH_POLICY_PROPOSAL_SPAN_NAME: &str = "graph.policy_proposal";

/// Stable tracing span name identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AgentSpanName(String);

impl AgentSpanName {
    /// Creates a tracing span name identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the span name as text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts the span name into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for AgentSpanName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<String> for AgentSpanName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for AgentSpanName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Captured tracing span metadata for one span creation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentTraceSpanRecord {
    /// Stable span name, such as `agent.provider` or `hook.dispatch`.
    pub name: AgentSpanName,
    /// Field values recorded when the span was created.
    pub fields: BTreeMap<String, String>,
}

impl AgentTraceSpanRecord {
    /// Creates a span record with no captured fields.
    pub fn new(name: impl Into<AgentSpanName>) -> Self {
        Self {
            name: name.into(),
            fields: BTreeMap::new(),
        }
    }

    /// Adds one captured field value to this span record.
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Creates a span that exposes Rust-side graph policy proposal validation.
    pub fn graph_policy_proposal_receipt(receipt: &GraphPolicyProposalReceipt) -> Self {
        let mut span = Self::new(GRAPH_POLICY_PROPOSAL_SPAN_NAME)
            .with_field("schema_id", receipt.schema_id.clone())
            .with_field("strategy_id", receipt.strategy_id.as_str())
            .with_field("status", graph_policy_proposal_status_name(&receipt.status))
            .with_field("diagnostic_count", receipt.diagnostics.len().to_string());
        if let Some(graph_id) = &receipt.selected_graph_id {
            span = span.with_field("selected_graph_id", graph_id.as_str());
        }
        span
    }

    /// Returns true when this span records a graph policy proposal receipt.
    pub fn is_graph_policy_proposal(&self) -> bool {
        self.name.as_str() == GRAPH_POLICY_PROPOSAL_SPAN_NAME
    }

    /// Graph policy proposal strategy id captured by this span.
    pub fn graph_policy_proposal_strategy_id(&self) -> Option<GraphLoopStrategyId> {
        self.fields
            .get("strategy_id")
            .map(|strategy_id| GraphLoopStrategyId::new(strategy_id.clone()))
    }

    /// Graph policy proposal status captured by this span.
    pub fn graph_policy_proposal_status(&self) -> Option<GraphPolicyProposalStatus> {
        self.fields
            .get("status")
            .and_then(|status| graph_policy_proposal_status_from_name(status.as_str()))
    }

    /// Accepted graph id selected by this proposal span, when present.
    pub fn graph_policy_proposal_selected_graph_id(&self) -> Option<GraphId> {
        self.fields
            .get("selected_graph_id")
            .map(|graph_id| GraphId::new(graph_id.clone()))
    }
}

fn graph_policy_proposal_status_name(status: &GraphPolicyProposalStatus) -> &'static str {
    match status {
        GraphPolicyProposalStatus::Accepted => "Accepted",
        GraphPolicyProposalStatus::Rejected => "Rejected",
    }
}

fn graph_policy_proposal_status_from_name(name: &str) -> Option<GraphPolicyProposalStatus> {
    match name {
        "Accepted" => Some(GraphPolicyProposalStatus::Accepted),
        "Rejected" => Some(GraphPolicyProposalStatus::Rejected),
        _ => None,
    }
}

/// Compact execution trace facts for debugging one graph-loop run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentExecutionTrace {
    /// Graph-loop run identifier.
    pub run_id: RunId,
    /// Graph identifier executed for this trace.
    pub graph_id: GraphId,
    /// Terminal graph-loop status.
    pub status: GraphLoopExecutionStatus,
    /// Runtime events captured during execution.
    pub events: Vec<AgentEvent>,
    /// Tracing spans captured during execution.
    pub spans: Vec<AgentTraceSpanRecord>,
    /// Execution diagnostics emitted by the graph loop.
    pub diagnostics: Vec<String>,
}

/// Compact count-only summary for scanning many execution traces.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentExecutionTraceSummary {
    /// Graph-loop run identifier.
    pub run_id: RunId,
    /// Graph identifier executed for this trace.
    pub graph_id: GraphId,
    /// Terminal graph-loop status.
    pub status: GraphLoopExecutionStatus,
    /// Number of runtime events captured in the trace.
    pub event_count: usize,
    /// Number of tracing spans captured in the trace.
    pub span_count: usize,
    /// Number of execution diagnostics captured in the trace.
    pub diagnostic_count: usize,
}

impl AgentExecutionTrace {
    /// Creates an empty execution trace for a terminal graph-loop status.
    pub fn new(
        run_id: impl Into<RunId>,
        graph_id: impl Into<GraphId>,
        status: GraphLoopExecutionStatus,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            graph_id: graph_id.into(),
            status,
            events: Vec::new(),
            spans: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Adds captured runtime events to this execution trace.
    pub fn with_events(mut self, events: Vec<AgentEvent>) -> Self {
        self.events = events;
        self
    }

    /// Adds captured tracing spans to this execution trace.
    pub fn with_spans(mut self, spans: Vec<AgentTraceSpanRecord>) -> Self {
        self.spans = spans;
        self
    }

    /// Adds graph-loop diagnostics to this execution trace.
    pub fn with_diagnostics(mut self, diagnostics: Vec<String>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    /// Returns spans that record Rust-side graph policy proposal receipts.
    pub fn graph_policy_proposal_spans(&self) -> impl Iterator<Item = &AgentTraceSpanRecord> {
        self.spans
            .iter()
            .filter(|span| span.is_graph_policy_proposal())
    }

    /// Finds the first graph policy proposal span for a strategy id.
    pub fn find_graph_policy_proposal_span(
        &self,
        strategy_id: &GraphLoopStrategyId,
    ) -> Option<&AgentTraceSpanRecord> {
        self.graph_policy_proposal_spans().find(|span| {
            span.graph_policy_proposal_strategy_id()
                .is_some_and(|actual| &actual == strategy_id)
        })
    }

    /// Returns true when a strategy id has a graph policy proposal span with this status.
    pub fn has_graph_policy_proposal_status(
        &self,
        strategy_id: &GraphLoopStrategyId,
        status: GraphPolicyProposalStatus,
    ) -> bool {
        self.find_graph_policy_proposal_span(strategy_id)
            .and_then(AgentTraceSpanRecord::graph_policy_proposal_status)
            .is_some_and(|actual| actual == status)
    }
    /// Returns true when the trace captured at least one span with this name.
    pub fn has_span(&self, name: &AgentSpanName) -> bool {
        self.spans_by_name(name).next().is_some()
    }

    /// Counts spans captured with this name.
    pub fn count_span(&self, name: &AgentSpanName) -> usize {
        self.spans_by_name(name).count()
    }

    /// Returns spans captured with this name.
    pub fn spans_by_name(
        &self,
        name: &AgentSpanName,
    ) -> impl Iterator<Item = &AgentTraceSpanRecord> {
        let name = name.clone();
        self.spans.iter().filter(move |span| span.name == name)
    }

    /// Returns the first span captured with this name.
    pub fn find_span(&self, name: &AgentSpanName) -> Option<&AgentTraceSpanRecord> {
        self.spans_by_name(name).next()
    }

    /// Returns spans that captured a matching field value.
    pub fn spans_with_field(
        &self,
        field: &str,
        value: &str,
    ) -> impl Iterator<Item = &AgentTraceSpanRecord> {
        let field = field.to_owned();
        let value = value.to_owned();
        self.spans.iter().filter(move |span| {
            span.fields
                .get(&field)
                .is_some_and(|actual| actual.as_str() == value)
        })
    }

    /// Returns the first span with this name and matching field value.
    pub fn find_span_with_field(
        &self,
        name: &AgentSpanName,
        field: &str,
        value: &str,
    ) -> Option<&AgentTraceSpanRecord> {
        self.spans_by_name(name).find(|span| {
            span.fields
                .get(field)
                .is_some_and(|actual| actual.as_str() == value)
        })
    }

    /// Returns a compact summary suitable for scanning many traces.
    pub fn summary(&self) -> AgentExecutionTraceSummary {
        AgentExecutionTraceSummary {
            run_id: self.run_id.clone(),
            graph_id: self.graph_id.clone(),
            status: self.status.clone(),
            event_count: self.events.len(),
            span_count: self.spans.len(),
            diagnostic_count: self.diagnostics.len(),
        }
    }
}
