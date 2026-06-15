//! Agent-system harness for scenario execution, replay fixtures, and evidence assertions.
//!
//! This crate belongs to the agent harness boundary. It validates agent loop
//! behavior, sub-agent/session isolation, hook replay, runtime visibility,
//! graph-policy receipts, and no-LLM deterministic scenario execution.
//!
//! It is intentionally separate from the Rust project harness in
//! `build-support/marlin-rust-project-harness-policy`, which owns package
//! engineering quality gates, build-script receipts, CI evidence, and Rust
//! project evidence graphs.

mod assertion;
mod evidence;
mod evidence_graph;
mod fakes;
mod graph;
mod release_visibility;
mod runtime;
mod trace;

pub use assertion::{HarnessAssertionError, assert_evidence_kinds};
pub use evidence::{AgentHarness, AgentHarnessReport};
pub use evidence_graph::{
    HARNESS_EVIDENCE_GRAPH_SCHEMA_ID, HarnessEvidenceGraph, HarnessEvidenceGraphEdge,
    HarnessEvidenceGraphEdgeKind, HarnessEvidenceGraphNode, HarnessEvidenceGraphNodeKind,
    HarnessEvidenceGraphSummary,
};
pub use fakes::{
    StaticHookRuntime, StaticProviderRuntime, StaticSubAgentRuntime, StaticToolRuntime,
};
pub use graph::HarnessGraphBuilder;
pub use marlin_agent_harness_types::{
    GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX,
    HARNESS_PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE, HARNESS_PERFORMANCE_EVIDENCE_BASELINE,
    HARNESS_PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND, HARNESS_PERFORMANCE_EVIDENCE_KEYS,
    HARNESS_PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT,
    HARNESS_PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT,
    HARNESS_PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD, HARNESS_SCENARIO_CONTRACT_SCHEMA_ID,
    HARNESS_STABILITY_EVIDENCE_ARTIFACT, HARNESS_STABILITY_EVIDENCE_COMMAND,
    HARNESS_STABILITY_EVIDENCE_DETERMINISM, HARNESS_STABILITY_EVIDENCE_ITERATION_WINDOW,
    HARNESS_STABILITY_EVIDENCE_KEYS, HARNESS_STABILITY_EVIDENCE_LATENCY_DISTRIBUTION,
    HARNESS_STABILITY_EVIDENCE_RESOURCE_DELTA, HARNESS_STABILITY_EVIDENCE_STATE_GROWTH,
    HarnessEvidence, HarnessEvidenceKind, HarnessPerformanceEvidence, HarnessScenario,
    HarnessScenarioContract, HarnessStabilityEvidence, graph_policy_proposal_visibility_evidence,
};
pub use release_visibility::{
    ReleaseGateExecutionReceipt, ReleaseGateExecutionStatus,
    native_abi_readiness_release_gate_execution_receipt, release_gate_execution_receipt,
    release_gate_visibility_evidence, release_topology_execution_receipts,
    release_topology_visibility_evidence, release_visibility_evidence,
};
pub use runtime::{
    HarnessExecutionReport, HarnessRuntime, runtime_environment_visibility_evidence,
    working_copy_isolation_visibility_evidence,
};
pub use trace::TraceRecorder;
