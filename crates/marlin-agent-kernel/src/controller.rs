//! Kernel-side graph-loop controller runner.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use marlin_agent_protocol::{
    AgentExecutionTrace, FailureClassificationReceipt, GraphId, GraphLoopContinuationAction,
    GraphLoopContinuationDecision, GraphLoopContinuationReceipt, GraphLoopEventId,
    GraphLoopEvidencePolicy, GraphLoopExecutionResult, GraphLoopExecutionStatus,
    GraphLoopFailureKind, GraphLoopIterationId, GraphLoopIterationReport, GraphLoopNextAction,
    GraphLoopRunRequest, GraphLoopStopPolicy, GraphNodeExecutionStatus, HumanGateReceipt,
    HumanReviewKind, LoopContinuationPolicy, LoopEvidenceCapturePolicy, LoopFailurePolicy,
    LoopGraph, LoopPolicyProfile, RunId,
};
use marlin_agent_runtime::{
    GraphLoopRunProgressUpdate, RuntimeFuture, RuntimeTask, TokioAgentRuntime,
};

use crate::driver::{GraphLoopKernel, TokioGraphLoopKernel};

const CONTROLLER_EVENT_CAPTURE_BUFFER: usize = 64;

/// Controller contract for spawning a bounded graph-loop run.
pub trait GraphLoopController: Send + Sync + 'static {
    fn spawn_loop(
        &self,
        request: GraphLoopRunRequest,
        runtime: &TokioAgentRuntime,
    ) -> RuntimeTask<Vec<GraphLoopIterationReport>>;
}

/// Input passed to a typed graph-loop continuation planner after one iteration.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphLoopContinuationInput {
    pub run_id: RunId,
    pub iteration_id: GraphLoopIterationId,
    pub execution_result: GraphLoopExecutionResult,
}

/// Typed continuation policy boundary for producing the next controller decision.
pub trait GraphLoopContinuationPlanner: Send + Sync + 'static {
    fn decide(
        &self,
        input: GraphLoopContinuationInput,
    ) -> RuntimeFuture<GraphLoopContinuationDecision>;
}

/// Default continuation planner that preserves one-shot graph execution behavior.
#[derive(Clone, Debug, Default)]
pub struct TerminalGraphLoopContinuationPlanner;

impl GraphLoopContinuationPlanner for TerminalGraphLoopContinuationPlanner {
    fn decide(
        &self,
        input: GraphLoopContinuationInput,
    ) -> RuntimeFuture<GraphLoopContinuationDecision> {
        Box::pin(async move {
            GraphLoopContinuationDecision::new(GraphLoopContinuationReceipt::new(
                input.run_id,
                input.iteration_id,
                terminal_continuation_action_for_execution_result(&input.execution_result),
            ))
        })
    }
}

struct NextIterationDecision {
    next_action: GraphLoopNextAction,
    continuation_receipt: GraphLoopContinuationReceipt,
    human_gate_receipt: Option<HumanGateReceipt>,
}

#[derive(Clone, Debug)]
struct EffectiveGraphLoopPolicy {
    profile_id: Option<String>,
    evidence_policy: GraphLoopEvidencePolicy,
    continuation_policy: LoopContinuationPolicy,
    failure_policy: LoopFailurePolicy,
    require_human_gate_for_unverified_root_cause: bool,
}

impl EffectiveGraphLoopPolicy {
    fn from_request(
        evidence_policy: GraphLoopEvidencePolicy,
        policy_profile: Option<&LoopPolicyProfile>,
    ) -> Self {
        let Some(policy_profile) = policy_profile else {
            return Self {
                profile_id: None,
                evidence_policy,
                continuation_policy: LoopContinuationPolicy::default(),
                failure_policy: LoopFailurePolicy::default(),
                require_human_gate_for_unverified_root_cause: false,
            };
        };

        Self {
            profile_id: Some(policy_profile.profile_id.as_str().to_owned()),
            evidence_policy: evidence_policy_from_profile(&policy_profile.evidence_policy),
            continuation_policy: policy_profile.continuation_policy.clone(),
            failure_policy: policy_profile.failure_policy.clone(),
            require_human_gate_for_unverified_root_cause: policy_profile
                .human_gate_policy
                .require_for_unverified_root_cause,
        }
    }
}

