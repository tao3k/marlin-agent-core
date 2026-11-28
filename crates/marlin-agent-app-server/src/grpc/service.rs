// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{future::Future, pin::Pin, sync::Arc};

use marlin_agent_protocol::{
    ModelEndpoint, ModelEndpointContractError, ModelGatewayError, ModelGatewayRequest,
};
use marlin_agent_stream::{ModelGatewayEventStream, ModelStreamingGateway};
use tokio::{
    sync::{Semaphore, mpsc},
    time::Instant,
};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, metadata::MetadataMap};

use crate::{
    SessionBinding, SessionOpen,
    v1::{
        GetCapabilitiesRequest, GetCapabilitiesResponse, StreamTurnRequest, TurnEvent,
        app_server_server,
    },
};

use super::helpers::{
    MAX_MESSAGES, MAX_OUTPUT_TEXT_BYTES, MAX_REQUEST_BYTES, check_session_deadline,
    cursor_to_proto, deadline_exceeded, detour_mode_from_proto, detour_mode_to_proto,
    disclosure_depth_from_proto, disclosure_depth_to_proto, forward_turn, gateway_status,
    messages, operation_from_proto, operation_to_proto, receipt_status_from_proto,
    scope_mode_from_proto, session_deadline, session_deadline_exceeded, session_unavailable,
    session_until, status_to_proto, substrate_from_proto, substrate_to_proto,
    task_actor_error_status, turn_timeout, workbench_tabs_to_proto,
};

const MAX_IN_FLIGHT: usize = 16;
const MAX_SESSION_WATCHES: usize = 16;

type SessionLookup<S> = dyn Fn(
        MetadataMap,
        marlin_agent_storage::SessionId,
        SessionOpen,
    ) -> Pin<Box<dyn Future<Output = Result<SessionBinding<S>, Status>> + Send>>
    + Send
    + Sync;

type Authorize = dyn Fn(&MetadataMap) -> Result<(), Status> + Send + Sync;

/// A shareable service with a host-selected model route and authorization policy.
/// The host supplies the gateway and owns the listener, TLS and shutdown.
#[derive(Clone)]
pub struct AppServer<S = marlin_agent_storage::InMemoryAgentStorage> {
    endpoint: ModelEndpoint,
    gateway: Arc<dyn ModelStreamingGateway>,
    authorize: Arc<Authorize>,
    in_flight: Arc<Semaphore>,
    session_watches: Arc<Semaphore>,
    session: Option<SessionBinding<S>>,
    session_lookup: Option<Arc<SessionLookup<S>>>,
    task_actors: crate::TaskActorControlPlane,
}

struct ControlPlaneOnlyGateway;

impl ModelStreamingGateway for ControlPlaneOnlyGateway {
    fn stream(
        &self,
        _request: marlin_agent_protocol::ModelGatewayRequest,
    ) -> marlin_agent_protocol::ModelGatewayFuture<
        marlin_agent_protocol::ModelGatewayResult<ModelGatewayEventStream>,
    > {
        Box::pin(async move {
            Err(ModelGatewayError::RequestNotDispatched(
                "model streaming gateway not configured on control-plane-only AppServer".to_owned(),
            ))
        })
    }
}

impl AppServer {
    /// Construct a service for one configured model route. The authorization
    /// callback must grant access to that route before any RPC reaches it.
    pub fn new(
        endpoint: ModelEndpoint,
        gateway: Arc<dyn ModelStreamingGateway>,
        authorize: impl Fn(&MetadataMap) -> Result<(), Status> + Send + Sync + 'static,
    ) -> Result<Self, ModelEndpointContractError> {
        endpoint.validate_contract()?;
        Ok(Self {
            endpoint,
            gateway,
            authorize: Arc::new(authorize),
            in_flight: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
            session_watches: Arc::new(Semaphore::new(MAX_SESSION_WATCHES)),
            session: None,
            session_lookup: None,
            task_actors: crate::TaskActorControlPlane::new(),
        })
    }

