//! Typed `scenario_benchmark` receipts shared by crate and workspace gates.

use std::path::PathBuf;

use serde::Serialize;

/// Status for a crate-local scenario performance optimization frontier.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustScenarioPerformanceOptimizationStatus {
    Healthy,
    ActionRequired,
}

/// Priority for one scenario performance optimization finding.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustScenarioPerformanceOptimizationPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// First-batch anomaly class for scenario performance baselines.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustScenarioPerformanceOptimizationFindingKind {
    PlaceholderBaseline,
    LooseTotalBudget,
    LooseMemoryBudget,
    MissingPhaseTiming,
}

/// Workspace-wide scenario benchmark gate status.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustScenarioWorkspaceBenchmarkStatus {
    Pass,
    Fail,
}

/// Actionable workspace-wide improvement class.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustScenarioWorkspaceBenchmarkImprovementKind {
    InvalidScenarioBenchmarkSchema,
    ScenarioPerformanceOptimization,
}

/// Crate name that owns a scenario performance optimization receipt.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RustScenarioPerformanceCrateName(String);

impl RustScenarioPerformanceCrateName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Stable scenario id for an optimization finding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RustScenarioPerformanceScenarioId(String);

impl RustScenarioPerformanceScenarioId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Relative score used to order first-batch optimization findings.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RustScenarioPerformanceOptimizationScore(u64);

impl RustScenarioPerformanceOptimizationScore {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Scenario performance duration in milliseconds.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RustScenarioPerformanceDurationMs(u64);

impl RustScenarioPerformanceDurationMs {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Scenario performance memory size in bytes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RustScenarioPerformanceMemoryBytes(u64);

impl RustScenarioPerformanceMemoryBytes {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// One actionable scenario performance optimization finding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustScenarioPerformanceOptimizationFinding {
    pub kind: RustScenarioPerformanceOptimizationFindingKind,
    pub priority: RustScenarioPerformanceOptimizationPriority,
    pub scenario_id: RustScenarioPerformanceScenarioId,
    pub optimization_score: RustScenarioPerformanceOptimizationScore,
    pub problem: String,
    pub next_action: String,
    pub observed_total_ms: RustScenarioPerformanceDurationMs,
    pub target_total_ms: RustScenarioPerformanceDurationMs,
    pub max_total_ms: RustScenarioPerformanceDurationMs,
    pub regression_budget_ms: RustScenarioPerformanceDurationMs,
    pub observed_memory_bytes: RustScenarioPerformanceMemoryBytes,
    pub memory_budget_bytes: RustScenarioPerformanceMemoryBytes,
}

/// Optimization frontier compiled from a crate-local scenario performance receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustScenarioPerformanceOptimizationReceipt {
    pub schema_id: String,
    pub schema_version: String,
    pub crate_name: RustScenarioPerformanceCrateName,
    pub scenario_id: RustScenarioPerformanceScenarioId,
    pub status: RustScenarioPerformanceOptimizationStatus,
    pub findings: Vec<RustScenarioPerformanceOptimizationFinding>,
}

/// One crate-local result inside the workspace scenario benchmark gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustScenarioWorkspaceBenchmarkCrateReceipt {
    pub crate_name: RustScenarioPerformanceCrateName,
    pub crate_root: PathBuf,
    pub status: RustScenarioWorkspaceBenchmarkStatus,
    pub required_scenario_count: usize,
    pub violation_count: usize,
    pub optimization_status: Option<RustScenarioPerformanceOptimizationStatus>,
    pub optimization_finding_count: usize,
    pub error: Option<String>,
    pub improvements: Vec<RustScenarioWorkspaceBenchmarkImprovementSuggestion>,
}

/// One actionable suggestion compiled from benchmark validation/reflection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustScenarioWorkspaceBenchmarkImprovementSuggestion {
    pub kind: RustScenarioWorkspaceBenchmarkImprovementKind,
    pub priority: RustScenarioPerformanceOptimizationPriority,
    pub crate_name: RustScenarioPerformanceCrateName,
    pub crate_root: PathBuf,
    pub scenario_id: Option<RustScenarioPerformanceScenarioId>,
    pub problem: String,
    pub next_action: String,
    pub verification_command: String,
    pub evidence: Vec<String>,
}

/// Workspace-wide scenario benchmark gate receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustScenarioWorkspaceBenchmarkGateReceipt {
    pub schema_id: String,
    pub schema_version: String,
    pub workspace_root: PathBuf,
    pub status: RustScenarioWorkspaceBenchmarkStatus,
    pub crate_count: usize,
    pub passed_crate_count: usize,
    pub failed_crate_count: usize,
    pub improvement_count: usize,
    pub crates: Vec<RustScenarioWorkspaceBenchmarkCrateReceipt>,
    pub improvements: Vec<RustScenarioWorkspaceBenchmarkImprovementSuggestion>,
}
