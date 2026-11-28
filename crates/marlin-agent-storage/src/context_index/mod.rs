// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rebuildable locator metadata within a single authorized storage scope.

mod model;
mod retention;
mod store;

pub use model::{
    ContextCut, ContextCutChange, ContextIndexRead, ContextIndexUpdate, ContextLocator,
    ContextLocatorKey, ContextOwnerRevision, ContextRetirement,
};
pub use store::TursoContextIndex;