    /// Construct a Session-free control-plane `AppServer` (e.g. for TaskActor /
    /// GraphEngine oracle + MyGo UI hosting) whose model streaming RPCs fail closed
    /// without dispatching to any LLM provider.
    pub fn control_plane_only(
        authorize: impl Fn(&MetadataMap) -> Result<(), Status> + Send + Sync + 'static,
    ) -> Self {
        Self::new(
            ModelEndpoint::new("marlin", "control-plane-only"),
            Arc::new(ControlPlaneOnlyGateway),
            authorize,
        )
        .expect("control-plane-only endpoint satisfies contract")
    }

    /// Attach a host-owned Session and its Runtime to the external service.
    pub fn with_session<S: marlin_agent_storage::SessionEventStore + Clone + 'static>(
        self,
        session: crate::SessionBinding<S>,
    ) -> AppServer<S> {
        AppServer {
            endpoint: self.endpoint,
            gateway: self.gateway,
            authorize: self.authorize,
            in_flight: self.in_flight,
            session_watches: self.session_watches,
            session: Some(session),
            session_lookup: None,
            task_actors: self.task_actors,
        }
    }

    /// Attach the host's Session resolver. The host checks per-Session scope on
    /// every lookup and returns the same Runtime owner for an active Session.
    pub fn with_session_lookup<S, F, Fut>(self, lookup: F) -> AppServer<S>
    where
        S: marlin_agent_storage::SessionEventStore + Clone + 'static,
        F: Fn(MetadataMap, marlin_agent_storage::SessionId, SessionOpen) -> Fut
            + Send
            + Sync
            + 'static,
        Fut: Future<Output = Result<SessionBinding<S>, Status>> + Send + 'static,
    {
        AppServer {
            endpoint: self.endpoint,
            gateway: self.gateway,
            authorize: self.authorize,
            in_flight: self.in_flight,
            session_watches: self.session_watches,
            session: None,
            session_lookup: Some(Arc::new(move |metadata, id, mode| {
                Box::pin(lookup(metadata, id, mode))
            })),
            task_actors: self.task_actors,
        }
    }
}

impl<S> AppServer<S> {
    /// Mount a shared Session-free `TaskActorControlPlane` onto this `AppServer`.
    pub fn with_task_actors(mut self, task_actors: crate::TaskActorControlPlane) -> Self {
        self.task_actors = task_actors;
        self
    }

    /// Access the Session-free `TaskActorControlPlane` mounted on this `AppServer`.
    pub fn task_actors(&self) -> &crate::TaskActorControlPlane {
        &self.task_actors
    }

    /// Fail-closed control-plane guard ensuring active Operator E-Stop / tripped
    /// `ExecutionCircuitBreaker` blocks LLM turn execution and control-plane mutations (SM26).
    fn ensure_governance_live(&self, rpc_name: &str) -> Result<(), Status> {
        if let Some(gate) = self.task_actors.governance_gate() {
            match gate.execution_breaker().snapshot() {
                Ok(snap) if !snap.tripped => {}
                Ok(_) => {
                    return Err(Status::permission_denied(format!(
                        "governance E-Stop/circuit breaker tripped: refusing AppServer RPC `{rpc_name}`"
                    )));
                }
                Err(_) => {
                    return Err(Status::permission_denied(format!(
                        "governance breaker unavailable (fail-closed): refusing AppServer RPC `{rpc_name}`"
                    )));
                }
            }
        }
        Ok(())
    }
}

