// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Production-grade Marlin MyGo Authority Host daemon serving all 6 orthogonal
//! `marlin.{app,ui,gate,plugin,device,projection}.v1` gRPC services over HTTP/2
//! (loopback sidecar plaintext with ephemeral Bearer token, or TLS 1.3 via
//! `MARLIN_BRIDGE_TLS_CERT` + `MARLIN_BRIDGE_TLS_KEY`).
//!
//! Set `MARLIN_BRIDGE_REAL_SESSION=1` to co-host the full durable `AppServer`
//! (`SessionBinding<S>` + `ConversationJournal` + deterministic local
//! multi-chunk `ModelStreamingGateway`) alongside `MyGoBridgeHost`'s five UI,
//! Gate, Plugin, Device, and Projection services on the same HTTP/2 listener.
//!
//! When built with `--features turso-host`, set `MARLIN_BRIDGE_TURSO_PATH` to
//! persist sessions, checkpoints, and cancellation receipts to a local Turso
//! SQLite database that survives full process restarts.

use super::config as mygo_host_config;

use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Write},
    sync::Arc,
    time::Duration,
};

use crate::{
    AppServer, CompositeAtomicEffectExecutor, SessionBinding, SessionOpen, TaskActorControlPlane,
    TaskActorPersistenceHandle,
    ui_grpc::{DispatchedUiCommand, MyGoBridgeHost},
    wire::marlin::{
        gate::v1::{GateKind, PendingGate},
        projection::v1::GraphRecordView,
        ui::v1::{UiCommand, UiKeyBinding, UiTransient},
    },
};
use marlin_agent_kernel::{
    ExecutionCircuitBreaker, GovernanceAdmissionGate, GovernedSubjectName, ToolBudgetUnits,
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    assistant_gateway_message,
};
use marlin_agent_runtime::{
    TokioAgentRuntime, TokioAgentRuntimeBuildRequest,
    plugin::{PublisherId, ReleaseTrust},
};
use marlin_agent_sessions::{AgentSessionContext, ContextVisibility};
use marlin_agent_storage::{
    AgentId, EventId, InMemoryAgentStorage, ProjectId, SessionEventPageRequest, SessionEventRecord,
    SessionEventStore, StorageError, StoragePageLimit, TurnId,
};
#[cfg(feature = "turso-host")]
use marlin_agent_storage::{
    TursoAgentStorage, TursoAgentStorageConfig, TursoBatchTransactionMode, TursoOptimizationProfile,
};
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
};
use tokio::{net::TcpListener, sync::Mutex};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{
    Status,
    transport::{Identity, Server, ServerTlsConfig, server::Router},
};

const HOLD_AFTER_CHUNK1_PREFIX: &str = "[hold-after-chunk-1]";
const ADMISSION_TURN_ID: &str = "app-session";
const ADMISSION_EVENT_ID: &str = "admitted";
const ADMISSION_EVENT_KIND: &str = "marlin.mygo.session.admitted.v1";

/// Deterministic local multi-chunk streaming gateway used when
/// `MARLIN_BRIDGE_REAL_SESSION=1` is enabled without any live external LLM calls.
/// Prompts prefixed with `[hold-after-chunk-1]` emit chunk 1 durably and then
/// wait deterministically so callers can exercise authoritative cancellation.
#[derive(Clone, Copy)]
pub struct DeterministicLocalGateway {
    pub chunk_delay: Duration,
}

impl ModelGateway for DeterministicLocalGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        let prompt = request
            .messages()
            .last()
            .map(|msg| msg.content.clone())
            .unwrap_or_default();
        Box::pin(async move {
            let reply =
                format!("[marlin-core] processed \"{prompt}\" via durable ConversationJournal.");
            Ok(ModelGatewayCompletionResponse::new(
                "marlin-local-deterministic",
                "gpt-marlin-local",
                vec![ModelGatewayCompletionChoice::new(
                    0,
                    assistant_gateway_message(reply),
                    None,
                )],
            ))
        })
    }
}

