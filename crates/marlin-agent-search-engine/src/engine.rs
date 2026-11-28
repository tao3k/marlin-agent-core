// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Converged Marlin search engine constructor.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use agent_semantic_search::{
    MarlinConvergedSearchEngine, SearchEngineStoragePort, TrigramSnapshotError,
    WorkspaceIndexConfig, WorkspaceSearchIndex,
};
use agent_semantic_topology::OrgDocumentProjectionPort;

static SNAPSHOT_TMP_NONCE: AtomicU64 = AtomicU64::new(0);

fn verify_non_symlink_snapshot_path(snapshot_path: &Path) -> Result<(), TrigramSnapshotError> {
    if snapshot_path.as_os_str().is_empty() {
        return Err(TrigramSnapshotError::Io("empty snapshot path".to_owned()));
    }
    if let Ok(meta) = std::fs::symlink_metadata(snapshot_path)
        && (meta.file_type().is_symlink() || !meta.file_type().is_file())
    {
        return Err(TrigramSnapshotError::Io(
            "snapshot path must not be a symlink or non-regular file".to_owned(),
        ));
    }
    if let Some(parent) = snapshot_path.parent()
        && !parent.as_os_str().is_empty()
        && let Ok(meta) = std::fs::symlink_metadata(parent)
        && (meta.file_type().is_symlink() || !meta.file_type().is_dir())
    {
        return Err(TrigramSnapshotError::Io(
            "snapshot parent directory must not be a symlink".to_owned(),
        ));
    }
    Ok(())
}

/// Atomically persist a resident search snapshot via a sibling temp file + rename
/// so any concurrent or active `mmap` reader on the old inode is never truncated.
pub fn save_workspace_snapshot_atomic(
    index: &WorkspaceSearchIndex,
    snapshot_path: &Path,
) -> Result<(), TrigramSnapshotError> {
    verify_non_symlink_snapshot_path(snapshot_path)?;
    let bytes = index.encode_snapshot_bytes();
    if let Some(parent) = snapshot_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| TrigramSnapshotError::Io(e.to_string()))?;
        verify_non_symlink_snapshot_path(snapshot_path)?;
    }
    let nonce = SNAPSHOT_TMP_NONCE.fetch_add(1, Ordering::Relaxed);
    let tmp_name = format!(
        ".{}.tmp.{}.{nonce}",
        snapshot_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace.asp-index.bin"),
        std::process::id(),
    );
    let tmp_path = snapshot_path.with_file_name(tmp_name);
    let write_res = (|| -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(err) = write_res {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(TrigramSnapshotError::Io(err.to_string()));
    }
    if let Err(err) = verify_non_symlink_snapshot_path(snapshot_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(err);
    }
    if let Err(err) = std::fs::rename(&tmp_path, snapshot_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(TrigramSnapshotError::Io(err.to_string()));
    }
    if let Some(parent) = snapshot_path.parent()
        && !parent.as_os_str().is_empty()
        && let Ok(dir) = File::open(parent)
    {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// Open a cached zero-copy `mmap` snapshot if present, incrementally sync disk
/// changes, compact before rewriting (dropping any active `mmap`), and atomically
/// replace `snapshot_path` so base trigram postings are never corrupted in place.
#[must_use]
pub fn open_or_build_workspace_index_safe(
    root: &Path,
    snapshot_path: &Path,
    config: WorkspaceIndexConfig,
) -> WorkspaceSearchIndex {
    if verify_non_symlink_snapshot_path(snapshot_path).is_ok()
        && std::fs::symlink_metadata(snapshot_path).is_ok_and(|m| m.file_type().is_file())
        && let Ok(mut idx) =
            WorkspaceSearchIndex::load_mmap_snapshot(root.to_path_buf(), snapshot_path, config.clone())
    {
        let receipt = idx.sync_workspace_from_disk();
        if receipt.upserted_files > 0 || receipt.deleted_files > 0 {
            idx.compact();
            let _ = save_workspace_snapshot_atomic(&idx, snapshot_path);
        }
        return idx;
    }
    let idx = WorkspaceSearchIndex::build_with_config(root, config);
    let _ = save_workspace_snapshot_atomic(&idx, snapshot_path);
    idx
}

/// Construct a converged ASP search engine backed by Marlin's Org and Storage ports.
#[must_use]
pub fn marlin_search_engine(
    org_port: Arc<dyn OrgDocumentProjectionPort>,
    storage_port: Arc<dyn SearchEngineStoragePort>,
) -> MarlinConvergedSearchEngine {
    MarlinConvergedSearchEngine::new(org_port, storage_port)
}

/// Deterministic user-cache path for a workspace's zero-copy mmap trigram snapshot.
///
/// Stores snapshots outside the repository working tree (`$XDG_CACHE_HOME/marlin/search/`,
/// `$HOME/.cache/marlin/search/`, or `std::env::temp_dir()/marlin/search/`) keyed by a
/// deterministic 64-bit hash of the canonical workspace path so git worktrees stay clean.
#[must_use]
pub fn default_workspace_snapshot_path(workspace_root: &Path) -> PathBuf {
    let canonical =
        std::fs::canonicalize(workspace_root).unwrap_or_else(|_| workspace_root.to_path_buf());
    let short_hex = fnv1a_64_hex(canonical.to_string_lossy().as_bytes());
    let cache_base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".cache"))
        })
        .unwrap_or_else(std::env::temp_dir);
    cache_base
        .join("marlin")
        .join("search")
        .join(format!("workspace-{short_hex}.asp-index.bin"))
}

/// Deterministic user-cache path for a workspace's BLAKE3 content-hash-verified
/// Open EDB fact cache (`semantic-open-edb.v1`).
#[must_use]
pub fn default_workspace_edb_cache_path(workspace_root: &Path) -> PathBuf {
    let canonical =
        std::fs::canonicalize(workspace_root).unwrap_or_else(|_| workspace_root.to_path_buf());
    let short_hex = fnv1a_64_hex(canonical.to_string_lossy().as_bytes());
    let cache_base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".cache"))
        })
        .unwrap_or_else(std::env::temp_dir);
    cache_base
        .join("marlin")
        .join("search")
        .join(format!("workspace-{short_hex}.open-edb.json"))
}

fn fnv1a_64_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}
