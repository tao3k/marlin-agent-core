// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Native Scheme tooling projection and reusable resident-search tool bindings
//! connecting declared capabilities to Marlin's `ModelToolCatalog`.

mod projection;
mod search_tool;

pub use projection::{
    ToolingProjectionError, catalog_from_declarations, catalog_from_declarations_with_search,
    decode_tooling, project_linked,
};
pub use search_tool::{
    ResidentSearchToolHandler, SharedSearchIndexSlot, SharedWorkspaceSearchIndex,
};
