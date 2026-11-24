// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{ffi::OsString, fmt::Write, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    GitBlobId, GitCommitId, GitReadError,
    read::{LocalExecutor, MAX_OUTPUT_BYTES, git_with_input, read_git_with},
};

use super::custody::GitRetentionError;

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Claims {
    pub(super) generation: u64,
    pub(super) disposed: bool,
    pub(super) holds: std::collections::BTreeMap<String, HeldEntry>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HeldEntry {
    pub(super) generation: u64,
    pub(super) path: String,
    pub(super) blob: String,
}

pub(super) struct RetentionRefs<'a> {
    repository: &'a Path,
    commit: &'a GitCommitId,
    pin: String,
    journal: String,
}

pub(super) struct Revision {
    pub(super) claims: Claims,
    pin: Option<String>,
    pub(super) journal: Option<String>,
}

impl<'a> RetentionRefs<'a> {
    pub(super) fn new(repository: &'a Path, commit: &'a GitCommitId) -> Self {
        let oid = commit.as_str();
        Self {
            repository,
            commit,
            pin: format!("refs/marlin/context/retained/{oid}"),
            journal: format!("refs/marlin/context/claims/{oid}"),
        }
    }

    pub(super) async fn read(&self) -> Result<Revision, GitRetentionError> {
        let output = read_git_with(
            &LocalExecutor,
            self.repository,
            &arguments(
                self.repository,
                &[
                    "for-each-ref",
                    "--format=%(refname) %(objectname) %(symref)",
                    &self.pin,
                    &self.journal,
                ],
            ),
        )
        .await?;
        let mut pin = None;
        let mut journal = None;
        let text = std::str::from_utf8(&output).map_err(|_| GitRetentionError::InvalidState)?;
        for entry in text.lines() {
            let mut fields = entry.split(' ');
            let (Some(name), Some(oid), Some(""), None) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                return Err(GitRetentionError::InvalidState);
            };
            let oid = GitBlobId::parse(oid)?.as_str().to_owned();
            match name {
                name if name == self.pin && pin.is_none() => pin = Some(oid),
                name if name == self.journal && journal.is_none() => journal = Some(oid),
                _ => return Err(GitRetentionError::InvalidState),
            }
        }
        let claims = match &journal {
            None if pin.is_none() => Claims::default(),
            None => return Err(GitRetentionError::InvalidState),
            Some(oid) => self.read_claims(oid).await?,
        };
        if journal.is_some()
            && (claims.generation == 0
                || (claims.disposed && (!claims.holds.is_empty() || pin.is_some()))
                || (!claims.disposed && pin.as_deref() != Some(self.commit.as_str()))
                || claims
                    .holds
                    .values()
                    .any(|entry| entry.generation == 0 || entry.generation > claims.generation))
        {
            return Err(GitRetentionError::InvalidState);
        }
        Ok(Revision {
            claims,
            pin,
            journal,
        })
    }

    async fn read_claims(&self, oid: &str) -> Result<Claims, GitRetentionError> {
        let bytes = read_git_with(
            &LocalExecutor,
            self.repository,
            &arguments(self.repository, &["cat-file", "blob", oid]),
        )
        .await?;
        serde_json::from_slice(&bytes).map_err(|_| GitRetentionError::InvalidState)
    }

    pub(super) async fn verify_branch_absent(&self, branch: &str) -> Result<(), GitRetentionError> {
        match read_git_with(
            &LocalExecutor,
            self.repository,
            &arguments(
                self.repository,
                &["show-ref", "--verify", "--quiet", branch],
            ),
        )
        .await
        {
            Err(GitReadError::Failed(Some(1))) => Ok(()),
            Ok(_) => Err(GitRetentionError::ActiveBranch),
            Err(error) => Err(error.into()),
        }
    }

    pub(super) async fn write(
        &self,
        revision: &Revision,
        absent_branch: Option<&str>,
        check_owner: impl FnOnce() -> Result<(), GitRetentionError>,
    ) -> Result<(), GitRetentionError> {
        let bytes =
            serde_json::to_vec(&revision.claims).map_err(|_| GitRetentionError::InvalidState)?;
        if bytes.len() > MAX_OUTPUT_BYTES {
            return Err(GitRetentionError::ClaimsLimit);
        }
        let digest = git_with_input(
            self.repository,
            &arguments(self.repository, &["hash-object", "-w", "--stdin"]),
            &bytes,
        )
        .await?;
        let digest = std::str::from_utf8(&digest)
            .ok()
            .and_then(|value| value.strip_suffix('\n'))
            .ok_or(GitRetentionError::InvalidState)?;
        let digest = GitBlobId::parse(digest)?;
        let zero = "0".repeat(self.commit.as_str().len());
        let old_pin = revision.pin.as_deref().unwrap_or(&zero);
        let old_journal = revision.journal.as_deref().unwrap_or(&zero);
        let mut command = String::from("start\n");
        if let Some(branch) = absent_branch {
            let _ = writeln!(command, "option no-deref\nverify {branch} {zero}");
        }
        let _ = writeln!(command, "option no-deref");
        if revision.claims.disposed {
            let _ = writeln!(command, "delete {} {old_pin}", self.pin);
        } else {
            let oid = self.commit.as_str();
            let _ = writeln!(command, "update {} {oid} {old_pin}", self.pin);
        }
        let _ = writeln!(
            command,
            "option no-deref\nupdate {} {} {old_journal}\nprepare\ncommit",
            self.journal,
            digest.as_str()
        );
        check_owner()?;
        git_with_input(
            self.repository,
            &arguments(self.repository, &["update-ref", "--stdin"]),
            command.as_bytes(),
        )
        .await?;
        Ok(())
    }
}

fn arguments(repository: &Path, subcommand: &[&str]) -> Vec<OsString> {
    let mut git: Vec<OsString> = [
        "--no-replace-objects".into(),
        "--no-lazy-fetch".into(),
        "--git-dir".into(),
        repository.as_os_str().to_owned(),
    ]
    .into();
    git.extend(subcommand.iter().map(OsString::from));
    git
}
