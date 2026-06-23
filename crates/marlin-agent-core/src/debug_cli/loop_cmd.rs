//! `marlin loop ...` command implementations.

use std::path::{Path, PathBuf};

use crate::{
    GraphId, GraphLoopContinuationAction, GraphLoopContinuationDecision,
    GraphLoopContinuationInput, GraphLoopContinuationPlanner, GraphLoopContinuationReceipt,
    GraphLoopController, GraphLoopEvidencePolicy, GraphLoopExecutionRequest,
    GraphLoopExecutionStatus, GraphLoopGovernancePolicy, GraphLoopGovernedContextNamespace,
    GraphLoopGovernedSessionKind, GraphLoopIterationReport, GraphLoopRunRequest,
    GraphLoopSandboxBackend, GraphLoopStopPolicy, LoopGraph, RunId, RuntimeFuture,
    TokioAgentRuntime, TokioGraphLoopController, runtime::GraphLoopRunObservation,
};

use super::{
    MarlinCliResult,
    args::{
        ArgCursor, LoopContinuationPlannerOption, LoopInspectOptions, LoopReplayOptions,
        LoopRunOptions,
    },
    catalog::DebugExecutorCatalog,
    executor::{debug_kernel, read_debug_executor_catalog},
    graph::parse_graph_input,
    io::{
        block_on, read_input, read_iteration_reports_from_path, read_reports_from_store,
        write_loop_reports, write_loop_reports_to_path,
    },
    loop_usage,
    receipts::{
        LoopGovernanceReceipt, LoopGovernanceSandboxReceipt, LoopGovernanceSessionReceipt,
        LoopGovernanceStateReceipt, LoopGovernanceVerifierDecision, LoopGovernanceVerifierReceipt,
        LoopInspectReceipt, LoopReplayReceipt, LoopRunReceipt,
    },
    state_home::resolve_runtime_state_layout,
};
use crate::{ContextNamespace, ContextVisibility, SessionKind};

pub(super) fn dispatch_loop(cursor: &mut ArgCursor) -> Result<MarlinCliResult, String> {
    let Some(command) = cursor.next() else {
        return Err(format!("missing loop command\n{}", loop_usage()));
    };

    match command.as_str() {
        "run" => {
            let options = LoopRunOptions::parse(cursor)?;
            let request = read_loop_run_request(options.input.as_deref(), options.max_iterations)?;
            let catalog = read_debug_executor_catalog(options.catalog.as_deref())?;
            let requested_run_id = RunId::new(request.initial_request.run_id.clone());
            let output = block_on(run_loop_controller(
                request,
                catalog,
                options.continuation_planner,
            ))?;
            let report_path =
                write_loop_reports_for_options(&options, &requested_run_id, &output.reports)?;
            let receipt = loop_run_receipt(
                requested_run_id,
                output.reports,
                report_path,
                output.runtime_observation,
                output.governance_receipt,
            );
            Ok(MarlinCliResult::success_json(&receipt))
        }
        "replay" => {
            let options = LoopReplayOptions::parse(cursor)?;
            let reports = read_iteration_reports_from_path(&options.trace_or_report)?;
            let receipt = replay_reports(&options.trace_or_report, &reports);
            Ok(MarlinCliResult::success_json(&receipt))
        }
        "inspect" => {
            let options = LoopInspectOptions::parse(cursor)?;
            let (report_path, reports) = read_reports_for_options(&options)?;
            let receipt = inspect_reports(&options.run_id, &report_path, &reports);
            Ok(MarlinCliResult::success_json(&receipt))
        }
        "-h" | "--help" | "help" => Ok(MarlinCliResult::success_text(loop_usage())),
        unknown => Err(format!(
            "unknown loop command `{unknown}`\n{}",
            loop_usage()
        )),
    }
}

fn write_loop_reports_for_options(
    options: &LoopRunOptions,
    run_id: &RunId,
    reports: &[GraphLoopIterationReport],
) -> Result<Option<PathBuf>, String> {
    if options.no_store {
        return Ok(None);
    }
    if let Some(store) = options.store.as_deref() {
        return write_loop_reports(store, reports);
    }

    let layout = resolve_runtime_state_layout(options.home.clone())?;
    let path = layout
        .receipt_path(run_id.as_str())
        .ok_or_else(|| "runtime state home does not include receipts directory".to_owned())?
        .path;
    write_loop_reports_to_path(&path, reports)
}

