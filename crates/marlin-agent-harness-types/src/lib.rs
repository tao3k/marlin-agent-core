//! Agent-harness typed contracts shared by agent harness and test-support crates.
//!
//! These DTOs describe evidence observed while testing this project's agent
//! system: agent loop behavior, runtime visibility, sub-agent/session behavior,
//! hook replay, graph-policy receipts, and deterministic no-LLM scenarios.
//!
//! They are not Rust project harness contracts. Rust package/build quality
//! evidence belongs under `build-support/marlin-rust-project-harness-policy`
//! and should use `RustProjectHarness*` names.

mod evidence;
mod scenario;

pub use evidence::{
    AGENT_HARNESS_GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_BASELINE,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_KEYS,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD,
    AGENT_HARNESS_STABILITY_EVIDENCE_ARTIFACT, AGENT_HARNESS_STABILITY_EVIDENCE_COMMAND,
    AGENT_HARNESS_STABILITY_EVIDENCE_DETERMINISM,
    AGENT_HARNESS_STABILITY_EVIDENCE_ITERATION_WINDOW,
    AGENT_HARNESS_STABILITY_EVIDENCE_KEYS,
    AGENT_HARNESS_STABILITY_EVIDENCE_LATENCY_DISTRIBUTION,
    AGENT_HARNESS_STABILITY_EVIDENCE_RESOURCE_DELTA,
    AGENT_HARNESS_STABILITY_EVIDENCE_STATE_GROWTH, AgentHarnessEvidence,
    AgentHarnessEvidenceKind, AgentHarnessPerformanceEvidence, AgentHarnessStabilityEvidence,
    agent_harness_graph_policy_proposal_visibility_evidence,
};
pub use scenario::{
    AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID, AgentHarnessScenario,
    AgentHarnessScenarioContract,
};
