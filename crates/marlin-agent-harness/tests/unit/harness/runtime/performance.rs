use std::time::Duration;

use marlin_agent_harness::{AgentHarness, HarnessGraphBuilder, HarnessRuntime};
use marlin_agent_kernel::{GraphLoopExecutionRequest, TokioGraphLoopKernel};
use marlin_agent_protocol::{
    AgentScenario, GraphLoopExecutionStatus, LoopEvidence, LoopEvidenceKind,
    LoopPerformanceEvidence, PERFORMANCE_EVIDENCE_KEYS, STABILITY_EVIDENCE_KEYS,
};
use marlin_agent_test_support::{
    RuntimeStabilityEvidenceInput, runtime_stability_budget_diagnostics,
    runtime_stability_budget_evidence,
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

#[tokio::test]
async fn harness_execution_report_reports_missing_runtime_stability_evidence() {
    let execution_scenario = AgentScenario::new("runtime-stability-missing-evidence");
    let validation_scenario = AgentScenario::new("runtime-stability-missing-evidence")
        .expecting_evidence(LoopEvidenceKind::Stability);
    let graph = HarnessGraphBuilder::new("graph")
        .node("node-1", "eventful")
        .build();
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("eventful", EventfulExecutor);
    let mut harness = HarnessRuntime::new(16);

    let report = harness
        .execute_graph(&execution_scenario, &kernel, request)
        .await;
    let evaluated = AgentHarness::evaluate_execution_report(&validation_scenario, &report);

    assert_eq!(report.result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(
        evaluated.diagnostics,
        vec!["missing expected evidence `Stability`"]
    );
}

#[test]
fn harness_runtime_stability_budget_reports_negative_gate_diagnostics() {
    let input = RuntimeStabilityEvidenceInput {
        subject: "crates/marlin-agent-harness/src/runtime.rs".to_owned(),
        stability_command:
            "cargo test -p marlin-agent-harness --test unit_test harness::runtime::performance"
                .to_owned(),
        duration: Duration::from_millis(251),
        duration_budget: Duration::from_millis(250),
        event_count: 6,
        event_budget: 5,
        custom_event_count: Some(1),
        span_count: 33,
        span_budget: 32,
        diagnostic_count: 2,
        state_growth: "event_queue=drained,trace_spans=bounded".to_owned(),
        determinism: "scripted-eventful-executor,node_order=stable".to_owned(),
        stability_artifact: "target/agent-harness/stability/runtime-performance.json".to_owned(),
    };

    assert_eq!(
        runtime_stability_budget_diagnostics(&input),
        vec![
            "runtime stability duration budget exceeded: actual_ms=251 budget_ms=250",
            "runtime stability event budget exceeded: actual=6 budget=5",
            "runtime stability span budget exceeded: actual=33 budget=32",
            "runtime stability diagnostics present: count=2",
        ]
    );
}

#[tokio::test]
async fn harness_execution_report_carries_runtime_stability_budget_evidence() {
    const DURATION_BUDGET: Duration = Duration::from_millis(250);
    const EVENT_BUDGET: usize = 5;
    const SPAN_BUDGET: usize = 32;

    let execution_scenario = AgentScenario::new("runtime-stability-gate");
    let validation_scenario = AgentScenario::new("runtime-stability-gate")
        .expecting_evidence(LoopEvidenceKind::Stability);
    let graph = HarnessGraphBuilder::new("graph")
        .node("node-1", "eventful")
        .build();
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("eventful", EventfulExecutor);
    let mut harness = HarnessRuntime::new(16);

    let mut report = harness
        .execute_graph(&execution_scenario, &kernel, request)
        .await;

    assert_eq!(report.result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(report.summary.status, GraphLoopExecutionStatus::Completed);
    assert!(report.assertion.is_none());
    assert_eq!(report.summary.event_count, report.events.len());
    assert_eq!(report.summary.span_count, report.trace_spans.len());
    assert_eq!(
        report.summary.diagnostic_count,
        report.result.diagnostics.len()
    );
    let custom_event_count = report
        .events
        .iter()
        .filter(|event| event.topic == "test.harness")
        .count();

    assert_eq!(custom_event_count, 1);
    assert!(report.summary.event_count <= EVENT_BUDGET);
    assert!(
        report.summary.duration <= DURATION_BUDGET,
        "runtime stability gate exceeded duration budget: {:?} > {:?}",
        report.summary.duration,
        DURATION_BUDGET
    );
    assert!(
        report.summary.span_count <= SPAN_BUDGET,
        "runtime stability gate exceeded span budget: {} > {}",
        report.summary.span_count,
        SPAN_BUDGET
    );
    assert_eq!(report.summary.diagnostic_count, 0);

    let stability_evidence = runtime_stability_budget_evidence(RuntimeStabilityEvidenceInput {
        subject: "crates/marlin-agent-harness/src/runtime.rs".to_owned(),
        stability_command:
            "cargo test -p marlin-agent-harness --test unit_test harness::runtime::performance"
                .to_owned(),
        duration: report.summary.duration,
        duration_budget: DURATION_BUDGET,
        event_count: report.summary.event_count,
        event_budget: EVENT_BUDGET,
        custom_event_count: Some(custom_event_count),
        span_count: report.summary.span_count,
        span_budget: SPAN_BUDGET,
        diagnostic_count: report.summary.diagnostic_count,
        state_growth: "event_queue=drained,trace_spans=bounded".to_owned(),
        determinism: "scripted-eventful-executor,node_order=stable".to_owned(),
        stability_artifact: "target/agent-harness/stability/runtime-performance.json".to_owned(),
    });

    report.evidence.push(stability_evidence);

    let evaluated = AgentHarness::evaluate_execution_report(&validation_scenario, &report);
    let detail = report
        .evidence
        .iter()
        .find(|evidence| evidence.kind == LoopEvidenceKind::Stability)
        .and_then(|evidence| evidence.detail.as_deref())
        .expect("stability detail");

    assert!(evaluated.is_success());
    for key in STABILITY_EVIDENCE_KEYS {
        assert!(detail.contains(key), "missing stability evidence key {key}");
    }
    for expected_observation in [
        "event_count=5",
        "event_budget=5",
        "custom_event_count=1",
        "span_budget=32",
        "diagnostic_count=0",
        "duration_budget_ms=250",
    ] {
        assert!(
            detail.contains(expected_observation),
            "missing stability observation {expected_observation}"
        );
    }
}
