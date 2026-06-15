//! Harness/debug command surface behind the `marlin` binary.

mod args;
mod catalog;
mod driver;
mod executor;
mod graph;
mod io;
mod loop_cmd;
mod process_command;
mod receipts;

pub use driver::{MarlinCliResult, run_marlin_cli, run_marlin_cli_from_args};
pub(in crate::debug_cli) use driver::{graph_usage, loop_usage, usage};
pub use receipts::{
    GraphQueryOutput, GraphQuerySummary, LoopEventQuerySummary, LoopInspectReceipt,
    LoopQuerySummary, LoopReplayReceipt, LoopRunReceipt, ProjectRuntimeQuerySummary,
};
