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
mod gerbil_continuation;
mod graph;
mod intent_case_artifact;
mod intent_case_artifact_error;
mod intent_case_artifact_manifest;
mod intent_case_artifact_model_events;
mod intent_case_artifact_receipt_header;
mod intent_case_artifact_replay;
mod intent_case_artifact_runtime_repair;
mod intent_case_artifact_test_receipts;
mod intent_case_observed_span;
mod release_visibility;
mod runtime;
mod trace;

pub use assertion::{AgentHarnessAssertionError, assert_agent_harness_evidence_kinds};
pub use evidence::{AgentHarness, AgentHarnessReport};
pub use evidence_graph::{
    AGENT_HARNESS_EVIDENCE_GRAPH_SCHEMA_ID, AgentHarnessEvidenceGraph,
    AgentHarnessEvidenceGraphEdge, AgentHarnessEvidenceGraphEdgeKind,
    AgentHarnessEvidenceGraphNode, AgentHarnessEvidenceGraphNodeKind,
    AgentHarnessEvidenceGraphSummary,
};
pub use fakes::{
    StaticHookRuntime, StaticProviderRuntime, StaticSubAgentRuntime, StaticToolRuntime,
};
pub use gerbil_continuation::{
    AgentHarnessGerbilLoopContinuationError, AgentHarnessGerbilLoopContinuationPlanner,
    AgentHarnessGerbilLoopContinuationProjector,
};
pub use graph::AgentHarnessGraphBuilder;
pub use intent_case_artifact::{
    GerbilScriptedIntentCaseArtifactBundleRequest, IntentCaseArtifactBundleMaterializationReceipt,
    IntentCaseMaterializedArtifactReceipt, materialize_gerbil_scripted_intent_case_artifact_bundle,
};
pub use intent_case_artifact_error::IntentCaseArtifactBundleMaterializationError;
pub use intent_case_observed_span::IntentCaseObservedSpanSource;
pub use marlin_agent_harness_types::{
    AGENT_HARNESS_GRAPH_POLICY_PROPOSAL_VISIBILITY_SUBJECT_PREFIX,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_ALLOCATION_PROFILE,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_BASELINE,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_BENCHMARK_COMMAND, AGENT_HARNESS_PERFORMANCE_EVIDENCE_KEYS,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_LATENCY_OR_THROUGHPUT,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_PROFILE_ARTIFACT,
    AGENT_HARNESS_PERFORMANCE_EVIDENCE_REGRESSION_THRESHOLD,
    AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID, AGENT_HARNESS_STABILITY_EVIDENCE_ARTIFACT,
    AGENT_HARNESS_STABILITY_EVIDENCE_COMMAND, AGENT_HARNESS_STABILITY_EVIDENCE_DETERMINISM,
    AGENT_HARNESS_STABILITY_EVIDENCE_ITERATION_WINDOW, AGENT_HARNESS_STABILITY_EVIDENCE_KEYS,
    AGENT_HARNESS_STABILITY_EVIDENCE_LATENCY_DISTRIBUTION,
    AGENT_HARNESS_STABILITY_EVIDENCE_RESOURCE_DELTA, AGENT_HARNESS_STABILITY_EVIDENCE_STATE_GROWTH,
    AgentHarnessEvidence, AgentHarnessEvidenceKind, AgentHarnessPerformanceEvidence,
    AgentHarnessScenario, AgentHarnessScenarioContract, AgentHarnessStabilityEvidence,
    INTENT_CASE_ARTIFACT_COMPLETENESS_RECEIPT_SCHEMA_ID, INTENT_CASE_ARTIFACT_MANIFEST_SCHEMA_ID,
    INTENT_CASE_RUN_RECEIPT_SCHEMA_ID, IntentCaseArtifactCompletenessReceipt,
    IntentCaseArtifactCompletenessStatus, IntentCaseArtifactId, IntentCaseArtifactKind,
    IntentCaseArtifactManifest, IntentCaseArtifactManifestRequest, IntentCaseArtifactRef,
    IntentCaseCorrelationKey, IntentCaseId, IntentCaseLoopProgramId, IntentCasePolicyDigest,
    IntentCaseRunId, IntentCaseRunReceipt, IntentCaseRunStatus, IntentCaseRuntimeOwner,
    IntentCaseSpanName, IntentCaseTraceAction, IntentCaseTraceEntry, IntentCaseTraceEntryId,
    IntentCaseTraceEntryRequest, IntentCaseTraceEvent, IntentCaseTraceIndex,
    IntentCaseTransitionId, agent_harness_graph_policy_proposal_visibility_evidence,
};
pub use release_visibility::{
    ReleaseGateExecutionReceipt, ReleaseGateExecutionStatus,
    native_abi_readiness_release_gate_execution_receipt, release_gate_execution_receipt,
    release_gate_visibility_evidence, release_topology_execution_receipts,
    release_topology_visibility_evidence, release_visibility_evidence,
};
pub use runtime::{
    AgentHarnessExecutionReport, AgentHarnessExecutionSummary,
    AgentHarnessGraphLoopExecutionReport, AgentHarnessGraphLoopExecutionSummary,
    AgentHarnessRuntime, agent_harness_runtime_environment_visibility_evidence,
    agent_harness_working_copy_isolation_visibility_evidence,
};
pub use trace::TraceRecorder;
