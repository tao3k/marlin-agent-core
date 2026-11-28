// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    pin::Pin,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_runtime::plugin::{
    PanelPluginHost, PanelRuntimeKind as RuntimePanelKind, PluginDenial,
    SlotCardinality as RuntimeSlotCardinality,
};
use tokio_stream::Stream;
use tonic::{Status, metadata::MetadataMap};

use crate::wire::marlin::{
    common::v1::{DenialCode, ReceiptRef, TargetKind, TypedDenial},
    device::v1::{DeviceCapabilityDescriptor, DeviceInvocationResult},
    gate::v1::{GateDecisionReceipt, PendingGate},
    plugin::v1::PanelRuntimeKind,
    projection::v1::{GraphRecordView, GraphTrajectoryEvent, QueryProjectGraphRequest},
    ui::v1::{
        SlotCardinality, SlotDeclaration, UiCommand, UiCommandArguments, UiElement,
        UiElementSequence, UiKeyBinding, UiLayoutPolicy, UiRgb, UiSlotNode,
        UiSlotSource, UiSpan, UiTextNode, UiTheme, UiTone, UiTransient, UiWorkbenchSnapshot,
        ui_element,
    },
};

pub(super) type AuthorizeFn = dyn Fn(&MetadataMap) -> Result<(), Status> + Send + Sync;
pub(super) type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send + 'static>>;

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

/// Notification emitted after a one-shot `GraphHumanGate` is durably claimed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimedGraphHumanDecision {
    pub gate_id: String,
    pub decision: i32,
    pub note: Option<String>,
    pub receipt_id: String,
}

/// Notification emitted when a UI client dispatches a declarative command via
/// `DispatchUiEventRequest::RecordCommand`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchedUiCommand {
    pub session_id: String,
    pub command: String,
    pub generation: u64,
}

/// Optional live search handler invoked by `ProjectionQueryService::query_project_graph`
/// when no static test fixture records are seeded on `MyGoBridgeHost`.
pub type GraphQueryHandler =
    dyn Fn(&QueryProjectGraphRequest) -> Option<Vec<GraphRecordView>> + Send + Sync;

/// Bounded retention limits refining `dev/quint/stability/MarlinMyGoBridgeStabilityOverlay.qnt` (SM3c).
pub(super) const MAX_BRIDGE_ARTIFACTS: usize = 64;
pub(super) const MAX_BRIDGE_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_BRIDGE_RECORDS: usize = 256;

pub(super) struct BridgeState {
    pub(super) generation: u64,
    pub(super) profile_name: String,
    pub(super) focus_id: String,
    pub(super) model_label: String,
    pub(super) session_label: String,
    pub(super) overlays: BTreeMap<i32, SchemeOverlayProjection>,
    pub(super) slots: Vec<SlotDeclaration>,
    pub(super) sessions: BTreeSet<String>,
    pub(super) pending_gates: BTreeMap<String, PendingGate>,
    pub(super) claimed_gates: BTreeMap<String, GateDecisionReceipt>,
    pub(super) claimed_gate_order: VecDeque<String>,
    pub(super) panel_host: PanelPluginHost,
    pub(super) receipt_seq: u64,
    pub(super) registered_devices: BTreeMap<String, String>,
    pub(super) device_capabilities: BTreeMap<String, Vec<DeviceCapabilityDescriptor>>,
    pub(super) device_order: VecDeque<String>,
    pub(super) active_leases: BTreeMap<String, String>,
    pub(super) device_leases: BTreeMap<String, String>,
    pub(super) next_lease_seq: u64,
    pub(super) device_results: BTreeMap<String, DeviceInvocationResult>,
    pub(super) device_result_order: VecDeque<String>,
    pub(super) capture_keys: BTreeSet<String>,
    pub(super) capture_key_order: VecDeque<String>,
    pub(super) artifacts: BTreeMap<String, Vec<u8>>,
    pub(super) artifact_order: VecDeque<String>,
    pub(super) graph_projection_records: Vec<GraphRecordView>,
    pub(super) graph_trajectory_history: Vec<GraphTrajectoryEvent>,
    pub(super) scheme_commands: Vec<UiCommand>,
    pub(super) scheme_keymap: Vec<UiKeyBinding>,
    pub(super) scheme_transients: Vec<UiTransient>,
    pub(super) scheme_mode_records: Vec<GraphRecordView>,
}

impl BridgeState {
    pub(super) fn insert_artifact(&mut self, id: String, bytes: Vec<u8>) {
        if !self.artifacts.contains_key(&id) {
            while self.artifacts.len() >= MAX_BRIDGE_ARTIFACTS {
                let Some(evict_id) = self.artifact_order.pop_front() else {
                    break;
                };
                self.artifacts.remove(&evict_id);
            }
            self.artifact_order.push_back(id.clone());
        }
        self.artifacts.insert(id, bytes);
    }

    pub(super) fn insert_claimed_gate(&mut self, gate_id: String, receipt: GateDecisionReceipt) {
        if !self.claimed_gates.contains_key(&gate_id) {
            while self.claimed_gates.len() >= MAX_BRIDGE_RECORDS {
                let Some(evict_id) = self.claimed_gate_order.pop_front() else {
                    break;
                };
                self.claimed_gates.remove(&evict_id);
            }
            self.claimed_gate_order.push_back(gate_id.clone());
        }
        self.claimed_gates.insert(gate_id, receipt);
    }

