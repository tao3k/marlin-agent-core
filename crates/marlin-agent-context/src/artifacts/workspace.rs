// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Git-derived workspace and durable artifact I/O.

use std::{
    env,
    error::Error,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use gerbil_scheme::{CollectionKind, GerbilRuntime, IdentifierKind, RootedSchemeObject};
use marlin_agent_gitops::{CommittedCheckout, GitContext, GitOperations};
#[cfg(unix)]
use marlin_agent_gitops::{EnrolledRepository, GitReadError};
use sha2::{Digest, Sha256};

pub(super) type ArtifactResult<T> = Result<T, Box<dyn Error>>;

/// The Scheme declaration admits the supplied kind before it enters the journal.
#[cfg(feature = "artifacts")]
pub struct ArtifactKind(String);

#[cfg(feature = "artifacts")]
impl ArtifactKind {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// The shared Org projection and the verified files it refers to.
pub struct ArtifactProjection {
    pub org: PathBuf,
    pub files: Vec<PathBuf>,
}

/// Shared storage for linked worktrees. Each record retains the active Git revision.
pub struct WorkspaceArtifacts {
    context: GitContext,
    root: PathBuf,
    #[cfg(unix)]
    enrollment: Option<(PathBuf, EnrolledRepository)>,
}

impl WorkspaceArtifacts {
    pub async fn discover(directory: &Path) -> ArtifactResult<Option<Self>> {
        let enabled = env::var_os("MARLIN_AGENT_CORE_DEV");
        Self::discover_with_mode(directory, enabled.as_deref()).await
    }

    /// Accept the flag from an already authorized worktree environment snapshot.
    pub async fn discover_with_mode(
        directory: &Path,
        mode: Option<&OsStr>,
    ) -> ArtifactResult<Option<Self>> {
        if mode != Some(OsStr::new("1")) {
            #[cfg(unix)]
            return Self::open_home(directory).await;
            #[cfg(not(unix))]
            return Ok(None);
        }
        let context = GitOperations::context(directory).await?;
        let root = match context.primary_worktree() {
            Some(primary) => primary.as_path().join(".data").join("marlin"),
            None => context.common_directory().join("marlin"),
        };
        Ok(Some(Self {
            context,
            root,
            #[cfg(unix)]
            enrollment: None,
        }))
    }

    #[cfg(unix)]
    async fn open_home(directory: &Path) -> ArtifactResult<Option<Self>> {
        let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
            return Ok(None);
        };
        let common = match GitOperations::repository_owner(directory).await {
            Ok(common) => common,
            Err(GitReadError::Failed(Some(128))) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let Some(owner) = EnrolledRepository::load(&home, &common)? else {
            return Ok(None);
        };
        let context = GitOperations::context(directory).await?;
        if context.common_directory() != common {
            return Err("repository changed while opening Context Artifacts".into());
        }
        Ok(Some(Self {
            root: owner.root().to_owned(),
            context,
            enrollment: Some((home, owner)),
        }))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Compare the observed artifact producer with the admitted checkout.
    pub fn matches_checkout(&self, checkout: &CommittedCheckout) -> bool {
        self.context.common_directory() == checkout.owner
            && self.context.worktree() == &checkout.root
            && self.context.commit() == &checkout.commit
    }

    #[cfg(feature = "artifacts")]
    pub async fn record_git_patch(
        &self,
        runtime: &GerbilRuntime,
        package: &str,
    ) -> ArtifactResult<ArtifactProjection> {
        let patch = GitOperations::context_patch(&self.context).await?;
        self.record(runtime, package, &ArtifactKind::new("git-patch"), &patch)
    }

    #[cfg(feature = "artifacts")]
    pub fn record(
        &self,
        runtime: &GerbilRuntime,
        package: &str,
        kind: &ArtifactKind,
        bytes: &[u8],
    ) -> ArtifactResult<ArtifactProjection> {
        self.exchange(runtime, package, Some((kind, bytes)))
    }

    #[cfg(feature = "artifacts")]
    pub fn restore(
        &self,
        runtime: &GerbilRuntime,
        package: &str,
    ) -> ArtifactResult<ArtifactProjection> {
        self.exchange(runtime, package, None)
    }

    #[cfg(feature = "artifacts")]
    fn exchange(
        &self,
        runtime: &GerbilRuntime,
        package: &str,
        artifact: Option<(&ArtifactKind, &[u8])>,
    ) -> ArtifactResult<ArtifactProjection> {
        let lock = self.open_lock()?;
        lock.lock()?;
        self.check_enrollment()?;
        let artifact_dir = self.root.join("artifacts");
        ensure_directory(&artifact_dir)?;
        let journal = self.checked_journal()?;
        let module = runtime
            .intern_identifier(IdentifierKind::Symbol, ":marlin-agent-core/artifacts/funcs")?;
        let name = runtime.intern_identifier(IdentifierKind::Symbol, "artifacts-exchange")?;
        let procedure = runtime.resolve_package_export(package, &module, &name)?;
        let request = match artifact {
            Some((kind, bytes)) => {
                let journal = journal_value(runtime, &journal)?;
                let digest = format!("{:x}", Sha256::digest(bytes));
                let path = artifact_dir.join(&digest);
                if regular_or_missing(&path)? {
                    verify_artifact(&path, &digest)?;
                } else {
                    atomic_write(&path, bytes)?;
                }
                let action = runtime.intern_identifier(IdentifierKind::Symbol, "record")?;
                let kind = runtime.intern_identifier(IdentifierKind::Symbol, &kind.0)?;
                let commit = runtime
                    .string_from_utf8(self.context.commit().as_str())?
                    .into_object();
                let worktree = self
                    .context
                    .worktree()
                    .as_path()
                    .to_str()
                    .ok_or("worktree path must be UTF-8")?;
                let worktree = runtime.string_from_utf8(worktree)?.into_object();
                let digest = runtime.string_from_utf8(&digest)?.into_object();
                runtime
                    .collection_from_objects(
                        CollectionKind::Vector,
                        &[&action, &journal, &kind, &commit, &worktree, &digest],
                    )?
                    .into_object()
            }
            None => restore_request(runtime, &journal)?,
        };
        let response = runtime.call_rooted(&procedure, &request)?;
        let contents = ProjectionContents::decode(response)?;
        let files = contents.verify_files(&self.root)?;
        self.publish_org(&contents.org, files)
    }

    pub(super) fn checked_journal(&self) -> io::Result<PathBuf> {
        let journal = self.root.join("context.db");
        for name in [
            "context.db",
            "context.db-wal",
            "context.db-shm",
            "context.db-journal",
        ] {
            regular_or_missing(&self.root.join(name))?;
        }
        Ok(journal)
    }

    pub(super) fn publish_org(
        &self,
        org: &str,
        files: Vec<PathBuf>,
    ) -> ArtifactResult<ArtifactProjection> {
        self.check_enrollment()?;
        let path = self.root.join("context.org");
        atomic_write(&path, org.as_bytes())?;
        Ok(ArtifactProjection { org: path, files })
    }

    pub(super) fn check_enrollment(&self) -> io::Result<()> {
        #[cfg(unix)]
        if let Some((home, owner)) = &self.enrollment {
            let current = EnrolledRepository::load(home, self.context.common_directory())
                .map_err(io::Error::other)?;
            if current.as_ref() != Some(owner) {
                return Err(io::Error::other("repository enrollment has changed"));
            }
        }
        Ok(())
    }

    pub(super) fn open_lock(&self) -> io::Result<File> {
        self.check_enrollment()?;
        #[cfg(unix)]
        let development = self.enrollment.is_none();
        #[cfg(not(unix))]
        let development = true;
        if development && let Some(primary) = self.context.primary_worktree() {
            ensure_directory(&primary.as_path().join(".data"))?;
        }
        if development {
            ensure_directory(&self.root)?;
        }
        let path = self.root.join("context.lock");
        regular_or_missing(&path)?;
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
    }
}

fn journal_value<'vm>(
    runtime: &'vm GerbilRuntime,
    journal: &Path,
) -> ArtifactResult<RootedSchemeObject<'vm>> {
    let path = journal.to_str().ok_or("Context path must be UTF-8")?;
    Ok(runtime.string_from_utf8(path)?.into_object())
}

pub(super) fn restore_request<'vm>(
    runtime: &'vm GerbilRuntime,
    journal: &Path,
) -> ArtifactResult<RootedSchemeObject<'vm>> {
    let journal = journal_value(runtime, journal)?;
    let action = runtime.intern_identifier(IdentifierKind::Symbol, "restore")?;
    Ok(runtime
        .collection_from_objects(CollectionKind::Vector, &[&action, &journal])?
        .into_object())
}

