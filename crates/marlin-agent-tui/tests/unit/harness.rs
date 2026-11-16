// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Unit cases exercise the actual library modules, rather than compiling a
// second copy of the application into an integration-test crate.
pub(crate) use crate::{
    app, cli, composer, config, context_budget, edge_gateway, event_queue, frame_schedule, input,
    journal_gateway, model_catalog, output_limit, prompt_history, render, replay, session_browser,
    ui_elements, ui_fixture, viewport, workspace, wrap,
};

#[path = "app_cases.rs"]
mod app_cases;
#[path = "composer.rs"]
mod composer_cases;
#[path = "context_budget.rs"]
mod context_budget_cases;
#[path = "edge_gateway.rs"]
mod edge_gateway_cases;
#[path = "event_queue.rs"]
mod event_queue_cases;
#[path = "frame_schedule.rs"]
mod frame_schedule_cases;
#[path = "input.rs"]
mod input_cases;
#[path = "journal_gateway.rs"]
mod journal_gateway_cases;
#[path = "model_catalog.rs"]
mod model_catalog_cases;
#[path = "output_limit.rs"]
mod output_limit_cases;
#[path = "prompt_history.rs"]
mod prompt_history_cases;
#[path = "replay.rs"]
mod replay_cases;
#[path = "session_browser.rs"]
mod session_browser_cases;
#[path = "streaming.rs"]
mod streaming;
#[path = "ui_elements/mod.rs"]
mod ui_elements_cases;
#[path = "viewport.rs"]
mod viewport_cases;
#[path = "workspace.rs"]
mod workspace_cases;
#[path = "wrap_comparison.rs"]
mod wrap_comparison;
