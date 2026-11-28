// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Unified Package Bridge framework (`PackageBridge`, `BridgeRegistry`, and
//! generic `SwrCache<K, V>`) for `marlin-agent-app-server`.
//!
//! All Rust-backed and Scheme-declared workbench packages (e.g. `magit`, `fs`,
//! and future `org-roam`, `lsp`, `treemacs`, `vterm`, `dired`) register into
//! one uniform registry instead of duplicating query routing, command dispatch,
//! background pre-warming, or stale-while-revalidate caching across gRPC
//! handlers. Performance improvements made to `SwrCache` or `BridgeRegistry`
//! automatically accelerate every registered package.

use std::{
    collections::{HashMap, HashSet},
    fs,
    future::Future,
    hash::Hash,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, UNIX_EPOCH},
};

use crate::wire::marlin::projection::v1::{
    GraphRecordView, GraphTrajectoryEvent, QueryProjectGraphRequest, WorkspaceDiffSnapshot,
};

/// Uniform typed outcome returned by a `PackageBridge` mutation command.
#[derive(Clone, Debug)]
pub struct BridgeCommandOutcome {
    pub ok: bool,
    pub summary: String,
    pub owner_id: String,
    pub trajectory: Option<GraphTrajectoryEvent>,
}

/// Lock-free telemetry counters shared by every `SwrCache` and `BridgeRegistry`.
#[derive(Debug, Default)]
pub struct BridgeMetrics {
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub swr_revalidations: AtomicU64,
    pub queries_dispatched: AtomicU64,
    pub commands_dispatched: AtomicU64,
}

impl BridgeMetrics {
    pub fn snapshot(&self) -> BridgeMetricsSnapshot {
        BridgeMetricsSnapshot {
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            swr_revalidations: self.swr_revalidations.load(Ordering::Relaxed),
            queries_dispatched: self.queries_dispatched.load(Ordering::Relaxed),
            commands_dispatched: self.commands_dispatched.load(Ordering::Relaxed),
        }
    }
}

/// Point-in-time snapshot of `BridgeMetrics`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BridgeMetricsSnapshot {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub swr_revalidations: u64,
    pub queries_dispatched: u64,
    pub commands_dispatched: u64,
}

/// Generic thread-safe TTL + Stale-While-Revalidate cache with coalesced
/// background refresh deduplication shared across all `PackageBridge` modules.
#[derive(Clone)]
pub struct SwrCache<K, V> {
    ttl: Duration,
    entries: Arc<RwLock<HashMap<K, (Instant, V)>>>,
    in_flight: Arc<Mutex<HashSet<K>>>,
    metrics: Arc<BridgeMetrics>,
}

