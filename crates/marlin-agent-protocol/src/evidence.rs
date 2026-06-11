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
