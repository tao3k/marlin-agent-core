// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Managed `asp-gateway` child-process supervisor and background Open EDB
//! hydration bridge for `marlin-tui`.
//!
//! - Honors an explicit `MARLIN_ASP_FORWARDING_URL` when already running externally.
//! - Otherwise auto-discovers a local `asp-gateway` binary (`MARLIN_ASP_GATEWAY_BIN`,
//!   sibling `agent-semantic-protocols/target/{release,debug}/asp-gateway`, or `PATH`)
//!   and spawns it lazily (~7ms boot, zero compiler subprocesses until needed) with a
//!   held `stdin` pipe so kernel stdin-EOF guarantees `ORPHANS=0` even on crash.
//! - Cleanly shuts down any owned gateway via authenticated `POST /shutdown` on Drop.

use std::env;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Upper bound on the single JSON handshake line emitted by `asp-gateway` at boot (SM20).
const MAX_ASP_BOOT_LINE_BYTES: usize = 64 * 1024;
/// Upper bound on waiting for `asp-gateway` to emit its single JSON boot line (SM39).
const ASP_GATEWAY_BOOT_TIMEOUT: Duration = Duration::from_secs(5);

/// Minimal allowlisted toolchain environment variables forwarded to managed `asp-gateway` (SM39).
/// Excludes host API/cloud/bridge secrets and dynamic-loader hijack variables.
const ASP_GATEWAY_ENV_ALLOWLIST: &[&str] = &[
    "PATH",
    "HOME",
    "TMPDIR",
    "USER",
    "LOGNAME",
    "LANG",
    "LC_ALL",
    "GERBIL_PATH",
    "GERBIL_LOADPATH",
    "RUSTUP_HOME",
    "CARGO_HOME",
    "XDG_CACHE_HOME",
    "XDG_RUNTIME_DIR",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
];

fn apply_scrubbed_asp_env(cmd: &mut Command) {
    cmd.env_clear();
    for key in ASP_GATEWAY_ENV_ALLOWLIST {
        if let Some(value) = env::var_os(key) {
            cmd.env(key, value);
        }
    }
}

/// Strictly validates a managed child's reported loopback endpoint (`http://127.0.0.1:<port>`
/// or `http://localhost:<port>`), rejecting `@` userinfo, control bytes, paths, and invalid ports.
fn validate_managed_asp_endpoint(raw: &str) -> Option<AspForwardingEndpoint> {
    let trimmed = raw.trim();
    if trimmed.chars().any(char::is_control) {
        return None;
    }
    let rest = trimmed.strip_prefix("http://")?;
    if rest.contains(['@', '/', '?', '#', '\\']) || rest.chars().any(char::is_whitespace) {
        return None;
    }
    let (host, port_raw) = rest.split_once(':')?;
    if port_raw.contains(':') || !matches!(host, "127.0.0.1" | "localhost") {
        return None;
    }
    let port: u16 = port_raw.parse().ok()?;
    if port == 0 {
        return None;
    }
    AspForwardingEndpoint::new(format!("http://{host}:{port}")).ok()
}

/// Reads a single newline-delimited handshake frame without allowing an unbounded
/// delimiter-free child payload to exhaust host memory (SM20).
fn read_bounded_boot_line<R: Read>(reader: R, max_line_bytes: usize) -> Option<String> {
    let cap = u64::try_from(max_line_bytes.saturating_add(1)).unwrap_or(u64::MAX);
    let mut limited = BufReader::new(reader).take(cap);
    let mut line = String::new();
    let n = limited.read_line(&mut line).ok()?;
    if n == 0 || n > max_line_bytes {
        return None;
    }
    Some(line)
}

use marlin_agent_search_engine::{
    AspForwardingEndpoint, fetch_asp_provider_catalog, shutdown_asp_gateway,
};

/// Live ASP gateway status surfaced to TUI Helm and status messages.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AspLiveStatus {
    pub(crate) connected: bool,
    pub(crate) managed: bool,
    pub(crate) provider_count: usize,
    pub(crate) providers: Vec<String>,
    pub(crate) hydrated_entities: usize,
    pub(crate) hydrating: bool,
    pub(crate) boot_micros: u64,
}

