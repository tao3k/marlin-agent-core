// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Reusable resident workspace-search tool capability executing three-tier
//! concurrent search over ASP's resident index and Open EDB:
//! - **L0 (Pure-Rust Brute-Force)**: `(brute-search ...)` runs in parallel on
//!   Tokio's blocking pool under `RwLock::read()` (zero Gerbil VM contention).
//! - **L1 (Case A Fast Ascent)**: `(ascent-fragment ...)` / `:gerbil-parser`
//!   dispatched asynchronously at high VM priority with single-flight caching.
//! - **L2 (Case B Dusa Solver)**: `(search-choice-program ...)` /
//!   `(relational-choice-program ...)` dispatched asynchronously with time-sliced
//!   VM priority, cancellation drop, and single-flight coalescing.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use marlin_agent_protocol::ModelToolCall;
use marlin_agent_runtime::{
    ModelToolDeclaration, ModelToolOutcome, RuntimeContext, RuntimeFuture, ToolRuntime,
};
use marlin_agent_search_engine::{
    SearchDslRunner, WorkspaceIndexConfig, WorkspaceSearchIndex, default_workspace_edb_cache_path,
    default_workspace_snapshot_path, hydrate_workspace_open_edb_org_and_cache,
    open_or_build_workspace_index_safe, save_workspace_snapshot_atomic,
};

use crate::brute_search::execute_l0_search;
use crate::search_scheduler::{
    AsyncSearchDslRunner, CacheDependencyProfile, DebouncedSyncCoordinator, FencedCacheLookup,
    QueryCacheKey, SchedulerCounters, SearchQueryTier, SearchSchedulerMetrics,
    SingleFlightQueryCache,
};

const DEFAULT_SYNC_DEBOUNCE: Duration = Duration::from_millis(500);
const DEFAULT_QUERY_CACHE_CAPACITY: usize = 256;
const COLD_GERBIL_VM_TIMEOUT_FLOOR: Duration = Duration::from_secs(30);
const COLD_GERBIL_VM_ELIGIBLE_MIN_TIMEOUT: Duration = Duration::from_secs(5);

static GERBIL_VM_WARM: AtomicBool = AtomicBool::new(false);

fn effective_dsl_timeout(declared: Duration) -> Duration {
    if !GERBIL_VM_WARM.load(Ordering::Acquire) && declared >= COLD_GERBIL_VM_ELIGIBLE_MIN_TIMEOUT {
        declared.max(COLD_GERBIL_VM_TIMEOUT_FLOOR)
    } else {
        declared
    }
}

/// Shared resident index handle reused across TUI Helm and Agent tool calls.
pub type SharedWorkspaceSearchIndex = Arc<RwLock<WorkspaceSearchIndex>>;

#[derive(Clone)]
struct AttachedWorkspaceIndex {
    workspace_root: Option<PathBuf>,
    index: SharedWorkspaceSearchIndex,
}

fn canonicalize_workspace_root(workspace: &Path) -> PathBuf {
    std::fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf())
}

/// Late-bound slot sharing one resident [`WorkspaceSearchIndex`] between the host
/// UI (e.g. TUI Helm warm-up / `C-l` incremental sync) and concurrent LLM Agent tool calls.
#[derive(Clone)]
pub struct SharedSearchIndexSlot {
    inner: Arc<RwLock<Option<AttachedWorkspaceIndex>>>,
    open_lock: Arc<Mutex<()>>,
    dsl_runner: Arc<RwLock<Option<SearchDslRunner>>>,
    async_dsl_runner: Arc<RwLock<Option<AsyncSearchDslRunner>>>,
    sync_coordinator: Arc<DebouncedSyncCoordinator>,
    query_cache: Arc<SingleFlightQueryCache>,
    counters: Arc<SchedulerCounters>,
}

impl Default for SharedSearchIndexSlot {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
            open_lock: Arc::new(Mutex::new(())),
            dsl_runner: Arc::new(RwLock::new(None)),
            async_dsl_runner: Arc::new(RwLock::new(None)),
            sync_coordinator: Arc::new(DebouncedSyncCoordinator::new(DEFAULT_SYNC_DEBOUNCE)),
            query_cache: Arc::new(SingleFlightQueryCache::new(DEFAULT_QUERY_CACHE_CAPACITY)),
            counters: Arc::new(SchedulerCounters::default()),
        }
    }
}

