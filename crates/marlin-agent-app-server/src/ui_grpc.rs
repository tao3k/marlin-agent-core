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
    ContributionId, PanelRuntimeKind as RuntimePanelKind, PluginDenial, PublisherId,
    ReleaseAttestation as RuntimeAttestation, ReleaseTrust, SignedPanelRelease,
    SlotCardinality as RuntimeSlotCardinality, VerifiedPanelRelease,
};
use tokio::sync::broadcast;
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming, metadata::MetadataMap};

use crate::wire::marlin::{
    common::v1::{ArtifactRef, CursorPageInfo, DenialCode, ReceiptRef, TargetKind, TypedDenial},
    device::v1::{
        DownloadArtifactRequest, DownloadArtifactResponse, IngestCaptureRequest,
        IngestCaptureResponse, OpenDeviceChannelRequest, OpenDeviceChannelResponse,
        RegisterDeviceNodeRequest, RegisterDeviceNodeResponse, UploadArtifactRequest,
        UploadArtifactResponse,
        device_bridge_service_server::{DeviceBridgeService, DeviceBridgeServiceServer},
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

/// Shared authoritative state for all 5 MyGo gRPC services.
#[derive(Clone)]
pub struct MyGoBridgeHost {
    authorize: Arc<AuthorizeFn>,
    trust: Arc<ReleaseTrust>,
    state: Arc<Mutex<BridgeState>>,
    wb_tx: broadcast::Sender<UiWorkbenchSnapshot>,
    gate_tx: broadcast::Sender<PendingGate>,
    panel_tx: broadcast::Sender<ScopedPanelEvent>,
}

struct BridgeState {
    generation: u64,
    profile_name: String,
    focus_id: String,
    model_label: String,
    session_label: String,
    slots: Vec<SlotDeclaration>,
    pending_gates: BTreeMap<String, PendingGate>,
    claimed_gates: BTreeMap<String, GateDecisionReceipt>,
    installed_releases: BTreeMap<String, InstalledPanelRecord>,
    active_generations: BTreeMap<(String, String), ActivePanel>,
    receipt_seq: u64,
    registered_devices: BTreeMap<String, String>,
    capture_keys: BTreeSet<String>,
    artifacts: BTreeMap<String, Vec<u8>>,
}

struct InstalledPanelRecord {
    verified: VerifiedPanelRelease,
    summary: InstalledReleaseSummary,
}

struct ActivePanel {
    generation: u64,
    declared_actions: BTreeSet<String>,
    read_scopes: BTreeSet<String>,
}

impl MyGoBridgeHost {
    pub fn new(
        trust: ReleaseTrust,
        authorize: impl Fn(&MetadataMap) -> Result<(), Status> + Send + Sync + 'static,
    ) -> Self {
        let (wb_tx, _) = broadcast::channel(64);
        let (gate_tx, _) = broadcast::channel(64);
        let (panel_tx, _) = broadcast::channel(128);
        let state = BridgeState {
            generation: 1,
            profile_name: "desktop-dev".to_owned(),
            focus_id: "composer".to_owned(),
            model_label: "marlin-core".to_owned(),
            session_label: "default".to_owned(),
            slots: default_slots(),
            pending_gates: BTreeMap::new(),
            claimed_gates: BTreeMap::new(),
            installed_releases: BTreeMap::new(),
            active_generations: BTreeMap::new(),
            receipt_seq: 0,
            registered_devices: BTreeMap::new(),
            capture_keys: BTreeSet::new(),
            artifacts: BTreeMap::new(),
        };
        Self {
            authorize: Arc::new(authorize),
            trust: Arc::new(trust),
            state: Arc::new(Mutex::new(state)),
            wb_tx,
            gate_tx,
            panel_tx,
        }
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

fn profile_for_target(kind: i32) -> &'static str {
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
        st.profile_name = req
            .preferred_profile
            .unwrap_or_else(|| profile_for_target(req.client_target_kind).to_owned());
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
            receipt: Some(rref),
            denial: None,
        };
        st.claimed_gates.insert(pending.gate_id, receipt.clone());
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
                .installed_releases
                .values()
                .map(|r| r.summary.clone())
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

        let verified = match self.trust.verify_panel(SignedPanelRelease {
            manifest: &env.manifest_toml,
            attestation: RuntimeAttestation {
                publisher,
                source_revision,
                build_evidence_digest,
            },
            signature: &sig_arr,
        }) {
            Ok(v) => v,
            Err(denial) => {
                return Ok(Response::new(InstallSignedReleaseResponse {
                    installed: false,
                    summary: None,
                    denial: Some(map_denial(denial)),
                }));
            }
        };

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
                byte_size: env.manifest_toml.len() as u64,
                read_scopes: c.panel.read_scopes.clone(),
                declared_actions: c.panel.declared_actions.clone(),
            })
            .collect();

        let summary = InstalledReleaseSummary {
            plugin_id: verified.id().as_str().to_owned(),
            version: format!("{:?}", verified.version()),
            publisher_id: verified.attestation().publisher.as_str().to_owned(),
            bundle_digest_sha256: verified.digest().as_bytes().to_vec(),
            enabled: false,
            active_generation: 0,
            declared_capabilities: Vec::new(),
            granted_capabilities: Vec::new(),
            panel_assets,
        };
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        st.installed_releases.insert(
            verified.id().as_str().to_owned(),
            InstalledPanelRecord {
                verified,
                summary: summary.clone(),
            },
        );
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
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        let Some(rec) = st.installed_releases.get(&req.plugin_id) else {
            return Ok(Response::new(EnableWorkflowResponse {
                receipt: Some(WorkflowActivationReceipt {
                    activated: false,
                    activation_id: String::new(),
                    generation: 0,
                    effective_settings_sha256: Vec::new(),
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::NotFound as i32,
                        message: "plugin release not installed".to_owned(),
                        owner_id: Some("marlin.plugin.v1".to_owned()),
                        details: HashMap::new(),
                    }),
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
        let Some(contrib) = rec.verified.contribution(&cid) else {
            return Ok(Response::new(EnableWorkflowResponse {
                receipt: Some(WorkflowActivationReceipt {
                    activated: false,
                    activation_id: String::new(),
                    generation: 0,
                    effective_settings_sha256: Vec::new(),
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::NotFound as i32,
                        message: "contribution not found in installed release".to_owned(),
                        owner_id: Some("marlin.plugin.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        };
        let declared_actions: BTreeSet<String> =
            contrib.panel.declared_actions.iter().cloned().collect();
        let read_scopes: BTreeSet<String> = contrib.panel.read_scopes.iter().cloned().collect();
        let key = (req.plugin_id.clone(), req.contribution_id.clone());
        let next_gen = st
            .active_generations
            .get(&key)
            .map(|a| a.generation + 1)
            .unwrap_or(1);
        st.active_generations.insert(
            key,
            ActivePanel {
                generation: next_gen,
                declared_actions,
                read_scopes,
            },
        );
        if let Some(rec_mut) = st.installed_releases.get_mut(&req.plugin_id) {
            rec_mut.summary.enabled = true;
            rec_mut.summary.active_generation = next_gen;
        }
        let rref = mint_receipt(&mut st, "plugin-activate", req.plugin_id.as_bytes());
        Ok(Response::new(EnableWorkflowResponse {
            receipt: Some(WorkflowActivationReceipt {
                activated: true,
                activation_id: format!("act-{}-{next_gen}", req.plugin_id),
                generation: next_gen,
                effective_settings_sha256: rref.digest_sha256.clone(),
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
        let rref = mint_receipt(&mut st, "plugin-disable", req.activation_id.as_bytes());
        Ok(Response::new(DisableWorkflowResponse {
            receipt: Some(WorkflowDisableReceipt {
                disabled: true,
                closed_generation: 1,
                receipt: Some(rref),
                denial: None,
            }),
        }))
    }

    async fn set_project_grants(
        &self,
        request: Request<SetProjectGrantsRequest>,
    ) -> Result<Response<SetProjectGrantsResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        let rref = mint_receipt(&mut st, "plugin-grant", req.plugin_id.as_bytes());
        Ok(Response::new(SetProjectGrantsResponse {
            receipt: Some(ProjectGrantReceipt {
                updated: true,
                granted_capabilities: req.granted_capabilities,
                receipt: Some(rref),
                denial: None,
            }),
        }))
    }

    async fn get_workflow_evidence(
        &self,
        request: Request<GetWorkflowEvidenceRequest>,
    ) -> Result<Response<GetWorkflowEvidenceResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        Ok(Response::new(GetWorkflowEvidenceResponse {
            activation_id: req.activation_id,
            bundle_digest_sha256: vec![0u8; 32],
            project_digest_sha256: vec![0u8; 32],
            effect_receipts: Vec::new(),
            replay_bundle: None,
        }))
    }

    async fn open_scoped_panel_stream(
        &self,
        request: Request<OpenScopedPanelStreamRequest>,
    ) -> Result<Response<Self::OpenScopedPanelStreamStream>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let allowed_topics: BTreeSet<String> = {
            let st = self.state.lock().expect("bridge mutex poisoned");
            let key = (req.plugin_id.clone(), req.contribution_id.clone());
            match st.active_generations.get(&key) {
                Some(active) if active.generation == req.activation_generation => {
                    active.read_scopes.clone()
                }
                _ => return Err(Status::failed_precondition("plugin generation not active")),
            }
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
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        let key = (req.plugin_id.clone(), req.contribution_id.clone());
        let Some(active) = st.active_generations.get(&key) else {
            return Ok(Response::new(SubmitScopedPanelActionResponse {
                receipt: Some(ScopedPanelActionReceipt {
                    admitted: false,
                    result_json: None,
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::NotFound as i32,
                        message: "plugin contribution is not active".to_owned(),
                        owner_id: Some("marlin.plugin.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        };
        if active.generation != req.activation_generation {
            return Ok(Response::new(SubmitScopedPanelActionResponse {
                receipt: Some(ScopedPanelActionReceipt {
                    admitted: false,
                    result_json: None,
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::StaleGeneration as i32,
                        message: format!(
                            "stale plugin generation {} != active {}",
                            req.activation_generation, active.generation
                        ),
                        owner_id: Some("marlin.plugin.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        }
        if !active.declared_actions.contains(&req.action_name) {
            return Ok(Response::new(SubmitScopedPanelActionResponse {
                receipt: Some(ScopedPanelActionReceipt {
                    admitted: false,
                    result_json: None,
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::CapabilityNotGranted as i32,
                        message: format!("action {:?} not in declared_actions", req.action_name),
                        owner_id: Some("marlin.plugin.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        }
        let rref = mint_receipt(&mut st, "plugin-action", req.action_name.as_bytes());
        Ok(Response::new(SubmitScopedPanelActionResponse {
            receipt: Some(ScopedPanelActionReceipt {
                admitted: true,
                result_json: Some(br#"{"status":"admitted"}"#.to_vec()),
                receipt: Some(rref),
                denial: None,
            }),
        }))
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
        let overlay = profile_for_target(req.target_kind).to_owned();
        let mut st = self.state.lock().expect("bridge mutex poisoned");
        let lease_id = format!(
            "lease-{}-{}",
            req.device_id,
            st.registered_devices.len() + 1
        );
        st.registered_devices.insert(req.device_id, overlay.clone());
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
        let stream = async_stream::try_stream! {
            use tokio_stream::StreamExt;
            while let Some(frame) = inbound.next().await {
                let _ = frame?;
                yield OpenDeviceChannelResponse { frame: None };
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
