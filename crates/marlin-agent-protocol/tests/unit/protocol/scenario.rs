use marlin_agent_protocol::{
    AGENT_SCENARIO_CONTRACT_SCHEMA_ID, AgentEventTopic, AgentScenario, AgentScenarioContract,
    AgentScenarioStep, AgentSpanName, LoopEvidenceKind,
};

#[test]
fn scenario_records_expected_evidence_and_steps() {
    let scenario = AgentScenario::new("content-loop")
        .with_description("validates content-backed loop evidence")
        .with_step(
            AgentScenarioStep::new("load-content")
                .with_input("path", "LOOP.org")
                .expecting_event_topic("kernel.execution")
                .expecting_span_name("harness.execution"),
        )
        .expecting_evidence(LoopEvidenceKind::Content);

    assert_eq!(scenario.id, "content-loop");
    assert_eq!(scenario.steps[0].input["path"], "LOOP.org");
    assert_eq!(
        scenario.steps[0].expected_span_names,
        vec![AgentSpanName::new("harness.execution")]
    );
    assert_eq!(scenario.expected_evidence, vec![LoopEvidenceKind::Content]);
}

#[test]
fn scenario_contract_accepts_fixture_json_with_defaults() {
    let contract: AgentScenarioContract = serde_json::from_str(
        r#"{
  "schema_id": "marlin.agent.scenario.v1",
  "scenario": {
    "id": "fixture-loop",
    "steps": [
      {
        "name": "run",
        "expected_event_topics": ["kernel.execution"],
        "expected_span_names": ["harness.execution"]
      }
    ],
    "expected_evidence": ["Runtime"]
  }
}"#,
    )
    .expect("scenario contract fixture should deserialize");

    assert_eq!(contract.schema_id, AGENT_SCENARIO_CONTRACT_SCHEMA_ID);
    assert!(contract.is_supported_schema());
    let scenario = contract.into_scenario();
    assert_eq!(scenario.id, "fixture-loop");
    assert_eq!(scenario.steps.len(), 1);
    assert!(scenario.steps[0].input.is_empty());
    assert_eq!(
        scenario.steps[0].expected_event_topics,
        vec![AgentEventTopic::new("kernel.execution")]
    );
    assert_eq!(
        scenario.steps[0].expected_span_names,
        vec![AgentSpanName::new("harness.execution")]
    );
    assert_eq!(scenario.expected_evidence, vec![LoopEvidenceKind::Runtime]);
}
