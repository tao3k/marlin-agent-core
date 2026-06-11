//! Controlled harness runtime for scenario execution and evidence capture.

use marlin_agent_kernel::GraphLoopKernel;
use marlin_agent_protocol::{
    AgentEvent, AgentEventTopic, AgentScenario, GraphLoopExecutionRequest,
    GraphLoopExecutionResult, GraphLoopExecutionStatus, LoopEvidence, RuntimePlanSnapshot,
};
use marlin_agent_runtime::{
    CancellationToken, RuntimeEnvironment, RuntimeEventStream, TokioAgentRuntime,
};
use std::time::{Duration, Instant};
use tokio_stream::StreamExt;

use crate::{HarnessAssertionError, TraceRecorder, TraceSpanRecord, assert_evidence_kinds};

/// Controlled runtime plus typed evidence captured for one harness scenario.
#[derive(Debug)]
pub struct HarnessRuntime {
    runtime: TokioAgentRuntime,
    events: RuntimeEventStream,
    evidence: Vec<LoopEvidence>,
}

impl HarnessRuntime {
    pub fn new(event_buffer: usize) -> Self {
        let (runtime, events) = TokioAgentRuntime::new(event_buffer);
        Self {
            runtime,
            events,
            evidence: Vec::new(),
        }
    }

    /// Create a harness runtime with an explicit runtime environment snapshot.
    pub fn with_environment(event_buffer: usize, environment: RuntimeEnvironment) -> Self {
        let (runtime, events) = TokioAgentRuntime::with_environment(
            event_buffer,
            CancellationToken::new(),
            environment,
        );
        Self {
            runtime,
            events,
            evidence: Vec::new(),
        }
    }

    pub fn runtime(&self) -> TokioAgentRuntime {
        self.runtime.clone()
    }

    /// Borrow the environment visible to harness-owned runtime work.
    pub fn environment(&self) -> &RuntimeEnvironment {
        self.runtime.environment()
    }

    pub fn events(&mut self) -> &mut RuntimeEventStream {
        &mut self.events
    }

    pub fn record_evidence(&mut self, evidence: LoopEvidence) {
        self.evidence.push(evidence);
    }

    pub fn evidence(&self) -> &[LoopEvidence] {
        self.evidence.as_slice()
    }

    pub fn into_parts(self) -> (TokioAgentRuntime, RuntimeEventStream, Vec<LoopEvidence>) {
        (self.runtime, self.events, self.evidence)
    }

    pub fn assert_scenario_evidence(
        &self,
        scenario: &AgentScenario,
    ) -> Result<(), HarnessAssertionError> {
        assert_evidence_kinds(self.evidence(), scenario.expected_evidence.as_slice())
    }

    pub async fn execute_graph<K>(
        &mut self,
        scenario: &AgentScenario,
        kernel: &K,
        request: GraphLoopExecutionRequest,
    ) -> HarnessExecutionReport
    where
        K: GraphLoopKernel,
    {
        let started_at = Instant::now();
        let span_recorder = TraceRecorder::new();
        let _span_guard = span_recorder.install();
        let _harness_span = tracing::info_span!(
            "harness.execution",
            scenario_id = scenario.id.as_str(),
            run_id = request.run_id.as_str(),
            graph_id = request.graph.graph_id.as_str()
        );
        let task = kernel.spawn_execution(request, &self.runtime);
        let result = match task.join().await {
            Ok(result) => result,
            Err(error) => GraphLoopExecutionResult::failed(
                RuntimePlanSnapshot {
                    run_id: scenario.id.clone(),
                    graph_id: "harness".to_owned(),
                    active_node: None,
                },
                vec![format!("harness task join failed: {error}")],
            ),
        };
        let assertion = self.assert_scenario_evidence(scenario).err();
        let events = self.drain_ready_events().await;
        let duration = started_at.elapsed();
        record_result_span(&result, events.len(), duration);
        let trace_spans = span_recorder.spans();

        self.execution_report(scenario, result, events, trace_spans, duration, assertion)
    }

