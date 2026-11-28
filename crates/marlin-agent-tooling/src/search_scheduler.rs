// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Three-tier async multi-agent search scheduling primitives (`L0` + `L1` + `L2`):
//! - Query priority classification (`L0 BruteForce` vs `L1 FastAscent` vs `L2 DusaSolver`)
//! - Non-blocking async DSL runner boundary (`tokio::sync::oneshot` + priority)
//! - Debounced single-flight disk synchronization (eliminates per-call `RwLock::write()` storms)
//! - Bounded `(generation_digest, query)` single-flight result cache (16 identical concurrent
//!   agents coalesce into 1 Gerbil VM evaluation).

#[cfg(test)]
use std::collections::{HashMap, HashSet, VecDeque};
#[cfg(test)]
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use marlin_agent_runtime::RuntimeContext;
use tokio::sync::Notify;
#[cfg(test)]
use tokio::sync::watch;

use crate::brute_search::is_l0_search_form;

/// Scheduling priority tier for concurrent multi-agent `workspace_search` calls.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SearchQueryTier {
    /// L0: Pure-Rust parallel RAM fast path (`brute-search`, `file-outline`, `read-file`, zero VM lock).
    BruteForce,
    /// L1: Fast Case A `:gerbil-ascent` / `:gerbil-parser` queries (high VM priority).
    FastAscent,
    /// L2: Heavy Case B Dusa finite-choice constraint solving (time-sliced VM priority).
    DusaSolver,
}

impl SearchQueryTier {
    /// Classify a raw S-expression query into L0 / L1 / L2 in O(1).
    #[must_use]
    pub fn classify(raw_query: &str) -> Self {
        if is_l0_search_form(raw_query) {
            return Self::BruteForce;
        }
        if raw_query.contains("search-choice-program")
            || raw_query.contains("relational-choice-program")
            || raw_query.contains("solve-placement")
            || raw_query.contains("solve-resolve")
        {
            return Self::DusaSolver;
        }
        Self::FastAscent
    }
}

/// Snapshot of scheduler concurrency & coalescing metrics for diagnostics and benchmarks.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SearchSchedulerMetrics {
    pub l0_brute_queries: u64,
    pub l1_ascent_queries: u64,
    pub l2_dusa_queries: u64,
    pub cache_hits: u64,
    pub single_flight_coalesced: u64,
    pub disk_sync_executed: u64,
    pub disk_sync_coalesced_or_debounced: u64,
}

#[derive(Default)]
pub(crate) struct SchedulerCounters {
    pub(crate) l0_brute_queries: AtomicU64,
    pub(crate) l1_ascent_queries: AtomicU64,
    pub(crate) l2_dusa_queries: AtomicU64,
    pub(crate) cache_hits: AtomicU64,
    pub(crate) single_flight_coalesced: AtomicU64,
    pub(crate) disk_sync_executed: AtomicU64,
    pub(crate) disk_sync_coalesced_or_debounced: AtomicU64,
}

impl SchedulerCounters {
    pub(crate) fn snapshot(&self) -> SearchSchedulerMetrics {
        SearchSchedulerMetrics {
            l0_brute_queries: self.l0_brute_queries.load(Ordering::Relaxed),
            l1_ascent_queries: self.l1_ascent_queries.load(Ordering::Relaxed),
            l2_dusa_queries: self.l2_dusa_queries.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            single_flight_coalesced: self.single_flight_coalesced.load(Ordering::Relaxed),
            disk_sync_executed: self.disk_sync_executed.load(Ordering::Relaxed),
            disk_sync_coalesced_or_debounced: self
                .disk_sync_coalesced_or_debounced
                .load(Ordering::Relaxed),
        }
    }
}

/// Single-flight debounced disk sync gate preventing N concurrent agents from
/// triggering N redundant `RwLock::write()` filesystem walks.
pub(crate) struct DebouncedSyncCoordinator {
    debounce: Duration,
    in_flight: AtomicBool,
    last_completed: Mutex<Option<Instant>>,
    notify: Notify,
}

impl DebouncedSyncCoordinator {
    pub(crate) fn new(debounce: Duration) -> Self {
        Self {
            debounce,
            in_flight: AtomicBool::new(false),
            last_completed: Mutex::new(None),
            notify: Notify::new(),
        }
    }

    pub(crate) fn record_external_sync(&self) {
        if let Ok(mut guard) = self.last_completed.lock() {
            *guard = Some(Instant::now());
        }
    }

