// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Read-only diffs with explicit index or worktree scope.

use crate::{GitReadError, ProcessGitTooling};
use std::{
    ffi::OsString,
    path::{Component, Path},
};
use thiserror::Error;

/// Comparison selected by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitDiffScope {
    Worktree,
    Index,
}

/// A UTF-8 path relative to the repository root, interpreted literally by Git.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitRepositoryPath(String);

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("path must be a nonempty repository-relative path without parent components or NUL")]
pub struct InvalidGitRepositoryPath;

impl GitRepositoryPath {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidGitRepositoryPath> {
        let value = value.into();
        if value.is_empty()
            || value.contains('\0')
            || !Path::new(&value)
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(InvalidGitRepositoryPath);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Explicit comparison and optional literal repository-root-relative path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitDiffRequest {
    pub scope: GitDiffScope,
    pub path: Option<GitRepositoryPath>,
}

impl GitDiffRequest {
    pub fn all(scope: GitDiffScope) -> Self {
        Self { scope, path: None }
    }
    pub fn for_path(scope: GitDiffScope, path: GitRepositoryPath) -> Self {
        Self {
            scope,
            path: Some(path),
        }
    }
}

/// Complete bounded Git output; byte identity is retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitDiff {
    pub request: GitDiffRequest,
    bytes: Vec<u8>,
}

impl GitDiff {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Escapes controls while preserving line boundaries and valid Unicode.
    pub fn display_escaped(&self) -> String {
        let mut output = String::new();
        match std::str::from_utf8(&self.bytes) {
            Ok(value) => {
                for c in value.chars() {
                    if c.is_control() && c != '\n' {
                        output.extend(c.escape_debug());
                    } else {
                        output.push(c);
                    }
                }
            }
            Err(_) => {
                for byte in &self.bytes {
                    if *byte == b'\n' {
                        output.push('\n');
                    } else {
                        output.extend(std::ascii::escape_default(*byte).map(char::from));
                    }
                }
            }
        }
        output
    }
}

impl ProcessGitTooling {
    /// Read a bounded diff; external diff programs and text conversion are disabled.
    pub async fn diff(path: &Path, request: GitDiffRequest) -> Result<GitDiff, GitReadError> {
        Self::diff_with_executor(&crate::read::LocalExecutor, path, request).await
    }

    pub async fn diff_with_executor(
        executor: &dyn marlin_agent_protocol::BoundedProcessExecutor,
        path: &Path,
        request: GitDiffRequest,
    ) -> Result<GitDiff, GitReadError> {
        let mut args: Vec<OsString> = [
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--no-renames",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        if request.scope == GitDiffScope::Index {
            args.push("--cached".into());
        }
        args.push("--".into());
        if let Some(selected) = &request.path {
            args.push(format!(":(top,literal){}", selected.as_str()).into());
        }
        Ok(GitDiff {
            request,
            bytes: crate::read::read_git_with(executor, path, &args).await?,
        })
    }
}
