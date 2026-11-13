// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use marlin_agent_protocol::{
    ModelEndpoint, ModelEndpointContractError, ModelGatewayError, ModelGatewayMessage,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelStreamInterruption,
    assistant_gateway_message, user_gateway_message,
};
use marlin_agent_stream::{
    MODEL_STREAM_MAX_CHUNK_BYTES, ModelGatewayEventStream, ModelStreamEvent, ModelStreamingGateway,
};
use tokio::{
    sync::{Semaphore, mpsc},
    time::Instant,
};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, metadata::MetadataMap};

use crate::{
    SessionBinding, SessionOpen,
    v1::{
        GetCapabilitiesRequest, GetCapabilitiesResponse, MessageRole, StreamTurnRequest, TextDelta,
        TurnCompleted, TurnEvent, TurnStarted, app_server_server, turn_event::Event,
    },
};

pub(crate) const MAX_REQUEST_BYTES: u32 = 256 * 1024;
pub(crate) const MAX_MESSAGES: u32 = 128;
const MAX_OUTPUT_TEXT_BYTES: u32 = 1024 * 1024;
const MAX_IN_FLIGHT: usize = 16;
const MAX_SESSION_WATCHES: usize = 16;
pub(crate) const MAX_TURN_TIME: Duration = Duration::from_secs(120);

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
        })
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
        }
    }
}

impl<S: marlin_agent_storage::SessionEventStore + Clone + 'static> AppServer<S> {
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
}

async fn forward_turn(
    mut upstream: ModelGatewayEventStream,
    send: &mpsc::Sender<TurnEvent>,
) -> Result<(), Status> {
    let mut started = false;
    let mut text_chunks = 0;
    let mut text_bytes = 0;
    let mut output_bytes = 0;
    loop {
        let event = upstream
            .next()
            .await
            .ok_or_else(invalid_stream)?
            .map_err(gateway_status)?;
        let (event, terminal) = match event {
            ModelStreamEvent::Started { .. } if !started => {
                started = true;
                (Event::Started(TurnStarted {}), false)
            }
            ModelStreamEvent::Chunk(chunk) if started && chunk.sequence == text_chunks + 1 => {
                account_output(&mut output_bytes, &chunk.content)?;
                text_chunks = chunk.sequence;
                text_bytes += chunk.content.len() as u64;
                (
                    Event::TextDelta(TextDelta {
                        sequence: text_chunks,
                        text: chunk.content,
                    }),
                    false,
                )
            }
            ModelStreamEvent::Thinking(chunk) if started => {
                account_output(&mut output_bytes, &chunk.content)?;
                continue;
            }
            ModelStreamEvent::Completed { response } if started => {
                let [choice] = response.choices.as_slice() else {
                    return Err(invalid_stream());
                };
                if choice.index != 0
                    || choice.message.role != ModelGatewayMessageRole::Assistant
                    || !choice.message.tool_calls.is_empty()
                    || choice.message.content.len() as u64 != text_bytes
                {
                    return Err(invalid_stream());
                }
                (
                    Event::Completed(TurnCompleted {
                        text_chunks,
                        text_bytes,
                    }),
                    true,
                )
            }
            _ => return Err(invalid_stream()),
        };
        if send.send(TurnEvent { event: Some(event) }).await.is_err() || terminal {
            return Ok(());
        }
    }
}

fn messages(request: StreamTurnRequest) -> Result<Vec<ModelGatewayMessage>, Status> {
    if request.messages.len() > MAX_MESSAGES as usize {
        return Err(Status::resource_exhausted(
            "conversation has too many messages",
        ));
    }
    if request.messages.is_empty() || request.messages.len().is_multiple_of(2) {
        return Err(Status::invalid_argument(
            "conversation must end with a user message",
        ));
    }
    request
        .messages
        .into_iter()
        .enumerate()
        .map(|(index, message)| {
            if message.text.trim().is_empty() {
                return Err(Status::invalid_argument(
                    "conversation message must contain text",
                ));
            }
            match (index.is_multiple_of(2), MessageRole::try_from(message.role)) {
                (true, Ok(MessageRole::User)) => Ok(user_gateway_message(message.text)),
                (false, Ok(MessageRole::Assistant)) => Ok(assistant_gateway_message(message.text)),
                _ => Err(Status::invalid_argument(
                    "expected alternating user and assistant messages",
                )),
            }
        })
        .collect()
}

