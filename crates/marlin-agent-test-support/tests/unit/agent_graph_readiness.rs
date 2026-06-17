use marlin_agent_harness_types::{AgentHarnessEvidence, AgentHarnessEvidenceKind};
use marlin_agent_test_support::{
    AGENT_GRAPH_READINESS_REPLAY_ARTIFACT_ID, AGENT_GRAPH_READINESS_REPLAY_CONTRACT_JSON,
    AgentGraphReadinessReplayArtifactLoadError, agent_graph_readiness_replay_artifact_fixture,
    load_agent_graph_readiness_replay_artifact,
};

#[test]
fn agent_graph_readiness_replay_artifact_loads_no_execution_evidence() {
    let artifact = agent_graph_readiness_replay_artifact_fixture();
    let contract = artifact.contract();
    let scenario = artifact.scenario();
    let evidence = artifact.replay_evidence();

    assert!(contract.is_supported_schema());
    assert_eq!(scenario.id(), AGENT_GRAPH_READINESS_REPLAY_ARTIFACT_ID);
    assert_eq!(scenario.steps().len(), 1);
    assert_eq!(
        scenario.steps()[0].input["artifact"],
        AGENT_GRAPH_READINESS_REPLAY_ARTIFACT_ID
    );
    assert_eq!(
        scenario.expected_evidence,
        vec![
            AgentHarnessEvidenceKind::Runtime,
            AgentHarnessEvidenceKind::Visibility,
        ]
    );
    assert_eq!(evidence.len(), 2);
    assert!(detail_contains(evidence, "planning_status=Planned"));
    assert!(detail_contains(evidence, "projection_status=Projected"));
    assert!(detail_contains(evidence, "readiness_status=Ready"));
    assert!(detail_contains(evidence, "root_loop_entry=loop.planner"));
    assert!(detail_contains(evidence, "graph_loop_execution=false"));
    assert!(detail_contains(evidence, "controller_execution=false"));
    assert!(detail_contains(evidence, "tool_execution=false"));
    assert!(detail_contains(evidence, "live_llm=false"));
    assert!(detail_contains(
        evidence,
        "org_memory_scope_ref=memory.scope.project"
    ));
    assert!(detail_contains(
        evidence,
        "gerbil_policy_scope_ref=policy.scope.implementation"
    ));
    assert!(detail_contains(evidence, "memory_query=false"));
    assert!(detail_contains(evidence, "policy_program=false"));
}

#[test]
fn agent_graph_readiness_replay_artifact_loads_from_serialized_contract() {
    let artifact =
        load_agent_graph_readiness_replay_artifact(AGENT_GRAPH_READINESS_REPLAY_CONTRACT_JSON)
            .expect("serialized AgentGraph readiness contract should load");
    let (contract, evidence) = artifact.into_parts();

    assert!(contract.is_supported_schema());
    assert_eq!(
        contract.scenario.id(),
        AGENT_GRAPH_READINESS_REPLAY_ARTIFACT_ID
    );
    assert_eq!(contract.scenario.expected_evidence.len(), 2);
    assert_eq!(evidence.len(), 2);
    assert!(detail_contains(&evidence, "graph_loop_execution=false"));
}

#[test]
fn agent_graph_readiness_replay_artifact_rejects_unsupported_schema() {
    let error = load_agent_graph_readiness_replay_artifact(
        r#"{
  "schema_id": "marlin.agent.scenario.v0",
  "scenario": {
    "agent_scenario": {
      "id": "agent-graph-readiness-replay"
    }
  }
}"#,
    )
    .expect_err("unsupported schema should be rejected");

    assert!(matches!(
        error,
        AgentGraphReadinessReplayArtifactLoadError::UnsupportedSchema { .. }
    ));
}

fn detail_contains(evidence: &[AgentHarnessEvidence], needle: &str) -> bool {
    evidence
        .iter()
        .filter_map(|entry| entry.detail.as_deref())
        .any(|detail| detail.contains(needle))
}
