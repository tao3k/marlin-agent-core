// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Thin binary launcher for `marlin-mygo-host`.
//! All host configuration, auth, governance, and routing logic lives in
//! `marlin_agent_app_server::mygo_host`.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    marlin_agent_app_server::mygo_host::run_cli().await
}
