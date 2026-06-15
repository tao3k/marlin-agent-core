use std::sync::Arc;

use marlin_agent_harness::{
    AgentHarness, AgentHarnessEvidence, AgentHarnessEvidenceKind, AgentHarnessRuntime,
    AgentHarnessScenario, StaticProviderRuntime,
};
use marlin_agent_runtime::TokioAgentRuntime;

#[tokio::test]
async fn static_provider_runtime_feeds_harness_turn_evidence_without_live_llm() {
    let provider = Arc::new(StaticProviderRuntime::<String, String>::new(
        "scripted provider response".to_owned(),
    ));
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let output = runtime
        .spawn_provider(provider, "turn input".to_owned())
        .join()
        .await
        .expect("provider task should finish");
    let scenario = AgentHarnessScenario::new("fake-provider-turn")
        .expecting_evidence(AgentHarnessEvidenceKind::Runtime);
    let mut harness = AgentHarnessRuntime::new(4);
    harness.record_evidence(
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Runtime,
            "fake-provider-turn:static",
        )
        .with_detail(format!(
            "request=turn input response={output} live_llm=false"
        )),
    );

    let report = AgentHarness::evaluate(&scenario, &[], harness.evidence());
    let detail = harness.evidence()[0]
        .detail
        .as_deref()
        .expect("fake provider turn detail");

    assert_eq!(output, "scripted provider response");
    assert!(report.is_success());
    assert!(detail.contains("response=scripted provider response"));
    assert!(detail.contains("live_llm=false"));
}
