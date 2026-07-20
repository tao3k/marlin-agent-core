use std::env;
use std::process::ExitCode;

use marlin_rust_project_harness_policy::workspace_package_gate::run_workspace_package_gate_cli;

fn main() -> ExitCode {
    run_workspace_package_gate_cli(env::args_os().skip(1))
}
