use marlin_agent_harness::{AgentHarness, HarnessRuntime};
use marlin_agent_protocol::{AgentScenario, LoopEvidenceKind};
use marlin_agent_test_support::{
    ScriptedChunkGate, ScriptedModelStream, scripted_stream_gate_evidence,
};

#[tokio::test]
async fn harness_consumes_scripted_stream_gate_evidence_without_live_llm() {
    let gate = ScriptedChunkGate::closed();
    let collection = tokio::spawn(
        ScriptedModelStream::single_text_delta("stream delta")
            .with_chunk_gate(gate.clone())
            .collect(),
    );

    gate.release_next();
    let receipt = collection
        .await
        .expect("scripted stream task should complete");
    let scenario =
        AgentScenario::new("scripted-stream-gate").expecting_evidence(LoopEvidenceKind::Runtime);
    let mut harness = HarnessRuntime::new(4);
    harness.record_evidence(scripted_stream_gate_evidence(
        "review-stream",
        &receipt,
        &gate,
    ));

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    let detail = harness.evidence()[0]
        .detail
        .as_deref()
        .expect("stream gate evidence detail");

    assert!(report.is_success());
    assert!(detail.contains("chunk_count=1"));
    assert!(detail.contains("gate_sequences=[1]"));
    assert!(detail.contains("live_llm=false"));
}
