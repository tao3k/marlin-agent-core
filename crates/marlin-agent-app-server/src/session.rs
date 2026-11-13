// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Transport adapter for the host's existing Session and Runtime.

use std::{
    num::NonZeroU64,
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayError, ModelGatewayMessageRole, ModelGatewayRequest,
    ModelStreamInterruption, user_gateway_message,
};
use marlin_agent_runtime::{
    RuntimeTaskScope, RuntimeTaskShutdownReceipt, RuntimeTaskShutdownStatus, TokioAgentRuntime,
};
use marlin_agent_sessions::{
    ConversationCheckpointChannel, ConversationEvent, ConversationEventReader, ConversationJournal,
    ConversationJournalError, ConversationTerminalObservation, ConversationTurnId,
    ConversationTurnOutcome, ConversationTurnStart, ConversationUncertaintyReason,
    RuntimeTaskTrackerPolicy,
};
use marlin_agent_storage::{
    AgentId, EventId, ProjectId, SessionEventStore, SessionId, StorageError, StoragePageLimit,
};
use marlin_agent_stream::{ModelStreamEvent, ModelStreamingGateway};
use tokio::{
    sync::{Mutex, OwnedMutexGuard, oneshot, watch},
    time::Instant,
};
use tokio_stream::{Stream, StreamExt};
use tonic::Status;

use crate::{
    grpc::{MAX_REQUEST_BYTES, MAX_TURN_TIME, account_output},
    v1::{
        CancelSessionTurnRequest, CancelSessionTurnResponse, RuntimeShutdownReceipt,
        SessionTurnEvent, SessionTurnOutcome, SessionTurnRef, StartSessionTurnRequest,
        TextCheckpoint, TurnStarted, WatchSessionTurnRequest, session_turn_event::Event,
    },
};

pub(crate) type WatchStream = Pin<Box<dyn Stream<Item = Result<SessionTurnEvent, Status>> + Send>>;

/// One authorized project/Session binding. All service clones share its owner.
#[derive(Clone)]
pub struct SessionBinding<S> {
    inner: Arc<Binding<S>>,
}

struct Binding<S> {
    storage: S,
    project: ProjectId,
    session: SessionId,
    agent: AgentId,
    reader: ConversationEventReader<S>,
    runtime: TokioAgentRuntime,
    current: Arc<Mutex<Option<ActiveTurn>>>,
    changed: watch::Sender<()>,
}

#[derive(Clone)]
struct ActiveTurn {
    id: ConversationTurnId,
    runtime: RuntimeTaskScope,
    finished: watch::Receiver<Option<Result<SessionTurnOutcome, Status>>>,
}

