//! Local filesystem source store adapter.

use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{OrgSourceStore, OrgSourceStoreError, OrgSourceStoreResult};

/// Root-scoped filesystem implementation of the source store contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSystemOrgSourceStore {
    root: PathBuf,
    clean: bool,
}

impl FileSystemOrgSourceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            clean: true,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn set_clean(&mut self, clean: bool) {
        self.clean = clean;
    }

    fn document_path(&self, document: &str) -> OrgSourceStoreResult<PathBuf> {
        let relative = validated_relative_path(document)?;
        Ok(self.root.join(relative))
    }
}

impl OrgSourceStore for FileSystemOrgSourceStore {
    fn read_document(&self, document: &str) -> Option<String> {
        self.document_path(document)
            .ok()
            .and_then(|path| fs::read_to_string(path).ok())
    }

    fn write_documents(&mut self, documents: BTreeMap<String, String>) -> OrgSourceStoreResult<()> {
        let writes = planned_writes(self, documents)?;
        writes
            .iter()
            .try_for_each(|write| atomic_write(&write.path, &write.text))?;
        self.clean = false;
        Ok(())
    }

    fn is_clean(&self) -> bool {
        self.clean
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
    documents
        .into_iter()
        .map(|(document, text)| {
            store
                .document_path(&document)
                .map(|path| PlannedWrite { path, text })
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
    let parent = path
        .parent()
        .ok_or_else(|| OrgSourceStoreError::new("document path has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|error| OrgSourceStoreError::new(format!("create parent failed: {error}")))?;

    let temp = temp_path(path);
    fs::write(&temp, text)
        .map_err(|error| OrgSourceStoreError::new(format!("write temp file failed: {error}")))?;
    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        OrgSourceStoreError::new(format!("rename temp file failed: {error}"))
    })
}

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
