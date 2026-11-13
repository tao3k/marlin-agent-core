// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Workspace query selectors, filters, and result records.

mod filter;
mod result;
mod selector;

pub use filter::{PropertyFilter, QueryFilter, QueryOrder, WorkspaceQuery};
pub use result::{QueryMatch, WorkspaceQueryResult};
pub use selector::{NodeSelector, SourceRange, WorkspaceScope};
