//! Serializable tracing contracts emitted by agent-owned execution.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::event::AgentEvent;
use crate::graph::{GraphId, GraphLoopExecutionStatus, RunId};

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
