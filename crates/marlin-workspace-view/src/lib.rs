// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rendered view contracts for agent and UI context.

mod render;
mod spec;

pub use render::{
    RenderedContractFacts, RenderedContractFactsInput, RenderedViewNode, RenderedWorkspaceView,
};
pub use spec::{RenderMode, WorkspaceField, WorkspaceViewSpec};
