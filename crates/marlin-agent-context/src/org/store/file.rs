// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Local filesystem source store adapter.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::org::store::{OrgSourceStore, OrgSourceStoreError, OrgSourceStoreResult};

/// Maximum document byte size read from `FileSystemOrgSourceStore` (16 MiB).
pub const MAX_ORG_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

static TEMP_FILE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Root-scoped filesystem implementation of the source store contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSystemOrgSourceStore {
    root: PathBuf,
}

impl FileSystemOrgSourceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn document_path(&self, document: &str) -> OrgSourceStoreResult<PathBuf> {
        let relative = validated_relative_path(document)?;
        verify_confined_no_symlinks(&self.root, &relative)
    }
}

impl OrgSourceStore for FileSystemOrgSourceStore {
    fn read_document(&self, document: &str) -> OrgSourceStoreResult<Option<String>> {
        let path = self.document_path(document)?;
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(OrgSourceStoreError::new(format!(
                    "read document failed: {error}"
                )));
            }
        };
        let metadata = file.metadata().map_err(|error| {
            OrgSourceStoreError::new(format!("read document metadata failed: {error}"))
        })?;
        if !metadata.is_file() {
            return Err(OrgSourceStoreError::new(
                "document path must point to a regular file",
            ));
        }
        let mut text = String::new();
        file.take((MAX_ORG_DOCUMENT_BYTES + 1) as u64)
            .read_to_string(&mut text)
            .map_err(|error| OrgSourceStoreError::new(format!("read document failed: {error}")))?;
        if text.len() > MAX_ORG_DOCUMENT_BYTES {
            return Err(OrgSourceStoreError::new(format!(
                "document exceeds maximum allowed size of {MAX_ORG_DOCUMENT_BYTES} bytes"
            )));
        }
        Ok(Some(text))
    }

    fn write_documents(&mut self, documents: BTreeMap<String, String>) -> OrgSourceStoreResult<()> {
        let writes = planned_writes(self, documents)?;
        for write in &writes {
            verify_confined_no_symlinks(&self.root, &write.relative)?;
            atomic_write(&write.path, &write.text)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlannedWrite {
    relative: PathBuf,
    path: PathBuf,
    text: String,
}

fn planned_writes(
    store: &FileSystemOrgSourceStore,
    documents: BTreeMap<String, String>,
) -> OrgSourceStoreResult<Vec<PlannedWrite>> {
    let mut paths = BTreeSet::new();
    documents
        .into_iter()
        .map(|(document, text)| {
            let relative = validated_relative_path(&document)?;
            let path = verify_confined_no_symlinks(&store.root, &relative)?;
            if !paths.insert(path.clone()) {
                return Err(OrgSourceStoreError::new(format!(
                    "multiple document paths resolve to the same lexical path: {document}"
                )));
            }
            Ok(PlannedWrite {
                relative,
                path,
                text,
            })
        })
        .collect()
}

fn verify_confined_no_symlinks(root: &Path, relative: &Path) -> OrgSourceStoreResult<PathBuf> {
    match fs::symlink_metadata(root) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
            return Err(OrgSourceStoreError::new(
                "store root must be a real directory, not a symlink",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(OrgSourceStoreError::new(format!(
                "inspect store root failed: {error}"
            )));
        }
    }

    let canonical_root = fs::canonicalize(root).ok();
    let mut current = root.to_path_buf();
    let mut stop_fs_walk = false;
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        let is_last = components.peek().is_none();
        match component {
            Component::Normal(part) => {
                current.push(part);
                if stop_fs_walk {
                    continue;
                }
                match fs::symlink_metadata(&current) {
                    Ok(meta) => {
                        if meta.file_type().is_symlink() {
                            return Err(OrgSourceStoreError::new(format!(
                                "document path traverses symlink outside store root: {}",
                                relative.display()
                            )));
                        }
                        if !is_last && !meta.is_dir() {
                            stop_fs_walk = true;
                        } else if !is_last
                            && meta.is_dir()
                            && let Some(canon_root) = &canonical_root
                            && let Ok(canon_current) = fs::canonicalize(&current)
                            && !canon_current.starts_with(canon_root)
                        {
                            return Err(OrgSourceStoreError::new(format!(
                                "document path escapes canonical store root: {}",
                                relative.display()
                            )));
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        stop_fs_walk = true;
                    }
                    Err(error) => {
                        return Err(OrgSourceStoreError::new(format!(
                            "inspect document path failed: {error}"
                        )));
                    }
                }
            }
            Component::CurDir => {}
            _ => {
                return Err(OrgSourceStoreError::new(format!(
                    "document path escapes store root: {}",
                    relative.display()
                )));
            }
        }
    }

    Ok(current)
}

fn validated_relative_path(document: &str) -> OrgSourceStoreResult<PathBuf> {
    let path = Path::new(document);
    if path.is_absolute() {
        return Err(OrgSourceStoreError::new(format!(
            "absolute document path is not allowed: {document}"
        )));
    }

    let relative = path
        .components()
        .try_fold(PathBuf::new(), |mut relative, component| match component {
            Component::Normal(part) => {
                relative.push(part);
                Ok(relative)
            }
            Component::CurDir => Ok(relative),
            _ => Err(OrgSourceStoreError::new(format!(
                "document path escapes store root: {document}"
            ))),
        })?;

    if relative.as_os_str().is_empty() {
        return Err(OrgSourceStoreError::new("document path is empty"));
    }

    Ok(relative)
}

fn atomic_write(path: &Path, text: &str) -> OrgSourceStoreResult<()> {
    atomic_write_with_parent_sync(path, text, sync_parent)
}

fn atomic_write_with_parent_sync(
    path: &Path,
    text: &str,
    sync: impl FnOnce(&Path) -> std::io::Result<()>,
) -> OrgSourceStoreResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| OrgSourceStoreError::new("document path has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|error| OrgSourceStoreError::new(format!("create parent failed: {error}")))?;

    let temp = temp_path(path);
    write_new_temp(&temp, text)?;
    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        OrgSourceStoreError::new(format!("rename temp file failed: {error}"))
    })?;
    sync(parent).map_err(|error| {
        OrgSourceStoreError::new(format!("sync parent after rename failed: {error}"))
    })
}

fn write_new_temp(path: &Path, text: &str) -> OrgSourceStoreResult<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| OrgSourceStoreError::new(format!("create temp file failed: {error}")))?;
    if let Err(error) = file
        .write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
    {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(OrgSourceStoreError::new(format!(
            "write or sync temp file failed: {error}"
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> std::io::Result<()> {
    fs::File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> std::io::Result<()> {
    // Directory sync is not supplied by the portable standard library.
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/unit/org_store/temporary_file.rs"]
mod tests;

fn temp_path(path: &Path) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let seq = TEMP_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document");
    path.with_file_name(format!(
        ".{file_name}.marlin-tmp-{}-{suffix}-{seq}",
        std::process::id()
    ))
}