impl<S: marlin_agent_storage::SessionEventStore + Clone + 'static> AppServer<S> {
    /// Compose this real `AppServer` (backed by a host `ModelStreamingGateway`
    /// and durable `SessionBinding`) with `MyGoBridgeHost`'s 5 orthogonal UI,
    /// Gate, Plugin, Device, and Projection services on one shared HTTP/2 router.
    #[cfg(feature = "ui-grpc")]
    pub fn into_ui_router(
        self,
        bridge: crate::ui_grpc::MyGoBridgeHost,
    ) -> tonic::transport::server::Router {
        self.into_ui_router_on(tonic::transport::Server::builder(), bridge)
    }

    /// Compose this real `AppServer` with `MyGoBridgeHost`'s 5 orthogonal UI,
    /// Gate, Plugin, Device, and Projection services on a caller-configured
    /// `tonic::transport::Server` builder (e.g. with TLS 1.3 / mTLS enabled).
    #[cfg(feature = "ui-grpc")]
    pub fn into_ui_router_on(
        self,
        mut builder: tonic::transport::Server,
        bridge: crate::ui_grpc::MyGoBridgeHost,
    ) -> tonic::transport::server::Router {
        let base = builder.add_service(self.into_service());
        bridge.attach_ui_services(base)
    }

    /// Serve this co-hosted `AppServer` plus `MyGoBridgeHost`'s 5 orthogonal UI/Gate/Plugin/
    /// Device/Projection services on an already-bound `tokio::net::TcpListener`.
    #[cfg(feature = "ui-grpc")]
    pub async fn serve_with_ui_on(
        self,
        bridge: crate::ui_grpc::MyGoBridgeHost,
        listener: tokio::net::TcpListener,
    ) -> Result<(), tonic::transport::Error> {
        self.serve_with_ui_on_shutdown(bridge, listener, std::future::pending())
            .await
    }

    /// Serve this co-hosted `AppServer` plus `MyGoBridgeHost` on `listener` until `signal`
    /// resolves, then shut down gracefully.
    #[cfg(feature = "ui-grpc")]
    pub async fn serve_with_ui_on_shutdown<F: Future<Output = ()>>(
        self,
        bridge: crate::ui_grpc::MyGoBridgeHost,
        listener: tokio::net::TcpListener,
        signal: F,
    ) -> Result<(), tonic::transport::Error> {
        let incoming = async_stream::stream! {
            loop {
                yield listener.accept().await.map(|(stream, _)| stream);
            }
        };
        self.into_ui_router(bridge)
            .serve_with_incoming_shutdown(incoming, signal)
            .await
    }

    async fn resolve_session(
        &self,
        metadata: MetadataMap,
        id: &str,
        mode: SessionOpen,
        deadline: Instant,
    ) -> Result<SessionBinding<S>, Status> {
        check_session_deadline(deadline)?;
        if let Some(lookup) = &self.session_lookup {
            let id = marlin_agent_storage::SessionId::new(id)
                .map_err(|_| Status::invalid_argument("invalid Session identity"))?;
            let binding =
                session_until(deadline, async { lookup(metadata, id.clone(), mode).await }).await?;
            if binding.session_id() != id.as_str() {
                return Err(Status::data_loss(
                    "host returned a mismatched Session binding",
                ));
            }
            return Ok(binding);
        }
        let binding = self.session.as_ref().ok_or_else(session_unavailable)?;
        if mode == SessionOpen::Create || id != binding.session_id() {
            return Err(Status::not_found("Session is outside this service binding"));
        }
        Ok(binding.clone())
    }

    async fn resolve_turn(
        &self,
        metadata: MetadataMap,
        turn: Option<&crate::v1::SessionTurnRef>,
        deadline: Instant,
    ) -> Result<SessionBinding<S>, Status> {
        let turn = turn.ok_or_else(|| Status::invalid_argument("turn reference required"))?;
        if turn.ordinal == 0 {
            return Err(Status::invalid_argument("turn ordinal must be positive"));
        }
        self.resolve_session(metadata, &turn.session_id, SessionOpen::Existing, deadline)
            .await
    }

    /// Service suitable for `tonic::transport::Server::add_service`.
    pub fn into_service(self) -> app_server_server::AppServerServer<Self> {
        app_server_server::AppServerServer::new(self)
            .max_decoding_message_size(MAX_REQUEST_BYTES as usize)
            .max_encoding_message_size(MAX_REQUEST_BYTES as usize)
    }
}