impl<K, V> SwrCache<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: Arc::new(RwLock::new(HashMap::new())),
            in_flight: Arc::new(Mutex::new(HashSet::new())),
            metrics: Arc::new(BridgeMetrics::default()),
        }
    }

    pub fn metrics(&self) -> BridgeMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Returns the cached value without spawning a refresh.
    pub fn get_cached(&self, key: &K) -> Option<V> {
        self.entries
            .read()
            .ok()
            .and_then(|map| map.get(key).map(|(_, v)| v.clone()))
    }

    /// Inserts `value` under all `keys` sharing the same capture timestamp.
    pub fn insert_keys(&self, keys: impl IntoIterator<Item = K>, value: V) {
        let now = Instant::now();
        if let Ok(mut map) = self.entries.write() {
            for k in keys {
                map.insert(k, (now, value.clone()));
            }
        }
    }

    /// Spawns a deduplicated background refresh when a Tokio runtime is active.
    /// Concurrent requests for the same `key` coalesce into a single execution.
    pub fn spawn_background<F, Fut>(&self, key: K, loader: F)
    where
        F: FnOnce(K) -> Fut + Send + 'static,
        Fut: Future<Output = Result<V, String>> + Send + 'static,
    {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        {
            let Ok(mut guard) = self.in_flight.lock() else {
                return;
            };
            if !guard.insert(key.clone()) {
                return;
            }
        }
        let cache = self.clone();
        handle.spawn(async move {
            if let Ok(val) = loader(key.clone()).await {
                cache.insert_keys([key.clone()], val);
            }
            if let Ok(mut guard) = cache.in_flight.lock() {
                guard.remove(&key);
            }
        });
    }

    /// Synchronous Stale-While-Revalidate lookup for blocking/filesystem bridges:
    /// - `!force_refresh` + fresh entry -> immediate return.
    /// - `!force_refresh` + stale entry -> immediate return + coalesced background
    ///   `spawn_blocking` refresh when a Tokio runtime is active.
    /// - Cache miss or `force_refresh` -> executes `fg_loader` synchronously and caches.
    pub fn get_or_load_sync<Fg, Bg>(&self, key: K, force_refresh: bool, fg_loader: Fg, bg_loader: Bg) -> V
    where
        Fg: FnOnce() -> V,
        Bg: FnOnce(K) -> V + Send + 'static,
    {
        if !force_refresh {
            let cached = self
                .entries
                .read()
                .ok()
                .and_then(|map| map.get(&key).cloned());
            if let Some((captured_at, val)) = cached {
                self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
                if captured_at.elapsed() >= self.ttl {
                    self.metrics
                        .swr_revalidations
                        .fetch_add(1, Ordering::Relaxed);
                    self.spawn_background_blocking(key, bg_loader);
                }
                return val;
            }
        }
        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        let val = fg_loader();
        self.insert_keys([key], val.clone());
        val
    }

    /// Spawns a deduplicated `spawn_blocking` refresh when a Tokio runtime is active.
    pub fn spawn_background_blocking<F>(&self, key: K, loader: F)
    where
        F: FnOnce(K) -> V + Send + 'static,
    {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        {
            let Ok(mut guard) = self.in_flight.lock() else {
                return;
            };
            if !guard.insert(key.clone()) {
                return;
            }
        }
        let cache = self.clone();
        handle.spawn_blocking(move || {
            let val = loader(key.clone());
            cache.insert_keys([key.clone()], val);
            if let Ok(mut guard) = cache.in_flight.lock() {
                guard.remove(&key);
            }
        });
    }

    /// Serves a value with Stale-While-Revalidate semantics:
    /// - `!force_refresh` + fresh cached entry -> immediate return (< 50µs).
    /// - `!force_refresh` + stale cached entry -> immediate return + coalesced
    ///   background refresh via `bg_loader`.
    /// - Cache miss or `force_refresh` -> awaits `fg_loader` and caches result.
    pub async fn get_or_load<Fg, FgFut, Bg, BgFut>(
        &self,
        key: K,
        force_refresh: bool,
        fg_loader: Fg,
        bg_loader: Bg,
    ) -> Result<V, String>
    where
        Fg: FnOnce() -> FgFut,
        FgFut: Future<Output = Result<V, String>>,
        Bg: FnOnce(K) -> BgFut + Send + 'static,
        BgFut: Future<Output = Result<V, String>> + Send + 'static,
    {
        if !force_refresh {
            let cached = self
                .entries
                .read()
                .ok()
                .and_then(|map| map.get(&key).cloned());
            if let Some((captured_at, val)) = cached {
                self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
                if captured_at.elapsed() >= self.ttl {
                    self.metrics
                        .swr_revalidations
                        .fetch_add(1, Ordering::Relaxed);
                    self.spawn_background(key, bg_loader);
                }
                return Ok(val);
            }
        }
        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        let val = fg_loader().await?;
        self.insert_keys([key], val.clone());
        Ok(val)
    }
}

/// Uniform contract implemented by every Rust/Scheme package bridge module.
///
/// Adding a new workbench package (e.g. `org-roam`, `lsp`, `treemacs`, `vterm`)
/// only requires implementing this trait and calling `registry.register(Arc::new(...))`.
#[tonic::async_trait]
pub trait PackageBridge: Send + Sync {
    /// Short lowercase namespace routing `<namespace>:*` queries and `<namespace>.*` commands.
    fn namespace(&self) -> &str;

