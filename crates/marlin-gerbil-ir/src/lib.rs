//! Typed `Gerbil` output contracts consumed by Rust runtimes.

mod loop_graph;
mod workspace_policy;

pub use loop_graph::{CompiledLoopGraph, LoopEdgeSpec, LoopNodeSpec};
pub use workspace_policy::{
    GerbilWorkspaceContractFacts, MemoryDispatchPolicySpec, ReleaseGateSpec, ReleaseTopologySpec,
    ReleaseVisibilitySpec, WorkspacePatchIntentSpec, WorkspaceSchemaSpec,
    WorkspaceValidationPolicySpec, WorkspaceViewPolicySpec,
};