pub(super) struct ProjectionContents {
    pub(super) org: String,
    digests: Vec<String>,
}

impl ProjectionContents {
    pub(super) fn decode(response: RootedSchemeObject<'_>) -> ArtifactResult<Self> {
        let response = response.into_collection(CollectionKind::Vector)?;
        if response.len() != 2 {
            return Err("invalid artifact projection".into());
        }
        let org = response
            .element(0)?
            .into_string()?
            .to_string()
            .into_result()?;
        let digests = response
            .element(1)?
            .into_collection(CollectionKind::Vector)?;
        let mut decoded = Vec::with_capacity(digests.len());
        for i in 0..digests.len() {
            let digest = digests
                .element(i)?
                .into_string()?
                .to_string()
                .into_result()?;
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                return Err("invalid artifact digest".into());
            }
            decoded.push(digest);
        }
        Ok(Self {
            org,
            digests: decoded,
        })
    }

    pub(super) fn verify_files(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        let directory = root.join("artifacts");
        if !fs::symlink_metadata(&directory)?.is_dir() {
            return Err(io::Error::other("expected a real artifact directory"));
        }
        self.digests
            .iter()
            .map(|digest| {
                let path = directory.join(digest);
                verify_artifact(&path, digest)?;
                Ok(path)
            })
            .collect()
    }
}

fn ensure_directory(path: &Path) -> io::Result<()> {
    match fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if fs::symlink_metadata(path)?.is_dir() {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "expected a real Context directory",
                ))
            }
        }
        Err(error) => Err(error),
    }
}

pub(super) fn regular_or_missing(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected a regular Context file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    regular_or_missing(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("missing Context directory"))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn verify_artifact(path: &Path, digest: &str) -> io::Result<()> {
    if !regular_or_missing(path)? {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "committed Context artifact is missing",
        ));
    }
    let mut hasher = Sha256::new();
    io::copy(&mut File::open(path)?, &mut hasher)?;
    if format!("{:x}", hasher.finalize()) != digest {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Context artifact digest mismatch",
        ));
    }
    Ok(())
}
