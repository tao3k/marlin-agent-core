use marlin_agent_kernel::{
    GraphLoopContinuationAction, GraphLoopController, GraphLoopExecutionBudget,
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphLoopFailureKind, GraphLoopNextAction,
    GraphLoopRunRequest, GraphLoopStopPolicy, HumanReviewKind, LoopContinuationCapability,
    LoopContinuationPolicy, LoopGraph, LoopPolicyProfile,
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
    let continuation_receipt = reports[0]
        .continuation_receipt
        .as_ref()
        .expect("continuation receipt");
    assert_eq!(continuation_receipt.run_id.as_str(), "run");
    assert_eq!(continuation_receipt.iteration_id.get(), 0);
    assert!(matches!(
        continuation_receipt.action,
        GraphLoopContinuationAction::Rewrite { .. }
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
    let continuation_receipt = reports[0]
        .continuation_receipt
        .as_ref()
        .expect("deferred continuation receipt");
    assert!(matches!(
        continuation_receipt.action,
        GraphLoopContinuationAction::Defer { .. }
    ));
    let human_gate_receipt = reports[0]
        .human_gate_receipt
        .as_ref()
        .expect("human gate receipt");
    assert_eq!(human_gate_receipt.gate_id.as_str(), "human-gate:run:0");
    assert_eq!(human_gate_receipt.run_id.as_str(), "run");
    assert_eq!(human_gate_receipt.iteration_id.get(), 0);
    assert_eq!(human_gate_receipt.required_review, HumanReviewKind::General);
    assert!(matches!(
        human_gate_receipt.proposed_next_action,
        GraphLoopNextAction::ContinueWithGraph(_)
    ));
}

#[tokio::test]
async fn controller_policy_profile_blocks_disabled_rewrite_continuation() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph-initial".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_policy_profile(
        LoopPolicyProfile::new("profile.locked").with_continuation_policy(LoopContinuationPolicy {
            allow_rewrite: LoopContinuationCapability::disabled(),
            ..LoopContinuationPolicy::default()
        }),
    )
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(2));
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
            reason: "graph_loop.policy_profile.continuation_action_disabled".to_owned(),
        }
    );
    let continuation_receipt = reports[0]
        .continuation_receipt
        .as_ref()
        .expect("policy-profile defer receipt");
    assert!(matches!(
        &continuation_receipt.action,
        GraphLoopContinuationAction::Defer { reason }
            if reason == "graph_loop.policy_profile.continuation_action_disabled"
    ));
    assert!(
        continuation_receipt
            .diagnostics
            .contains(&"disabled_continuation_action=rewrite".to_owned())
    );
    assert!(
        continuation_receipt
            .diagnostics
            .contains(&"policy_profile=profile.locked".to_owned())
    );
    assert!(reports[0].human_gate_receipt.is_none());
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
    let classification = reports[0]
        .failure_classification_receipt
        .as_ref()
        .expect("failure classification receipt");
    assert_eq!(classification.run_id.as_str(), "run");
    assert_eq!(classification.iteration_id.get(), 0);
    assert_eq!(classification.failure_kind, GraphLoopFailureKind::Unknown);
    assert!(classification.suggested_recovery_graph.is_some());
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
async fn controller_policy_profile_gates_failed_repair_without_root_cause_receipt() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph-initial".to_owned(),
            nodes: vec![node("plan"), node("apply")],
            edges: vec![edge("plan", "apply")],
        },
    ))
    .with_policy_profile(LoopPolicyProfile::new("profile.causal"))
    .with_iteration_budget(GraphLoopExecutionBudget::max_node_executions(1))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(2));
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
    assert_eq!(
        reports[0].next_action,
        GraphLoopNextAction::EscalateToHuman {
            reason: "graph_loop.policy_profile.unverified_root_cause_requires_human_gate"
                .to_owned(),
        }
    );
    let continuation_receipt = reports[0]
        .continuation_receipt
        .as_ref()
        .expect("root-cause human-gate continuation receipt");
    assert!(matches!(
        &continuation_receipt.action,
        GraphLoopContinuationAction::Defer { reason }
            if reason == "graph_loop.policy_profile.unverified_root_cause_requires_human_gate"
    ));
    assert!(
        continuation_receipt
            .diagnostics
            .contains(&"policy_profile=profile.causal".to_owned())
    );
    let human_gate_receipt = reports[0]
        .human_gate_receipt
        .as_ref()
        .expect("root-cause human gate receipt");
    assert_eq!(human_gate_receipt.gate_id.as_str(), "human-gate:run:0");
    assert_eq!(human_gate_receipt.required_review, HumanReviewKind::General);
    assert!(matches!(
        human_gate_receipt.proposed_next_action,
        GraphLoopNextAction::ContinueWithGraph(_)
    ));
    assert!(reports[0].failure_classification_receipt.is_some());
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
    let continuation_receipt = reports[0]
        .continuation_receipt
        .as_ref()
        .expect("stop-on-failed execution should record terminal continuation receipt");
    assert_eq!(continuation_receipt.run_id.as_str(), "run");
    assert_eq!(continuation_receipt.iteration_id.get(), 0);
    assert!(matches!(
        &continuation_receipt.action,
        GraphLoopContinuationAction::Deny { reason }
            if reason == "graph_loop.execution_failed"
    ));
    assert!(reports[0].failure_classification_receipt.is_some());
}
