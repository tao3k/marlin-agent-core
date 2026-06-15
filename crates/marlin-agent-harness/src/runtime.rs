//! Controlled harness runtime for scenario execution and evidence capture.

use marlin_agent_kernel::{GraphLoopController, GraphLoopKernel};
use marlin_agent_protocol::{
    AgentEvent, AgentEventTopic, AgentExecutionTrace, AgentSpanName, AgentTraceSpanRecord,
    GraphLoopExecutionRequest, GraphLoopExecutionResult, GraphLoopExecutionStatus,
    GraphLoopIterationReport, GraphLoopNextAction, GraphLoopRunRequest, GraphLoopStrategyId,
    GraphPolicyProposalReceipt, GraphPolicyProposalStatus, RuntimePlanSnapshot,
};
use marlin_agent_runtime::{
    CancellationToken, RuntimeEnvironment, RuntimeEventStream, TokioAgentRuntime,
    WorkingCopyCommandStatus, WorkingCopyIsolationReceipt,
};
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec};
use std::time::{Duration, Instant};

use crate::{
    AGENT_HARNESS_GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX, AgentHarnessAssertionError,
    AgentHarnessEvidence, AgentHarnessEvidenceGraph, AgentHarnessEvidenceKind,
    AgentHarnessScenario, TraceRecorder, agent_harness_graph_policy_proposal_visibility_evidence,
    assert_agent_harness_evidence_kinds, release_gate_visibility_evidence,
    release_topology_visibility_evidence,
};

/// Controlled runtime plus typed evidence captured for one agent harness scenario.
#[derive(Debug)]
pub struct AgentHarnessRuntime {
    runtime: TokioAgentRuntime,
    events: RuntimeEventStream,
    evidence: Vec<AgentHarnessEvidence>,
}

impl AgentHarnessRuntime {
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

    pub fn record_evidence(&mut self, evidence: impl Into<AgentHarnessEvidence>) {
        self.evidence.push(evidence.into());
    }

    /// Record evidence describing the runtime environment visible to harness work.
    pub fn record_environment_visibility(&mut self) {
        let evidence = agent_harness_runtime_environment_visibility_evidence(self.environment());
        self.record_evidence(evidence);
    }

    /// Record evidence describing an isolated working copy visible to runtime work.
    pub fn record_working_copy_isolation_visibility(
        &mut self,
        receipt: &WorkingCopyIsolationReceipt,
    ) {
        self.record_evidence(agent_harness_working_copy_isolation_visibility_evidence(
            receipt,
        ));
    }

    /// Record visibility evidence for a Rust-validated graph policy proposal.
    pub fn record_graph_policy_proposal_visibility(
        &mut self,
        receipt: &GraphPolicyProposalReceipt,
    ) {
        let span = AgentTraceSpanRecord::graph_policy_proposal_receipt(receipt);
        let evidence = agent_harness_graph_policy_proposal_visibility_evidence(&span)
            .expect("graph policy proposal receipt span should project visibility evidence");
        self.record_evidence(evidence);
    }

    /// Record visibility evidence declared by one `Gerbil` release gate.
    pub fn record_release_gate_visibility(
        &mut self,
        topology: &ReleaseTopologySpec,
        gate: &ReleaseGateSpec,
    ) {
        for evidence in release_gate_visibility_evidence(topology, gate) {
            self.record_evidence(evidence);
        }
    }

    /// Record visibility evidence declared by all gates in a `Gerbil` release topology.
    pub fn record_release_topology_visibility(&mut self, topology: &ReleaseTopologySpec) {
        for evidence in release_topology_visibility_evidence(topology) {
            self.record_evidence(evidence);
        }
    }

    pub fn evidence(&self) -> &[AgentHarnessEvidence] {
        self.evidence.as_slice()
    }

    pub fn into_parts(
        self,
    ) -> (
        TokioAgentRuntime,
        RuntimeEventStream,
        Vec<AgentHarnessEvidence>,
    ) {
        (self.runtime, self.events, self.evidence)
    }

    pub fn assert_scenario_evidence(
        &self,
        scenario: &AgentHarnessScenario,
    ) -> Result<(), AgentHarnessAssertionError> {
        assert_agent_harness_evidence_kinds(self.evidence(), scenario.expected_evidence.as_slice())
    }

