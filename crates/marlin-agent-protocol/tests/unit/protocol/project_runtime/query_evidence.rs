use marlin_agent_protocol::{
    AgentExecutionTrace, GraphLoopExecutionResult, GraphLoopExecutionStatus,
    GraphLoopIterationReport, GraphLoopNextAction, GraphNodeExecutionReceipt, GraphQueryContext,
    GraphQueryFamily, GraphQueryResponse, RuntimePlanSnapshot,
};

#[test]
fn iteration_reports_project_trace_and_node_receipts_to_evidence_query_response() {
    let report = GraphLoopIterationReport::new(
        0,
        GraphLoopExecutionResult::completed(
            RuntimePlanSnapshot {
                run_id: "run-1".to_owned(),
                graph_id: "graph".to_owned(),
                active_node: None,
            },
            vec!["plan".to_owned()],
        )
        .with_node_receipts(vec![
            GraphNodeExecutionReceipt::completed("plan", "echo"),
            GraphNodeExecutionReceipt::failed("apply", "shell", vec!["tool.denied".to_owned()]),
        ]),
        GraphLoopNextAction::StopCompleted,
    )
    .with_trace(AgentExecutionTrace::new(
        "run-1",
        "graph",
        GraphLoopExecutionStatus::Completed,
    ));

    let response = GraphQueryResponse::from_iteration_reports_evidence(
        "receipt:evidence-reports",
        GraphQueryContext::new("project-alpha"),
        "controller replay evidence",
        vec![report],
    );

    assert_eq!(response.receipt_id.as_str(), "receipt:evidence-reports");
    assert_eq!(response.request.family, GraphQueryFamily::Evidence);
    assert_eq!(response.request.query, "controller replay evidence");
    assert_eq!(response.matches.len(), 3);
    assert_eq!(
        response.matches[0]
            .evidence_id
            .as_ref()
            .expect("trace evidence")
            .as_str(),
        "trace:run-1:iteration:0"
    );
    assert_eq!(
        response.matches[0]
            .source_anchor_id
            .as_ref()
            .expect("trace source anchor")
            .as_str(),
        "graph-loop:run-1:iteration:0:trace"
    );
    assert_eq!(
        response.matches[1]
            .evidence_id
            .as_ref()
            .expect("plan node evidence")
            .as_str(),
        "node-receipt:run-1:0:plan"
    );
    assert_eq!(
        response.matches[2]
            .source_anchor_id
            .as_ref()
            .expect("apply node source anchor")
            .as_str(),
        "graph-loop:run-1:iteration:0:node:apply"
    );
    assert_eq!(response.matches[2].score_basis_points.as_u16(), 9_000);
}