impl SharedSearchIndexSlot {
    /// Create an empty slot that lazily opens a cached `mmap` snapshot if queried
    /// before the host attaches a warm index.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a slot pre-populated with an existing warm resident index.
    #[must_use]
    pub fn from_index(index: SharedWorkspaceSearchIndex) -> Self {
        let slot = Self::default();
        slot.attach(index);
        slot
    }

    /// Create a slot with a custom disk-sync debounce window (e.g. for benchmarks/tests).
    #[must_use]
    pub fn with_sync_debounce(
        index: Option<SharedWorkspaceSearchIndex>,
        debounce: Duration,
    ) -> Self {
        let attached = index.map(|idx| AttachedWorkspaceIndex {
            workspace_root: None,
            index: idx,
        });
        Self {
            inner: Arc::new(RwLock::new(attached)),
            open_lock: Arc::new(Mutex::new(())),
            dsl_runner: Arc::new(RwLock::new(None)),
            async_dsl_runner: Arc::new(RwLock::new(None)),
            sync_coordinator: Arc::new(DebouncedSyncCoordinator::new(debounce)),
            query_cache: Arc::new(SingleFlightQueryCache::new(DEFAULT_QUERY_CACHE_CAPACITY)),
            counters: Arc::new(SchedulerCounters::default()),
        }
    }

    /// Attach or replace the shared resident index (e.g. after background warm-up).
    pub fn attach(&self, index: SharedWorkspaceSearchIndex) {
        if let Ok(mut guard) = self.inner.write() {
            *guard = Some(AttachedWorkspaceIndex {
                workspace_root: None,
                index,
            });
        }
    }

    fn attach_for_root(&self, workspace_root: PathBuf, index: SharedWorkspaceSearchIndex) {
        if let Ok(mut guard) = self.inner.write() {
            *guard = Some(AttachedWorkspaceIndex {
                workspace_root: Some(workspace_root),
                index,
            });
        }
    }

    /// Attach an AOT-linked or custom synchronous Scheme Search DSL runner.
    pub fn attach_dsl_runner(&self, runner: SearchDslRunner) {
        if let Ok(mut guard) = self.dsl_runner.write() {
            *guard = Some(runner);
        }
    }

    /// Attach a non-blocking priority-aware async Scheme Search DSL runner
    /// (`L1 FastAscent` / `L2 DusaSolver` via `tokio::sync::oneshot`).
    pub fn attach_async_dsl_runner(&self, runner: AsyncSearchDslRunner) {
        if let Ok(mut guard) = self.async_dsl_runner.write() {
            *guard = Some(runner);
        }
    }

    /// Snapshot scheduler concurrency, cache, and single-flight telemetry.
    #[must_use]
    pub fn scheduler_metrics(&self) -> SearchSchedulerMetrics {
        self.counters.snapshot()
    }

    /// Current monotonic structural AST epoch used to fence KV/query cache reuse.
    #[must_use]
    pub fn structural_epoch(&self) -> u64 {
        self.query_cache.structural_epoch()
    }