    /// Governance owner identifier emitted in typed denials & receipts (e.g. `"marlin.magit"`).
    fn owner_id(&self) -> &str;

    /// Optional non-blocking background pre-warm invoked when the host boots.
    fn prewarm(&self) {}

    /// Handles a `<namespace>:<verb>[@<arg>]` projection query.
    async fn query(
        &self,
        verb: &str,
        arg: Option<&str>,
        raw_query: &str,
    ) -> Result<Vec<GraphRecordView>, String>;

    /// Handles a `<namespace>.<action> [args]` UI mutation command.
    /// Read-only bridges keep the default `Ok(None)` implementation.
    async fn execute_command(
        &self,
        _raw_command: &str,
    ) -> Result<Option<BridgeCommandOutcome>, String> {
        Ok(None)
    }

    /// Optional fallback handler for untagged free-text projection queries
    /// (e.g. Helm multi-axis search queries without a `<ns>:` prefix).
    async fn fallback_query(
        &self,
        _req: &QueryProjectGraphRequest,
    ) -> Option<Vec<GraphRecordView>> {
        None
    }

    /// Optional live workspace diff projection for `WatchWorkspaceDiff`.
    async fn workspace_diff(&self) -> Option<WorkspaceDiffSnapshot> {
        None
    }
}

/// Resident single-threaded Gerbil Scheme VM backend plugged once into
/// `BridgeRegistry`. Every Scheme-owned package (`org`, `markdown`, `doc`, and
/// any future Scheme module registered in `scheme/ui-bridge`) is served through
/// this single uniform trait with zero per-package Rust bridge boilerplate.
pub trait SchemeBridgeBackend: Send + Sync {
    /// Lists all namespaces dynamically exported by `scheme/ui-bridge`.
    fn namespaces(&self) -> Vec<String>;

    /// Invokes `marlin-scheme-bridge-dispatch(#(namespace verb arg payload))`
    /// over native Gerbil C-ABI FFI and returns projected `GraphRecordView` rows.
    fn dispatch_query(
        &self,
        namespace: &str,
        verb: &str,
        arg: Option<&str>,
        payload: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<GraphRecordView>, String>> + Send + 'static>,
    >;
}

const MAX_SCHEME_BRIDGE_FILE_BYTES: u64 = 512 * 1024;
const SCHEME_BRIDGE_CACHE_TTL: Duration = Duration::from_secs(30);

/// Single generic `PackageBridge` adapter serving ANY namespace declared inside
/// Gerbil Scheme (`scheme/ui-bridge`), backed by one shared `SwrCache` keyed by
/// `(namespace, verb, canonical_path, file_mtime_nanos)`.
#[derive(Clone)]
pub struct UnifiedSchemeBridge {
    namespace: String,
    owner_id: String,
    workspace_root: PathBuf,
    backend: Arc<dyn SchemeBridgeBackend>,
    cache: SwrCache<String, Vec<GraphRecordView>>,
}

impl UnifiedSchemeBridge {
    pub fn new(
        namespace: String,
        workspace_root: PathBuf,
        backend: Arc<dyn SchemeBridgeBackend>,
        shared_cache: SwrCache<String, Vec<GraphRecordView>>,
    ) -> Self {
        let owner_id = format!("marlin.scheme.{namespace}");
        Self {
            namespace,
            owner_id,
            workspace_root,
            backend,
            cache: shared_cache,
        }
    }

    fn resolve_file_payload(&self, arg: Option<&str>) -> Result<(String, String, u128), String> {
        let Some(raw) = arg.map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok((String::new(), String::new(), 0));
        };
        let expanded = if let Some(rest) = raw.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(rest)
            } else {
                PathBuf::from(raw)
            }
        } else {
            let p = Path::new(raw);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                self.workspace_root.join(p)
            }
        };
        let meta = match fs::metadata(&expanded) {
            Ok(m) if m.is_file() => m,
            _ => return Ok((raw.to_owned(), String::new(), 0)),
        };
        if meta.len() > MAX_SCHEME_BRIDGE_FILE_BYTES {
            return Err(format!(
                "file exceeds Scheme bridge limit ({} > {} bytes): {}",
                meta.len(),
                MAX_SCHEME_BRIDGE_FILE_BYTES,
                expanded.display()
            ));
        }
        let mtime_nanos = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let bytes = fs::read(&expanded)
            .map_err(|e| format!("read {}: {e}", expanded.display()))?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let display_path = fs::canonicalize(&expanded)
            .unwrap_or(expanded)
            .to_string_lossy()
            .into_owned();
        Ok((display_path, text, mtime_nanos))
    }
}