impl AspLiveStatus {
    pub(crate) fn compact_badge(&self) -> String {
        if !self.connected {
            return String::new();
        }
        let suffix = if self.hydrating { "…" } else { "" };
        if self.hydrated_entities > 0 {
            format!(
                "ASP:{}p·{}e{suffix}",
                self.provider_count, self.hydrated_entities
            )
        } else if self.hydrating {
            format!("ASP:{}p…", self.provider_count)
        } else {
            format!("ASP:{}p", self.provider_count)
        }
    }
}

/// Process handle keeping a TUI-owned `asp-gateway` alive for the lifetime of the TUI.
pub(crate) struct AspGatewaySupervisor {
    endpoint: AspForwardingEndpoint,
    status: AspLiveStatus,
    child: Option<Child>,
    stdin_watchdog: Option<ChildStdin>,
}

impl AspGatewaySupervisor {
    /// Connect to an external `MARLIN_ASP_FORWARDING_URL` or lazily boot a local
    /// `asp-gateway` binary. Returns `None` when disabled or unavailable so TUI
    /// remains 100% functional offline using resident trigram/Org/cache facts.
    pub(crate) fn start_or_connect(workspace_root: &Path) -> Option<Self> {
        if let Some(endpoint) = AspForwardingEndpoint::from_env() {
            let started = Instant::now();
            let (connected, providers) = match fetch_asp_provider_catalog(&endpoint) {
                Ok(cat) => (
                    true,
                    cat.providers
                        .into_iter()
                        .map(|p| p.manifest.language_id.as_str().to_owned())
                        .collect::<Vec<_>>(),
                ),
                Err(_) => (false, Vec::new()),
            };
            let boot_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
            return Some(Self {
                endpoint,
                status: AspLiveStatus {
                    connected,
                    managed: false,
                    provider_count: providers.len(),
                    providers,
                    hydrated_entities: 0,
                    hydrating: false,
                    boot_micros,
                },
                child: None,
                stdin_watchdog: None,
            });
        }
        if env::var_os("MARLIN_ASP_DISABLE_AUTO_GATEWAY").is_some() {
            return None;
        }
        let bin = discover_asp_gateway_binary(workspace_root)?;
        spawn_managed_asp_gateway(&bin)
    }

    pub(crate) fn endpoint(&self) -> &AspForwardingEndpoint {
        &self.endpoint
    }

    pub(crate) fn status(&self) -> &AspLiveStatus {
        &self.status
    }
}

