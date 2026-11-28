// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Entrypoint for the Marlin terminal client.

fn main() {
    if std::env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some() {
        eprintln!("marlin-tui startup Rust main");
    }
    marlin_agent_tui::launch();
}
