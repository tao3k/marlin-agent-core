// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Production-grade Marlin MyGo Authority Host daemon module.

pub mod config;
mod runner;

pub use runner::{
    DeterministicLocalGateway, MAX_RESIDENT_SESSIONS, ResidentSessionTable, lookup_host_session,
    run_cli, run_ui_command_router_loop,
};
