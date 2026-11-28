// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_gitops::{CommittedCheckout, GitBlobLocator, GitCommitId};

use crate::{StorageError, StorageResult};

/// Stable branch identity or the full commit of a detached cut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextLocatorKey(String);

impl ContextLocatorKey {
    pub fn for_checkout(checkout: &CommittedCheckout) -> Self {
        let reference = checkout.reference.as_str();
        if reference == "HEAD" {
            return Self(format!("detached/{}", checkout.commit.as_str()));
        }
        Self(reference.to_owned())
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

/// One observed Git tree entry and the generation admitted by its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextLocator {
    pub generation: u16,
    pub git: GitBlobLocator,
}

/// A complete set of selected entries for an observed ref or detached commit.
#[derive(Clone, Debug)]
pub struct ContextCut {
    pub key: ContextLocatorKey,
    pub tip: GitCommitId,
    pub entries: Vec<ContextLocator>,
}

impl ContextCut {
    pub fn for_checkout(checkout: &CommittedCheckout, entries: Vec<ContextLocator>) -> Self {
        Self {
            key: ContextLocatorKey::for_checkout(checkout),
            tip: checkout.commit.clone(),
            entries,
        }
    }

    pub(super) fn validate(&self) -> StorageResult<()> {
        let mut paths = std::collections::BTreeSet::new();
        if self.entries.is_empty()
            || self.entries.iter().any(|record| {
                record.generation == 0
                    || record.generation == u16::MAX
                    || record.git.commit != self.tip
                    || !paths.insert(record.git.path.as_str())
            })
        {
            return Err(StorageError::InvalidIdentifier {
                kind: "Context cut entries",
            });
        }
        Ok(())
    }
}

/// Identity and sequence of one owner-issued publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextOwnerRevision {
    pub(super) sequence: i64,
    pub(super) receipt: String,
}

impl ContextOwnerRevision {
    pub fn new(sequence: i64, receipt: impl Into<String>) -> StorageResult<Self> {
        let receipt = receipt.into();
        if sequence <= 0
            || receipt.trim().is_empty()
            || receipt.len() > 256
            || receipt.bytes().any(|byte| byte < 0x20 || byte == 0x7f)
        {
            return Err(StorageError::InvalidIdentifier {
                kind: "Context owner revision",
            });
        }
        Ok(Self { sequence, receipt })
    }
}

/// Physical custody stays with the producer when a current ref disappears.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextRetirement {
    Deleted,
    Replaced,
}

impl ContextRetirement {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Deleted => "deleted",
            Self::Replaced => "replaced",
        }
    }
}

/// One increment of an owner's publication stream.
pub enum ContextCutChange<'a> {
    Publish(&'a ContextCut),
    Retire {
        key: &'a ContextLocatorKey,
        expected_tip: &'a GitCommitId,
        reason: ContextRetirement,
    },
}

/// Observable outcome of applying an owner event to the derived store.
#[derive(Debug, PartialEq, Eq)]
pub enum ContextIndexUpdate {
    Applied,
    Replayed,
    Superseded,
    Gap { cursor: i64, observed: i64 },
    Conflict,
}

/// A scoped lookup may need owner replay before it can prove absence.
#[derive(Debug, PartialEq, Eq)]
pub enum ContextIndexRead {
    Incomplete,
    Absent,
    Current(ContextLocator),
}
