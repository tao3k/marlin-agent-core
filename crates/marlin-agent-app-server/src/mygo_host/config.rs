// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Live TOML model gateway loader (`example.toml` / `MARLIN_CONFIG`) and
//! resident Rust `WorkspaceSearchIndex` HELM bridge for `marlin-mygo-host`.

use std::{
    collections::{BTreeMap, HashMap},
    env, fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use agent_semantic_search::{
    WorkspaceIndexConfig, WorkspaceSearchHit, WorkspaceSearchIndex, WorkspaceSearchQuery,
};
use crate::wire::marlin::projection::v1::{
    GraphRecordView, GraphVisibleSurface, QueryProjectGraphRequest,
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGateway, ModelGatewayCompletionResponse, ModelGatewayFuture,
    ModelGatewayRequest, ModelGatewayResult,
};
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamOutputLimit, ModelStreamingGateway, WeaverModelGateway,
};
use serde::Deserialize;
use weaver_llm::{AnthropicSettings, Client, Endpoint, Protocol};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_HELM_MAX_HITS: usize = 64;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum WireProtocol {
    OpenAi,
    OpenAiResponses,
    Anthropic,
}

#[derive(Deserialize)]
pub struct HostModelConfig {
    pub provider: String,
    pub model: Option<String>,
    endpoint: Option<String>,
    protocol: WireProtocol,
    credential_env: String,
    anthropic: Option<AnthropicSettings>,
    #[serde(default)]
    pub models: BTreeMap<String, HostModelProfile>,
    #[serde(default = "default_timeout_seconds")]
    timeout_seconds: u64,
}

#[derive(Deserialize)]
pub struct HostModelProfile {
    pub provider: String,
    pub model: Option<String>,
    endpoint: Option<String>,
    protocol: WireProtocol,
    credential_env: String,
    anthropic: Option<AnthropicSettings>,
}

fn default_timeout_seconds() -> u64 {
    120
}

pub trait LiveRouteGateway: ModelGateway + ModelStreamingGateway {}

impl<T: ModelGateway + ModelStreamingGateway> LiveRouteGateway for T {}

pub struct RouteEntry {
    pub label: String,
    pub endpoint: ModelEndpoint,
    pub standard_gateway: Arc<dyn LiveRouteGateway>,
    pub thinking_gateway: Option<Arc<dyn LiveRouteGateway>>,
}

/// Live hot-swappable multi-route `ModelStreamingGateway` built from
/// `example.toml` (`default` + `[models.<alias>]`). Responds immediately to
/// `model.select <alias>` and `thinking.set <on|off>` UI commands without
/// restarting `marlin-mygo-host`.
pub struct HotSwappableModelGateway {
    active_alias: RwLock<String>,
    routes: BTreeMap<String, RouteEntry>,
    thinking_enabled: AtomicBool,
}

impl HotSwappableModelGateway {
    pub fn new(
        initial_alias: String,
        routes: BTreeMap<String, RouteEntry>,
        thinking_enabled: bool,
    ) -> Result<Self, String> {
        if !routes.contains_key(&initial_alias) {
            return Err(format!(
                "initial model alias {initial_alias:?} missing from loaded routes"
            ));
        }
        Ok(Self {
            active_alias: RwLock::new(initial_alias),
            routes,
            thinking_enabled: AtomicBool::new(thinking_enabled),
        })
    }

    pub fn select_alias(&self, alias: &str) -> Option<String> {
        let key = alias.trim();
        let entry = self.routes.get(key)?;
        if let Ok(mut guard) = self.active_alias.write() {
            *guard = key.to_owned();
        }
        Some(entry.label.clone())
    }

    pub fn set_thinking(&self, enabled: bool) -> bool {
        self.thinking_enabled.store(enabled, Ordering::Relaxed);
        enabled
    }


    pub fn active_alias(&self) -> String {
        self.active_alias
            .read()
            .expect("active_alias lock poisoned")
            .clone()
    }

    pub fn active_label(&self) -> String {
        self.current_route()
            .map(|r| r.label.clone())
            .unwrap_or_else(|| "marlin-core".to_owned())
    }

    pub fn available_aliases(&self) -> Vec<String> {
        self.routes.keys().cloned().collect()
    }

    fn current_route(&self) -> Option<&RouteEntry> {
        let alias = self.active_alias.read().ok()?;
        self.routes.get(alias.as_str()).or_else(|| self.routes.values().next())
    }