    /// Run `sync_fn` through `context.spawn_blocking` at most once per `debounce` window;
    /// concurrent callers coalesce onto the in-flight sync without contending.
    ///
    /// Cancellation-safe (`SM9`): leader ownership is guarded by an RAII `Drop` guard
    /// so dropping the leader future mid-`.await` resets `in_flight` and wakes all
    /// waiters; waiters pin and arm `Notify::notified().enable()` BEFORE checking
    /// `in_flight` to eliminate lost wakeups.
    pub(crate) async fn ensure_synced<F>(
        &self,
        context: &RuntimeContext,
        counters: &SchedulerCounters,
        sync_fn: F,
    ) where
        F: FnOnce() + Send + 'static,
    {
        struct SyncLeaderGuard<'a> {
            coord: &'a DebouncedSyncCoordinator,
        }
        impl Drop for SyncLeaderGuard<'_> {
            fn drop(&mut self) {
                self.coord.in_flight.store(false, Ordering::Release);
                self.coord.notify.notify_waiters();
            }
        }

        if self.is_fresh() {
            counters
                .disk_sync_coalesced_or_debounced
                .fetch_add(1, Ordering::Relaxed);
            return;
        }
        if self
            .in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            counters
                .disk_sync_coalesced_or_debounced
                .fetch_add(1, Ordering::Relaxed);
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.in_flight.load(Ordering::Acquire) {
                notified.await;
            }
            return;
        }
        let _leader_guard = SyncLeaderGuard { coord: self };
        counters.disk_sync_executed.fetch_add(1, Ordering::Relaxed);
        let _ = context.spawn_blocking(sync_fn).join().await;
        self.record_external_sync();
    }

    fn is_fresh(&self) -> bool {
        let Ok(guard) = self.last_completed.lock() else {
            return false;
        };
        guard.is_some_and(|ts| ts.elapsed() < self.debounce)
    }
}

/// Composite cache key pinning a search query and output cap to an index generation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QueryCacheKey {
    pub generation: String,
    pub query: String,
    pub output_limit: usize,
}

#[cfg(test)]
type CachedOutcome = Result<String, String>;

/// Dependency profile attached to a cached search/evidence entry refining
/// `SearchEvidenceChainIntegrity.qnt`:
/// - `content_sensitive`: `true` for L0 raw text queries (invalidated by comment edits),
///   `false` for L1/L2 AST structural & Dusa proofs (preserved across comment-only edits!).
/// - `file_deps`: workspace paths this entry structurally depends on.
/// - `negative_scope_prefix`: optional watched directory prefix for universal/negative
///   queries invalidated under DE01 negative-domain expansion.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CacheDependencyProfile {
    pub content_sensitive: bool,
    pub file_deps: Vec<String>,
    pub negative_scope_prefix: Option<String>,
}

/// Fenced lookup outcome enforcing structural epoch and poison tombstone laws.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FencedCacheLookup {
    Hit {
        value: String,
        epoch: u64,
    },
    Miss,
    RejectedStaleEpoch {
        cached_epoch: u64,
        active_epoch: u64,
    },
    RejectedPoisonTombstone,
}

#[cfg(test)]
#[derive(Clone, Debug)]
struct CacheEntryRecord {
    value: String,
    epoch: u64,
    deps: CacheDependencyProfile,
}

#[cfg(test)]
struct CacheState {
    entries: HashMap<QueryCacheKey, CacheEntryRecord>,
    order: VecDeque<QueryCacheKey>,
    in_flight: HashMap<QueryCacheKey, watch::Receiver<Option<CachedOutcome>>>,
    poison_tombstones: HashSet<QueryCacheKey>,
    structural_epoch: u64,
    capacity: usize,
}

/// Bounded `(generation_digest, query)` single-flight cache with selective
/// content vs structural invalidation, DE01 negative-scope invalidation,
/// structural epoch fencing, and poison-immunization tombstones.
#[cfg(test)]
pub(crate) struct SingleFlightQueryCache {
    state: Mutex<CacheState>,
}

#[cfg(test)]
enum LookupOutcome {
    Hit(String),
    Wait(watch::Receiver<Option<CachedOutcome>>),
    Leader(watch::Sender<Option<CachedOutcome>>),
}

