use std::time::Duration;

use marlin_agent_harness::{
    AgentHarness, AgentHarnessEvidence, AgentHarnessEvidenceKind, AgentHarnessGraphBuilder,
    AgentHarnessRuntime, AgentHarnessScenario,
};
use marlin_agent_kernel::{GraphLoopExecutionRequest, TokioGraphLoopKernel};
use marlin_agent_protocol::GraphLoopExecutionStatus;
use marlin_agent_test_support::{
    RuntimeStabilityEvidenceInput, ScriptedChunkGate, ScriptedModelStream,
    no_llm_runtime_replay_artifact_fixture, runtime_stability_budget_evidence,
    scripted_stream_gate_evidence,
};

use super::support::EventfulExecutor;

#[tokio::test]
async fn harness_execution_report_composes_no_llm_runtime_evidence_chain() {
    const DURATION_BUDGET: Duration = Duration::from_millis(250);
    const EVENT_BUDGET: usize = 5;
    const SPAN_BUDGET: usize = 32;

    let replay_artifact = no_llm_runtime_replay_artifact_fixture();

    let gate = ScriptedChunkGate::closed();
    let collection = tokio::spawn(
        ScriptedModelStream::single_text_delta("composite stream")
            .with_chunk_gate(gate.clone())
            .collect(),
    );
    gate.release_next();
    let stream_receipt = collection
        .await
        .expect("scripted stream task should complete");
    let stream_evidence =
        scripted_stream_gate_evidence("composite-review-stream", &stream_receipt, &gate);

    let execution_scenario = AgentHarnessScenario::new("composite-runtime-evidence");
    let validation_scenario = replay_artifact
        .scenario()
        .clone()
        .expecting_evidence(AgentHarnessEvidenceKind::Stability);
    let graph = AgentHarnessGraphBuilder::new("graph")
        .node("node-1", "eventful")
        .build();
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("eventful", EventfulExecutor);
    let mut harness = AgentHarnessRuntime::new(16);
    for evidence in replay_artifact.replay_evidence().iter().cloned() {
        harness.record_evidence(evidence);
    }
    harness.record_evidence(stream_evidence);

    let mut report = harness
        .execute_graph(&execution_scenario, &kernel, request)
        .await;
    let custom_event_count = report
        .events
        .iter()
        .filter(|event| event.topic == "test.harness")
        .count();
    let stability_evidence = runtime_stability_budget_evidence(RuntimeStabilityEvidenceInput {
        subject: "crates/marlin-agent-harness/src/runtime.rs".to_owned(),
        stability_command:
            "cargo test -p marlin-agent-harness --test unit_test harness::runtime::composite"
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
        determinism: "scripted-fixtures,scripted-stream,scripted-eventful-executor".to_owned(),
        stability_artifact: "target/agent-harness/stability/runtime-composite.json".to_owned(),
    });
    report.evidence.push(stability_evidence);

    let evaluated = AgentHarness::evaluate_execution_report(&validation_scenario, &report);

    assert_eq!(report.result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(report.summary.status, GraphLoopExecutionStatus::Completed);
    assert!(report.assertion.is_none());
    assert_eq!(custom_event_count, 1);
    assert!(report.summary.event_count <= EVENT_BUDGET);
    assert!(report.summary.duration <= DURATION_BUDGET);
    assert!(report.summary.span_count <= SPAN_BUDGET);
    assert_eq!(report.summary.diagnostic_count, 0);
    assert!(evaluated.is_success());
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == AgentHarnessEvidenceKind::Visibility)
            .count(),
        3
    );
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == AgentHarnessEvidenceKind::Runtime)
            .count(),
        7
    );
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == AgentHarnessEvidenceKind::Stability)
            .count(),
        1
    );
    assert!(detail_contains(&report.evidence, "denied_memory=true"));
    assert!(detail_contains(
        &report.evidence,
        "denied_namespaces=[Memory]"
    ));
    assert!(detail_contains(
        &report.evidence,
        "visibility_contracted=true"
    ));
    assert!(detail_contains(&report.evidence, "policy_decisions=2"));
    assert!(detail_contains(&report.evidence, "live_llm=false"));
    assert!(detail_contains(&report.evidence, "custom_event_count=1"));
}

fn detail_contains(evidence: &[AgentHarnessEvidence], needle: &str) -> bool {
    evidence
        .iter()
        .filter_map(|evidence| evidence.detail.as_deref())
        .any(|detail| detail.contains(needle))
}