    pub async fn execute_graph<K>(
        &mut self,
        scenario: &AgentHarnessScenario,
        kernel: &K,
        request: GraphLoopExecutionRequest,
    ) -> AgentHarnessExecutionReport
    where
        K: GraphLoopKernel,
    {
        let started_at = Instant::now();
        let span_recorder = TraceRecorder::new();
        let _span_guard = span_recorder.install();
        let _harness_span = tracing::info_span!(
            "harness.execution",
            scenario_id = scenario.id(),
            run_id = request.run_id.as_str(),
            graph_id = request.graph.graph_id.as_str()
        );
        let task = kernel.spawn_execution(request, &self.runtime);
        let result = match task.join().await {
            Ok(result) => result,
            Err(error) => GraphLoopExecutionResult::failed(
                RuntimePlanSnapshot {
                    run_id: scenario.id().to_owned(),
                    graph_id: "harness".to_owned(),
                    active_node: None,
                },
                vec![format!("harness task join failed: {error}")],
            ),
        };
        let assertion = self.assert_scenario_evidence(scenario).err();
        let events = self.drain_ready_events();
        let duration = started_at.elapsed();
        record_result_span(&result, events.len(), duration);
        let trace_spans = span_recorder.spans();

        self.execution_report(scenario, result, events, trace_spans, duration, assertion)
    }

    pub async fn execute_graph_loop<C>(
        &mut self,
        scenario: &AgentHarnessScenario,
        controller: &C,
        request: GraphLoopRunRequest,
    ) -> AgentHarnessGraphLoopExecutionReport
    where
        C: GraphLoopController,
    {
        let started_at = Instant::now();
        let span_recorder = TraceRecorder::new();
        let _span_guard = span_recorder.install();
        let _harness_span = graph_loop_harness_span(scenario, &request);
        let iteration_reports = self.join_graph_loop_controller(controller, request).await;
        let trace_spans = span_recorder.spans();

        self.finish_graph_loop_execution(scenario, started_at, trace_spans, iteration_reports)
    }

    async fn join_graph_loop_controller<C>(
        &self,
        controller: &C,
        request: GraphLoopRunRequest,
    ) -> Vec<GraphLoopIterationReport>
    where
        C: GraphLoopController,
    {
        let (run_id, graph_id) = graph_loop_request_identity(&request);
        let task = controller.spawn_loop(request, &self.runtime);
        match task.join().await {
            Ok(iteration_reports) => iteration_reports,
            Err(error) => vec![graph_loop_join_failure_report(run_id, graph_id, error)],
        }
    }

    fn finish_graph_loop_execution(
        &mut self,
        scenario: &AgentHarnessScenario,
        started_at: Instant,
        trace_spans: Vec<AgentTraceSpanRecord>,
        iteration_reports: Vec<GraphLoopIterationReport>,
    ) -> AgentHarnessGraphLoopExecutionReport {
        let assertion = self.assert_scenario_evidence(scenario).err();
        let events = self.drain_ready_events();
        let duration = started_at.elapsed();
        record_final_iteration_span(&iteration_reports, events.len(), duration);

        self.graph_loop_execution_report(
            scenario,
            iteration_reports,
            events,
            trace_spans,
            duration,
            assertion,
        )
    }

    fn execution_report(
        &self,
        scenario: &AgentHarnessScenario,
        result: GraphLoopExecutionResult,
        events: Vec<AgentEvent>,
        trace_spans: Vec<AgentTraceSpanRecord>,
        duration: Duration,
        assertion: Option<AgentHarnessAssertionError>,
    ) -> AgentHarnessExecutionReport {
        let evidence_graph = AgentHarnessEvidenceGraph::from_agent_harness_evidence(
            format!("execution:{}", scenario.id()),
            &self.evidence,
        )
        .with_graph_execution_result(&result);

        AgentHarnessExecutionReport {
            scenario_id: scenario.id().to_owned(),
            summary: execution_summary(&result, events.len(), trace_spans.len(), duration),
            result,
            events,
            evidence: self.evidence.clone(),
            evidence_graph,
            span_names: span_names(&trace_spans),
            trace_spans,
            assertion,
        }
    }

    fn graph_loop_execution_report(
        &self,
        scenario: &AgentHarnessScenario,
        iteration_reports: Vec<GraphLoopIterationReport>,
        events: Vec<AgentEvent>,
        trace_spans: Vec<AgentTraceSpanRecord>,
        duration: Duration,
        assertion: Option<AgentHarnessAssertionError>,
    ) -> AgentHarnessGraphLoopExecutionReport {
        let evidence_graph = iteration_reports.iter().fold(
            AgentHarnessEvidenceGraph::from_agent_harness_evidence(
                format!("graph-loop:{}", scenario.id()),
                &self.evidence,
            ),
            |graph, report| graph.with_graph_execution_result(&report.execution_result),
        );

        AgentHarnessGraphLoopExecutionReport {
            scenario_id: scenario.id().to_owned(),
            summary: graph_loop_execution_summary(
                &iteration_reports,
                events.len(),
                trace_spans.len(),
                duration,
            ),
            iteration_reports,
            events,
            evidence: self.evidence.clone(),
            evidence_graph,
            span_names: span_names(&trace_spans),
            trace_spans,
            assertion,
        }
    }

