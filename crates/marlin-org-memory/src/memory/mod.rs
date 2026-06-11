//! In-memory Org workspace backend for protocol tests and local agents.

mod contracts;
mod format;
mod patch;
mod query;
mod render;
mod status;
mod workspace;

pub use workspace::MemoryOrgWorkspace;