    pub(super) fn insert_device_result(&mut self, invocation_id: String, result: DeviceInvocationResult) {
        if !self.device_results.contains_key(&invocation_id) {
            while self.device_results.len() >= MAX_BRIDGE_RECORDS {
                let Some(evict_id) = self.device_result_order.pop_front() else {
                    break;
                };
                self.device_results.remove(&evict_id);
            }
            self.device_result_order.push_back(invocation_id.clone());
        }
        self.device_results.insert(invocation_id, result);
    }

    pub(super) fn insert_capture_key(&mut self, key: String) -> bool {
        if self.capture_keys.contains(&key) {
            return false;
        }
        while self.capture_keys.len() >= MAX_BRIDGE_RECORDS {
            let Some(evict_key) = self.capture_key_order.pop_front() else {
                break;
            };
            self.capture_keys.remove(&evict_key);
        }
        self.capture_key_order.push_back(key.clone());
        self.capture_keys.insert(key)
    }

    pub(super) fn register_device(
        &mut self,
        device_id: String,
        overlay: String,
        capabilities: Vec<DeviceCapabilityDescriptor>,
    ) -> String {
        if !self.registered_devices.contains_key(&device_id) {
            while self.registered_devices.len() >= MAX_BRIDGE_RECORDS {
                let Some(evict_id) = self.device_order.pop_front() else {
                    break;
                };
                self.registered_devices.remove(&evict_id);
                self.device_capabilities.remove(&evict_id);
                if let Some(old_lease) = self.device_leases.remove(&evict_id) {
                    self.active_leases.remove(&old_lease);
                }
            }
            self.device_order.push_back(device_id.clone());
        }
        if let Some(prev_lease) = self.device_leases.remove(&device_id) {
            self.active_leases.remove(&prev_lease);
        }
        self.next_lease_seq = self.next_lease_seq.saturating_add(1);
        let lease_id = format!("lease-{device_id}-{}", self.next_lease_seq);
        self.active_leases
            .insert(lease_id.clone(), device_id.clone());
        self.device_leases
            .insert(device_id.clone(), lease_id.clone());
        self.registered_devices.insert(device_id.clone(), overlay);
        self.device_capabilities.insert(device_id, capabilities);
        lease_id
    }

    /// Update capabilities strictly for a registered `lease_id` bound in `active_leases` (SM28).
    /// Returns `false` and refuses state mutation on forged or revoked leases without evicting devices.
    pub(super) fn update_device_capabilities(
        &mut self,
        lease_id: &str,
        capabilities: Vec<DeviceCapabilityDescriptor>,
    ) -> bool {
        let Some(device_id) = self.active_leases.get(lease_id).cloned() else {
            return false;
        };
        if !self.registered_devices.contains_key(&device_id) {
            self.active_leases.remove(lease_id);
            return false;
        }
        self.device_capabilities.insert(device_id, capabilities);
        true
    }
}


pub(super) fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub(super) fn mint_receipt(st: &mut BridgeState, domain: &str, material: &[u8]) -> ReceiptRef {
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

pub(super) fn default_slots() -> Vec<SlotDeclaration> {
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

pub(super) fn default_theme() -> UiTheme {
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

pub(super) fn default_profile_overlays() -> Vec<SchemeOverlayProjection> {
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

pub(super) fn host_target_kind() -> TargetKind {
    if cfg!(target_os = "macos") {
        TargetKind::DesktopMacos
    } else if cfg!(target_os = "windows") {
        TargetKind::DesktopWindows
    } else {
        TargetKind::DesktopLinux
    }
}

pub(super) fn profile_for_target(st: &BridgeState, kind: i32) -> &str {
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

pub(super) fn build_snapshot(st: &BridgeState) -> UiWorkbenchSnapshot {
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
        commands: if st.scheme_commands.is_empty() {
            vec![
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
            ]
        } else {
            st.scheme_commands.clone()
        },
        keymap: st.scheme_keymap.clone(),
        transients: st.scheme_transients.clone(),
        slots: st.slots.clone(),
    }
}

pub(super) fn map_denial(err: PluginDenial) -> TypedDenial {
    let (code, msg) = match err {
        PluginDenial::UnknownPublisher => (DenialCode::Unauthorized, "unknown publisher key"),
        PluginDenial::InvalidSignature => {
            (DenialCode::InvalidSignature, "invalid Ed25519 signature")
        }
        PluginDenial::UnsupportedContract => (
            DenialCode::UnsupportedContract,
            "unsupported plugin contract",
        ),
        PluginDenial::MissingCapability(_) => (
            DenialCode::CapabilityNotGranted,
            "required plugin capability not granted",
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

pub(super) fn to_proto_runtime(rt: RuntimePanelKind) -> i32 {
    match rt {
        RuntimePanelKind::Declarative => PanelRuntimeKind::Declarative as i32,
        RuntimePanelKind::WebviewUntrusted => PanelRuntimeKind::WebviewUntrusted as i32,
        RuntimePanelKind::WasmUiIr => PanelRuntimeKind::WasmUiIr as i32,
        RuntimePanelKind::MygoJsWasm => PanelRuntimeKind::MygoJsWasm as i32,
    }
}

pub(super) fn to_proto_cardinality(c: RuntimeSlotCardinality) -> i32 {
    match c {
        RuntimeSlotCardinality::Single => SlotCardinality::Single as i32,
        RuntimeSlotCardinality::List => SlotCardinality::List as i32,
        RuntimeSlotCardinality::Keyed => SlotCardinality::Keyed as i32,
    }
}