fn read_reports_for_options(
    options: &LoopInspectOptions,
) -> Result<(PathBuf, Vec<GraphLoopIterationReport>), String> {
    if let Some(store) = options.store.as_deref() {
        return read_reports_from_store(store, &options.run_id);
    }

    let layout = resolve_runtime_state_layout(options.home.clone())?;
    let path = layout
        .receipt_path(options.run_id.as_str())
        .ok_or_else(|| "runtime state home does not include receipts directory".to_owned())?
        .path;
    read_iteration_reports_from_path(&path).map(|reports| (path, reports))
}

fn read_loop_run_request(
    input: Option<&Path>,
    max_iterations: Option<u64>,
) -> Result<GraphLoopRunRequest, String> {
    let raw = read_input(input)?;
    let mut request = serde_json::from_str::<GraphLoopRunRequest>(&raw).or_else(|_| {
        parse_graph_input(&raw).map(|graph| {
            GraphLoopRunRequest::new(GraphLoopExecutionRequest::new("marlin-loop-run", graph))
        })
    })?;
    if let Some(max_iterations) = max_iterations {
        request = request.with_stop_policy(GraphLoopStopPolicy::max_iterations(max_iterations));
    }
    if request.evidence_policy == GraphLoopEvidencePolicy::default() {
        request = request.with_evidence_policy(GraphLoopEvidencePolicy::replayable_runtime());
    }
    Ok(request)
}

struct LoopRunControllerOutput {
    reports: Vec<GraphLoopIterationReport>,
    runtime_observation: Option<GraphLoopRunObservation>,
    governance_receipt: Option<LoopGovernanceReceipt>,
}

async fn run_loop_controller(
    request: GraphLoopRunRequest,
    catalog: DebugExecutorCatalog,
    continuation_planner: LoopContinuationPlannerOption,
) -> Result<LoopRunControllerOutput, String> {
    let (runtime, _events) = TokioAgentRuntime::new(64);
    let requested_run_id = request.initial_request.run_id.clone();
    let requested_max_iterations = request.stop_policy.max_iterations;
    let governance_policy = request.governance_policy.clone();
    let governance_materialization = governance_policy
        .as_ref()
        .map(|policy| materialize_governed_runtime(&runtime, &requested_run_id, policy));
    let execution_runtime = governance_materialization
        .as_ref()
        .map(|materialization| materialization.runtime.clone())
        .unwrap_or_else(|| runtime.clone());
    let continuation_graph = request.initial_request.graph.clone();
    let kernel = debug_kernel(
        &request.initial_request.run_id,
        &request.initial_request.graph,
        catalog,
    )?;
    let mut controller = TokioGraphLoopController::new(kernel);
    match continuation_planner {
        LoopContinuationPlannerOption::Terminal => {}
        LoopContinuationPlannerOption::RepeatGraph => {
            controller = controller.with_continuation_planner(
                DebugRepeatGraphContinuationPlanner::new(continuation_graph),
            );
        }
        LoopContinuationPlannerOption::RetryOnFailure => {
            controller = controller.with_continuation_planner(
                DebugRetryOnFailureContinuationPlanner::new(continuation_graph),
            );
        }
    }
    let reports = controller
        .spawn_loop(request, &execution_runtime)
        .join()
        .await
        .map_err(|error| format!("loop controller task failed to join: {error}"))?;
    let runtime_observation = runtime.graph_loop_runs().read_registry(|registry| {
        registry
            .snapshot(0)
            .runs
            .into_iter()
            .find(|observation| observation.run_id.as_str() == requested_run_id)
    });
    let governance_receipt = governance_policy.and_then(|policy| {
        governance_materialization.map(|materialization| {
            governance_receipt_from_reports(
                requested_run_id.clone(),
                materialization,
                policy,
                requested_max_iterations,
                &reports,
            )
        })
    });
    Ok(LoopRunControllerOutput {
        reports,
        runtime_observation,
        governance_receipt,
    })
}

struct GovernedRuntimeMaterialization {
    runtime: TokioAgentRuntime,
    state: LoopGovernanceStateReceipt,
    sandbox: LoopGovernanceSandboxReceipt,
    session: LoopGovernanceSessionReceipt,
}

