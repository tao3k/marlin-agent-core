// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded, read-only Git worktree status.

use std::path::Path;

use crate::{GitReadError, ProcessGitTooling, read::MAX_OUTPUT_BYTES};
use thiserror::Error;

#[cfg(test)]
#[path = "../tests/unit/status_parser.rs"]
mod tests;

const MAX_ENTRIES: usize = 10_000;

/// A repository-relative path as emitted by Git, preserving non-UTF-8 bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitPath(Vec<u8>);

impl GitPath {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Escape controls and invalid UTF-8 for display without terminal escapes.
    pub fn display_escaped(&self) -> String {
        match std::str::from_utf8(&self.0) {
            Ok(value) => value.chars().flat_map(char::escape_debug).collect(),
            Err(_) => self
                .0
                .iter()
                .flat_map(|byte| std::ascii::escape_default(*byte))
                .map(char::from)
                .collect(),
        }
    }
}

/// One index or worktree state in Git porcelain status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitFileState {
    Unchanged,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Untracked,
}

impl GitFileState {
    fn parse(value: u8) -> Result<Self, GitStatusError> {
        match value {
            b' ' => Ok(Self::Unchanged),
            b'M' => Ok(Self::Modified),
            b'A' => Ok(Self::Added),
            b'D' => Ok(Self::Deleted),
            b'R' => Ok(Self::Renamed),
            b'C' => Ok(Self::Copied),
            b'T' => Ok(Self::TypeChanged),
            b'U' => Ok(Self::Unmerged),
            b'?' => Ok(Self::Untracked),
            _ => Err(GitStatusError::InvalidOutput),
        }
    }
    pub fn code(self) -> char {
        match self {
            Self::Unchanged => ' ',
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Copied => 'C',
            Self::TypeChanged => 'T',
            Self::Unmerged => 'U',
            Self::Untracked => '?',
        }
    }
}

/// One status entry, including the source of a rename or copy when present.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitStatusEntry {
    pub index: GitFileState,
    pub worktree: GitFileState,
    pub path: GitPath,
    pub original_path: Option<GitPath>,
}

/// Complete status within the declared output and entry limits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitStatus {
    pub entries: Vec<GitStatusEntry>,
}

/// Failure never returns a partial status as complete.
#[derive(Debug, Error)]
pub enum GitStatusError {
    #[error(transparent)]
    Read(#[from] GitReadError),
    #[error("git status exceeded the entry limit")]
    EntryLimit,
    #[error("git status returned invalid porcelain output")]
    InvalidOutput,
}

impl ProcessGitTooling {
    /// Read the current worktree status without optional index locks or hooks.
    pub async fn status(path: &Path) -> Result<GitStatus, GitStatusError> {
        Self::status_with_executor(&crate::read::LocalExecutor, path).await
    }

    pub async fn status_with_executor(
        executor: &dyn marlin_agent_protocol::BoundedProcessExecutor,
        path: &Path,
    ) -> Result<GitStatus, GitStatusError> {
        let args = ["status", "--porcelain=v1", "-z", "--untracked-files=normal"].map(Into::into);
        let output = crate::read::read_git_with(executor, path, &args).await?;
        parse_status(&output)
    }
}

fn parse_status(output: &[u8]) -> Result<GitStatus, GitStatusError> {
    if output.len() > MAX_OUTPUT_BYTES {
        return Err(GitReadError::OutputLimit.into());
    }
    if output.is_empty() {
        return Ok(GitStatus {
            entries: Vec::new(),
        });
    }
    if !output.is_empty() && !output.ends_with(&[0]) {
        return Err(GitStatusError::InvalidOutput);
    }
    let mut fields = output[..output.len() - 1].split(|byte| *byte == 0);
    let mut entries = Vec::new();
    while let Some(item) = fields.next() {
        if entries.len() == MAX_ENTRIES {
            return Err(GitStatusError::EntryLimit);
        }
        if item.len() < 4 || item[2] != b' ' {
            return Err(GitStatusError::InvalidOutput);
        }
        let index = GitFileState::parse(item[0])?;
        let worktree = GitFileState::parse(item[1])?;
        let rename = matches!(index, GitFileState::Renamed | GitFileState::Copied)
            || matches!(worktree, GitFileState::Renamed | GitFileState::Copied);
        let original_path = if rename {
            Some(GitPath(
                fields
                    .next()
                    .filter(|value| !value.is_empty())
                    .ok_or(GitStatusError::InvalidOutput)?
                    .to_vec(),
            ))
        } else {
            None
        };
        entries.push(GitStatusEntry {
            index,
            worktree,
            path: GitPath(item[3..].to_vec()),
            original_path,
        });
    }
    Ok(GitStatus { entries })
}
