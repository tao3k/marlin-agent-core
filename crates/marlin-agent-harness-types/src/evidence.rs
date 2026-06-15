//! Agent-harness evidence DTOs shared by agent harness and test-support.

use marlin_agent_protocol::AgentTraceSpanRecord;
use serde::{Deserialize, Serialize};

/// Detail key for the benchmark command used by agent-harness performance evidence.
pub const AGENT_HARNESS_PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND: &str = "benchmark_command";
/// Detail key for the baseline used by agent-harness performance evidence.
pub const AGENT_HARNESS_PERFORMANCE_EVIDENCE_BASELINE: &str = "baseline";
/// Detail key for the accepted regression threshold.
pub const AGENT_HARNESS_PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD: &str = "regression_threshold";
/// Detail key for latency or throughput observations.
pub const AGENT_HARNESS_PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT: &str = "latency_or_throughput";
/// Detail key for allocation profile observations.
pub const AGENT_HARNESS_PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE: &str = "allocation_profile";
/// Detail key for the durable performance profile artifact.
pub const AGENT_HARNESS_PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT: &str = "profile_artifact";
/// Ordered detail keys expected in agent-harness performance evidence.
pub const AGENT_HARNESS_PERFORMANCE_EVIDENCE_KEYS: [&str; 6] = [
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_BASELINE,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT,
];

/// Detail key for the long-running command used by agent-harness stability evidence.
pub const AGENT_HARNESS_STABILITY_EVIDENCE_COMMAND: &str = "stability_command";
/// Detail key for the iteration count or duration window.
pub const AGENT_HARNESS_STABILITY_EVIDENCE_ITERATION_WINDOW: &str = "iteration_window";
/// Detail key for observed latency distribution.
pub const AGENT_HARNESS_STABILITY_EVIDENCE_LATENCY_DISTRIBUTION: &str = "latency_distribution";
/// Detail key for resource growth observations.
pub const AGENT_HARNESS_STABILITY_EVIDENCE_RESOURCE_DELTA: &str = "resource_delta";
/// Detail key for cache, queue, database, or artifact growth.
pub const AGENT_HARNESS_STABILITY_EVIDENCE_STATE_GROWTH: &str = "state_growth";
/// Detail key for repeated-run determinism evidence.
pub const AGENT_HARNESS_STABILITY_EVIDENCE_DETERMINISM: &str = "determinism";
/// Detail key for the durable stability report artifact.
pub const AGENT_HARNESS_STABILITY_EVIDENCE_ARTIFACT: &str = "stability_artifact";
/// Ordered detail keys expected in agent-harness stability evidence.
pub const AGENT_HARNESS_STABILITY_EVIDENCE_KEYS: [&str; 7] = [
    AGENT_HARNESS_STABILITY_EVIDENCE_COMMAND,
    AGENT_HARNESS_STABILITY_EVIDENCE_ITERATION_WINDOW,
    AGENT_HARNESS_STABILITY_EVIDENCE_LATENCY_DISTRIBUTION,
    AGENT_HARNESS_STABILITY_EVIDENCE_RESOURCE_DELTA,
    AGENT_HARNESS_STABILITY_EVIDENCE_STATE_GROWTH,
    AGENT_HARNESS_STABILITY_EVIDENCE_DETERMINISM,
    AGENT_HARNESS_STABILITY_EVIDENCE_ARTIFACT,
];

/// Subject prefix for agent-harness visibility evidence projected from graph policy proposal spans.
pub const AGENT_HARNESS_GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX: &str =
    "graph-policy-proposal";

/// Evidence category captured by this project's agent harness and test-support layers.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum AgentHarnessEvidenceKind {
    Content,
    Safety,
    Budget,
    Registry,
    Workflow,
    RunLog,
    Provider,
    Tool,
    SubAgent,
    Runtime,
    Visibility,
    Performance,
    Stability,
}

/// Typed fact captured while validating this project's agent-harness gates.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentHarnessEvidence {
    /// Category of harness evidence represented by this fact.
    pub kind: AgentHarnessEvidenceKind,
    /// Stable subject this evidence fact describes.
    pub subject: String,
    /// Whether the evidence was observed.
    pub present: bool,
    /// Optional human-readable detail or receipt summary.
    pub detail: Option<String>,
}

impl AgentHarnessEvidence {
    /// Creates a present agent-harness evidence fact.
    pub fn present(kind: AgentHarnessEvidenceKind, subject: impl Into<String>) -> Self {
        Self {
            kind,
            subject: subject.into(),
            present: true,
            detail: None,
        }
    }

    /// Creates a missing agent-harness evidence fact.
    pub fn missing(kind: AgentHarnessEvidenceKind, subject: impl Into<String>) -> Self {
        Self {
            kind,
            subject: subject.into(),
            present: false,
            detail: None,
        }
    }

    /// Attaches detail to this agent-harness evidence fact.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Named performance evidence input used to produce an `AgentHarnessEvidence` fact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentHarnessPerformanceEvidence {
    /// Subject being measured.
    pub subject: String,
    /// Command used to run the benchmark or performance gate.
    pub benchmark_command: String,
    /// Baseline used for comparison.
    pub baseline: String,
    /// Maximum accepted regression threshold.
    pub regression_threshold: String,
    /// Latency or throughput observation.
    pub latency_or_throughput: String,
    /// Allocation profile observation.
    pub allocation_profile: String,
    /// Path or URI for the profile artifact.
    pub profile_artifact: String,
}

