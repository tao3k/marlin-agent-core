//! Controlled harness runtime for scenario execution and evidence capture.

use marlin_agent_kernel::GraphLoopKernel;
use marlin_agent_protocol::{
    AgentEvent, AgentEventTopic, AgentExecutionTrace, AgentScenario, AgentSpanName,
    AgentTraceSpanRecord, GraphLoopExecutionRequest, GraphLoopExecutionResult,
    GraphLoopExecutionStatus, LoopEvidence, RuntimePlanSnapshot,
};
use marlin_agent_runtime::{
    CancellationToken, RuntimeEnvironment, RuntimeEventStream, TokioAgentRuntime,
};
use std::time::{Duration, Instant};
use tokio_stream::StreamExt;

use crate::{HarnessAssertionError, TraceRecorder, assert_evidence_kinds};

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
        trace_spans: Vec<AgentTraceSpanRecord>,
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
        run_id = result.snapshot.run_id.as_str(),
        graph_id = result.snapshot.graph_id.as_str(),
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

fn span_names(trace_spans: &[AgentTraceSpanRecord]) -> Vec<AgentSpanName> {
    trace_spans.iter().map(|span| span.name.clone()).collect()
}

/// Result of running one graph-loop request through the harness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessExecutionReport {
    pub scenario_id: String,
    pub result: GraphLoopExecutionResult,
    pub events: Vec<AgentEvent>,
    pub evidence: Vec<LoopEvidence>,
    pub trace_spans: Vec<AgentTraceSpanRecord>,
    pub span_names: Vec<AgentSpanName>,
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
        self.events_by_topic(topic).next().is_some()
    }

    /// Returns events captured with this topic.
    pub fn events_by_topic(&self, topic: &AgentEventTopic) -> impl Iterator<Item = &AgentEvent> {
        let topic = topic.clone();
        self.events
            .iter()
            .filter(move |event| event.topic_id() == topic)
    }

    /// Returns true when the report captured at least one tracing span with this name.
    pub fn has_span(&self, name: &AgentSpanName) -> bool {
        self.spans_by_name(name).next().is_some()
    }

    /// Counts tracing spans captured with this name.
    pub fn count_span(&self, name: &AgentSpanName) -> usize {
        self.spans_by_name(name).count()
    }

    /// Returns tracing spans captured with this name.
    pub fn spans_by_name(
        &self,
        name: &AgentSpanName,
    ) -> impl Iterator<Item = &AgentTraceSpanRecord> {
        let name = name.clone();
        self.trace_spans
            .iter()
            .filter(move |span| span.name == name)
    }

    /// Returns the first tracing span captured with this name.
    pub fn find_span(&self, name: &AgentSpanName) -> Option<&AgentTraceSpanRecord> {
        self.spans_by_name(name).next()
    }

    /// Returns tracing spans that captured a matching field value.
    pub fn spans_with_field(
        &self,
        field: &str,
        value: &str,
    ) -> impl Iterator<Item = &AgentTraceSpanRecord> {
        let field = field.to_owned();
        let value = value.to_owned();
        self.trace_spans.iter().filter(move |span| {
            span.fields
                .get(&field)
                .is_some_and(|actual| actual.as_str() == value)
        })
    }

    /// Returns the first tracing span with this name and matching field value.
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

    /// Builds a compact protocol-owned trace package for this execution.
    pub fn execution_trace(&self) -> AgentExecutionTrace {
        AgentExecutionTrace::new(
            self.result.snapshot.run_id.clone(),
            self.result.snapshot.graph_id.clone(),
            self.result.status.clone(),
        )
        .with_events(self.events.clone())
        .with_spans(self.trace_spans.clone())
        .with_diagnostics(self.result.diagnostics.clone())
    }
}
