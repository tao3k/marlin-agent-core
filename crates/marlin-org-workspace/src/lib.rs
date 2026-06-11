//! First-party `Org` document loading adapter for workspace backends.

mod contract;
mod document;

pub use document::{OrgDocument, OrgDocumentId, OrgDocumentLoader, OrgDocumentWorkspace};
