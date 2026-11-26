// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rust-owned `ui-grpc` host bridging `marlin-agent-runtime` (signed unified
//! panel releases, capability grants) and Scheme `ui-mygo` overlays to the 5
//! MyGo gRPC services (`marlin.{ui,gate,plugin,device,projection}.v1`).

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    pin::Pin,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_runtime::plugin::{
    CapabilityId, ContributionId, PanelActivationId, PanelPluginHost, PanelProjectId,
    PanelRuntimeKind as RuntimePanelKind, PluginDenial, PluginId, PublisherId,
    ReleaseAttestation as RuntimeAttestation, ReleaseTrust, ScopedPanelActionRequest,
    SignedPanelRelease, SlotCardinality as RuntimeSlotCardinality, VerifiedPanelRelease,
};
use tokio::sync::broadcast;
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming, metadata::MetadataMap};

use crate::wire::marlin::{
    app::v1::{
        CancelSessionTurnRequest, CancelSessionTurnResponse, CancellationEvidenceUnavailable,
        GetCapabilitiesRequest, GetCapabilitiesResponse, GetSessionSummaryRequest,
        GetSessionSummaryResponse, GetSessionTurnCancellationRequest,
        GetSessionTurnCancellationResponse, GetSessionTurnStatusRequest,
        GetSessionTurnStatusResponse, OpenSessionRequest, OpenSessionResponse,
        RuntimeRetirementStatus, RuntimeShutdownReceipt, SessionRecoveryState, SessionTurnEvent,
        SessionTurnOutcome, SessionTurnRef, StartSessionTurnRequest, StartSessionTurnResponse,
        StreamTurnRequest, TextCheckpoint, TextDelta, TurnCompleted, TurnEvent, TurnStarted,
        WatchSessionTurnRequest,
        app_server_server::{AppServer, AppServerServer},
        get_session_turn_cancellation_response, session_turn_event, turn_event,
    },
    common::v1::{ArtifactRef, CursorPageInfo, DenialCode, ReceiptRef, TargetKind, TypedDenial},
    device::v1::{
        DeviceCapabilityDescriptor, DeviceHeartbeat, DeviceInvocationRequest,
        DeviceInvocationResult, DownloadArtifactRequest, DownloadArtifactResponse,
        IngestCaptureRequest, IngestCaptureResponse, OpenDeviceChannelRequest,
        OpenDeviceChannelResponse, RegisterDeviceNodeRequest, RegisterDeviceNodeResponse,
        UploadArtifactRequest, UploadArtifactResponse,
        device_bridge_service_server::{DeviceBridgeService, DeviceBridgeServiceServer},
        open_device_channel_request, open_device_channel_response,
    },
    gate::v1::{
        GateDecisionReceipt, GateKind, GetGateReceiptRequest, GetGateReceiptResponse,
        ListPendingGatesRequest, ListPendingGatesResponse, PendingGate,
        SubmitGraphHumanDecisionRequest, SubmitGraphHumanDecisionResponse,
        SubmitToolReviewDecisionRequest, SubmitToolReviewDecisionResponse,
        WatchPendingGatesRequest, WatchPendingGatesResponse,
        approval_gate_service_server::{ApprovalGateService, ApprovalGateServiceServer},
    },
    plugin::v1::{
        DisableWorkflowRequest, DisableWorkflowResponse, EnableWorkflowRequest,
        EnableWorkflowResponse, GetWorkflowEvidenceRequest, GetWorkflowEvidenceResponse,
        InstallSignedReleaseRequest, InstallSignedReleaseResponse, InstalledReleaseSummary,
        ListReleasesRequest, ListReleasesResponse, OpenScopedPanelStreamRequest,
        OpenScopedPanelStreamResponse, PanelAssetDescriptor, PanelRuntimeKind, ProjectGrantReceipt,
        ScopedPanelActionReceipt, ScopedPanelEvent, SetProjectGrantsRequest,
        SetProjectGrantsResponse, SubmitScopedPanelActionRequest, SubmitScopedPanelActionResponse,
        WorkflowActivationReceipt, WorkflowDisableReceipt,
        plugin_service_server::{PluginService, PluginServiceServer},
    },
    projection::v1::{
        FileDiffEntry, GraphRecordView, GraphTrajectoryEvent, GraphVisibleSurface,
        ManagedProcessSnapshot, QueryProjectGraphRequest, QueryProjectGraphResponse,
        SecretVisibility, WatchGraphTrajectoryRequest, WatchGraphTrajectoryResponse,
        WatchManagedProcessesRequest, WatchManagedProcessesResponse, WatchWorkspaceDiffRequest,
        WatchWorkspaceDiffResponse, WorkspaceDiffSnapshot,
        projection_query_service_server::{ProjectionQueryService, ProjectionQueryServiceServer},
    },
    ui::v1::{
        BrowseCommandsRequest, BrowseCommandsResponse, DispatchUiEventRequest,
        DispatchUiEventResponse, OpenWorkbenchRequest, OpenWorkbenchResponse, SlotCardinality,
        SlotDeclaration, UiCommand, UiCommandArguments, UiCommandCandidates, UiElement,
        UiElementSequence, UiEmptyMarker, UiIntent, UiKeyBinding, UiKeyScope, UiLayoutPolicy,
        UiRgb, UiSlotNode, UiSlotSource, UiSpan, UiTextNode, UiTheme, UiTone, UiWorkbenchSnapshot,
        WatchWorkbenchRequest, WatchWorkbenchResponse, dispatch_ui_event_request, ui_element,
        ui_intent,
        ui_workbench_service_server::{UiWorkbenchService, UiWorkbenchServiceServer},
    },
};

type AuthorizeFn = dyn Fn(&MetadataMap) -> Result<(), Status> + Send + Sync;
type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send + 'static>>;

/// Typed Rust projection of one `scheme/ui-mygo` Target Profile Overlay.
/// Mirrors the native Scheme vector emitted by `project-profile-overlay`.
#[derive(Clone, Debug)]
pub struct SchemeOverlayProjection {
    pub target_kind: TargetKind,
    pub profile_name: String,
    pub layout_kind: String,
    pub allowed_runtimes: Vec<PanelRuntimeKind>,
    pub device_protocols: Vec<String>,
    pub slots: Vec<SlotDeclaration>,
}

impl SchemeOverlayProjection {
    /// Enforce the exact Scheme `ui-mygo` and Quint formal security invariants:
    /// 1. `approval.gate` must exist, be locked (`allow_untrusted_plugin == false`),
    ///    and be owned by Tier-0 `host.gate`.
    /// 2. No slot may reference legacy `native.*` renderers.
    /// 3. iOS/Android overlays permit `Declarative` packs only (App Store policy).
    pub fn validate(&self) -> Result<(), String> {
        let mut saw_gate = false;
        for sl in &self.slots {
            if sl.fallback_renderer.starts_with("native.") {
                return Err(format!(
                    "slot {:?} uses disallowed legacy native.* renderer {:?}",
                    sl.slot_id, sl.fallback_renderer
                ));
            }
            if sl.slot_id == "approval.gate" {
                saw_gate = true;
                if sl.allow_untrusted_plugin || sl.fallback_renderer != "host.gate" {
                    return Err(
                        "approval.gate must disable untrusted plugins and bind host.gate"
                            .to_owned(),
                    );
                }
            }
        }
        if !saw_gate {
            return Err("profile overlay missing mandatory approval.gate slot".to_owned());
        }
        if matches!(
            self.target_kind,
            TargetKind::MobileIos | TargetKind::MobileAndroid
        ) && self
            .allowed_runtimes
            .iter()
            .any(|r| *r != PanelRuntimeKind::Declarative)
        {
            return Err(format!(
                "mobile target {:?} permits declarative runtimes only",
                self.target_kind
            ));
        }
        Ok(())
    }
}

