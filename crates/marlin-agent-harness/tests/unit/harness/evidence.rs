use marlin_agent_harness::AgentHarness;
use marlin_agent_protocol::{
    AgentEvent, AgentScenario, AgentScenarioContract, AgentScenarioStep, LoopEvidence,
    LoopEvidenceKind,
};
use marlin_agent_runtime::observability;

#[test]
fn harness_accepts_present_evidence_and_event_topics() {
    let scenario = AgentScenario::new("loop")
        .with_step(
            AgentScenarioStep::new("run")
                .expecting_event_topic(observability::TOPIC_KERNEL_EXECUTION),
        )
        .expecting_evidence(LoopEvidenceKind::Runtime);
    let events = vec![AgentEvent::new(
        observability::TOPIC_KERNEL_EXECUTION,
        "run started",
    )];
    let evidence = vec![LoopEvidence::present(LoopEvidenceKind::Runtime, "tokio")];
    let contract = AgentScenarioContract::new(scenario);

    let report = AgentHarness::evaluate_contract(&contract, &events, &evidence);

    assert!(report.is_success());
    assert_eq!(report.scenario_id, "loop");
}

#[test]
fn harness_reports_unsupported_scenario_contract_schema() {
    let scenario = AgentScenario::new("loop");
    let mut contract = AgentScenarioContract::new(scenario);
    contract.schema_id = "marlin.agent.scenario.v0".to_owned();

    let report = AgentHarness::evaluate_contract(&contract, &[], &[]);

    assert_eq!(
        report.diagnostics,
        vec!["unsupported scenario contract schema `marlin.agent.scenario.v0`"]
    );
}

#[test]
fn harness_reports_missing_evidence_and_event_topics() {
    let scenario = AgentScenario::new("loop")
        .with_step(
            AgentScenarioStep::new("run")
                .expecting_event_topic(observability::TOPIC_KERNEL_EXECUTION)
                .expecting_span_name(observability::SPAN_HARNESS_EXECUTION),
        )
        .expecting_evidence(LoopEvidenceKind::Runtime);

    let report = AgentHarness::evaluate(&scenario, &[], &[]);

    assert_eq!(
        report.diagnostics,
        vec![
            "missing expected evidence `Runtime`",
            "missing expected event topic `kernel.execution` for step run",
            "missing expected span `harness.execution` for step run",
        ]
    );
}