impl Drop for AspGatewaySupervisor {
    fn drop(&mut self) {
        if self.child.is_none() {
            return;
        }
        let _ = shutdown_asp_gateway(&self.endpoint);
        let _ = self.stdin_watchdog.take();
        if let Some(mut child) = self.child.take() {
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                if let Ok(Some(_)) = child.try_wait() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn discover_asp_gateway_binary(workspace_root: &Path) -> Option<PathBuf> {
    if let Some(explicit) = env::var_os("MARLIN_ASP_GATEWAY_BIN") {
        let p = PathBuf::from(explicit);
        if p.is_absolute() && p.is_file() {
            return Some(p);
        }
    }
    discover_asp_gateway_binary_with_path(workspace_root, env::var_os("PATH").as_deref())
}

pub(crate) fn discover_asp_gateway_binary_with_path(
    workspace_root: &Path,
    path_var: Option<&std::ffi::OsStr>,
) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(parent) = workspace_root.parent() {
        candidates.push(parent.join("agent-semantic-protocols/target/release/asp-gateway"));
        candidates.push(parent.join("agent-semantic-protocols/target/debug/asp-gateway"));
    }
    if let Ok(exe) = env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join("asp-gateway"));
    }
    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if let Some(path_var) = path_var {
        for dir in env::split_paths(path_var) {
            if !dir.is_absolute() {
                continue;
            }
            let candidate = dir.join("asp-gateway");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(unix)]
fn terminate_child_process_group(child_pid: u32) {
    let Ok(pgid_i32) = i32::try_from(child_pid) else {
        return;
    };
    let Ok(host_pid) = i32::try_from(std::process::id()) else {
        return;
    };
    let host_pgid = unsafe { libc::getpgrp() };
    if pgid_i32 <= 1 || pgid_i32 == host_pid || pgid_i32 == host_pgid {
        return;
    }
    unsafe {
        let _ = libc::kill(-pgid_i32, libc::SIGKILL);
    }
}

struct SpawnedChildGuard {
    child: Option<Child>,
}

impl SpawnedChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn take(&mut self) -> Option<Child> {
        self.child.take()
    }

    fn kill_and_reap_now(&mut self) {
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            terminate_child_process_group(child.id());
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for SpawnedChildGuard {
    fn drop(&mut self) {
        self.kill_and_reap_now();
    }
}

fn read_bounded_boot_line_with_timeout(
    stdout: ChildStdout,
    max_line_bytes: usize,
    boot_timeout: Duration,
    child_guard: &mut SpawnedChildGuard,
    stdin_watchdog: &mut Option<ChildStdin>,
) -> Option<String> {
    let (tx, rx) = mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        let _ = tx.send(read_bounded_boot_line(stdout, max_line_bytes));
    });
    match rx.recv_timeout(boot_timeout) {
        Ok(line) => {
            let _ = reader.join();
            line
        }
        Err(_) => {
            let _ = stdin_watchdog.take();
            child_guard.kill_and_reap_now();
            let _ = reader.join();
            None
        }
    }
}

pub(crate) fn spawn_managed_asp_gateway_with_timeout(
    bin: &Path,
    boot_timeout: Duration,
) -> Option<AspGatewaySupervisor> {
    let started = Instant::now();
    let mut cmd = Command::new(bin);
    apply_scrubbed_asp_env(&mut cmd);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Isolate `asp-gateway` from abrupt PTY SIGHUP/SIGINT so parent death always
        // flows cleanly through the stdin-EOF watchdog -> `port.shutdown_now()`,
        // guaranteeing zero orphan child compilers (`gxi`, `asp-rust`, etc.).
        cmd.process_group(0);
    }
    let child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut child_guard = SpawnedChildGuard::new(child);
    let mut stdin_watchdog = child_guard
        .child
        .as_mut()
        .and_then(|child| child.stdin.take());
    let stdout = child_guard
        .child
        .as_mut()
        .and_then(|child| child.stdout.take())?;
    let line = read_bounded_boot_line_with_timeout(
        stdout,
        MAX_ASP_BOOT_LINE_BYTES,
        boot_timeout,
        &mut child_guard,
        &mut stdin_watchdog,
    )?;
    let boot_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let boot: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    let raw_endpoint = boot["endpoint"].as_str()?;
    let mut endpoint = validate_managed_asp_endpoint(raw_endpoint)?;
    if let Some(token) = boot["controlToken"].as_str() {
        endpoint = endpoint.with_control_token(token);
    }
    let providers: Vec<String> = boot["providers"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    Some(AspGatewaySupervisor {
        endpoint,
        status: AspLiveStatus {
            connected: true,
            managed: true,
            provider_count: providers.len(),
            providers,
            hydrated_entities: 0,
            hydrating: false,
            boot_micros,
        },
        child: child_guard.take(),
        stdin_watchdog,
    })
}

pub(crate) fn spawn_managed_asp_gateway(bin: &Path) -> Option<AspGatewaySupervisor> {
    spawn_managed_asp_gateway_with_timeout(bin, ASP_GATEWAY_BOOT_TIMEOUT)
}

/// Discover `WorkspaceArtifacts` (dev `.data/marlin/` via `.envrc`'s
/// `MARLIN_AGENT_CORE_DEV=1` or enrolled `~/marlin-ai/repos/<owner>/`) and
/// project the declarative Scheme `DefaultArtifactCatalog.workspace-layout`
/// over the zero-JSON C-ABI (`marlin_context_artifacts_exchange`).
#[cfg(feature = "linked-ui")]
pub(super) fn discover_workspace_artifact_layout(
    scheme_runtime: &gerbil_scheme::GerbilRuntime,
    runtime: &tokio::runtime::Runtime,
    directory: &Path,
) -> Result<Option<marlin_agent_context::artifacts::WorkspaceArtifactLayout>, String> {
    let trace = env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some();
    let artifacts = match runtime
        .block_on(marlin_agent_context::artifacts::WorkspaceArtifacts::discover(directory))
    {
        Ok(Some(artifacts)) => artifacts,
        Ok(None) => {
            if trace {
                eprintln!("marlin-tui startup artifacts layout: None (not enrolled / dev off)");
            }
            return Ok(None);
        }
        Err(err) => {
            if trace {
                eprintln!("marlin-tui startup artifacts discover error: {err}");
            }
            return Err(format!("artifact discovery failed: {err}"));
        }
    };
    match artifacts.project_layout(scheme_runtime, |request| {
        Ok(crate::linked_ui::artifacts_exchange(
            scheme_runtime,
            request,
        )?)
    }) {
        Ok(layout) => Ok(Some(layout)),
        Err(err) => {
            if trace {
                eprintln!("marlin-tui startup artifacts project_layout error: {err}");
            }
            Err(format!("artifact layout projection failed: {err}"))
        }
    }
}

/// First-frame non-blocking bootstrap of `asp-gateway`, dynamic POO provider
/// descriptors, Helm TAB completions, and resident workspace index warm-up.
pub(super) fn bootstrap_first_frame_search_and_asp(
    app: &mut super::App,
    #[cfg(feature = "linked-ui")] scheme_runtime: &gerbil_scheme::GerbilRuntime,
    runtime: &tokio::runtime::Handle,
) -> Option<AspGatewaySupervisor> {
    let asp_workspace = app
        .context_worktree
        .clone()
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let supervisor = AspGatewaySupervisor::start_or_connect(&asp_workspace);
    #[cfg(feature = "linked-ui")]
    {
        if let Ok(policy) = super::super::linked_ui::helm_index_policy(scheme_runtime) {
            app.search_hydration_policy = Some(policy.hydration);
            app.search_presets = policy.presets;
            app.search_index_config = policy.index;
        }
        if let Some(sup) = supervisor.as_ref() {
            if let Err(error) = super::super::linked_ui::sync_asp_provider_descriptors(
                scheme_runtime,
                sup.endpoint(),
            ) {
                let detail = error.to_string();
                app.publish_message(super::MessageEvent {
                    source: super::MessageSource::Workspace,
                    severity: super::MessageSeverity::Warning,
                    summary: "ASP provider descriptor sync failed",
                    detail: &detail,
                });
            }
            let asp_status = sup.status().clone();
            let detail = format!(
                "{} ({} providers: [{}], boot={}µs)",
                if !asp_status.connected {
                    "External asp-gateway unreachable"
                } else if asp_status.managed {
                    "Managed asp-gateway started"
                } else {
                    "Connected to external asp-gateway"
                },
                asp_status.provider_count,
                asp_status.providers.join(", "),
                asp_status.boot_micros,
            );
            app.publish_message(super::MessageEvent {
                source: super::MessageSource::Workspace,
                severity: if asp_status.connected {
                    super::MessageSeverity::Info
                } else {
                    super::MessageSeverity::Warning
                },
                summary: if asp_status.connected {
                    "ASP semantic gateway ready"
                } else {
                    "ASP semantic gateway degraded"
                },
                detail: &detail,
            });
            app.attach_asp_endpoint(sup.endpoint().clone(), asp_status, runtime);
        }
        match super::super::linked_ui::search_dsl_completions(scheme_runtime, "") {
            Ok(catalog) => {
                let merged = if let Some(sup) = supervisor.as_ref()
                    && let Ok(provider_cat) = fetch_asp_provider_catalog(sup.endpoint())
                {
                    marlin_agent_search_engine::merge_completion_catalogs(
                        catalog,
                        marlin_agent_search_engine::project_provider_catalog_completions(
                            &provider_cat,
                        ),
                    )
                } else {
                    catalog
                };
                app.search_dsl_catalog = merged;
            }
            Err(error) => {
                if let Some(sup) = supervisor.as_ref()
                    && let Ok(provider_cat) = fetch_asp_provider_catalog(sup.endpoint())
                {
                    app.search_dsl_catalog =
                        marlin_agent_search_engine::project_provider_catalog_completions(
                            &provider_cat,
                        );
                }
                let detail = error.to_string();
                app.publish_message(super::MessageEvent {
                    source: super::MessageSource::Workspace,
                    severity: super::MessageSeverity::Warning,
                    summary: "Search DSL completion catalog unavailable",
                    detail: &detail,
                });
            }
        }
        let index_cfg = app.search_index_config.clone();
        app.warm_search_index(index_cfg, runtime);
    }
    #[cfg(not(feature = "linked-ui"))]
    {
        if let Some(sup) = supervisor.as_ref() {
            if let Ok(provider_cat) = fetch_asp_provider_catalog(sup.endpoint()) {
                app.search_dsl_catalog =
                    marlin_agent_search_engine::project_provider_catalog_completions(&provider_cat);
            }
            app.attach_asp_endpoint(sup.endpoint().clone(), sup.status().clone(), runtime);
        }
        app.warm_search_index(
            marlin_agent_search_engine::WorkspaceIndexConfig::default(),
            runtime,
        );
    }
    supervisor
}

impl super::App {
    fn resolve_search_cache_paths(&self, workspace_root: &Path) -> (PathBuf, PathBuf) {
        if let Some(dir) = self.search_cache_dir.as_ref() {
            (
                dir.join("workspace.asp-index.bin"),
                dir.join("workspace.open-edb.v1.json"),
            )
        } else {
            (
                marlin_agent_search_engine::default_workspace_snapshot_path(workspace_root),
                marlin_agent_search_engine::default_workspace_edb_cache_path(workspace_root),
            )
        }
    }

