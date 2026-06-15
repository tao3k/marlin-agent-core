use marlin_agent_kernel::{
    GraphLoopController, GraphLoopExecutionBudget, GraphLoopExecutionRequest,
    GraphLoopExecutionStatus, GraphLoopNextAction, GraphLoopRunRequest, GraphLoopStopPolicy,
    LoopGraph,
};
use marlin_agent_runtime::TokioAgentRuntime;

use super::{ContinueOncePlanner, RepairFailurePlanner, controller, edge, node};

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

#[tokio::test]
async fn controller_allows_continuation_planner_to_repair_failed_execution_by_default() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph-initial".to_owned(),
            nodes: vec![node("plan"), node("apply")],
            edges: vec![edge("plan", "apply")],
        },
    ))
    .with_iteration_budget(GraphLoopExecutionBudget::max_node_executions(1))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(2));
    let (runtime, _events) = TokioAgentRuntime::new(16);

    let reports = controller()
        .with_continuation_planner(RepairFailurePlanner)
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 2);
    assert_eq!(
        reports[0].execution_result.status,
        GraphLoopExecutionStatus::Failed
    );
    assert!(matches!(
        reports[0].next_action,
        GraphLoopNextAction::ContinueWithGraph(_)
    ));
    assert_eq!(
        reports[1].execution_result.snapshot.graph_id,
        "graph-repair"
    );
    assert_eq!(
        reports[1].execution_result.status,
        GraphLoopExecutionStatus::Completed
    );
    assert_eq!(reports[1].execution_result.visited_nodes, vec!["repair"]);
}

#[tokio::test]
async fn controller_stop_on_failed_execution_bypasses_repair_planner() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph-initial".to_owned(),
            nodes: vec![node("plan"), node("apply")],
            edges: vec![edge("plan", "apply")],
        },
    ))
    .with_iteration_budget(GraphLoopExecutionBudget::max_node_executions(1))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(2).stop_on_failed_execution());
    let (runtime, _events) = TokioAgentRuntime::new(16);

    let reports = controller()
        .with_continuation_planner(RepairFailurePlanner)
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    assert_eq!(
        reports[0].execution_result.status,
        GraphLoopExecutionStatus::Failed
    );
    assert_eq!(reports[0].next_action, GraphLoopNextAction::StopFailed);
}
