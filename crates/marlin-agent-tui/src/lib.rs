//! Terminal user interface for the Marlin model gateway.

mod app;
mod cli;
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
mod viewport;
mod workspace;
mod wrap;

pub use app::run_cli;
