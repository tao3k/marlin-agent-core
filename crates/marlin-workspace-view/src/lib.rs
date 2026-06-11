//! Rendered view contracts for agent and UI context.

mod render;
mod spec;

pub use render::{RenderedContractFacts, RenderedViewNode, RenderedWorkspaceView};
pub use spec::{RenderMode, WorkspaceField, WorkspaceViewSpec};