#[tonic::async_trait]
impl PackageBridge for UnifiedSchemeBridge {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    fn owner_id(&self) -> &str {
        &self.owner_id
    }

    async fn query(
        &self,
        verb: &str,
        arg: Option<&str>,
        _raw_query: &str,
    ) -> Result<Vec<GraphRecordView>, String> {
        let (resolved_arg, payload, mtime_nanos) = self.resolve_file_payload(arg)?;
        let cache_key = format!("{}|{}|{}|{}", self.namespace, verb, resolved_arg, mtime_nanos);
        let ns = self.namespace.clone();
        let verb_owned = verb.to_owned();
        let arg_for_fg = resolved_arg.clone();
        let payload_for_fg = payload.clone();
        let backend_fg = Arc::clone(&self.backend);
        let ns_bg = self.namespace.clone();
        let verb_bg = verb.to_owned();
        let arg_bg = resolved_arg;
        let payload_bg = payload;
        let backend_bg = Arc::clone(&self.backend);

        self.cache
            .get_or_load(
                cache_key,
                false,
                || async move {
                    let arg_opt = if arg_for_fg.is_empty() {
                        None
                    } else {
                        Some(arg_for_fg.as_str())
                    };
                    backend_fg
                        .dispatch_query(&ns, &verb_owned, arg_opt, &payload_for_fg)
                        .await
                },
                move |_| async move {
                    let arg_opt = if arg_bg.is_empty() {
                        None
                    } else {
                        Some(arg_bg.as_str())
                    };
                    backend_bg
                        .dispatch_query(&ns_bg, &verb_bg, arg_opt, &payload_bg)
                        .await
                },
            )
            .await
    }
}

/// Unified `PackageBridge` serving Scheme-projected Major/Minor/Polymode &
/// Consult virtual-source registry records under the `modes:*` namespace.
#[derive(Clone, Default)]
pub struct SchemeModesBridge {
    records: Arc<RwLock<Vec<GraphRecordView>>>,
}

impl SchemeModesBridge {
    pub fn new(records: Vec<GraphRecordView>) -> Self {
        Self {
            records: Arc::new(RwLock::new(records)),
        }
    }

    pub fn replace_records(&self, records: Vec<GraphRecordView>) {
        if let Ok(mut guard) = self.records.write() {
            *guard = records;
        }
    }
}

#[tonic::async_trait]
impl PackageBridge for SchemeModesBridge {
    fn namespace(&self) -> &'static str {
        "modes"
    }

    fn owner_id(&self) -> &'static str {
        "marlin.scheme.modes"
    }

    async fn query(
        &self,
        _verb: &str,
        _arg: Option<&str>,
        _raw_query: &str,
    ) -> Result<Vec<GraphRecordView>, String> {
        let recs = self
            .records
            .read()
            .map_err(|_| "scheme modes bridge lock poisoned".to_owned())?
            .clone();
        Ok(recs)
    }
}

/// Central thread-safe registry routing all package projection queries
/// (`<ns>:<verb>[@<arg>]`), untagged fallback searches, workspace diffs,
/// and UI mutation commands (`<ns>.<action> ...`).
#[derive(Clone, Default)]
pub struct BridgeRegistry {
    bridges: Arc<RwLock<HashMap<String, Arc<dyn PackageBridge>>>>,
    fallback: Arc<RwLock<Option<Arc<dyn PackageBridge>>>>,
    metrics: Arc<BridgeMetrics>,
}

