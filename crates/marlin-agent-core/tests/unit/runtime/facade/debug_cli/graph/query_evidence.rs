use marlin_agent_core::{
    ProjectRuntimeQuerySummary,
    protocol::{
        AgentExecutionTrace, FailureClassificationReceipt, GraphLoopExecutionResult,
        GraphLoopExecutionStatus, GraphLoopFailureKind, GraphLoopIterationReport,
        GraphLoopNextAction, GraphNodeExecutionReceipt, GraphQueryContext, GraphQueryFamily,
        GraphQueryResponse, RuntimePlanSnapshot,
    },
    run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn debug_cli_graph_query_reads_evidence_query_facts() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("evidence-query.json");
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
    let response = GraphQueryResponse::from_iteration_reports_evidence(
        "receipt:evidence-query",
        GraphQueryContext::new("project-alpha"),
        "replay evidence",
        vec![report],
    );
    fs::write(
        &input,
        serde_json::to_string(&response).expect("response JSON"),
    )
    .expect("write evidence response");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        input.to_str().expect("utf8 path"),
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: ProjectRuntimeQuerySummary =
        serde_json::from_str(&query.stdout).expect("evidence query summary");
    assert_eq!(summary.family, GraphQueryFamily::Evidence);
    assert_eq!(summary.match_count, 2);
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
fn debug_cli_graph_query_reads_failure_query_facts() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("failure-query.json");
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
    let response = GraphQueryResponse::from_iteration_reports_failure(
        "receipt:failure-query",
        GraphQueryContext::new("project-alpha"),
        "policy failure",
        vec![report],
    );
    fs::write(
        &input,
        serde_json::to_string(&response).expect("response JSON"),
    )
    .expect("write failure response");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        input.to_str().expect("utf8 path"),
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: ProjectRuntimeQuerySummary =
        serde_json::from_str(&query.stdout).expect("failure query summary");
    assert_eq!(summary.family, GraphQueryFamily::Failure);
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
        summary.source_anchor_ids[0].as_str(),
        "graph-loop:run:iteration:0"
    );
    assert_eq!(summary.score_basis_points, vec![9_500]);
}
