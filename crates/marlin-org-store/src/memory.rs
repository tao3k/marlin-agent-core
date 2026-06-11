//! In-memory source store used by tests and non-filesystem callers.

use std::collections::BTreeMap;

use crate::{OrgSourceStore, OrgSourceStoreError, OrgSourceStoreResult};

/// In-memory implementation of the source store contract.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemoryOrgSourceStore {
    documents: BTreeMap<String, String>,
    clean: bool,
    fail_writes: bool,
}

impl MemoryOrgSourceStore {
    pub fn new(documents: BTreeMap<String, String>) -> Self {
        Self {
            documents,
            clean: true,
            fail_writes: false,
        }
    }

    pub fn document(&self, document: &str) -> Option<&str> {
        self.documents.get(document).map(String::as_str)
    }

    pub fn set_clean(&mut self, clean: bool) {
        self.clean = clean;
    }

    pub fn fail_writes(&mut self) {
        self.fail_writes = true;
    }
}

impl OrgSourceStore for MemoryOrgSourceStore {
    fn read_document(&self, document: &str) -> Option<String> {
        self.documents.get(document).cloned()
    }

    fn write_documents(&mut self, documents: BTreeMap<String, String>) -> OrgSourceStoreResult<()> {
        if self.fail_writes {
            return Err(OrgSourceStoreError::new("in-memory store rejected write"));
        }

        self.documents.extend(documents);
        self.clean = false;
        Ok(())
    }

    fn is_clean(&self) -> bool {
        self.clean
    }
}