impl<S: SessionEventStore + Clone + 'static> SessionBinding<S> {
    pub fn new(
        storage: S,
        project: ProjectId,
        agent: AgentId,
        runtime: TokioAgentRuntime,
    ) -> Result<Self, StorageError> {
        let session = SessionId::new(runtime.session().session_id().as_str())?;
        let reader = ConversationEventReader::new(
            storage.clone(),
            project.clone(),
            session.clone(),
            agent.clone(),
        );
        Ok(Self {
            inner: Arc::new(Binding {
                storage,
                project,
                session,
                agent,
                reader,
                runtime,
                current: Arc::new(Mutex::new(None)),
                changed: watch::channel(()).0,
            }),
        })
    }

    pub(crate) fn session_id(&self) -> &str {
        self.inner.session.as_str()
    }

    fn check_scope(&self, session: &str) -> Result<(), Status> {
        if session == self.session_id() {
            Ok(())
        } else {
            Err(Status::not_found("Session is outside this service binding"))
        }
    }

    fn turn_ref(
        &self,
        turn: Option<SessionTurnRef>,
    ) -> Result<(SessionTurnRef, ConversationTurnId), Status> {
        let turn = turn.ok_or_else(|| Status::invalid_argument("turn reference required"))?;
        self.check_scope(&turn.session_id)?;
        let ordinal = NonZeroU64::new(turn.ordinal)
            .ok_or_else(|| Status::invalid_argument("turn ordinal must be positive"))?;
        Ok((turn, ConversationTurnId::from_ordinal(ordinal)))
    }

    pub(crate) async fn start(
        &self,
        request: StartSessionTurnRequest,
        endpoint: ModelEndpoint,
        gateway: Arc<dyn ModelStreamingGateway>,
    ) -> Result<SessionTurnRef, Status> {
        self.check_scope(&request.session_id)?;
        if request.user_text.trim().is_empty()
            || request.user_text.len() > MAX_REQUEST_BYTES as usize
        {
            return Err(Status::invalid_argument("invalid user text"));
        }
        let owner = self.inner.clone();
        let current = owner
            .current
            .clone()
            .try_lock_owned()
            .map_err(|_| Status::aborted("Session admission is busy"))?;
        let (reply, result) = owner.runtime.oneshot_channel();
        // Admission outlives its RPC after the journal write has begun.
        let admission = self.inner.runtime.spawn(async move {
            admit(owner, current, request, endpoint, gateway, reply).await;
        });
        admission
            .admission()
            .map_err(|_| Status::unavailable("Session Runtime is not admitting work"))?;
        result
            .await
            .map_err(|_| Status::internal("turn admission task failed"))?
    }

    pub(crate) async fn watch(
        &self,
        request: WatchSessionTurnRequest,
        deadline: Instant,
    ) -> Result<WatchStream, Status> {
        let (_, id) = self.turn_ref(request.turn)?;
        let owner = self.inner.clone();
        let mut changed = owner.changed.subscribe();
        let limit = StoragePageLimit::new(16).expect("valid journal page limit");
        let mut cursor = match request.after_event_id {
            Some(event_id) => {
                let event_id = EventId::new(event_id)
                    .map_err(|_| Status::invalid_argument("invalid event cursor"))?;
                let cursor = owner
                    .reader
                    .resolve_cursor(id, event_id)
                    .await
                    .map_err(journal_status)?
                    .ok_or_else(|| {
                        Status::invalid_argument("cursor is not a committed event in this turn")
                    })?;
                if matches!(cursor.event, ConversationEvent::Outcome(_)) {
                    return Ok(Box::pin(tokio_stream::empty()));
                }
                Some(cursor.cursor)
            }
            None => None,
        };
        // Subscribe before reading and re-read after each committed notification.
        let stream = async_stream::try_stream! {
            let mut known = cursor.is_some();
            loop {
                if Instant::now() >= deadline {
                    Err(Status::deadline_exceeded("turn watch deadline elapsed"))?;
                }
                changed.borrow_and_update();
                // Snapshot liveness before reading the page to avoid missing a
                // final commit racing the empty page.
                let running = owner.current.lock().await.as_ref().is_some_and(|active|
                    active.id == id && active.finished.borrow().is_none());
                let page = owner.reader.page(id, cursor.clone(), limit).await.map_err(journal_status)?;
                if page.is_empty() {
                    if !known { Err(Status::not_found("turn has no committed start"))?; }
                    if !running {
                        Err(Status::failed_precondition("turn has no live owner or committed outcome"))?;
                    }
                    tokio::time::timeout_at(deadline, changed.changed()).await
                        .map_err(|_| Status::deadline_exceeded("turn watch deadline elapsed"))?
                        .map_err(|_| Status::unavailable("Session owner closed"))?;
                    continue;
                }
                known = true;
                for entry in page {
                    let event_id = entry.cursor.event_id().as_str().to_owned();
                    cursor = Some(entry.cursor);
                    let (event, terminal) = match entry.event {
                        ConversationEvent::Started(_) => (Event::Started(TurnStarted {}), false),
                        ConversationEvent::Checkpoint { channel: ConversationCheckpointChannel::Text, text } =>
                            (Event::TextCheckpoint(TextCheckpoint { text }), false),
                        ConversationEvent::Checkpoint { channel: ConversationCheckpointChannel::Thinking, .. } => continue,
                        ConversationEvent::Outcome(outcome) => (Event::Outcome(outcome_status(&outcome).into()), true),
                    };
                    yield SessionTurnEvent { event_id, event: Some(event) };
                    if terminal { return; }
                }
            }
        };
        Ok(Box::pin(stream))
    }

    pub(crate) async fn cancel(
        &self,
        request: CancelSessionTurnRequest,
    ) -> Result<CancelSessionTurnResponse, Status> {
        let (turn, id) = self.turn_ref(request.turn)?;
        let mut active = self
            .inner
            .current
            .lock()
            .await
            .as_ref()
            .filter(|active| active.id == id)
            .cloned()
            .ok_or_else(|| Status::not_found("turn has no local Runtime task scope"))?;
        let policy = if active.finished.borrow().is_some() {
            RuntimeTaskTrackerPolicy::wait_on_shutdown(Some(10_000))
        } else {
            RuntimeTaskTrackerPolicy::cancel_on_shutdown(Some(10_000))
        };
        let receipt = active.runtime.shutdown_tasks(&policy).await;
        let outcome = if *receipt.status() == RuntimeTaskShutdownStatus::Completed {
            let outcome = active
                .finished
                .wait_for(Option::is_some)
                .await
                .map_err(|_| Status::internal("turn task ended without an outcome"))?;
            outcome.as_ref().expect("completed outcome").clone()?
        } else {
            SessionTurnOutcome::Unspecified
        };
        Ok(CancelSessionTurnResponse {
            turn: Some(turn),
            runtime: Some(runtime_receipt(&receipt)),
            outcome: outcome.into(),
        })
    }
}

