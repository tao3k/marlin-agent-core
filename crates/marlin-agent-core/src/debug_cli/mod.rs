//! Harness/debug command surface behind the `marlin` binary.

mod args;
mod catalog;
mod driver;
mod executor;
mod gerbil_cmd;
mod graph;
mod io;
mod loop_cmd;
mod process_command;
mod receipts;
mod smoke_cmd;
mod state_cmd;
mod state_home;

pub use driver::{MarlinCliResult, run_marlin_cli, run_marlin_cli_from_args};
pub(in crate::debug_cli) use driver::{
    gerbil_usage, graph_usage, loop_usage, smoke_usage, state_usage, usage,
};
pub use receipts::{
    GraphQueryOutput, GraphQuerySummary, LoopEventQuerySummary, LoopGovernanceReceipt,
    LoopGovernanceSandboxReceipt, LoopGovernanceSessionReceipt, LoopGovernanceStateReceipt,
    LoopGovernanceVerifierDecision, LoopGovernanceVerifierReceipt, LoopInspectReceipt,
    LoopProgramDerivedSessionPolicyStatusReceipt, LoopProgramRunReceipt,
    LoopProgramRunStatusReceipt, LoopProgramRuntimeHandoffStatusReceipt,
    LoopProgramRuntimeHandoffSummary, LoopProgramRuntimeReplaySummary,
    LoopProgramRuntimeSideEffectStatusReceipt, LoopQuerySummary, LoopReplayReceipt, LoopRunReceipt,
    ProjectRuntimeQuerySummary, SmokeLlmMode, SmokeRuntimeModelRouteDryRun, SmokeRuntimeReceipt,
    SmokeRuntimeScenario, SmokeRuntimeStateHome,
};
