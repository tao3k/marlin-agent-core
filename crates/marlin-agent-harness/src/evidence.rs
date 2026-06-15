//! Evidence evaluator for agent scenario contracts.

use std::collections::BTreeSet;

use marlin_agent_protocol::{AgentEvent, AgentEventTopic, AgentSpanName};

use crate::{
    AgentHarnessEvidence, AgentHarnessEvidenceGraph, AgentHarnessEvidenceGraphEdge,
    AgentHarnessEvidenceGraphEdgeKind, AgentHarnessEvidenceGraphNode,
    AgentHarnessEvidenceGraphNodeKind, AgentHarnessEvidenceKind, AgentHarnessScenario,
    AgentHarnessScenarioContract, runtime::AgentHarnessExecutionReport,
};

/// Result of validating a scenario transcript and evidence set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentHarnessReport {
    pub scenario_id: String,
    pub evidence: Vec<AgentHarnessEvidence>,
    pub evidence_graph: AgentHarnessEvidenceGraph,
    pub diagnostics: Vec<String>,
}

impl AgentHarnessReport {
    pub fn is_success(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Stateless evaluator for agent scenario contracts.
#[derive(Clone, Debug, Default)]
pub struct AgentHarness;

impl AgentHarness {
    pub fn evaluate(
        scenario: &AgentHarnessScenario,
        events: &[AgentEvent],
        evidence: &[AgentHarnessEvidence],
    ) -> AgentHarnessReport {
        Self::evaluate_with_span_names(scenario, events, evidence, &BTreeSet::new())
    }

    pub fn evaluate_contract(
        contract: &AgentHarnessScenarioContract,
        events: &[AgentEvent],
        evidence: &[AgentHarnessEvidence],
    ) -> AgentHarnessReport {
        let report = Self::evaluate(&contract.scenario, events, evidence);
        append_contract_schema_diagnostic(contract, report)
    }

    fn evaluate_with_span_names(
        scenario: &AgentHarnessScenario,
        events: &[AgentEvent],
        evidence: &[AgentHarnessEvidence],
        span_names: &BTreeSet<AgentSpanName>,
    ) -> AgentHarnessReport {
        let present_evidence = evidence
            .iter()
            .filter(|fact| fact.present)
            .map(|fact| fact.kind.clone())
            .collect::<BTreeSet<_>>();
        let event_topics = events
            .iter()
            .map(AgentEvent::topic_id)
            .collect::<BTreeSet<_>>();

        let mut diagnostics = missing_evidence_diagnostics(scenario, &present_evidence);
        diagnostics.extend(missing_event_diagnostics(scenario, &event_topics));
        diagnostics.extend(missing_span_diagnostics(scenario, span_names));

        AgentHarnessReport {
            scenario_id: scenario.id().to_owned(),
            evidence: evidence.to_vec(),
            evidence_graph: build_agent_harness_evidence_graph(scenario, evidence),
            diagnostics,
        }
    }

    pub fn evaluate_execution_report(
        scenario: &AgentHarnessScenario,
        report: &AgentHarnessExecutionReport,
    ) -> AgentHarnessReport {
        let span_names = report
            .span_names
            .iter()
            .map(|span_name| AgentSpanName::new(span_name.as_str()))
            .collect::<BTreeSet<_>>();

        Self::evaluate_with_span_names(scenario, &report.events, &report.evidence, &span_names)
    }

    pub fn evaluate_contract_execution_report(
        contract: &AgentHarnessScenarioContract,
        report: &AgentHarnessExecutionReport,
    ) -> AgentHarnessReport {
        let report = Self::evaluate_execution_report(&contract.scenario, report);
        append_contract_schema_diagnostic(contract, report)
    }
}

fn build_agent_harness_evidence_graph(
    scenario: &AgentHarnessScenario,
    evidence: &[AgentHarnessEvidence],
) -> AgentHarnessEvidenceGraph {
    let intent_node_id = "intent:scenario".to_owned();
    let intent_detail = scenario
        .description()
        .map(str::to_owned)
        .unwrap_or_else(|| "scenario contract".to_owned());
    let mut graph = AgentHarnessEvidenceGraph::from_agent_harness_evidence(
        format!("scenario:{}", scenario.id()),
        evidence,
    )
    .with_node(
        AgentHarnessEvidenceGraphNode::present(
            intent_node_id.clone(),
            AgentHarnessEvidenceGraphNodeKind::HumanIntent,
            scenario.id(),
        )
        .with_detail(intent_detail),
    );

    for (index, fact) in evidence.iter().enumerate() {
        let evidence_node_id = format!("evidence:{index}");
        graph = graph.with_edge(
            AgentHarnessEvidenceGraphEdge::new(
                intent_node_id.clone(),
                evidence_node_id.clone(),
                AgentHarnessEvidenceGraphEdgeKind::Requires,
            )
            .with_detail("scenario intent requires this evidence fact"),
        );
        if fact.present {
            graph = graph.with_edge(
                AgentHarnessEvidenceGraphEdge::new(
                    evidence_node_id,
                    intent_node_id.clone(),
                    AgentHarnessEvidenceGraphEdgeKind::Supports,
                )
                .with_detail("present evidence supports the scenario intent"),
            );
        }
    }

    graph
}

fn append_contract_schema_diagnostic(
    contract: &AgentHarnessScenarioContract,
    mut report: AgentHarnessReport,
) -> AgentHarnessReport {
    if !contract.is_supported_schema() {
        report.diagnostics.insert(
            0,
            format!(
                "unsupported scenario contract schema `{}`",
                contract.schema_id
            ),
        );
    }

    report
}

fn missing_evidence_diagnostics(
    scenario: &AgentHarnessScenario,
    present_evidence: &BTreeSet<AgentHarnessEvidenceKind>,
) -> Vec<String> {
    scenario
        .expected_evidence
        .iter()
        .filter(|kind| !present_evidence.contains(*kind))
        .map(|kind| format!("missing expected evidence `{kind:?}`"))
        .collect()
}

fn missing_event_diagnostics(
    scenario: &AgentHarnessScenario,
    event_topics: &BTreeSet<AgentEventTopic>,
) -> Vec<String> {
    scenario
        .steps()
        .iter()
        .flat_map(|step| {
            step.expected_event_topics
                .iter()
                .filter(|topic| !event_topics.contains(*topic))
                .map(|topic| {
                    format!(
                        "missing expected event topic `{topic}` for step {}",
                        step.name
                    )
                })
        })
        .collect()
}

fn missing_span_diagnostics(
    scenario: &AgentHarnessScenario,
    span_names: &BTreeSet<AgentSpanName>,
) -> Vec<String> {
    scenario
        .steps()
        .iter()
        .flat_map(|step| {
            step.expected_span_names
                .iter()
                .filter(|span_name| !span_names.contains(*span_name))
                .map(|span_name| {
                    format!("missing expected span `{span_name}` for step {}", step.name)
                })
        })
        .collect()
}
