// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Thin, typed, Bearer-authenticated HTTP/2 client for `marlin-scheme-host` and
//! `marlin-mygo-host`, unifying all 4 live control-plane services over one shared
//! `tonic::transport::Channel`:
//! - `AppServer` (`GetWorkbenchTabs`, `WatchWorkbenchTabs`, `TaskActor` RPCs)
//! - `UiWorkbenchService` (`OpenWorkbench`, declarative `DispatchUiEvent`)
//! - `ApprovalGateService` (`ListPendingGates`, one-shot `SubmitGraphHumanDecision`)
//! - `ProjectionQueryService` (read-only non-authorizing `FleetAuditCapsule` & `WatchGraphTrajectory`)

use std::collections::HashMap;

pub use tonic::{Code as GrpcCode, Status as GrpcStatus};
use tonic::{
    Request, Status, Streaming,
    metadata::MetadataValue,
    transport::{Channel, Endpoint},
};

use crate::{
    v1::{
        GetWorkbenchTabsRequest, WatchWorkbenchTabsRequest, WorkbenchTabKindProto,
        WorkbenchTabsResponse, app_server_client::AppServerClient as V1AppServerClient,
    },
    wire::marlin::{
        common::v1::TargetKind,
        gate::v1::{
            GateDecisionReceipt, GraphHumanDecision, ListPendingGatesRequest, PendingGate,
            SubmitGraphHumanDecisionRequest,
            approval_gate_service_client::ApprovalGateServiceClient,
        },
        projection::v1::{
            GraphRecordView, QueryProjectGraphRequest, SecretVisibility,
            WatchGraphTrajectoryRequest, WatchGraphTrajectoryResponse,
            projection_query_service_client::ProjectionQueryServiceClient,
        },
        ui::v1::{
            DispatchUiEventRequest, DispatchUiEventResponse, OpenWorkbenchRequest,
            UiWorkbenchSnapshot, dispatch_ui_event_request,
            ui_workbench_service_client::UiWorkbenchServiceClient,
        },
    },
};

/// Typed read-only view of a Scheme `FleetAuditCapsule` projected over
/// `ProjectionQueryService::query_project_graph`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAuditCapsuleView {
    pub record_id: String,
    pub kind: String,
    pub summary: String,
    pub active_actor_count: usize,
    pub archived_actor_count: usize,
    pub peek_count: usize,
    pub total_active_defeaters: usize,
    pub total_human_inbox_items: usize,
    pub disclosure_level: String,
    pub favored_path_collapsed: bool,
    pub authenticates_evidence: bool,
    pub authorizes_effect: bool,
}

impl FleetAuditCapsuleView {
    fn from_record(rec: &GraphRecordView) -> Result<Self, String> {
        let parse_usize = |k: &str| -> usize {
            rec.facets
                .get(k)
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0)
        };
        let parse_bool = |k: &str| -> bool {
            rec.facets
                .get(k)
                .map(|v| v.eq_ignore_ascii_case("true"))
                .unwrap_or(false)
        };
        Ok(Self {
            record_id: rec.record_id.clone(),
            kind: rec.kind.clone(),
            summary: rec.summary.clone(),
            active_actor_count: parse_usize("active_actor_count"),
            archived_actor_count: parse_usize("archived_actor_count"),
            peek_count: parse_usize("peek_count"),
            total_active_defeaters: parse_usize("total_active_defeaters"),
            total_human_inbox_items: parse_usize("total_human_inbox_items"),
            disclosure_level: rec
                .facets
                .get("disclosure_level")
                .cloned()
                .unwrap_or_else(|| "L1".to_owned()),
            favored_path_collapsed: parse_bool("favored_path_collapsed"),
            authenticates_evidence: parse_bool("authenticates_evidence"),
            authorizes_effect: parse_bool("authorizes_effect"),
        })
    }

    /// Serializes this capsule view as an RFC 8259 JSON object without external serde deps.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"record_id\":{},\"kind\":{},\"summary\":{},\"active_actor_count\":{},\"archived_actor_count\":{},\"peek_count\":{},\"total_active_defeaters\":{},\"total_human_inbox_items\":{},\"disclosure_level\":{},\"favored_path_collapsed\":{},\"authenticates_evidence\":{},\"authorizes_effect\":{}}}",
            json_escape(&self.record_id),
            json_escape(&self.kind),
            json_escape(&self.summary),
            self.active_actor_count,
            self.archived_actor_count,
            self.peek_count,
            self.total_active_defeaters,
            self.total_human_inbox_items,
            json_escape(&self.disclosure_level),
            self.favored_path_collapsed,
            self.authenticates_evidence,
            self.authorizes_effect
        )
    }
}

