//! Agent-system harness for scenario execution, replay fixtures, and evidence assertions.

mod assertion;
mod evidence;
mod fakes;
mod graph;
mod release_visibility;
mod runtime;
mod trace;

pub use assertion::{HarnessAssertionError, assert_evidence_kinds};
pub use evidence::{AgentHarness, AgentHarnessReport};
pub use fakes::{
    StaticHookRuntime, StaticProviderRuntime, StaticSubAgentRuntime, StaticToolRuntime,
};
pub use graph::HarnessGraphBuilder;
pub use release_visibility::{
    release_gate_visibility_evidence, release_topology_visibility_evidence,
    release_visibility_evidence,
};
pub use runtime::{
    HarnessExecutionReport, HarnessRuntime, runtime_environment_visibility_evidence,
};
pub use trace::TraceRecorder;
