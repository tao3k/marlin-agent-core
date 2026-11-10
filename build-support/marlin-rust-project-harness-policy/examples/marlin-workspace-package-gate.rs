#[path = "../tests/support/mod.rs"]
pub mod policy;

use std::env;
use std::process::ExitCode;

use crate::policy::workspace_package_gate::run_workspace_package_gate_cli;

fn main() -> ExitCode {
    run_workspace_package_gate_cli(env::args_os().skip(1))
}