    fn drain_ready_events(&mut self) -> Vec<AgentEvent> {
        let mut events = Vec::new();

        while let Some(event) = self.events.try_next() {
            events.push(event);
        }

        events
    }
}

/// Build evidence summarizing the runtime environment visible to harness work.
pub fn agent_harness_runtime_environment_visibility_evidence(
    environment: &RuntimeEnvironment,
) -> AgentHarnessEvidence {
    let detail = format!(
        "home={} cwd={} config_layers={} writable_roots={} network_access={}",
        environment.home.is_some(),
        environment.cwd.is_some(),
        environment.config_layers.len(),
        environment.sandbox.writable_roots.len(),
        environment.sandbox.network_access,
    );

    AgentHarnessEvidence::present(AgentHarnessEvidenceKind::Visibility, "runtime-environment")
        .with_detail(detail)
}

/// Build evidence summarizing a working-copy isolation receipt.
pub fn agent_harness_working_copy_isolation_visibility_evidence(
    receipt: &WorkingCopyIsolationReceipt,
) -> AgentHarnessEvidence {
    let failed_commands = receipt
        .command_receipts
        .iter()
        .filter(|command| command.status == WorkingCopyCommandStatus::Failed)
        .count();
    let working_copy_id = receipt
        .working_copy
        .as_ref()
        .map(|copy| copy.id.as_str())
        .unwrap_or("none");
    let reason = receipt.reason.as_deref().unwrap_or("none");
    let detail = format!(
        "project_id={} provider={:?} operation={:?} status={:?} working_copy={} command_count={} failed_command_count={} reason={}",
        receipt.project_id.as_str(),
        receipt.provider,
        receipt.operation,
        receipt.status,
        working_copy_id,
        receipt.command_receipts.len(),
        failed_commands,
        reason,
    );

    AgentHarnessEvidence::present(
        AgentHarnessEvidenceKind::Visibility,
        format!("working-copy-isolation:{}", receipt.project_id.as_str()),
    )
    .with_detail(detail)
}

fn graph_loop_harness_span(
    scenario: &AgentHarnessScenario,
    request: &GraphLoopRunRequest,
) -> tracing::Span {
    tracing::info_span!(
        "harness.graph_loop",
        scenario_id = scenario.id(),
        run_id = request.initial_request.run_id.as_str(),
        graph_id = request.initial_request.graph.graph_id.as_str()
    )
}

fn graph_loop_request_identity(request: &GraphLoopRunRequest) -> (String, String) {
    (
        request.initial_request.run_id.clone(),
        request.initial_request.graph.graph_id.clone(),
    )
}

fn graph_loop_join_failure_report(
    run_id: String,
    graph_id: String,
    error: impl std::fmt::Display,
) -> GraphLoopIterationReport {
    GraphLoopIterationReport::new(
        0,
        GraphLoopExecutionResult::failed(
            RuntimePlanSnapshot {
                run_id,
                graph_id,
                active_node: None,
            },
            vec![format!("harness graph-loop task join failed: {error}")],
        ),
        GraphLoopNextAction::StopFailed,
    )
}

fn record_final_iteration_span(
    iteration_reports: &[GraphLoopIterationReport],
    event_count: usize,
    duration: Duration,
) {
    if let Some(final_result) = iteration_reports
        .last()
        .map(|report| &report.execution_result)
    {
        record_result_span(final_result, event_count, duration);
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
) -> AgentHarnessExecutionSummary {
    AgentHarnessExecutionSummary {
        status: result.status.clone(),
        duration,
        event_count,
        span_count,
        diagnostic_count: result.diagnostics.len(),
    }
}

fn graph_loop_execution_summary(
    iteration_reports: &[GraphLoopIterationReport],
    event_count: usize,
    span_count: usize,
    duration: Duration,
) -> AgentHarnessGraphLoopExecutionSummary {
    AgentHarnessGraphLoopExecutionSummary {
        final_status: iteration_reports
            .last()
            .map(|report| report.execution_result.status.clone()),
        iteration_count: iteration_reports.len(),
        duration,
        event_count,
        span_count,
        diagnostic_count: iteration_reports
            .iter()
            .map(|report| report.execution_result.diagnostics.len())
            .sum(),
    }
}

fn span_names(trace_spans: &[AgentTraceSpanRecord]) -> Vec<AgentSpanName> {
    trace_spans.iter().map(|span| span.name.clone()).collect()
}

/// Result of running one graph-loop request through the harness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentHarnessExecutionReport {
    pub scenario_id: String,
    pub result: GraphLoopExecutionResult,
    pub events: Vec<AgentEvent>,
    pub evidence: Vec<AgentHarnessEvidence>,
    pub evidence_graph: AgentHarnessEvidenceGraph,
    pub trace_spans: Vec<AgentTraceSpanRecord>,
    pub span_names: Vec<AgentSpanName>,
    pub summary: AgentHarnessExecutionSummary,
    pub assertion: Option<AgentHarnessAssertionError>,
}

