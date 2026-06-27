//! Typed file-write side effects with `LoopProgram` sandbox evidence.

use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

use crate::LoopProgramToolProcessProjectionReceipt;

use super::LoopProgramFileWriteSideEffectStatus;

/// Sandbox used to resolve file-write side effects under a workspace root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramFileSandbox {
    root: PathBuf,
    allowed_relative_paths: Box<[PathBuf]>,
}

impl LoopProgramFileSandbox {
    pub fn deny_all() -> Self {
        Self {
            root: PathBuf::new(),
            allowed_relative_paths: Box::new([]),
        }
    }

    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            allowed_relative_paths: Box::new([]),
        }
    }

    pub fn with_allowed_relative_paths<I, P>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.allowed_relative_paths = paths.into_iter().map(Into::into).collect();
        self
    }

    pub fn root(&self) -> &Path {
        self.root.as_path()
    }

    pub fn allowed_relative_paths(&self) -> &[PathBuf] {
        &self.allowed_relative_paths
    }

    pub(crate) fn resolve(&self, relative_path: &Path) -> Result<(PathBuf, PathBuf), String> {
        let normalized = normalize_relative_path(relative_path)?;
        let allowed = self.allowed_relative_paths.iter().any(|allowed_path| {
            match normalize_relative_path(allowed_path) {
                Ok(allowed_path) => allowed_path == normalized,
                Err(_) => false,
            }
        });
        if !allowed {
            return Err(format!(
                "loop_program.file_write.sandbox_denied:{}",
                normalized.display()
            ));
        }
        Ok((self.root.join(&normalized), normalized))
    }
}

/// File-write request resolved from a projected tool intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramFileWriteRequest {
    pub projection: LoopProgramToolProcessProjectionReceipt,
    pub relative_path: PathBuf,
    pub contents: Box<[u8]>,
}

impl LoopProgramFileWriteRequest {
    pub fn new(
        projection: LoopProgramToolProcessProjectionReceipt,
        relative_path: impl Into<PathBuf>,
        contents: impl Into<Box<[u8]>>,
    ) -> Self {
        Self {
            projection,
            relative_path: relative_path.into(),
            contents: contents.into(),
        }
    }
}

/// Receipt emitted after a sandbox-authorized file write.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramFileWriteReceipt {
    pub projection: LoopProgramToolProcessProjectionReceipt,
    pub relative_path: PathBuf,
    pub path: PathBuf,
    pub before_hash: Option<String>,
    pub after_hash: String,
    pub bytes_written: usize,
}

/// File-write side-effect receipt with sandbox decision evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramFileWriteSideEffectReceipt {
    pub projection: LoopProgramToolProcessProjectionReceipt,
    pub relative_path: PathBuf,
    pub status: LoopProgramFileWriteSideEffectStatus,
    pub write_receipt: Option<LoopProgramFileWriteReceipt>,
    pub diagnostic: Option<String>,
}

impl LoopProgramFileWriteSideEffectReceipt {
    pub(crate) fn completed(write_receipt: LoopProgramFileWriteReceipt) -> Self {
        Self {
            projection: write_receipt.projection.clone(),
            relative_path: write_receipt.relative_path.clone(),
            status: LoopProgramFileWriteSideEffectStatus::Completed,
            diagnostic: None,
            write_receipt: Some(write_receipt),
        }
    }

    pub(crate) fn denied(request: LoopProgramFileWriteRequest, diagnostic: String) -> Self {
        Self {
            projection: request.projection,
            relative_path: request.relative_path,
            status: LoopProgramFileWriteSideEffectStatus::Denied,
            write_receipt: None,
            diagnostic: Some(diagnostic),
        }
    }

    pub(crate) fn failed(request: LoopProgramFileWriteRequest, error: io::Error) -> Self {
        Self {
            projection: request.projection,
            relative_path: request.relative_path,
            status: LoopProgramFileWriteSideEffectStatus::Failed,
            write_receipt: None,
            diagnostic: Some(format!("loop_program.file_write.failed:{error}")),
        }
    }
}

/// Resolves a typed tool projection into an explicit file-write request.
pub trait LoopProgramFileWriteResolver: Send + Sync + 'static {
    fn resolve(
        &self,
        projection: &LoopProgramToolProcessProjectionReceipt,
    ) -> Option<LoopProgramFileWriteRequest>;
}

/// Static file-write resolver for tests, smoke fixtures, and TOML-backed configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StaticLoopProgramFileWriteResolver {
    templates: Box<[LoopProgramFileWriteTemplate]>,
}

impl StaticLoopProgramFileWriteResolver {
    pub fn new(templates: impl Into<Box<[LoopProgramFileWriteTemplate]>>) -> Self {
        Self {
            templates: templates.into(),
        }
    }
}

impl LoopProgramFileWriteResolver for StaticLoopProgramFileWriteResolver {
    fn resolve(
        &self,
        projection: &LoopProgramToolProcessProjectionReceipt,
    ) -> Option<LoopProgramFileWriteRequest> {
        self.templates
            .iter()
            .find(|template| template.matches(projection))
            .map(|template| template.write_request(projection.clone()))
    }
}

/// One static mapping from a projected command observation to a file write.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramFileWriteTemplate {
    command_kind: String,
    argv: Box<[String]>,
    relative_path: PathBuf,
    contents: Box<[u8]>,
}

impl LoopProgramFileWriteTemplate {
    pub fn new<I, S>(
        command_kind: impl Into<String>,
        argv: I,
        relative_path: impl Into<PathBuf>,
        contents: impl Into<Box<[u8]>>,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            command_kind: command_kind.into(),
            argv: argv.into_iter().map(Into::into).collect(),
            relative_path: relative_path.into(),
            contents: contents.into(),
        }
    }

    fn matches(&self, projection: &LoopProgramToolProcessProjectionReceipt) -> bool {
        projection.command.command_kind.as_str() == self.command_kind.as_str()
            && projection.command.argv.as_slice() == self.argv.as_ref()
    }

    fn write_request(
        &self,
        projection: LoopProgramToolProcessProjectionReceipt,
    ) -> LoopProgramFileWriteRequest {
        LoopProgramFileWriteRequest::new(
            projection,
            self.relative_path.clone(),
            self.contents.clone(),
        )
    }
}

pub(crate) fn read_existing_digest(path: &Path) -> Result<Option<String>, io::Error> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(stable_bytes_digest(&bytes))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn stable_bytes_digest(bytes: &[u8]) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;

    let mut value = FNV_OFFSET;
    for byte in bytes {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(FNV_PRIME);
    }
    format!("fnv1a64:{value:016x}")
}

fn normalize_relative_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Err(format!(
            "loop_program.file_write.absolute_path_denied:{}",
            path.display()
        ));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "loop_program.file_write.relative_path_denied:{}",
                    path.display()
                ));
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err("loop_program.file_write.empty_path_denied".to_owned());
    }
    Ok(normalized)
}
