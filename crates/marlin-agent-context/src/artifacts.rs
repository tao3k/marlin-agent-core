// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Git-derived workspace and durable artifact I/O for the selected Scheme package.

use std::{
    env,
    error::Error,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use gerbil_scheme::{CollectionKind, GerbilRuntime, IdentifierKind};
use marlin_agent_gitops::{GitContext, GitOperations};
use sha2::{Digest, Sha256};

type ArtifactResult<T> = Result<T, Box<dyn Error>>;

/// The Scheme declaration admits the supplied kind before it enters the journal.
pub struct ArtifactKind(String);

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
            return Ok(None);
        }
        let context = GitOperations::context(directory).await?;
        let root = match context.primary_worktree() {
            Some(primary) => primary.as_path().join(".data").join("marlin"),
            None => context.common_directory().join("marlin"),
        };
        Ok(Some(Self { context, root }))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn record_git_patch(
        &self,
        runtime: &GerbilRuntime,
        package: &str,
    ) -> ArtifactResult<ArtifactProjection> {
        let patch = GitOperations::context_patch(&self.context).await?;
        self.record(runtime, package, &ArtifactKind::new("git-patch"), &patch)
    }

    pub fn record(
        &self,
        runtime: &GerbilRuntime,
        package: &str,
        kind: &ArtifactKind,
        bytes: &[u8],
    ) -> ArtifactResult<ArtifactProjection> {
        self.exchange(runtime, package, Some((kind, bytes)))
    }

    pub fn restore(
        &self,
        runtime: &GerbilRuntime,
        package: &str,
    ) -> ArtifactResult<ArtifactProjection> {
        self.exchange(runtime, package, None)
    }

    fn exchange(
        &self,
        runtime: &GerbilRuntime,
        package: &str,
        artifact: Option<(&ArtifactKind, &[u8])>,
    ) -> ArtifactResult<ArtifactProjection> {
        let _lock = self.lock()?;
        let artifact_dir = self.root.join("artifacts");
        ensure_directory(&artifact_dir)?;
        let journal = self.root.join("context.db");
        for name in [
            "context.db",
            "context.db-wal",
            "context.db-shm",
            "context.db-journal",
        ] {
            regular_or_missing(&self.root.join(name))?;
        }
        let journal = runtime
            .string_from_utf8(journal.to_str().ok_or("Context path must be UTF-8")?)?
            .into_object();
        let module = runtime
            .intern_identifier(IdentifierKind::Symbol, ":marlin-agent-core/artifacts/funcs")?;
        let name = runtime.intern_identifier(IdentifierKind::Symbol, "artifacts-exchange")?;
        let procedure = runtime.resolve_package_export(package, &module, &name)?;
        let request = match artifact {
            Some((kind, bytes)) => {
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
                runtime.collection_from_objects(
                    CollectionKind::Vector,
                    &[&action, &journal, &kind, &commit, &worktree, &digest],
                )?
            }
            None => {
                let action = runtime.intern_identifier(IdentifierKind::Symbol, "restore")?;
                runtime.collection_from_objects(CollectionKind::Vector, &[&action, &journal])?
            }
        };
        let response = runtime
            .call_rooted(&procedure, &request.into_object())?
            .into_collection(CollectionKind::Vector)?;
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
        let mut files = Vec::with_capacity(digests.len());
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
            let path = artifact_dir.join(&digest);
            verify_artifact(&path, &digest)?;
            files.push(path);
        }
        let path = self.root.join("context.org");
        atomic_write(&path, org.as_bytes())?;
        Ok(ArtifactProjection { org: path, files })
    }

    fn lock(&self) -> io::Result<File> {
        if let Some(primary) = self.context.primary_worktree() {
            ensure_directory(&primary.as_path().join(".data"))?;
        }
        ensure_directory(&self.root)?;
        let path = self.root.join("context.lock");
        regular_or_missing(&path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        lock.lock()?;
        Ok(lock)
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

fn regular_or_missing(path: &Path) -> io::Result<bool> {
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
