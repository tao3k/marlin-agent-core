// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Thread-safe resident index handle for `SearchGatewayService` enforcing:
//! - SM15 cold-open serialization (zero split-brain builds) and canonical workspace isolation
//! - SM9 debounced single-flight disk synchronization with atomic mmap snapshot persistence
//! - Hot-swap `attach(...)` for TUI background warm-up and incremental Helm sync.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use marlin_agent_runtime::RuntimeContext;
use marlin_agent_search_engine::{
    WorkspaceIndexConfig, default_workspace_edb_cache_path, default_workspace_snapshot_path,
    hydrate_workspace_open_edb_org_and_cache, open_or_build_workspace_index_safe,
    save_workspace_snapshot_atomic,
};

use crate::SharedWorkspaceSearchIndex;
use crate::search_scheduler::{
    DebouncedSyncCoordinator, SchedulerCounters, SearchSchedulerMetrics,
};

const DEFAULT_SYNC_DEBOUNCE: Duration = Duration::from_millis(500);

#[derive(Clone)]
struct AttachedWorkspaceIndex {
    workspace_root: Option<PathBuf>,
    index: SharedWorkspaceSearchIndex,
}

fn canonicalize_workspace_root(workspace: &Path) -> PathBuf {
    std::fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf())
}

/// Thread-safe resident index handle supporting pre-warmed indexes, lazy SM15
/// cold-start opening, hot-swap background warm-up attachment, and debounced
/// single-flight disk synchronization for [`super::SearchGatewayService`].
#[derive(Clone)]
pub struct SearchGatewayIndexHandle {
    workspace_root: Option<PathBuf>,
    slot: Arc<RwLock<Option<AttachedWorkspaceIndex>>>,
    open_lock: Arc<Mutex<()>>,
    sync_coordinator: Arc<DebouncedSyncCoordinator>,
    counters: Arc<SchedulerCounters>,
}

impl SearchGatewayIndexHandle {
    /// Wrap an already-built resident [`SharedWorkspaceSearchIndex`] without
    /// background disk synchronization.
    #[must_use]
    pub fn from_index(index: SharedWorkspaceSearchIndex) -> Self {
        Self::with_sync_debounce(None, Some(index), DEFAULT_SYNC_DEBOUNCE)
    }

    /// Wrap an already-built resident index bound to `workspace_root` with
    /// debounced single-flight disk synchronization.
    #[must_use]
    pub fn from_index_with_workspace(
        workspace_root: PathBuf,
        index: SharedWorkspaceSearchIndex,
    ) -> Self {
        Self::with_sync_debounce(Some(workspace_root), Some(index), DEFAULT_SYNC_DEBOUNCE)
    }

    /// Construct a lazily attached handle backed by `workspace_root` on demand.
    #[must_use]
    pub fn lazy(workspace_root: PathBuf) -> Self {
        Self::with_sync_debounce(Some(workspace_root), None, DEFAULT_SYNC_DEBOUNCE)
    }

    /// Construct an unattached handle with no default root (used when callers
    /// supply explicit roots to [`Self::get_or_open`]).
    #[must_use]
    pub fn empty() -> Self {
        Self::with_sync_debounce(None, None, DEFAULT_SYNC_DEBOUNCE)
    }

    /// Construct a handle with an explicit disk-sync debounce window.
    #[must_use]
    pub fn with_sync_debounce(
        workspace_root: Option<PathBuf>,
        index: Option<SharedWorkspaceSearchIndex>,
        debounce: Duration,
    ) -> Self {
        let attached = index.map(|idx| AttachedWorkspaceIndex {
            workspace_root: workspace_root
                .as_deref()
                .map(canonicalize_workspace_root),
            index: idx,
        });
        Self {
            workspace_root,
            slot: Arc::new(RwLock::new(attached)),
            open_lock: Arc::new(Mutex::new(())),
            sync_coordinator: Arc::new(DebouncedSyncCoordinator::new(debounce)),
            counters: Arc::new(SchedulerCounters::default()),
        }
    }