impl AgentHarnessPerformanceEvidence {
    /// Converts named performance evidence into a generic agent-harness evidence fact.
    pub fn into_agent_harness_evidence(self) -> AgentHarnessEvidence {
        let detail = format!(
            "{benchmark_command}={} {baseline}={} {regression_threshold}={} {latency_or_throughput}={} {allocation_profile}={} {profile_artifact}={}",
            self.benchmark_command,
            self.baseline,
            self.regression_threshold,
            self.latency_or_throughput,
            self.allocation_profile,
            self.profile_artifact,
            benchmark_command = AGENT_HARNESS_PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND,
            baseline = AGENT_HARNESS_PERFORMANCE_EVIDENCE_BASELINE,
            regression_threshold = AGENT_HARNESS_PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD,
            latency_or_throughput = AGENT_HARNESS_PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT,
            allocation_profile = AGENT_HARNESS_PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE,
            profile_artifact = AGENT_HARNESS_PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT,
        );

        AgentHarnessEvidence::present(AgentHarnessEvidenceKind::Performance, self.subject)
            .with_detail(detail)
    }
}

impl From<AgentHarnessPerformanceEvidence> for AgentHarnessEvidence {
    fn from(evidence: AgentHarnessPerformanceEvidence) -> Self {
        evidence.into_agent_harness_evidence()
    }
}

/// Named stability evidence input used to produce an `AgentHarnessEvidence` fact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentHarnessStabilityEvidence {
    /// Subject being measured.
    pub subject: String,
    /// Command used to run the long-running stability gate.
    pub stability_command: String,
    /// Iteration count or duration window covered by the run.
    pub iteration_window: String,
    /// Latency distribution observed during the stability run.
    pub latency_distribution: String,
    /// Resource growth observation such as RSS, FDs, or threads.
    pub resource_delta: String,
    /// Cache, queue, database, or artifact growth observation.
    pub state_growth: String,
    /// Repeated-run determinism or bounded nondeterminism evidence.
    pub determinism: String,
    /// Durable stability report artifact path or URI.
    pub stability_artifact: String,
}

impl AgentHarnessStabilityEvidence {
    /// Converts named stability evidence into a generic agent-harness evidence fact.
    pub fn into_agent_harness_evidence(self) -> AgentHarnessEvidence {
        let detail = format!(
            "{stability_command}={} {iteration_window}={} {latency_distribution}={} {resource_delta}={} {state_growth}={} {determinism}={} {stability_artifact}={}",
            self.stability_command,
            self.iteration_window,
            self.latency_distribution,
            self.resource_delta,
            self.state_growth,
            self.determinism,
            self.stability_artifact,
            stability_command = AGENT_HARNESS_STABILITY_EVIDENCE_COMMAND,
            iteration_window = AGENT_HARNESS_STABILITY_EVIDENCE_ITERATION_WINDOW,
            latency_distribution = AGENT_HARNESS_STABILITY_EVIDENCE_LATENCY_DISTRIBUTION,
            resource_delta = AGENT_HARNESS_STABILITY_EVIDENCE_RESOURCE_DELTA,
            state_growth = AGENT_HARNESS_STABILITY_EVIDENCE_STATE_GROWTH,
            determinism = AGENT_HARNESS_STABILITY_EVIDENCE_DETERMINISM,
            stability_artifact = AGENT_HARNESS_STABILITY_EVIDENCE_ARTIFACT,
        );

        AgentHarnessEvidence::present(AgentHarnessEvidenceKind::Stability, self.subject)
            .with_detail(detail)
    }
}

impl From<AgentHarnessStabilityEvidence> for AgentHarnessEvidence {
    fn from(evidence: AgentHarnessStabilityEvidence) -> Self {
        evidence.into_agent_harness_evidence()
    }
}

/// Projects a graph policy proposal span into agent-harness visibility evidence.
pub fn agent_harness_graph_policy_proposal_visibility_evidence(
    span: &AgentTraceSpanRecord,
) -> Option<AgentHarnessEvidence> {
    if !span.is_graph_policy_proposal() {
        return None;
    }

    let schema_id = span.fields.get("schema_id")?;
    let strategy_id = span.fields.get("strategy_id")?;
    let status = span.fields.get("status")?;
    let diagnostic_count = span.fields.get("diagnostic_count")?;
    let subject =
        format!("{AGENT_HARNESS_GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX}:{strategy_id}");
    let mut detail = format!(
        "schema_id={schema_id} strategy_id={strategy_id} status={status} diagnostic_count={diagnostic_count}",
    );
    if let Some(selected_graph_id) = span.fields.get("selected_graph_id") {
        detail.push_str(" selected_graph_id=");
        detail.push_str(selected_graph_id);
    }

    Some(
        AgentHarnessEvidence::present(AgentHarnessEvidenceKind::Visibility, subject)
            .with_detail(detail),
    )
}