impl BridgeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers (or replaces) a package bridge under its `namespace()` and
    /// immediately triggers its non-blocking `prewarm()` hook.
    pub fn register(&self, bridge: Arc<dyn PackageBridge>) {
        let ns = bridge.namespace().to_ascii_lowercase();
        bridge.prewarm();
        if let Ok(mut map) = self.bridges.write() {
            map.insert(ns, bridge);
        }
    }

    /// Registers a package bridge as both its `<namespace>:*` owner AND the
    /// fallback handler for untagged free-text projection queries (e.g. Helm).
    pub fn register_fallback(&self, bridge: Arc<dyn PackageBridge>) {
        self.register(Arc::clone(&bridge));
        if let Ok(mut slot) = self.fallback.write() {
            *slot = Some(bridge);
        }
    }

    /// Dispatches an untagged free-text projection query to the registered
    /// fallback search bridge (if any).
    pub async fn dispatch_fallback_query(
        &self,
        req: &QueryProjectGraphRequest,
    ) -> Option<Vec<GraphRecordView>> {
        let fb = self.fallback.read().ok()?.clone()?;
        self.metrics
            .queries_dispatched
            .fetch_add(1, Ordering::Relaxed);
        fb.fallback_query(req).await
    }

    /// Queries registered bridges for a live `WorkspaceDiffSnapshot` without
    /// coupling `MyGoBridgeHost` to any concrete VCS bridge type.
    pub async fn workspace_diff(&self) -> Option<WorkspaceDiffSnapshot> {
        let snapshot_list: Vec<Arc<dyn PackageBridge>> = self
            .bridges
            .read()
            .ok()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default();
        for bridge in snapshot_list {
            if let Some(diff) = bridge.workspace_diff().await {
                return Some(diff);
            }
        }
        None
    }

    /// Dynamically wires EVERY namespace exported by a resident Gerbil Scheme
    /// `SchemeBridgeBackend` into this registry behind one shared `SwrCache`.
    pub fn register_scheme_backend(
        &self,
        workspace_root: PathBuf,
        backend: Arc<dyn SchemeBridgeBackend>,
    ) -> Vec<String> {
        let shared_cache = SwrCache::new(SCHEME_BRIDGE_CACHE_TTL);
        let namespaces = backend.namespaces();
        for ns in &namespaces {
            let bridge = Arc::new(UnifiedSchemeBridge::new(
                ns.clone(),
                workspace_root.clone(),
                Arc::clone(&backend),
                shared_cache.clone(),
            ));
            self.register(bridge);
        }
        namespaces
    }

    /// Lists all currently registered package namespaces (sorted deterministically).
    pub fn namespaces(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .bridges
            .read()
            .ok()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        names.sort();
        names
    }

    pub fn metrics(&self) -> BridgeMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Parses `<namespace>:<verb>[@<arg>]` and dispatches to the matching
    /// registered `PackageBridge`. Returns `None` when no bridge owns the prefix.
    pub async fn dispatch_query(&self, raw_query: &str) -> Option<Result<Vec<GraphRecordView>, String>> {
        let trimmed = raw_query.trim();
        let (ns_raw, rest) = trimmed.split_once(':')?;
        let ns = ns_raw.trim().to_ascii_lowercase();
        let bridge = self.bridges.read().ok()?.get(&ns).cloned()?;
        self.metrics
            .queries_dispatched
            .fetch_add(1, Ordering::Relaxed);

        let (verb, arg) = match rest.split_once('@') {
            Some((v, a)) => {
                let a = a.trim();
                (v.trim(), if a.is_empty() { None } else { Some(a) })
            }
            None => (rest.trim(), None),
        };
        Some(bridge.query(verb, arg, trimmed).await)
    }

    /// Parses `<namespace>.<action> ...` and dispatches to the matching
    /// registered `PackageBridge`. Returns `Ok(None)` when no bridge owns the command.
    pub async fn dispatch_command(
        &self,
        raw_command: &str,
    ) -> Result<Option<BridgeCommandOutcome>, (String, String)> {
        let trimmed = raw_command.trim();
        let first_tok = trimmed.split_whitespace().next().unwrap_or("");
        let Some((ns_raw, _)) = first_tok.split_once('.') else {
            return Ok(None);
        };
        let ns = ns_raw.trim().to_ascii_lowercase();
        let Some(bridge) = self
            .bridges
            .read()
            .ok()
            .and_then(|m| m.get(&ns).cloned())
        else {
            return Ok(None);
        };
        self.metrics
            .commands_dispatched
            .fetch_add(1, Ordering::Relaxed);
        let owner = bridge.owner_id().to_owned();
        bridge
            .execute_command(trimmed)
            .await
            .map_err(|err| (owner, err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::marlin::projection::v1::GraphVisibleSurface;

    struct EchoPackageBridge {
        cache: SwrCache<String, String>,
        calls: Arc<AtomicU64>,
    }

    impl EchoPackageBridge {
        fn new() -> Self {
            Self {
                cache: SwrCache::new(Duration::from_secs(5)),
                calls: Arc::new(AtomicU64::new(0)),
            }
        }
    }

    #[tonic::async_trait]
    impl PackageBridge for EchoPackageBridge {
        fn namespace(&self) -> &'static str {
            "demo"
        }

        fn owner_id(&self) -> &'static str {
            "marlin.demo"
        }

        async fn query(
            &self,
            verb: &str,
            arg: Option<&str>,
            _raw_query: &str,
        ) -> Result<Vec<GraphRecordView>, String> {
            let key = format!("{verb}:{}", arg.unwrap_or(""));
            let force = verb == "refresh";
            let calls = Arc::clone(&self.calls);
            let bg_calls = Arc::clone(&self.calls);
            let val = self
                .cache
                .get_or_load(
                    key.clone(),
                    force,
                    || async move {
                        let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
                        Ok(format!("v{n}"))
                    },
                    move |_| async move {
                        let n = bg_calls.fetch_add(1, Ordering::SeqCst) + 1;
                        Ok(format!("v{n}"))
                    },
                )
                .await?;
            Ok(vec![GraphRecordView {
                record_id: format!("demo:{key}"),
                surface: GraphVisibleSurface::Workspace as i32,
                kind: "demo.item".to_owned(),
                title: val,
                summary: arg.unwrap_or("").to_owned(),
                source_path: None,
                start_line: None,
                end_line: None,
                facets: HashMap::new(),
            }])
        }

        async fn execute_command(
            &self,
            raw_command: &str,
        ) -> Result<Option<BridgeCommandOutcome>, String> {
            if raw_command == "demo.ping" {
                return Ok(Some(BridgeCommandOutcome {
                    ok: true,
                    summary: "pong".to_owned(),
                    owner_id: self.owner_id().to_owned(),
                    trajectory: None,
                }));
            }
            Ok(None)
        }
    }

    #[tokio::test]
    async fn unified_registry_routes_queries_commands_and_swr_cache() {
        let reg = BridgeRegistry::new();
        let demo = Arc::new(EchoPackageBridge::new());
        reg.register(demo.clone());
        assert_eq!(reg.namespaces(), vec!["demo".to_owned()]);

        // First query misses cache -> v1
        let r1 = reg
            .dispatch_query("demo:status@/repo")
            .await
            .expect("handled")
            .expect("ok");
        assert_eq!(r1[0].title, "v1");

        // Second query hits SwrCache -> still v1 without re-invoking loader
        let r2 = reg
            .dispatch_query("demo:status@/repo")
            .await
            .expect("handled")
            .expect("ok");
        assert_eq!(r2[0].title, "v1");
        assert_eq!(demo.cache.metrics().cache_hits, 1);

        // Command dispatch routes cleanly to demo.ping
        let out = reg
            .dispatch_command("demo.ping")
            .await
            .expect("ok")
            .expect("handled");
        assert!(out.ok);
        assert_eq!(out.summary, "pong");
        assert_eq!(out.owner_id, "marlin.demo");

        // Unregistered namespace returns None cleanly
        assert!(reg.dispatch_query("unknown:foo").await.is_none());
        assert!(reg.dispatch_command("unknown.bar").await.expect("ok").is_none());
    }
}