/// Shared authoritative state for all 6 MyGo gRPC services
/// (`marlin.{app,ui,gate,plugin,device,projection}.v1`).
#[derive(Clone)]
pub struct MyGoBridgeHost {
    authorize: Arc<AuthorizeFn>,
    state: Arc<Mutex<BridgeState>>,
    wb_tx: broadcast::Sender<UiWorkbenchSnapshot>,
    gate_tx: broadcast::Sender<PendingGate>,
    panel_tx: broadcast::Sender<ScopedPanelEvent>,
    device_inv_tx: broadcast::Sender<DeviceInvocationRequest>,
    task_actors: crate::TaskActorControlPlane,
    atomic_executor: Option<Arc<dyn marlin_agent_runtime::AtomicEffectExecutor>>,
}

struct BridgeState {
    generation: u64,
    profile_name: String,
    focus_id: String,
    model_label: String,
    session_label: String,
    overlays: BTreeMap<i32, SchemeOverlayProjection>,
    slots: Vec<SlotDeclaration>,
    sessions: BTreeSet<String>,
    pending_gates: BTreeMap<String, PendingGate>,
    claimed_gates: BTreeMap<String, GateDecisionReceipt>,
    panel_host: PanelPluginHost,
    receipt_seq: u64,
    registered_devices: BTreeMap<String, String>,
    device_capabilities: BTreeMap<String, Vec<DeviceCapabilityDescriptor>>,
    device_results: BTreeMap<String, DeviceInvocationResult>,
    capture_keys: BTreeSet<String>,
    artifacts: BTreeMap<String, Vec<u8>>,
}

impl MyGoBridgeHost {
    pub fn new(
        trust: ReleaseTrust,
        authorize: impl Fn(&MetadataMap) -> Result<(), Status> + Send + Sync + 'static,
    ) -> Self {
        Self::with_profile_overlays(trust, default_profile_overlays(), authorize)
            .expect("built-in Scheme profile overlays must satisfy invariants")
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
            panel_host: PanelPluginHost::new(trust),
            receipt_seq: 0,
            registered_devices: BTreeMap::new(),
            device_capabilities: BTreeMap::new(),
            device_results: BTreeMap::new(),
            capture_keys: BTreeSet::new(),
            artifacts: BTreeMap::new(),
        };
        Ok(Self {
            authorize: Arc::new(authorize),
            state: Arc::new(Mutex::new(state)),
            wb_tx,
            gate_tx,
            panel_tx,
            device_inv_tx,
            task_actors: crate::TaskActorControlPlane::new(),
            atomic_executor: None,
        })
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
        let st = self.state.lock().expect("bridge mutex poisoned");
        st.device_results.get(invocation_id).cloned()
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

