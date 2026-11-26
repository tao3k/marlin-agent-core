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
        GetWorkbenchTabsRequest, WatchWorkbenchTabsRequest, WorkbenchTabsResponse,
        app_server_client::AppServerClient as V1AppServerClient,
    },
    wire::marlin::{
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
            DispatchUiEventRequest, DispatchUiEventResponse, dispatch_ui_event_request,
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
