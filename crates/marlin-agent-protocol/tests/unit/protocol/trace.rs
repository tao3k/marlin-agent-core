use marlin_agent_protocol::{
    AgentEvent, AgentExecutionTrace, AgentSpanName, AgentTraceSpanRecord,
    GRAPH_POLICY_PROPOSAL_SPAN_NAME, GraphId, GraphLoopExecutionStatus, GraphLoopStrategy,
    GraphLoopStrategyId, GraphPolicyProposal, GraphPolicyProposalReceipt,
    GraphPolicyProposalStatus, LoopGraph,
};

#[test]
fn agent_trace_span_record_keeps_name_and_fields() {
    let record = AgentTraceSpanRecord::new("agent.provider").with_field("node_id", "plan");

    assert_eq!(record.name, AgentSpanName::new("agent.provider"));
    assert_eq!(
        record.fields.get("node_id").map(String::as_str),
        Some("plan")
    );
}

#[test]
fn agent_trace_span_record_projects_graph_policy_proposal_receipts() {
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::native_scheme("scheme-loop-ranker", "v1"),
        LoopGraph {
            graph_id: "turn-frontier-1".to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
        },
        "sha256:input",
        "sha256:output",
    );
    let accepted = GraphPolicyProposalReceipt::accepted(&proposal);
    let rejected = GraphPolicyProposalReceipt::rejected(
        &proposal,
        vec!["graph_policy_proposal.nodes_empty".to_string()],
    );

    let accepted_span = AgentTraceSpanRecord::graph_policy_proposal_receipt(&accepted);
    let rejected_span = AgentTraceSpanRecord::graph_policy_proposal_receipt(&rejected);

    assert_eq!(
        accepted_span.name,
        AgentSpanName::new(GRAPH_POLICY_PROPOSAL_SPAN_NAME)
    );
    assert_eq!(
        accepted_span.fields.get("strategy_id").map(String::as_str),
        Some("scheme-loop-ranker")
    );
    assert!(accepted_span.is_graph_policy_proposal());
    assert_eq!(
        accepted_span.graph_policy_proposal_strategy_id(),
        Some(GraphLoopStrategyId::new("scheme-loop-ranker"))
    );
    assert_eq!(
        accepted_span.graph_policy_proposal_status(),
        Some(GraphPolicyProposalStatus::Accepted)
    );
    assert_eq!(
        accepted_span.graph_policy_proposal_selected_graph_id(),
        Some(GraphId::new("turn-frontier-1"))
    );
    assert_eq!(
        accepted_span.fields.get("status").map(String::as_str),
        Some("Accepted")
    );
    assert_eq!(
        accepted_span
            .fields
            .get("selected_graph_id")
            .map(String::as_str),
        Some("turn-frontier-1")
    );
    assert_eq!(
        rejected_span.fields.get("status").map(String::as_str),
        Some("Rejected")
    );
    assert_eq!(
        rejected_span.graph_policy_proposal_status(),
        Some(GraphPolicyProposalStatus::Rejected)
    );
    assert!(
        rejected_span
            .graph_policy_proposal_selected_graph_id()
            .is_none()
    );
    assert_eq!(
        rejected_span
            .fields
            .get("diagnostic_count")
            .map(String::as_str),
        Some("1")
    );
    assert!(!rejected_span.fields.contains_key("selected_graph_id"));

    assert!(!AgentTraceSpanRecord::new("agent.provider").is_graph_policy_proposal());
}