    pub(crate) fn warm_search_index(
        &mut self,
        config: marlin_agent_search_engine::WorkspaceIndexConfig,
        runtime: &tokio::runtime::Handle,
    ) {
        self.search_index_config = config.clone();
        if self.search_index.is_some() || self.search_index_warming.is_some() {
            return;
        }
        let workspace_root = self
            .context_worktree
            .clone()
            .or_else(|| env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let (snapshot_path, edb_cache_path) = self.resolve_search_cache_paths(&workspace_root);
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.search_index_warming = Some(rx);
        runtime.spawn_blocking(move || {
            if tx.is_closed() {
                return;
            }
            let mut idx = marlin_agent_search_engine::open_or_build_workspace_index_safe(
                &workspace_root,
                &snapshot_path,
                config,
            );
            if tx.is_closed() {
                return;
            }
            let _ = marlin_agent_search_engine::hydrate_workspace_open_edb_org_and_cache(
                &mut idx,
                &edb_cache_path,
            );
            let built = super::Arc::new(std::sync::RwLock::new(idx));
            let _ = tx.send(built);
        });
    }

    pub(crate) fn sync_search_index_background(&mut self, runtime: &tokio::runtime::Handle) {
        if self.search_index_syncing.is_some() || self.search_index_warming.is_some() {
            return;
        }
        let Some(existing) = self.search_index.clone() else {
            return;
        };
        let workspace_root = self
            .context_worktree
            .clone()
            .or_else(|| env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let (snapshot_path, _) = self.resolve_search_cache_paths(&workspace_root);
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.search_index_syncing = Some(rx);
        runtime.spawn_blocking(move || {
            let mut changed = false;
            if let Ok(mut guard) = existing.write() {
                let receipt = guard.sync_workspace_from_disk();
                if receipt.upserted_files > 0 || receipt.deleted_files > 0 {
                    guard.compact();
                    let _ = marlin_agent_search_engine::save_workspace_snapshot_atomic(
                        &guard,
                        &snapshot_path,
                    );
                    changed = true;
                }
            }
            let _ = tx.send(changed);
        });
    }

    pub(crate) fn attach_asp_endpoint(
        &mut self,
        endpoint: AspForwardingEndpoint,
        status: AspLiveStatus,
        runtime: &tokio::runtime::Handle,
    ) {
        let badge = status.compact_badge();
        self.asp_endpoint = Some(endpoint);
        self.asp_live_status = status;
        if let Some(helm) = self.helm.as_mut() {
            helm.set_asp_status_badge(badge);
        }
        self.kick_asp_http_hydration(runtime);
    }

    pub(crate) fn kick_asp_http_hydration(&mut self, runtime: &tokio::runtime::Handle) {
        if self.asp_hydrating.is_some() {
            return;
        }
        let Some(endpoint) = self.asp_endpoint.clone() else {
            return;
        };
        let Some(shared) = self.search_index.clone() else {
            return;
        };
        let workspace_root = self
            .context_worktree
            .clone()
            .or_else(|| env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let (_, edb_cache_path) = self.resolve_search_cache_paths(&workspace_root);
        let Some(policy) = self.search_hydration_policy.clone() else {
            return;
        };
        let (tx, rx) = tokio::sync::watch::channel::<Option<(usize, usize)>>(None);
        self.asp_hydrating = Some(rx);
        self.asp_live_status.hydrating = true;
        if let Some(helm) = self.helm.as_mut() {
            helm.set_asp_status_badge(self.asp_live_status.compact_badge());
        }
        runtime.spawn_blocking(move || {
            let req = marlin_agent_search_engine::AspChunkedHydrationRequest {
                endpoint: &endpoint,
                workspace_root: &workspace_root,
                cache_path: &edb_cache_path,
                policy: &policy,
            };
            let mut high_water_entities = 0usize;
            let receipt = marlin_agent_search_engine::hydrate_workspace_chunked_via_asp_http(
                &req,
                &shared,
                |entities, remaining| {
                    high_water_entities = high_water_entities.max(entities);
                    let _ = tx.send(Some((high_water_entities, remaining)));
                },
            );
            let final_entities = receipt.map_or(high_water_entities, |r| {
                high_water_entities.max(r.http_ingested_entities)
            });
            let _ = tx.send(Some((final_entities, 0)));
        });
    }

    pub(crate) fn poll_search_index(&mut self, runtime: &tokio::runtime::Handle) -> bool {
        let mut changed = false;
        if let Some(rx) = self.search_index_warming.as_mut() {
            match rx.try_recv() {
                Ok(built) => {
                    self.search_index_warming = None;
                    self.search_index = Some(built.clone());
                    #[cfg(feature = "linked-ui")]
                    self.search_index_handle.attach(built.clone());
                    if let Some(helm) = self.helm.as_mut() {
                        helm.attach_warm_index(built);
                    }
                    self.kick_asp_http_hydration(runtime);
                    changed = true;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    self.search_index_warming = None;
                }
            }
        }
        if let Some(rx) = self.asp_hydrating.as_mut() {
            let (has_update, disconnected) = match rx.has_changed() {
                Ok(true) => (true, false),
                Ok(false) => (false, false),
                Err(_) => (true, true),
            };
            let latest = if has_update {
                *rx.borrow_and_update()
            } else {
                None
            };
            if let Some((entities, remaining)) = latest {
                if entities > 0 {
                    self.asp_live_status.hydrated_entities =
                        self.asp_live_status.hydrated_entities.max(entities);
                }
                self.asp_live_status.hydrating = remaining > 0 && !disconnected;
                if disconnected || remaining == 0 {
                    self.asp_hydrating = None;
                    self.asp_live_status.hydrating = false;
                }
                let badge = self.asp_live_status.compact_badge();
                if let Some(helm) = self.helm.as_mut() {
                    helm.set_asp_status_badge(badge);
                    if (entities > 0 || remaining == 0)
                        && let Some(existing) = self.search_index.clone()
                    {
                        helm.attach_warm_index(existing);
                    }
                }
                changed = true;
            } else if disconnected {
                self.asp_hydrating = None;
                self.asp_live_status.hydrating = false;
                if let Some(helm) = self.helm.as_mut() {
                    helm.set_asp_status_badge(self.asp_live_status.compact_badge());
                }
                changed = true;
            }
        }
        if let Some(rx) = self.search_index_syncing.as_mut() {
            match rx.try_recv() {
                Ok(synced_changes) => {
                    self.search_index_syncing = None;
                    if synced_changes && let Some(existing) = self.search_index.clone() {
                        #[cfg(feature = "linked-ui")]
                        self.search_index_handle.attach(existing.clone());
                        if let Some(helm) = self.helm.as_mut() {
                            helm.attach_warm_index(existing);
                        }
                        self.kick_asp_http_hydration(runtime);
                        changed = true;
                    }
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    self.search_index_syncing = None;
                }
            }
        }
        if let Some(built) = self.helm.as_mut().and_then(super::HelmOverlay::poll) {
            #[cfg(feature = "linked-ui")]
            self.search_index_handle.attach(built.clone());
            self.search_index = Some(built);
            self.kick_asp_http_hydration(runtime);
            changed = true;
        }
        changed
    }

    pub(crate) fn open_helm(
        &mut self,
        initial_query: Option<&str>,
        runtime: &tokio::runtime::Handle,
    ) {
        self.history_search = None;
        self.command_selection = None;
        self.command_selection_locked = false;
        let workspace_root = self
            .context_worktree
            .clone()
            .or_else(|| env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let (snapshot_path, edb_cache_path) = self.resolve_search_cache_paths(&workspace_root);
        self.helm = Some(
            super::HelmOverlay::open(
                initial_query,
                workspace_root,
                &self.command_catalog,
                self.search_index_config.clone(),
                self.search_index.clone(),
                runtime,
            )
            .with_cache_paths(snapshot_path, edb_cache_path)
            .with_search_presets(self.search_presets.clone())
            .with_completion_catalog(self.search_dsl_catalog.clone())
            .with_asp_status_badge(self.asp_live_status.compact_badge()),
        );
        self.sync_search_index_background(runtime);
        self.kick_asp_http_hydration(runtime);
    }

    pub(crate) fn apply_helm_action(&mut self, action: super::HelmAction) {
        match action {
            super::HelmAction::InsertCitations(text) => {
                let existing = self.composer.text();
                let needs_space = !existing.is_empty() && !existing.ends_with(char::is_whitespace);
                if needs_space {
                    self.composer.insert_text(" ");
                }
                self.composer.insert_text(&text);
                self.enter_insert_mode(false);
            }
            super::HelmAction::SelectBuffer(buffer) => {
                self.ui_request = Some(super::UiEvent::SelectBuffer(buffer));
            }
            super::HelmAction::PrepareCommand(name) => {
                let _ = self.prepare_bound_command(&name);
            }
        }
    }
}

#[cfg(test)]
mod stability_tests {
    use super::*;

    #[test]
    fn asp_watch_channel_coalesces_burst_progress_monotonically_in_constant_memory() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let mut app = super::super::App::new(marlin_agent_protocol::ModelEndpoint::new(
            "provider", "model",
        ));
        let (tx, rx) = tokio::sync::watch::channel::<Option<(usize, usize)>>(None);
        app.asp_hydrating = Some(rx);
        app.asp_live_status.connected = true;
        app.asp_live_status.hydrating = true;

        // Emit 5,000 rapid chunk progress updates before a single UI poll tick.
        // Watch channel coalesces in O(1) memory while preserving monotonic watermark.
        let mut high_water = 0usize;
        for step in 1..=5_000usize {
            let entities = step * 3;
            high_water = high_water.max(entities);
            let _ = tx.send(Some((high_water, 5_000 - step + 1)));
        }
        assert!(app.poll_search_index(rt.handle()));
        assert_eq!(app.asp_live_status.hydrated_entities, 15_000);
        assert!(app.asp_live_status.hydrating);
        assert!(app.asp_hydrating.is_some());

        // Final completion + sender drop must clear hydrating cleanly.
        let _ = tx.send(Some((15_042, 0)));
        drop(tx);
        assert!(app.poll_search_index(rt.handle()));
        assert_eq!(app.asp_live_status.hydrated_entities, 15_042);
        assert!(!app.asp_live_status.hydrating);
        assert!(app.asp_hydrating.is_none());
    }

    #[test]
    fn asp_watch_channel_clears_spinner_on_unexpected_sender_drop() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let mut app = super::super::App::new(marlin_agent_protocol::ModelEndpoint::new(
            "provider", "model",
        ));
        let (tx, rx) = tokio::sync::watch::channel::<Option<(usize, usize)>>(None);
        app.asp_hydrating = Some(rx);
        app.asp_live_status.connected = true;
        app.asp_live_status.hydrating = true;
        drop(tx);

        assert!(app.poll_search_index(rt.handle()));
        assert!(!app.asp_live_status.hydrating);
        assert!(app.asp_hydrating.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn spawn_managed_asp_gateway_reaps_child_when_boot_json_is_malformed() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("bad-asp-gateway.sh");
        let pid_file = dir.path().join("child.pid");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\necho $$ > {}\necho 'NOT_VALID_JSON'\nsleep 30\n",
                pid_file.display()
            ),
        )
        .expect("write mock script");
        let mut perms = std::fs::metadata(&script).expect("meta").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).expect("chmod");

        assert!(spawn_managed_asp_gateway(&script).is_none());

        let pid: i32 = std::fs::read_to_string(&pid_file)
            .expect("pid file")
            .trim()
            .parse()
            .expect("pid int");
        // Verify child was killed and reaped (kill -0 must fail with ESRCH).
        let alive = is_pid_alive(pid);
        assert!(
            !alive,
            "SM4 orphan child leak: bad-asp-gateway PID {pid} survived malformed boot JSON"
        );
    }