    fn active_gateway(&self) -> Arc<dyn LiveRouteGateway> {
        let route = self
            .current_route()
            .expect("HotSwappableModelGateway always has at least one route");
        if self.thinking_enabled.load(Ordering::Relaxed)
            && let Some(gw) = &route.thinking_gateway
        {
            return Arc::clone(gw);
        }
        Arc::clone(&route.standard_gateway)
    }
}

impl ModelGateway for HotSwappableModelGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        let gw = self.active_gateway();
        gw.complete(request)
    }
}

impl ModelStreamingGateway for HotSwappableModelGateway {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        let gw = self.active_gateway();
        gw.stream(request)
    }
}

pub struct ResolvedLiveGateway {
    pub endpoint: ModelEndpoint,
    pub gateway: Arc<dyn ModelStreamingGateway>,
    pub router: Arc<HotSwappableModelGateway>,
    pub label: String,
}

/// Attempt to load a hot-swappable live `WeaverModelGateway` route table from
/// `MARLIN_CONFIG` or `MARLIN_BRIDGE_CONFIG` (compatible with
/// `crates/marlin-agent-tui/example.toml`). Returns `Ok(None)` when no config
/// path is provided so deterministic local tests continue to run without live
/// credentials.
pub fn load_live_gateway_from_env() -> Result<Option<ResolvedLiveGateway>, String> {
    let path = env::var("MARLIN_CONFIG")
        .ok()
        .or_else(|| env::var("MARLIN_BRIDGE_CONFIG").ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    let Some(path) = path else {
        return Ok(None);
    };
    let raw = fs::read_to_string(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let cfg: HostModelConfig =
        toml::from_str(&raw).map_err(|e| format!("invalid TOML config {path}: {e}"))?;

    let requested_alias = env::var("MARLIN_BRIDGE_MODEL_ALIAS")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default".to_owned());

    let default_profile = HostModelProfile {
        provider: cfg.provider,
        model: cfg.model,
        endpoint: cfg.endpoint,
        protocol: cfg.protocol,
        credential_env: cfg.credential_env,
        anthropic: cfg.anthropic,
    };

    let mut routes = BTreeMap::new();
    let default_has_thinking = default_profile
        .anthropic
        .as_ref()
        .and_then(|a| a.thinking.as_ref())
        .is_some();

    match build_route_entry("default", &default_profile, cfg.timeout_seconds) {
        Ok(entry) => {
            routes.insert("default".to_owned(), entry);
        }
        Err(err) if requested_alias == "default" => return Err(err),
        Err(err) => {
            eprintln!("marlin-mygo-host: skipping default route ({err})");
        }
    }

    for (alias, profile) in &cfg.models {
        match build_route_entry(alias, profile, cfg.timeout_seconds) {
            Ok(entry) => {
                routes.insert(alias.clone(), entry);
            }
            Err(err) if alias == &requested_alias => return Err(err),
            Err(err) => {
                eprintln!("marlin-mygo-host: skipping optional model route {alias:?} ({err})");
            }
        }
    }

    let initial_entry = routes
        .get(&requested_alias)
        .ok_or_else(|| format!("unknown model alias {requested_alias:?} in {path}"))?;
    let endpoint = initial_entry.endpoint.clone();
    let label = initial_entry.label.clone();

    let router = Arc::new(HotSwappableModelGateway::new(
        requested_alias,
        routes,
        default_has_thinking,
    )?);
    let gateway: Arc<dyn ModelStreamingGateway> = router.clone();

    Ok(Some(ResolvedLiveGateway {
        endpoint,
        gateway,
        router,
        label,
    }))
}

fn build_route_entry(
    alias: &str,
    profile: &HostModelProfile,
    timeout_seconds: u64,
) -> Result<RouteEntry, String> {
    let protocol = match profile.protocol {
        WireProtocol::OpenAi => Protocol::OpenAi,
        WireProtocol::OpenAiResponses => Protocol::OpenAiResponses,
        WireProtocol::Anthropic => Protocol::Anthropic,
    };
    let read_env = |name: &str| env::var(name).ok();
    let (resolved_url, resolved_model) = weaver_llm::resolve_direct_target(
        protocol,
        profile.endpoint.as_deref(),
        profile.model.as_deref(),
        read_env,
    )
    .map_err(|_| {
        format!(
            "missing model/endpoint for provider={} (set ANTHROPIC_BASE_URL or endpoint in TOML)",
            profile.provider
        )
    })?;
    let credential = env::var(&profile.credential_env)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "credential env {} is missing or empty",
                profile.credential_env
            )
        })?;
    let identity = ModelEndpoint::new(profile.provider.clone(), resolved_model.clone());
    identity.validate_contract().map_err(|e| e.to_string())?;

    let build_one = |anthropic_cfg: Option<AnthropicSettings>| -> Result<Arc<dyn LiveRouteGateway>, String> {
        let options = anthropic_cfg
            .map(|settings| settings.options(protocol))
            .transpose()
            .map_err(|e| format!("invalid [anthropic] settings: {e}"))?
            .unwrap_or_default();
        let client = Client::new(Duration::from_secs(timeout_seconds), 8 * 1024 * 1024)
            .map_err(|e| format!("cannot create HTTP client: {e}"))?;
        let endpoint = Endpoint::new(protocol, &resolved_url)
            .map_err(|e| format!("invalid model endpoint {resolved_url}: {e}"))?;
        let gateway = WeaverModelGateway::new(
            client,
            profile.provider.clone().into(),
            endpoint,
            credential.clone(),
        )
        .with_default_options(options)
        .with_stream_output_limit(ModelStreamOutputLimit::new(
            NonZeroUsize::new(MAX_RESPONSE_BYTES).expect("positive limit"),
        ));
        Ok(Arc::new(gateway))
    };

    let thinking_gateway = if let Some(settings) = profile.anthropic {
        Some(build_one(Some(settings))?)
    } else {
        None
    };

    let mut standard_settings = profile.anthropic;
    if let Some(ref mut s) = standard_settings {
        s.thinking = None;
    }
    let standard_gateway = build_one(standard_settings)?;

    let label = if alias == "default" {
        format!("{}/{}", profile.provider, resolved_model)
    } else {
        format!("{alias} · {resolved_model} ({})", profile.provider)
    };
    Ok(RouteEntry {
        label,
        endpoint: identity,
        standard_gateway,
        thinking_gateway,
    })
}

