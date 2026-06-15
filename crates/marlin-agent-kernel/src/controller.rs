//! Kernel-side graph-loop controller runner.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use marlin_agent_protocol::{
    AgentExecutionTrace, GraphId, GraphLoopEvidencePolicy, GraphLoopExecutionResult,
    GraphLoopExecutionStatus, GraphLoopIterationReport, GraphLoopNextAction, GraphLoopRunRequest,
    LoopGraph, RunId,
};
use marlin_agent_runtime::{RuntimeFuture, RuntimeTask, TokioAgentRuntime};

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
    pub iteration: u64,
    pub execution_result: GraphLoopExecutionResult,
}

/// Typed continuation policy boundary for producing the next controller action.
pub trait GraphLoopContinuationPlanner: Send + Sync + 'static {
    fn next_action(&self, input: GraphLoopContinuationInput) -> RuntimeFuture<GraphLoopNextAction>;
}

/// Default continuation planner that preserves one-shot graph execution behavior.
#[derive(Clone, Debug, Default)]
pub struct TerminalGraphLoopContinuationPlanner;

impl GraphLoopContinuationPlanner for TerminalGraphLoopContinuationPlanner {
    fn next_action(&self, input: GraphLoopContinuationInput) -> RuntimeFuture<GraphLoopNextAction> {
        Box::pin(async move { next_action_for_execution_result(&input.execution_result) })
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

        let started_at = Instant::now();
        let evidence_policy = request.evidence_policy;
        let iteration_budget = request.iteration_budget;
        let stop_policy = request.stop_policy;
        let base_run_id = request.initial_request.run_id.clone();
        let mut captured_events = captured_events;
        let mut execution_request = request.initial_request;
        let mut reports = Vec::new();

        loop {
            let iteration = reports.len() as u64;
            let execution_result = self
                .execute_iteration(execution_request, iteration_budget.clone(), runtime)
                .await;
            let iteration_events = drain_captured_events(captured_events.as_mut());
            let next_action = self
                .next_iteration_action(iteration, &execution_result, &stop_policy, started_at)
                .await;
            let next_graph = continued_graph(&next_action);

            reports.push(iteration_report(
                iteration,
                execution_result,
                &evidence_policy,
                iteration_events,
                next_action,
            ));

            let Some(next_graph) = next_graph else {
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

    async fn next_iteration_action(
        &self,
        iteration: u64,
        execution_result: &GraphLoopExecutionResult,
        stop_policy: &marlin_agent_protocol::GraphLoopStopPolicy,
        started_at: Instant,
    ) -> GraphLoopNextAction {
        if execution_result.status != GraphLoopExecutionStatus::Completed
            && stop_policy.stop_on_failed_execution
        {
            return GraphLoopNextAction::StopFailed;
        }
        if !allows_another_iteration(stop_policy, iteration, started_at) {
            return next_action_for_execution_result(execution_result);
        }

        let planned = self
            .continuation_planner
            .next_action(GraphLoopContinuationInput {
                iteration,
                execution_result: execution_result.clone(),
            })
            .await;
        if stop_policy.human_gate_required
            && matches!(planned, GraphLoopNextAction::ContinueWithGraph(_))
        {
            GraphLoopNextAction::EscalateToHuman {
                reason: "graph_loop.human_gate_required".to_owned(),
            }
        } else {
            planned
        }
    }
}

impl GraphLoopController for TokioGraphLoopController {
    fn spawn_loop(
        &self,
        request: GraphLoopRunRequest,
        runtime: &TokioAgentRuntime,
    ) -> RuntimeTask<Vec<GraphLoopIterationReport>> {
        let controller = self.clone();
        let controller_runtime = runtime.child_runtime();
        let (execution_runtime, captured_events) = if request.evidence_policy.capture_runtime_events
        {
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
    next_action: GraphLoopNextAction,
) -> GraphLoopIterationReport {
    let trace = (evidence_policy.capture_trace || evidence_policy.capture_runtime_events)
        .then(|| trace_from_execution_result(&execution_result, captured_events));
    let mut report_result = execution_result;
    if !evidence_policy.capture_node_receipts {
        report_result.node_receipts.clear();
    }

    let report = GraphLoopIterationReport::new(iteration, report_result, next_action);
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
    stop_policy: &marlin_agent_protocol::GraphLoopStopPolicy,
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