#[test]
fn agent_execution_trace_packages_run_facts() {
    let event = AgentEvent::new("kernel.execution", "failed");
    let result_span = AgentTraceSpanRecord::new("harness.result")
        .with_field("run_id", "run-1")
        .with_field("graph_id", "graph-1")
        .with_field("status", "Failed");
    let provider_span = AgentTraceSpanRecord::new("agent.provider")
        .with_field("node_id", "plan")
        .with_field("executor", "provider");
    let trace = AgentExecutionTrace::new("run-1", "graph-1", GraphLoopExecutionStatus::Failed)
        .with_events(vec![event.clone()])
        .with_spans(vec![result_span.clone(), provider_span.clone()])
        .with_diagnostics(vec!["failed intentionally".to_owned()]);

    assert_eq!(trace.run_id.as_str(), "run-1");
    assert_eq!(trace.graph_id.as_str(), "graph-1");
    assert_eq!(trace.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(trace.events, vec![event]);
    assert_eq!(trace.spans, vec![result_span.clone(), provider_span]);
    assert_eq!(trace.diagnostics, vec!["failed intentionally"]);
    assert!(trace.has_span(&AgentSpanName::new("harness.result")));
    assert_eq!(trace.count_span(&AgentSpanName::new("harness.result")), 1);
    assert_eq!(
        trace
            .find_span(&AgentSpanName::new("harness.result"))
            .map(|span| span.name.as_str()),
        Some("harness.result")
    );
    assert_eq!(trace.spans_with_field("status", "Failed").count(), 1);
    assert_eq!(
        trace.find_span_with_field(&AgentSpanName::new("harness.result"), "run_id", "run-1"),
        Some(&result_span)
    );
    assert_eq!(trace.graph_policy_proposal_spans().count(), 0);

    let summary = trace.summary();
    assert_eq!(summary.run_id.as_str(), "run-1");
    assert_eq!(summary.graph_id.as_str(), "graph-1");
    assert_eq!(summary.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(summary.event_count, 1);
    assert_eq!(summary.span_count, 2);
    assert_eq!(summary.diagnostic_count, 1);
}

#[test]
fn agent_execution_trace_finds_graph_policy_proposal_spans_by_strategy() {
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::native_scheme("scheme-loop-ranker", "v1"),
        LoopGraph {
            graph_id: "turn-frontier-1".to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
        },
        "sha256:input",
        "sha256:output",
    );
    let receipt = GraphPolicyProposalReceipt::accepted(&proposal);
    let strategy_id = GraphLoopStrategyId::new("scheme-loop-ranker");
    let proposal_span = AgentTraceSpanRecord::graph_policy_proposal_receipt(&receipt);
    let trace = AgentExecutionTrace::new(
        "run-1",
        "turn-frontier-1",
        GraphLoopExecutionStatus::Completed,
    )
    .with_spans(vec![proposal_span]);

    assert_eq!(trace.graph_policy_proposal_spans().count(), 1);
    assert!(
        trace
            .find_graph_policy_proposal_span(&strategy_id)
            .is_some()
    );
    assert!(
        trace.has_graph_policy_proposal_status(&strategy_id, GraphPolicyProposalStatus::Accepted)
    );
    assert!(
        !trace.has_graph_policy_proposal_status(&strategy_id, GraphPolicyProposalStatus::Rejected)
    );
}

#[test]
fn agent_execution_traces_support_batch_failure_lookup() {
    let result_span_name = AgentSpanName::new("harness.result");
    let traces = [
        ("run-ok-0", GraphLoopExecutionStatus::Completed, "Completed"),
        ("run-fail-1", GraphLoopExecutionStatus::Failed, "Failed"),
        ("run-ok-2", GraphLoopExecutionStatus::Completed, "Completed"),
        ("run-fail-3", GraphLoopExecutionStatus::Failed, "Failed"),
    ]
    .into_iter()
    .map(|(run_id, status, status_field)| {
        AgentExecutionTrace::new(run_id, "graph", status).with_spans(vec![
            AgentTraceSpanRecord::new(result_span_name.clone())
                .with_field("run_id", run_id)
                .with_field("graph_id", "graph")
                .with_field("status", status_field),
        ])
    })
    .collect::<Vec<_>>();

    let failed_run_ids = traces
        .iter()
        .filter(|trace| trace.status == GraphLoopExecutionStatus::Failed)
        .map(|trace| trace.run_id.as_str())
        .collect::<Vec<_>>();
    let failed_result_run_ids = traces
        .iter()
        .filter_map(|trace| {
            trace
                .find_span_with_field(&result_span_name, "status", "Failed")
                .and_then(|span| span.fields.get("run_id").map(String::as_str))
        })
        .collect::<Vec<_>>();

    assert_eq!(failed_run_ids, vec!["run-fail-1", "run-fail-3"]);
    assert_eq!(failed_result_run_ids, failed_run_ids);
    assert!(traces.iter().all(|trace| trace.has_span(&result_span_name)));
}
