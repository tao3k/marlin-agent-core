// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{collections::HashMap, sync::Arc};

use tokio::sync::broadcast;
use tonic::{Request, Response, Status, Streaming};

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
        WatchSessionTurnRequest, app_server_server::AppServer,
        get_session_turn_cancellation_response, session_turn_event, turn_event,
    },
    common::v1::{ArtifactRef, CursorPageInfo, DenialCode, TypedDenial},
    device::v1::{
        DeviceHeartbeat, DownloadArtifactRequest, DownloadArtifactResponse, IngestCaptureRequest,
        IngestCaptureResponse, OpenDeviceChannelRequest, OpenDeviceChannelResponse,
        RegisterDeviceNodeRequest, RegisterDeviceNodeResponse, UploadArtifactRequest,
        UploadArtifactResponse, device_bridge_service_server::DeviceBridgeService,
        open_device_channel_request, open_device_channel_response,
    },
    projection::v1::{
        FileDiffEntry, GraphRecordView, GraphTrajectoryEvent, GraphVisibleSurface,
        ManagedProcessSnapshot, ManagedProcessView, QueryProjectGraphRequest,
        QueryProjectGraphResponse, SecretVisibility, WatchGraphTrajectoryRequest,
        WatchGraphTrajectoryResponse, WatchManagedProcessesRequest, WatchManagedProcessesResponse,
        WatchWorkspaceDiffRequest, WatchWorkspaceDiffResponse, WorkspaceDiffSnapshot,
        projection_query_service_server::ProjectionQueryService,
    },
};

use super::{
    host::MyGoBridgeHost,
    state::{
        BoxStream, MAX_BRIDGE_ARTIFACT_BYTES, host_target_kind, mint_receipt, profile_for_target,
    },
};

#[tonic::async_trait]
impl DeviceBridgeService for MyGoBridgeHost {
    type OpenDeviceChannelStream = BoxStream<OpenDeviceChannelResponse>;
    type DownloadArtifactStream = BoxStream<DownloadArtifactResponse>;

