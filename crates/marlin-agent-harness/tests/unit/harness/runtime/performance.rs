use marlin_agent_harness::{AgentHarness, HarnessGraphBuilder, HarnessRuntime};
use marlin_agent_kernel::{GraphLoopExecutionRequest, TokioGraphLoopKernel};
use marlin_agent_protocol::{
    AgentScenario, LoopEvidence, LoopEvidenceKind, LoopPerformanceEvidence,
    PERFORMANCE_EVIDENCE_KEYS,
};

use super::support::EventfulExecutor;

#[tokio::test]
async fn harness_execution_report_carries_performance_benchmark_evidence() {
    let scenario = AgentScenario::new("bench").expecting_evidence(LoopEvidenceKind::Performance);
    let graph = HarnessGraphBuilder::new("graph")
        .node("node-1", "eventful")
        .build();
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("eventful", EventfulExecutor);
    let mut harness = HarnessRuntime::new(16);
    let performance_evidence: LoopEvidence = LoopPerformanceEvidence {
        subject: "src/runtime.rs".to_owned(),
        benchmark_command: "cargo bench -p marlin-agent-harness".to_owned(),
        baseline: "p95=10ms".to_owned(),
        regression_threshold: "5%".to_owned(),
        latency_or_throughput: "throughput=1000/s".to_owned(),
        allocation_profile: "allocations=steady".to_owned(),
        profile_artifact: "target/criterion/report/index.html".to_owned(),
    }
    .into();

    harness.record_evidence(performance_evidence);

    let report = harness.execute_graph(&scenario, &kernel, request).await;
    let evaluated = AgentHarness::evaluate_execution_report(&scenario, &report);
    let detail = report.evidence[0]
        .detail
        .as_deref()
        .expect("performance detail");

    assert!(report.assertion.is_none());
    assert_eq!(report.evidence[0].kind, LoopEvidenceKind::Performance);
    assert!(evaluated.is_success());
    for key in PERFORMANCE_EVIDENCE_KEYS {
        assert!(
            detail.contains(key),
            "missing performance evidence key {key}"
        );
    }
}