#[tonic::async_trait]
impl<S: marlin_agent_storage::SessionEventStore + Clone + 'static> app_server_server::AppServer
    for AppServer<S>
{
    type StreamTurnStream = Pin<Box<dyn Stream<Item = Result<TurnEvent, Status>> + Send>>;
    type WatchWorkbenchTabsStream =
        Pin<Box<dyn Stream<Item = Result<crate::v1::WorkbenchTabsResponse, Status>> + Send>>;

    async fn get_capabilities(
        &self,
        request: Request<GetCapabilitiesRequest>,
    ) -> Result<Response<GetCapabilitiesResponse>, Status> {
        (self.authorize)(request.metadata())?;
        Ok(Response::new(GetCapabilitiesResponse {
            max_request_bytes: MAX_REQUEST_BYTES,
            max_messages: MAX_MESSAGES,
            max_output_text_bytes: MAX_OUTPUT_TEXT_BYTES,
            session_id: self
                .session
                .as_ref()
                .map(|binding| binding.session_id().to_owned()),
        }))
    }

    type WatchSessionTurnStream = crate::session::WatchStream;

    async fn open_session(
        &self,
        request: Request<crate::v1::OpenSessionRequest>,
    ) -> Result<Response<crate::v1::OpenSessionResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let deadline = session_deadline(request.metadata())?;
        let (metadata, _, request) = request.into_parts();
        let mode = match request.mode() {
            crate::v1::SessionOpenMode::Create => SessionOpen::Create,
            crate::v1::SessionOpenMode::Existing => SessionOpen::Existing,
            crate::v1::SessionOpenMode::Unspecified => {
                return Err(Status::invalid_argument("Session open mode is required"));
            }
        };
        let binding = self
            .resolve_session(metadata, &request.session_id, mode, deadline)
            .await?;
        Ok(Response::new(crate::v1::OpenSessionResponse {
            session_id: binding.session_id().into(),
        }))
    }

    async fn get_session_summary(
        &self,
        request: Request<crate::v1::GetSessionSummaryRequest>,
    ) -> Result<Response<crate::v1::GetSessionSummaryResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let deadline = session_deadline(request.metadata())?;
        let (metadata, _, request) = request.into_parts();
        let binding = self
            .resolve_session(
                metadata,
                &request.session_id,
                SessionOpen::Existing,
                deadline,
            )
            .await?;
        let summary = session_until(deadline, binding.summary(request)).await?;
        Ok(Response::new(summary))
    }

    async fn get_session_turn_cancellation(
        &self,
        request: Request<crate::v1::GetSessionTurnCancellationRequest>,
    ) -> Result<Response<crate::v1::GetSessionTurnCancellationResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let deadline = session_deadline(request.metadata())?;
        let (metadata, _, request) = request.into_parts();
        let binding = self
            .resolve_turn(metadata, request.turn.as_ref(), deadline)
            .await?;
        let evidence = session_until(deadline, binding.read_cancellation(request)).await?;
        Ok(Response::new(evidence))
    }

    async fn get_session_turn_status(
        &self,
        request: Request<crate::v1::GetSessionTurnStatusRequest>,
    ) -> Result<Response<crate::v1::GetSessionTurnStatusResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let deadline = session_deadline(request.metadata())?;
        let (metadata, _, request) = request.into_parts();
        let binding = self
            .resolve_turn(metadata, request.turn.as_ref(), deadline)
            .await?;
        let status = session_until(deadline, binding.turn_status(request)).await?;
        Ok(Response::new(status))
    }

    async fn start_session_turn(
        &self,
        request: Request<crate::v1::StartSessionTurnRequest>,
    ) -> Result<Response<crate::v1::StartSessionTurnResponse>, Status> {
        (self.authorize)(request.metadata())?;
        self.ensure_governance_live("start_session_turn")?;
        let deadline = session_deadline(request.metadata())?;
        let (metadata, _, request) = request.into_parts();
        let binding = self
            .resolve_session(
                metadata,
                &request.session_id,
                SessionOpen::Existing,
                deadline,
            )
            .await?;
        let turn = session_until(
            deadline,
            binding.start(
                request,
                self.endpoint.clone(),
                self.gateway.clone(),
                deadline,
            ),
        )
        .await?;
        Ok(Response::new(crate::v1::StartSessionTurnResponse {
            turn: Some(turn),
        }))
    }

    async fn watch_session_turn(
        &self,
        request: Request<crate::v1::WatchSessionTurnRequest>,
    ) -> Result<Response<Self::WatchSessionTurnStream>, Status> {
        (self.authorize)(request.metadata())?;
        let deadline = session_deadline(request.metadata())?;
        check_session_deadline(deadline)?;
        let permit = self
            .session_watches
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                Status::resource_exhausted("App Server has reached its Session watch limit")
            })?;
        let (metadata, _, request) = request.into_parts();
        let binding = self
            .resolve_turn(metadata, request.turn.as_ref(), deadline)
            .await?;
        let mut replay = session_until(deadline, binding.watch(request, deadline)).await?;
        let (send, mut receive) = mpsc::channel(1);
        let task = tokio::spawn(async move {
            let _permit = permit;
            tokio::select! {
                biased;
                _ = send.closed() => Ok(()),
                _ = tokio::time::sleep_until(deadline) => Err(session_deadline_exceeded()),
                result = async {
                    while let Ok(slot) = send.reserve().await {
                        match replay.next().await {
                            Some(event) => slot.send(event?),
                            None => break,
                        }
                    }
                    Ok(())
                } => result,
            }
        });
        let stream = async_stream::try_stream! {
            while let Some(event) = receive.recv().await {
                check_session_deadline(deadline)?;
                yield event;
            }
            task.await.map_err(|_| Status::internal("Session replay task failed"))??;
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn cancel_session_turn(
        &self,
        request: Request<crate::v1::CancelSessionTurnRequest>,
    ) -> Result<Response<crate::v1::CancelSessionTurnResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let deadline = session_deadline(request.metadata())?;
        let (metadata, _, request) = request.into_parts();
        let binding = self
            .resolve_turn(metadata, request.turn.as_ref(), deadline)
            .await?;
        let receipt = session_until(deadline, binding.cancel(request, deadline)).await?;
        Ok(Response::new(receipt))
    }

    async fn stream_turn(
        &self,
        request: Request<StreamTurnRequest>,
    ) -> Result<Response<Self::StreamTurnStream>, Status> {
        (self.authorize)(request.metadata())?;
        self.ensure_governance_live("stream_turn")?;
        let deadline = Instant::now() + turn_timeout(request.metadata())?;
        let messages = messages(request.into_inner())?;
        let permit = self.in_flight.clone().try_acquire_owned().map_err(|_| {
            Status::resource_exhausted("App Server has reached its active turn limit")
        })?;
        if Instant::now() >= deadline {
            return Err(deadline_exceeded());
        }
        let request = ModelGatewayRequest::new(self.endpoint.clone(), messages);
        let upstream = tokio::time::timeout_at(deadline, self.gateway.stream(request))
            .await
            .map_err(|_| deadline_exceeded())?
            .map_err(gateway_status)?;
        let (send, mut receive) = mpsc::channel(1);
        let task = tokio::spawn(async move {
            let _permit = permit;
            // This timer remains live while HTTP/2 is waiting for client credit.
            tokio::select! {
                biased;
                _ = send.closed() => Ok(()),
                _ = tokio::time::sleep_until(deadline) => Err(deadline_exceeded()),
                result = forward_turn(upstream, &send) => result,
            }
        });
        let stream = async_stream::try_stream! {
            while let Some(event) = receive.recv().await {
                yield event;
            }
            task.await.map_err(|_| Status::internal("model turn task failed"))??;
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn register_task_actor(
        &self,
        request: Request<crate::v1::RegisterTaskActorRequest>,
    ) -> Result<Response<crate::v1::TaskActorCursorResponse>, Status> {
        (self.authorize)(request.metadata())?;
        self.ensure_governance_live("register_task_actor")?;
        let req = request.into_inner();
        let task_id = marlin_agent_runtime::TaskActorId::new(req.task_id)
            .map_err(|err| Status::invalid_argument(format!("invalid task_id: {err}")))?;
        let scope_mode = scope_mode_from_proto(req.scope_mode)?;
        let cursor = self
            .task_actors
            .register_actor(crate::RegisterTaskActorRequest {
                task_id,
                title: req.title,
                scope_mode,
            })
            .map_err(task_actor_error_status)?;
        Ok(Response::new(crate::v1::TaskActorCursorResponse {
            cursor: Some(cursor_to_proto(&cursor)),
        }))
    }

    async fn dispatch_task_actor(
        &self,
        request: Request<crate::v1::DispatchTaskActorRequest>,
    ) -> Result<Response<crate::v1::DispatchTaskActorResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let req = request.into_inner();
        let request_id = marlin_agent_runtime::DispatchRequestId::new(req.request_id)
            .map_err(|err| Status::invalid_argument(format!("invalid request_id: {err}")))?;
        let task_id = marlin_agent_runtime::TaskActorId::new(req.task_id)
            .map_err(|err| Status::invalid_argument(format!("invalid task_id: {err}")))?;
        let fencing_epoch = marlin_agent_runtime::FencingEpoch::new(req.fencing_epoch)
            .map_err(|err| Status::invalid_argument(format!("invalid fencing_epoch: {err}")))?;
        let operation = operation_from_proto(req.operation)?;
        let substrate_target = substrate_from_proto(req.substrate_target)?;
        let scope_mode = scope_mode_from_proto(req.scope_mode)?;
        let join_node_id = req
            .join_node_id
            .map(|raw| {
                marlin_agent_runtime::GraphConvergenceNodeId::new(raw)
                    .map_err(|err| Status::invalid_argument(format!("invalid join_node_id: {err}")))
            })
            .transpose()?;
        let envelope = marlin_agent_runtime::MarlinDispatchEnvelope {
            request_id,
            task_id,
            operation,
            fencing_epoch,
            scope_mode,
            join_node_id,
            substrate_target,
        };
        let admitted = self
            .task_actors
            .admit_dispatch(envelope)
            .map_err(task_actor_error_status)?;
        let env = admitted.envelope();
        Ok(Response::new(crate::v1::DispatchTaskActorResponse {
            request_id: env.request_id.as_str().to_owned(),
            task_id: env.task_id.as_str().to_owned(),
            fencing_epoch: env.fencing_epoch.value(),
            operation: operation_to_proto(env.operation),
            substrate_target: substrate_to_proto(env.substrate_target),
        }))
    }

    async fn reconcile_task_receipt(
        &self,
        request: Request<crate::v1::ReconcileTaskReceiptRequest>,
    ) -> Result<Response<crate::v1::ReconcileTaskReceiptResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let req = request.into_inner();
        let request_id = marlin_agent_runtime::DispatchRequestId::new(req.request_id)
            .map_err(|err| Status::invalid_argument(format!("invalid request_id: {err}")))?;
        let task_id = marlin_agent_runtime::TaskActorId::new(req.task_id)
            .map_err(|err| Status::invalid_argument(format!("invalid task_id: {err}")))?;
        let fencing_epoch = marlin_agent_runtime::FencingEpoch::new(req.fencing_epoch)
            .map_err(|err| Status::invalid_argument(format!("invalid fencing_epoch: {err}")))?;
        let status = receipt_status_from_proto(req.status)?;
        let receipt = marlin_agent_runtime::MarlinExecutionReceipt {
            request_id,
            task_id,
            fencing_epoch,
            status,
        };
        let decision = self
            .task_actors
            .reconcile_receipt(&receipt)
            .map_err(task_actor_error_status)?;
        Ok(Response::new(crate::v1::ReconcileTaskReceiptResponse {
            accepted: decision.accepted,
            reason: format!("{:?}", decision.reason),
            next_epoch: decision.next_epoch.value(),
            next_status: status_to_proto(decision.next_status),
        }))
    }

    async fn bind_governed_context(
        &self,
        request: Request<crate::v1::BindGovernedContextRequest>,
    ) -> Result<Response<crate::v1::BindGovernedContextResponse>, Status> {
        (self.authorize)(request.metadata())?;
        self.ensure_governance_live("bind_governed_context")?;
        let req = request.into_inner();
        let task_id = marlin_agent_runtime::TaskActorId::new(req.task_id)
            .map_err(|err| Status::invalid_argument(format!("invalid task_id: {err}")))?;
        let disclosure_depth = disclosure_depth_from_proto(req.disclosure_depth)?;
        let kv_prefix_digest = marlin_agent_runtime::KvPrefixDigest::new(req.kv_prefix_digest)
            .map_err(|err| Status::invalid_argument(format!("invalid kv_prefix_digest: {err}")))?;
        let detour_mode = detour_mode_from_proto(req.detour_mode)?;
        let capsule = marlin_agent_runtime::GovernedContextCapsule {
            task_id,
            context_cut: req.context_cut,
            kv_prefix_digest,
            disclosure_depth,
            has_active_defeater: req.has_active_defeater,
            quarantined_memory_count: req.quarantined_memory_count,
            sandbox_admitted: req.sandbox_admitted,
            cedar_handoff_valid: req.cedar_handoff_valid,
            detour_mode,
            auto_archive_enabled: req.auto_archive_enabled,
        };
        let admitted = self
            .task_actors
            .bind_governed_context(capsule)
            .map_err(task_actor_error_status)?;
        let cap = admitted.capsule();
        Ok(Response::new(crate::v1::BindGovernedContextResponse {
            task_id: cap.task_id.as_str().to_owned(),
            context_cut: cap.context_cut,
            disclosure_depth: disclosure_depth_to_proto(cap.disclosure_depth),
            kv_prefix_digest: cap.kv_prefix_digest.as_str().to_owned(),
            detour_mode: detour_mode_to_proto(cap.detour_mode),
            auto_archive_enabled: cap.auto_archive_enabled,
        }))
    }

    async fn get_workbench_tabs(
        &self,
        request: Request<crate::v1::GetWorkbenchTabsRequest>,
    ) -> Result<Response<crate::v1::WorkbenchTabsResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let tabs = self
            .task_actors
            .project_workbench_tabs()
            .map_err(task_actor_error_status)?;
        Ok(Response::new(workbench_tabs_to_proto(&tabs)))
    }

    async fn watch_workbench_tabs(
        &self,
        request: Request<crate::v1::WatchWorkbenchTabsRequest>,
    ) -> Result<Response<Self::WatchWorkbenchTabsStream>, Status> {
        (self.authorize)(request.metadata())?;
        let permit = self
            .session_watches
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                Status::resource_exhausted("App Server has reached its active watch limit")
            })?;
        let mut rx = self.task_actors.subscribe_workbench_tabs();
        let initial_tabs = self
            .task_actors
            .project_workbench_tabs()
            .map_err(task_actor_error_status)?;
        let _ = rx.borrow_and_update();
        let initial = workbench_tabs_to_proto(&initial_tabs);
        let stream = async_stream::try_stream! {
            let _permit = permit;
            yield initial;
            while rx.changed().await.is_ok() {
                let next = workbench_tabs_to_proto(&rx.borrow());
                yield next;
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }
}
