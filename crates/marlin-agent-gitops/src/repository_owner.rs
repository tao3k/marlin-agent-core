// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Durable, local enrollment of a Git common directory in the home store.

use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tempfile::{Builder, NamedTempFile};
use thiserror::Error;

use crate::{GitContext, GitOperations, GitRemoteName, RepositoryRoute, RepositoryRouteError};

#[cfg(test)]
#[path = "../tests/unit/repository_owner.rs"]
mod tests;

const OWNER_REF: &str = "marlin-owner-ref";
const OWNER_RECORD: &str = "owner.json";
const INSTANCE_PREFIX: &str = "repo-";
const INSTANCE_RANDOM_LENGTH: usize = 24;
const MAX_OWNER_BYTES: u64 = 16 * 1024;

/// The fresh identity of a local repository instance, shared by its worktrees.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryInstanceId(String);

impl RepositoryInstanceId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_path(path: &Path) -> Result<Self, RepositoryEnrollmentError> {
        let name = path.file_name().and_then(|name| name.to_str());
        let valid = name.and_then(|name| name.strip_prefix(INSTANCE_PREFIX));
        match (name, valid) {
            (Some(name), Some(token))
                if token.len() == INSTANCE_RANDOM_LENGTH
                    && token.bytes().all(|byte| byte.is_ascii_alphanumeric()) =>
            {
                Ok(Self(name.into()))
            }
            _ => Err(RepositoryEnrollmentError::InvalidBinding),
        }
    }
}

/// Verified owner and its custody directory within the user's repository store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnrolledRepository {
    instance: RepositoryInstanceId,
    root: PathBuf,
}