    fn check_auth<T>(&self, req: &Request<T>) -> Result<(), Status> {
        (self.authorize)(req.metadata())
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn mint_receipt(st: &mut BridgeState, domain: &str, material: &[u8]) -> ReceiptRef {
    st.receipt_seq += 1;
    // Deterministic 32-byte digest from domain + seq + material (FNV-1a x 4 lanes).
    let mut digest = [0u8; 32];
    for lane in 0..4u64 {
        let mut h: u64 = 14695981039346656037u64 ^ lane.wrapping_mul(0x9e3779b97f4a7c15);
        for &b in domain.as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
        for &b in &st.receipt_seq.to_be_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
        for &b in material {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
        digest[(lane as usize) * 8..(lane as usize + 1) * 8].copy_from_slice(&h.to_be_bytes());
    }
    ReceiptRef {
        receipt_id: format!("rcpt-{domain}-{}", st.receipt_seq),
        owner_domain: domain.to_owned(),
        digest_sha256: digest.to_vec(),
        committed_at_unix_ms: now_unix_ms(),
    }
}

fn default_slots() -> Vec<SlotDeclaration> {
    vec![
        SlotDeclaration {
            slot_id: "workbench.main".to_owned(),
            cardinality: SlotCardinality::Single as i32,
            fallback_renderer: "host.main".to_owned(),
            allow_untrusted_plugin: true,
        },
        SlotDeclaration {
            slot_id: "workbench.inspector".to_owned(),
            cardinality: SlotCardinality::Single as i32,
            fallback_renderer: "host.inspector".to_owned(),
            allow_untrusted_plugin: true,
        },
        SlotDeclaration {
            slot_id: "workbench.tool-card".to_owned(),
            cardinality: SlotCardinality::Keyed as i32,
            fallback_renderer: "host.tool-card".to_owned(),
            allow_untrusted_plugin: true,
        },
        SlotDeclaration {
            slot_id: "approval.gate".to_owned(),
            cardinality: SlotCardinality::Single as i32,
            fallback_renderer: "host.gate".to_owned(),
            allow_untrusted_plugin: false,
        },
    ]
}

fn default_theme() -> UiTheme {
    UiTheme {
        canvas: Some(UiRgb {
            red: 40,
            green: 44,
            blue: 52,
        }),
        ink: Some(UiRgb {
            red: 187,
            green: 194,
            blue: 207,
        }),
        bar: Some(UiRgb {
            red: 28,
            green: 31,
            blue: 36,
        }),
        accent: Some(UiRgb {
            red: 198,
            green: 120,
            blue: 221,
        }),
        info: Some(UiRgb {
            red: 81,
            green: 175,
            blue: 239,
        }),
        success: Some(UiRgb {
            red: 152,
            green: 190,
            blue: 101,
        }),
        warning: Some(UiRgb {
            red: 236,
            green: 190,
            blue: 123,
        }),
        muted: Some(UiRgb {
            red: 115,
            green: 121,
            blue: 126,
        }),
    }
}

fn default_profile_overlays() -> Vec<SchemeOverlayProjection> {
    let base_slots = default_slots();
    let desktop_runtimes = vec![
        PanelRuntimeKind::Declarative,
        PanelRuntimeKind::WebviewUntrusted,
        PanelRuntimeKind::WasmUiIr,
        PanelRuntimeKind::MygoJsWasm,
    ];
    let desktop_protocols = vec![
        "device.desktop.clipboard".to_owned(),
        "device.desktop.notifications".to_owned(),
        "device.desktop.files-picker".to_owned(),
    ];
    vec![
        SchemeOverlayProjection {
            target_kind: TargetKind::DesktopMacos,
            profile_name: "desktop-dev".to_owned(),
            layout_kind: "desktop-split".to_owned(),
            allowed_runtimes: desktop_runtimes.clone(),
            device_protocols: desktop_protocols.clone(),
            slots: base_slots.clone(),
        },
        SchemeOverlayProjection {
            target_kind: TargetKind::DesktopLinux,
            profile_name: "desktop-dev".to_owned(),
            layout_kind: "desktop-split".to_owned(),
            allowed_runtimes: desktop_runtimes.clone(),
            device_protocols: desktop_protocols.clone(),
            slots: base_slots.clone(),
        },
        SchemeOverlayProjection {
            target_kind: TargetKind::DesktopWindows,
            profile_name: "desktop-dev".to_owned(),
            layout_kind: "desktop-split".to_owned(),
            allowed_runtimes: desktop_runtimes,
            device_protocols: desktop_protocols,
            slots: base_slots.clone(),
        },
        SchemeOverlayProjection {
            target_kind: TargetKind::MobileIos,
            profile_name: "ios-personal".to_owned(),
            layout_kind: "mobile-stack".to_owned(),
            allowed_runtimes: vec![PanelRuntimeKind::Declarative],
            device_protocols: vec![
                "device.ios.photos".to_owned(),
                "device.ios.eventkit".to_owned(),
                "device.ios.contacts".to_owned(),
                "device.ios.voice".to_owned(),
                "device.ios.share-extension".to_owned(),
            ],
            slots: base_slots.clone(),
        },
        SchemeOverlayProjection {
            target_kind: TargetKind::MobileAndroid,
            profile_name: "android-personal".to_owned(),
            layout_kind: "mobile-stack".to_owned(),
            allowed_runtimes: vec![PanelRuntimeKind::Declarative],
            device_protocols: vec![
                "device.android.content-resolver".to_owned(),
                "device.android.notifications".to_owned(),
                "device.android.intents".to_owned(),
            ],
            slots: base_slots.clone(),
        },
        SchemeOverlayProjection {
            target_kind: TargetKind::RemoteHeadless,
            profile_name: "remote-headless".to_owned(),
            layout_kind: "desktop-split".to_owned(),
            allowed_runtimes: vec![PanelRuntimeKind::Declarative],
            device_protocols: Vec::new(),
            slots: base_slots,
        },
    ]
}

fn host_target_kind() -> TargetKind {
    if cfg!(target_os = "macos") {
        TargetKind::DesktopMacos
    } else if cfg!(target_os = "windows") {
        TargetKind::DesktopWindows
    } else {
        TargetKind::DesktopLinux
    }
}

fn profile_for_target(st: &BridgeState, kind: i32) -> &str {
    if let Some(overlay) = st.overlays.get(&kind) {
        return overlay.profile_name.as_str();
    }
    match TargetKind::try_from(kind).unwrap_or(TargetKind::Unspecified) {
        TargetKind::MobileIos => "ios-personal",
        TargetKind::MobileAndroid => "android-personal",
        TargetKind::RemoteHeadless => "remote-headless",
        _ => "desktop-dev",
    }
}

fn build_snapshot(st: &BridgeState) -> UiWorkbenchSnapshot {
    let main_slot = UiElement {
        node: Some(ui_element::Node::Slot(UiSlotNode {
            source: UiSlotSource::NamedPluginSlot as i32,
            slot_id: Some("workbench.main".to_owned()),
        })),
    };
    let status_text = UiElement {
        node: Some(ui_element::Node::Text(UiTextNode {
            spans: vec![UiSpan {
                text: format!("profile={} gen={}", st.profile_name, st.generation),
                tone: UiTone::Info as i32,
            }],
        })),
    };
    UiWorkbenchSnapshot {
        generation: st.generation,
        profile_name: st.profile_name.clone(),
        model_label: st.model_label.clone(),
        session_label: st.session_label.clone(),
        focus_component_id: st.focus_id.clone(),
        theme: Some(default_theme()),
        frame: None,
        layout: Some(UiLayoutPolicy {
            frame_tree: Some(UiElement {
                node: Some(ui_element::Node::Rows(UiElementSequence {
                    children: vec![main_slot, status_text],
                })),
            }),
            role_regions: HashMap::new(),
            max_chatbox_rows: 10,
            max_command_rows: 10,
            transcript_policy: None,
        }),
        regions: Vec::new(),
        commands: vec![
            UiCommand {
                name: "select-buffer".to_owned(),
                description: "Switch active Scheme workbench buffer".to_owned(),
                arguments: UiCommandArguments::Optional as i32,
                capability: "ui.buffer".to_owned(),
                fixed_value: None,
            },
            UiCommand {
                name: "split-vertical".to_owned(),
                description: "Split workbench pane vertically".to_owned(),
                arguments: UiCommandArguments::None as i32,
                capability: "ui.layout".to_owned(),
                fixed_value: None,
            },
        ],
        keymap: vec![UiKeyBinding {
            scope: UiKeyScope::Normal as i32,
            sequence: vec!["SPC".to_owned(), "w".to_owned(), "v".to_owned()],
            action_kind: "invoke".to_owned(),
            command_name: Some("split-vertical".to_owned()),
        }],
        transients: Vec::new(),
        slots: st.slots.clone(),
    }
}

fn map_denial(err: PluginDenial) -> TypedDenial {
    let (code, msg) = match err {
        PluginDenial::UnknownPublisher => (DenialCode::Unauthorized, "unknown publisher key"),
        PluginDenial::InvalidSignature => {
            (DenialCode::InvalidSignature, "invalid Ed25519 signature")
        }
        PluginDenial::UnsupportedContract => (
            DenialCode::UnsupportedContract,
            "unsupported plugin contract",
        ),
        PluginDenial::ScopeMismatch => (
            DenialCode::CapabilityNotGranted,
            "restricted slot or unauthorized scope",
        ),
        PluginDenial::StaleActivation => (DenialCode::StaleGeneration, "stale plugin generation"),
        _ => (DenialCode::InvalidDeclaration, "invalid plugin declaration"),
    };
    TypedDenial {
        code: code as i32,
        message: msg.to_owned(),
        owner_id: Some("marlin-agent-runtime".to_owned()),
        details: HashMap::new(),
    }
}

fn to_proto_runtime(rt: RuntimePanelKind) -> i32 {
    match rt {
        RuntimePanelKind::Declarative => PanelRuntimeKind::Declarative as i32,
        RuntimePanelKind::WebviewUntrusted => PanelRuntimeKind::WebviewUntrusted as i32,
        RuntimePanelKind::WasmUiIr => PanelRuntimeKind::WasmUiIr as i32,
        RuntimePanelKind::MygoJsWasm => PanelRuntimeKind::MygoJsWasm as i32,
    }
}

fn to_proto_cardinality(c: RuntimeSlotCardinality) -> i32 {
    match c {
        RuntimeSlotCardinality::Single => SlotCardinality::Single as i32,
        RuntimeSlotCardinality::List => SlotCardinality::List as i32,
        RuntimeSlotCardinality::Keyed => SlotCardinality::Keyed as i32,
    }
}

#[tonic::async_trait]
impl UiWorkbenchService for MyGoBridgeHost {
    type WatchWorkbenchStream = BoxStream<WatchWorkbenchResponse>;

    async fn open_workbench(
        &self,
        request: Request<OpenWorkbenchRequest>,
    ) -> Result<Response<OpenWorkbenchResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        if !req.session_id.is_empty() {
            st.session_label = req.session_id;
        }
        let resolved_profile = req
            .preferred_profile
            .unwrap_or_else(|| profile_for_target(&st, req.client_target_kind).to_owned());
        st.profile_name = resolved_profile;
        if let Some(overlay) = st.overlays.get(&req.client_target_kind) {
            st.slots = overlay.slots.clone();
        }
        let snap = build_snapshot(&st);
        Ok(Response::new(OpenWorkbenchResponse {
            workbench: Some(snap),
        }))
    }

    async fn watch_workbench(
        &self,
        request: Request<WatchWorkbenchRequest>,
    ) -> Result<Response<Self::WatchWorkbenchStream>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let initial = {
            let st = self.state.lock().expect("bridge mutex poisoned");
            if st.generation > req.after_generation {
                Some(build_snapshot(&st))
            } else {
                None
            }
        };
        let mut rx = self.wb_tx.subscribe();
        let stream = async_stream::try_stream! {
            if let Some(snap) = initial {
                yield WatchWorkbenchResponse { workbench: Some(snap) };
            }
            while let Ok(snap) = rx.recv().await {
                yield WatchWorkbenchResponse { workbench: Some(snap) };
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn dispatch_ui_event(
        &self,
        request: Request<DispatchUiEventRequest>,
    ) -> Result<Response<DispatchUiEventResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        if req.expected_generation != 0 && req.expected_generation != st.generation {
            return Ok(Response::new(DispatchUiEventResponse {
                admitted: false,
                intent: None,
                next_workbench: None,
                denial: Some(TypedDenial {
                    code: DenialCode::StaleGeneration as i32,
                    message: format!(
                        "expected generation {}, current {}",
                        req.expected_generation, st.generation
                    ),
                    owner_id: Some("marlin.ui.v1".to_owned()),
                    details: HashMap::new(),
                }),
            }));
        }

        let intent = match req.event {
            Some(dispatch_ui_event_request::Event::SelectProfile(prof)) => {
                st.profile_name = prof;
                st.generation += 1;
                Some(UiIntent {
                    intent: Some(ui_intent::Intent::FocusComponentId(st.focus_id.clone())),
                })
            }
            Some(dispatch_ui_event_request::Event::SelectFocus(target)) => {
                st.focus_id = target.clone();
                st.generation += 1;
                Some(UiIntent {
                    intent: Some(ui_intent::Intent::FocusComponentId(target)),
                })
            }
            Some(dispatch_ui_event_request::Event::SelectBuffer(buf)) => {
                st.generation += 1;
                Some(UiIntent {
                    intent: Some(ui_intent::Intent::SelectBufferId(buf)),
                })
            }
            _ => Some(UiIntent {
                intent: Some(ui_intent::Intent::SubmitDraft(UiEmptyMarker {})),
            }),
        };
        let next = build_snapshot(&st);
        let _ = self.wb_tx.send(next.clone());
        Ok(Response::new(DispatchUiEventResponse {
            admitted: true,
            intent,
            next_workbench: Some(next),
            denial: None,
        }))
    }

    async fn browse_commands(
        &self,
        request: Request<BrowseCommandsRequest>,
    ) -> Result<Response<BrowseCommandsResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let st = self.state.lock().expect("bridge mutex poisoned");
        let snap = build_snapshot(&st);
        let q = req.query.to_lowercase();
        let names: Vec<String> = snap
            .commands
            .into_iter()
            .map(|c| c.name)
            .filter(|n| q.is_empty() || n.to_lowercase().contains(&q))
            .collect();
        let selected = names.first().cloned();
        Ok(Response::new(BrowseCommandsResponse {
            candidates: Some(UiCommandCandidates { names, selected }),
        }))
    }
}

#[tonic::async_trait]
impl ApprovalGateService for MyGoBridgeHost {
    type WatchPendingGatesStream = BoxStream<WatchPendingGatesResponse>;

    async fn list_pending_gates(
        &self,
        request: Request<ListPendingGatesRequest>,
    ) -> Result<Response<ListPendingGatesResponse>, Status> {
        self.check_auth(&request)?;
        let st = self.state.lock().expect("bridge mutex poisoned");
        Ok(Response::new(ListPendingGatesResponse {
            gates: st.pending_gates.values().cloned().collect(),
        }))
    }

    async fn watch_pending_gates(
        &self,
        request: Request<WatchPendingGatesRequest>,
    ) -> Result<Response<Self::WatchPendingGatesStream>, Status> {
        self.check_auth(&request)?;
        let existing: Vec<PendingGate> = {
            let st = self.state.lock().expect("bridge mutex poisoned");
            st.pending_gates.values().cloned().collect()
        };
        let mut rx = self.gate_tx.subscribe();
        let stream = async_stream::try_stream! {
            for gate in existing {
                yield WatchPendingGatesResponse { gate: Some(gate) };
            }
            while let Ok(gate) = rx.recv().await {
                yield WatchPendingGatesResponse { gate: Some(gate) };
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn submit_tool_review_decision(
        &self,
        request: Request<SubmitToolReviewDecisionRequest>,
    ) -> Result<Response<SubmitToolReviewDecisionResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        if st.claimed_gates.contains_key(&req.gate_id) {
            return Ok(Response::new(SubmitToolReviewDecisionResponse {
                receipt: Some(GateDecisionReceipt {
                    gate_id: req.gate_id,
                    kind: GateKind::ModelToolReview as i32,
                    claimed: false,
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::StaleOrdinal as i32,
                        message: "one-shot gate already claimed".to_owned(),
                        owner_id: Some("marlin.gate.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        }
        let Some(pending) = st.pending_gates.remove(&req.gate_id) else {
            return Ok(Response::new(SubmitToolReviewDecisionResponse {
                receipt: Some(GateDecisionReceipt {
                    gate_id: req.gate_id,
                    kind: GateKind::ModelToolReview as i32,
                    claimed: false,
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::NotFound as i32,
                        message: "pending gate not found".to_owned(),
                        owner_id: Some("marlin.gate.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        };
        let rref = mint_receipt(&mut st, "gate", req.gate_id.as_bytes());
        let receipt = GateDecisionReceipt {
            gate_id: pending.gate_id.clone(),
            kind: pending.kind,
            claimed: true,
            receipt: Some(rref.clone()),
            denial: None,
        };
        st.claimed_gates
            .insert(pending.gate_id.clone(), receipt.clone());
        let should_dispatch_device = pending.kind == GateKind::DeviceCapability as i32
            && req.decision
                == crate::wire::marlin::gate::v1::ToolReviewDecision::ApproveOnce as i32;
        let session_label = st.session_label.clone();
        drop(st);
        if should_dispatch_device {
            let _ = self.device_inv_tx.send(DeviceInvocationRequest {
                invocation_id: format!("inv-{}", pending.gate_id),
                session_id: session_label,
                capability_id: pending.tool_or_node_name,
                gate_receipt: Some(rref),
                args_json:
                    br#"{"title":"Marlin Approved","body":"Authorized via one-shot Rust gate"}"#
                        .to_vec(),
                timeout_ms: 5_000,
            });
        }
        Ok(Response::new(SubmitToolReviewDecisionResponse {
            receipt: Some(receipt),
        }))
    }

    async fn submit_graph_human_decision(
        &self,
        request: Request<SubmitGraphHumanDecisionRequest>,
    ) -> Result<Response<SubmitGraphHumanDecisionResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        if st.claimed_gates.contains_key(&req.gate_id) {
            return Ok(Response::new(SubmitGraphHumanDecisionResponse {
                receipt: Some(GateDecisionReceipt {
                    gate_id: req.gate_id,
                    kind: GateKind::GraphHumanGate as i32,
                    claimed: false,
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::StaleOrdinal as i32,
                        message: "one-shot gate already claimed".to_owned(),
                        owner_id: Some("marlin.gate.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        }
        let Some(pending) = st.pending_gates.remove(&req.gate_id) else {
            return Ok(Response::new(SubmitGraphHumanDecisionResponse {
                receipt: Some(GateDecisionReceipt {
                    gate_id: req.gate_id,
                    kind: GateKind::GraphHumanGate as i32,
                    claimed: false,
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::NotFound as i32,
                        message: "pending gate not found".to_owned(),
                        owner_id: Some("marlin.gate.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        };
        let rref = mint_receipt(&mut st, "graph-gate", req.gate_id.as_bytes());
        let receipt = GateDecisionReceipt {
            gate_id: pending.gate_id.clone(),
            kind: pending.kind,
            claimed: true,
            receipt: Some(rref),
            denial: None,
        };
        st.claimed_gates.insert(pending.gate_id, receipt.clone());
        Ok(Response::new(SubmitGraphHumanDecisionResponse {
            receipt: Some(receipt),
        }))
    }

    async fn get_gate_receipt(
        &self,
        request: Request<GetGateReceiptRequest>,
    ) -> Result<Response<GetGateReceiptResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let st = self.state.lock().expect("bridge mutex poisoned");
        let receipt = st
            .claimed_gates
            .get(&req.gate_id)
            .cloned()
            .unwrap_or(GateDecisionReceipt {
                gate_id: req.gate_id,
                kind: GateKind::Unspecified as i32,
                claimed: false,
                receipt: None,
                denial: Some(TypedDenial {
                    code: DenialCode::NotFound as i32,
                    message: "gate receipt not found".to_owned(),
                    owner_id: Some("marlin.gate.v1".to_owned()),
                    details: HashMap::new(),
                }),
            });
        Ok(Response::new(GetGateReceiptResponse {
            receipt: Some(receipt),
        }))
    }
}

fn summarize_verified_panel(
    panel_host: &PanelPluginHost,
    verified: &VerifiedPanelRelease,
) -> InstalledReleaseSummary {
    let status = panel_host.active_generation_for(verified.id());
    let enabled = status.enabled;
    let active_generation = status.active_generation;
    let panel_assets: Vec<PanelAssetDescriptor> = verified
        .contributions()
        .iter()
        .map(|(cid, c)| PanelAssetDescriptor {
            contribution_id: cid.as_str().to_owned(),
            runtime: to_proto_runtime(c.panel.runtime),
            slot_id: c.panel.slot_id.as_str().to_owned(),
            cardinality: to_proto_cardinality(c.panel.cardinality),
            slot_key: c.panel.slot_key.clone(),
            entry_path: c.panel.entry_path.clone().unwrap_or_default(),
            archive_sha256: verified.digest().as_bytes().to_vec(),
            byte_size: 0,
            read_scopes: c.panel.read_scopes.clone(),
            declared_actions: c.panel.declared_actions.clone(),
        })
        .collect();
    InstalledReleaseSummary {
        plugin_id: verified.id().as_str().to_owned(),
        version: format!("{:?}", verified.version()),
        publisher_id: verified.attestation().publisher.as_str().to_owned(),
        bundle_digest_sha256: verified.digest().as_bytes().to_vec(),
        enabled,
        active_generation,
        declared_capabilities: Vec::new(),
        granted_capabilities: Vec::new(),
        panel_assets,
    }
}

#[tonic::async_trait]
impl PluginService for MyGoBridgeHost {
    type OpenScopedPanelStreamStream = BoxStream<OpenScopedPanelStreamResponse>;

    async fn list_releases(
        &self,
        request: Request<ListReleasesRequest>,
    ) -> Result<Response<ListReleasesResponse>, Status> {
        self.check_auth(&request)?;
        let st = self.state.lock().expect("bridge mutex poisoned");
        Ok(Response::new(ListReleasesResponse {
            releases: st
                .panel_host
                .installed_releases()
                .map(|v| summarize_verified_panel(&st.panel_host, v))
                .collect(),
        }))
    }

    async fn install_signed_release(
        &self,
        request: Request<InstallSignedReleaseRequest>,
    ) -> Result<Response<InstallSignedReleaseResponse>, Status> {
        self.check_auth(&request)?;
        let Some(env) = request.into_inner().release else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
            }));
        };
        let Some(att_pb) = env.attestation else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
            }));
        };
        let Ok(publisher) = PublisherId::try_from(att_pb.publisher_id) else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
            }));
        };
        let Ok(source_revision): Result<[u8; 20], _> = att_pb.source_revision.try_into() else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
            }));
        };
        let Ok(build_evidence_digest): Result<[u8; 32], _> =
            att_pb.build_evidence_digest.try_into()
        else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
            }));
        };
        let Ok(sig_arr): Result<[u8; 64], _> = env.ed25519_signature.try_into() else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidSignature)),
            }));
        };

        let mut st = self.state.lock().expect("bridge mutex poisoned");
        let installed_id = match st.panel_host.install(SignedPanelRelease {
            manifest: &env.manifest_toml,
            attestation: RuntimeAttestation {
                publisher,
                source_revision,
                build_evidence_digest,
            },
            signature: &sig_arr,
        }) {
            Ok(verified) => verified.id().clone(),
            Err(denial) => {
                return Ok(Response::new(InstallSignedReleaseResponse {
                    installed: false,
                    summary: None,
                    denial: Some(map_denial(denial)),
                }));
            }
        };
        let verified = st
            .panel_host
            .get_release(&installed_id)
            .expect("just installed release must exist");
        let mut summary = summarize_verified_panel(&st.panel_host, verified);
        for asset in &mut summary.panel_assets {
            asset.byte_size = env.manifest_toml.len() as u64;
        }
        Ok(Response::new(InstallSignedReleaseResponse {
            installed: true,
            summary: Some(summary),
            denial: None,
        }))
    }

    async fn enable_workflow(
        &self,
        request: Request<EnableWorkflowRequest>,
    ) -> Result<Response<EnableWorkflowResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let Ok(pid) = PluginId::try_from(req.plugin_id.clone()) else {
            return Ok(Response::new(EnableWorkflowResponse {
                receipt: Some(WorkflowActivationReceipt {
                    activated: false,
                    activation_id: String::new(),
                    generation: 0,
                    effective_settings_sha256: Vec::new(),
                    receipt: None,
                    denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                }),
            }));
        };
        let Ok(cid) = ContributionId::try_from(req.contribution_id.clone()) else {
            return Ok(Response::new(EnableWorkflowResponse {
                receipt: Some(WorkflowActivationReceipt {
                    activated: false,
                    activation_id: String::new(),
                    generation: 0,
                    effective_settings_sha256: Vec::new(),
                    receipt: None,
                    denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                }),
            }));
        };

        let mut st = self.state.lock().expect("bridge mutex poisoned");
        let act = match st.panel_host.enable_panel(
            PanelProjectId::new(req.project_id),
            &pid,
            &cid,
            &req.settings_toml,
        ) {
            Ok(rec) => rec,
            Err(denial) => {
                return Ok(Response::new(EnableWorkflowResponse {
                    receipt: Some(WorkflowActivationReceipt {
                        activated: false,
                        activation_id: String::new(),
                        generation: 0,
                        effective_settings_sha256: Vec::new(),
                        receipt: None,
                        denial: Some(map_denial(denial)),
                    }),
                }));
            }
        };
        let first_scope = act
            .read_scopes
            .iter()
            .find(|t| t.as_str() != "host.secrets")
            .cloned();
        let rref = mint_receipt(&mut st, "plugin-activate", act.activation_id.as_bytes());
        let event_seq = st.receipt_seq;
        drop(st);
        if let Some(topic) = first_scope {
            let _ = self.panel_tx.send(ScopedPanelEvent {
                event_id: format!("ev-activate-{event_seq}"),
                plugin_id: req.plugin_id,
                activation_generation: act.generation,
                topic,
                payload_json: br#"{"nodeId":"verify_receipts","status":"activated"}"#.to_vec(),
            });
        }
        Ok(Response::new(EnableWorkflowResponse {
            receipt: Some(WorkflowActivationReceipt {
                activated: true,
                activation_id: act.activation_id,
                generation: act.generation,
                effective_settings_sha256: act.settings_digest.to_vec(),
                receipt: Some(rref),
                denial: None,
            }),
        }))
    }

    async fn disable_workflow(
        &self,
        request: Request<DisableWorkflowRequest>,
    ) -> Result<Response<DisableWorkflowResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        let activation_id = PanelActivationId::new(req.activation_id);
        match st.panel_host.disable_panel(&activation_id) {
            Ok(closed) => {
                let rref = ReceiptRef {
                    receipt_id: format!("rcpt-plugin-disable-{}", closed.closed_generation),
                    owner_domain: "marlin-agent-runtime".to_owned(),
                    digest_sha256: closed.closure_digest.to_vec(),
                    committed_at_unix_ms: now_unix_ms(),
                };
                Ok(Response::new(DisableWorkflowResponse {
                    receipt: Some(WorkflowDisableReceipt {
                        disabled: true,
                        closed_generation: closed.closed_generation,
                        receipt: Some(rref),
                        denial: None,
                    }),
                }))
            }
            Err(denial) => Ok(Response::new(DisableWorkflowResponse {
                receipt: Some(WorkflowDisableReceipt {
                    disabled: false,
                    closed_generation: 0,
                    receipt: None,
                    denial: Some(map_denial(denial)),
                }),
            })),
        }
    }

    async fn set_project_grants(
        &self,
        request: Request<SetProjectGrantsRequest>,
    ) -> Result<Response<SetProjectGrantsResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let Ok(pid) = PluginId::try_from(req.plugin_id.clone()) else {
            return Ok(Response::new(SetProjectGrantsResponse {
                receipt: Some(ProjectGrantReceipt {
                    updated: false,
                    granted_capabilities: Vec::new(),
                    receipt: None,
                    denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                }),
            }));
        };
        let mut caps = Vec::with_capacity(req.granted_capabilities.len());
        for raw in req.granted_capabilities {
            let Ok(cap) = CapabilityId::try_from(raw) else {
                return Ok(Response::new(SetProjectGrantsResponse {
                    receipt: Some(ProjectGrantReceipt {
                        updated: false,
                        granted_capabilities: Vec::new(),
                        receipt: None,
                        denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                    }),
                }));
            };
            caps.push(cap);
        }
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        match st
            .panel_host
            .set_project_grants(PanelProjectId::new(req.project_id), pid, caps)
        {
            Ok(granted) => {
                let rref = mint_receipt(&mut st, "plugin-grant", req.plugin_id.as_bytes());
                Ok(Response::new(SetProjectGrantsResponse {
                    receipt: Some(ProjectGrantReceipt {
                        updated: true,
                        granted_capabilities: granted
                            .into_iter()
                            .map(|c| c.as_str().to_owned())
                            .collect(),
                        receipt: Some(rref),
                        denial: None,
                    }),
                }))
            }
            Err(denial) => Ok(Response::new(SetProjectGrantsResponse {
                receipt: Some(ProjectGrantReceipt {
                    updated: false,
                    granted_capabilities: Vec::new(),
                    receipt: None,
                    denial: Some(map_denial(denial)),
                }),
            })),
        }
    }

    async fn get_workflow_evidence(
        &self,
        request: Request<GetWorkflowEvidenceRequest>,
    ) -> Result<Response<GetWorkflowEvidenceResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let st = self.state.lock().expect("bridge mutex poisoned");
        let activation_id = PanelActivationId::new(req.activation_id);
        let Some(ev) = st.panel_host.evidence(&activation_id) else {
            return Err(Status::not_found("activation evidence not found"));
        };
        let effect_receipts: Vec<ReceiptRef> = ev
            .actions
            .iter()
            .map(|a| ReceiptRef {
                receipt_id: a.receipt_id.clone(),
                owner_domain: "marlin-agent-runtime".to_owned(),
                digest_sha256: a.receipt_digest.to_vec(),
                committed_at_unix_ms: now_unix_ms(),
            })
            .collect();
        Ok(Response::new(GetWorkflowEvidenceResponse {
            activation_id: ev.activation_id.clone(),
            bundle_digest_sha256: ev.bundle_digest.as_bytes().to_vec(),
            project_digest_sha256: ev.settings_digest.to_vec(),
            effect_receipts,
            replay_bundle: None,
        }))
    }

