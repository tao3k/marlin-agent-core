//! Workspace query selectors, filters, and result records.

mod filter;
mod result;
mod selector;

pub use filter::{PropertyFilter, QueryFilter, QueryOrder, WorkspaceQuery};
pub use result::{QueryMatch, WorkspaceQueryResult};
pub use selector::{NodeSelector, SourceRange, WorkspaceScope};
