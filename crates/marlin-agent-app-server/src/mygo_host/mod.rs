// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Production-grade Marlin MyGo Authority Host daemon module.

pub mod config;
pub mod fs_bridge;
pub mod magit_bridge;
mod runner;

pub use fs_bridge::query_fs_records;
pub use magit_bridge::{WorkspaceMagitBridge, parse_magit_ui_command};
pub use runner::{
    DeterministicLocalGateway, MAX_RESIDENT_SESSIONS, ResidentSessionTable, SchemeUiSurfaceBundle,
    lookup_host_session, run_cli, run_cli_with_host_ready, run_cli_with_scheme_surface,
    run_ui_command_router_loop,
};
