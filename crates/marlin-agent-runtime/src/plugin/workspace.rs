// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Resolve authorized file capabilities relative to a host-opened worktree.

use std::{
    ffi::{CStr, CString},
    fs::File,
    io::{self, Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    path::{Component, Path},
};

use marlin_agent_protocol::{RuntimeWorkspaceProject, RuntimeWorkspaceProjectId};
use ring::rand::{SecureRandom, SystemRandom};

use super::{
    ActivationId, CapabilityId, EffectId, EffectOutcome, FileCapability, PluginDenial, PluginHost,
};

use super::{evidence::hash, lifecycle::EffectScope};

const MAX_FILE_BYTES: usize = 1024 * 1024;
const DIRECTORY: libc::c_int =
    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;

/// One committed file effect and the value available to the invoking workflow.
#[derive(Debug, Eq, PartialEq)]
pub struct WorkspaceEffect<T> {
    pub id: EffectId,
    pub digest: [u8; 32],
    pub value: T,
}

/// Failure to admit or complete an authorized filesystem operation.
#[derive(Debug)]
pub enum WorkspaceEffectError {
    Admission(PluginDenial),
    Effect {
        id: EffectId,
        outcome: EffectOutcome,
        source: io::Error,
    },
}

/// A trusted host binds the imported project and opens its worktree once.
/// Invocation receives logical capability IDs and uses only registered paths.
pub struct WorkspaceFileBroker {
    project: RuntimeWorkspaceProjectId,
    root: std::sync::Arc<File>,
}

impl WorkspaceFileBroker {
    /// The caller supplies the host-selected project, never a plugin path.
    pub fn open(project: &RuntimeWorkspaceProject) -> io::Result<Self> {
        if !project.root.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "worktree root must be absolute",
            ));
        }
        let root = open_at(libc::AT_FDCWD, &c_path(&project.root)?, DIRECTORY, 0)?;
        Ok(Self {
            project: project.id.clone(),
            root: root.into(),
        })
    }

    pub fn read(
        &self,
        host: &PluginHost<'_>,
        activation: &ActivationId,
        capability: &CapabilityId,
    ) -> Result<WorkspaceEffect<Vec<u8>>, WorkspaceEffectError> {
        let (admission, effect) = host
            .admit_registered_effect(&self.project, activation, capability, |effect| {
                matches!(effect, FileCapability::Read { .. })
            })
            .map_err(WorkspaceEffectError::Admission)?;
        let id = admission.id();
        match self.read_file(effect.path()) {
            Ok(bytes) => {
                let digest = hash(&bytes);
                admission.finish(EffectOutcome::Committed {
                    output_digest: digest,
                });
                Ok(WorkspaceEffect {
                    id,
                    digest,
                    value: bytes,
                })
            }
            Err(source) => {
                let outcome = file_failure(&source);
                admission.finish(outcome);
                Err(WorkspaceEffectError::Effect {
                    id,
                    outcome,
                    source,
                })
            }
        }
    }

    pub fn create(
        &self,
        host: &PluginHost<'_>,
        activation: &ActivationId,
        capability: &CapabilityId,
        bytes: &[u8],
    ) -> Result<WorkspaceEffect<()>, WorkspaceEffectError> {
        self.prepare_create(host, activation, capability)?
            .execute(bytes)
    }

    /// Bind a logical capability for a later executor. Admission happens on execution.
    pub fn prepare_create(
        &self,
        host: &PluginHost<'_>,
        activation: &ActivationId,
        capability: &CapabilityId,
    ) -> Result<WorkspaceCreate, WorkspaceEffectError> {
        let scope = host
            .effect_scope(&self.project, activation)
            .map_err(WorkspaceEffectError::Admission)?;
        Ok(WorkspaceCreate {
            broker: Self {
                project: self.project.clone(),
                root: self.root.clone(),
            },
            scope,
            capability: capability.clone(),
        })
    }

    fn parent(&self, path: &Path) -> io::Result<(File, CString)> {
        let mut directory = self.root.try_clone()?;
        let mut components = path.components().peekable();
        while let Some(Component::Normal(name)) = components.next() {
            let name = c_path(Path::new(name))?;
            if components.peek().is_none() {
                return Ok((directory, name));
            }
            directory = open_at(directory.as_raw_fd(), &name, DIRECTORY, 0)?;
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid registered workspace path",
        ))
    }

    fn read_file(&self, path: &Path) -> io::Result<Vec<u8>> {
        let (parent, name) = self.parent(path)?;
        let file = open_at(
            parent.as_raw_fd(),
            &name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "brief must be a regular file",
            ));
        }
        let mut bytes = Vec::new();
        file.take((MAX_FILE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        check_size(bytes.len())?;
        Ok(bytes)
    }

    fn create_file(&self, path: &Path, bytes: &[u8]) -> Result<(), (EffectOutcome, io::Error)> {
        check_size(bytes.len()).map_err(before_publication)?;
        let (parent, name) = self.parent(path).map_err(before_publication)?;
        let (mut file, temporary) = temporary_file(&parent).map_err(before_publication)?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(before_publication)?;
        let fd = parent.as_raw_fd();
        // SAFETY: both names are valid C strings relative to the same live directory FD.
        let linked = unsafe { libc::linkat(fd, temporary.name.as_ptr(), fd, name.as_ptr(), 0) };
        if linked == -1 {
            return Err(before_publication(io::Error::last_os_error()));
        }
        drop(temporary);
        parent
            .sync_all()
            .map_err(|error| (EffectOutcome::Indeterminate, error))
    }
}