/// Tokio-backed controller for the kernel graph-loop substrate.
#[derive(Clone)]
pub struct TokioGraphLoopController {
    kernel: TokioGraphLoopKernel,
    continuation_planner: Arc<dyn GraphLoopContinuationPlanner>,
}

impl TokioGraphLoopController {
    pub fn new(kernel: TokioGraphLoopKernel) -> Self {
        Self {
            kernel,
            continuation_planner: Arc::new(TerminalGraphLoopContinuationPlanner),
        }
    }

    pub fn kernel(&self) -> &TokioGraphLoopKernel {
        &self.kernel
    }

    pub fn with_continuation_planner<P>(mut self, continuation_planner: P) -> Self
    where
        P: GraphLoopContinuationPlanner,
    {
        self.continuation_planner = Arc::new(continuation_planner);
        self
    }

    async fn run_loop(
        &self,
        request: GraphLoopRunRequest,
        runtime: &TokioAgentRuntime,
        captured_events: Option<marlin_agent_runtime::RuntimeEventStream>,
    ) -> Vec<GraphLoopIterationReport> {
        if request.stop_policy.max_iterations == Some(0) {
            return Vec::new();
        }

        let GraphLoopRunRequest {
            initial_request,
            policy_profile,
            governance_policy: _,
            stop_policy,
            iteration_budget,
            evidence_policy,
        } = request;
        let started_at = Instant::now();
        let runtime_policy =
            EffectiveGraphLoopPolicy::from_request(evidence_policy, policy_profile.as_ref());
        let base_run_id = initial_request.run_id.clone();
        let initial_graph_id = initial_request.graph.graph_id.clone();
        let mut captured_events = captured_events;
        let mut execution_request = initial_request;
        let mut reports = Vec::new();
        record_loop_started(runtime, &base_run_id, &initial_graph_id);

        loop {
            let iteration = reports.len() as u64;
            let execution_result = self
                .execute_iteration(execution_request, iteration_budget.clone(), runtime)
                .await;
            let iteration_events = drain_captured_events(captured_events.as_mut());
            let decision = self
                .next_iteration_decision(
                    &base_run_id,
                    iteration,
                    &execution_result,
                    &stop_policy,
                    &runtime_policy,
                    started_at,
                )
                .await;
            let failure_classification_receipt = failure_classification_receipt(
                &base_run_id,
                iteration,
                &execution_result,
                &decision.next_action,
                &runtime_policy.failure_policy,
            );
            let next_graph = continued_graph(&decision.next_action);
            let observed_at_ms = elapsed_loop_ms(started_at);
            record_loop_progress(runtime, &base_run_id, iteration, observed_at_ms);

            reports.push(iteration_report(
                iteration,
                execution_result,
                &runtime_policy.evidence_policy,
                iteration_events,
                decision,
                failure_classification_receipt,
            ));

            let Some(next_graph) = next_graph else {
                let terminal_report = reports
                    .last()
                    .expect("terminal report should exist before loop stop");
                record_loop_completed(
                    runtime,
                    &base_run_id,
                    terminal_report.iteration,
                    &terminal_report.execution_result,
                    elapsed_loop_ms(started_at),
                );
                break;
            };
            execution_request =
                continuation_execution_request(&base_run_id, iteration + 1, next_graph);
        }

        reports
    }

