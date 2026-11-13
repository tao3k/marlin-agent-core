// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed `Gerbil` output contracts consumed by Rust runtimes.

mod loop_graph;
mod workspace_policy;

pub use loop_graph::{
    CompiledLoopGraph, LoopEdgeSpec, LoopGraphCompileError, LoopGraphCompileLimits,
    LoopGraphExecutionFrontier, LoopGraphExecutionPlan, LoopGraphValidationError, LoopNodeSpec,
};
pub use workspace_policy::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};
