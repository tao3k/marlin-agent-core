//! File-backed release status sidecar persistence.

use std::{
    fs,
    path::{Path, PathBuf},
};

use marlin_gerbil_ir::ReleaseTopologySpec;
use marlin_workspace_status::{ReleaseGateReceipt, ReleaseStatus};

use crate::{OrgSourceStoreError, OrgSourceStoreResult};

const DEFAULT_RELEASE_STATUS_PATH: &str = ".marlin/release-status.json";

/// File-backed sidecar for release topology status and gate receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSystemReleaseStatusStore {
    path: PathBuf,
}

impl FileSystemReleaseStatusStore {
    /// Create a release status sidecar under the workspace root.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            path: root.into().join(DEFAULT_RELEASE_STATUS_PATH),
        }
    }

    /// Create a release status store using an explicit status file path.
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Return the release status sidecar path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the persisted release status, if present.
    pub fn read_status(&self) -> OrgSourceStoreResult<Option<ReleaseStatus>> {
        read_status_file(&self.path)
    }

    /// Persist a release status snapshot.
    pub fn write_status(&self, status: &ReleaseStatus) -> OrgSourceStoreResult<()> {
        write_status_file(&self.path, status)
    }

    /// Persist a pending release status from a `Gerbil` release topology artifact.
    pub fn record_release_topology(
        &self,
        topology: &ReleaseTopologySpec,
    ) -> OrgSourceStoreResult<ReleaseStatus> {
        let status = ReleaseStatus::pending_from_topology(topology);
        self.write_status(&status)?;
        Ok(status)
    }

    /// Record execution evidence for one gate in the persisted release status.
    pub fn record_release_gate_receipt(
        &self,
        receipt: ReleaseGateReceipt,
    ) -> OrgSourceStoreResult<bool> {
        match self.read_status()? {
            Some(mut status) => self.write_updated_gate_receipt(&mut status, receipt),
            None => Ok(false),
        }
    }

    fn write_updated_gate_receipt(
        &self,
        status: &mut ReleaseStatus,
        receipt: ReleaseGateReceipt,
    ) -> OrgSourceStoreResult<bool> {
        let recorded = status.record_gate_receipt(receipt);
        if recorded {
            self.write_status(status)?;
        }
        Ok(recorded)
    }
}

fn read_status_file(path: &Path) -> OrgSourceStoreResult<Option<ReleaseStatus>> {
    match fs::read_to_string(path) {
        Ok(text) => parse_status_json(&text).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(OrgSourceStoreError::new(format!(
            "read release status failed: {error}"
        ))),
    }
}

fn parse_status_json(text: &str) -> OrgSourceStoreResult<ReleaseStatus> {
    serde_json::from_str(text)
        .map_err(|error| OrgSourceStoreError::new(format!("parse release status failed: {error}")))
}

fn write_status_file(path: &Path, status: &ReleaseStatus) -> OrgSourceStoreResult<()> {
    let text = serde_json::to_string_pretty(status).map_err(|error| {
        OrgSourceStoreError::new(format!("encode release status failed: {error}"))
    })?;
    atomic_write(path, &text)
}

fn atomic_write(path: &Path, text: &str) -> OrgSourceStoreResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| OrgSourceStoreError::new("release status path has no parent"))?;
    fs::create_dir_all(parent).map_err(|error| {
        OrgSourceStoreError::new(format!("create release status parent failed: {error}"))
    })?;

    let temp = temp_path(path);
    fs::write(&temp, text).map_err(|error| {
        OrgSourceStoreError::new(format!("write release status temp file failed: {error}"))
    })?;
    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        OrgSourceStoreError::new(format!("rename release status temp file failed: {error}"))
    })
}

fn temp_path(path: &Path) -> PathBuf {
    let mut temp = path.to_path_buf();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!("{value}.tmp"))
        .unwrap_or_else(|| "tmp".to_owned());
    temp.set_extension(extension);
    temp
}
