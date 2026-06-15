//! Helpers for projecting no-LLM runtime summaries into `HarnessStabilityEvidence`.

use std::time::Duration;

use marlin_agent_harness_types::{AgentHarnessEvidence, AgentHarnessStabilityEvidence};

/// Input for projecting one no-LLM runtime stability gate into typed evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStabilityEvidenceInput {
    pub subject: String,
    pub stability_command: String,
    pub duration: Duration,
    pub duration_budget: Duration,
    pub event_count: usize,
    pub event_budget: usize,
    pub custom_event_count: Option<usize>,
    pub span_count: usize,
    pub span_budget: usize,
    pub diagnostic_count: usize,
    pub state_growth: String,
    pub determinism: String,
    pub stability_artifact: String,
}

/// Project runtime summary counters into stability evidence consumed by harness tests.
pub fn runtime_stability_budget_evidence(
    input: RuntimeStabilityEvidenceInput,
) -> AgentHarnessEvidence {
    let custom_event_detail = input
        .custom_event_count
        .map(|count| format!(",custom_event_count={count}"))
        .unwrap_or_default();

    AgentHarnessStabilityEvidence {
        subject: input.subject,
        stability_command: input.stability_command,
        iteration_window: "single-run,no-llm".to_owned(),
        latency_distribution: format!(
            "duration_ms={},duration_budget_ms={}",
            input.duration.as_millis(),
            input.duration_budget.as_millis()
        ),
        resource_delta: format!(
            "event_count={},event_budget={}{}\
             ,span_count={},span_budget={},diagnostic_count={}",
            input.event_count,
            input.event_budget,
            custom_event_detail,
            input.span_count,
            input.span_budget,
            input.diagnostic_count
        ),
        state_growth: input.state_growth,
        determinism: input.determinism,
        stability_artifact: input.stability_artifact,
    }
    .into()
}

/// Return stable diagnostics when a no-LLM runtime stability gate exceeds its budget.
pub fn runtime_stability_budget_diagnostics(input: &RuntimeStabilityEvidenceInput) -> Vec<String> {
    let mut diagnostics = Vec::new();

    if input.duration > input.duration_budget {
        diagnostics.push(format!(
            "runtime stability duration budget exceeded: actual_ms={} budget_ms={}",
            input.duration.as_millis(),
            input.duration_budget.as_millis()
        ));
    }
    if input.event_count > input.event_budget {
        diagnostics.push(format!(
            "runtime stability event budget exceeded: actual={} budget={}",
            input.event_count, input.event_budget
        ));
    }
    if input.span_count > input.span_budget {
        diagnostics.push(format!(
            "runtime stability span budget exceeded: actual={} budget={}",
            input.span_count, input.span_budget
        ));
    }
    if input.diagnostic_count > 0 {
        diagnostics.push(format!(
            "runtime stability diagnostics present: count={}",
            input.diagnostic_count
        ));
    }

    diagnostics
}