async fn admit<S: SessionEventStore + Clone + 'static>(
    owner: Arc<Binding<S>>,
    mut current: OwnedMutexGuard<Option<ActiveTurn>>,
    request: StartSessionTurnRequest,
    endpoint: ModelEndpoint,
    gateway: Arc<dyn ModelStreamingGateway>,
    reply: oneshot::Sender<Result<SessionTurnRef, Status>>,
) {
    let admitted = async {
        if reply.is_closed() || owner.runtime.cancellation_token().is_cancelled() {
            return Err(Status::cancelled("turn admission cancelled"));
        }
        if current
            .as_ref()
            .is_some_and(|active| active.finished.borrow().is_none())
        {
            return Err(Status::failed_precondition(
                "Session already has an active turn",
            ));
        }
        let mut last_turn = 0;
        let mut history = Vec::new();
        let mut bytes = request.user_text.len();
        let mut history_error = None;
        let mut journal = ConversationJournal::open_controlled(
            owner.storage.clone(),
            owner.project.clone(),
            owner.session.clone(),
            owner.agent.clone(),
            |turn| {
                last_turn = turn.id.get();
                if matches!(
                    turn.outcome,
                    Some(ConversationTurnOutcome::ToolRequested { .. })
                ) {
                    history_error = Some("Session history contains a pending tool request");
                    return false;
                }
                if let Some(ConversationTurnOutcome::Completed { response }) = &turn.outcome {
                    let Some(choice) = response
                        .choices
                        .iter()
                        .find(|choice| choice.message.role == ModelGatewayMessageRole::Assistant)
                    else {
                        history_error = Some("stored response has no assistant message");
                        return false;
                    };
                    if let Some(continuation) = &choice.message.continuation
                        && (continuation.endpoint != endpoint || continuation.destination.is_none())
                    {
                        history_error = Some("stored continuation requires its original route");
                        return false;
                    }
                    bytes = bytes
                        .saturating_add(turn.start.user_text.len())
                        .saturating_add(choice.message.content.len())
                        .saturating_add(
                            choice
                                .message
                                .continuation
                                .as_ref()
                                .map_or(0, |continuation| continuation.accounted_bytes()),
                        );
                    if bytes > MAX_REQUEST_BYTES as usize
                        || history.len() >= crate::grpc::MAX_MESSAGES as usize - 1
                    {
                        return false;
                    }
                    history.push(user_gateway_message(turn.start.user_text.clone()));
                    history.push(choice.message.clone());
                }
                true
            },
        )
        .await
        .map_err(|error| {
            if let Some(reason) = history_error {
                Status::failed_precondition(reason)
            } else if matches!(error, ConversationJournalError::ReplayStopped) {
                Status::resource_exhausted("Session history exceeds the model input budget")
            } else {
                journal_status(error)
            }
        })?;
        if request.after_turn != last_turn {
            return Err(Status::failed_precondition("Session turn cursor is stale"));
        }
        if reply.is_closed() || owner.runtime.cancellation_token().is_cancelled() {
            return Err(Status::cancelled("turn admission cancelled before commit"));
        }
        history.push(user_gateway_message(request.user_text.clone()));
        let id = journal
            .start(
                ConversationTurnStart {
                    endpoint: endpoint.clone(),
                    user_text: request.user_text,
                },
                now_ms(),
            )
            .await
            .map_err(journal_status)?;
        Ok((id, journal, ModelGatewayRequest::new(endpoint, history)))
    }
    .await;
    let (id, journal, model_request) = match admitted {
        Ok(value) => value,
        Err(status) => {
            let _ = reply.send(Err(status));
            return;
        }
    };
    let runtime = owner.runtime.child_task_scope();
    let token = runtime.cancellation_token();
    let changed = owner.changed.clone();
    let (finish, finished) = owner.runtime.watch_channel(None);
    let task = runtime.spawn(execute(journal, id, model_request, gateway, token, changed));
    *current = Some(ActiveTurn {
        id,
        runtime,
        finished,
    });
    drop(current);
    owner.changed.send_replace(());
    let _ = reply.send(Ok(SessionTurnRef {
        session_id: owner.session.as_str().to_owned(),
        ordinal: id.get(),
    }));
    let outcome = task
        .join()
        .await
        .map_err(|_| Status::internal("turn Runtime task failed"))
        .and_then(|outcome| outcome);
    finish.send_replace(Some(outcome));
    owner.changed.send_replace(());
}

