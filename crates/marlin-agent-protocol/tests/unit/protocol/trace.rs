use marlin_agent_protocol::{
    AgentEvent, AgentExecutionTrace, AgentSpanName, AgentTraceSpanRecord, GraphLoopExecutionStatus,
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

    let summary = trace.summary();
    assert_eq!(summary.run_id.as_str(), "run-1");
    assert_eq!(summary.graph_id.as_str(), "graph-1");
    assert_eq!(summary.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(summary.event_count, 1);
    assert_eq!(summary.span_count, 2);
    assert_eq!(summary.diagnostic_count, 1);
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
