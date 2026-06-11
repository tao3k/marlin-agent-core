use marlin_agent_core::{
    AgentExecutionTrace, AgentExecutionTraceSummary, AgentSpanName, AgentTraceSpanRecord,
    GraphLoopExecutionStatus, HookDispatcher, HookRegistry, LoopEvidence, LoopEvidenceKind,
    LoopPerformanceEvidence, PERFORMANCE_EVIDENCE_KEYS, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, RuntimeExecutionIdentity,
};

#[test]
fn core_facade_exposes_environment_resolver() {
    let environment = RuntimeEnvironmentResolver::new().resolve(
        RuntimeEnvironmentRequest::default()
            .with_custom_home("/tmp/marlin-home")
            .with_cwd("/tmp/workspace"),
    );

    assert_eq!(
        environment
            .home
            .as_ref()
            .expect("home should be resolved")
            .path,
        std::path::PathBuf::from("/tmp/marlin-home")
    );
}

#[test]
fn core_facade_exposes_hook_dispatcher() {
    let dispatcher = HookDispatcher::new(HookRegistry::new());

    assert_eq!(dispatcher.registry().registrations().len(), 0);
}

#[test]
fn core_facade_exposes_runtime_execution_identity() {
    let identity = RuntimeExecutionIdentity::new("run-core", "graph-core");

    assert_eq!(identity.run_id(), "run-core");
    assert_eq!(identity.graph_id(), "graph-core");
}

#[test]
fn core_facade_exposes_execution_trace_summary() {
    let result_span = AgentTraceSpanRecord::new("harness.result")
        .with_field("run_id", "run")
        .with_field("status", "Completed");
    let trace = AgentExecutionTrace::new("run", "graph", GraphLoopExecutionStatus::Completed)
        .with_spans(vec![result_span]);
    let summary: AgentExecutionTraceSummary = trace.summary();

    assert_eq!(summary.run_id.as_str(), "run");
    assert_eq!(summary.graph_id.as_str(), "graph");
    assert_eq!(summary.status, GraphLoopExecutionStatus::Completed);
    assert!(
        trace
            .find_span_with_field(&AgentSpanName::new("harness.result"), "run_id", "run")
            .is_some()
    );
}

#[test]
fn core_facade_exposes_performance_evidence_contract() {
    let evidence: LoopEvidence = LoopPerformanceEvidence {
        subject: "core-runtime".to_owned(),
        benchmark_command: "cargo bench -p marlin-agent-core".to_owned(),
        baseline: "p95=10ms".to_owned(),
        regression_threshold: "5%".to_owned(),
        latency_or_throughput: "throughput=1000/s".to_owned(),
        allocation_profile: "allocations=steady".to_owned(),
        profile_artifact: "target/criterion/core/index.html".to_owned(),
    }
    .into();
    let detail = evidence.detail.as_deref().expect("performance detail");

    assert_eq!(evidence.kind, LoopEvidenceKind::Performance);
    assert_eq!(evidence.subject, "core-runtime");
    for key in PERFORMANCE_EVIDENCE_KEYS {
        assert!(
            detail.contains(key),
            "missing performance evidence key {key}"
        );
    }
}
