use marlin_agent_core::{
    ProjectRuntimeQuerySummary,
    protocol::{
        AgentExecutionTrace, FailureClassificationReceipt, GraphLoopExecutionResult,
        GraphLoopExecutionStatus, GraphLoopFailureKind, GraphLoopIterationReport,
        GraphLoopNextAction, GraphNodeExecutionReceipt, GraphQueryFamily, RuntimePlanSnapshot,
    },
    run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn debug_cli_graph_query_projects_evidence_family_from_loop_reports() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("loop-report.json");
    let report = GraphLoopIterationReport::new(
        0,
        GraphLoopExecutionResult::completed(
            RuntimePlanSnapshot {
                run_id: "run".to_owned(),
                graph_id: "graph".to_owned(),
                active_node: None,
            },
            vec!["plan".to_owned()],
        )
        .with_node_receipts(vec![GraphNodeExecutionReceipt::completed("plan", "echo")]),
        GraphLoopNextAction::StopCompleted,
    )
    .with_trace(AgentExecutionTrace::new(
        "run",
        "graph",
        GraphLoopExecutionStatus::Completed,
    ));
    fs::write(
        &input,
        serde_json::to_string(&vec![report]).expect("loop report JSON"),
    )
    .expect("write loop report");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--family",
        "evidence",
        "--input",
        input.to_str().expect("utf8 path"),
        "--receipt-id",
        "receipt:cli-evidence",
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: ProjectRuntimeQuerySummary =
        serde_json::from_str(&query.stdout).expect("evidence query summary");
    assert_eq!(summary.receipt_id.as_str(), "receipt:cli-evidence");
    assert_eq!(summary.family, GraphQueryFamily::Evidence);
    assert_eq!(summary.query.as_str(), "loop report evidence");
    assert_eq!(
        summary
            .source_project_ids
            .iter()
            .map(|project_id| project_id.as_str())
            .collect::<Vec<_>>(),
        vec!["debug-loop-report"]
    );
    assert_eq!(
        summary
            .evidence_ids
            .iter()
            .map(|evidence_id| evidence_id.as_str())
            .collect::<Vec<_>>(),
        vec!["node-receipt:run:0:plan", "trace:run:iteration:0"]
    );
    assert_eq!(
        summary
            .match_receipt_ids
            .iter()
            .map(|receipt_id| receipt_id.as_str())
            .collect::<Vec<_>>(),
        vec!["node-receipt:run:0:plan", "trace:run:iteration:0"]
    );
}

#[test]
fn debug_cli_graph_query_projects_failure_family_from_loop_reports() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("loop-report.json");
    let receipt = FailureClassificationReceipt::new(
        "failure:run:0",
        "run",
        0,
        GraphLoopFailureKind::PolicyFailure,
    )
    .with_requires_human(true)
    .with_diagnostic("sandbox.denied");
    let report = GraphLoopIterationReport::new(
        0,
        GraphLoopExecutionResult::failed(
            RuntimePlanSnapshot {
                run_id: "run".to_owned(),
                graph_id: "graph".to_owned(),
                active_node: None,
            },
            vec!["sandbox.denied".to_owned()],
        ),
        GraphLoopNextAction::StopFailed,
    )
    .with_failure_classification_receipt(receipt);
    fs::write(
        &input,
        serde_json::to_string(&vec![report]).expect("loop report JSON"),
    )
    .expect("write loop report");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--family",
        "failure",
        "--input",
        input.to_str().expect("utf8 path"),
        "--receipt-id",
        "receipt:cli-failure",
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: ProjectRuntimeQuerySummary =
        serde_json::from_str(&query.stdout).expect("failure query summary");
    assert_eq!(summary.receipt_id.as_str(), "receipt:cli-failure");
    assert_eq!(summary.family, GraphQueryFamily::Failure);
    assert_eq!(summary.query.as_str(), "loop report failures");
    assert_eq!(
        summary
            .source_project_ids
            .iter()
            .map(|project_id| project_id.as_str())
            .collect::<Vec<_>>(),
        vec!["debug-loop-report"]
    );
    assert_eq!(
        summary
            .evidence_ids
            .iter()
            .map(|evidence_id| evidence_id.as_str())
            .collect::<Vec<_>>(),
        vec!["failure:run:0"]
    );
    assert_eq!(
        summary
            .match_receipt_ids
            .iter()
            .map(|receipt_id| receipt_id.as_str())
            .collect::<Vec<_>>(),
        vec!["failure:run:0"]
    );
    assert_eq!(
        summary
            .source_anchor_ids
            .iter()
            .map(|anchor_id| anchor_id.as_str())
            .collect::<Vec<_>>(),
        vec!["graph-loop:run:iteration:0"]
    );
    assert_eq!(summary.score_basis_points, vec![9_500]);
}

#[test]
fn debug_cli_graph_query_rejects_non_projection_family_for_raw_input() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("loop-report.json");
    fs::write(&input, "{}").expect("write input");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--family",
        "memory",
        "--input",
        input.to_str().expect("utf8 path"),
    ]);

    assert_eq!(query.status, 2);
    assert!(
        query.stderr.contains("use evidence or failure"),
        "{}",
        query.stderr
    );
}

#[test]
fn debug_cli_graph_query_rejects_family_with_org_backed_options() {
    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--family",
        "evidence",
        "--input",
        "missing.json",
        "--org-memory-fixture",
        "memory.org",
    ]);

    assert_eq!(query.status, 2);
    assert!(
        query
            .stderr
            .contains("--family cannot be combined with Org-backed graph query options"),
        "{}",
        query.stderr
    );
}