#[cfg(test)]
impl SingleFlightQueryCache {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(CacheState {
                entries: HashMap::new(),
                order: VecDeque::new(),
                in_flight: HashMap::new(),
                poison_tombstones: HashSet::new(),
                structural_epoch: 1,
                capacity: capacity.max(16),
            }),
        }
    }

    pub(crate) fn structural_epoch(&self) -> u64 {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .structural_epoch
    }

    pub(crate) fn insert_with_profile(
        &self,
        key: QueryCacheKey,
        value: String,
        deps: CacheDependencyProfile,
    ) -> Result<u64, &'static str> {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.poison_tombstones.contains(&key) {
            return Err("poison-tombstone-blocked");
        }
        let epoch = guard.structural_epoch;
        if !guard.entries.contains_key(&key) {
            while guard.order.len() >= guard.capacity {
                if let Some(evicted) = guard.order.pop_front() {
                    guard.entries.remove(&evicted);
                }
            }
            guard.order.push_back(key.clone());
        }
        guard
            .entries
            .insert(key, CacheEntryRecord { value, epoch, deps });
        Ok(epoch)
    }

    pub(crate) fn fenced_lookup(
        &self,
        key: &QueryCacheKey,
        expected_epoch: u64,
    ) -> FencedCacheLookup {
        let guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.poison_tombstones.contains(key) {
            return FencedCacheLookup::RejectedPoisonTombstone;
        }
        let active_epoch = guard.structural_epoch;
        if expected_epoch != active_epoch {
            return FencedCacheLookup::RejectedStaleEpoch {
                cached_epoch: expected_epoch,
                active_epoch,
            };
        }
        match guard.entries.get(key) {
            Some(rec) if rec.epoch == active_epoch => FencedCacheLookup::Hit {
                value: rec.value.clone(),
                epoch: rec.epoch,
            },
            Some(rec) => FencedCacheLookup::RejectedStaleEpoch {
                cached_epoch: rec.epoch,
                active_epoch,
            },
            None => FencedCacheLookup::Miss,
        }
    }

    /// Comment-only / formatting edit: invalidates ONLY content-sensitive L0 entries
    /// touching `changed_paths`; structural L1/L2 entries and `structural_epoch` are preserved.
    pub(crate) fn invalidate_content_paths(&self, changed_paths: &[&str]) -> usize {
        let changed: HashSet<&str> = changed_paths.iter().copied().collect();
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let doomed: Vec<QueryCacheKey> = guard
            .entries
            .iter()
            .filter(|(_, rec)| {
                rec.deps.content_sensitive
                    && rec
                        .deps
                        .file_deps
                        .iter()
                        .any(|p| changed.contains(p.as_str()))
            })
            .map(|(k, _)| k.clone())
            .collect();
        Self::evict_keys(&mut guard, &doomed)
    }

    /// Structural AST edit: selectively evicts ONLY entries depending on `changed_paths`
    /// and advances `structural_epoch` so unaffected files remain cached while stale KV epochs
    /// are fenced.
    pub(crate) fn invalidate_structural_paths(&self, changed_paths: &[&str]) -> usize {
        let changed: HashSet<&str> = changed_paths.iter().copied().collect();
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.structural_epoch = guard.structural_epoch.saturating_add(1);
        let new_epoch = guard.structural_epoch;
        let doomed: Vec<QueryCacheKey> = guard
            .entries
            .iter()
            .filter(|(_, rec)| {
                rec.deps
                    .file_deps
                    .iter()
                    .any(|p| changed.contains(p.as_str()))
            })
            .map(|(k, _)| k.clone())
            .collect();
        let evicted = Self::evict_keys(&mut guard, &doomed);
        // Surviving unaffected structural entries remain valid at the new structural epoch.
        for rec in guard.entries.values_mut() {
            if !rec.deps.content_sensitive {
                rec.epoch = new_epoch;
            }
        }
        evicted
    }

    /// DE01 Negative-domain expansion: when a new file/symbol enters `added_path`,
    /// evicts ONLY universal/negative queries whose watched `negative_scope_prefix`
    /// matches `added_path`, advancing `structural_epoch`.
    pub(crate) fn invalidate_negative_scope(&self, added_path: &str) -> usize {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.structural_epoch = guard.structural_epoch.saturating_add(1);
        let new_epoch = guard.structural_epoch;
        let doomed: Vec<QueryCacheKey> = guard
            .entries
            .iter()
            .filter(|(_, rec)| {
                rec.deps
                    .negative_scope_prefix
                    .as_deref()
                    .is_some_and(|prefix| added_path.starts_with(prefix))
            })
            .map(|(k, _)| k.clone())
            .collect();
        let evicted = Self::evict_keys(&mut guard, &doomed);
        for rec in guard.entries.values_mut() {
            if !rec.deps.content_sensitive {
                rec.epoch = new_epoch;
            }
        }
        evicted
    }

    /// Retract a poisoned entry and pin a permanent forensic tombstone blocking re-insertion.
    pub(crate) fn retract_and_tombstone(&self, key: QueryCacheKey) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.structural_epoch = guard.structural_epoch.saturating_add(1);
        guard.entries.remove(&key);
        guard.order.retain(|k| k != &key);
        guard.poison_tombstones.insert(key);
    }

    fn evict_keys(guard: &mut CacheState, doomed: &[QueryCacheKey]) -> usize {
        if doomed.is_empty() {
            return 0;
        }
        let doomed_set: HashSet<&QueryCacheKey> = doomed.iter().collect();
        guard.entries.retain(|k, _| !doomed_set.contains(k));
        guard.order.retain(|k| !doomed_set.contains(k));
        doomed.len()
    }

    fn begin_lookup(&self, key: &QueryCacheKey) -> LookupOutcome {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(hit) = guard.entries.get(key).map(|rec| rec.value.clone()) {
            LookupOutcome::Hit(hit)
        } else if let Some(rx) = guard.in_flight.get(key).cloned() {
            LookupOutcome::Wait(rx)
        } else {
            let (tx, rx) = watch::channel(None);
            guard.in_flight.insert(key.clone(), rx);
            LookupOutcome::Leader(tx)
        }
    }

    async fn await_in_flight(
        mut rx: watch::Receiver<Option<CachedOutcome>>,
    ) -> Option<CachedOutcome> {
        loop {
            if let Some(res) = rx.borrow().clone() {
                return Some(res);
            }
            if rx.changed().await.is_err() {
                return None;
            }
        }
    }

    fn publish_leader_outcome(&self, key: QueryCacheKey, outcome: &CachedOutcome) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.in_flight.remove(&key);
        if guard.poison_tombstones.contains(&key) {
            return;
        }
        let epoch = guard.structural_epoch;
        if let Ok(value) = outcome {
            if !guard.entries.contains_key(&key) {
                while guard.order.len() >= guard.capacity {
                    if let Some(evicted) = guard.order.pop_front() {
                        guard.entries.remove(&evicted);
                    }
                }
                guard.order.push_back(key.clone());
            }
            guard.entries.insert(
                key,
                CacheEntryRecord {
                    value: value.clone(),
                    epoch,
                    deps: CacheDependencyProfile::default(),
                },
            );
        }
    }

    pub(crate) async fn get_or_compute<F, Fut>(
        &self,
        key: QueryCacheKey,
        counters: &SchedulerCounters,
        compute: F,
    ) -> CachedOutcome
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = CachedOutcome>,
    {
        let mut compute = Some(compute);
        loop {
            match self.begin_lookup(&key) {
                LookupOutcome::Hit(hit) => {
                    counters.cache_hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(hit);
                }
                LookupOutcome::Wait(rx) => {
                    counters
                        .single_flight_coalesced
                        .fetch_add(1, Ordering::Relaxed);
                    // SM15: if leader cancelled/dropped before publishing (`None`),
                    // re-enter election so a surviving healthy follower completes the query.
                    if let Some(res) = Self::await_in_flight(rx).await {
                        return res;
                    }
                }
                LookupOutcome::Leader(tx) => {
                    struct CleanupGuard<'a> {
                        cache: &'a SingleFlightQueryCache,
                        key: Option<QueryCacheKey>,
                    }
                    impl Drop for CleanupGuard<'_> {
                        fn drop(&mut self) {
                            if let Some(key) = self.key.take() {
                                let mut guard = self
                                    .cache
                                    .state
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                                guard.in_flight.remove(&key);
                            }
                        }
                    }
                    let mut cleanup = CleanupGuard {
                        cache: self,
                        key: Some(key.clone()),
                    };
                    let compute_fn = compute
                        .take()
                        .expect("single-flight leader computes at most once");
                    let outcome = compute_fn().await;
                    self.publish_leader_outcome(key, &outcome);
                    cleanup.key = None;
                    let _ = tx.send(Some(outcome.clone()));
                    return outcome;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/search_scheduler_stability.rs"]
mod stability_tests;