    async fn execute_iteration(
        &self,
        mut execution_request: marlin_agent_protocol::GraphLoopExecutionRequest,
        iteration_budget: Option<marlin_agent_protocol::GraphLoopExecutionBudget>,
        runtime: &TokioAgentRuntime,
    ) -> GraphLoopExecutionResult {
        if let Some(iteration_budget) = iteration_budget {
            execution_request = execution_request.with_budget(iteration_budget);
        }

        match self
            .kernel
            .spawn_execution(execution_request, runtime)
            .join()
            .await
        {
            Ok(execution_result) => execution_result,
            Err(error) => GraphLoopExecutionResult::failed(
                self.kernel.snapshot(),
                vec![format!(
                    "graph_loop_controller.execution_task_join_failed:{error}"
                )],
            ),
        }
    }

    async fn next_iteration_decision(
        &self,
        base_run_id: &str,
        iteration: u64,
        execution_result: &GraphLoopExecutionResult,
        stop_policy: &GraphLoopStopPolicy,
        runtime_policy: &EffectiveGraphLoopPolicy,
        started_at: Instant,
    ) -> NextIterationDecision {
        if execution_result.status != GraphLoopExecutionStatus::Completed
            && stop_policy.stop_on_failed_execution
        {
            let terminal_decision =
                terminal_continuation_decision(base_run_id, iteration, execution_result);
            return NextIterationDecision {
                next_action: GraphLoopNextAction::StopFailed,
                continuation_receipt: terminal_decision.receipt,
                human_gate_receipt: None,
            };
        }
        if !allows_another_iteration(stop_policy, iteration, started_at) {
            let terminal_decision =
                terminal_continuation_decision(base_run_id, iteration, execution_result);
            return NextIterationDecision {
                next_action: next_action_for_execution_result(execution_result),
                continuation_receipt: terminal_decision.receipt,
                human_gate_receipt: None,
            };
        }

        let planner_decision = self
            .continuation_planner
            .decide(GraphLoopContinuationInput {
                run_id: RunId::new(base_run_id),
                iteration_id: GraphLoopIterationId::new(iteration),
                execution_result: execution_result.clone(),
            })
            .await;
        let planner_decision =
            enforce_continuation_policy(planner_decision, runtime_policy, base_run_id, iteration);
        let planned = next_action_for_continuation_decision(&planner_decision, execution_result);
        if requires_unverified_root_cause_gate(runtime_policy, execution_result, &planned) {
            let reason = "graph_loop.policy_profile.unverified_root_cause_requires_human_gate";
            return NextIterationDecision {
                next_action: GraphLoopNextAction::EscalateToHuman {
                    reason: reason.to_owned(),
                },
                continuation_receipt: policy_profile_defer_receipt(
                    base_run_id,
                    iteration,
                    reason,
                    runtime_policy.profile_id.as_deref(),
                )
                .with_diagnostic("proposed_next_action=continue_with_graph"),
                human_gate_receipt: Some(
                    HumanGateReceipt::new(
                        format!("human-gate:{base_run_id}:{iteration}"),
                        base_run_id,
                        iteration,
                        reason,
                    )
                    .with_required_review(HumanReviewKind::General)
                    .with_proposed_next_action(planned),
                ),
            };
        }
        if stop_policy.human_gate_required
            && matches!(planned, GraphLoopNextAction::ContinueWithGraph(_))
        {
            let reason = "graph_loop.human_gate_required";
            NextIterationDecision {
                next_action: GraphLoopNextAction::EscalateToHuman {
                    reason: reason.to_owned(),
                },
                continuation_receipt: GraphLoopContinuationReceipt::new(
                    base_run_id,
                    iteration,
                    GraphLoopContinuationAction::Defer {
                        reason: reason.to_owned(),
                    },
                )
                .with_diagnostic("proposed_next_action=continue_with_graph"),
                human_gate_receipt: Some(
                    HumanGateReceipt::new(
                        format!("human-gate:{base_run_id}:{iteration}"),
                        base_run_id,
                        iteration,
                        reason,
                    )
                    .with_required_review(HumanReviewKind::General)
                    .with_proposed_next_action(planned),
                ),
            }
        } else {
            NextIterationDecision {
                next_action: planned,
                continuation_receipt: planner_decision.receipt,
                human_gate_receipt: None,
            }
        }
    }
}

