//! Typed `Gerbil` output contracts consumed by Rust runtimes.

mod loop_graph;
mod workspace_policy;

pub use loop_graph::{
    CompiledLoopGraph, LoopEdgeSpec, LoopGraphCompileError, LoopGraphCompileLimits,
    LoopGraphExecutionFrontier, LoopGraphExecutionPlan, LoopGraphValidationError, LoopNodeSpec,
};
pub use workspace_policy::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};