    fn execution_report(
        &self,
        scenario: &AgentScenario,
        result: GraphLoopExecutionResult,
        events: Vec<AgentEvent>,
        trace_spans: Vec<TraceSpanRecord>,
        duration: Duration,
        assertion: Option<HarnessAssertionError>,
    ) -> HarnessExecutionReport {
        HarnessExecutionReport {
            scenario_id: scenario.id.clone(),
            summary: execution_summary(&result, events.len(), trace_spans.len(), duration),
            result,
            events,
            evidence: self.evidence.clone(),
            span_names: span_names(&trace_spans),
            trace_spans,
            assertion,
        }
    }

    async fn drain_ready_events(&mut self) -> Vec<AgentEvent> {
        let mut events = Vec::new();

        while let Ok(Some(event)) = self
            .runtime
            .timeout(Duration::from_millis(1), self.events.next())
            .await
        {
            events.push(event);
        }

        events
    }
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn record_result_span(result: &GraphLoopExecutionResult, event_count: usize, duration: Duration) {
    let status = result.status.clone();
    let diagnostic_count = result.diagnostics.len();
    let _result_span = tracing::info_span!(
        "harness.result",
        status = ?status,
        duration_ms = duration_ms(duration),
        diagnostic_count,
        event_count
    );
}

fn execution_summary(
    result: &GraphLoopExecutionResult,
    event_count: usize,
    span_count: usize,
    duration: Duration,
) -> HarnessExecutionSummary {
    HarnessExecutionSummary {
        status: result.status.clone(),
        duration,
        event_count,
        span_count,
        diagnostic_count: result.diagnostics.len(),
    }
}

fn span_names(trace_spans: &[TraceSpanRecord]) -> Vec<HarnessSpanName> {
    trace_spans
        .iter()
        .map(|span| HarnessSpanName::new(span.name))
        .collect()
}

/// Result of running one graph-loop request through the harness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessExecutionReport {
    pub scenario_id: String,
    pub result: GraphLoopExecutionResult,
    pub events: Vec<AgentEvent>,
    pub evidence: Vec<LoopEvidence>,
    pub trace_spans: Vec<TraceSpanRecord>,
    pub span_names: Vec<HarnessSpanName>,
    pub summary: HarnessExecutionSummary,
    pub assertion: Option<HarnessAssertionError>,
}

/// Compact execution summary for scanning many harness runs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessExecutionSummary {
    /// Final graph-loop execution status.
    pub status: GraphLoopExecutionStatus,
    /// Wall-clock duration observed by the harness.
    pub duration: Duration,
    /// Number of runtime events captured in the report.
    pub event_count: usize,
    /// Number of tracing spans captured in the report.
    pub span_count: usize,
    /// Number of execution diagnostics captured in the result.
    pub diagnostic_count: usize,
}

impl HarnessExecutionReport {
    /// Returns true when the report captured at least one event with this topic.
    pub fn has_event_topic(&self, topic: &AgentEventTopic) -> bool {
        self.events.iter().any(|event| event.topic_id() == *topic)
    }

    /// Returns true when the report captured at least one tracing span with this name.
    pub fn has_span(&self, name: &HarnessSpanName) -> bool {
        self.trace_spans
            .iter()
            .any(|span| span.name == name.as_str())
    }

    /// Counts tracing spans captured with this name.
    pub fn count_span(&self, name: &HarnessSpanName) -> usize {
        self.trace_spans
            .iter()
            .filter(|span| span.name == name.as_str())
            .count()
    }

    /// Returns the first tracing span captured with this name.
    pub fn find_span(&self, name: &HarnessSpanName) -> Option<&TraceSpanRecord> {
        self.trace_spans
            .iter()
            .find(|span| span.name == name.as_str())
    }
}

/// Harness-owned tracing span name captured during one execution report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessSpanName(String);

impl HarnessSpanName {
    /// Creates a harness-owned span-name fact.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the captured span name.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
