use std::collections::BTreeMap;

use marlin_agent_protocol::{
    AgentExecutionTrace, FailureClassificationReceipt, GraphLoopContinuationAction,
    GraphLoopContinuationReceipt, GraphLoopEvidencePolicy, GraphLoopExecutionBudget,
    GraphLoopExecutionRequest, GraphLoopExecutionResult, GraphLoopExecutionStatus,
    GraphLoopFailureKind, GraphLoopGovernancePolicy, GraphLoopInputDrainPolicy,
    GraphLoopIterationReport, GraphLoopNextAction, GraphLoopRunRequest, GraphLoopStopPolicy,
    GraphToolBatchExecutionMode, HumanDecision, HumanDecisionReceipt, HumanGateReceipt,
    HumanReviewKind, LoopContinuationCapability, LoopContinuationPolicy, LoopEvidenceCapturePolicy,
    LoopFailurePolicy, LoopGraph, LoopHumanGatePolicy, LoopMemoryPolicy, LoopNodeSpec,
    LoopPolicyProfile, LoopQueuePolicy, LoopSelfEvolutionPolicy, LoopToolBatchPolicy,
    RuntimePlanSnapshot,
};

#[test]
fn graph_loop_run_request_records_stop_budget_and_replayable_evidence_policy() {
    let initial_request = GraphLoopExecutionRequest::new("loop-run", loop_graph())
        .with_budget(GraphLoopExecutionBudget::max_node_executions(2));

    let request = GraphLoopRunRequest::new(initial_request)
        .with_policy_profile(LoopPolicyProfile::new("profile:replayable-runtime"))
        .with_stop_policy(
            GraphLoopStopPolicy::max_iterations(2)
                .with_max_duration_ms(1_000)
                .stop_on_failed_execution()
                .require_human_gate(),
        )
        .with_iteration_budget(GraphLoopExecutionBudget::max_node_executions(1))
        .with_evidence_policy(GraphLoopEvidencePolicy::replayable_runtime());

    let value = serde_json::to_value(&request).expect("loop run request serializes");

    assert_eq!(value["initial_request"]["run_id"], "loop-run");
    assert_eq!(value["initial_request"]["budget"]["max_node_executions"], 2);
    assert_eq!(
        value["policy_profile"]["profile_id"],
        "profile:replayable-runtime"
    );
    assert_eq!(
        value["policy_profile"]["continuation_policy"]["require_decision_receipt"],
        true
    );
    assert_eq!(value["stop_policy"]["max_iterations"], 2);
    assert_eq!(value["stop_policy"]["max_duration_ms"], 1_000);
    assert_eq!(value["stop_policy"]["stop_on_failed_execution"], true);
    assert_eq!(value["stop_policy"]["human_gate_required"], true);
    assert_eq!(value["iteration_budget"]["max_node_executions"], 1);
    assert_eq!(value["evidence_policy"]["capture_runtime_events"], true);
    assert_eq!(value["evidence_policy"]["capture_node_receipts"], true);
    assert_eq!(value["evidence_policy"]["capture_trace"], true);
    assert_eq!(value["evidence_policy"]["replayable"], true);
}

#[test]
fn graph_loop_run_request_records_governance_policy_handoff() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "governed-loop-run",
        loop_graph(),
    ))
    .with_governance_policy(GraphLoopGovernancePolicy::nono("nono-profile"));

    let value = serde_json::to_value(&request).expect("loop run request serializes");

    assert_eq!(
        value["governance_policy"]["state_policy"]["read_before_run"],
        true
    );
    assert_eq!(
        value["governance_policy"]["state_policy"]["write_receipt_on_pass"],
        true
    );
    assert_eq!(
        value["governance_policy"]["sandbox_policy"]["backend"],
        "nono"
    );
    assert_eq!(
        value["governance_policy"]["sandbox_policy"]["profile_ref"],
        "nono-profile"
    );
    assert_eq!(
        value["governance_policy"]["sandbox_policy"]["filesystem_scope"],
        "runtime"
    );
    assert_eq!(
        value["governance_policy"]["session_policy"]["session_kind"],
        "sub-agent"
    );
    assert_eq!(
        value["governance_policy"]["session_policy"]["visible_namespaces"],
        serde_json::json!(["system", "workspace", "tools"])
    );
    assert_eq!(
        value["governance_policy"]["verifier_policy"]["pass_statuses"],
        serde_json::json!(["Completed"])
    );
    assert_eq!(
        value["governance_policy"]["verifier_policy"]["retry_statuses"],
        serde_json::json!(["Failed"])
    );
    assert_eq!(
        value["governance_policy"]["verifier_policy"]["human_audit_statuses"],
        serde_json::json!(["Cancelled"])
    );
}