fn record_loop_started(runtime: &TokioAgentRuntime, run_id: &str, graph_id: &str) {
    runtime.graph_loop_runs().with_registry(|registry| {
        let _ = registry.start_run(RunId::new(run_id), GraphId::new(graph_id), 0);
    });
}

fn record_loop_progress(
    runtime: &TokioAgentRuntime,
    run_id: &str,
    iteration: u64,
    observed_at_ms: u64,
) {
    runtime.graph_loop_runs().with_registry(|registry| {
        let _ = registry.record_progress(GraphLoopRunProgressUpdate::new(
            RunId::new(run_id),
            GraphLoopIterationId::new(iteration),
            observed_at_ms,
            loop_event_id(iteration, "progress"),
        ));
    });
}

fn record_loop_completed(
    runtime: &TokioAgentRuntime,
    run_id: &str,
    iteration: u64,
    execution_result: &GraphLoopExecutionResult,
    observed_at_ms: u64,
) {
    runtime.graph_loop_runs().with_registry(|registry| {
        let _ = registry.complete_run(
            &RunId::new(run_id),
            execution_result.status.clone(),
            observed_at_ms,
            loop_event_id(iteration, "terminal"),
        );
    });
}

fn loop_event_id(iteration: u64, kind: &str) -> GraphLoopEventId {
    GraphLoopEventId::new(format!("loop.iteration.{iteration}.{kind}"))
}

fn elapsed_loop_ms(started_at: Instant) -> u64 {
    let elapsed_ms = started_at.elapsed().as_millis();
    elapsed_ms.min(u128::from(u64::MAX)) as u64
}

impl GraphLoopController for TokioGraphLoopController {
    fn spawn_loop(
        &self,
        request: GraphLoopRunRequest,
        runtime: &TokioAgentRuntime,
    ) -> RuntimeTask<Vec<GraphLoopIterationReport>> {
        let controller = self.clone();
        let controller_runtime = runtime.child_runtime();
        let capture_runtime_events = request
            .policy_profile
            .as_ref()
            .map(|profile| profile.evidence_policy.capture_events)
            .unwrap_or(request.evidence_policy.capture_runtime_events);
        let (execution_runtime, captured_events) = if capture_runtime_events {
            let (runtime, events) = controller_runtime
                .child_runtime_with_event_capture(CONTROLLER_EVENT_CAPTURE_BUFFER);
            (runtime, Some(events))
        } else {
            (controller_runtime.child_runtime(), None)
        };
        controller_runtime.spawn(async move {
            controller
                .run_loop(request, &execution_runtime, captured_events)
                .await
        })
    }
}

fn evidence_policy_from_profile(
    evidence_policy: &LoopEvidenceCapturePolicy,
) -> GraphLoopEvidencePolicy {
    GraphLoopEvidencePolicy {
        capture_runtime_events: evidence_policy.capture_events,
        capture_node_receipts: evidence_policy.capture_node_receipts,
        capture_trace: evidence_policy.capture_trace,
        replayable: evidence_policy.replayable,
    }
}

fn next_action_for_execution_result(
    execution_result: &GraphLoopExecutionResult,
) -> GraphLoopNextAction {
    match &execution_result.status {
        GraphLoopExecutionStatus::Completed => GraphLoopNextAction::StopCompleted,
        GraphLoopExecutionStatus::Cancelled | GraphLoopExecutionStatus::Failed => {
            GraphLoopNextAction::StopFailed
        }
    }
}