fn session_deadline(metadata: &MetadataMap) -> Result<Instant, Status> {
    Ok(Instant::now() + turn_timeout(metadata)?)
}

pub(crate) fn check_session_deadline(deadline: Instant) -> Result<(), Status> {
    if Instant::now() >= deadline {
        return Err(session_deadline_exceeded());
    }
    Ok(())
}

fn session_deadline_exceeded() -> Status {
    Status::deadline_exceeded("Session request deadline elapsed")
}

pub(crate) async fn session_until<T>(
    deadline: Instant,
    operation: impl Future<Output = Result<T, Status>>,
) -> Result<T, Status> {
    check_session_deadline(deadline)?;
    let result = tokio::time::timeout_at(deadline, operation)
        .await
        .map_err(|_| session_deadline_exceeded())?;
    check_session_deadline(deadline)?;
    result
}

fn turn_timeout(metadata: &MetadataMap) -> Result<Duration, Status> {
    let Some(value) = metadata.get("grpc-timeout") else {
        return Ok(MAX_TURN_TIME);
    };
    let raw = value.as_encoded_bytes();
    let invalid = || Status::invalid_argument("invalid gRPC timeout");
    let (unit, digits) = raw.split_last().ok_or_else(invalid)?;
    if digits.is_empty() || digits.len() > 8 || !digits.iter().all(u8::is_ascii_digit) {
        return Err(invalid());
    }
    let value = digits
        .iter()
        .fold(0, |n, digit| n * 10 + u64::from(digit - b'0'));
    let duration = match unit {
        b'H' => Duration::from_secs(value * 3600),
        b'M' => Duration::from_secs(value * 60),
        b'S' => Duration::from_secs(value),
        b'm' => Duration::from_millis(value),
        b'u' => Duration::from_micros(value),
        b'n' => Duration::from_nanos(value),
        _ => return Err(invalid()),
    };
    Ok(duration.min(MAX_TURN_TIME))
}

pub(crate) fn account_output(bytes: &mut usize, text: &str) -> Result<(), Status> {
    if text.is_empty() || text.len() > MODEL_STREAM_MAX_CHUNK_BYTES {
        return Err(invalid_stream());
    }
    *bytes += text.len();
    if *bytes > MAX_OUTPUT_TEXT_BYTES as usize {
        return Err(Status::resource_exhausted(
            "model output exceeds the turn budget",
        ));
    }
    Ok(())
}

fn invalid_stream() -> Status {
    Status::data_loss("invalid model turn stream")
}

fn deadline_exceeded() -> Status {
    Status::deadline_exceeded("model turn deadline elapsed")
}

fn gateway_status(error: ModelGatewayError) -> Status {
    match error {
        ModelGatewayError::StreamOpenTimeout
        | ModelGatewayError::StreamInterrupted(ModelStreamInterruption::Timeout) => {
            deadline_exceeded()
        }
        ModelGatewayError::StreamInterrupted(
            ModelStreamInterruption::OutputLimit { .. }
            | ModelStreamInterruption::CompletionHeapLimit { .. }
            | ModelStreamInterruption::ProviderByteLimit,
        ) => Status::resource_exhausted("model output exceeds the turn budget"),
        ModelGatewayError::EndpointContract(_) | ModelGatewayError::RequestNotDispatched(_) => {
            Status::failed_precondition("configured model route rejected the turn")
        }
        ModelGatewayError::StreamInterrupted(
            ModelStreamInterruption::EndedWithoutTerminal
            | ModelStreamInterruption::InvalidCompletion,
        ) => invalid_stream(),
        _ => Status::unavailable("model gateway could not complete the turn"),
    }
}

fn session_unavailable() -> Status {
    Status::failed_precondition("the host has not bound a Session")
}
