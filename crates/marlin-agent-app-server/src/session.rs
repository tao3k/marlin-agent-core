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
    ConversationCancellationEvidence, ConversationCheckpointChannel, ConversationEvent,
    ConversationEventReader, ConversationJournal, ConversationJournalError,
    ConversationRetirementStatus, ConversationSubmissionId, ConversationTailState,
    ConversationTerminalObservation, ConversationTurnId, ConversationTurnOutcome,
    ConversationTurnStart, ConversationUncertaintyReason, RuntimeTaskTrackerPolicy,
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
        CancelSessionTurnRequest, CancelSessionTurnResponse, CancellationEvidenceUnavailable,
        GetSessionSummaryRequest, GetSessionSummaryResponse, GetSessionTurnCancellationRequest,
        GetSessionTurnCancellationResponse, RuntimeRetirementStatus, RuntimeShutdownReceipt,
        SessionRecoveryState, SessionTurnEvent, SessionTurnOutcome, SessionTurnRef,
        StartSessionTurnRequest, TextCheckpoint, TurnStarted, WatchSessionTurnRequest,
        get_session_turn_cancellation_response::Evidence, session_turn_event::Event,
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

    pub(crate) async fn summary(
        &self,
        request: GetSessionSummaryRequest,
    ) -> Result<GetSessionSummaryResponse, Status> {
        self.check_scope(&request.session_id)?;
        let owner = &self.inner;
        // Pair one indexed read with this binding's local owner. A concurrent
        // admission can retry the summary once it has published its task scope.
        let current = owner
            .current
            .try_lock()
            .map_err(|_| Status::aborted("Session admission is busy"))?;
        let running = current
            .as_ref()
            .filter(|active| active.finished.borrow().is_none())
            .map(|active| active.id);
        let Some(tail) = owner.reader.latest_turn().await.map_err(journal_status)? else {
            return Ok(GetSessionSummaryResponse {
                latest_turn: None,
                latest_event_id: None,
                recovery_state: SessionRecoveryState::Empty.into(),
            });
        };
        let recovery_state = match tail.state {
            ConversationTailState::Finished => SessionRecoveryState::Finished,
            ConversationTailState::Uncertain => SessionRecoveryState::Unresolved,
            ConversationTailState::ToolContinuationPending => {
                SessionRecoveryState::ToolContinuationPending
            }
            ConversationTailState::Unfinished => {
                if running == Some(tail.turn) {
                    SessionRecoveryState::Active
                } else {
                    SessionRecoveryState::Unresolved
                }
            }
        };
        Ok(GetSessionSummaryResponse {
            latest_turn: Some(owner.wire_turn(tail.turn)),
            latest_event_id: Some(tail.event_id.as_str().to_owned()),
            recovery_state: recovery_state.into(),
        })
    }

    pub(crate) async fn start(
        &self,
        request: StartSessionTurnRequest,
        endpoint: ModelEndpoint,
        gateway: Arc<dyn ModelStreamingGateway>,
    ) -> Result<SessionTurnRef, Status> {
        self.check_scope(&request.session_id)?;
        let submission = ConversationSubmissionId::new(request.submission_id.clone())
            .ok_or_else(|| Status::invalid_argument("invalid submission ID"))?;
        let owner = self.inner.clone();
        if let Some(turn) = owner.resolve_submission(&submission, &request).await? {
            return Ok(turn);
        }
        if request.user_text.trim().is_empty()
            || request.user_text.len() > MAX_REQUEST_BYTES as usize
        {
            return Err(Status::invalid_argument("invalid user text"));
        }
        let Ok(current) = owner.current.clone().try_lock_owned() else {
            return owner
                .resolve_submission(&submission, &request)
                .await?
                .ok_or_else(|| Status::aborted("Session admission is busy"));
        };
        let (reply, result) = owner.runtime.oneshot_channel();
        // Admission outlives its RPC after the journal write has begun.
        let admission = self.inner.runtime.spawn(async move {
            admit(
                owner, current, submission, request, endpoint, gateway, reply,
            )
            .await;
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
                if matches!(
                    cursor.event,
                    ConversationEvent::Outcome(_)
                        | ConversationEvent::ToolReview(_)
                        | ConversationEvent::ToolExecutionClaim(_)
                        | ConversationEvent::ToolResults(_)
                ) {
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
                        ConversationEvent::ToolReview(_) | ConversationEvent::ToolExecutionClaim(_) | ConversationEvent::ToolResults(_) => continue,
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
        let evidence = cancellation_evidence(&receipt);
        self.inner
            .reader
            .record_cancellation(id, &evidence, now_ms())
            .await
            .map_err(journal_status)?;
        let outcome = if evidence.completed() {
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
            runtime: Some(runtime_receipt(evidence)),
            outcome: outcome.into(),
        })
    }

    pub(crate) async fn read_cancellation(
        &self,
        request: GetSessionTurnCancellationRequest,
    ) -> Result<GetSessionTurnCancellationResponse, Status> {
        let (turn, id) = self.turn_ref(request.turn)?;
        let reader = &self.inner.reader;
        if !reader
            .has_committed_start(id)
            .await
            .map_err(journal_status)?
        {
            return Err(Status::not_found("turn has no committed start"));
        }
        let evidence = match reader
            .cancellation_evidence(id)
            .await
            .map_err(journal_status)?
        {
            Some(evidence) => Evidence::Recorded(runtime_receipt(evidence)),
            None => Evidence::Unavailable(CancellationEvidenceUnavailable {}),
        };
        let outcome = reader.committed_outcome(id).await.map_err(journal_status)?;
        Ok(GetSessionTurnCancellationResponse {
            turn: Some(turn),
            evidence: Some(evidence),
            outcome: outcome
                .as_ref()
                .map_or(SessionTurnOutcome::Unspecified, outcome_status)
                .into(),
        })
    }
}

impl<S: SessionEventStore> Binding<S> {
    fn wire_turn(&self, id: ConversationTurnId) -> SessionTurnRef {
        SessionTurnRef {
            session_id: self.session.as_str().to_owned(),
            ordinal: id.get(),
        }
    }

    async fn resolve_submission(
        &self,
        id: &ConversationSubmissionId,
        request: &StartSessionTurnRequest,
    ) -> Result<Option<SessionTurnRef>, Status> {
        self.reader
            .resolve_submission(id, &request.user_text, request.after_turn)
            .await
            .map(|turn| turn.map(|turn| self.wire_turn(turn)))
            .map_err(journal_status)
    }
}

async fn admit<S: SessionEventStore + Clone + 'static>(
    owner: Arc<Binding<S>>,
    mut current: OwnedMutexGuard<Option<ActiveTurn>>,
    submission: ConversationSubmissionId,
    request: StartSessionTurnRequest,
    endpoint: ModelEndpoint,
    gateway: Arc<dyn ModelStreamingGateway>,
    reply: oneshot::Sender<Result<SessionTurnRef, Status>>,
) {
    match owner.resolve_submission(&submission, &request).await {
        Ok(None) => {}
        Ok(Some(turn)) => {
            let _ = reply.send(Ok(turn));
            return;
        }
        Err(error) => {
            let _ = reply.send(Err(error));
            return;
        }
    }
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
        let mut history = Vec::new();
        let mut bytes = request.user_text.len();
        let mut history_error = None;
        let mut journal = ConversationJournal::open_controlled(
            owner.storage.clone(),
            owner.project.clone(),
            owner.session.clone(),
            owner.agent.clone(),
            |turn| {
                if matches!(
                    turn.outcome,
                    Some(ConversationTurnOutcome::ToolRequested { .. })
                ) {
                    history_error = Some("Session history contains a pending tool request");
                    return false;
                }
                if let Some(ConversationTurnOutcome::Completed { response }) = &turn.outcome {
                    let Some(user_text) = turn.start.user_text() else {
                        history_error = Some("Session history contains a tool continuation");
                        return false;
                    };
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
                        .saturating_add(user_text.len())
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
                    history.push(user_gateway_message(user_text.to_owned()));
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
        if reply.is_closed() || owner.runtime.cancellation_token().is_cancelled() {
            return Err(Status::cancelled("turn admission cancelled before commit"));
        }
        history.push(user_gateway_message(request.user_text.clone()));
        let id = journal
            .start_submission(
                &submission,
                ConversationTurnStart {
                    endpoint: endpoint.clone(),
                    user_text: request.user_text.clone(),
                },
                request.after_turn,
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
            // Another binding can commit during replay or race the atomic write.
            // A recovered receipt is read-only, including after an ambiguous write.
            let result = owner
                .resolve_submission(&submission, &request)
                .await
                .and_then(|turn| turn.ok_or(status));
            let _ = reply.send(result);
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
    let _ = reply.send(Ok(owner.wire_turn(id)));
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

fn cancellation_evidence(receipt: &RuntimeTaskShutdownReceipt) -> ConversationCancellationEvidence {
    ConversationCancellationEvidence {
        status: match receipt.status() {
            RuntimeTaskShutdownStatus::Completed => ConversationRetirementStatus::Completed,
            RuntimeTaskShutdownStatus::TimedOut => ConversationRetirementStatus::TimedOut,
            RuntimeTaskShutdownStatus::CleanupUnconfirmed => {
                ConversationRetirementStatus::CleanupUnconfirmed
            }
        },
        cancellation_requested: receipt.cancellation_requested(),
        tracker_closed: receipt.tracker_closed(),
        tracked_tasks_at_shutdown: receipt.tracked_resource_count_at_shutdown() as u64,
        remaining_tasks: receipt.remaining_resource_count() as u64,
        timeout_ms: receipt.timeout_ms(),
    }
}

fn runtime_receipt(evidence: ConversationCancellationEvidence) -> RuntimeShutdownReceipt {
    RuntimeShutdownReceipt {
        completed: evidence.completed(),
        cancellation_requested: evidence.cancellation_requested,
        tracker_closed: evidence.tracker_closed,
        tracked_tasks_at_shutdown: evidence.tracked_tasks_at_shutdown,
        remaining_tasks: evidence.remaining_tasks,
        timeout_ms: evidence.timeout_ms,
        status: match evidence.status {
            ConversationRetirementStatus::Completed => RuntimeRetirementStatus::Completed,
            ConversationRetirementStatus::TimedOut => RuntimeRetirementStatus::TimedOut,
            ConversationRetirementStatus::CleanupUnconfirmed => {
                RuntimeRetirementStatus::CleanupUnconfirmed
            }
        }
        .into(),
    }
}

fn journal_status(error: ConversationJournalError) -> Status {
    match error {
        ConversationJournalError::SubmissionConflict => {
            Status::already_exists("submission ID was already used with different input")
        }
        ConversationJournalError::StaleTurn => {
            Status::failed_precondition("Session turn cursor is stale")
        }
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