    /// Attach or replace the resident index once background warm-up or sync finishes.
    pub fn attach(&self, index: SharedWorkspaceSearchIndex) {
        let canonical = self
            .workspace_root
            .as_deref()
            .map(canonicalize_workspace_root);
        let mut guard = self
            .slot
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = Some(AttachedWorkspaceIndex {
            workspace_root: canonical,
            index,
        });
    }

    /// Snapshot scheduler concurrency and disk-sync telemetry counters.
    #[must_use]
    pub fn scheduler_metrics(&self) -> SearchSchedulerMetrics {
        self.counters.snapshot()
    }

    pub(crate) fn counters(&self) -> &SchedulerCounters {
        &self.counters
    }

    pub(crate) fn current(&self) -> Option<SharedWorkspaceSearchIndex> {
        self.slot
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|attached| Arc::clone(&attached.index))
    }

    fn current_for_root(&self, canonical: &Path) -> Option<SharedWorkspaceSearchIndex> {
        let guard = self
            .slot
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let existing = guard.as_ref()?;
        existing
            .workspace_root
            .as_deref()
            .is_none_or(|root| root == canonical)
            .then(|| Arc::clone(&existing.index))
    }

    /// Return the attached resident index for `workspace` or lazily open/build a
    /// cached `mmap` snapshot under SM15 double-checked locking.
    #[must_use]
    pub fn get_or_open(&self, workspace: &Path) -> SharedWorkspaceSearchIndex {
        let canonical = canonicalize_workspace_root(workspace);
        if let Some(existing) = self.current_for_root(&canonical) {
            return existing;
        }
        let _open_guard = self
            .open_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(existing) = self.current_for_root(&canonical) {
            return existing;
        }
        let snapshot_path = default_workspace_snapshot_path(&canonical);
        let edb_cache_path = default_workspace_edb_cache_path(&canonical);
        let index_cfg = WorkspaceIndexConfig {
            max_line_matches_per_hit: 32,
            ..WorkspaceIndexConfig::default()
        };
        let mut idx = open_or_build_workspace_index_safe(&canonical, &snapshot_path, index_cfg);
        let _ = hydrate_workspace_open_edb_org_and_cache(&mut idx, &edb_cache_path);
        let opened = Arc::new(RwLock::new(idx));
        let mut guard = self
            .slot
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = Some(AttachedWorkspaceIndex {
            workspace_root: Some(canonical),
            index: Arc::clone(&opened),
        });
        opened
    }

    /// Resolve the attached index or lazily build/open one for the configured workspace root.
    pub(crate) fn resolve_or_open(&self) -> Result<SharedWorkspaceSearchIndex, String> {
        if let Some(existing) = self.current() {
            return Ok(existing);
        }
        let root = self
            .workspace_root
            .as_ref()
            .ok_or_else(|| "resident search index not attached".to_owned())?;
        Ok(self.get_or_open(root))
    }

    /// Incrementally reconcile the attached resident index with on-disk workspace
    /// edits and atomically persist the `mmap` snapshot when files changed.
    #[must_use]
    pub fn sync_from_disk_and_persist(&self, workspace: &Path) -> bool {
        let shared = self.get_or_open(workspace);
        let Ok(mut guard) = shared.write() else {
            return false;
        };
        let receipt = guard.sync_workspace_from_disk();
        let changed = receipt.upserted_files > 0 || receipt.deleted_files > 0;
        let edb_cache_path = default_workspace_edb_cache_path(workspace);
        let _ = hydrate_workspace_open_edb_org_and_cache(&mut guard, &edb_cache_path);
        if changed {
            guard.compact();
            let snapshot_path = default_workspace_snapshot_path(workspace);
            let _ = save_workspace_snapshot_atomic(&guard, &snapshot_path);
        }
        self.sync_coordinator.record_external_sync();
        changed
    }

    /// Non-blocking SM9 debounced single-flight disk synchronization when a
    /// workspace root is configured on this handle.
    pub(crate) async fn ensure_synced_async(&self, context: &RuntimeContext) {
        let Some(workspace) = self.workspace_root.clone() else {
            return;
        };
        let handle = self.clone();
        self.sync_coordinator
            .ensure_synced(context, &self.counters, move || {
                let _ = handle.sync_from_disk_and_persist(&workspace);
            })
            .await;
    }
}