/// Resident zero-copy mmap `WorkspaceSearchIndex` backing `MyGoBridgeHost::query_project_graph`
/// with authentic Rust HELM multi-axis queries (`@lang:`, `@doc:`, `@prefix:`, `@ext:`, `@sym:`, `@graph:`).
#[derive(Clone)]
pub struct WorkspaceHelmSearchBridge {
    index: Arc<RwLock<Option<WorkspaceSearchIndex>>>,
}

impl WorkspaceHelmSearchBridge {
    /// Creates a bridge backed by an already-opened `WorkspaceSearchIndex`.
    pub fn from_index(idx: WorkspaceSearchIndex) -> Self {
        Self {
            index: Arc::new(RwLock::new(Some(idx))),
        }
    }

    pub fn spawn_background(workspace_root: PathBuf) -> Self {
        let index = Arc::new(RwLock::new(None));
        let slot = Arc::clone(&index);
        tokio::task::spawn_blocking(move || {
            let snapshot_path = default_snapshot_path(&workspace_root);
            let idx = WorkspaceSearchIndex::open_or_build_cached(
                &workspace_root,
                &snapshot_path,
                WorkspaceIndexConfig::default(),
            );
            if let Ok(mut guard) = slot.write() {
                *guard = Some(idx);
            }
        });
        Self { index }
    }

    pub fn query(&self, req: &QueryProjectGraphRequest) -> Option<Vec<GraphRecordView>> {
        let guard = self.index.read().ok()?;
        let idx = guard.as_ref()?;
        let parsed = ParsedHelmQuery::parse(&req.query);
        let limit = req
            .page
            .as_ref()
            .map(|p| p.max_items as usize)
            .filter(|&n| n > 0)
            .unwrap_or(DEFAULT_HELM_MAX_HITS)
            .min(128);
        let report = idx.query(&WorkspaceSearchQuery {
            pattern: &parsed.pattern,
            languages: &parsed.languages,
            documents: &parsed.documents,
            path_prefix: parsed.path_prefix.as_deref(),
            extension: parsed.extension.as_deref(),
            symbol: parsed.symbol.as_deref(),
            graph_target: parsed.graph_target.as_deref(),
            max_hits: limit,
            context_radius: 3,
        });
        let digest = report.generation_digest.as_str().to_owned();
        let records = report
            .hits
            .into_iter()
            .enumerate()
            .map(|(i, hit)| hit_to_graph_record(i, hit, &digest))
            .collect();
        Some(records)
    }
}