/// A single create operation whose grant and generation are checked at execution.
pub struct WorkspaceCreate {
    broker: WorkspaceFileBroker,
    scope: EffectScope,
    capability: CapabilityId,
}

impl WorkspaceCreate {
    pub fn execute(self, bytes: &[u8]) -> Result<WorkspaceEffect<()>, WorkspaceEffectError> {
        let (admission, effect) = self
            .scope
            .admit(&self.capability, |effect| {
                matches!(effect, FileCapability::Create { .. })
            })
            .map_err(WorkspaceEffectError::Admission)?;
        let id = admission.id();
        match self.broker.create_file(effect.path(), bytes) {
            Ok(()) => {
                let digest = hash(bytes);
                admission.finish(EffectOutcome::Committed {
                    output_digest: digest,
                });
                Ok(WorkspaceEffect {
                    id,
                    digest,
                    value: (),
                })
            }
            Err((outcome, source)) => {
                admission.finish(outcome);
                Err(WorkspaceEffectError::Effect {
                    id,
                    outcome,
                    source,
                })
            }
        }
    }
}

fn check_size(size: usize) -> io::Result<()> {
    if size > MAX_FILE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "workspace file exceeds the effect limit",
        ));
    }
    Ok(())
}

fn before_publication(error: io::Error) -> (EffectOutcome, io::Error) {
    (file_failure(&error), error)
}

fn file_failure(error: &io::Error) -> EffectOutcome {
    match error.kind() {
        io::ErrorKind::AlreadyExists
        | io::ErrorKind::InvalidInput
        | io::ErrorKind::PermissionDenied => EffectOutcome::Denied,
        _ if matches!(error.raw_os_error(), Some(libc::ELOOP | libc::ENOTDIR)) => {
            EffectOutcome::Denied
        }
        _ => EffectOutcome::Failed,
    }
}

fn c_path(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

fn open_at(
    fd: libc::c_int,
    path: &CStr,
    flags: libc::c_int,
    mode: libc::mode_t,
) -> io::Result<File> {
    // SAFETY: path is a valid C string and the caller holds the directory FD.
    let opened = unsafe { libc::openat(fd, path.as_ptr(), flags, libc::c_uint::from(mode)) };
    if opened == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful openat transfers a fresh file descriptor.
    Ok(unsafe { File::from_raw_fd(opened) })
}

struct TemporaryFile<'a> {
    parent: &'a File,
    name: CString,
}

impl Drop for TemporaryFile<'_> {
    fn drop(&mut self) {
        // SAFETY: the directory and C string remain live for this call.
        unsafe { libc::unlinkat(self.parent.as_raw_fd(), self.name.as_ptr(), 0) };
    }
}

fn temporary_file(parent: &File) -> io::Result<(File, TemporaryFile<'_>)> {
    let mut nonce = [0; 16];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| io::Error::other("temporary name entropy"))?;
    let name = CString::new(format!(
        ".marlin-effect-{:032x}",
        u128::from_be_bytes(nonce)
    ))
    .expect("generated file name contains no NUL");
    let file = open_at(
        parent.as_raw_fd(),
        &name,
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        0o600,
    )?;
    Ok((file, TemporaryFile { parent, name }))
}
