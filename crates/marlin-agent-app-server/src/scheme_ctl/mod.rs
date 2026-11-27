// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Operator CLI (`marlin-scheme-ctl`) module owned by `marlin-agent-app-server`.

pub mod args;
mod runner;

pub use runner::run_cli;