    async fn open_scoped_panel_stream(
        &self,
        request: Request<OpenScopedPanelStreamRequest>,
    ) -> Result<Response<Self::OpenScopedPanelStreamStream>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let pid = PluginId::try_from(req.plugin_id.clone())
            .map_err(|_| Status::invalid_argument("invalid plugin_id"))?;
        let cid = ContributionId::try_from(req.contribution_id.clone())
            .map_err(|_| Status::invalid_argument("invalid contribution_id"))?;
        let allowed_topics: BTreeSet<String> = {
            let st = self.state.lock().expect("bridge mutex poisoned");
            let project_id = PanelProjectId::new(req.project_id);
            st.panel_host
                .active_read_scopes(&project_id, &pid, &cid, req.activation_generation)
                .map_err(|_| Status::failed_precondition("plugin generation not active"))?
        };
        let mut rx = self.panel_tx.subscribe();
        let plugin_id = req.plugin_id;
        let active_gen = req.activation_generation;
        let stream = async_stream::try_stream! {
            while let Ok(ev) = rx.recv().await {
                if ev.plugin_id == plugin_id
                    && ev.activation_generation == active_gen
                    && ev.topic != "host.secrets"
                    && allowed_topics.contains(&ev.topic)
                {
                    yield OpenScopedPanelStreamResponse { event: Some(ev) };
                }
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn submit_scoped_panel_action(
        &self,
        request: Request<SubmitScopedPanelActionRequest>,
    ) -> Result<Response<SubmitScopedPanelActionResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let Ok(pid) = PluginId::try_from(req.plugin_id.clone()) else {
            return Ok(Response::new(SubmitScopedPanelActionResponse {
                receipt: Some(ScopedPanelActionReceipt {
                    admitted: false,
                    result_json: None,
                    receipt: None,
                    denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                }),
            }));
        };
        let Ok(cid) = ContributionId::try_from(req.contribution_id.clone()) else {
            return Ok(Response::new(SubmitScopedPanelActionResponse {
                receipt: Some(ScopedPanelActionReceipt {
                    admitted: false,
                    result_json: None,
                    receipt: None,
                    denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                }),
            }));
        };
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        let project_id = PanelProjectId::new(req.project_id);
        match st.panel_host.admit_scoped_action(ScopedPanelActionRequest {
            project_id: &project_id,
            plugin_id: &pid,
            contribution_id: &cid,
            generation: req.activation_generation,
            action_name: &req.action_name,
            args_json: &req.args_json,
            idempotency_key: &req.idempotency_key,
        }) {
            Ok(action_rec) => Ok(Response::new(SubmitScopedPanelActionResponse {
                receipt: Some(ScopedPanelActionReceipt {
                    admitted: true,
                    result_json: Some(br#"{"status":"admitted"}"#.to_vec()),
                    receipt: Some(ReceiptRef {
                        receipt_id: action_rec.receipt_id,
                        owner_domain: "marlin-agent-runtime".to_owned(),
                        digest_sha256: action_rec.receipt_digest.to_vec(),
                        committed_at_unix_ms: now_unix_ms(),
                    }),
                    denial: None,
                }),
            })),
            Err(denial) => Ok(Response::new(SubmitScopedPanelActionResponse {
                receipt: Some(ScopedPanelActionReceipt {
                    admitted: false,
                    result_json: None,
                    receipt: None,
                    denial: Some(map_denial(denial)),
                }),
            })),
        }
    }
}

#[tonic::async_trait]
impl DeviceBridgeService for MyGoBridgeHost {
    type OpenDeviceChannelStream = BoxStream<OpenDeviceChannelResponse>;
    type DownloadArtifactStream = BoxStream<DownloadArtifactResponse>;

    async fn register_device_node(
        &self,
        request: Request<RegisterDeviceNodeRequest>,
    ) -> Result<Response<RegisterDeviceNodeResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        let overlay = profile_for_target(&st, req.target_kind).to_owned();
        let lease_id = format!(
            "lease-{}-{}",
            req.device_id,
            st.registered_devices.len() + 1
        );
        st.registered_devices
            .insert(req.device_id.clone(), overlay.clone());
        st.device_capabilities
            .insert(req.device_id, req.capabilities);
        Ok(Response::new(RegisterDeviceNodeResponse {
            lease_id,
            lease_ttl_ms: 60_000,
            active_profile_overlay: overlay,
        }))
    }

    async fn open_device_channel(
        &self,
        request: Request<Streaming<OpenDeviceChannelRequest>>,
    ) -> Result<Response<Self::OpenDeviceChannelStream>, Status> {
        self.check_auth(&request)?;
        let mut inbound = request.into_inner();
        let mut inv_rx = self.device_inv_tx.subscribe();
        let state = Arc::clone(&self.state);
        let stream = async_stream::try_stream! {
            use tokio_stream::StreamExt;
            loop {
                tokio::select! {
                    maybe_frame = inbound.next() => {
                        match maybe_frame {
                            Some(Ok(req)) => {
                                match req.frame {
                                    Some(open_device_channel_request::Frame::CapabilityUpdate(upd)) => {
                                        let mut st = state.lock().expect("bridge mutex poisoned");
                                        st.device_capabilities.insert(upd.lease_id, upd.capabilities);
                                    }
                                    Some(open_device_channel_request::Frame::InvocationResult(res)) => {
                                        let mut st = state.lock().expect("bridge mutex poisoned");
                                        st.device_results.insert(res.invocation_id.clone(), res);
                                    }
                                    Some(open_device_channel_request::Frame::Heartbeat(hb)) => {
                                        yield OpenDeviceChannelResponse {
                                            frame: Some(open_device_channel_response::Frame::Heartbeat(
                                                DeviceHeartbeat { unix_ms: hb.unix_ms },
                                            )),
                                        };
                                    }
                                    None => {}
                                }
                            }
                            Some(Err(_err)) => {
                                break;
                            }
                            None => break,
                        }
                    }
                    inv = inv_rx.recv() => {
                        if let Ok(req) = inv {
                            yield OpenDeviceChannelResponse {
                                frame: Some(open_device_channel_response::Frame::InvocationRequest(req)),
                            };
                        }
                    }
                }
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn ingest_capture(
        &self,
        request: Request<IngestCaptureRequest>,
    ) -> Result<Response<IngestCaptureResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        if req.idempotency_key.is_empty() || !st.capture_keys.insert(req.idempotency_key.clone()) {
            return Ok(Response::new(IngestCaptureResponse {
                accepted: false,
                capture_id: None,
                receipt: None,
                denial: Some(TypedDenial {
                    code: DenialCode::StaleOrdinal as i32,
                    message: "empty or duplicate capture idempotency key".to_owned(),
                    owner_id: Some("marlin.device.v1".to_owned()),
                    details: HashMap::new(),
                }),
            }));
        }
        let rref = mint_receipt(&mut st, "device-capture", req.idempotency_key.as_bytes());
        Ok(Response::new(IngestCaptureResponse {
            accepted: true,
            capture_id: Some(format!("cap-{}", st.receipt_seq)),
            receipt: Some(rref),
            denial: None,
        }))
    }

    async fn upload_artifact(
        &self,
        request: Request<Streaming<UploadArtifactRequest>>,
    ) -> Result<Response<UploadArtifactResponse>, Status> {
        self.check_auth(&request)?;
        let mut inbound = request.into_inner();
        let mut buf = Vec::new();
        let mut mime = String::new();
        use tokio_stream::StreamExt;
        while let Some(chunk) = inbound.next().await {
            let chunk = chunk?;
            if mime.is_empty() {
                mime = chunk.mime_type;
            }
            buf.extend_from_slice(&chunk.data);
            if chunk.eof {
                break;
            }
        }
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        let rref = mint_receipt(&mut st, "artifact", &buf);
        let id = format!("art-{}", st.receipt_seq);
        let size = buf.len() as u64;
        st.artifacts.insert(id.clone(), buf);
        Ok(Response::new(UploadArtifactResponse {
            artifact: Some(ArtifactRef {
                artifact_id: id,
                sha256: rref.digest_sha256,
                mime_type: mime,
                byte_size: size,
            }),
        }))
    }

    async fn download_artifact(
        &self,
        request: Request<DownloadArtifactRequest>,
    ) -> Result<Response<Self::DownloadArtifactStream>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let bytes = {
            let st = self.state.lock().expect("bridge mutex poisoned");
            st.artifacts
                .get(&req.artifact_id)
                .cloned()
                .ok_or_else(|| Status::not_found("artifact not found"))?
        };
        let stream = async_stream::try_stream! {
            yield DownloadArtifactResponse {
                offset: 0,
                data: bytes,
                eof: true,
            };
        };
        Ok(Response::new(Box::pin(stream)))
    }
}

#[tonic::async_trait]
impl ProjectionQueryService for MyGoBridgeHost {
    type WatchWorkspaceDiffStream = BoxStream<WatchWorkspaceDiffResponse>;
    type WatchManagedProcessesStream = BoxStream<WatchManagedProcessesResponse>;
    type WatchGraphTrajectoryStream = BoxStream<WatchGraphTrajectoryResponse>;

    async fn query_project_graph(
        &self,
        request: Request<QueryProjectGraphRequest>,
    ) -> Result<Response<QueryProjectGraphResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        if req.secret_visibility == SecretVisibility::Allowed as i32 {
            return Err(Status::permission_denied(
                "secret visibility cannot be elevated by UI clients",
            ));
        }
        Ok(Response::new(QueryProjectGraphResponse {
            records: vec![GraphRecordView {
                record_id: "rec-1".to_owned(),
                surface: GraphVisibleSurface::Workspace as i32,
                kind: "org.task".to_owned(),
                title: "Marlin MyGo Bridge Ready".to_owned(),
                summary: format!("project={}", req.project_id),
                source_path: Some("README.org".to_owned()),
                start_line: Some(1),
                end_line: Some(10),
                facets: HashMap::new(),
            }],
            page_info: Some(CursorPageInfo {
                next_cursor: None,
                has_more: false,
                returned_items: 1,
            }),
        }))
    }

    async fn watch_workspace_diff(
        &self,
        request: Request<WatchWorkspaceDiffRequest>,
    ) -> Result<Response<Self::WatchWorkspaceDiffStream>, Status> {
        self.check_auth(&request)?;
        let stream = async_stream::try_stream! {
            yield WatchWorkspaceDiffResponse {
                snapshot: Some(WorkspaceDiffSnapshot {
                    revision: 1,
                    branch: "main".to_owned(),
                    files: vec![FileDiffEntry {
                        path: "scheme/ui-mygo/interface.ss".to_owned(),
                        status: "A".to_owned(),
                        added_lines: 12,
                        removed_lines: 0,
                        diff_artifact: None,
                    }],
                }),
            };
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn watch_managed_processes(
        &self,
        request: Request<WatchManagedProcessesRequest>,
    ) -> Result<Response<Self::WatchManagedProcessesStream>, Status> {
        self.check_auth(&request)?;
        let stream = async_stream::try_stream! {
            yield WatchManagedProcessesResponse {
                snapshot: Some(ManagedProcessSnapshot {
                    revision: 1,
                    processes: Vec::new(),
                }),
            };
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn watch_graph_trajectory(
        &self,
        request: Request<WatchGraphTrajectoryRequest>,
    ) -> Result<Response<Self::WatchGraphTrajectoryStream>, Status> {
        self.check_auth(&request)?;
        let stream = async_stream::try_stream! {
            yield WatchGraphTrajectoryResponse {
                event: Some(GraphTrajectoryEvent {
                    event_id: "ev-1".to_owned(),
                    run_id: "run-01".to_owned(),
                    node_id: "plan_org_context".to_owned(),
                    node_kind: "planner".to_owned(),
                    status: "completed".to_owned(),
                    node_receipt: None,
                    metadata: HashMap::new(),
                }),
            };
        };
        Ok(Response::new(Box::pin(stream)))
    }
}

#[tonic::async_trait]
impl AppServer for MyGoBridgeHost {
    type StreamTurnStream = BoxStream<TurnEvent>;
    type WatchSessionTurnStream = BoxStream<SessionTurnEvent>;

    async fn get_capabilities(
        &self,
        request: Request<GetCapabilitiesRequest>,
    ) -> Result<Response<GetCapabilitiesResponse>, Status> {
        self.check_auth(&request)?;
        let st = self.state.lock().expect("bridge mutex poisoned");
        Ok(Response::new(GetCapabilitiesResponse {
            max_request_bytes: 256 * 1024,
            max_messages: 128,
            max_output_text_bytes: 256 * 1024,
            session_id: Some(st.session_label.clone()),
            host_target_kind: host_target_kind() as i32,
            supported_services: vec![
                "marlin.app.v1".to_owned(),
                "marlin.ui.v1".to_owned(),
                "marlin.gate.v1".to_owned(),
                "marlin.plugin.v1".to_owned(),
                "marlin.device.v1".to_owned(),
                "marlin.projection.v1".to_owned(),
            ],
        }))
    }

    async fn stream_turn(
        &self,
        request: Request<StreamTurnRequest>,
    ) -> Result<Response<Self::StreamTurnStream>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let reply = format!("marlin-bridge received {} message(s)", req.messages.len());
        let bytes = reply.len() as u64;
        let stream = async_stream::try_stream! {
            yield TurnEvent { event: Some(turn_event::Event::Started(TurnStarted {})) };
            yield TurnEvent {
                event: Some(turn_event::Event::TextDelta(TextDelta {
                    sequence: 1,
                    text: reply,
                })),
            };
            yield TurnEvent {
                event: Some(turn_event::Event::Completed(TurnCompleted {
                    text_chunks: 1,
                    text_bytes: bytes,
                })),
            };
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn start_session_turn(
        &self,
        request: Request<StartSessionTurnRequest>,
    ) -> Result<Response<StartSessionTurnResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        st.sessions.insert(req.session_id.clone());
        Ok(Response::new(StartSessionTurnResponse {
            turn: Some(SessionTurnRef {
                session_id: req.session_id,
                ordinal: req.after_turn.saturating_add(1),
            }),
        }))
    }

    async fn watch_session_turn(
        &self,
        request: Request<WatchSessionTurnRequest>,
    ) -> Result<Response<Self::WatchSessionTurnStream>, Status> {
        self.check_auth(&request)?;
        let stream = async_stream::try_stream! {
            yield SessionTurnEvent {
                event_id: "turn-ev-1".to_owned(),
                event: Some(session_turn_event::Event::Started(TurnStarted {})),
            };
            yield SessionTurnEvent {
                event_id: "turn-ev-2".to_owned(),
                event: Some(session_turn_event::Event::TextCheckpoint(TextCheckpoint {
                    text: "session turn active".to_owned(),
                })),
            };
            yield SessionTurnEvent {
                event_id: "turn-ev-3".to_owned(),
                event: Some(session_turn_event::Event::Outcome(
                    SessionTurnOutcome::Completed as i32,
                )),
            };
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn cancel_session_turn(
        &self,
        request: Request<CancelSessionTurnRequest>,
    ) -> Result<Response<CancelSessionTurnResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        Ok(Response::new(CancelSessionTurnResponse {
            turn: req.turn,
            runtime: Some(RuntimeShutdownReceipt {
                completed: true,
                cancellation_requested: true,
                tracker_closed: true,
                tracked_tasks_at_shutdown: 0,
                remaining_tasks: 0,
                timeout_ms: None,
                status: RuntimeRetirementStatus::Completed as i32,
            }),
            outcome: SessionTurnOutcome::Completed as i32,
        }))
    }

    async fn open_session(
        &self,
        request: Request<OpenSessionRequest>,
    ) -> Result<Response<OpenSessionResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        let id = if req.session_id.is_empty() {
            "default".to_owned()
        } else {
            req.session_id
        };
        st.sessions.insert(id.clone());
        Ok(Response::new(OpenSessionResponse { session_id: id }))
    }

    async fn get_session_summary(
        &self,
        request: Request<GetSessionSummaryRequest>,
    ) -> Result<Response<GetSessionSummaryResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        Ok(Response::new(GetSessionSummaryResponse {
            latest_turn: Some(SessionTurnRef {
                session_id: req.session_id,
                ordinal: 1,
            }),
            latest_event_id: Some("turn-ev-3".to_owned()),
            recovery_state: SessionRecoveryState::Finished as i32,
        }))
    }

    async fn get_session_turn_status(
        &self,
        request: Request<GetSessionTurnStatusRequest>,
    ) -> Result<Response<GetSessionTurnStatusResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        Ok(Response::new(GetSessionTurnStatusResponse {
            turn: req.turn,
            recovery_state: SessionRecoveryState::Finished as i32,
            latest_event_id: "turn-ev-3".to_owned(),
            outcome: Some(SessionTurnOutcome::Completed as i32),
        }))
    }

    async fn get_session_turn_cancellation(
        &self,
        request: Request<GetSessionTurnCancellationRequest>,
    ) -> Result<Response<GetSessionTurnCancellationResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        Ok(Response::new(GetSessionTurnCancellationResponse {
            turn: req.turn,
            evidence: Some(
                get_session_turn_cancellation_response::Evidence::Unavailable(
                    CancellationEvidenceUnavailable {},
                ),
            ),
            outcome: SessionTurnOutcome::Completed as i32,
        }))
    }
}