fn materialize_governed_runtime(
    runtime: &TokioAgentRuntime,
    run_id: &str,
    policy: &GraphLoopGovernancePolicy,
) -> GovernedRuntimeMaterialization {
    let requested_visibility = context_visibility_from_governance_policy(policy);
    let child_session_id = policy
        .session_policy
        .child_session_id
        .clone()
        .unwrap_or_else(|| format!("govern-loop:{run_id}"));
    let (governed_runtime, isolation_receipt) = runtime.child_runtime_for_session(
        session_kind_from_governance_policy(&policy.session_policy.session_kind),
        child_session_id,
        requested_visibility,
    );
    GovernedRuntimeMaterialization {
        runtime: governed_runtime,
        state: LoopGovernanceStateReceipt {
            read_before_run: policy.state_policy.read_before_run,
            write_receipt_on_pass: policy.state_policy.write_receipt_on_pass,
            state_ref: policy.state_policy.state_ref.clone(),
        },
        sandbox: LoopGovernanceSandboxReceipt {
            backend: sandbox_backend_name(&policy.sandbox_policy.backend).to_owned(),
            profile_ref: policy.sandbox_policy.profile_ref.clone(),
            filesystem_scope: policy.sandbox_policy.filesystem_scope.clone(),
            network_access: policy.sandbox_policy.network_access,
            runtime_owner: "marlin-agent-core".to_owned(),
            materialized_by: "debug_cli.govern_loop".to_owned(),
        },
        session: LoopGovernanceSessionReceipt {
            parent_session_id: isolation_receipt.parent_session_id().as_str().to_owned(),
            child_session_id: isolation_receipt.child_session_id().as_str().to_owned(),
            requested_namespaces: policy
                .session_policy
                .visible_namespaces
                .iter()
                .map(governed_context_namespace_name)
                .map(str::to_owned)
                .collect(),
            granted_namespaces: isolation_receipt
                .granted_visibility()
                .namespaces()
                .map(context_namespace_name)
                .map(str::to_owned)
                .collect(),
            denied_namespaces: isolation_receipt
                .denied_namespaces()
                .iter()
                .map(context_namespace_name)
                .map(str::to_owned)
                .collect(),
            max_history_items: isolation_receipt.granted_visibility().max_history_items(),
        },
    }
}

fn governance_receipt_from_reports(
    run_id: String,
    materialization: GovernedRuntimeMaterialization,
    policy: GraphLoopGovernancePolicy,
    max_iterations: Option<u64>,
    reports: &[GraphLoopIterationReport],
) -> LoopGovernanceReceipt {
    let terminal_status = reports
        .last()
        .map(|report| report.execution_result.status.clone());
    let iteration_count = reports.len() as u64;
    let decision = governance_verifier_decision(
        &policy,
        terminal_status.as_ref(),
        iteration_count,
        max_iterations,
    );
    let retryable = decision == LoopGovernanceVerifierDecision::Retry;
    let human_audit_required = decision == LoopGovernanceVerifierDecision::HumanAudit;
    LoopGovernanceReceipt {
        run_id: RunId::new(run_id),
        state: materialization.state,
        sandbox: materialization.sandbox,
        session: materialization.session,
        verifier: LoopGovernanceVerifierReceipt {
            decision,
            terminal_status,
            retryable,
            human_audit_required,
            diagnostics: governance_verifier_diagnostics(&policy, reports),
        },
    }
}

fn governance_verifier_decision(
    policy: &GraphLoopGovernancePolicy,
    terminal_status: Option<&GraphLoopExecutionStatus>,
    iteration_count: u64,
    max_iterations: Option<u64>,
) -> LoopGovernanceVerifierDecision {
    let Some(terminal_status) = terminal_status else {
        return LoopGovernanceVerifierDecision::HumanAudit;
    };
    if policy
        .verifier_policy
        .pass_statuses
        .contains(terminal_status)
    {
        return LoopGovernanceVerifierDecision::Passed;
    }
    if policy
        .verifier_policy
        .human_audit_statuses
        .contains(terminal_status)
    {
        return LoopGovernanceVerifierDecision::HumanAudit;
    }
    if policy
        .verifier_policy
        .retry_statuses
        .contains(terminal_status)
    {
        let retry_budget_remaining = max_iterations
            .map(|max_iterations| iteration_count < max_iterations)
            .unwrap_or(true);
        return if retry_budget_remaining {
            LoopGovernanceVerifierDecision::Retry
        } else {
            LoopGovernanceVerifierDecision::HumanAudit
        };
    }
    LoopGovernanceVerifierDecision::HumanAudit
}

