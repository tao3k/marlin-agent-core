// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Repository discovery through the bounded Git read path.

use std::path::{Path, PathBuf};

use crate::{
    GitReadError,
    read::{LocalExecutor, read_git_with},
    working_copy::{
        WorkingCopyBranchName, WorkingCopyGitTopLevel, WorkspaceProjectGitHubRepository,
    },
};

/// Typed repository discovery, status and diff operations.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GitOperations;

/// An immutable commit object ID, independent of branch names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitCommitId(String);

impl GitCommitId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Git-owned inputs for shared Context and its worktree evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitContext {
    worktree: WorkingCopyGitTopLevel,
    common_directory: PathBuf,
    primary_worktree: Option<WorkingCopyGitTopLevel>,
    commit: GitCommitId,
}

/// Exact Git checkout used to fence committed Context reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedCheckout {
    pub root: WorkingCopyGitTopLevel,
    pub owner: PathBuf,
    pub git_directory: PathBuf,
    pub reference: GitCheckoutRef,
    pub commit: GitCommitId,
}

/// Git's symbolic HEAD ref, or `HEAD` for a detached checkout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitCheckoutRef(String);

impl GitCheckoutRef {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextDocument {
    Index,
    Topology,
}

impl ContextDocument {
    const fn path(self) -> &'static str {
        match self {
            Self::Index => "index.org",
            Self::Topology => "topology.org",
        }
    }
}

impl CommittedCheckout {
    pub async fn read_context_document(
        &self,
        document: ContextDocument,
    ) -> Result<Vec<u8>, GitReadError> {
        read_git_with(
            &LocalExecutor,
            self.root.as_path(),
            &[
                "--no-replace-objects".into(),
                "--no-lazy-fetch".into(),
                "--git-dir".into(),
                self.owner.as_os_str().to_owned(),
                "cat-file".into(),
                "blob".into(),
                format!("{}:{}", self.commit.as_str(), document.path()).into(),
            ],
        )
        .await
    }

    /// Accept only the name of a local branch before passing it to a provider.
    pub async fn has_local_branch(
        &self,
        branch: &WorkingCopyBranchName,
    ) -> Result<bool, GitReadError> {
        let branch = branch.as_str();
        if branch.starts_with('-') || branch.starts_with('@') || branch.contains(':') {
            return Ok(false);
        }
        match read_git_with(
            &LocalExecutor,
            self.root.as_path(),
            &[
                "show-ref".into(),
                "--verify".into(),
                "--hash".into(),
                format!("refs/heads/{branch}").into(),
            ],
        )
        .await
        {
            Ok(_) => Ok(true),
            Err(GitReadError::Failed(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

impl GitContext {
    pub fn worktree(&self) -> &WorkingCopyGitTopLevel {
        &self.worktree
    }

    pub fn common_directory(&self) -> &Path {
        &self.common_directory
    }

    /// A repository with a bare main entry has no primary worktree.
    pub fn primary_worktree(&self) -> Option<&WorkingCopyGitTopLevel> {
        self.primary_worktree.as_ref()
    }

    pub fn commit(&self) -> &GitCommitId {
        &self.commit
    }
}

impl GitOperations {
    /// Read the stable Git directory shared by a repository's worktrees.
    pub async fn repository_owner(path: &Path) -> Result<PathBuf, GitReadError> {
        common_directory(path).await
    }

    pub async fn committed_checkout(path: &Path) -> Result<CommittedCheckout, GitReadError> {
        let root = Self::resolve_repository_root(path).await?;
        let directory = root.as_path();
        let git_dir = ["rev-parse".into(), "--absolute-git-dir".into()];
        let symbolic = [
            "rev-parse".into(),
            "--symbolic-full-name".into(),
            "HEAD".into(),
        ];
        let head = ["rev-parse".into(), "--verify".into(), "HEAD".into()];
        let (owner, git_directory, reference, commit) = tokio::try_join!(
            common_directory(directory),
            read_git_with(&LocalExecutor, directory, &git_dir),
            read_git_with(&LocalExecutor, directory, &symbolic),
            read_git_with(&LocalExecutor, directory, &head),
        )?;
        let reference = std::str::from_utf8(&reference)
            .map_err(|_| GitReadError::InvalidCommit)?
            .trim_end_matches('\n')
            .to_owned();
        Ok(CommittedCheckout {
            git_directory: canonical(&absolute_path(&git_directory)?).await?,
            root,
            owner,
            reference: GitCheckoutRef(reference),
            commit: commit_id(&commit)?,
        })
    }

    /// Resolve a worktree, its repository owner, and its current committed revision.
    pub async fn context(path: &Path) -> Result<GitContext, GitReadError> {
        let worktree = Self::resolve_repository_root(path).await?;
        let root = worktree.as_path();
        let list_worktrees = ["worktree", "list", "--porcelain", "-z"].map(Into::into);
        let (common_directory, listing, commit) = tokio::try_join!(
            common_directory(root),
            read_git_with(&LocalExecutor, root, &list_worktrees),
            Self::head_commit(&worktree),
        )?;
        let primary_worktree = match main_worktree(&listing)? {
            Some(primary) => Some(Self::resolve_repository_worktree(&worktree, &primary).await?),
            None => None,
        };
        Ok(GitContext {
            worktree,
            common_directory,
            primary_worktree,
            commit,
        })
    }

    pub async fn head_commit(
        worktree: &WorkingCopyGitTopLevel,
    ) -> Result<GitCommitId, GitReadError> {
        let output = read_git_with(
            &LocalExecutor,
            worktree.as_path(),
            &[
                "rev-parse".into(),
                "--verify".into(),
                "HEAD^{commit}".into(),
            ],
        )
        .await?;
        commit_id(&output)
    }

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

fn commit_id(output: &[u8]) -> Result<GitCommitId, GitReadError> {
    let text = std::str::from_utf8(output).map_err(|_| GitReadError::InvalidCommit)?;
    let oid = text.strip_suffix('\n').ok_or(GitReadError::InvalidCommit)?;
    if !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GitReadError::InvalidCommit);
    }
    Ok(GitCommitId(oid.to_ascii_lowercase()))
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
    absolute_path_bytes(raw_path)
}

fn main_worktree(output: &[u8]) -> Result<Option<PathBuf>, GitReadError> {
    let mut fields = output.split(|byte| *byte == 0);
    let path = fields
        .next()
        .and_then(|field| field.strip_prefix(b"worktree "))
        .ok_or(GitReadError::InvalidRepositoryRoot)?;
    if fields
        .take_while(|field| !field.is_empty())
        .any(|field| field == b"bare")
    {
        return Ok(None);
    }
    absolute_path_bytes(path).map(Some)
}

fn absolute_path_bytes(raw_path: &[u8]) -> Result<PathBuf, GitReadError> {
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
