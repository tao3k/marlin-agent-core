use marlin_agent_protocol::{LoopEvidence, LoopEvidenceKind};
use marlin_agent_test_support::{
    NO_LLM_RUNTIME_REPLAY_ARTIFACT_ID, NO_LLM_RUNTIME_REPLAY_CONTRACT_JSON,
    NoLlmRuntimeReplayArtifactLoadError, load_no_llm_runtime_replay_artifact,
    no_llm_runtime_replay_artifact_fixture,
};

#[test]
fn no_llm_runtime_replay_artifact_loads_graph_session_and_hook_evidence() {
    let artifact = no_llm_runtime_replay_artifact_fixture();
    let contract = artifact.contract();
    let scenario = artifact.scenario();
    let evidence = artifact.replay_evidence();

    assert!(contract.is_supported_schema());
    assert_eq!(scenario.id, NO_LLM_RUNTIME_REPLAY_ARTIFACT_ID);
    assert_eq!(scenario.steps.len(), 1);
    assert_eq!(
        scenario.steps[0].input["artifact"],
        NO_LLM_RUNTIME_REPLAY_ARTIFACT_ID
    );
    assert_eq!(
        scenario.expected_evidence,
        vec![LoopEvidenceKind::Visibility, LoopEvidenceKind::Runtime]
    );
    assert_eq!(evidence.len(), 4);
    assert_eq!(
        evidence
            .iter()
            .filter(|entry| entry.kind == LoopEvidenceKind::Visibility)
            .count(),
        3
    );
    assert_eq!(
        evidence
            .iter()
            .filter(|entry| entry.kind == LoopEvidenceKind::Runtime)
            .count(),
        1
    );
    assert!(detail_contains(evidence, "status=Accepted"));
    assert!(detail_contains(evidence, "denied_memory=true"));
    assert!(detail_contains(evidence, "visibility_contracted=true"));
    assert!(detail_contains(evidence, "policy_decisions=2"));
    assert!(detail_contains(evidence, "live_llm=false"));
}

#[test]
fn no_llm_runtime_replay_artifact_loads_from_serialized_contract() {
    let artifact = load_no_llm_runtime_replay_artifact(NO_LLM_RUNTIME_REPLAY_CONTRACT_JSON)
        .expect("serialized replay contract should load");
    let (contract, evidence) = artifact.into_parts();

    assert!(contract.is_supported_schema());
    assert_eq!(contract.scenario.id, NO_LLM_RUNTIME_REPLAY_ARTIFACT_ID);
    assert_eq!(contract.scenario.expected_evidence.len(), 2);
    assert_eq!(evidence.len(), 4);
    assert!(detail_contains(&evidence, "status=Accepted"));
}

#[test]
fn no_llm_runtime_replay_artifact_rejects_unsupported_schema() {
    let error = load_no_llm_runtime_replay_artifact(
        r#"{
  "schema_id": "marlin.agent.scenario.v0",
  "scenario": {
    "id": "no-llm-runtime-replay"
  }
}"#,
    )
    .expect_err("unsupported schema should be rejected");

    assert!(matches!(
        error,
        NoLlmRuntimeReplayArtifactLoadError::UnsupportedSchema { .. }
    ));
}

fn detail_contains(evidence: &[LoopEvidence], needle: &str) -> bool {
    evidence
        .iter()
        .filter_map(|entry| entry.detail.as_deref())
        .any(|detail| detail.contains(needle))
}
