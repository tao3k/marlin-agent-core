use marlin_agent_harness::{
    AgentHarness, AgentHarnessEvidence, AgentHarnessEvidenceGraphEdgeKind,
    AgentHarnessEvidenceGraphNodeKind, AgentHarnessEvidenceKind, AgentHarnessScenario,
    AgentHarnessScenarioContract,
};
use marlin_agent_protocol::{AgentEvent, AgentScenarioStep};
use marlin_agent_runtime::observability;

#[test]
fn harness_accepts_present_evidence_and_event_topics() {
    let scenario = AgentHarnessScenario::new("loop")
        .with_step(
            AgentScenarioStep::new("run")
                .expecting_event_topic(observability::TOPIC_KERNEL_EXECUTION),
        )
        .expecting_evidence(AgentHarnessEvidenceKind::Runtime);
    let events = vec![AgentEvent::new(
        observability::TOPIC_KERNEL_EXECUTION,
        "run started",
    )];
    let evidence = vec![AgentHarnessEvidence::present(
        AgentHarnessEvidenceKind::Runtime,
        "tokio",
    )];
    let contract = AgentHarnessScenarioContract::new(scenario);

    let report = AgentHarness::evaluate_contract(&contract, &events, &evidence);

    assert!(report.is_success());
    assert_eq!(report.scenario_id, "loop");
    assert!(
        report
            .evidence_graph
            .has_node_kind(AgentHarnessEvidenceGraphNodeKind::HumanIntent)
    );
    assert!(
        report
            .evidence_graph
            .has_node_kind(AgentHarnessEvidenceGraphNodeKind::ExecutionReceipt)
    );
    assert!(report.evidence_graph.edges.iter().any(|edge| {
        edge.kind == AgentHarnessEvidenceGraphEdgeKind::Requires
            && edge.from == "intent:scenario"
            && edge.to == "evidence:0"
    }));
    assert!(report.evidence_graph.edges.iter().any(|edge| {
        edge.kind == AgentHarnessEvidenceGraphEdgeKind::Supports
            && edge.from == "evidence:0"
            && edge.to == "intent:scenario"
    }));
}

#[test]
fn harness_reports_unsupported_scenario_contract_schema() {
    let scenario = AgentHarnessScenario::new("loop");
    let mut contract = AgentHarnessScenarioContract::new(scenario);
    contract.schema_id = "marlin.agent.scenario.v0".to_owned();

    let report = AgentHarness::evaluate_contract(&contract, &[], &[]);

    assert_eq!(
        report.diagnostics,
        vec!["unsupported scenario contract schema `marlin.agent.scenario.v0`"]
    );
}

#[test]
fn harness_reports_missing_evidence_and_event_topics() {
    let scenario = AgentHarnessScenario::new("loop")
        .with_step(
            AgentScenarioStep::new("run")
                .expecting_event_topic(observability::TOPIC_KERNEL_EXECUTION)
                .expecting_span_name(observability::SPAN_HARNESS_EXECUTION),
        )
        .expecting_evidence(AgentHarnessEvidenceKind::Runtime);

    let report = AgentHarness::evaluate(&scenario, &[], &[]);

    assert_eq!(
        report.diagnostics,
        vec![
            "missing expected evidence `Runtime`",
            "missing expected event topic `kernel.execution` for step run",
            "missing expected span `harness.execution` for step run",
        ]
    );
    assert_eq!(report.evidence_graph.summary().nodes, 1);
    assert!(
        report
            .evidence_graph
            .has_node_kind(AgentHarnessEvidenceGraphNodeKind::HumanIntent)
    );
}
