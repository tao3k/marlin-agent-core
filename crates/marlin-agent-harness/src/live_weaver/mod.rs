// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Reusable Live Weaver LLM Evaluation & Scheme-to-Rust Policy Harness.
//!
//! Decoupled completely from TUI presentation (`marlin-agent-tui`). Drives real
//! multi-turn agent scenarios declaratively authored in Gerbil Scheme
//! (`scheme/agent-harness/`):
//! - Typed TOML model route configuration -> [`marlin_agent_stream::WeaverModelGateway`]
//! - Single thread-affine Gerbil Scheme VM bridge (`HarnessSchemeBridge`) projecting
//!   `ToolingCatalogValue`, `HelmIndexPolicy`, declarative POO `HarnessSuite`s,
//!   Search DSL, and Bi-Temporal Causality trajectory verification
//! - Real fail-closed Cedar Governance enforcement (`marlin_agent_kernel::governance_cedar`)
//!   denying write/shell capabilities on read-only Search/Helm/ASP/Dusa scenarios
//! - Full OTel-compatible `TraceRecorder` telemetry, causal `TrajectoryEvent` DAG,
//!   Bi-Temporal Causality proof digest, `AgentHarnessEvidenceGraph`, and Markdown reports.

pub mod aitia_grpc;
mod artifacts;
pub mod config;
pub mod governance;
pub mod report;
pub mod runner;
pub mod scheme_bridge;

pub use config::{
    LiveEvalBudget, LiveWeaverModelProfile, LiveWeaverRoute, LiveWeaverTomlConfig,
    LiveWeaverWireProtocol,
};
pub use governance::{
    CompiledHarnessGovernanceGate, GovernanceDecisionReceipt, ProjectedCedarPolicySpec,
    ProjectedHarnessGovernance, format_actionable_denial_message,
};
pub use report::{
    DetectedGap, ParsedSearchReceipt, QueryTierKind, classify_tool_call, detect_scenario_gaps,
    parse_search_receipt, render_live_eval_markdown_report, score_ground_truth,
};
pub use runner::{
    LiveScenarioReport, LiveWeaverEvalSession, ReflectionItem, ScenarioReflection,
    ToolCallTelemetry, TrajectoryEvent, TrajectoryEventKind, TurnTelemetry,
};
pub use scheme_bridge::{
    HarnessSchemeBridge, ProjectedCheckpoint, ProjectedHarnessScenario, ProjectedHarnessSuite,
    TemporalCausalityVerdict,
};
