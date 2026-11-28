// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, Mutex, MutexGuard},
};

use marlin_agent_runtime::plugin::{PanelPluginHost, ReleaseTrust};
use tokio::sync::broadcast;
use tonic::{Request, Status, metadata::MetadataMap};

use crate::wire::marlin::{
    app::v1::app_server_server::AppServerServer,
    device::v1::{
        DeviceCapabilityDescriptor, DeviceInvocationRequest, DeviceInvocationResult,
        device_bridge_service_server::DeviceBridgeServiceServer,
    },
    gate::v1::{PendingGate, approval_gate_service_server::ApprovalGateServiceServer},
    plugin::v1::{ScopedPanelEvent, plugin_service_server::PluginServiceServer},
    projection::v1::{
        GraphRecordView, GraphTrajectoryEvent,
        projection_query_service_server::ProjectionQueryServiceServer,
    },
    ui::v1::{
        UiCommand, UiKeyBinding, UiTransient, UiWorkbenchSnapshot,
        ui_workbench_service_server::UiWorkbenchServiceServer,
    },
};

use super::state::{
    AuthorizeFn, BridgeState, ClaimedGraphHumanDecision, DispatchedUiCommand,
    SchemeOverlayProjection, build_snapshot, default_profile_overlays, default_slots,
};

/// Shared authoritative state for all 6 MyGo gRPC services
/// (`marlin.{app,ui,gate,plugin,device,projection}.v1`).
#[derive(Clone)]
pub struct MyGoBridgeHost {
    pub(super) authorize: Arc<AuthorizeFn>,
    pub(super) state: Arc<Mutex<BridgeState>>,
    pub(super) wb_tx: broadcast::Sender<UiWorkbenchSnapshot>,
    pub(super) gate_tx: broadcast::Sender<PendingGate>,
    pub(super) graph_gate_tx: broadcast::Sender<ClaimedGraphHumanDecision>,
    pub(super) ui_cmd_tx: broadcast::Sender<DispatchedUiCommand>,
    pub(super) trajectory_tx: broadcast::Sender<GraphTrajectoryEvent>,
    pub(super) panel_tx: broadcast::Sender<ScopedPanelEvent>,
    pub(super) device_inv_tx: broadcast::Sender<DeviceInvocationRequest>,
    pub(super) task_actors: crate::TaskActorControlPlane,
    pub(super) atomic_executor: Option<Arc<dyn marlin_agent_runtime::AtomicEffectExecutor>>,
    pub(super) governance_gate: Option<Arc<marlin_agent_kernel::GovernanceAdmissionGate>>,
    pub(super) process_registry: Option<marlin_agent_runtime::RuntimeProcessRegistryHandle>,
    pub(super) bridges: crate::mygo_host::BridgeRegistry,
    pub(super) scheme_modes: Arc<crate::mygo_host::SchemeModesBridge>,
}


impl MyGoBridgeHost {
    pub fn new(
        trust: ReleaseTrust,
        authorize: impl Fn(&MetadataMap) -> Result<(), Status> + Send + Sync + 'static,
    ) -> Self {
        Self::with_profile_overlays(trust, default_profile_overlays(), authorize)
            .expect("built-in Scheme profile overlays must satisfy invariants")
    }

    /// Construct a `MyGoBridgeHost` with an empty publisher trust set (rejecting all untrusted
    /// signed panel releases while serving built-in Scheme profile overlays and gates).
    pub fn new_default_trust(
        authorize: impl Fn(&MetadataMap) -> Result<(), Status> + Send + Sync + 'static,
    ) -> Self {
        Self::new(ReleaseTrust::new(BTreeMap::new()), authorize)
    }

