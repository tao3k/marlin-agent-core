// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Store trait for durable `Org` document text.

use std::collections::BTreeMap;

/// Result type returned by source store adapters.
pub type OrgSourceStoreResult<T> = Result<T, OrgSourceStoreError>;

/// An adapter's observation of its source state, not a lock or transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrgSourceCleanState {
    Clean,
    Dirty,
    Unknown,
}

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
    /// Distinguish a missing document from an invalid path or read failure.
    fn read_document(&self, document: &str) -> OrgSourceStoreResult<Option<String>>;

    fn write_documents(&mut self, documents: BTreeMap<String, String>) -> OrgSourceStoreResult<()>;

    /// Reports observed state without locking the source or providing CAS.
    fn clean_state(&self) -> OrgSourceCleanState {
        OrgSourceCleanState::Unknown
    }
}

/// Result of a single atomic comparison and write operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrgAtomicWriteOutcome {
    /// All expected document contents matched and all changes were accepted.
    Applied,
    /// At least one expected document differed; no changes were applied.
    Conflict,
}

/// Document identity within an `Org` source store.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OrgSourceDocumentId(String);

impl OrgSourceDocumentId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for OrgSourceDocumentId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for OrgSourceDocumentId {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

/// Store capable of atomically checking exact source contents and writing changes.
///
/// The comparison covers every entry in `expected`, including unchanged inputs.
/// Implementations must serialize this operation with all other writes to the
/// same source. On `Conflict` no write occurred; an error may be uncertain.
/// Matching bytes do not establish revision identity or prevent an ABA change.
pub trait OrgAtomicSourceStore: OrgSourceStore {
    fn compare_and_write(
        &mut self,
        expected: &BTreeMap<OrgSourceDocumentId, String>,
        changes: BTreeMap<OrgSourceDocumentId, String>,
    ) -> OrgSourceStoreResult<OrgAtomicWriteOutcome>;
}
