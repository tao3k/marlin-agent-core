use marlin_agent_harness_types::{AgentHarnessEvidence, AgentHarnessEvidenceKind};
use marlin_agent_test_support::{
    PROJECT_RUNTIME_READ_MODEL_REPLAY_ARTIFACT_ID, PROJECT_RUNTIME_READ_MODEL_REPLAY_CONTRACT_JSON,
    ProjectRuntimeReadModelReplayArtifactLoadError,
    load_project_runtime_read_model_replay_artifact,
    project_runtime_read_model_replay_artifact_fixture,
};

#[test]
fn project_runtime_read_model_replay_artifact_loads_read_family_evidence() {
    let artifact = project_runtime_read_model_replay_artifact_fixture();
    let contract = artifact.contract();
    let scenario = artifact.scenario();
    let evidence = artifact.replay_evidence();

    assert!(contract.is_supported_schema());
    assert_eq!(scenario.id(), PROJECT_RUNTIME_READ_MODEL_REPLAY_ARTIFACT_ID);
    assert_eq!(scenario.steps().len(), 1);
    assert_eq!(
        scenario.steps()[0].input["artifact"],
        PROJECT_RUNTIME_READ_MODEL_REPLAY_ARTIFACT_ID
    );
    assert_eq!(
        scenario.expected_evidence,
        vec![
            AgentHarnessEvidenceKind::Visibility,
            AgentHarnessEvidenceKind::Runtime,
            AgentHarnessEvidenceKind::Tool,
            AgentHarnessEvidenceKind::Content,
        ]
    );
    assert_eq!(evidence.len(), 6);
    assert_eq!(
        evidence
            .iter()
            .filter(|entry| entry.kind == AgentHarnessEvidenceKind::Visibility)
            .count(),
        2
    );
    assert_eq!(
        evidence
            .iter()
            .filter(|entry| entry.kind == AgentHarnessEvidenceKind::Runtime)
            .count(),
        2
    );
    assert_eq!(
        evidence
            .iter()
            .filter(|entry| entry.kind == AgentHarnessEvidenceKind::Tool)
            .count(),
        1
    );
    assert_eq!(
        evidence
            .iter()
            .filter(|entry| entry.kind == AgentHarnessEvidenceKind::Content)
            .count(),
        1
    );
    assert!(detail_contains(evidence, "sibling_transcript_hidden=true"));
    assert!(detail_contains(evidence, "context_pack_bounded=true"));
    assert!(detail_contains(
        evidence,
        "parent_content_id=content:turn-7"
    ));
    assert!(detail_contains(evidence, "root_kind=ToolCapability"));
    assert!(detail_contains(evidence, "live_llm=false"));
    assert!(detail_contains(evidence, "sandbox_execution=false"));
    assert!(detail_contains(evidence, "unsupported_org_fixture=true"));
}

#[test]
fn project_runtime_read_model_replay_artifact_loads_from_serialized_contract() {
    let artifact = load_project_runtime_read_model_replay_artifact(
        PROJECT_RUNTIME_READ_MODEL_REPLAY_CONTRACT_JSON,
    )
    .expect("serialized read-model contract should load");
    let (contract, evidence) = artifact.into_parts();

    assert!(contract.is_supported_schema());
    assert_eq!(
        contract.scenario.id(),
        PROJECT_RUNTIME_READ_MODEL_REPLAY_ARTIFACT_ID
    );
    assert_eq!(contract.scenario.expected_evidence.len(), 4);
    assert_eq!(evidence.len(), 6);
    assert!(detail_contains(&evidence, "family=Memory"));
    assert!(detail_contains(&evidence, "family=Tool"));
}

#[test]
fn project_runtime_read_model_replay_artifact_rejects_unsupported_schema() {
    let error = load_project_runtime_read_model_replay_artifact(
        r#"{
  "schema_id": "marlin.agent.scenario.v0",
  "scenario": {
    "agent_scenario": {
      "id": "project-runtime-read-model-replay"
    }
  }
}"#,
    )
    .expect_err("unsupported schema should be rejected");

    assert!(matches!(
        error,
        ProjectRuntimeReadModelReplayArtifactLoadError::UnsupportedSchema { .. }
    ));
}

fn detail_contains(evidence: &[AgentHarnessEvidence], needle: &str) -> bool {
    evidence
        .iter()
        .filter_map(|entry| entry.detail.as_deref())
        .any(|detail| detail.contains(needle))
}
