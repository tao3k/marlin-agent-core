//! Store trait for durable `Org` document text.

use std::collections::BTreeMap;

/// Result type returned by source store adapters.
pub type OrgSourceStoreResult<T> = Result<T, OrgSourceStoreError>;

/// Store adapter failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrgSourceStoreError {
    pub message: String,
}

impl OrgSourceStoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Persistence adapter for named `Org` documents.
pub trait OrgSourceStore {
    fn read_document(&self, document: &str) -> Option<String>;

    fn write_documents(&mut self, documents: BTreeMap<String, String>) -> OrgSourceStoreResult<()>;

    fn is_clean(&self) -> bool {
        true
    }
}
