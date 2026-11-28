// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Git tree observations for Context locator metadata.

use crate::{
    CommittedCheckout, GitCommitId, GitReadError, GitRepositoryPath,
    read::{LocalExecutor, read_git_with},
};

/// The full Git identity of a blob, separate from an immutable commit ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitBlobId(String);

impl GitBlobId {
    pub fn parse(oid: &str) -> Result<Self, GitReadError> {
        if !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(GitReadError::InvalidBlob);
        }
        Ok(Self(oid.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The commit, repository-relative path and blob of one tree entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitBlobLocator {
    pub commit: GitCommitId,
    pub path: GitRepositoryPath,
    pub blob: GitBlobId,
}

impl CommittedCheckout {
    /// Observe an exact tree entry within an already authorized source cut.
    ///
    /// Git resolves the path against the immutable commit. The object payload
    /// is read separately when the caller holds current display authority.
    pub async fn locate_blob(
        &self,
        path: GitRepositoryPath,
    ) -> Result<GitBlobLocator, GitReadError> {
        let output = read_git_with(
            &LocalExecutor,
            self.root.as_path(),
            &[
                "--no-replace-objects".into(),
                "--no-lazy-fetch".into(),
                "--literal-pathspecs".into(),
                "--git-dir".into(),
                self.owner.as_os_str().to_owned(),
                "ls-tree".into(),
                "--full-tree".into(),
                "-z".into(),
                self.commit.as_str().into(),
                "--".into(),
                path.as_str().into(),
            ],
        )
        .await?;
        let text = std::str::from_utf8(&output).map_err(|_| GitReadError::InvalidBlob)?;
        let (header, name) = text
            .strip_suffix('\0')
            .and_then(|entry| entry.split_once('\t'))
            .ok_or(GitReadError::InvalidBlob)?;
        let mut fields = header.split(' ');
        let kind = (fields.next(), fields.next(), fields.next(), fields.next());
        let oid = match kind {
            (Some("100644" | "100755"), Some("blob"), Some(oid), None) if name == path.as_str() => {
                oid
            }
            _ => return Err(GitReadError::InvalidBlob),
        };
        Ok(GitBlobLocator {
            commit: self.commit.clone(),
            path,
            blob: GitBlobId::parse(oid)?,
        })
    }

    /// Reconcile an indexed locator with the owner before opening a Git blob.
    /// The caller checks the source grant again before publishing these bytes.
    pub async fn read_located_blob(
        &self,
        locator: &GitBlobLocator,
    ) -> Result<Vec<u8>, GitReadError> {
        if self.commit != locator.commit
            || self.locate_blob(locator.path.clone()).await? != *locator
        {
            return Err(GitReadError::LocatorMismatch);
        }
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
                locator.blob.as_str().into(),
            ],
        )
        .await
    }
}
