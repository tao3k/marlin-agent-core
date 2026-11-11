// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[path = "../tests/support/mod.rs"]
pub mod policy;

use std::env;
use std::process::ExitCode;

use crate::policy::workspace_package_gate::run_workspace_package_gate_cli;

fn main() -> ExitCode {
    run_workspace_package_gate_cli(env::args_os().skip(1))
}