impl ModelStreamingGateway for DeterministicLocalGateway {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        let prompt = request
            .messages()
            .last()
            .map(|msg| msg.content.clone())
            .unwrap_or_default();
        let hold_after_first = prompt.starts_with(HOLD_AFTER_CHUNK1_PREFIX);
        let delay = self.chunk_delay;
        Box::pin(async move {
            let chunk1 = "[marlin-core] ".to_owned();
            let chunk2 = format!("processed \"{prompt}\" ");
            let chunk3 = "via durable ConversationJournal.".to_owned();
            let full_reply = format!("{chunk1}{chunk2}{chunk3}");
            let stream = async_stream::try_stream! {
                yield ModelStreamEvent::Started {
                    transport: ModelGatewayTransport::Sse,
                };
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(1, chunk1));
                if hold_after_first {
                    tokio::time::sleep(Duration::from_secs(15)).await;
                } else if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(2, chunk2));
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(3, chunk3));
                yield ModelStreamEvent::Completed {
                    response: ModelGatewayCompletionResponse::new(
                        "marlin-local-deterministic",
                        "gpt-marlin-local",
                        vec![ModelGatewayCompletionChoice::new(
                            0,
                            assistant_gateway_message(full_reply),
                            None,
                        )],
                    ),
                };
            };
            Ok(Box::pin(stream) as ModelGatewayEventStream)
        })
    }
}

pub const MAX_RESIDENT_SESSIONS: usize = 64;

/// Bounded LRU cache of resident `SessionBinding<S>` handles refining
/// `BoundedProjectionEviction` (SM3c). Durable session admissions live in `S`,
/// so evicting an idle resident runtime never loses session state and prevents
/// permanent 64-slot host lockout.
pub struct ResidentSessionTable<S> {
    entries: BTreeMap<String, SessionBinding<S>>,
    lru_order: std::collections::VecDeque<String>,
}

impl<S> Default for ResidentSessionTable<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> ResidentSessionTable<S> {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            lru_order: std::collections::VecDeque::with_capacity(MAX_RESIDENT_SESSIONS),
        }
    }

    fn contains_key(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn get_and_touch(&mut self, id: &str) -> Option<SessionBinding<S>>
    where
        S: Clone,
    {
        let binding = self.entries.get(id)?.clone();
        self.touch_lru(id);
        Some(binding)
    }

    fn insert_bounded(&mut self, id: String, binding: SessionBinding<S>) {
        if self.entries.contains_key(&id) {
            self.touch_lru(&id);
        } else {
            while self.entries.len() >= MAX_RESIDENT_SESSIONS {
                if let Some(oldest) = self.lru_order.pop_front() {
                    self.entries.remove(&oldest);
                } else {
                    let _ = self.entries.pop_first();
                }
            }
            self.lru_order.push_back(id.clone());
        }
        self.entries.insert(id, binding);
    }

    fn touch_lru(&mut self, id: &str) {
        if let Some(pos) = self.lru_order.iter().position(|k| k == id) {
            self.lru_order.remove(pos);
        }
        self.lru_order.push_back(id.to_owned());
    }
}