/// Unified operator snapshot combining live `WorkbenchTabs`, read-only `FleetAuditCapsule`,
/// and pending one-shot `ApprovalGateService` gates from a single HTTP/2 round-trip batch.
#[derive(Clone, Debug)]
pub struct HostStatusSnapshot {
    pub tabs: WorkbenchTabsResponse,
    pub audit: Option<FleetAuditCapsuleView>,
    pub gates: Vec<PendingGate>,
}

impl HostStatusSnapshot {
    /// Serializes this snapshot as a deterministic RFC 8259 JSON object.
    pub fn to_json(&self) -> String {
        let tabs_json: Vec<String> = self
            .tabs
            .tabs
            .iter()
            .map(|t| {
                format!(
                    "{{\"kind\":{},\"id\":{},\"fencing_epoch\":{},\"status_label\":{}}}",
                    json_escape(workbench_tab_kind_label(t.kind)),
                    json_escape(&t.id),
                    t.fencing_epoch,
                    json_escape(&t.status_label)
                )
            })
            .collect();
        let audit_json = self
            .audit
            .as_ref()
            .map_or_else(|| "null".to_owned(), FleetAuditCapsuleView::to_json);
        let gates_json: Vec<String> = self
            .gates
            .iter()
            .map(|g| {
                format!(
                    "{{\"gate_id\":{},\"tool_or_node_name\":{}}}",
                    json_escape(&g.gate_id),
                    json_escape(&g.tool_or_node_name)
                )
            })
            .collect();
        format!(
            "{{\"tabs\":[{}],\"audit\":{},\"gates\":[{}]}}",
            tabs_json.join(","),
            audit_json,
            gates_json.join(",")
        )
    }
}

/// Thin Bearer-authenticated HTTP/2 client for a live `marlin-scheme-host` daemon.
#[derive(Clone, Debug)]
pub struct MarlinSchemeHostClient {
    channel: Channel,
    bearer_header: Option<MetadataValue<tonic::metadata::Ascii>>,
}

impl MarlinSchemeHostClient {
    /// Connects to a running `marlin-scheme-host` or `marlin-mygo-host` endpoint
    /// (e.g. `"http://127.0.0.1:50051"`), optionally attaching a Bearer token to every RPC.
    pub async fn connect(
        endpoint: impl Into<String>,
        bearer_token: Option<&str>,
    ) -> Result<Self, String> {
        let ep_str = endpoint.into();
        let bearer_header = match bearer_token.map(str::trim).filter(|t| !t.is_empty()) {
            Some(tok) => {
                let raw = format!("Bearer {tok}");
                let val = MetadataValue::try_from(raw)
                    .map_err(|err| format!("invalid bearer token metadata: {err}"))?;
                Some(val)
            }
            None => None,
        };
        let channel = Endpoint::from_shared(ep_str.clone())
            .map_err(|err| format!("invalid endpoint {ep_str}: {err}"))?
            .connect()
            .await
            .map_err(|err| format!("connect {ep_str}: {err}"))?;
        Ok(Self {
            channel,
            bearer_header,
        })
    }

    fn auth_req<T>(&self, payload: T) -> Request<T> {
        let mut req = Request::new(payload);
        if let Some(header) = &self.bearer_header {
            req.metadata_mut().insert("authorization", header.clone());
        }
        req
    }

    /// Fetches the current Session-free `WorkbenchTabs` snapshot from `AppServer`.
    pub async fn get_workbench_tabs(&self) -> Result<WorkbenchTabsResponse, Status> {
        let mut client = V1AppServerClient::new(self.channel.clone());
        Ok(client
            .get_workbench_tabs(self.auth_req(GetWorkbenchTabsRequest {}))
            .await?
            .into_inner())
    }

    /// Subscribes to live HTTP/2 `WatchWorkbenchTabs` updates from `AppServer`.
    pub async fn watch_workbench_tabs(&self) -> Result<Streaming<WorkbenchTabsResponse>, Status> {
        let mut client = V1AppServerClient::new(self.channel.clone());
        Ok(client
            .watch_workbench_tabs(self.auth_req(WatchWorkbenchTabsRequest {}))
            .await?
            .into_inner())
    }

    /// Lists currently pending `ApprovalGateService` gates.
    pub async fn list_pending_gates(&self) -> Result<Vec<PendingGate>, Status> {
        let mut client = ApprovalGateServiceClient::new(self.channel.clone());
        Ok(client
            .list_pending_gates(self.auth_req(ListPendingGatesRequest { session_id: None }))
            .await?
            .into_inner()
            .gates)
    }

