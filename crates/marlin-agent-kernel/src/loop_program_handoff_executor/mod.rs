// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Public interface for `LoopProgram` runtime handoff execution.

mod hybrid;
mod policy_gated;
mod runtime;
mod tool_process;

pub use hybrid::HybridLoopProgramRuntimeHandoffExecutor;
pub use policy_gated::PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor;
pub use runtime::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, DeferredLoopProgramRuntimeHandoffHandler,
    DenylistedLoopProgramToolDispatchHandler, LoopProgramAgentFlowRuntimeHandoffRequest,
    LoopProgramMemoryProjectionReceipt, LoopProgramRuntimeHandoffExecution,
    LoopProgramRuntimeHandoffExecutionReceipt, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffExecutionStatus, LoopProgramRuntimeHandoffExecutor,
    LoopProgramRuntimeHandoffHandler, LoopProgramRuntimeHandoffRouter,
    LoopProgramRuntimeHandoffRouterHandlers, LoopProgramRuntimeOwner, RetryBudgetToolHandler,
    StaticLoopProgramRuntimeHandoffHandler,
};
pub use tool_process::{
    LoopProgramToolProcessProgram, LoopProgramToolProcessProjectionReceipt,
    LoopProgramToolProcessSpawnReceipt, LoopProgramToolProcessSpawnRequest,
    spawn_loop_program_tool_process,
};