    async fn register_device_node(
        &self,
        request: Request<RegisterDeviceNodeRequest>,
    ) -> Result<Response<RegisterDeviceNodeResponse>, Status> {
        self.check_auth(&request)?;
        self.ensure_governance_live("register_device_node")?;
        let req = request.into_inner();
        let mut st = self.lock_state()?;
        let overlay = profile_for_target(&st, req.target_kind).to_owned();
        let lease_id = st.register_device(req.device_id, overlay.clone(), req.capabilities);
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
        self.ensure_governance_live("open_device_channel")?;
        let mut inbound = request.into_inner();
        let mut inv_rx = self.device_inv_tx.subscribe();
        let state = Arc::clone(&self.state);
        let governance_gate = self.governance_gate.clone();
        let stream = async_stream::try_stream! {
            use tokio_stream::StreamExt;
            enum DeviceLoopEvent {
                Inbound(Option<Result<OpenDeviceChannelRequest, Status>>),
                Invocation(
                    Result<
                        crate::wire::marlin::device::v1::DeviceInvocationRequest,
                        broadcast::error::RecvError,
                    >,
                ),
            }
            loop {
                let event = tokio::select! {
                    maybe_frame = inbound.next() => DeviceLoopEvent::Inbound(maybe_frame),
                    inv = inv_rx.recv() => DeviceLoopEvent::Invocation(inv),
                };
                match event {
                    DeviceLoopEvent::Inbound(Some(Ok(req))) => {
                        if let Some(ref gate) = governance_gate {
                            let snap = gate
                                .snapshot()
                                .map_err(|err| Status::internal(format!("governance gate unavailable: {err}")))?;
                            if snap.breaker.tripped {
                                Err(Status::permission_denied(
                                    "device channel closed: active operator E-Stop",
                                ))?;
                            }
                        }
                        match req.frame {
                            Some(open_device_channel_request::Frame::CapabilityUpdate(upd)) => {
                                let updated: Result<bool, Status> = match state.lock() {
                                    Ok(mut st) => Ok(st.update_device_capabilities(
                                        &upd.lease_id,
                                        upd.capabilities,
                                    )),
                                    Err(_) => Err(Status::internal("bridge state lock poisoned")),
                                };
                                if !updated? {
                                    Err(Status::permission_denied(
                                        "unregistered or revoked device lease_id",
                                    ))?;
                                }
                            }
                            Some(open_device_channel_request::Frame::InvocationResult(res)) => {
                                let recorded: Result<(), Status> = match state.lock() {
                                    Ok(mut st) => {
                                        st.insert_device_result(res.invocation_id.clone(), res);
                                        Ok(())
                                    }
                                    Err(_) => Err(Status::internal("bridge state lock poisoned")),
                                };
                                recorded?;
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
                    DeviceLoopEvent::Inbound(Some(Err(_err))) => break,
                    DeviceLoopEvent::Inbound(None) => break,
                    DeviceLoopEvent::Invocation(Ok(req)) => {
                        yield OpenDeviceChannelResponse {
                            frame: Some(open_device_channel_response::Frame::InvocationRequest(req)),
                        };
                    }
                    DeviceLoopEvent::Invocation(Err(broadcast::error::RecvError::Lagged(_))) => {
                        continue;
                    }
                    DeviceLoopEvent::Invocation(Err(broadcast::error::RecvError::Closed)) => break,
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
        self.ensure_governance_live("ingest_capture")?;
        let req = request.into_inner();
        let mut st = self.lock_state()?;
        if req.idempotency_key.is_empty() || !st.insert_capture_key(req.idempotency_key.clone()) {
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
        self.ensure_governance_live("upload_artifact")?;
        let mut inbound = request.into_inner();
        let mut buf = Vec::new();
        let mut mime = String::new();
        use tokio_stream::StreamExt;
        while let Some(chunk) = inbound.next().await {
            let chunk = chunk?;
            if mime.is_empty() {
                mime = chunk.mime_type;
            }
            if buf.len().saturating_add(chunk.data.len()) > MAX_BRIDGE_ARTIFACT_BYTES {
                return Err(Status::resource_exhausted(
                    "artifact exceeds maximum allowed byte size",
                ));
            }
            buf.extend_from_slice(&chunk.data);
            if chunk.eof {
                break;
            }
        }
        let mut st = self.lock_state()?;
        let rref = mint_receipt(&mut st, "artifact", &buf);
        let id = format!("art-{}", st.receipt_seq);
        let size = buf.len() as u64;
        st.insert_artifact(id.clone(), buf);
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
            let st = self.lock_state()?;
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
        let (seeded, mode_records) = {
            let st = self.lock_state()?;
            let s = if !st.graph_projection_records.is_empty() {
                Some(st.graph_projection_records.clone())
            } else {
                None
            };
            (s, st.scheme_mode_records.clone())
        };
        let trimmed_query = req.query.trim();
        let default_fs_root = self
            .magit_bridge
            .as_ref()
            .map(|b| b.workspace_root().to_path_buf())
            .unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            });
        let records = if trimmed_query.starts_with("modes:") && !mode_records.is_empty() {
            mode_records
        } else if let Some(fs_res) =
            crate::mygo_host::query_fs_records(trimmed_query, &default_fs_root)
        {
            fs_res.map_err(Status::failed_precondition)?
        } else if trimmed_query.starts_with("magit:aitia-review")
            && let Some(bridge) = &self.magit_bridge
        {
            let repo_override = trimmed_query
                .strip_prefix("magit:aitia-review@")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(std::path::PathBuf::from);
            let review = if let Some(ref repo) = repo_override {
                bridge
                    .run_progressive_review_for(Some(repo.as_path()), None)
                    .await
                    .map_err(|err| {
                        Status::failed_precondition(format!("aitia review error: {err}"))
                    })?
            } else if let Some(cached) = bridge.latest_progressive_review() {
                cached
            } else {
                bridge
                    .run_progressive_review_for(None, None)
                    .await
                    .map_err(|err| {
                        Status::failed_precondition(format!("aitia review error: {err}"))
                    })?
            };
            let root_str = repo_override
                .as_deref()
                .unwrap_or_else(|| bridge.workspace_root())
                .to_string_lossy()
                .into_owned();
            crate::mygo_host::WorkspaceMagitBridge::project_progressive_review(&review, &root_str)
        } else if trimmed_query.starts_with("magit:")
            && let Some(bridge) = &self.magit_bridge
        {
            let repo_override = trimmed_query
                .strip_prefix("magit:snapshot@")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(std::path::PathBuf::from);
            let snap = bridge
                .refresh_snapshot_for(repo_override.as_deref())
                .await
                .map_err(|err| {
                    Status::failed_precondition(format!("magit snapshot error: {err}"))
                })?;
            let receipt = bridge.last_receipt();
            let review = bridge.latest_progressive_review();
            crate::mygo_host::WorkspaceMagitBridge::project_snapshot_with_review(
                &snap,
                receipt.as_ref(),
                review.as_ref(),
            )
        } else if let Some(records) = seeded {
            records
        } else if let Some(handler) = &self.graph_query_handler
            && let Some(live_records) = handler(&req)
        {
            live_records
        } else {
            vec![GraphRecordView {
                record_id: "rec-1".to_owned(),
                surface: GraphVisibleSurface::Workspace as i32,
                kind: "org.task".to_owned(),
                title: "Marlin MyGo Bridge Ready".to_owned(),
                summary: format!("project={}", req.project_id),
                source_path: Some("README.org".to_owned()),
                start_line: Some(1),
                end_line: Some(10),
                facets: HashMap::new(),
            }]
        };
        let returned_items = records.len() as u32;
        Ok(Response::new(QueryProjectGraphResponse {
            records,
            page_info: Some(CursorPageInfo {
                next_cursor: None,
                has_more: false,
                returned_items,
            }),
        }))
    }

    async fn watch_workspace_diff(
        &self,
        request: Request<WatchWorkspaceDiffRequest>,
    ) -> Result<Response<Self::WatchWorkspaceDiffStream>, Status> {
        self.check_auth(&request)?;
        let snapshot = if let Some(bridge) = &self.magit_bridge
            && let Ok(snap) = bridge.refresh_snapshot().await
        {
            crate::mygo_host::WorkspaceMagitBridge::to_workspace_diff_snapshot(&snap)
        } else {
            WorkspaceDiffSnapshot {
                revision: 1,
                branch: "main".to_owned(),
                files: vec![FileDiffEntry {
                    path: "scheme/ui-mygo/interface.ss".to_owned(),
                    status: "A".to_owned(),
                    added_lines: 12,
                    removed_lines: 0,
                    diff_artifact: None,
                }],
            }
        };
        let stream = async_stream::try_stream! {
            yield WatchWorkspaceDiffResponse {
                snapshot: Some(snapshot),
            };
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn watch_managed_processes(
        &self,
        request: Request<WatchManagedProcessesRequest>,
    ) -> Result<Response<Self::WatchManagedProcessesStream>, Status> {
        self.check_auth(&request)?;
        let session_filter = request.into_inner().session_id;
        let snapshot = match &self.process_registry {
            Some(registry) => {
                let mut views = Vec::new();
                for obs in registry.active_processes() {
                    if !session_filter.is_empty()
                        && let Some(id) = obs.resource_id()
                        && id.session().as_str() != session_filter
                    {
                        continue;
                    }
                    let state = match obs.status {
                        marlin_agent_runtime::process::RuntimeProcessStatus::Running => "running",
                        marlin_agent_runtime::process::RuntimeProcessStatus::Finished => "exited",
                        marlin_agent_runtime::process::RuntimeProcessStatus::Failed => "failed",
                        marlin_agent_runtime::process::RuntimeProcessStatus::Orphaned => {
                            "survived_restart"
                        }
                        marlin_agent_runtime::process::RuntimeProcessStatus::CleanupRequested => {
                            "cleaning_up"
                        }
                    };
                    let command = obs
                        .command
                        .as_ref()
                        .map(|cmd| cmd.argv.join(" "))
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| obs.owner_reference.clone());
                    views.push(ManagedProcessView {
                        process_id: format!("pid:{}:{}", obs.pid, obs.handle.as_str()),
                        command,
                        state: state.to_owned(),
                        exit_code: None,
                        started_at_unix_ms: obs.started_at_ms().unwrap_or(0),
                        shutdown_receipt: None,
                    });
                }
                ManagedProcessSnapshot {
                    revision: registry.managed_revision().max(1),
                    processes: views,
                }
            }
            None => ManagedProcessSnapshot {
                revision: 1,
                processes: Vec::new(),
            },
        };
        let stream = async_stream::try_stream! {
            yield WatchManagedProcessesResponse {
                snapshot: Some(snapshot),
            };
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn watch_graph_trajectory(
        &self,
        request: Request<WatchGraphTrajectoryRequest>,
    ) -> Result<Response<Self::WatchGraphTrajectoryStream>, Status> {
        self.check_auth(&request)?;
        let history = {
            let st = self.lock_state()?;
            st.graph_trajectory_history.clone()
        };
        if history.is_empty() {
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
            return Ok(Response::new(Box::pin(stream)));
        }
        let mut rx = self.trajectory_tx.subscribe();
        let state = Arc::clone(&self.state);
        let stream = async_stream::try_stream! {
            let mut last_event_id = history.last().map(|ev| ev.event_id.clone());
            for ev in history {
                yield WatchGraphTrajectoryResponse { event: Some(ev) };
            }
            loop {
                match rx.recv().await {
                    Ok(ev) => {
                        last_event_id = Some(ev.event_id.clone());
                        yield WatchGraphTrajectoryResponse { event: Some(ev) };
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let resync_tail: Vec<GraphTrajectoryEvent> = {
                            let st = state.lock().expect("bridge mutex poisoned");
                            match last_event_id.as_deref() {
                                Some(last_id) => st
                                    .graph_trajectory_history
                                    .iter()
                                    .skip_while(|ev| ev.event_id != last_id)
                                    .skip(1)
                                    .cloned()
                                    .collect(),
                                None => st.graph_trajectory_history.clone(),
                            }
                        };
                        for ev in resync_tail {
                            last_event_id = Some(ev.event_id.clone());
                            yield WatchGraphTrajectoryResponse { event: Some(ev) };
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
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
        let st = self.lock_state()?;
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
        self.ensure_governance_live("stream_turn")?;
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
        self.ensure_governance_live("start_session_turn")?;
        let req = request.into_inner();
        let mut st = self.lock_state()?;
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
        let mut st = self.lock_state()?;
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