pub async fn lookup_host_session<S>(
    bindings: &Mutex<ResidentSessionTable<S>>,
    storage: S,
    project: ProjectId,
    agent: AgentId,
    id: marlin_agent_storage::SessionId,
    mode: SessionOpen,
) -> Result<SessionBinding<S>, Status>
where
    S: SessionEventStore + Clone + 'static,
{
    let mut guard = bindings.lock().await;
    match mode {
        SessionOpen::Create => {
            if guard.contains_key(id.as_str()) {
                return Err(Status::already_exists("session already exists"));
            }
            // Commit a durable admission event so duplicate Create is rejected
            // and Existing rehydration works cleanly across evictions and restarts.
            storage
                .append_session_event(SessionEventRecord {
                    project_id: project.clone(),
                    session_id: id.clone(),
                    agent_id: agent.clone(),
                    turn_id: TurnId::new(ADMISSION_TURN_ID)
                        .map_err(|_| Status::internal("invalid admission turn id"))?,
                    event_id: EventId::new(ADMISSION_EVENT_ID)
                        .map_err(|_| Status::internal("invalid admission event id"))?,
                    event_kind: ADMISSION_EVENT_KIND.to_owned(),
                    causality_parent_event_id: None,
                    body: b"admitted".to_vec(),
                    created_at_unix_ms: 0,
                })
                .await
                .map_err(|error| match error {
                    StorageError::DuplicateSessionEvent { .. } => {
                        Status::already_exists("session already exists")
                    }
                    _ => Status::internal("failed to persist session admission"),
                })?;
            let (runtime, _events) = TokioAgentRuntime::from_build_request(
                TokioAgentRuntimeBuildRequest::runtime_root(16).with_session(
                    AgentSessionContext::root(
                        id.as_str(),
                        ContextVisibility::default_runtime(),
                    ),
                ),
            );
            let binding = SessionBinding::new(storage, project, agent, runtime)
                .map_err(|_| Status::internal("failed to bind session runtime"))?;
            guard.insert_bounded(id.as_str().to_owned(), binding.clone());
            Ok(binding)
        }
        SessionOpen::Existing => {
            if let Some(existing) = guard.get_and_touch(id.as_str()) {
                return Ok(existing);
            }
            // Check durable storage so evicted and restarted sessions rehydrate cleanly.
            let page = storage
                .list_session_events_page(SessionEventPageRequest::new(
                    project.clone(),
                    id.clone(),
                    StoragePageLimit::new(1).expect("page limit 1 is valid"),
                ))
                .await
                .map_err(|_| Status::internal("failed to query session storage"))?;
            if page.items.is_empty() {
                return Err(Status::not_found("session not opened"));
            }
            let (runtime, _events) = TokioAgentRuntime::from_build_request(
                TokioAgentRuntimeBuildRequest::runtime_root(16).with_session(
                    AgentSessionContext::root(
                        id.as_str(),
                        ContextVisibility::default_runtime(),
                    ),
                ),
            );
            let binding = SessionBinding::new(storage, project, agent, runtime)
                .map_err(|_| Status::internal("failed to rehydrate session runtime"))?;
            guard.insert_bounded(id.as_str().to_owned(), binding.clone());
            Ok(binding)
        }
    }
}

fn build_real_session_router<S>(
    storage: S,
    builder: Server,
    host: MyGoBridgeHost,
    expected_header: Option<Arc<str>>,
    model_endpoint: ModelEndpoint,
    gateway: Arc<dyn ModelStreamingGateway>,
) -> Result<Router, Box<dyn std::error::Error>>
where
    S: SessionEventStore + Clone + 'static,
{
    let project = ProjectId::new("default")?;
    let agent = AgentId::new("assistant")?;
    let bindings = Arc::new(Mutex::new(ResidentSessionTable::<S>::new()));
    let app_server = AppServer::new(model_endpoint, gateway, move |meta| {
        MyGoBridgeHost::check_bearer_header(meta, expected_header.as_deref())
    })?
    .with_task_actors(host.task_actors().clone())
    .with_session_lookup(move |_metadata, id, mode| {
        let bindings = Arc::clone(&bindings);
        let storage = storage.clone();
        let project = project.clone();
        let agent = agent.clone();
        async move { lookup_host_session(&bindings, storage, project, agent, id, mode).await }
    });
    Ok(app_server.into_ui_router_on(builder, host))
}

fn decode_hex_32(hex: &str) -> Result<[u8; 32], String> {
    let trimmed = hex.trim();
    if trimmed.len() != 64 {
        return Err(format!("expected 64 hex chars, got {}", trimmed.len()));
    }
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let byte = u8::from_str_radix(&trimmed[i * 2..i * 2 + 2], 16)
            .map_err(|e| format!("invalid hex at {i}: {e}"))?;
        *slot = byte;
    }
    Ok(out)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