fn governance_verifier_diagnostics(
    policy: &GraphLoopGovernancePolicy,
    reports: &[GraphLoopIterationReport],
) -> Vec<String> {
    let mut diagnostics = vec![
        format!(
            "sandbox_backend={}",
            sandbox_backend_name(&policy.sandbox_policy.backend)
        ),
        format!("sandbox_profile={}", policy.sandbox_policy.profile_ref),
        format!("governance_iterations={}", reports.len()),
    ];
    if let Some(report) = reports.last() {
        diagnostics.push(format!(
            "terminal_status={:?}",
            report.execution_result.status
        ));
    }
    diagnostics
}

fn context_visibility_from_governance_policy(
    policy: &GraphLoopGovernancePolicy,
) -> ContextVisibility {
    ContextVisibility::from_namespaces(
        policy
            .session_policy
            .visible_namespaces
            .iter()
            .map(context_namespace_from_governance_policy),
    )
    .with_max_history_items(policy.session_policy.max_history_items)
}

fn context_namespace_from_governance_policy(
    namespace: &GraphLoopGovernedContextNamespace,
) -> ContextNamespace {
    match namespace {
        GraphLoopGovernedContextNamespace::System => ContextNamespace::System,
        GraphLoopGovernedContextNamespace::User => ContextNamespace::User,
        GraphLoopGovernedContextNamespace::Workspace => ContextNamespace::Workspace,
        GraphLoopGovernedContextNamespace::Memory => ContextNamespace::Memory,
        GraphLoopGovernedContextNamespace::Tools => ContextNamespace::Tools,
        GraphLoopGovernedContextNamespace::Hooks => ContextNamespace::Hooks,
        GraphLoopGovernedContextNamespace::SubAgents => ContextNamespace::SubAgents,
    }
}

fn session_kind_from_governance_policy(kind: &GraphLoopGovernedSessionKind) -> SessionKind {
    match kind {
        GraphLoopGovernedSessionKind::SubAgent => SessionKind::SubAgent,
        GraphLoopGovernedSessionKind::Tool => SessionKind::Tool,
    }
}

fn sandbox_backend_name(backend: &GraphLoopSandboxBackend) -> &'static str {
    match backend {
        GraphLoopSandboxBackend::Nono => "nono",
    }
}

fn governed_context_namespace_name(namespace: &GraphLoopGovernedContextNamespace) -> &'static str {
    match namespace {
        GraphLoopGovernedContextNamespace::System => "system",
        GraphLoopGovernedContextNamespace::User => "user",
        GraphLoopGovernedContextNamespace::Workspace => "workspace",
        GraphLoopGovernedContextNamespace::Memory => "memory",
        GraphLoopGovernedContextNamespace::Tools => "tools",
        GraphLoopGovernedContextNamespace::Hooks => "hooks",
        GraphLoopGovernedContextNamespace::SubAgents => "sub-agents",
    }
}

fn context_namespace_name(namespace: &ContextNamespace) -> &'static str {
    match namespace {
        ContextNamespace::System => "system",
        ContextNamespace::User => "user",
        ContextNamespace::Workspace => "workspace",
        ContextNamespace::Memory => "memory",
        ContextNamespace::Tools => "tools",
        ContextNamespace::Hooks => "hooks",
        ContextNamespace::SubAgents => "sub-agents",
        ContextNamespace::Secrets => "secrets",
    }
}

#[derive(Clone, Debug)]
struct DebugRepeatGraphContinuationPlanner {
    graph: LoopGraph,
}

impl DebugRepeatGraphContinuationPlanner {
    fn new(graph: LoopGraph) -> Self {
        Self { graph }
    }
}

impl GraphLoopContinuationPlanner for DebugRepeatGraphContinuationPlanner {
    fn decide(
        &self,
        input: GraphLoopContinuationInput,
    ) -> RuntimeFuture<GraphLoopContinuationDecision> {
        let graph = self.graph.clone();
        Box::pin(async move {
            let action = if input.execution_result.status == GraphLoopExecutionStatus::Completed {
                GraphLoopContinuationAction::Rewrite {
                    graph,
                    reason: "debug_cli.repeat_graph_continuation_planner".to_owned(),
                }
            } else {
                GraphLoopContinuationAction::Deny {
                    reason: "debug_cli.repeat_graph_requires_completed_iteration".to_owned(),
                }
            };
            GraphLoopContinuationDecision::new(
                GraphLoopContinuationReceipt::new(input.run_id, input.iteration_id, action)
                    .with_diagnostic("continuation_planner=repeat-graph"),
            )
        })
    }
}

#[derive(Clone, Debug)]
struct DebugRetryOnFailureContinuationPlanner {
    graph: LoopGraph,
}

