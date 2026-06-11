//! Controlled harness runtime for scenario execution and evidence capture.

use marlin_agent_kernel::GraphLoopKernel;
use marlin_agent_protocol::{
    AgentEvent, AgentScenario, GraphLoopExecutionRequest, GraphLoopExecutionResult, LoopEvidence,
    RuntimePlanSnapshot,
};
use marlin_agent_runtime::{
    CancellationToken, RuntimeEnvironment, RuntimeEventStream, TokioAgentRuntime,
};
use std::time::Duration;
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
        let span_recorder = TraceRecorder::new();
        let _span_guard = span_recorder.install();
        let _harness_span = tracing::info_span!("harness.execute_graph");
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
        let trace_spans = span_recorder.spans();
        let span_names = trace_spans
            .iter()
            .map(|span| HarnessSpanName::new(span.name))
            .collect();
        HarnessExecutionReport {
            scenario_id: scenario.id.clone(),
            result,
            events,
            evidence: self.evidence.clone(),
            trace_spans,
            span_names,
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

/// Result of running one graph-loop request through the harness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessExecutionReport {
    pub scenario_id: String,
    pub result: GraphLoopExecutionResult,
    pub events: Vec<AgentEvent>,
    pub evidence: Vec<LoopEvidence>,
    pub trace_spans: Vec<TraceSpanRecord>,
    pub span_names: Vec<HarnessSpanName>,
    pub assertion: Option<HarnessAssertionError>,
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
