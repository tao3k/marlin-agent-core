// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Repository discovery through the bounded Git read path.

use std::path::{Path, PathBuf};

use crate::{
    GitReadError,
    read::{LocalExecutor, read_git_with},
    working_copy::{WorkingCopyGitTopLevel, WorkspaceProjectGitHubRepository},
};

/// Typed repository discovery, status and diff operations.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GitOperations;

impl GitOperations {
    /// Read configured and rewritten fetch URLs for the selected repository.
    pub async fn fetch_remote_urls(
        repository: &WorkingCopyGitTopLevel,
    ) -> Result<Vec<String>, GitReadError> {
        let remotes =
            read_git_with(&LocalExecutor, repository.as_path(), &["remote".into()]).await?;
        let remotes = std::str::from_utf8(&remotes).map_err(|_| GitReadError::InvalidRemoteUrls)?;
        let mut urls = Vec::new();
        for remote in remotes.lines() {
            for args in [
                vec![
                    "config".into(),
                    "--null".into(),
                    "--get-all".into(),
                    format!("remote.{remote}.url").into(),
                ],
                vec![
                    "remote".into(),
                    "get-url".into(),
                    "--all".into(),
                    "--".into(),
                    remote.into(),
                ],
            ] {
                let output = read_git_with(&LocalExecutor, repository.as_path(), &args).await?;
                let output =
                    std::str::from_utf8(&output).map_err(|_| GitReadError::InvalidRemoteUrls)?;
                let values = if let Some(raw) = output.strip_suffix('\0') {
                    raw.split('\0').collect::<Vec<_>>()
                } else {
                    output.lines().collect()
                };
                urls.extend(values.into_iter().map(str::to_owned));
            }
        }
        Ok(urls)
    }

    /// Resolve the absolute Git worktree root for a path inside the repository.
    pub async fn resolve_repository_root(
        path: &Path,
    ) -> Result<WorkingCopyGitTopLevel, GitReadError> {
        let output = read_git_with(
            &LocalExecutor,
            path,
            &["rev-parse".into(), "--show-toplevel".into()],
        )
        .await?;
        absolute_path(&output).map(WorkingCopyGitTopLevel::from_resolved_path)
    }

    /// Resolve a worktree root and check that it shares the selected repository.
    pub async fn resolve_repository_worktree(
        repository: &WorkingCopyGitTopLevel,
        target: &Path,
    ) -> Result<WorkingCopyGitTopLevel, GitReadError> {
        if target.as_os_str().is_empty() {
            return Err(GitReadError::UnrelatedWorktree(target.to_owned()));
        }
        let target = repository.as_path().join(target);
        let root = Self::resolve_repository_root(&target).await?;
        if canonical(&target).await? != canonical(root.as_path()).await?
            || common_directory(repository.as_path()).await?
                != common_directory(root.as_path()).await?
        {
            return Err(GitReadError::UnrelatedWorktree(target));
        }
        Ok(root)
    }
}

impl WorkspaceProjectGitHubRepository {
    /// Match the public GitHub repository of a standard fetch URL.
    pub fn matches_fetch_url(&self, url: &str) -> bool {
        let slug = [
            "https://github.com/",
            "ssh://git@github.com/",
            "git@github.com:",
        ]
        .into_iter()
        .find_map(|prefix| url.strip_prefix(prefix));
        slug.is_some_and(|slug| {
            let slug = slug.strip_suffix('/').unwrap_or(slug);
            slug.strip_suffix(".git")
                .unwrap_or(slug)
                .eq_ignore_ascii_case(&self.slug())
        })
    }
}

async fn common_directory(root: &Path) -> Result<PathBuf, GitReadError> {
    let output = read_git_with(
        &LocalExecutor,
        root,
        &[
            "rev-parse".into(),
            "--path-format=absolute".into(),
            "--git-common-dir".into(),
        ],
    )
    .await?;
    canonical(&absolute_path(&output)?).await
}

async fn canonical(path: &Path) -> Result<PathBuf, GitReadError> {
    tokio::fs::canonicalize(path)
        .await
        .map_err(|error| GitReadError::Io {
            kind: error.kind(),
            message: error.to_string(),
        })
}

fn absolute_path(output: &[u8]) -> Result<PathBuf, GitReadError> {
    let raw_path = output
        .strip_suffix(b"\n")
        .ok_or(GitReadError::InvalidRepositoryRoot)?;
    #[cfg(unix)]
    let path = {
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(std::ffi::OsStr::from_bytes(raw_path))
    };
    #[cfg(not(unix))]
    let path = PathBuf::from(
        std::str::from_utf8(raw_path).map_err(|_| GitReadError::InvalidRepositoryRoot)?,
    );
    if !path.is_absolute() {
        return Err(GitReadError::InvalidRepositoryRoot);
    }
    Ok(path)
}
