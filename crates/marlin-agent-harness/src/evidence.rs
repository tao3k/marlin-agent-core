//! Evidence evaluator for agent scenario contracts.

use std::collections::BTreeSet;

use marlin_agent_protocol::{
    AgentEvent, AgentEventTopic, AgentScenario, AgentSpanName, LoopEvidence, LoopEvidenceKind,
};

use crate::runtime::HarnessExecutionReport;

/// Result of validating a scenario transcript and evidence set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentHarnessReport {
    pub scenario_id: String,
    pub evidence: Vec<LoopEvidence>,
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
        scenario: &AgentScenario,
        events: &[AgentEvent],
        evidence: &[LoopEvidence],
    ) -> AgentHarnessReport {
        Self::evaluate_with_span_names(scenario, events, evidence, &BTreeSet::new())
    }

    fn evaluate_with_span_names(
        scenario: &AgentScenario,
        events: &[AgentEvent],
        evidence: &[LoopEvidence],
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
            scenario_id: scenario.id.clone(),
            evidence: evidence.to_vec(),
            diagnostics,
        }
    }

    pub fn evaluate_execution_report(
        scenario: &AgentScenario,
        report: &HarnessExecutionReport,
    ) -> AgentHarnessReport {
        let span_names = report
            .span_names
            .iter()
            .map(|span_name| AgentSpanName::new(span_name.as_str()))
            .collect::<BTreeSet<_>>();

        Self::evaluate_with_span_names(scenario, &report.events, &report.evidence, &span_names)
    }
}

fn missing_evidence_diagnostics(
    scenario: &AgentScenario,
    present_evidence: &BTreeSet<LoopEvidenceKind>,
) -> Vec<String> {
    scenario
        .expected_evidence
        .iter()
        .filter(|kind| !present_evidence.contains(*kind))
        .map(|kind| format!("missing expected evidence `{kind:?}`"))
        .collect()
}

fn missing_event_diagnostics(
    scenario: &AgentScenario,
    event_topics: &BTreeSet<AgentEventTopic>,
) -> Vec<String> {
    scenario
        .steps
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
    scenario: &AgentScenario,
    span_names: &BTreeSet<AgentSpanName>,
) -> Vec<String> {
    scenario
        .steps
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