/// Processes dispatched UI hot-swap commands (`model.select`, `thinking.set`)
/// without silently exiting on `broadcast::error::RecvError::Lagged` (SM5c).
pub async fn run_ui_command_router_loop(
    mut cmd_rx: tokio::sync::broadcast::Receiver<DispatchedUiCommand>,
    router_ref: Arc<mygo_host_config::HotSwappableModelGateway>,
    host_for_cmds: MyGoBridgeHost,
) {
    loop {
        match cmd_rx.recv().await {
            Ok(ev) => {
                let cmd = ev.command.trim();
                if let Some(alias) = cmd.strip_prefix("model.select ") {
                    if let Some(new_label) = router_ref.select_alias(alias.trim()) {
                        host_for_cmds.set_model_label(new_label.clone());
                        eprintln!("marlin-mygo-host: hot-switched active LLM route -> {new_label}");
                    }
                } else if let Some(mode) = cmd.strip_prefix("thinking.set ") {
                    let enabled = matches!(
                        mode.trim().to_ascii_lowercase().as_str(),
                        "on" | "true" | "1" | "enabled"
                    );
                    router_ref.set_thinking(enabled);
                    eprintln!(
                        "marlin-mygo-host: updated Deep Think reasoning -> {}",
                        if enabled { "on" } else { "off" }
                    );
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                eprintln!(
                    "marlin-mygo-host: ui command router lagged by {skipped} events; continuing live hot-swap subscription"
                );
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}

/// Authoritative UI surface bundle projected from Gerbil Scheme over native C-ABI FFI.
#[derive(Clone, Debug, Default)]
pub struct SchemeUiSurfaceBundle {
    pub commands: Vec<UiCommand>,
    pub keymap: Vec<UiKeyBinding>,
    pub transients: Vec<UiTransient>,
    pub mode_records: Vec<GraphRecordView>,
}

/// Runs the `marlin-mygo-host` HTTP/2 gRPC authority daemon without embedded Scheme projection.
pub async fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    run_cli_with_scheme_surface(None).await
}

/// Runs the HTTP/2 gRPC authority daemon with an optional native Gerbil Scheme UI surface bundle.
pub async fn run_cli_with_scheme_surface(
    scheme_surface: Option<SchemeUiSurfaceBundle>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_cli_with_host_ready(scheme_surface, None).await
}

/// Runs the HTTP/2 gRPC authority daemon and notifies `on_host_ready` with a cloned
/// `MyGoBridgeHost` handle right before printing `READY`, enabling non-blocking
/// background/main-thread Scheme C-ABI FFI surface projection with instant gRPC boot.
pub async fn run_cli_with_host_ready(
    scheme_surface: Option<SchemeUiSurfaceBundle>,
    on_host_ready: Option<std::sync::mpsc::Sender<MyGoBridgeHost>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr = env::var("MARLIN_BRIDGE_ADDR").unwrap_or_else(|_| "127.0.0.1:0".to_owned());
    let expected_header =
        MyGoBridgeHost::resolve_bridge_auth_header(&bind_addr, env::var("MARLIN_BRIDGE_TOKEN"))?;
    let publisher_id =
        env::var("MARLIN_BRIDGE_PUBLISHER").unwrap_or_else(|_| "tao3k.official".to_owned());
    let real_session =
        env::var("MARLIN_BRIDGE_REAL_SESSION").unwrap_or_else(|_| "0".to_owned()) == "1";

    let mut keys = BTreeMap::new();
    if let Ok(pub_hex) = env::var("MARLIN_BRIDGE_PUBKEY_HEX")
        && !pub_hex.trim().is_empty()
    {
        let pub_bytes = decode_hex_32(&pub_hex)?;
        keys.insert(PublisherId::try_from(publisher_id)?, pub_bytes);
    }

    let bridge_header = expected_header.clone();
    let shared_breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(4096)));
    let shared_governance_gate = Arc::new(GovernanceAdmissionGate::new(shared_breaker));
    let host_gov_actor = GovernedSubjectName::new("marlin-mygo-host")?;
    let mut host = MyGoBridgeHost::new(ReleaseTrust::new(keys), move |meta| {
        MyGoBridgeHost::check_bearer_header(meta, bridge_header.as_deref())
    })
    .with_governance_gate(Arc::clone(&shared_governance_gate));

    if let Some(surface) = scheme_surface {
        host = host
            .with_scheme_ui_surface(surface.commands, surface.keymap, surface.transients)
            .with_scheme_mode_records(surface.mode_records);
    }
    let host_for_hook = host.clone();

    let ws_root = env::var("MARLIN_BRIDGE_WORKSPACE_ROOT")
        .ok()
        .map(|s| std::path::PathBuf::from(s.trim()))
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
    host = host.with_magit_workspace(ws_root.clone());

    if env::var("MARLIN_BRIDGE_DISABLE_WORKSPACE_SEARCH")
        .unwrap_or_else(|_| "0".to_owned())
        != "1"
    {
        let helm_bridge = mygo_host_config::WorkspaceHelmSearchBridge::spawn_background(ws_root);
        host = host.with_graph_query_handler(move |req| helm_bridge.query(req));
    }

    if env::var("MARLIN_BRIDGE_SEED_DEMO_GATE").unwrap_or_else(|_| "1".to_owned()) != "0" {
        let gate_id = env::var("MARLIN_BRIDGE_GATE_ID").unwrap_or_else(|_| "gate-e2e-1".to_owned());
        host.enqueue_pending_gate(PendingGate {
            gate_id,
            kind: GateKind::DeviceCapability as i32,
            turn: None,
            call_id: Some("call-e2e-1".to_owned()),
            tool_or_node_name: "device.ios.notify.send".to_owned(),
            prompt: "Approve device notification delivery on iPhone?".to_owned(),
            details_heading: "platform.ios Notifications".to_owned(),
            choices: Vec::new(),
            card: None,
            created_at_unix_ms: 1,
            expires_at_unix_ms: None,
        });
    }

    let mut builder = Server::builder();
    if let (Ok(cert_path), Ok(key_path)) = (
        env::var("MARLIN_BRIDGE_TLS_CERT"),
        env::var("MARLIN_BRIDGE_TLS_KEY"),
    ) && !cert_path.trim().is_empty()
        && !key_path.trim().is_empty()
    {
        let cert_pem = fs::read(cert_path)?;
        let key_pem = fs::read(key_path)?;
        let identity = Identity::from_pem(cert_pem, key_pem);
        builder = builder.tls_config(ServerTlsConfig::new().identity(identity))?;
    }

    #[cfg_attr(not(feature = "turso-host"), allow(unused_mut))]
    let mut persistence_handle: Option<TaskActorPersistenceHandle> = None;
    let mut cmd_router_handle: Option<tokio::task::JoinHandle<()>> = None;

    let router = if real_session {
        let delay_ms = env::var("MARLIN_BRIDGE_CHUNK_DELAY_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5);
        let chunk_delay = Duration::from_millis(delay_ms);
        let (model_endpoint, active_gateway): (ModelEndpoint, Arc<dyn ModelStreamingGateway>) =
            match mygo_host_config::load_live_gateway_from_env() {
                Ok(Some(live)) => {
                    eprintln!(
                        "marlin-mygo-host: live hot-swappable LLM gateway enabled ({}; routes={:?})",
                        live.label,
                        live.router.available_aliases()
                    );
                    host.set_model_label(live.label);
                    let router_ref = Arc::clone(&live.router);
                    let cmd_rx = host.subscribe_dispatched_ui_commands();
                    let host_for_cmds = host.clone();
                    cmd_router_handle = Some(tokio::spawn(run_ui_command_router_loop(
                        cmd_rx,
                        router_ref,
                        host_for_cmds,
                    )));
                    (live.endpoint, live.gateway)
                }
                Ok(None) => (
                    ModelEndpoint::new("openai", "gpt-marlin-local"),
                    Arc::new(DeterministicLocalGateway { chunk_delay }),
                ),
                Err(err) => {
                    return Err(format!(
                        "marlin-mygo-host: configured live TOML gateway failed to load (refusing silent local fallback): {err}"
                    )
                    .into());
                }
            };
        let turso_path = env::var("MARLIN_BRIDGE_TURSO_PATH")
            .ok()
            .map(|p| p.trim().to_owned())
            .filter(|p| !p.is_empty());

        if let Some(path) = turso_path {
            #[cfg(feature = "turso-host")]
            {
                let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
                    path: std::path::PathBuf::from(path),
                    optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
                    batch_transaction_mode: TursoBatchTransactionMode::Immediate,
                })
                .await?;
                let task_project = ProjectId::new("default")?;
                let task_pointer = marlin_agent_storage::ArtifactPointerKey::new(
                    "pointer:graph-engine:task-actors",
                )?;
                let task_agent = AgentId::new("marlin-mygo-host")?;
                let task_actors =
                    TaskActorControlPlane::rehydrate_or_new(&storage, &task_project, &task_pointer)
                        .await?
                        .with_governance_gate(
                            Arc::clone(&shared_governance_gate),
                            host_gov_actor.clone(),
                        );
                let atomic_exec = Arc::new(
                    CompositeAtomicEffectExecutor::new(
                        storage.clone(),
                        DeterministicLocalGateway { chunk_delay },
                        task_project.clone(),
                        marlin_agent_storage::SessionId::new("marlin-mygo-atomic-pump")?,
                        task_agent.clone(),
                    )
                    .with_governance_gate(
                        Arc::clone(&shared_governance_gate),
                        host_gov_actor.clone(),
                        None,
                    ),
                );
                persistence_handle = Some(task_actors.spawn_storage_persistence_loop(
                    storage.clone(),
                    task_project,
                    task_pointer,
                    task_agent,
                ));
                let host = host
                    .with_governance_gate(Arc::clone(&shared_governance_gate))
                    .with_task_actors(task_actors)
                    .with_atomic_executor(atomic_exec);
                build_real_session_router(
                    storage,
                    builder,
                    host,
                    expected_header,
                    model_endpoint,
                    active_gateway,
                )?
            }
            #[cfg(not(feature = "turso-host"))]
            {
                let _ = path;
                return Err(
                    "MARLIN_BRIDGE_TURSO_PATH requires building marlin-mygo-host with --features turso-host"
                        .into(),
                );
            }
        } else {
            let storage = InMemoryAgentStorage::default();
            let task_actors = TaskActorControlPlane::new().with_governance_gate(
                Arc::clone(&shared_governance_gate),
                host_gov_actor.clone(),
            );
            let atomic_exec = Arc::new(
                CompositeAtomicEffectExecutor::new(
                    storage.clone(),
                    DeterministicLocalGateway { chunk_delay },
                    ProjectId::new("default")?,
                    marlin_agent_storage::SessionId::new("marlin-mygo-atomic-pump")?,
                    AgentId::new("marlin-mygo-host")?,
                )
                .with_governance_gate(
                    Arc::clone(&shared_governance_gate),
                    host_gov_actor,
                    None,
                ),
            );
            let host = host
                .with_governance_gate(shared_governance_gate)
                .with_task_actors(task_actors)
                .with_atomic_executor(atomic_exec);
            build_real_session_router(
                storage,
                builder,
                host,
                expected_header,
                model_endpoint,
                active_gateway,
            )?
        }
    } else {
        host.attach_ui_services(builder.add_service(host.app_service()))
    };

    let listener = TcpListener::bind(&bind_addr).await?;
    let local_addr = listener.local_addr()?;

    if let Some(tx) = on_host_ready {
        let _ = tx.send(host_for_hook);
    }

    println!("READY {local_addr}");
    io::stdout().flush()?;

    router
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown_signal())
        .await?;
    if let Some(handle) = cmd_router_handle {
        handle.abort();
        let _ = handle.await;
    }
    if let Some(handle) = persistence_handle {
        handle.flush_and_stop().await?;
    }
    Ok(())
}