    /// Register a dependency-tagged cached search result pinned to the current structural epoch.
    pub fn cache_with_dependency_profile(
        &self,
        key: QueryCacheKey,
        value: String,
        deps: CacheDependencyProfile,
    ) -> Result<u64, &'static str> {
        self.query_cache.insert_with_profile(key, value, deps)
    }

    /// Epoch- and poison-tombstone-fenced lookup over cached search evidence.
    #[must_use]
    pub fn fenced_cached_query(
        &self,
        key: &QueryCacheKey,
        expected_epoch: u64,
    ) -> FencedCacheLookup {
        self.query_cache.fenced_lookup(key, expected_epoch)
    }

    /// Comment-only edit invalidation: evicts ONLY content-sensitive L0 entries on `changed_paths`.
    #[must_use]
    pub fn invalidate_content_paths(&self, changed_paths: &[&str]) -> usize {
        self.query_cache.invalidate_content_paths(changed_paths)
    }

    /// Structural AST edit invalidation: selectively evicts entries depending on `changed_paths`
    /// and advances `structural_epoch`.
    #[must_use]
    pub fn invalidate_structural_paths(&self, changed_paths: &[&str]) -> usize {
        self.query_cache.invalidate_structural_paths(changed_paths)
    }

    /// DE01 Negative-domain expansion invalidation for universal/negative queries watching a scope.
    #[must_use]
    pub fn invalidate_negative_scope(&self, added_path: &str) -> usize {
        self.query_cache.invalidate_negative_scope(added_path)
    }

    /// Retract a poisoned cached entry and pin a permanent forensic tombstone blocking re-entry.
    pub fn retract_poisoned_query(&self, key: QueryCacheKey) {
        self.query_cache.retract_and_tombstone(key);
    }

    /// Resolve the active synchronous [`SearchDslRunner`], falling back to the
    /// compiled package runner when `linked-ui` is not active.
    #[must_use]
    pub fn dsl_runner(&self) -> Option<SearchDslRunner> {
        if let Ok(guard) = self.dsl_runner.read()
            && let Some(existing) = guard.as_ref()
        {
            return Some(existing.clone());
        }
        marlin_agent_search_engine::default_search_dsl_runner()
    }

    /// Resolve the active priority-aware [`AsyncSearchDslRunner`], if attached.
    #[must_use]
    pub fn async_dsl_runner(&self) -> Option<AsyncSearchDslRunner> {
        self.async_dsl_runner
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Return the attached resident index or lazily open/build a cached `mmap` index
    /// for `workspace` and store it in the slot for subsequent calls.
    ///
    /// Enforces `SM15` (`SearchIndexSnapshotIsolation`): concurrent cold-start opens
    /// serialize behind `open_lock` with double-checked canonical root verification so
    /// parallel callers share a single authoritative `Arc` and switched worktrees never
    /// read a stale workspace's cached snapshot.
    #[must_use]
    pub fn get_or_open(&self, workspace: &Path) -> SharedWorkspaceSearchIndex {
        let canonical = canonicalize_workspace_root(workspace);
        if let Ok(guard) = self.inner.read()
            && let Some(existing) = guard.as_ref()
            && existing
                .workspace_root
                .as_deref()
                .is_none_or(|root| root == canonical.as_path())
        {
            return existing.index.clone();
        }
        let _open_guard = self
            .open_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Ok(guard) = self.inner.read()
            && let Some(existing) = guard.as_ref()
            && existing
                .workspace_root
                .as_deref()
                .is_none_or(|root| root == canonical.as_path())
        {
            return existing.index.clone();
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
        self.attach_for_root(canonical, opened.clone());
        opened
    }

    /// Incrementally reconcile the attached resident index with on-disk workspace
    /// changes and persist the cached `mmap` snapshot when files changed.
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

    /// Non-blocking debounced single-flight disk synchronization for concurrent agents.
    async fn ensure_synced_async(&self, context: &RuntimeContext, workspace: PathBuf) {
        let slot = self.clone();
        self.sync_coordinator
            .ensure_synced(context, &self.counters, move || {
                let _ = slot.sync_from_disk_and_persist(&workspace);
            })
            .await;
    }
}

/// Reusable resident workspace search capability for Agent tool calls.
/// Routes `(brute-search ...)` to L0 parallel Rust search and native
/// `:gerbil-ascent` / Dusa S-expression forms to priority-scheduled L1/L2 execution.
pub struct ResidentSearchToolHandler {
    workspace: PathBuf,
    declaration: ModelToolDeclaration,
    slot: SharedSearchIndexSlot,
}

impl ResidentSearchToolHandler {
    /// Construct a handler bound to `workspace`, `declaration`, and `slot`.
    #[must_use]
    pub fn new(
        workspace: PathBuf,
        declaration: ModelToolDeclaration,
        slot: SharedSearchIndexSlot,
    ) -> Self {
        Self {
            workspace,
            declaration,
            slot,
        }
    }
}

impl ToolRuntime for ResidentSearchToolHandler {
    type Invocation = ModelToolCall;
    type Output = ModelToolOutcome;

    fn run_tool(
        &self,
        call: ModelToolCall,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let workspace = self.workspace.clone();
        let declaration = self.declaration.clone();
        let slot = self.slot.clone();
        Box::pin(async move {
            if !declaration.admits(&call) {
                return ModelToolOutcome::Failed("invalid search tool arguments".into());
            }
            let raw_query = call.arguments[declaration.argument()]
                .as_str()
                .expect("admitted string query")
                .to_owned();
            let active_root = context
                .active_working_copy()
                .map_or(workspace, |binding| binding.working_copy.path.clone());
            let timeout = declaration.timeout().max(Duration::from_secs(1));
            let tool_name = declaration.definition().name.as_str().to_owned();
            match execute_resident_search_tool_async(
                context,
                active_root,
                slot,
                tool_name,
                raw_query,
                declaration.output_limit(),
                timeout,
            )
            .await
            {
                Ok(rendered) => ModelToolOutcome::Completed(rendered),
                Err(message) => ModelToolOutcome::Failed(message),
            }
        })
    }
}

async fn execute_resident_search_tool_async(
    context: RuntimeContext,
    workspace: PathBuf,
    slot: SharedSearchIndexSlot,
    tool_name: String,
    raw_query: String,
    output_limit: usize,
    timeout: Duration,
) -> Result<String, String> {
    let trimmed = raw_query.trim().to_owned();
    if trimmed.is_empty() {
        return Err(format!(
            "{tool_name} requires a non-empty S-expression query: (brute-search ...), (file-outline ...), (read-file ...), (ascent-fragment ...), or (search-choice-program ...)"
        ));
    }

    let tier = SearchQueryTier::classify(&trimmed);
    slot.ensure_synced_async(&context, workspace.clone()).await;

    let open_slot = slot.clone();
    let open_ws = workspace.clone();
    let shared = context
        .spawn_blocking(move || open_slot.get_or_open(&open_ws))
        .join()
        .await
        .map_err(|err| format!("resident search index open failed: {err}"))?;

    let generation = shared
        .read()
        .map_err(|_| "resident search index lock poisoned".to_owned())?
        .generation_digest()
        .as_str()
        .to_owned();

    let cache_key = QueryCacheKey {
        generation,
        query: trimmed.clone(),
        output_limit,
    };

    match tier {
        SearchQueryTier::BruteForce => {
            slot.counters
                .l0_brute_queries
                .fetch_add(1, Ordering::Relaxed);
            slot.query_cache
                .get_or_compute(cache_key, &slot.counters, move || async move {
                    let task = context.spawn_blocking(move || {
                        let guard = shared
                            .read()
                            .map_err(|_| "resident search index lock poisoned".to_owned())?;
                        execute_l0_search(&guard, &trimmed, output_limit)
                    });
                    match context.timeout(timeout, task.join()).await {
                        Ok(Ok(res)) => res,
                        Ok(Err(join_err)) => {
                            Err(format!("L0 brute-search task failed: {join_err}"))
                        }
                        Err(_) => Err(format!(
                            "L0 brute-search timed out after {}s",
                            timeout.as_secs()
                        )),
                    }
                })
                .await
        }
        SearchQueryTier::FastAscent | SearchQueryTier::DusaSolver => {
            if tier == SearchQueryTier::FastAscent {
                slot.counters
                    .l1_ascent_queries
                    .fetch_add(1, Ordering::Relaxed);
            } else {
                slot.counters
                    .l2_dusa_queries
                    .fetch_add(1, Ordering::Relaxed);
            }
            let async_runner = slot.async_dsl_runner();
            let sync_runner = if async_runner.is_none() {
                slot.dsl_runner()
            } else {
                None
            };
            if async_runner.is_none() && sync_runner.is_none() {
                return Err(
                    "search DSL Scheme runtime is not attached to SharedSearchIndexSlot".to_owned(),
                );
            }
            let dsl_timeout = effective_dsl_timeout(timeout);
            slot.query_cache
                .get_or_compute(cache_key, &slot.counters, move || async move {
                    if let Some(runner) = async_runner {
                        return match context
                            .timeout(dsl_timeout, runner(shared, trimmed, output_limit, tier))
                            .await
                        {
                            Ok(Ok(rendered)) => {
                                GERBIL_VM_WARM.store(true, Ordering::Release);
                                Ok(rendered)
                            }
                            Ok(Err(err)) => {
                                GERBIL_VM_WARM.store(true, Ordering::Release);
                                Err(err.to_string())
                            }
                            Err(_) => Err(format!(
                                "search DSL query ({tier:?}) timed out after {}s",
                                dsl_timeout.as_secs()
                            )),
                        };
                    }
                    let runner = sync_runner.expect("checked sync_runner is Some");
                    let task = context.spawn_blocking(move || {
                        runner(shared, &trimmed, output_limit).map_err(|err| err.to_string())
                    });
                    match context.timeout(dsl_timeout, task.join()).await {
                        Ok(Ok(res)) => {
                            GERBIL_VM_WARM.store(true, Ordering::Release);
                            res
                        }
                        Ok(Err(join_err)) => Err(format!("search DSL task failed: {join_err}")),
                        Err(_) => Err(format!(
                            "search DSL query ({tier:?}) timed out after {}s",
                            dsl_timeout.as_secs()
                        )),
                    }
                })
                .await
        }
    }
}