async fn execute<S: SessionEventStore>(
    mut journal: ConversationJournal<S>,
    id: ConversationTurnId,
    request: ModelGatewayRequest,
    gateway: Arc<dyn ModelStreamingGateway>,
    token: marlin_agent_runtime::CancellationToken,
    changed: watch::Sender<()>,
) -> Result<SessionTurnOutcome, Status> {
    let deadline = Instant::now() + MAX_TURN_TIME;
    let opened = tokio::select! {
        biased;
        () = token.cancelled() => return uncertain(&mut journal, id, interrupted(ModelStreamInterruption::CancelledLocally)).await,
        result = tokio::time::timeout_at(deadline, gateway.stream(request)) => result,
    };
    let mut upstream = match opened {
        Err(_) | Ok(Err(ModelGatewayError::StreamOpenTimeout)) => {
            return uncertain(
                &mut journal,
                id,
                ConversationUncertaintyReason::StreamOpenTimeout,
            )
            .await;
        }
        Ok(Err(
            ModelGatewayError::RequestNotDispatched(_) | ModelGatewayError::EndpointContract(_),
        )) => {
            journal
                .finish(
                    id,
                    ConversationTerminalObservation::RejectedBeforeDispatch {
                        diagnostic: "configured model route rejected the turn".into(),
                    },
                    now_ms(),
                )
                .await
                .map_err(journal_status)?;
            return Ok(SessionTurnOutcome::RejectedBeforeDispatch);
        }
        Ok(Err(ModelGatewayError::StreamOpenTransport)) => {
            return uncertain(
                &mut journal,
                id,
                ConversationUncertaintyReason::StreamOpenTransport,
            )
            .await;
        }
        Ok(Err(_)) => {
            return uncertain(
                &mut journal,
                id,
                ConversationUncertaintyReason::StreamOpenUnclassified,
            )
            .await;
        }
        Ok(Ok(stream)) => stream,
    };
    let mut started = false;
    let mut sequence = 0;
    let mut bytes = 0;
    let outcome = loop {
        let next = tokio::select! {
            biased;
            () = token.cancelled() => break Err(ModelStreamInterruption::CancelledLocally),
            () = tokio::time::sleep_until(deadline) => break Err(ModelStreamInterruption::Timeout),
            event = upstream.next() => event,
        };
        match next {
            Some(Ok(ModelStreamEvent::Started { .. })) if !started => started = true,
            Some(Ok(ModelStreamEvent::Chunk(chunk)))
                if started && chunk.sequence == sequence + 1 =>
            {
                if account_output(&mut bytes, &chunk.content).is_err() {
                    break Err(ModelStreamInterruption::InvalidResponse);
                }
                journal
                    .checkpoint(id, &chunk.content, now_ms())
                    .await
                    .map_err(journal_status)?;
                sequence = chunk.sequence;
                changed.send_replace(());
            }
            Some(Ok(ModelStreamEvent::Thinking(chunk))) if started => {
                if account_output(&mut bytes, &chunk.content).is_err() {
                    break Err(ModelStreamInterruption::InvalidResponse);
                }
                journal
                    .checkpoint_thinking(id, &chunk.content, now_ms())
                    .await
                    .map_err(journal_status)?;
            }
            Some(Ok(ModelStreamEvent::Completed { response })) if started => {
                let text = &journal
                    .unfinished()
                    .expect("owned turn is active")
                    .partial_text;
                if !matches!(response.choices.as_slice(), [choice] if choice.index == 0 && choice.message.role == ModelGatewayMessageRole::Assistant && choice.message.tool_calls.is_empty() && &choice.message.content == text)
                {
                    break Err(ModelStreamInterruption::InvalidCompletion);
                }
                break Ok(response);
            }
            Some(Err(ModelGatewayError::StreamInterrupted(reason))) => break Err(reason),
            Some(_) => break Err(ModelStreamInterruption::InvalidResponse),
            None => break Err(ModelStreamInterruption::EndedWithoutTerminal),
        }
    };
    drop(upstream);
    match outcome {
        Ok(response) => {
            journal
                .finish(
                    id,
                    ConversationTerminalObservation::Completed { response },
                    now_ms(),
                )
                .await
                .map_err(journal_status)?;
            Ok(SessionTurnOutcome::Completed)
        }
        Err(reason) => uncertain(&mut journal, id, interrupted(reason)).await,
    }
}

