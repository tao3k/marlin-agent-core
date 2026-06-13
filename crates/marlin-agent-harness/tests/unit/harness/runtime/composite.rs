use std::time::Duration;

use marlin_agent_harness::{AgentHarness, HarnessGraphBuilder, HarnessRuntime};
use marlin_agent_kernel::{GraphLoopExecutionRequest, TokioGraphLoopKernel};
use marlin_agent_protocol::{AgentScenario, GraphLoopExecutionStatus, LoopEvidenceKind};
use marlin_agent_sessions::SessionKind;
use marlin_agent_test_support::{
    RuntimeStabilityEvidenceInput, ScriptedChunkGate, ScriptedModelStream,
    custom_hook_policy_receipt_fixture, custom_sub_agent_start_hook_summary_fixture,
    hook_dispatch_replay_evidence, runtime_stability_budget_evidence,
    scripted_stream_gate_evidence, sub_agent_hook_dispatch_selection_fixture,
    sub_agent_memory_denied_fixture, sub_agent_memory_session_visibility_evidence,
};

use super::support::EventfulExecutor;

#[tokio::test]
async fn harness_execution_report_composes_no_llm_runtime_evidence_chain() {
    const DURATION_BUDGET: Duration = Duration::from_millis(250);
    const EVENT_BUDGET: usize = 5;
    const SPAN_BUDGET: usize = 32;

    let memory_fixture = sub_agent_memory_denied_fixture();
    let (child_session, isolation_receipt) = memory_fixture.parent_session().child_session(
        SessionKind::SubAgent,
        memory_fixture.config().child_session_id(),
        memory_fixture.requested_visibility(),
    );
    let visibility_evidence =
        sub_agent_memory_session_visibility_evidence(&child_session, &isolation_receipt);

    let hook_summary = custom_sub_agent_start_hook_summary_fixture();
    let hook_selection = sub_agent_hook_dispatch_selection_fixture();
    let hook_policy = custom_hook_policy_receipt_fixture();
    let hook_evidence = hook_dispatch_replay_evidence(&hook_summary, &hook_selection, &hook_policy);

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

    let execution_scenario = AgentScenario::new("composite-runtime-evidence");
    let validation_scenario = AgentScenario::new("composite-runtime-evidence")
        .expecting_evidence(LoopEvidenceKind::Visibility)
        .expecting_evidence(LoopEvidenceKind::Runtime)
        .expecting_evidence(LoopEvidenceKind::Stability);
    let graph = HarnessGraphBuilder::new("graph")
        .node("node-1", "eventful")
        .build();
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("eventful", EventfulExecutor);
    let mut harness = HarnessRuntime::new(16);
    harness.record_evidence(visibility_evidence);
    harness.record_evidence(hook_evidence);
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
            .filter(|evidence| evidence.kind == LoopEvidenceKind::Visibility)
            .count(),
        1
    );
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == LoopEvidenceKind::Runtime)
            .count(),
        2
    );
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == LoopEvidenceKind::Stability)
            .count(),
        1
    );
    assert!(detail_contains(&report.evidence, "denied_memory=true"));
    assert!(detail_contains(&report.evidence, "policy_decisions=2"));
    assert!(detail_contains(&report.evidence, "live_llm=false"));
    assert!(detail_contains(&report.evidence, "custom_event_count=1"));
}

fn detail_contains(evidence: &[marlin_agent_protocol::LoopEvidence], needle: &str) -> bool {
    evidence
        .iter()
        .filter_map(|evidence| evidence.detail.as_deref())
        .any(|detail| detail.contains(needle))
}
