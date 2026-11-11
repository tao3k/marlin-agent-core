//! Local filesystem source store adapter.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::org::store::{OrgSourceStore, OrgSourceStoreError, OrgSourceStoreResult};

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
        Ok(self.root.join(relative))
    }
}

impl OrgSourceStore for FileSystemOrgSourceStore {
    fn read_document(&self, document: &str) -> OrgSourceStoreResult<Option<String>> {
        let path = self.document_path(document)?;
        match fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(OrgSourceStoreError::new(format!(
                "read document failed: {error}"
            ))),
        }
    }

    fn write_documents(&mut self, documents: BTreeMap<String, String>) -> OrgSourceStoreResult<()> {
        let writes = planned_writes(self, documents)?;
        for write in &writes {
            atomic_write(&write.path, &write.text)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlannedWrite {
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
            let path = store.document_path(&document)?;
            if !paths.insert(path.clone()) {
                return Err(OrgSourceStoreError::new(format!(
                    "multiple document paths resolve to the same lexical path: {document}"
                )));
            }
            Ok(PlannedWrite { path, text })
        })
        .collect()
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
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document");
    path.with_file_name(format!(
        ".{file_name}.marlin-tmp-{}-{suffix}",
        std::process::id()
    ))
}