#[test]
fn loop_policy_profile_records_configurable_policy_surfaces() {
    let profile = LoopPolicyProfile::new("profile:causal-improvement")
        .with_queue_policy(LoopQueuePolicy {
            steering_drain_policy: GraphLoopInputDrainPolicy::DrainAll,
            follow_up_drain_policy: GraphLoopInputDrainPolicy::DrainOne,
            prioritize_steering: true,
        })
        .with_continuation_policy(LoopContinuationPolicy {
            allow_accept: LoopContinuationCapability::enabled(),
            allow_deny: LoopContinuationCapability::enabled(),
            allow_defer: LoopContinuationCapability::enabled(),
            allow_rewrite: LoopContinuationCapability::enabled(),
            require_decision_receipt: true,
        })
        .with_tool_batch_policy(LoopToolBatchPolicy {
            execution_mode: GraphToolBatchExecutionMode::Parallel,
            force_sequential: false,
            require_all_tools_to_terminate: true,
            require_before_after_hook_receipts: true,
        })
        .with_evidence_policy(LoopEvidenceCapturePolicy {
            capture_events: true,
            capture_node_receipts: true,
            capture_tool_receipts: true,
            capture_content_receipts: true,
            capture_trace: true,
            replayable: true,
        })
        .with_failure_policy(LoopFailurePolicy {
            classify_failure: true,
            allow_retry: true,
            allow_repair_graph: true,
            escalate_unknown_to_human: true,
        })
        .with_memory_policy(LoopMemoryPolicy {
            allow_project_promotion: true,
            require_contract_validated: true,
            allow_cross_project_memory: false,
            require_source_anchor: true,
        })
        .with_human_gate_policy(LoopHumanGatePolicy {
            require_for_permission_escalation: true,
            require_for_policy_change: true,
            require_for_cross_project_memory: true,
            require_for_unverified_root_cause: true,
        })
        .with_self_evolution_policy(LoopSelfEvolutionPolicy {
            require_failure_observation_receipt: true,
            require_root_cause_receipt: true,
            require_intervention_receipt: true,
            require_progress_receipt: true,
            allow_policy_update: false,
        });

    let value = serde_json::to_value(&profile).expect("loop policy profile serializes");

    assert_eq!(value["profile_id"], "profile:causal-improvement");
    assert_eq!(value["queue_policy"]["steering_drain_policy"], "DrainAll");
    assert_eq!(value["queue_policy"]["follow_up_drain_policy"], "DrainOne");
    assert_eq!(value["tool_batch_policy"]["execution_mode"], "Parallel");
    assert_eq!(value["tool_batch_policy"]["force_sequential"], false);
    assert_eq!(
        value["continuation_policy"]["require_decision_receipt"],
        true
    );
    assert_eq!(value["model_route_policy"]["require_route_receipt"], true);
    assert_eq!(value["memory_policy"]["allow_project_promotion"], true);
    assert_eq!(
        value["human_gate_policy"]["require_for_policy_change"],
        true
    );
    assert_eq!(
        value["self_evolution_policy"]["require_progress_receipt"],
        true
    );
}

