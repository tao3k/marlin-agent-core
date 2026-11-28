// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Thin binary launcher for `marlin-scheme-ctl`.
//! All CLI argument parsing and gRPC command orchestration lives in
//! `marlin_agent_app_server::scheme_ctl`.

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(err) = marlin_agent_app_server::scheme_ctl::run_cli().await {
        eprintln!("marlin-scheme-ctl error: {err}");
        std::process::exit(1);
    }
}