fn iteration_report(
    iteration: u64,
    execution_result: GraphLoopExecutionResult,
    evidence_policy: &GraphLoopEvidencePolicy,
    captured_events: Vec<marlin_agent_runtime::RuntimeEvent>,
    decision: NextIterationDecision,
    failure_classification_receipt: Option<FailureClassificationReceipt>,
) -> GraphLoopIterationReport {
    let trace = (evidence_policy.capture_trace || evidence_policy.capture_runtime_events)
        .then(|| trace_from_execution_result(&execution_result, captured_events));
    let mut report_result = execution_result;
    if !evidence_policy.capture_node_receipts {
        report_result.node_receipts.clear();
    }

    let mut report = GraphLoopIterationReport::new(iteration, report_result, decision.next_action)
        .with_continuation_receipt(decision.continuation_receipt);
    if let Some(receipt) = decision.human_gate_receipt {
        report = report.with_human_gate_receipt(receipt);
    }
    if let Some(receipt) = failure_classification_receipt {
        report = report.with_failure_classification_receipt(receipt);
    }
    match trace {
        Some(trace) => report.with_trace(trace),
        None => report,
    }
}

fn drain_captured_events(
    captured_events: Option<&mut marlin_agent_runtime::RuntimeEventStream>,
) -> Vec<marlin_agent_runtime::RuntimeEvent> {
    let Some(captured_events) = captured_events else {
        return Vec::new();
    };

    let mut events = Vec::new();
    while let Some(event) = captured_events.try_next() {
        events.push(event);
    }
    events
}

fn allows_another_iteration(
    stop_policy: &GraphLoopStopPolicy,
    iteration: u64,
    started_at: Instant,
) -> bool {
    let has_iteration_budget = stop_policy
        .max_iterations
        .is_none_or(|max_iterations| iteration + 1 < max_iterations);
    let has_duration_budget = stop_policy.max_duration_ms.is_none_or(|max_duration_ms| {
        started_at.elapsed() < Duration::from_millis(max_duration_ms)
    });
    has_iteration_budget && has_duration_budget
}

fn terminal_continuation_decision(
    base_run_id: &str,
    iteration: u64,
    execution_result: &GraphLoopExecutionResult,
) -> GraphLoopContinuationDecision {
    GraphLoopContinuationDecision::new(GraphLoopContinuationReceipt::new(
        base_run_id,
        iteration,
        terminal_continuation_action_for_execution_result(execution_result),
    ))
}

fn terminal_continuation_action_for_execution_result(
    execution_result: &GraphLoopExecutionResult,
) -> GraphLoopContinuationAction {
    match execution_result.status {
        GraphLoopExecutionStatus::Completed => GraphLoopContinuationAction::Accept,
        GraphLoopExecutionStatus::Cancelled => GraphLoopContinuationAction::Deny {
            reason: "graph_loop.execution_cancelled".to_owned(),
        },
        GraphLoopExecutionStatus::Failed => GraphLoopContinuationAction::Deny {
            reason: "graph_loop.execution_failed".to_owned(),
        },
    }
}

fn next_action_for_continuation_decision(
    decision: &GraphLoopContinuationDecision,
    execution_result: &GraphLoopExecutionResult,
) -> GraphLoopNextAction {
    match &decision.receipt.action {
        GraphLoopContinuationAction::Accept => next_action_for_execution_result(execution_result),
        GraphLoopContinuationAction::Deny { .. } => GraphLoopNextAction::StopFailed,
        GraphLoopContinuationAction::Defer { reason } => GraphLoopNextAction::EscalateToHuman {
            reason: reason.clone(),
        },
        GraphLoopContinuationAction::Rewrite { graph, .. } => {
            GraphLoopNextAction::ContinueWithGraph(graph.clone())
        }
    }
}

