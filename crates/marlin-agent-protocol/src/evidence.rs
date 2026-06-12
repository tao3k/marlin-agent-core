//! Typed evidence facts captured by the agent harness.

use serde::{Deserialize, Serialize};

/// Detail key for the benchmark command used to produce performance evidence.
pub const PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND: &str = "benchmark_command";

/// Detail key for the baseline that a performance run compares against.
pub const PERFORMANCE_EVIDENCE_BASELINE: &str = "baseline";

/// Detail key for the accepted performance regression threshold.
pub const PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD: &str = "regression_threshold";

/// Detail key for latency or throughput observations.
pub const PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT: &str = "latency_or_throughput";

/// Detail key for allocation profile observations.
pub const PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE: &str = "allocation_profile";

/// Detail key for the profile artifact path or URI.
pub const PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT: &str = "profile_artifact";

/// Ordered performance detail keys required by harness performance evidence.
pub const PERFORMANCE_EVIDENCE_KEYS: [&str; 6] = [
    PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND,
    PERFORMANCE_EVIDENCE_BASELINE,
    PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD,
    PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT,
    PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE,
    PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT,
];

/// Detail key for the long-running command used to produce stability evidence.
pub const STABILITY_EVIDENCE_COMMAND: &str = "stability_command";

/// Detail key for the iteration or duration window covered by stability evidence.
pub const STABILITY_EVIDENCE_ITERATION_WINDOW: &str = "iteration_window";

/// Detail key for latency distribution observed during a stability run.
pub const STABILITY_EVIDENCE_LATENCY_DISTRIBUTION: &str = "latency_distribution";

/// Detail key for resource-growth observations such as RSS, FDs, or threads.
pub const STABILITY_EVIDENCE_RESOURCE_DELTA: &str = "resource_delta";

/// Detail key for cache, queue, database, or artifact growth observations.
pub const STABILITY_EVIDENCE_STATE_GROWTH: &str = "state_growth";

/// Detail key for repeated-run determinism or bounded nondeterminism evidence.
pub const STABILITY_EVIDENCE_DETERMINISM: &str = "determinism";

/// Detail key for the durable stability report artifact path or URI.
pub const STABILITY_EVIDENCE_ARTIFACT: &str = "stability_artifact";

/// Ordered stability detail keys required by harness stability evidence.
pub const STABILITY_EVIDENCE_KEYS: [&str; 7] = [
    STABILITY_EVIDENCE_COMMAND,
    STABILITY_EVIDENCE_ITERATION_WINDOW,
    STABILITY_EVIDENCE_LATENCY_DISTRIBUTION,
    STABILITY_EVIDENCE_RESOURCE_DELTA,
    STABILITY_EVIDENCE_STATE_GROWTH,
    STABILITY_EVIDENCE_DETERMINISM,
    STABILITY_EVIDENCE_ARTIFACT,
];

/// Evidence category captured by the agent harness.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum LoopEvidenceKind {
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

/// Typed fact captured while validating an agent loop.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopEvidence {
    pub kind: LoopEvidenceKind,
    pub subject: String,
    pub present: bool,
    pub detail: Option<String>,
}

impl LoopEvidence {
    pub fn present(kind: LoopEvidenceKind, subject: impl Into<String>) -> Self {
        Self {
            kind,
            subject: subject.into(),
            present: true,
            detail: None,
        }
    }

    pub fn missing(kind: LoopEvidenceKind, subject: impl Into<String>) -> Self {
        Self {
            kind,
            subject: subject.into(),
            present: false,
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Named performance evidence input used to produce a `LoopEvidence` fact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopPerformanceEvidence {
    /// Subject being measured, usually an owner path or runtime surface.
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

impl LoopPerformanceEvidence {
    /// Convert named performance evidence into a generic loop evidence fact.
    pub fn into_loop_evidence(self) -> LoopEvidence {
        let detail = format!(
            "{benchmark_command}={} {baseline}={} {regression_threshold}={} {latency_or_throughput}={} {allocation_profile}={} {profile_artifact}={}",
            self.benchmark_command,
            self.baseline,
            self.regression_threshold,
            self.latency_or_throughput,
            self.allocation_profile,
            self.profile_artifact,
            benchmark_command = PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND,
            baseline = PERFORMANCE_EVIDENCE_BASELINE,
            regression_threshold = PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD,
            latency_or_throughput = PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT,
            allocation_profile = PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE,
            profile_artifact = PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT,
        );

        LoopEvidence::present(LoopEvidenceKind::Performance, self.subject).with_detail(detail)
    }
}

impl From<LoopPerformanceEvidence> for LoopEvidence {
    fn from(evidence: LoopPerformanceEvidence) -> Self {
        evidence.into_loop_evidence()
    }
}

/// Named stability evidence input used to produce a `LoopEvidence` fact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopStabilityEvidence {
    /// Subject being measured, usually an owner path or runtime surface.
    pub subject: String,
    /// Command used to run the long-running stability gate.
    pub stability_command: String,
    /// Iteration count or duration window covered by the run.
    pub iteration_window: String,
    /// Latency distribution observed during the stability run.
    pub latency_distribution: String,
    /// Resource growth observation, such as RSS, FDs, or threads.
    pub resource_delta: String,
    /// Cache, queue, database, or artifact growth observation.
    pub state_growth: String,
    /// Repeated-run determinism or bounded nondeterminism evidence.
    pub determinism: String,
    /// Durable stability report artifact path or URI.
    pub stability_artifact: String,
}

impl LoopStabilityEvidence {
    /// Convert named stability evidence into a generic loop evidence fact.
    pub fn into_loop_evidence(self) -> LoopEvidence {
        let detail = format!(
            "{stability_command}={} {iteration_window}={} {latency_distribution}={} {resource_delta}={} {state_growth}={} {determinism}={} {stability_artifact}={}",
            self.stability_command,
            self.iteration_window,
            self.latency_distribution,
            self.resource_delta,
            self.state_growth,
            self.determinism,
            self.stability_artifact,
            stability_command = STABILITY_EVIDENCE_COMMAND,
            iteration_window = STABILITY_EVIDENCE_ITERATION_WINDOW,
            latency_distribution = STABILITY_EVIDENCE_LATENCY_DISTRIBUTION,
            resource_delta = STABILITY_EVIDENCE_RESOURCE_DELTA,
            state_growth = STABILITY_EVIDENCE_STATE_GROWTH,
            determinism = STABILITY_EVIDENCE_DETERMINISM,
            stability_artifact = STABILITY_EVIDENCE_ARTIFACT,
        );

        LoopEvidence::present(LoopEvidenceKind::Stability, self.subject).with_detail(detail)
    }
}

impl From<LoopStabilityEvidence> for LoopEvidence {
    fn from(evidence: LoopStabilityEvidence) -> Self {
        evidence.into_loop_evidence()
    }
}
