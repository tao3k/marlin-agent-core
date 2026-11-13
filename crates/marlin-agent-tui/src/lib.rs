// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal user interface for the Marlin model gateway.

mod app;
mod bootstrap;
mod cli;
mod command;
mod composer;
mod config;
mod context_budget;
mod edge_gateway;
mod event_queue;
mod frame_schedule;
mod input;
mod journal_gateway;
mod model_catalog;
mod output_limit;
mod prompt_history;
mod render;
mod replay;
mod session_browser;
mod signals;
pub mod ui_elements;
#[cfg(all(test, not(feature = "linked-ui")))]
#[path = "../tests/unit/ui_fixture.rs"]
mod ui_fixture;
mod ui_host;
mod viewport;
mod workspace;
mod wrap;

pub use app::run_cli;
pub use bootstrap::launch;

#[cfg(feature = "linked-ui")]
mod linked_ui;