impl DebugRetryOnFailureContinuationPlanner {
    fn new(graph: LoopGraph) -> Self {
        Self { graph }
    }
}

impl GraphLoopContinuationPlanner for DebugRetryOnFailureContinuationPlanner {
    fn decide(
        &self,
        input: GraphLoopContinuationInput,
    ) -> RuntimeFuture<GraphLoopContinuationDecision> {
        let graph = self.graph.clone();
        Box::pin(async move {
            let status = input.execution_result.status.clone();
            let action = match status {
                GraphLoopExecutionStatus::Completed => GraphLoopContinuationAction::Rewrite {
                    graph,
                    reason: "debug_cli.retry_on_failure_completed_repeat".to_owned(),
                },
                GraphLoopExecutionStatus::Failed => GraphLoopContinuationAction::Rewrite {
                    graph,
                    reason: "debug_cli.retry_on_failure_failed_retry".to_owned(),
                },
                GraphLoopExecutionStatus::Cancelled => GraphLoopContinuationAction::Deny {
                    reason: "debug_cli.retry_on_failure_cancelled_not_retryable".to_owned(),
                },
            };
            GraphLoopContinuationDecision::new(
                GraphLoopContinuationReceipt::new(input.run_id, input.iteration_id, action)
                    .with_diagnostic("continuation_planner=retry-on-failure")
                    .with_diagnostic(format!("execution_status={status:?}")),
            )
        })
    }
}

fn loop_run_receipt(
    requested_run_id: RunId,
    reports: Vec<GraphLoopIterationReport>,
    report_path: Option<PathBuf>,
    runtime_observation: Option<GraphLoopRunObservation>,
    governance_receipt: Option<LoopGovernanceReceipt>,
) -> LoopRunReceipt {
    let terminal = reports.last();
    LoopRunReceipt {
        run_id: RunId::new(
            terminal
                .map(|report| report.execution_result.snapshot.run_id.clone())
                .unwrap_or_else(|| requested_run_id.into_string()),
        ),
        report_path,
        iteration_count: reports.len(),
        terminal_status: terminal.map(|report| report.execution_result.status.clone()),
        runtime_observation,
        governance_receipt,
        replayable: reports_are_replayable(&reports),
        missing_trace_count: missing_trace_count(&reports),
        reports,
    }
}

fn replay_reports(source: &Path, reports: &[GraphLoopIterationReport]) -> LoopReplayReceipt {
    let statuses = reports
        .iter()
        .map(|report| report.execution_result.status.clone())
        .collect::<Vec<_>>();
    let run_ids = reports
        .iter()
        .map(|report| RunId::new(report.execution_result.snapshot.run_id.clone()))
        .collect::<Vec<_>>();
    let graph_ids = reports
        .iter()
        .map(|report| GraphId::new(report.execution_result.snapshot.graph_id.clone()))
        .collect::<Vec<_>>();
    let diagnostics = reports
        .iter()
        .flat_map(|report| report.execution_result.diagnostics.clone())
        .collect::<Vec<_>>();
    LoopReplayReceipt {
        source: source.to_path_buf(),
        replayable: reports_are_replayable(reports),
        iteration_count: reports.len(),
        missing_trace_count: missing_trace_count(reports),
        statuses,
        run_ids,
        graph_ids,
        diagnostics,
    }
}

fn inspect_reports(
    run_id: &str,
    report_path: &Path,
    reports: &[GraphLoopIterationReport],
) -> LoopInspectReceipt {
    let terminal = reports.last();
    LoopInspectReceipt {
        run_id: RunId::new(run_id),
        report_path: report_path.to_path_buf(),
        iteration_count: reports.len(),
        terminal_status: terminal.map(|report| report.execution_result.status.clone()),
        terminal_graph_id: terminal
            .map(|report| GraphId::new(report.execution_result.snapshot.graph_id.clone())),
        replayable: reports_are_replayable(reports),
        missing_trace_count: missing_trace_count(reports),
        diagnostics: reports
            .iter()
            .flat_map(|report| report.execution_result.diagnostics.clone())
            .collect(),
    }
}

fn reports_are_replayable(reports: &[GraphLoopIterationReport]) -> bool {
    reports.iter().all(|report| report.trace.is_some())
}

fn missing_trace_count(reports: &[GraphLoopIterationReport]) -> usize {
    reports
        .iter()
        .filter(|report| report.trace.is_none())
        .count()
}
