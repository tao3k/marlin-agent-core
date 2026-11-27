// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

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

use super::views::{
    CedarProbeResultView, FleetAuditCapsuleView, FleetTaskAction, GovernanceAuthoritySnapshotView,
    GovernanceFactAction, HostStatusSnapshot,
};

pub fn parse_bearer_header(
    bearer_token: Option<&str>,
) -> Result<Option<MetadataValue<tonic::metadata::Ascii>>, String> {
    match bearer_token.map(str::trim).filter(|t| !t.is_empty()) {
        Some(tok) => {
            let mut val = MetadataValue::try_from(format!("Bearer {tok}"))
                .map_err(|err| format!("invalid bearer token metadata: {err}"))?;
            // Prevent HTTP/2 HPACK dynamic-table indexing and Debug plaintext leakage.
            val.set_sensitive(true);
            Ok(Some(val))
        }
        None => Ok(None),
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
        let bearer_header = parse_bearer_header(bearer_token)?;
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

    /// Dispatches a typed incremental governance fact-graph mutation via `UiWorkbenchService`.
    pub async fn dispatch_governance_fact(
        &self,
        session_id: impl Into<String>,
        action: &GovernanceFactAction,
    ) -> Result<DispatchUiEventResponse, Status> {
        let (action_sym, arg) = action.to_wire_command();
        self.dispatch_scheme_command(session_id, action_sym, arg.as_deref())
            .await
    }

    /// Dispatches a typed Scheme fleet/task steering action (`scheme:task-*`) via `UiWorkbenchService`.
    pub async fn dispatch_fleet_task(
        &self,
        session_id: impl Into<String>,
        action: &FleetTaskAction,
    ) -> Result<DispatchUiEventResponse, Status> {
        let (action_sym, arg) = action.to_wire_command();
        self.dispatch_scheme_command(session_id, action_sym, arg.as_deref())
            .await
    }

    /// Dispatches a live compiled-Cedar probe (`governance-check-cedar`) and polls
    /// `ProjectionQueryService` until the host publishes a strictly newer probe sequence
    /// number (`probe.seq > prev_seq`).
    pub async fn probe_cedar_and_wait(
        &self,
        session_id: impl Into<String>,
        actor: impl Into<String>,
        action: impl Into<String>,
        resource: impl Into<String>,
        step_up_fresh: bool,
        timeout: std::time::Duration,
    ) -> Result<CedarProbeResultView, String> {
        let prev_seq = self
            .query_governance_authority_snapshot("default")
            .await
            .map_err(|s| format!("query baseline governance snapshot: {s}"))?
            .and_then(|g| g.last_cedar_probe())
            .map(|p| p.seq)
            .unwrap_or(0);
        let fact = GovernanceFactAction::CheckCedar {
            actor: actor.into(),
            action: action.into(),
            resource: resource.into(),
            step_up_fresh,
        };
        let resp = self
            .dispatch_governance_fact(session_id, &fact)
            .await
            .map_err(|s| format!("dispatch governance-check-cedar: {s}"))?;
        if !resp.admitted {
            return Err("governance-check-cedar dispatch was not admitted".to_owned());
        }
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Some(probe) = self
                .query_governance_authority_snapshot("default")
                .await
                .map_err(|s| format!("poll governance snapshot for cedar probe: {s}"))?
                .and_then(|g| g.last_cedar_probe())
                && probe.seq > prev_seq
            {
                return Ok(probe);
            }
            if std::time::Instant::now() >= deadline {
                return Err(format!(
                    "timed out after {}ms waiting for compiled Cedar probe result (prev_seq={prev_seq})",
                    timeout.as_millis()
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
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

    /// Queries and parses the live read-only `GovernanceAuthoritySnapshotView`.
    pub async fn query_governance_authority_snapshot(
        &self,
        project_id: impl Into<String>,
    ) -> Result<Option<GovernanceAuthoritySnapshotView>, Status> {
        let records = self.query_project_graph_records(project_id).await?;
        let Some(rec) = records
            .iter()
            .find(|r| r.kind == "scheme.governance.authority_snapshot")
        else {
            return Ok(None);
        };
        let view = GovernanceAuthoritySnapshotView::from_record(rec)
            .map_err(|err| Status::internal(format!("decode governance authority snapshot: {err}")))?;
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

    /// Concurrently fetches `tabs`, `audit`, `governance`, and `gates` over the shared HTTP/2 channel.
    pub async fn fetch_status_snapshot(
        &self,
        project_id: impl Into<String>,
    ) -> Result<HostStatusSnapshot, Status> {
        let project = project_id.into();
        let (tabs, records, gates) = tokio::try_join!(
            self.get_workbench_tabs(),
            self.query_project_graph_records(project),
            self.list_pending_gates(),
        )?;
        let audit =
            match records
                .iter()
                .find(|r| r.kind == "scheme.fleet.audit_capsule")
            {
                Some(rec) => Some(FleetAuditCapsuleView::from_record(rec).map_err(|err| {
                    Status::internal(format!("decode fleet audit capsule: {err}"))
                })?),
                None => None,
            };
        let governance = match records
            .iter()
            .find(|r| r.kind == "scheme.governance.authority_snapshot")
        {
            Some(rec) => Some(GovernanceAuthoritySnapshotView::from_record(rec).map_err(
                |err| Status::internal(format!("decode governance authority snapshot: {err}")),
            )?),
            None => None,
        };
        Ok(HostStatusSnapshot {
            tabs,
            audit,
            governance,
            gates,
        })
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