pub struct ParsedHelmQuery {
    pub pattern: String,
    pub languages: Vec<String>,
    documents: Vec<String>,
    pub path_prefix: Option<String>,
    pub extension: Option<String>,
    pub symbol: Option<String>,
    graph_target: Option<String>,
}

impl ParsedHelmQuery {
    pub fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        let stage = trimmed.split('#').next().unwrap_or(trimmed).trim();
        let mut languages = Vec::new();
        let mut documents = Vec::new();
        let mut path_prefix = None;
        let mut extension = None;
        let mut symbol = None;
        let mut graph_target = None;
        let mut terms = Vec::new();

        for tok in stage.split_whitespace() {
            if let Some(v) = tok.strip_prefix("@lang:") {
                if !v.is_empty() {
                    languages.push(v.to_ascii_lowercase());
                }
            } else if let Some(v) = tok.strip_prefix("@doc:") {
                if !v.is_empty() {
                    documents.push(v.to_owned());
                }
            } else if let Some(v) = tok.strip_prefix("@prefix:") {
                if !v.is_empty() {
                    path_prefix = Some(v.to_owned());
                }
            } else if let Some(v) = tok.strip_prefix("@ext:") {
                if !v.is_empty() {
                    extension = Some(v.trim_start_matches('.').to_ascii_lowercase());
                }
            } else if let Some(v) = tok.strip_prefix("@sym:") {
                if !v.is_empty() {
                    symbol = Some(v.to_owned());
                }
            } else if let Some(v) = tok.strip_prefix("@graph:") {
                if !v.is_empty() {
                    graph_target = Some(v.to_owned());
                }
            } else {
                terms.push(tok);
            }
        }
        Self {
            pattern: terms.join(" "),
            languages,
            documents,
            path_prefix,
            extension,
            symbol,
            graph_target,
        }
    }
}

fn hit_to_graph_record(idx: usize, hit: WorkspaceSearchHit, digest: &str) -> GraphRecordView {
    let start_line = hit.first_line.map(|l| l as u32);
    let end_line = start_line.map(|l| l.saturating_add(hit.context_lines.len().min(8) as u32));
    let title = if let Some(sym) = hit.symbols.first() {
        format!("{} · {}", sym.name, hit.path)
    } else {
        hit.path.clone()
    };
    let mut summary_lines = Vec::new();
    if !hit.first_line_text.trim().is_empty() {
        summary_lines.push(hit.first_line_text.trim().to_owned());
    }
    for ctx in hit.context_lines.iter().take(6) {
        let line = ctx.text.trim_end();
        if !line.is_empty() {
            summary_lines.push(format!("L{}: {}", ctx.line_number, line));
        }
    }
    let lang = hit.language.as_deref().filter(|s| !s.is_empty());
    let kind = if !hit.symbols.is_empty() {
        lang.map_or_else(|| "symbol".to_owned(), |l| format!("symbol.{l}"))
    } else if let Some(l) = lang {
        format!("file.{l}")
    } else {
        "workspace.file".to_owned()
    };
    let mut facets = HashMap::new();
    facets.insert("generation_digest".to_owned(), digest.to_owned());
    if let Some(l) = lang {
        facets.insert("language".to_owned(), l.to_owned());
    }
    GraphRecordView {
        record_id: format!("ws-{idx}-{}", hit.path),
        surface: GraphVisibleSurface::Workspace as i32,
        kind,
        title,
        summary: summary_lines.join("\n"),
        source_path: Some(hit.path),
        start_line,
        end_line,
        facets,
    }
}

fn default_snapshot_path(workspace_root: &Path) -> PathBuf {
    let canonical =
        fs::canonicalize(workspace_root).unwrap_or_else(|_| workspace_root.to_path_buf());
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in canonical.to_string_lossy().as_bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let cache_base = env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .map(|h| PathBuf::from(h).join(".cache"))
        })
        .unwrap_or_else(env::temp_dir);
    cache_base
        .join("marlin")
        .join("search")
        .join(format!("workspace-{hash:016x}.asp-index.bin"))
}

