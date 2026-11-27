// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::time::Duration;

use marlin_agent_protocol::{
    ModelGatewayError, ModelGatewayMessage, ModelGatewayMessageRole, ModelStreamInterruption,
    assistant_gateway_message, user_gateway_message,
};
use marlin_agent_stream::{
    MODEL_STREAM_MAX_CHUNK_BYTES, ModelGatewayEventStream, ModelStreamEvent,
};
use tokio::{sync::mpsc, time::Instant};
use tokio_stream::StreamExt;
use tonic::{Status, metadata::MetadataMap};

use crate::v1::{
    MessageRole, StreamTurnRequest, TextDelta, TurnCompleted, TurnEvent, TurnStarted,
    turn_event::Event,
};

pub(crate) const MAX_REQUEST_BYTES: u32 = 256 * 1024;
pub(crate) const MAX_MESSAGES: u32 = 128;
pub(super) const MAX_OUTPUT_TEXT_BYTES: u32 = 1024 * 1024;
pub(crate) const MAX_TURN_TIME: Duration = Duration::from_secs(120);

pub(super) async fn forward_turn(
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

pub(super) fn messages(request: StreamTurnRequest) -> Result<Vec<ModelGatewayMessage>, Status> {
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

pub(super) fn session_deadline(metadata: &MetadataMap) -> Result<Instant, Status> {
    Ok(Instant::now() + turn_timeout(metadata)?)
}

pub(crate) fn check_session_deadline(deadline: Instant) -> Result<(), Status> {
    if Instant::now() >= deadline {
        return Err(session_deadline_exceeded());
    }
    Ok(())
}

pub(super) fn session_deadline_exceeded() -> Status {
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

pub(super) fn turn_timeout(metadata: &MetadataMap) -> Result<Duration, Status> {
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

pub(super) fn invalid_stream() -> Status {
    Status::data_loss("invalid model turn stream")
}

pub(super) fn deadline_exceeded() -> Status {
    Status::deadline_exceeded("model turn deadline elapsed")
}

pub(super) fn gateway_status(error: ModelGatewayError) -> Status {
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

pub(super) fn session_unavailable() -> Status {
    Status::failed_precondition("the host has not bound a Session")
}

pub(super) fn task_actor_error_status(err: marlin_agent_runtime::TaskActorRuntimeError) -> Status {
    use marlin_agent_runtime::TaskActorRuntimeError as E;
    match err {
        E::DuplicateActor { .. } => Status::already_exists(err.to_string()),
        E::ActorNotFound { .. } => Status::not_found(err.to_string()),
        E::ActorTerminal { .. }
        | E::FencingEpochMismatch { .. }
        | E::ScopeModeMismatch { .. }
        | E::DispatchRejected(_)
        | E::GovernedContextRejected(_)
        | E::StaleContextCut { .. }
        | E::InvalidDetourTransition { .. }
        | E::BlockerDetourActive { .. }
        | E::CannotPeekActiveActor { .. }
        | E::NoPendingSteeringBranches { .. } => Status::failed_precondition(err.to_string()),
        E::GovernanceDenied { .. } => Status::permission_denied(err.to_string()),
        E::OracleDivergence { .. } | E::PersistenceError { .. } => {
            Status::internal(err.to_string())
        }
    }
}

pub(super) fn scope_mode_from_proto(raw: i32) -> Result<marlin_agent_runtime::TaskActorScopeMode, Status> {
    match crate::v1::TaskActorScopeModeProto::try_from(raw) {
        Ok(crate::v1::TaskActorScopeModeProto::Project) => {
            Ok(marlin_agent_runtime::TaskActorScopeMode::Project)
        }
        Ok(crate::v1::TaskActorScopeModeProto::Detached) => {
            Ok(marlin_agent_runtime::TaskActorScopeMode::Detached)
        }
        _ => Err(Status::invalid_argument("invalid scope_mode")),
    }
}

pub(super) fn scope_mode_to_proto(mode: marlin_agent_runtime::TaskActorScopeMode) -> i32 {
    match mode {
        marlin_agent_runtime::TaskActorScopeMode::Project => {
            crate::v1::TaskActorScopeModeProto::Project.into()
        }
        marlin_agent_runtime::TaskActorScopeMode::Detached => {
            crate::v1::TaskActorScopeModeProto::Detached.into()
        }
    }
}

pub(super) fn status_to_proto(status: marlin_agent_runtime::TaskActorStatus) -> i32 {
    match status {
        marlin_agent_runtime::TaskActorStatus::OpenFocus => {
            crate::v1::TaskActorStatusProto::OpenFocus.into()
        }
        marlin_agent_runtime::TaskActorStatus::Running => {
            crate::v1::TaskActorStatusProto::Running.into()
        }
        marlin_agent_runtime::TaskActorStatus::WaitingSteering => {
            crate::v1::TaskActorStatusProto::WaitingSteering.into()
        }
        marlin_agent_runtime::TaskActorStatus::Settled => {
            crate::v1::TaskActorStatusProto::Settled.into()
        }
        marlin_agent_runtime::TaskActorStatus::Failed => {
            crate::v1::TaskActorStatusProto::Failed.into()
        }
        marlin_agent_runtime::TaskActorStatus::Aborted => {
            crate::v1::TaskActorStatusProto::Aborted.into()
        }
    }
}

pub(super) fn operation_from_proto(raw: i32) -> Result<marlin_agent_runtime::GraphDispatchOperation, Status> {
    match crate::v1::GraphDispatchOperationProto::try_from(raw) {
        Ok(crate::v1::GraphDispatchOperationProto::ExecuteFrontier) => {
            Ok(marlin_agent_runtime::GraphDispatchOperation::ExecuteFrontier)
        }
        Ok(crate::v1::GraphDispatchOperationProto::ApplySteering) => {
            Ok(marlin_agent_runtime::GraphDispatchOperation::ApplySteering)
        }
        Ok(crate::v1::GraphDispatchOperationProto::ConvergeApex) => {
            Ok(marlin_agent_runtime::GraphDispatchOperation::ConvergeApex)
        }
        Ok(crate::v1::GraphDispatchOperationProto::CommitSettlement) => {
            Ok(marlin_agent_runtime::GraphDispatchOperation::CommitSettlement)
        }
        _ => Err(Status::invalid_argument("invalid dispatch operation")),
    }
}

pub(super) fn operation_to_proto(op: marlin_agent_runtime::GraphDispatchOperation) -> i32 {
    match op {
        marlin_agent_runtime::GraphDispatchOperation::ExecuteFrontier => {
            crate::v1::GraphDispatchOperationProto::ExecuteFrontier.into()
        }
        marlin_agent_runtime::GraphDispatchOperation::ApplySteering => {
            crate::v1::GraphDispatchOperationProto::ApplySteering.into()
        }
        marlin_agent_runtime::GraphDispatchOperation::ConvergeApex => {
            crate::v1::GraphDispatchOperationProto::ConvergeApex.into()
        }
        marlin_agent_runtime::GraphDispatchOperation::CommitSettlement => {
            crate::v1::GraphDispatchOperationProto::CommitSettlement.into()
        }
    }
}

pub(super) fn substrate_from_proto(raw: i32) -> Result<marlin_agent_runtime::GraphSubstrateTarget, Status> {
    match crate::v1::GraphSubstrateTargetProto::try_from(raw) {
        Ok(crate::v1::GraphSubstrateTargetProto::PureMemory) => {
            Ok(marlin_agent_runtime::GraphSubstrateTarget::PureMemory)
        }
        Ok(crate::v1::GraphSubstrateTargetProto::SqliteTurso) => {
            Ok(marlin_agent_runtime::GraphSubstrateTarget::SqliteTurso)
        }
        _ => Err(Status::invalid_argument("invalid substrate target")),
    }
}

pub(super) fn substrate_to_proto(target: marlin_agent_runtime::GraphSubstrateTarget) -> i32 {
    match target {
        marlin_agent_runtime::GraphSubstrateTarget::PureMemory => {
            crate::v1::GraphSubstrateTargetProto::PureMemory.into()
        }
        marlin_agent_runtime::GraphSubstrateTarget::SqliteTurso => {
            crate::v1::GraphSubstrateTargetProto::SqliteTurso.into()
        }
    }
}

pub(super) fn receipt_status_from_proto(
    raw: i32,
) -> Result<marlin_agent_runtime::ExecutionReceiptStatus, Status> {
    match crate::v1::ExecutionReceiptStatusProto::try_from(raw) {
        Ok(crate::v1::ExecutionReceiptStatusProto::Ok) => {
            Ok(marlin_agent_runtime::ExecutionReceiptStatus::Ok)
        }
        Ok(crate::v1::ExecutionReceiptStatusProto::Settled) => {
            Ok(marlin_agent_runtime::ExecutionReceiptStatus::Settled)
        }
        Ok(crate::v1::ExecutionReceiptStatusProto::Failed) => {
            Ok(marlin_agent_runtime::ExecutionReceiptStatus::Failed)
        }
        Ok(crate::v1::ExecutionReceiptStatusProto::Rejected) => {
            Ok(marlin_agent_runtime::ExecutionReceiptStatus::Rejected)
        }
        _ => Err(Status::invalid_argument("invalid receipt status")),
    }
}

pub(super) fn disclosure_depth_from_proto(raw: i32) -> Result<marlin_agent_runtime::DisclosureDepth, Status> {
    match crate::v1::DisclosureDepthProto::try_from(raw) {
        Ok(crate::v1::DisclosureDepthProto::L0) => Ok(marlin_agent_runtime::DisclosureDepth::L0),
        Ok(crate::v1::DisclosureDepthProto::L1) => Ok(marlin_agent_runtime::DisclosureDepth::L1),
        Ok(crate::v1::DisclosureDepthProto::L2) => Ok(marlin_agent_runtime::DisclosureDepth::L2),
        Ok(crate::v1::DisclosureDepthProto::L3) => Ok(marlin_agent_runtime::DisclosureDepth::L3),
        _ => Err(Status::invalid_argument("invalid disclosure_depth")),
    }
}

pub(super) fn disclosure_depth_to_proto(depth: marlin_agent_runtime::DisclosureDepth) -> i32 {
    match depth {
        marlin_agent_runtime::DisclosureDepth::L0 => crate::v1::DisclosureDepthProto::L0.into(),
        marlin_agent_runtime::DisclosureDepth::L1 => crate::v1::DisclosureDepthProto::L1.into(),
        marlin_agent_runtime::DisclosureDepth::L2 => crate::v1::DisclosureDepthProto::L2.into(),
        marlin_agent_runtime::DisclosureDepth::L3 => crate::v1::DisclosureDepthProto::L3.into(),
    }
}

pub(super) fn detour_mode_from_proto(raw: i32) -> Result<marlin_agent_runtime::DetourMode, Status> {
    match crate::v1::DetourModeProto::try_from(raw) {
        Ok(crate::v1::DetourModeProto::Mainline) => Ok(marlin_agent_runtime::DetourMode::Mainline),
        Ok(crate::v1::DetourModeProto::BlockerActive) => {
            Ok(marlin_agent_runtime::DetourMode::BlockerActive)
        }
        Ok(crate::v1::DetourModeProto::SideQuestActive) => {
            Ok(marlin_agent_runtime::DetourMode::SideQuestActive)
        }
        _ => Err(Status::invalid_argument("invalid detour_mode")),
    }
}

pub(super) fn detour_mode_to_proto(mode: marlin_agent_runtime::DetourMode) -> i32 {
    match mode {
        marlin_agent_runtime::DetourMode::Mainline => crate::v1::DetourModeProto::Mainline.into(),
        marlin_agent_runtime::DetourMode::BlockerActive => {
            crate::v1::DetourModeProto::BlockerActive.into()
        }
        marlin_agent_runtime::DetourMode::SideQuestActive => {
            crate::v1::DetourModeProto::SideQuestActive.into()
        }
    }
}

pub(super) fn cursor_to_proto(
    cursor: &marlin_agent_runtime::TaskActorCursor,
) -> crate::v1::TaskActorCursorProto {
    crate::v1::TaskActorCursorProto {
        task_id: cursor.task_id.as_str().to_owned(),
        fencing_epoch: cursor.fencing_epoch.value(),
        status: status_to_proto(cursor.status),
        scope_mode: scope_mode_to_proto(cursor.scope_mode),
        pending_steering_branches: cursor.pending_steering_branches,
    }
}

pub(super) fn workbench_tabs_to_proto(
    tabs: &marlin_agent_runtime::WorkbenchTabProjection,
) -> crate::v1::WorkbenchTabsResponse {
    crate::v1::WorkbenchTabsResponse {
        focused_task_id: tabs
            .focused_task_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        tabs: tabs
            .tabs
            .iter()
            .map(|entry| crate::v1::WorkbenchTabEntryProto {
                kind: match entry.kind {
                    marlin_agent_runtime::WorkbenchTabKind::PinnedIndex => {
                        crate::v1::WorkbenchTabKindProto::PinnedIndex.into()
                    }
                    marlin_agent_runtime::WorkbenchTabKind::ActiveTask => {
                        crate::v1::WorkbenchTabKindProto::ActiveTask.into()
                    }
                    marlin_agent_runtime::WorkbenchTabKind::ReadOnlyPeek => {
                        crate::v1::WorkbenchTabKindProto::ReadOnlyPeek.into()
                    }
                },
                id: entry.id.clone(),
                title: entry.title.clone(),
                status_label: entry.status_label.clone(),
                focused: entry.focused,
                fencing_epoch: entry.fencing_epoch,
            })
            .collect(),
    }
}