fn enforce_continuation_policy(
    decision: GraphLoopContinuationDecision,
    runtime_policy: &EffectiveGraphLoopPolicy,
    base_run_id: &str,
    iteration: u64,
) -> GraphLoopContinuationDecision {
    if runtime_policy.profile_id.is_none()
        || continuation_action_allowed(
            &decision.receipt.action,
            &runtime_policy.continuation_policy,
        )
    {
        return decision;
    }

    let disabled_action = continuation_action_kind(&decision.receipt.action);
    let reason = "graph_loop.policy_profile.continuation_action_disabled";
    let action = fallback_disabled_continuation_action(&runtime_policy.continuation_policy, reason);
    let mut receipt = GraphLoopContinuationReceipt::new(base_run_id, iteration, action)
        .with_diagnostic(format!("disabled_continuation_action={disabled_action}"));
    if let Some(profile_id) = runtime_policy.profile_id.as_deref() {
        receipt = receipt.with_diagnostic(format!("policy_profile={profile_id}"));
    }

    GraphLoopContinuationDecision::new(receipt)
}

fn continuation_action_allowed(
    action: &GraphLoopContinuationAction,
    continuation_policy: &LoopContinuationPolicy,
) -> bool {
    match action {
        GraphLoopContinuationAction::Accept => continuation_policy.allow_accept.as_bool(),
        GraphLoopContinuationAction::Deny { .. } => continuation_policy.allow_deny.as_bool(),
        GraphLoopContinuationAction::Defer { .. } => continuation_policy.allow_defer.as_bool(),
        GraphLoopContinuationAction::Rewrite { .. } => continuation_policy.allow_rewrite.as_bool(),
    }
}

fn continuation_action_kind(action: &GraphLoopContinuationAction) -> &'static str {
    match action {
        GraphLoopContinuationAction::Accept => "accept",
        GraphLoopContinuationAction::Deny { .. } => "deny",
        GraphLoopContinuationAction::Defer { .. } => "defer",
        GraphLoopContinuationAction::Rewrite { .. } => "rewrite",
    }
}

fn fallback_disabled_continuation_action(
    continuation_policy: &LoopContinuationPolicy,
    reason: &str,
) -> GraphLoopContinuationAction {
    if continuation_policy.allow_defer.as_bool() {
        GraphLoopContinuationAction::Defer {
            reason: reason.to_owned(),
        }
    } else if continuation_policy.allow_deny.as_bool() {
        GraphLoopContinuationAction::Deny {
            reason: reason.to_owned(),
        }
    } else {
        GraphLoopContinuationAction::Defer {
            reason: "graph_loop.policy_profile.no_allowed_terminal_action".to_owned(),
        }
    }
}

fn requires_unverified_root_cause_gate(
    runtime_policy: &EffectiveGraphLoopPolicy,
    execution_result: &GraphLoopExecutionResult,
    planned: &GraphLoopNextAction,
) -> bool {
    runtime_policy.profile_id.is_some()
        && runtime_policy.require_human_gate_for_unverified_root_cause
        && execution_result.status != GraphLoopExecutionStatus::Completed
        && matches!(planned, GraphLoopNextAction::ContinueWithGraph(_))
}

fn policy_profile_defer_receipt(
    base_run_id: &str,
    iteration: u64,
    reason: &str,
    profile_id: Option<&str>,
) -> GraphLoopContinuationReceipt {
    let mut receipt = GraphLoopContinuationReceipt::new(
        base_run_id,
        iteration,
        GraphLoopContinuationAction::Defer {
            reason: reason.to_owned(),
        },
    );
    if let Some(profile_id) = profile_id {
        receipt = receipt.with_diagnostic(format!("policy_profile={profile_id}"));
    }
    receipt
}