#[derive(Debug, Error)]
pub enum RepositoryEnrollmentError {
    #[error(transparent)]
    Route(#[from] RepositoryRouteError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("the repository enrollment is malformed")]
    InvalidBinding,
    #[error("the Git common directory has moved or been replaced; explicit rebinding is required")]
    DirectoryChanged,
    #[error("the selected remote differs from the enrolled route")]
    RouteChanged,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnerRef {
    /// Relative to the repository store. The clone-local reference is only a locator.
    path: PathBuf,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnerRecord {
    location: PathBuf,
    directory: DirectoryIdentity,
    instance: String,
}

#[derive(Deserialize, Serialize, Eq, PartialEq)]
struct DirectoryIdentity {
    device: u64,
    inode: u64,
}

impl DirectoryIdentity {
    fn observe(path: &Path) -> Result<Self, RepositoryEnrollmentError> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_dir() || !path.is_absolute() || path.canonicalize()? != path {
            return Err(RepositoryEnrollmentError::DirectoryChanged);
        }
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }
}

impl EnrolledRepository {
    pub fn instance(&self) -> &RepositoryInstanceId {
        &self.instance
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read and verify an existing enrollment without creating any storage.
    pub fn load(
        home: &Path,
        common_directory: &Path,
    ) -> Result<Option<Self>, RepositoryEnrollmentError> {
        let Some(reference): Option<OwnerRef> = read_record(&common_directory.join(OWNER_REF))?
        else {
            return Ok(None);
        };
        let directory = DirectoryIdentity::observe(common_directory)?;
        let root = descend(store(home)?, &reference.path, false)?;
        let instance = RepositoryInstanceId::from_path(&reference.path)?;
        let record: OwnerRecord = read_record(&root.join(OWNER_RECORD))?
            .ok_or(RepositoryEnrollmentError::InvalidBinding)?;
        if record.location != common_directory || record.directory != directory {
            return Err(RepositoryEnrollmentError::DirectoryChanged);
        }
        if record.instance != instance.as_str() {
            return Err(RepositoryEnrollmentError::InvalidBinding);
        }
        Ok(Some(Self { instance, root }))
    }

    /// Explicitly enroll an observed checkout. Concurrent callers reuse the winning owner.
    pub async fn enroll(
        home: &Path,
        context: &GitContext,
        preferred: Option<&GitRemoteName>,
    ) -> Result<Self, RepositoryEnrollmentError> {
        let common = context.common_directory();
        let current = Self::load(home, common)?;
        if preferred.is_none()
            && let Some(owner) = current
        {
            return Ok(owner);
        }
        let directory = DirectoryIdentity::observe(common)?;
        let route = GitOperations::repository_route(context, preferred).await?;
        if GitOperations::repository_owner(context.worktree().as_path())
            .await
            .map_err(RepositoryRouteError::from)?
            != common
            || DirectoryIdentity::observe(common)? != directory
        {
            return Err(RepositoryEnrollmentError::DirectoryChanged);
        }
        let route = match &route {
            RepositoryRoute::Local => Path::new("_local"),
            RepositoryRoute::Network(route) => route.as_path(),
        };
        let store = store(home)?;
        if let Some(owner) = current {
            return owner.require_route(&store.join(route));
        }
        let parent = descend(store.clone(), route, true)?;
        let owner = Builder::new()
            .prefix(INSTANCE_PREFIX)
            .rand_bytes(INSTANCE_RANDOM_LENGTH)
            .tempdir_in(&parent)?;
        let instance = RepositoryInstanceId::from_path(owner.path())?;
        let record = OwnerRecord {
            location: common.into(),
            directory,
            instance: instance.0.clone(),
        };
        publish_record(&owner.path().join(OWNER_RECORD), &record)?;
        File::open(owner.path())?.sync_all()?;
        File::open(&parent)?.sync_all()?;
        let reference = OwnerRef {
            path: owner
                .path()
                .strip_prefix(&store)
                .map_err(|_| RepositoryEnrollmentError::InvalidBinding)?
                .into(),
        };
        if record.directory != DirectoryIdentity::observe(common)? {
            return Err(RepositoryEnrollmentError::DirectoryChanged);
        }
        match publish_record(&common.join(OWNER_REF), &reference) {
            Ok(()) => {
                let root = owner.keep();
                File::open(common)?.sync_all()?;
                Ok(Self { instance, root })
            }
            Err(RepositoryEnrollmentError::Io(error))
                if error.kind() == io::ErrorKind::AlreadyExists =>
            {
                Self::load(home, common)?
                    .ok_or(RepositoryEnrollmentError::InvalidBinding)?
                    .require_route(&parent)
            }
            Err(error) => Err(error),
        }
    }

    fn require_route(self, route: &Path) -> Result<Self, RepositoryEnrollmentError> {
        if self.root.parent() == Some(route) {
            Ok(self)
        } else {
            Err(RepositoryEnrollmentError::RouteChanged)
        }
    }
}

fn store(home: &Path) -> Result<PathBuf, RepositoryEnrollmentError> {
    if !home.is_absolute() {
        return Err(RepositoryEnrollmentError::InvalidBinding);
    }
    Ok(home.canonicalize()?.join("marlin-ai").join("repos"))
}

fn descend(
    store: PathBuf,
    relative: &Path,
    create: bool,
) -> Result<PathBuf, RepositoryEnrollmentError> {
    let home = store
        .parent()
        .and_then(Path::parent)
        .ok_or(RepositoryEnrollmentError::InvalidBinding)?;
    let path = store.join(relative);
    if !relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(RepositoryEnrollmentError::InvalidBinding);
    }
    let mut directory = home.to_owned();
    let suffix = path
        .strip_prefix(home)
        .map_err(|_| RepositoryEnrollmentError::InvalidBinding)?;
    for component in suffix.components() {
        let parent = directory.clone();
        directory.push(component);
        if create {
            match fs::create_dir(&directory) {
                Ok(()) => File::open(parent)?.sync_all()?,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
        }
        if !fs::symlink_metadata(&directory)?.is_dir() {
            return Err(RepositoryEnrollmentError::InvalidBinding);
        }
    }
    Ok(directory)
}

fn read_record<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, RepositoryEnrollmentError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_OWNER_BYTES => {}
        Ok(_) => return Err(RepositoryEnrollmentError::InvalidBinding),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_OWNER_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_OWNER_BYTES {
        return Err(RepositoryEnrollmentError::InvalidBinding);
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| RepositoryEnrollmentError::InvalidBinding)
}

fn publish_record(path: &Path, value: &impl Serialize) -> Result<(), RepositoryEnrollmentError> {
    let bytes = serde_json::to_vec(value).map_err(|_| RepositoryEnrollmentError::InvalidBinding)?;
    if bytes.len() as u64 > MAX_OWNER_BYTES {
        return Err(RepositoryEnrollmentError::InvalidBinding);
    }
    let parent = path
        .parent()
        .ok_or(RepositoryEnrollmentError::InvalidBinding)?;
    let mut staged = NamedTempFile::new_in(parent)?;
    staged.write_all(&bytes)?;
    staged.as_file().sync_all()?;
    staged
        .persist_noclobber(path)
        .map_err(|error| error.error)?;
    Ok(())
}
