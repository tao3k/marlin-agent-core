use marlin_agent_protocol::{
    FailureClassificationReceipt, GraphLoopExecutionResult, GraphLoopFailureKind,
    GraphLoopIterationReport, GraphLoopNextAction, GraphQueryContext, GraphQueryFamily,
    GraphQueryResponse, RuntimePlanSnapshot,
};

#[test]
fn failure_classification_receipt_projects_to_failure_query_response() {
    let receipt = FailureClassificationReceipt::new(
        "failure:run-1:0",
        "run-1",
        0,
        GraphLoopFailureKind::PolicyFailure,
    )
    .with_requires_human(true)
    .with_diagnostic("sandbox.denied");

    let response = GraphQueryResponse::from_failure_classification_receipt(
        "receipt:failure-query",
        GraphQueryContext::new("project-alpha"),
        receipt,
    );

    assert_eq!(response.receipt_id.as_str(), "receipt:failure-query");
    assert_eq!(response.request.family, GraphQueryFamily::Failure);
    assert_eq!(
        response.request.context.project_id.as_str(),
        "project-alpha"
    );
    assert!(response.request.query.contains("PolicyFailure"));
    assert_eq!(response.matches.len(), 1);
    let query_match = &response.matches[0];
    assert_eq!(query_match.source_project_id.as_str(), "project-alpha");
    assert_eq!(
        query_match
            .evidence_id
            .as_ref()
            .expect("evidence id")
            .as_str(),
        "failure:run-1:0"
    );
    assert_eq!(
        query_match
            .receipt_id
            .as_ref()
            .expect("receipt id")
            .as_str(),
        "failure:run-1:0"
    );
    assert_eq!(
        query_match
            .source_anchor_id
            .as_ref()
            .expect("source anchor")
            .as_str(),
        "graph-loop:run-1:iteration:0"
    );
    assert_eq!(query_match.score_basis_points.as_u16(), 9_500);
    assert!(query_match.summary.contains("requires_human=true"));
}

#[test]
fn iteration_reports_project_failure_receipts_to_failure_query_response() {
    let report_without_failure = GraphLoopIterationReport::new(
        0,
        GraphLoopExecutionResult::failed(
            RuntimePlanSnapshot {
                run_id: "run-1".to_owned(),
                graph_id: "graph".to_owned(),
                active_node: None,
            },
            vec!["executor.timeout".to_owned()],
        ),
        GraphLoopNextAction::StopFailed,
    );
    let report_with_failure = GraphLoopIterationReport::new(
        1,
        GraphLoopExecutionResult::failed(
            RuntimePlanSnapshot {
                run_id: "run-1".to_owned(),
                graph_id: "graph".to_owned(),
                active_node: None,
            },
            vec!["sandbox.denied".to_owned()],
        ),
        GraphLoopNextAction::StopFailed,
    )
    .with_failure_classification_receipt(
        FailureClassificationReceipt::new(
            "failure:run-1:1",
            "run-1",
            1,
            GraphLoopFailureKind::PolicyFailure,
        )
        .with_requires_human(true),
    );

    let response = GraphQueryResponse::from_iteration_reports_failure(
        "receipt:failure-reports",
        GraphQueryContext::new("project-alpha"),
        "controller failure receipts",
        vec![report_without_failure, report_with_failure],
    );

    assert_eq!(response.receipt_id.as_str(), "receipt:failure-reports");
    assert_eq!(response.request.family, GraphQueryFamily::Failure);
    assert_eq!(response.request.query, "controller failure receipts");
    assert_eq!(response.matches.len(), 1);
    let query_match = &response.matches[0];
    assert_eq!(
        query_match
            .evidence_id
            .as_ref()
            .expect("failure evidence")
            .as_str(),
        "failure:run-1:1"
    );
    assert_eq!(
        query_match
            .source_anchor_id
            .as_ref()
            .expect("source anchor")
            .as_str(),
        "graph-loop:run-1:iteration:1"
    );
}
