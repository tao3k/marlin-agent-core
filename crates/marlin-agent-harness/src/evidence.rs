//! Evidence evaluator for agent scenario contracts.

use std::collections::BTreeSet;

use marlin_agent_protocol::{AgentEvent, AgentScenario, LoopEvidence, LoopEvidenceKind};

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
        let present_evidence = evidence
            .iter()
            .filter(|fact| fact.present)
            .map(|fact| fact.kind.clone())
            .collect::<BTreeSet<_>>();
        let event_topics = events
            .iter()
            .map(|event| event.topic.as_str())
            .collect::<BTreeSet<_>>();

        let mut diagnostics = missing_evidence_diagnostics(scenario, &present_evidence);
        diagnostics.extend(missing_event_diagnostics(scenario, &event_topics));

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
        Self::evaluate(scenario, &report.events, &report.evidence)
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
    event_topics: &BTreeSet<&str>,
) -> Vec<String> {
    scenario
        .steps
        .iter()
        .flat_map(|step| {
            step.expected_event_topics
                .iter()
                .filter(|topic| !event_topics.contains(topic.as_str()))
                .map(|topic| {
                    format!(
                        "missing expected event topic `{topic}` for step {}",
                        step.name
                    )
                })
        })
        .collect()
}
