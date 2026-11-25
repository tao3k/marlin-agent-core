// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Converged Marlin search engine constructor.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_semantic_search::{MarlinConvergedSearchEngine, SearchEngineStoragePort};
use agent_semantic_topology::OrgDocumentProjectionPort;

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

fn fnv1a_64_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}