    /// Constant-time byte-slice equality over full length without short-circuit
    /// early exit on mismatched bytes (CWE-208, SM24).
    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        let mut diff = (a.len() ^ b.len()) as u64;
        let max_len = a.len().max(b.len());
        for idx in 0..max_len {
            let lhs = a.get(idx).copied().unwrap_or(0);
            let rhs = b.get(idx).copied().unwrap_or(0);
            diff |= u64::from(lhs ^ rhs);
        }
        std::hint::black_box(diff) == 0
    }

    /// Returns `true` when `bind_addr` targets a loopback host (`127.0.0.1`, `[::1]`, `localhost`).
    fn is_loopback_bind_addr(bind_addr: &str) -> bool {
        let trimmed = bind_addr.trim();
        if let Ok(sock) = trimmed.parse::<std::net::SocketAddr>() {
            return sock.ip().is_loopback();
        }
        let host = if let Some(rest) = trimmed.strip_prefix('[') {
            rest.split(']').next().unwrap_or("")
        } else {
            trimmed.rsplit_once(':').map_or(trimmed, |(h, _)| h)
        };
        host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    }

    /// Canonical fail-closed resolution for `MARLIN_BRIDGE_TOKEN` across all host binaries (SM26):
    /// - Rejects explicit empty or whitespace-only tokens on ALL bind addresses.
    /// - Rejects missing tokens on non-loopback bind addresses (`0.0.0.0`, LAN IPs).
    /// - Never falls back to a hardcoded `"marlin-dev-token"` credential.
    pub fn resolve_bridge_auth_header(
        bind_addr: &str,
        token_env: Result<String, std::env::VarError>,
    ) -> Result<Option<Arc<str>>, String> {
        match token_env {
            Ok(raw) => {
                let token = raw.trim();
                if token.is_empty() {
                    Err(format!(
                        "MARLIN_BRIDGE_TOKEN must not be empty or whitespace-only (bind_addr={bind_addr:?})"
                    ))
                } else {
                    Ok(Some(Arc::from(format!("Bearer {token}"))))
                }
            }
            Err(std::env::VarError::NotPresent) => {
                if Self::is_loopback_bind_addr(bind_addr) {
                    Ok(None)
                } else {
                    Err(format!(
                        "refusing to bind non-loopback address {bind_addr:?} without MARLIN_BRIDGE_TOKEN"
                    ))
                }
            }
            Err(std::env::VarError::NotUnicode(_)) => {
                Err("MARLIN_BRIDGE_TOKEN contains invalid UTF-8".to_owned())
            }
        }
    }

    /// Standard Bearer-token authorization check for gRPC metadata.
    /// When `expected_header` is `None`, all requests are admitted (explicit loopback dev mode).
    /// When `Some(expected)` is blank, whitespace-only, or `"Bearer "` with an empty credential
    /// payload, authentication fails closed (CWE-208, SM24, SM26).
    pub fn check_bearer_header(
        meta: &MetadataMap,
        expected_header: Option<&str>,
    ) -> Result<(), Status> {
        let Some(expected) = expected_header else {
            return Ok(());
        };
        if expected.trim().is_empty() {
            return Err(Status::unauthenticated(
                "configured bearer token must not be blank",
            ));
        }
        if expected.trim().eq_ignore_ascii_case("bearer")
            || expected
                .strip_prefix("Bearer ")
                .is_some_and(|tok| tok.trim().is_empty())
        {
            return Err(Status::unauthenticated(
                "configured bearer credential payload must not be blank",
            ));
        }
        match meta.get("authorization").and_then(|v| v.to_str().ok()) {
            Some(actual)
                if Self::constant_time_eq(actual.as_bytes(), expected.as_bytes()) =>
            {
                Ok(())
            }
            _ => Err(Status::unauthenticated("missing or invalid bearer token")),
        }
    }

    pub fn with_profile_overlays(
        trust: ReleaseTrust,
        overlays: Vec<SchemeOverlayProjection>,
        authorize: impl Fn(&MetadataMap) -> Result<(), Status> + Send + Sync + 'static,
    ) -> Result<Self, String> {
        for o in &overlays {
            o.validate()?;
        }
        let (wb_tx, _) = broadcast::channel(64);
        let (gate_tx, _) = broadcast::channel(64);
        let (graph_gate_tx, _) = broadcast::channel(64);
        let (ui_cmd_tx, _) = broadcast::channel(64);
        let (trajectory_tx, _) = broadcast::channel(128);
        let (panel_tx, _) = broadcast::channel(128);
        let (device_inv_tx, _) = broadcast::channel(64);
        let overlay_map: BTreeMap<i32, SchemeOverlayProjection> = overlays
            .into_iter()
            .map(|o| (o.target_kind as i32, o))
            .collect();
        let state = BridgeState {
            generation: 1,
            profile_name: "desktop-dev".to_owned(),
            focus_id: "composer".to_owned(),
            model_label: "marlin-core".to_owned(),
            session_label: "default".to_owned(),
            overlays: overlay_map,
            slots: default_slots(),
            sessions: BTreeSet::from(["default".to_owned()]),
            pending_gates: BTreeMap::new(),
            claimed_gates: BTreeMap::new(),
            claimed_gate_order: VecDeque::new(),
            panel_host: PanelPluginHost::new(trust),
            receipt_seq: 0,
            registered_devices: BTreeMap::new(),
            device_capabilities: BTreeMap::new(),
            device_order: VecDeque::new(),
            active_leases: BTreeMap::new(),
            device_leases: BTreeMap::new(),
            next_lease_seq: 0,
            device_results: BTreeMap::new(),
            device_result_order: VecDeque::new(),
            capture_keys: BTreeSet::new(),
            capture_key_order: VecDeque::new(),
            artifacts: BTreeMap::new(),
            artifact_order: VecDeque::new(),
            graph_projection_records: Vec::new(),
            graph_trajectory_history: Vec::new(),
            scheme_commands: Vec::new(),
            scheme_keymap: Vec::new(),
            scheme_transients: Vec::new(),
        };
        Ok(Self {
            authorize: Arc::new(authorize),
            state: Arc::new(Mutex::new(state)),
            wb_tx,
            gate_tx,
            graph_gate_tx,
            ui_cmd_tx,
            trajectory_tx,
            panel_tx,
            device_inv_tx,
            task_actors: crate::TaskActorControlPlane::new(),
            atomic_executor: None,
            governance_gate: None,
            process_registry: None,
            bridges: {
                let reg = crate::mygo_host::BridgeRegistry::new();
                let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                reg.register(Arc::new(crate::mygo_host::FsPackageBridge::new(cwd)));
                reg
            },
            scheme_modes: Arc::new(crate::mygo_host::SchemeModesBridge::default()),
        })
    }

    /// Register any custom Rust or Scheme-backed `PackageBridge` module into the
    /// unified host registry (routes `<ns>:*` queries and `<ns>.*` commands).
    pub fn with_package_bridge(
        self,
        bridge: Arc<dyn crate::mygo_host::PackageBridge>,
    ) -> Self {
        self.bridges.register(bridge);
        self
    }

    /// Register a `PackageBridge` as both its `<ns>:*` owner and the fallback
    /// handler for untagged free-text queries (e.g. resident ASP Helm search).
    pub fn with_fallback_bridge(
        self,
        bridge: Arc<dyn crate::mygo_host::PackageBridge>,
    ) -> Self {
        self.bridges.register_fallback(bridge);
        self
    }

    /// Dynamically register or replace a `PackageBridge` module on a running host.
    pub fn register_package_bridge(&self, bridge: Arc<dyn crate::mygo_host::PackageBridge>) {
        self.bridges.register(bridge);
    }

    /// Access the unified `BridgeRegistry` (e.g. for telemetry or package inspection).
    pub fn bridge_registry(&self) -> &crate::mygo_host::BridgeRegistry {
        &self.bridges
    }

    /// Attach workspace-scoped `PackageBridge` modules (`magit` + `fs`) bound to
    /// `workspace_root`, automatically triggering background pre-warming.
    pub fn with_magit_workspace(self, workspace_root: std::path::PathBuf) -> Self {
        let magit = Arc::new(crate::mygo_host::WorkspaceMagitBridge::new(
            workspace_root.clone(),
        ));
        self.bridges.register(magit);
        self.bridges
            .register(Arc::new(crate::mygo_host::FsPackageBridge::new(
                workspace_root,
            )));
        self
    }

    /// Attach Scheme-projected commands, dual keymap (`SPC` <-> `C-x`/`C-c`/`C-h`/`M-`),
    /// and transients decoded directly from Gerbil Scheme via native FFI (`marlin-gerbil-scheme`).
    pub fn with_scheme_ui_surface(
        self,
        commands: Vec<UiCommand>,
        keymap: Vec<UiKeyBinding>,
        transients: Vec<UiTransient>,
    ) -> Self {
        self.set_scheme_ui_surface(commands, keymap, transients);
        self
    }

    /// Replace the live Scheme-projected commands, keymap, and transients and
    /// broadcast the updated `UiWorkbenchSnapshot` to connected MyGo gRPC clients.
    pub fn set_scheme_ui_surface(
        &self,
        commands: Vec<UiCommand>,
        keymap: Vec<UiKeyBinding>,
        transients: Vec<UiTransient>,
    ) {
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        st.scheme_commands = commands;
        st.scheme_keymap = keymap;
        st.scheme_transients = transients;
        st.generation += 1;
        let snap = build_snapshot(&st);
        drop(st);
        let _ = self.wb_tx.send(snap);
    }

    /// Attach Scheme-projected Major/Minor/Polymode registry records decoded
    /// directly from `:marlin-agent-core/ui-modes` over Gerbil C-ABI FFI.
    pub fn with_scheme_mode_records(self, records: Vec<GraphRecordView>) -> Self {
        self.set_scheme_mode_records(records);
        self
    }

    /// Replace the live Scheme-projected Major/Minor/Polymode registry records
    /// via the unified `SchemeModesBridge` (`modes:*`) in `BridgeRegistry`.
    pub fn set_scheme_mode_records(&self, records: Vec<GraphRecordView>) {
        let has_records = !records.is_empty();
        self.scheme_modes.replace_records(records);
        if has_records {
            self.bridges.register(Arc::clone(&self.scheme_modes) as Arc<dyn crate::mygo_host::PackageBridge>);
        }
    }

    /// Update the active model label projected into `UIWorkbenchSnapshot` and
    /// broadcast the updated workbench snapshot to connected UI clients.
    pub fn set_model_label(&self, label: impl Into<String>) {
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        st.model_label = label.into();
        st.generation += 1;
        let snap = build_snapshot(&st);
        drop(st);
        let _ = self.wb_tx.send(snap);
    }

    /// Attach a runtime process registry so `ProjectionQueryService::WatchManagedProcesses`
    /// serves live managed processes and survived post-restart orphans.
    pub fn with_process_registry(
        mut self,
        registry: marlin_agent_runtime::RuntimeProcessRegistryHandle,
    ) -> Self {
        self.process_registry = Some(registry);
        self
    }

    /// Replaces the read-only non-authorizing `GraphRecordView` records served by
    /// `ProjectionQueryService::query_project_graph` (e.g. live Scheme `FleetAuditCapsule`).
    pub fn set_graph_projection_records(&self, records: Vec<GraphRecordView>) {
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        st.graph_projection_records = records;
    }

    /// Appends and broadcasts a read-only `GraphTrajectoryEvent` to all live
    /// `ProjectionQueryService::watch_graph_trajectory` subscribers.
    pub fn publish_graph_trajectory_event(&self, event: GraphTrajectoryEvent) {
        {
            let mut st = self.state.lock().expect("bridge mutex poisoned");
            if st.graph_trajectory_history.len() >= 256 {
                st.graph_trajectory_history.remove(0);
            }
            st.graph_trajectory_history.push(event.clone());
        }
        let _ = self.trajectory_tx.send(event);
    }

    /// Subscribe to claimed one-shot `GraphHumanGate` decision notifications.
    pub fn subscribe_claimed_graph_decisions(
        &self,
    ) -> broadcast::Receiver<ClaimedGraphHumanDecision> {
        self.graph_gate_tx.subscribe()
    }

    /// Subscribe to declarative UI commands dispatched via `DispatchUiEvent(RecordCommand)`.
    pub fn subscribe_dispatched_ui_commands(&self) -> broadcast::Receiver<DispatchedUiCommand> {
        self.ui_cmd_tx.subscribe()
    }

    /// Share an authoritative Session-free `TaskActorControlPlane` with `AppServer`.
    pub fn with_task_actors(mut self, task_actors: crate::TaskActorControlPlane) -> Self {
        self.task_actors = task_actors;
        self
    }

    /// Borrow the Session-free `TaskActorControlPlane` bound to this bridge host.
    pub fn task_actors(&self) -> &crate::TaskActorControlPlane {
        &self.task_actors
    }

    /// Mount a thin domain-agnostic 4-primitive `AtomicEffectExecutor` (e.g.
    /// `CompositeAtomicEffectExecutor` backed by Turso WAL + `ModelGateway`) onto this host.
    pub fn with_atomic_executor(
        mut self,
        executor: Arc<dyn marlin_agent_runtime::AtomicEffectExecutor>,
    ) -> Self {
        self.atomic_executor = Some(executor);
        self
    }

    /// Mount the authoritative `GovernanceAdmissionGate` governing GraphEngine
    /// TaskActor dispatches and physical atomic effects on this bridge host (SM25).
    pub fn with_governance_gate(
        mut self,
        gate: Arc<marlin_agent_kernel::GovernanceAdmissionGate>,
    ) -> Self {
        self.governance_gate = Some(gate);
        self
    }

    /// Borrow the authoritative `GovernanceAdmissionGate` mounted on this host, if any.
    pub fn governance_gate(&self) -> Option<&Arc<marlin_agent_kernel::GovernanceAdmissionGate>> {
        self.governance_gate.as_ref()
    }

    /// Fail-closed control-plane guard ensuring active Operator E-Stop / tripped
    /// `ExecutionCircuitBreaker` blocks every mutating and turn-executing RPC (SM26).
    pub(super) fn ensure_governance_live(&self, rpc_name: &str) -> Result<(), Status> {
        if let Some(gate) = &self.governance_gate {
            match gate.execution_breaker().snapshot() {
                Ok(snap) if !snap.tripped => {}
                Ok(_) => {
                    return Err(Status::permission_denied(format!(
                        "governance E-Stop/circuit breaker tripped: refusing control-plane RPC `{rpc_name}`"
                    )));
                }
                Err(_) => {
                    return Err(Status::permission_denied(format!(
                        "governance breaker unavailable (fail-closed): refusing control-plane RPC `{rpc_name}`"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Borrow the mounted 4-primitive `AtomicEffectExecutor`, if configured.
    pub fn atomic_executor(&self) -> Option<&Arc<dyn marlin_agent_runtime::AtomicEffectExecutor>> {
        self.atomic_executor.as_ref()
    }

    /// Execute a Scheme-emitted batch of primitive `AtomicEffect`s concurrently through the
    /// mounted leaf substrate; fails closed if no executor is mounted.
    pub async fn run_atomic_effects(
        &self,
        effects: Vec<marlin_agent_runtime::AtomicEffect>,
    ) -> Result<Vec<marlin_agent_runtime::AtomicEffectReceipt>, Status> {
        let Some(executor) = &self.atomic_executor else {
            return Err(Status::failed_precondition(
                "atomic effect executor not mounted on MyGoBridgeHost",
            ));
        };
        Ok(marlin_agent_runtime::run_atomic_effect_batch(Arc::clone(executor), effects).await)
    }

    /// Attach the 5 orthogonal UI/Gate/Plugin/Device/Projection services to an
    /// existing Tonic router (e.g. one already serving the real `AppServer`).
    pub fn attach_ui_services(
        &self,
        router: tonic::transport::server::Router,
    ) -> tonic::transport::server::Router {
        router
            .add_service(self.ui_service())
            .add_service(self.gate_service())
            .add_service(self.plugin_service())
            .add_service(self.device_service())
            .add_service(self.projection_service())
    }

    /// Attach all 6 services (`marlin.{app,ui,gate,plugin,device,projection}.v1`)
    /// onto a fresh Tonic `Server` builder.
    pub fn into_standalone_router(self) -> tonic::transport::server::Router {
        let base = tonic::transport::Server::builder().add_service(self.app_service());
        self.attach_ui_services(base)
    }

    /// Serve all 6 orthogonal bridge services (`marlin.{app,ui,gate,plugin,device,projection}.v1`)
    /// on an already-bound `tokio::net::TcpListener`.
    pub async fn serve_standalone_on(
        self,
        listener: tokio::net::TcpListener,
    ) -> Result<(), tonic::transport::Error> {
        let incoming = async_stream::stream! {
            loop {
                yield listener.accept().await.map(|(stream, _)| stream);
            }
        };
        self.into_standalone_router()
            .serve_with_incoming(incoming)
            .await
    }

    /// Enqueue an authoritative pending approval gate visible to connected MyGo shells.
    pub fn enqueue_pending_gate(&self, gate: PendingGate) {
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        st.pending_gates.insert(gate.gate_id.clone(), gate.clone());
        let _ = self.gate_tx.send(gate);
    }

    /// Publish a host-filtered event into subscribed ScopedPanel streams.
    pub fn publish_scoped_event(&self, event: ScopedPanelEvent) {
        let _ = self.panel_tx.send(event);
    }

    /// Dispatch a host-originated device.* invocation over live OpenDeviceChannel streams.
    pub fn dispatch_device_invocation(&self, req: DeviceInvocationRequest) {
        let _ = self.device_inv_tx.send(req);
    }

    /// Look up a DeviceInvocationResult returned by a connected device node.
    pub fn device_result(&self, invocation_id: &str) -> Option<DeviceInvocationResult> {
        let st = self.state.lock().ok()?;
        st.device_results.get(invocation_id).cloned()
    }

    /// Inspect capabilities currently registered for `device_id`, or `None` if unregistered.
    pub fn registered_device_capabilities(
        &self,
        device_id: &str,
    ) -> Option<Vec<DeviceCapabilityDescriptor>> {
        let st = self.state.lock().ok()?;
        if !st.registered_devices.contains_key(device_id) {
            return None;
        }
        st.device_capabilities.get(device_id).cloned()
    }

    pub(super) fn lock_state(&self) -> Result<MutexGuard<'_, BridgeState>, Status> {
        self.state
            .lock()
            .map_err(|_| Status::internal("bridge state lock poisoned"))
    }

    pub fn app_service(&self) -> AppServerServer<Self> {
        AppServerServer::new(self.clone())
    }

    pub fn ui_service(&self) -> UiWorkbenchServiceServer<Self> {
        UiWorkbenchServiceServer::new(self.clone())
    }

    pub fn gate_service(&self) -> ApprovalGateServiceServer<Self> {
        ApprovalGateServiceServer::new(self.clone())
    }

    pub fn plugin_service(&self) -> PluginServiceServer<Self> {
        PluginServiceServer::new(self.clone())
    }

    pub fn device_service(&self) -> DeviceBridgeServiceServer<Self> {
        DeviceBridgeServiceServer::new(self.clone())
    }

    pub fn projection_service(&self) -> ProjectionQueryServiceServer<Self> {
        ProjectionQueryServiceServer::new(self.clone())
    }

    pub(super) fn check_auth<T>(&self, req: &Request<T>) -> Result<(), Status> {
        (self.authorize)(req.metadata())
    }
}