/// Result of running one graph-loop controller request through the harness.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentHarnessGraphLoopExecutionReport {
    pub scenario_id: String,
    pub iteration_reports: Vec<GraphLoopIterationReport>,
    pub events: Vec<AgentEvent>,
    pub evidence: Vec<AgentHarnessEvidence>,
    pub evidence_graph: AgentHarnessEvidenceGraph,
    pub trace_spans: Vec<AgentTraceSpanRecord>,
    pub span_names: Vec<AgentSpanName>,
    pub summary: AgentHarnessGraphLoopExecutionSummary,
    pub assertion: Option<AgentHarnessAssertionError>,
}

/// Compact execution summary for scanning many harness runs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentHarnessExecutionSummary {
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

/// Compact execution summary for scanning graph-loop controller runs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentHarnessGraphLoopExecutionSummary {
    /// Terminal status of the last iteration, when the loop ran at least once.
    pub final_status: Option<GraphLoopExecutionStatus>,
    /// Number of controller iteration reports captured by the harness.
    pub iteration_count: usize,
    /// Wall-clock duration observed by the harness.
    pub duration: Duration,
    /// Number of runtime events captured in the report.
    pub event_count: usize,
    /// Number of tracing spans captured in the report.
    pub span_count: usize,
    /// Number of execution diagnostics across all iteration results.
    pub diagnostic_count: usize,
}

impl AgentHarnessExecutionReport {
    /// Returns evidence facts captured with this kind.
    pub fn evidence_by_kind(
        &self,
        kind: AgentHarnessEvidenceKind,
    ) -> impl Iterator<Item = &AgentHarnessEvidence> {
        self.evidence
            .iter()
            .filter(move |evidence| evidence.kind == kind)
    }

    /// Returns visibility evidence derived from graph policy proposal receipts.
    pub fn graph_policy_proposal_visibility_evidence(
        &self,
    ) -> impl Iterator<Item = &AgentHarnessEvidence> {
        self.evidence_by_kind(AgentHarnessEvidenceKind::Visibility)
            .filter(|evidence| {
                evidence
                    .subject
                    .starts_with(AGENT_HARNESS_GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX)
            })
    }

    /// Finds graph policy proposal visibility evidence for a strategy id.
    pub fn find_graph_policy_proposal_visibility_evidence(
        &self,
        strategy_id: &GraphLoopStrategyId,
    ) -> Option<&AgentHarnessEvidence> {
        let expected_subject = format!(
            "{}:{}",
            AGENT_HARNESS_GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX,
            strategy_id.as_str()
        );

        self.graph_policy_proposal_visibility_evidence()
            .find(|evidence| evidence.subject == expected_subject)
    }

    /// Returns true when graph policy proposal visibility evidence records this status.
    pub fn has_graph_policy_proposal_visibility_status(
        &self,
        strategy_id: &GraphLoopStrategyId,
        status: GraphPolicyProposalStatus,
    ) -> bool {
        self.find_graph_policy_proposal_visibility_evidence(strategy_id)
            .and_then(|evidence| evidence.detail.as_deref())
            .is_some_and(|detail| {
                let expected_status = match status {
                    GraphPolicyProposalStatus::Accepted => "status=Accepted",
                    GraphPolicyProposalStatus::Rejected => "status=Rejected",
                };
                detail.contains(expected_status)
            })
    }

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

impl AgentHarnessGraphLoopExecutionReport {
    /// Returns the last controller iteration report.
    pub fn final_iteration(&self) -> Option<&GraphLoopIterationReport> {
        self.iteration_reports.last()
    }

    /// Returns the last graph execution result produced by the controller.
    pub fn final_result(&self) -> Option<&GraphLoopExecutionResult> {
        self.final_iteration()
            .map(|iteration| &iteration.execution_result)
    }

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

    /// Builds protocol-owned traces emitted by controller iteration reports.
    pub fn iteration_traces(&self) -> impl Iterator<Item = &AgentExecutionTrace> {
        self.iteration_reports
            .iter()
            .filter_map(|report| report.trace.as_ref())
    }
}