#[test]
fn graph_loop_iteration_report_records_next_action_and_optional_trace() {
    let snapshot = RuntimePlanSnapshot {
        run_id: "loop-run".to_owned(),
        graph_id: "graph".to_owned(),
        active_node: None,
    };
    let report = GraphLoopIterationReport::new(
        1,
        GraphLoopExecutionResult::completed(snapshot, vec!["rank".to_owned()]),
        GraphLoopNextAction::ContinueWithGraph(loop_graph()),
    )
    .with_trace(AgentExecutionTrace::new(
        "loop-run",
        "graph",
        GraphLoopExecutionStatus::Completed,
    ));

    let value = serde_json::to_value(&report).expect("iteration report serializes");

    assert_eq!(value["iteration"], 1);
    assert_eq!(value["execution_result"]["status"], "Completed");
    assert_eq!(value["execution_result"]["visited_nodes"][0], "rank");
    assert_eq!(
        value["next_action"]["ContinueWithGraph"]["graph_id"],
        "graph"
    );
    assert_eq!(value["trace"]["run_id"], "loop-run");
    assert_eq!(value["trace"]["graph_id"], "graph");
    assert_eq!(value["trace"]["status"], "Completed");
}

#[test]
fn graph_loop_iteration_report_records_typed_continuation_human_and_failure_receipts() {
    let snapshot = RuntimePlanSnapshot {
        run_id: "loop-run".to_owned(),
        graph_id: "graph".to_owned(),
        active_node: None,
    };
    let next_graph = loop_graph();
    let human_gate_receipt = HumanGateReceipt::new(
        "gate-1",
        "loop-run",
        2,
        "requires_review: permission escalation",
    )
    .with_required_review(HumanReviewKind::PermissionEscalation)
    .with_proposed_next_action(GraphLoopNextAction::ContinueWithGraph(next_graph.clone()));
    let report = GraphLoopIterationReport::new(
        2,
        GraphLoopExecutionResult::completed(snapshot, vec!["rank".to_owned()]),
        GraphLoopNextAction::EscalateToHuman {
            reason: "requires_review: permission escalation".to_owned(),
        },
    )
    .with_continuation_receipt(GraphLoopContinuationReceipt::new(
        "loop-run",
        2,
        GraphLoopContinuationAction::Defer {
            reason: "human gate required".to_owned(),
        },
    ))
    .with_human_gate_receipt(human_gate_receipt)
    .with_human_decision_receipt(
        HumanDecisionReceipt::new("gate-1", HumanDecision::Approved)
            .with_reviewer("reviewer:operator")
            .with_approved_next_graph(next_graph.clone()),
    )
    .with_failure_classification_receipt(
        FailureClassificationReceipt::new(
            "failure-1",
            "loop-run",
            2,
            GraphLoopFailureKind::PolicyFailure,
        )
        .with_requires_human(true)
        .with_source_node("node-exec-1")
        .with_diagnostic("sandbox.denied")
        .with_suggested_recovery_graph(next_graph),
    );

    let value = serde_json::to_value(&report).expect("iteration report serializes");

    assert_eq!(value["continuation_receipt"]["run_id"], "loop-run");
    assert_eq!(value["continuation_receipt"]["iteration_id"], 2);
    assert_eq!(value["continuation_receipt"]["action"]["action"], "defer");
    assert_eq!(value["human_gate_receipt"]["gate_id"], "gate-1");
    assert_eq!(
        value["human_gate_receipt"]["required_review"],
        "PermissionEscalation"
    );
    assert_eq!(
        value["human_gate_receipt"]["proposed_next_action"]["ContinueWithGraph"]["graph_id"],
        "graph"
    );
    assert_eq!(value["human_decision_receipt"]["decision"], "Approved");
    assert_eq!(
        value["human_decision_receipt"]["reviewer"],
        "reviewer:operator"
    );
    assert_eq!(
        value["failure_classification_receipt"]["failure_kind"],
        "PolicyFailure"
    );
    assert_eq!(
        value["failure_classification_receipt"]["requires_human"],
        true
    );
    assert_eq!(
        value["failure_classification_receipt"]["source_nodes"][0],
        "node-exec-1"
    );
    assert_eq!(
        value["failure_classification_receipt"]["suggested_recovery_graph"]["graph_id"],
        "graph"
    );
}

fn loop_graph() -> LoopGraph {
    LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![LoopNodeSpec {
            id: "rank".to_owned(),
            executor: "gerbil.rank".to_owned(),
            config: BTreeMap::new(),
        }],
        edges: Vec::new(),
    }
}
