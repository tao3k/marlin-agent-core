// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_kernel::{
    GraphLoopContinuationAction, GraphLoopController, GraphLoopExecutionBudget,
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphLoopFailureKind,
    GraphLoopHumanReviewError, GraphLoopNextAction, GraphLoopRunRequest, GraphLoopStopPolicy,
    HumanDecision, HumanDecisionReceipt, HumanReviewKind, LoopContinuationCapability,
    LoopContinuationPolicy, LoopGraph, LoopPolicyProfile,
};
use marlin_agent_runtime::{
    GraphLoopRunRegistryError, GraphLoopRunStatus, GraphLoopRunWaitStatus, TokioAgentRuntime,
};

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
    let observation = runtime.graph_loop_runs().read_registry(|registry| {
        registry
            .inspect_run(&"run".into(), 1)
            .observation
            .expect("suspended run")
    });
    assert_eq!(observation.status, GraphLoopRunStatus::AwaitingHuman);
    assert_eq!(
        observation.pending_human_gate,
        Some(human_gate_receipt.clone())
    );
    assert_eq!(observation.terminal_status, None);
    assert_eq!(
        runtime
            .graph_loop_runs()
            .read_registry(|registry| registry.wait_run(&"run".into(), 1).status),
        GraphLoopRunWaitStatus::AwaitingHuman
    );
}

#[tokio::test]
async fn reviewable_controller_resumes_only_after_matching_approved_gate() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "review-run",
        LoopGraph {
            graph_id: "graph-initial".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(2).require_human_gate());
    let (runtime, _events) = TokioAgentRuntime::new(16);
    let (task, review) = controller()
        .with_continuation_planner(ContinueOncePlanner)
        .spawn_reviewable_loop(request, &runtime);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if runtime
                .graph_loop_runs()
                .read_registry(|registry| registry.wait_run(&"review-run".into(), 0).status)
                == GraphLoopRunWaitStatus::AwaitingHuman
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("controller should publish the gate");
    assert!(!task.is_finished());
    assert!(matches!(
        review
            .decide(HumanDecisionReceipt::new(
                "wrong-gate",
                HumanDecision::Approved
            ))
            .await,
        Err(GraphLoopHumanReviewError::Rejected(
            GraphLoopRunRegistryError::GateMismatch { .. }
        ))
    ));
    assert!(!task.is_finished());
    review
        .decide(HumanDecisionReceipt::new(
            "human-gate:review-run:0",
            HumanDecision::Approved,
        ))
        .await
        .expect("matching approval admitted");
    let reports = task.join().await.expect("controller task joins");
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[1].execution_result.snapshot.graph_id, "graph-next");
    assert_eq!(
        reports[0]
            .human_decision_receipt
            .as_ref()
            .map(|receipt| &receipt.decision),
        Some(&HumanDecision::Approved)
    );
    let observation = runtime.graph_loop_runs().read_registry(|registry| {
        registry
            .inspect_run(&"review-run".into(), 0)
            .observation
            .expect("run")
    });
    assert_eq!(observation.status, GraphLoopRunStatus::Completed);
    assert!(observation.pending_human_gate.is_none());
}

#[tokio::test]
async fn reviewable_controller_rejection_never_executes_proposed_graph() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "review-rejected",
        LoopGraph {
            graph_id: "graph-initial".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(2).require_human_gate());
    let (runtime, _events) = TokioAgentRuntime::new(16);
    let (task, review) = controller()
        .with_continuation_planner(ContinueOncePlanner)
        .spawn_reviewable_loop(request, &runtime);
    review
        .decide(HumanDecisionReceipt::new(
            "human-gate:review-rejected:0",
            HumanDecision::Rejected,
        ))
        .await
        .expect("matching rejection admitted");
    let reports = task.join().await.expect("controller task joins");
    assert_eq!(reports.len(), 1);
    assert_eq!(
        reports[0]
            .human_decision_receipt
            .as_ref()
            .map(|receipt| &receipt.decision),
        Some(&HumanDecision::Rejected)
    );
    assert_eq!(
        runtime
            .graph_loop_runs()
            .read_registry(|registry| { registry.run_status(&"review-rejected".into()) }),
        Some(GraphLoopRunStatus::Failed)
    );
}

#[tokio::test]
async fn closing_review_handle_cancels_pending_gate() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "review-closed",
        LoopGraph {
            graph_id: "graph-initial".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(2).require_human_gate());
    let (runtime, _events) = TokioAgentRuntime::new(16);
    let (task, review) = controller()
        .with_continuation_planner(ContinueOncePlanner)
        .spawn_reviewable_loop(request, &runtime);
    drop(review);
    let reports = task.join().await.expect("controller task joins");
    assert_eq!(reports.len(), 1);
    let observation = runtime.graph_loop_runs().read_registry(|registry| {
        registry
            .inspect_run(&"review-closed".into(), 0)
            .observation
            .expect("run")
    });
    assert_eq!(observation.status, GraphLoopRunStatus::Cancelled);
    assert!(observation.pending_human_gate.is_none());
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
