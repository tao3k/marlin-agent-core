use marlin_agent_kernel::{
    GraphLoopContinuationInput, GraphLoopContinuationPlanner, GraphLoopController,
    GraphLoopEvidencePolicy, GraphLoopExecutionBudget, GraphLoopExecutionRequest,
    GraphLoopExecutionStatus, GraphLoopNextAction, GraphLoopRunRequest, GraphLoopStopPolicy,
    LoopEdgeSpec, LoopGraph, LoopNodeSpec, TokioGraphLoopController, TokioGraphLoopKernel,
};
use marlin_agent_runtime::{RuntimeFuture, TokioAgentRuntime};

use super::support::CompletingExecutor;

#[tokio::test]
async fn controller_runs_initial_graph_and_reports_terminal_action() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(1));
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.iteration, 0);
    assert_eq!(
        report.execution_result.status,
        GraphLoopExecutionStatus::Completed
    );
    assert_eq!(report.execution_result.visited_nodes, vec!["plan"]);
    assert!(report.execution_result.node_receipts.is_empty());
    assert!(report.trace.is_none());
    assert_eq!(report.next_action, GraphLoopNextAction::StopCompleted);
}

#[tokio::test]
async fn controller_honors_zero_iteration_stop_policy_without_execution() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(0));
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert!(reports.is_empty());
}

#[tokio::test]
async fn controller_applies_iteration_budget_to_kernel_execution() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan"), node("apply")],
            edges: vec![edge("plan", "apply")],
        },
    ))
    .with_iteration_budget(GraphLoopExecutionBudget::max_node_executions(1));
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(
        report.execution_result.status,
        GraphLoopExecutionStatus::Failed
    );
    assert_eq!(report.next_action, GraphLoopNextAction::StopFailed);
    assert_eq!(
        report.execution_result.diagnostics,
        vec!["graph execution budget exceeded: planned node executions 2 > max 1"]
    );
}

#[tokio::test]
async fn controller_keeps_receipts_and_trace_for_replayable_evidence_policy() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_evidence_policy(GraphLoopEvidencePolicy::replayable_runtime());
    let (runtime, mut events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.execution_result.node_receipts.len(), 1);
    let trace = report.trace.as_ref().expect("replayable trace");
    assert_eq!(trace.run_id.as_str(), "run");
    assert_eq!(trace.graph_id.as_str(), "graph");
    assert_eq!(trace.status, GraphLoopExecutionStatus::Completed);
    assert!(
        trace
            .events
            .iter()
            .any(|event| event.topic == "kernel.execution"
                && event.message == "run run started graph graph")
    );
    assert!(trace.diagnostics.is_empty());
    assert_eq!(
        events
            .try_next()
            .expect("parent stream should still receive tee'd event")
            .topic,
        "kernel.execution"
    );
}

#[tokio::test]
async fn controller_executes_continued_graph_from_typed_planner() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph-initial".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(2));
    let (runtime, _events) = TokioAgentRuntime::new(16);

    let reports = controller()
        .with_continuation_planner(ContinueOncePlanner)
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].iteration, 0);
    assert!(matches!(
        reports[0].next_action,
        GraphLoopNextAction::ContinueWithGraph(_)
    ));
    assert_eq!(reports[1].iteration, 1);
    assert_eq!(
        reports[1].execution_result.snapshot.run_id,
        "run:iteration-1"
    );
    assert_eq!(reports[1].execution_result.snapshot.graph_id, "graph-next");
    assert_eq!(reports[1].execution_result.visited_nodes, vec!["review"]);
    assert_eq!(reports[1].next_action, GraphLoopNextAction::StopCompleted);
}

#[tokio::test]
async fn controller_human_gate_blocks_continuation_planner_graph() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph-initial".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(2).require_human_gate());
    let (runtime, _events) = TokioAgentRuntime::new(16);

    let reports = controller()
        .with_continuation_planner(ContinueOncePlanner)
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    assert_eq!(
        reports[0].next_action,
        GraphLoopNextAction::EscalateToHuman {
            reason: "graph_loop.human_gate_required".to_owned(),
        }
    );
}

fn controller() -> TokioGraphLoopController {
    TokioGraphLoopController::new(
        TokioGraphLoopKernel::new("run", "graph").with_executor("echo", CompletingExecutor),
    )
}

fn node(id: &str) -> LoopNodeSpec {
    LoopNodeSpec {
        id: id.to_owned(),
        executor: "echo".to_owned(),
        config: Default::default(),
    }
}

fn edge(from: &str, to: &str) -> LoopEdgeSpec {
    LoopEdgeSpec {
        from: from.to_owned(),
        to: to.to_owned(),
        condition: None,
    }
}

#[derive(Clone, Debug)]
struct ContinueOncePlanner;

impl GraphLoopContinuationPlanner for ContinueOncePlanner {
    fn next_action(&self, input: GraphLoopContinuationInput) -> RuntimeFuture<GraphLoopNextAction> {
        Box::pin(async move {
            if input.iteration == 0 {
                GraphLoopNextAction::ContinueWithGraph(LoopGraph {
                    graph_id: "graph-next".to_owned(),
                    nodes: vec![node("review")],
                    edges: Vec::new(),
                })
            } else {
                GraphLoopNextAction::StopCompleted
            }
        })
    }
}