    #[cfg(unix)]
    fn is_pid_alive(pid: i32) -> bool {
        if pid <= 1 {
            return false;
        }
        // SAFETY: signal 0 probes process existence without delivering a signal.
        let rc = unsafe { libc::kill(pid, 0) };
        if rc == 0 {
            return true;
        }
        std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    }

    #[cfg(unix)]
    #[test]
    fn sm20_spawn_managed_asp_gateway_rejects_oversize_boot_line_and_reaps_child() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("giant-boot-asp-gateway.sh");
        let pid_file = dir.path().join("giant.pid");
        // Emit a valid 256 KiB JSON boot line (> 64 KiB cap) followed by sleep 30.
        // Unhardened BufReader::read_line allocates the entire 256 KiB line and accepts it;
        // SM20 hardened bounded line read must refuse (> 64 KiB) and reap the child PID.
        let pad = "x".repeat(256 * 1024);
        let payload_file = dir.path().join("boot.json");
        std::fs::write(
            &payload_file,
            format!(
                "{{\"endpoint\":\"http://127.0.0.1:1\",\"providers\":[],\"pad\":\"{pad}\"}}\n"
            ),
        )
        .expect("write giant boot json");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\necho $$ > {}\ncat {}\nsleep 30\n",
                pid_file.display(),
                payload_file.display()
            ),
        )
        .expect("write mock script");
        let mut perms = std::fs::metadata(&script).expect("meta").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).expect("chmod");

        assert!(
            spawn_managed_asp_gateway(&script).is_none(),
            "SM20 unbounded read_line accepted a 256 KiB boot frame exceeding 64 KiB cap!"
        );

        let pid: i32 = std::fs::read_to_string(&pid_file)
            .expect("pid file")
            .trim()
            .parse()
            .expect("pid int");
        let alive = is_pid_alive(pid);
        assert!(
            !alive,
            "SM20 orphan child leak: giant-boot-asp-gateway PID {pid} survived oversize boot frame refusal"
        );
    }
}