fn interrupted(reason: ModelStreamInterruption) -> ConversationUncertaintyReason {
    ConversationUncertaintyReason::StreamInterrupted { reason }
}

async fn uncertain<S: SessionEventStore>(
    journal: &mut ConversationJournal<S>,
    id: ConversationTurnId,
    reason: ConversationUncertaintyReason,
) -> Result<SessionTurnOutcome, Status> {
    journal
        .mark_uncertain(id, reason, now_ms())
        .await
        .map_err(journal_status)?;
    Ok(SessionTurnOutcome::RemoteUncertain)
}

fn outcome_status(outcome: &ConversationTurnOutcome) -> SessionTurnOutcome {
    match outcome {
        ConversationTurnOutcome::Completed { .. } => SessionTurnOutcome::Completed,
        ConversationTurnOutcome::RejectedBeforeDispatch { .. } => {
            SessionTurnOutcome::RejectedBeforeDispatch
        }
        _ => SessionTurnOutcome::RemoteUncertain,
    }
}

fn runtime_receipt(receipt: &RuntimeTaskShutdownReceipt) -> RuntimeShutdownReceipt {
    RuntimeShutdownReceipt {
        completed: *receipt.status() == RuntimeTaskShutdownStatus::Completed,
        cancellation_requested: receipt.cancellation_requested(),
        tracker_closed: receipt.tracker_closed(),
        tracked_tasks_at_shutdown: receipt.tracked_resource_count_at_shutdown() as u64,
        remaining_tasks: receipt.remaining_resource_count() as u64,
        timeout_ms: receipt.timeout_ms(),
    }
}

fn journal_status(error: ConversationJournalError) -> Status {
    match error {
        ConversationJournalError::UnfinishedTurn
        | ConversationJournalError::LegacyOutcomeUnresolved => {
            Status::failed_precondition("Session has an unresolved turn")
        }
        ConversationJournalError::Storage(StorageError::DuplicateSessionEvent { .. })
        | ConversationJournalError::ReplayInconsistent => {
            Status::aborted("Session journal changed during admission")
        }
        ConversationJournalError::Storage(_) => {
            Status::unavailable("Session event storage unavailable")
        }
        _ => Status::data_loss("Session journal rejected the event"),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|time| i64::try_from(time.as_millis()).ok())
        .unwrap_or_default()
}
