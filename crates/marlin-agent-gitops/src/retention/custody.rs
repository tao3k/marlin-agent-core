// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::{
    CommittedCheckout, EnrolledRepository, GitBlobLocator, GitCommitId, GitReadError,
    RepositoryEnrollmentError, RepositoryInstanceId,
};

use super::refs::{Claims, HeldEntry, RetentionRefs, Revision};

#[cfg(test)]
#[path = "../../tests/unit/retention.rs"]
mod tests;

/// Stable identity assigned by the owner for one published reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitContextHoldId(String);

impl GitContextHoldId {
    pub fn new(value: impl Into<String>) -> Result<Self, GitRetentionError> {
        let value = value.into();
        if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return Err(GitRetentionError::InvalidHold);
        }
        Ok(Self(value))
    }
}

/// A retained blob's owner-local reference receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitContextHold {
    owner: RepositoryInstanceId,
    commit: GitCommitId,
    id: GitContextHoldId,
    generation: u64,
}

impl GitContextHold {
    /// Persist this generation with the publication's hold ID and locator.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

/// Exact Git state nominated for authorized maintenance after retirement.
#[derive(Debug)]
pub struct GitContextDisposal {
    owner: RepositoryInstanceId,
    commit: GitCommitId,
    branch: String,
    journal: String,
}

/// Git-side custody service bound to an enrolled owner.
pub struct GitContextCustodian {
    home: PathBuf,
    owner: EnrolledRepository,
    repository: PathBuf,
}

/// A failed custody check leaves publication or disposal unqualified.
#[derive(Debug, Error)]
pub enum GitRetentionError {
    #[error(transparent)]
    Git(#[from] GitReadError),
    #[error(transparent)]
    Enrollment(#[from] RepositoryEnrollmentError),
    #[error("checkout or custody receipt does not belong to the enrolled repository")]
    OwnerChanged,
    #[error("invalid or reused Git Context hold")]
    InvalidHold,
    #[error("the Git custody state is incomplete or inconsistent")]
    InvalidState,
    #[error("the Git custody journal has reached its size limit")]
    ClaimsLimit,
    #[error("the Git Context revision is still held by a publication")]
    LiveHold,
    #[error("a surviving branch prevents deletion-based retirement")]
    ActiveBranch,
    #[error("the retirement requires a named source branch")]
    UnnamedBranch,
    #[error("new custody evidence superseded the maintenance ticket")]
    StaleDisposal,
}

impl GitContextCustodian {
    pub fn open(home: &Path, repository: &Path) -> Result<Self, GitRetentionError> {
        let owner =
            EnrolledRepository::load(home, repository)?.ok_or(GitRetentionError::OwnerChanged)?;
        Ok(Self {
            home: home.to_owned(),
            owner,
            repository: repository.to_owned(),
        })
    }

    /// Identity of the producer whose private references this service can change.
    pub fn instance(&self) -> &RepositoryInstanceId {
        self.owner.instance()
    }

    /// Acquire the producer's hold before exposing a durable locator.
    pub async fn retain(
        &self,
        checkout: &CommittedCheckout,
        locator: &GitBlobLocator,
        id: &GitContextHoldId,
    ) -> Result<GitContextHold, GitRetentionError> {
        self.check_checkout(checkout)?;
        if checkout.commit != locator.commit
            || checkout.locate_blob(locator.path.clone()).await? != *locator
        {
            return Err(GitReadError::LocatorMismatch.into());
        }
        let refs = self.refs(&locator.commit);
        let mut revision = refs.read().await?;
        let generation = match revision.claims.holds.get(&id.0) {
            Some(hold) if matches_locator(hold, locator) => hold.generation,
            Some(_) => return Err(GitRetentionError::InvalidHold),
            None => {
                let generation = next_generation(&mut revision.claims)?;
                revision.claims.disposed = false;
                revision.claims.holds.insert(
                    id.0.clone(),
                    HeldEntry {
                        generation,
                        path: locator.path.as_str().to_owned(),
                        blob: locator.blob.as_str().to_owned(),
                    },
                );
                self.write(&refs, &revision, None).await?;
                generation
            }
        };
        self.check_identity(self.instance())?;
        Ok(self.hold_receipt(locator, id, generation))
    }

    /// Recover a previously published hold at its exact generation after restart.
    pub async fn recover_hold(
        &self,
        locator: &GitBlobLocator,
        id: &GitContextHoldId,
        generation: u64,
    ) -> Result<GitContextHold, GitRetentionError> {
        self.check_identity(self.instance())?;
        let revision = self.refs(&locator.commit).read().await?;
        match revision.claims.holds.get(&id.0) {
            Some(entry) if entry.generation == generation && matches_locator(entry, locator) => {
                Ok(self.hold_receipt(locator, id, generation))
            }
            _ => Err(GitRetentionError::InvalidHold),
        }
    }

    /// Withdraw one published reference. Git retains the underlying source.
    pub async fn release(&self, hold: &GitContextHold) -> Result<(), GitRetentionError> {
        self.check_identity(&hold.owner)?;
        let refs = self.refs(&hold.commit);
        let mut revision = refs.read().await?;
        if revision
            .claims
            .holds
            .get(&hold.id.0)
            .map(|entry| entry.generation)
            != Some(hold.generation)
        {
            return Err(GitRetentionError::InvalidHold);
        }
        revision.claims.holds.remove(&hold.id.0);
        next_generation(&mut revision.claims)?;
        self.write(&refs, &revision, None).await
    }

    /// Prepare an explicit disposal after the policy owner admits expiry and
    /// reference coverage for a deleted cut.
    pub async fn prepare_disposal(
        &self,
        deleted: &CommittedCheckout,
    ) -> Result<GitContextDisposal, GitRetentionError> {
        self.check_checkout(deleted)?;
        let branch = deleted.reference.as_str();
        if !branch.starts_with("refs/heads/")
            || branch.chars().any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(GitRetentionError::UnnamedBranch);
        }
        let refs = self.refs(&deleted.commit);
        refs.verify_branch_absent(branch).await?;
        let revision = refs.read().await?;
        verify_disposal(&revision.claims)?;
        let journal = revision.journal.ok_or(GitRetentionError::InvalidState)?;
        Ok(GitContextDisposal {
            owner: self.owner.instance().clone(),
            commit: deleted.commit.clone(),
            branch: branch.to_owned(),
            journal,
        })
    }

    /// Remove only the private pin while the journal, branch and authority match.
    pub async fn dispose(&self, ticket: GitContextDisposal) -> Result<(), GitRetentionError> {
        self.check_identity(&ticket.owner)?;
        let refs = self.refs(&ticket.commit);
        let mut revision = refs.read().await?;
        if revision.journal.as_deref() != Some(ticket.journal.as_str()) {
            return Err(GitRetentionError::StaleDisposal);
        }
        verify_disposal(&revision.claims)?;
        refs.verify_branch_absent(&ticket.branch).await?;
        next_generation(&mut revision.claims)?;
        revision.claims.disposed = true;
        self.write(&refs, &revision, Some(&ticket.branch)).await
    }

    fn refs<'a>(&'a self, commit: &'a GitCommitId) -> RetentionRefs<'a> {
        RetentionRefs::new(&self.repository, commit)
    }

    fn hold_receipt(
        &self,
        locator: &GitBlobLocator,
        id: &GitContextHoldId,
        generation: u64,
    ) -> GitContextHold {
        GitContextHold {
            owner: self.instance().clone(),
            commit: locator.commit.clone(),
            id: id.clone(),
            generation,
        }
    }

    fn check_checkout(&self, checkout: &CommittedCheckout) -> Result<(), GitRetentionError> {
        if checkout.owner != self.repository {
            return Err(GitRetentionError::OwnerChanged);
        }
        self.check_identity(self.owner.instance())
    }

    fn check_identity(&self, identity: &RepositoryInstanceId) -> Result<(), GitRetentionError> {
        if identity != self.owner.instance()
            || EnrolledRepository::load(&self.home, &self.repository)?.as_ref() != Some(&self.owner)
        {
            return Err(GitRetentionError::OwnerChanged);
        }
        Ok(())
    }

    async fn write(
        &self,
        refs: &RetentionRefs<'_>,
        revision: &Revision,
        branch: Option<&str>,
    ) -> Result<(), GitRetentionError> {
        self.check_identity(self.owner.instance())?;
        refs.write(revision, branch, || self.check_identity(self.instance()))
            .await
    }
}

fn next_generation(claims: &mut Claims) -> Result<u64, GitRetentionError> {
    claims.generation = claims
        .generation
        .checked_add(1)
        .ok_or(GitRetentionError::ClaimsLimit)?;
    Ok(claims.generation)
}

fn matches_locator(hold: &HeldEntry, locator: &GitBlobLocator) -> bool {
    hold.path == locator.path.as_str() && hold.blob == locator.blob.as_str()
}

fn verify_disposal(claims: &Claims) -> Result<(), GitRetentionError> {
    if claims.disposed || claims.generation == 0 {
        return Err(GitRetentionError::InvalidState);
    }
    if !claims.holds.is_empty() {
        return Err(GitRetentionError::LiveHold);
    }
    Ok(())
}
