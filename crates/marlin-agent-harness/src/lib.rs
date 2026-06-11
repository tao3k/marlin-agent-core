//! Agent-system harness for scenario execution, replay fixtures, and evidence assertions.

mod assertion;
mod evidence;
mod fakes;
mod graph;
mod runtime;
mod trace;

pub use assertion::{HarnessAssertionError, assert_evidence_kinds};
pub use evidence::{AgentHarness, AgentHarnessReport};
pub use fakes::{
    StaticHookRuntime, StaticProviderRuntime, StaticSubAgentRuntime, StaticToolRuntime,
};
pub use graph::HarnessGraphBuilder;
pub use runtime::{HarnessExecutionReport, HarnessRuntime};
pub use trace::TraceRecorder;
