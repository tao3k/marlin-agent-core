// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{pin::Pin, sync::Arc, time::Duration};

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

use crate::v1::{
    GetCapabilitiesRequest, GetCapabilitiesResponse, MessageRole, StreamTurnRequest, TextDelta,
    TurnCompleted, TurnEvent, TurnStarted, app_server_server, turn_event::Event,
};

pub(crate) const MAX_REQUEST_BYTES: u32 = 256 * 1024;
pub(crate) const MAX_MESSAGES: u32 = 128;
const MAX_OUTPUT_TEXT_BYTES: u32 = 1024 * 1024;
const MAX_IN_FLIGHT: usize = 16;
pub(crate) const MAX_TURN_TIME: Duration = Duration::from_secs(120);

type Authorize = dyn Fn(&MetadataMap) -> Result<(), Status> + Send + Sync;

/// A shareable service with a host-selected model route and authorization policy.
/// The host supplies the gateway and owns the listener, TLS and shutdown.
#[derive(Clone)]
pub struct AppServer<S = marlin_agent_storage::InMemoryAgentStorage> {
    endpoint: ModelEndpoint,
    gateway: Arc<dyn ModelStreamingGateway>,
    authorize: Arc<Authorize>,
    in_flight: Arc<Semaphore>,
    session: Option<crate::session::SessionBinding<S>>,
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
            session: None,
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
            session: Some(session),
        }
    }
}

impl<S: marlin_agent_storage::SessionEventStore + Clone + 'static> AppServer<S> {
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

    async fn get_session_summary(
        &self,
        request: Request<crate::v1::GetSessionSummaryRequest>,
    ) -> Result<Response<crate::v1::GetSessionSummaryResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let timeout = turn_timeout(request.metadata())?;
        let binding = self.session.as_ref().ok_or_else(session_unavailable)?;
        let summary = tokio::time::timeout(timeout, binding.summary(request.into_inner()))
            .await
            .map_err(|_| Status::deadline_exceeded("Session summary deadline elapsed"))??;
        Ok(Response::new(summary))
    }

    async fn get_session_turn_cancellation(
        &self,
        request: Request<crate::v1::GetSessionTurnCancellationRequest>,
    ) -> Result<Response<crate::v1::GetSessionTurnCancellationResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let timeout = turn_timeout(request.metadata())?;
        let binding = self.session.as_ref().ok_or_else(session_unavailable)?;
        let evidence =
            tokio::time::timeout(timeout, binding.read_cancellation(request.into_inner()))
                .await
                .map_err(|_| {
                    Status::deadline_exceeded("Cancellation evidence deadline elapsed")
                })??;
        Ok(Response::new(evidence))
    }

    async fn start_session_turn(
        &self,
        request: Request<crate::v1::StartSessionTurnRequest>,
    ) -> Result<Response<crate::v1::StartSessionTurnResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let turn = self
            .session
            .as_ref()
            .ok_or_else(session_unavailable)?
            .start(
                request.into_inner(),
                self.endpoint.clone(),
                self.gateway.clone(),
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
        let deadline = Instant::now() + turn_timeout(request.metadata())?;
        let stream = self
            .session
            .as_ref()
            .ok_or_else(session_unavailable)?
            .watch(request.into_inner(), deadline)
            .await?;
        Ok(Response::new(stream))
    }

    async fn cancel_session_turn(
        &self,
        request: Request<crate::v1::CancelSessionTurnRequest>,
    ) -> Result<Response<crate::v1::CancelSessionTurnResponse>, Status> {
        (self.authorize)(request.metadata())?;
        let receipt = self
            .session
            .as_ref()
            .ok_or_else(session_unavailable)?
            .cancel(request.into_inner())
            .await?;
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

pub(crate) fn turn_timeout(metadata: &MetadataMap) -> Result<Duration, Status> {
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
