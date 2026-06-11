//! First-party `Org` document loading adapter for workspace backends.

mod contract;
mod document;
mod validation;

pub use document::{OrgDocument, OrgDocumentId, OrgDocumentLoader, OrgDocumentWorkspace};