    /// Submits a one-shot `GraphHumanGate` decision (`Approve` or `Reject`).
    pub async fn submit_graph_human_decision(
        &self,
        gate_id: impl Into<String>,
        decision: GraphHumanDecision,
        idempotency_key: impl Into<String>,
        note: Option<String>,
    ) -> Result<GateDecisionReceipt, Status> {
        let mut client = ApprovalGateServiceClient::new(self.channel.clone());
        let resp = client
            .submit_graph_human_decision(self.auth_req(SubmitGraphHumanDecisionRequest {
                turn: None,
                gate_id: gate_id.into(),
                decision: decision as i32,
                idempotency_key: idempotency_key.into(),
                note,
            }))
            .await?
            .into_inner();
        resp.receipt
            .ok_or_else(|| Status::internal("approval gate response missing receipt"))
    }

    /// Dispatches a declarative `scheme:<action>` command via `UiWorkbenchService`.
    pub async fn dispatch_scheme_command(
        &self,
        session_id: impl Into<String>,
        action_sym: &str,
        arg: Option<&str>,
    ) -> Result<DispatchUiEventResponse, Status> {
        let mut client = UiWorkbenchServiceClient::new(self.channel.clone());
        let cmd = match arg.map(str::trim).filter(|a| !a.is_empty()) {
            Some(a) => format!("scheme:{action_sym} {a}"),
            None => format!("scheme:{action_sym}"),
        };
        Ok(client
            .dispatch_ui_event(self.auth_req(DispatchUiEventRequest {
                session_id: session_id.into(),
                expected_generation: 0,
                event: Some(dispatch_ui_event_request::Event::RecordCommand(cmd)),
            }))
            .await?
            .into_inner())
    }

    /// Queries raw read-only `GraphRecordView` records from `ProjectionQueryService`
    /// with fail-closed `SecretVisibility::Denied`.
    pub async fn query_project_graph_records(
        &self,
        project_id: impl Into<String>,
    ) -> Result<Vec<GraphRecordView>, Status> {
        let mut client = ProjectionQueryServiceClient::new(self.channel.clone());
        Ok(client
            .query_project_graph(self.auth_req(QueryProjectGraphRequest {
                project_id: project_id.into(),
                session_id: None,
                surfaces: Vec::new(),
                secret_visibility: SecretVisibility::Denied as i32,
                query: String::new(),
                page: None,
            }))
            .await?
            .into_inner()
            .records)
    }

    /// Queries and parses the live read-only non-authorizing `FleetAuditCapsuleView`.
    pub async fn query_fleet_audit_capsule(
        &self,
        project_id: impl Into<String>,
    ) -> Result<Option<FleetAuditCapsuleView>, Status> {
        let records = self.query_project_graph_records(project_id).await?;
        let Some(rec) = records
            .iter()
            .find(|r| r.kind == "scheme.fleet.audit_capsule")
        else {
            return Ok(None);
        };
        let view = FleetAuditCapsuleView::from_record(rec)
            .map_err(|err| Status::internal(format!("decode fleet audit capsule: {err}")))?;
        Ok(Some(view))
    }

    /// Opens a live `UiWorkbenchService` session snapshot for the given client target.
    pub async fn open_workbench(
        &self,
        session_id: impl Into<String>,
        target_kind: TargetKind,
        preferred_profile: Option<String>,
    ) -> Result<UiWorkbenchSnapshot, Status> {
        let mut client = UiWorkbenchServiceClient::new(self.channel.clone());
        let resp = client
            .open_workbench(self.auth_req(OpenWorkbenchRequest {
                session_id: session_id.into(),
                client_target_kind: target_kind as i32,
                preferred_profile,
            }))
            .await?
            .into_inner();
        resp.workbench
            .ok_or_else(|| Status::internal("open_workbench response missing workbench snapshot"))
    }

    /// Concurrently fetches `tabs`, `audit`, and `gates` over the shared HTTP/2 channel.
    pub async fn fetch_status_snapshot(
        &self,
        project_id: impl Into<String>,
    ) -> Result<HostStatusSnapshot, Status> {
        let project = project_id.into();
        let (tabs, audit, gates) = tokio::try_join!(
            self.get_workbench_tabs(),
            self.query_fleet_audit_capsule(project),
            self.list_pending_gates(),
        )?;
        Ok(HostStatusSnapshot { tabs, audit, gates })
    }