fn failure_classification_receipt(
    base_run_id: &str,
    iteration: u64,
    execution_result: &GraphLoopExecutionResult,
    next_action: &GraphLoopNextAction,
    failure_policy: &LoopFailurePolicy,
) -> Option<FailureClassificationReceipt> {
    if !failure_policy.classify_failure {
        return None;
    }
    if execution_result.status == GraphLoopExecutionStatus::Completed {
        return None;
    }

    let mut diagnostics = execution_result.diagnostics.clone();
    diagnostics.extend(
        execution_result
            .node_receipts
            .iter()
            .flat_map(|receipt| receipt.diagnostics.iter().cloned()),
    );
    let failure_kind = classify_failure(&diagnostics);
    let mut receipt = FailureClassificationReceipt::new(
        format!("failure:{base_run_id}:{iteration}"),
        base_run_id,
        iteration,
        failure_kind.clone(),
    )
    .with_retryable(matches!(
        failure_kind,
        GraphLoopFailureKind::TransientFailure
            | GraphLoopFailureKind::ToolUsageFailure
            | GraphLoopFailureKind::ContextFailure
    ))
    .with_requires_human(matches!(failure_kind, GraphLoopFailureKind::PolicyFailure));

    for node_receipt in execution_result
        .node_receipts
        .iter()
        .filter(|node_receipt| node_receipt.status == GraphNodeExecutionStatus::Failed)
    {
        receipt = receipt.with_source_node(node_receipt.node_id.clone());
    }
    for diagnostic in diagnostics {
        receipt = receipt.with_diagnostic(diagnostic);
    }
    if let GraphLoopNextAction::ContinueWithGraph(graph) = next_action {
        receipt = receipt.with_suggested_recovery_graph(graph.clone());
    }

    Some(receipt)
}

fn classify_failure(diagnostics: &[String]) -> GraphLoopFailureKind {
    let normalized = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if normalized
        .iter()
        .any(|diagnostic| diagnostic.contains("sandbox") || diagnostic.contains("denied"))
    {
        GraphLoopFailureKind::PolicyFailure
    } else if normalized.iter().any(|diagnostic| {
        diagnostic.contains("process-command")
            || diagnostic.contains("tool")
            || diagnostic.contains("executor")
    }) {
        GraphLoopFailureKind::ToolUsageFailure
    } else if normalized
        .iter()
        .any(|diagnostic| diagnostic.contains("verify") || diagnostic.contains("assert"))
    {
        GraphLoopFailureKind::VerificationFailure
    } else if normalized
        .iter()
        .any(|diagnostic| diagnostic.contains("content") || diagnostic.contains("context"))
    {
        GraphLoopFailureKind::ContextFailure
    } else if normalized
        .iter()
        .any(|diagnostic| diagnostic.contains("join_failed") || diagnostic.contains("timeout"))
    {
        GraphLoopFailureKind::TransientFailure
    } else {
        GraphLoopFailureKind::Unknown
    }
}

fn continued_graph(next_action: &GraphLoopNextAction) -> Option<LoopGraph> {
    match next_action {
        GraphLoopNextAction::ContinueWithGraph(graph) => Some(graph.clone()),
        GraphLoopNextAction::StopCompleted
        | GraphLoopNextAction::StopFailed
        | GraphLoopNextAction::EscalateToHuman { .. } => None,
    }
}

fn continuation_execution_request(
    base_run_id: &str,
    iteration: u64,
    graph: LoopGraph,
) -> marlin_agent_protocol::GraphLoopExecutionRequest {
    marlin_agent_protocol::GraphLoopExecutionRequest::new(
        format!("{base_run_id}:iteration-{iteration}"),
        graph,
    )
}

fn trace_from_execution_result(
    execution_result: &GraphLoopExecutionResult,
    captured_events: Vec<marlin_agent_runtime::RuntimeEvent>,
) -> AgentExecutionTrace {
    AgentExecutionTrace::new(
        RunId::new(execution_result.snapshot.run_id.clone()),
        GraphId::new(execution_result.snapshot.graph_id.clone()),
        execution_result.status.clone(),
    )
    .with_events(captured_events)
    .with_diagnostics(execution_result.diagnostics.clone())
}