    /// Subscribes to live read-only `GraphTrajectoryEvent` updates from `ProjectionQueryService`.
    pub async fn watch_graph_trajectory(
        &self,
        session_id: impl Into<String>,
    ) -> Result<Streaming<WatchGraphTrajectoryResponse>, Status> {
        let mut client = ProjectionQueryServiceClient::new(self.channel.clone());
        Ok(client
            .watch_graph_trajectory(self.auth_req(WatchGraphTrajectoryRequest {
                session_id: session_id.into(),
                run_id: None,
                after_event_id: None,
            }))
            .await?
            .into_inner())
    }
}

/// Maps a `WorkbenchTabKindProto` integer discriminant to its canonical label.
pub fn workbench_tab_kind_label(kind: i32) -> &'static str {
    match WorkbenchTabKindProto::try_from(kind) {
        Ok(WorkbenchTabKindProto::PinnedIndex) => "PinnedIndex",
        Ok(WorkbenchTabKindProto::ActiveTask) => "ActiveTask",
        Ok(WorkbenchTabKindProto::ReadOnlyPeek) => "ReadOnlyPeek",
        _ => "Unspecified",
    }
}

/// Quotes and escapes a Rust `&str` as an RFC 8259 JSON string literal.
pub fn json_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 2);
    out.push('"');
    for ch in raw.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Formats a `GraphRecordView` as a deterministic RFC 8259 JSON object.
pub fn graph_record_to_json(rec: &GraphRecordView) -> String {
    let mut pairs: Vec<(&String, &String)> = rec.facets.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    let facets_json: Vec<String> = pairs
        .into_iter()
        .map(|(k, v)| format!("{}:{}", json_escape(k), json_escape(v)))
        .collect();
    let source_path = rec
        .source_path
        .as_deref()
        .map_or_else(|| "null".to_owned(), json_escape);
    format!(
        "{{\"record_id\":{},\"kind\":{},\"title\":{},\"summary\":{},\"source_path\":{source_path},\"facets\":{{{}}}}}",
        json_escape(&rec.record_id),
        json_escape(&rec.kind),
        json_escape(&rec.title),
        json_escape(&rec.summary),
        facets_json.join(",")
    )
}

/// Formats a `GraphRecordView` facets map deterministically for CLI output.
pub fn format_facets_sorted(facets: &HashMap<String, String>) -> String {
    let mut pairs: Vec<(&String, &String)> = facets.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escape_handles_quotes_backslashes_and_control_chars() {
        assert_eq!(json_escape("hello"), "\"hello\"");
        assert_eq!(
            json_escape("a\"b\\c\nd\te\x01"),
            "\"a\\\"b\\\\c\\nd\\te\\u0001\""
        );
    }

    #[test]
    fn graph_record_to_json_orders_facets_deterministically() {
        let mut facets = HashMap::new();
        facets.insert("z_key".to_owned(), "2".to_owned());
        facets.insert("a_key".to_owned(), "1".to_owned());
        let rec = GraphRecordView {
            record_id: "scheme:fleet:audit-capsule".to_owned(),
            surface: 0,
            kind: "scheme.fleet.audit_capsule".to_owned(),
            title: "Audit".to_owned(),
            summary: "ok".to_owned(),
            source_path: Some("scheme/graph-engine/fleet-controller.ss".to_owned()),
            start_line: None,
            end_line: None,
            facets,
        };
        let raw = graph_record_to_json(&rec);
        assert!(raw.contains("\"facets\":{\"a_key\":\"1\",\"z_key\":\"2\"}"));
        assert!(raw.contains("\"source_path\":\"scheme/graph-engine/fleet-controller.ss\""));
    }

    #[test]
    fn host_status_snapshot_serializes_valid_structure() {
        let snap = HostStatusSnapshot {
            tabs: WorkbenchTabsResponse {
                focused_task_id: None,
                tabs: Vec::new(),
            },
            audit: Some(FleetAuditCapsuleView {
                record_id: "scheme:fleet:audit-capsule".to_owned(),
                kind: "scheme.fleet.audit_capsule".to_owned(),
                summary: "active=1".to_owned(),
                active_actor_count: 1,
                archived_actor_count: 1,
                peek_count: 1,
                total_active_defeaters: 0,
                total_human_inbox_items: 1,
                disclosure_level: "L3".to_owned(),
                favored_path_collapsed: false,
                authenticates_evidence: false,
                authorizes_effect: false,
            }),
            gates: Vec::new(),
        };
        let raw = snap.to_json();
        assert!(raw.contains("\"tabs\":[]"));
        assert!(raw.contains("\"active_actor_count\":1"));
        assert!(raw.contains("\"authorizes_effect\":false"));
        assert!(raw.contains("\"gates\":[]"));
    }
}
