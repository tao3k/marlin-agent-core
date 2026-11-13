// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded TUI projections of validated conversation replay.

use std::{collections::VecDeque, error::Error, fmt, mem::size_of};

use marlin_agent_protocol::{ModelGatewayMessage, ModelGatewayMessageRole};
use marlin_agent_sessions::{
    ConversationCheckpointSpan, ConversationJournal, ConversationJournalError, ConversationTurn,
    ConversationTurnId, ConversationTurnOutcome, ConversationUncertaintyReason,
};
use marlin_agent_storage::{AgentId, ProjectId, SessionEventStore, SessionId};

const MAX_REPLAY_TURNS: usize = 10_000;
const MAX_REPLAY_BYTES: usize = 64 * 1024 * 1024;
const HISTORY_WINDOW_TURNS: usize = 100;
const HISTORY_WINDOW_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) struct ReplayLimits {
    pub(crate) turns: usize,
    pub(crate) bytes: usize,
}

impl Default for ReplayLimits {
    fn default() -> Self {
        Self {
            turns: MAX_REPLAY_TURNS,
            bytes: MAX_REPLAY_BYTES,
        }
    }
}

pub(crate) struct ReplayTurn {
    pub(crate) id: ConversationTurnId,
    pub(crate) prompt: String,
    pub(crate) thinking: String,
    pub(crate) checkpoint_spans: Vec<ConversationCheckpointSpan>,
    pub(crate) outcome: ReplayOutcome,
}

pub(crate) enum ReplayOutcome {
    Completed(ModelGatewayMessage),
    ToolRequested(ModelGatewayMessage),
    Failed {
        diagnostic: String,
        partial: String,
    },
    LegacyFailed {
        diagnostic: String,
        partial: String,
    },
    LegacyCancelled {
        partial: String,
    },
    Unfinished {
        partial: String,
    },
    Uncertain {
        reason: ConversationUncertaintyReason,
        partial: String,
    },
    Omitted,
}

pub(crate) struct ReplayProjection {
    turns: Vec<ReplayTurn>,
    bytes: usize,
    max_turns: usize,
    max_bytes: usize,
    rejected: bool,
}

impl ReplayProjection {
    pub(crate) fn new(max_turns: usize, max_bytes: usize) -> Self {
        Self {
            turns: Vec::new(),
            bytes: 0,
            max_turns,
            max_bytes,
            rejected: false,
        }
    }

    pub(crate) fn push(&mut self, turn: &ConversationTurn) -> bool {
        let Some(bytes) = projected_bytes(turn) else {
            self.rejected = true;
            return false;
        };
        if self.turns.len() >= self.max_turns || self.bytes.saturating_add(bytes) > self.max_bytes {
            self.rejected = true;
            return false;
        }
        self.turns.push(project_turn(turn));
        self.bytes += bytes;
        true
    }
}

fn projected_bytes(turn: &ConversationTurn) -> Option<usize> {
    let bytes = size_of::<ReplayTurn>()
        .saturating_add(turn.start.user_text.len())
        .saturating_add(turn.partial_thinking.len())
        .saturating_add(
            turn.checkpoint_spans
                .len()
                .saturating_mul(size_of::<ConversationCheckpointSpan>()),
        );
    let extra = match &turn.outcome {
        Some(ConversationTurnOutcome::Completed { response }) => {
            let choice = response
                .choices
                .iter()
                .find(|choice| choice.message.role == ModelGatewayMessageRole::Assistant)?;
            choice
                .message
                .content
                .len()
                .saturating_add(choice.message.name.as_ref().map_or(0, String::len))
                .saturating_add(
                    choice
                        .message
                        .continuation
                        .as_ref()
                        .map_or(0, |state| state.accounted_bytes()),
                )
        }
        Some(ConversationTurnOutcome::ToolRequested { response }) => response.heap_capacity(),
        Some(ConversationTurnOutcome::Failed {
            diagnostic,
            partial_text,
        }) => diagnostic.len().saturating_add(partial_text.len()),
        Some(ConversationTurnOutcome::CancelledLocally { partial_text }) => partial_text.len(),
        Some(ConversationTurnOutcome::RejectedBeforeDispatch { diagnostic }) => diagnostic.len(),
        Some(ConversationTurnOutcome::Uncertain { .. }) => turn.partial_text.len(),
        None => turn.partial_text.len(),
    };
    Some(bytes.saturating_add(extra))
}

fn project_turn(turn: &ConversationTurn) -> ReplayTurn {
    let outcome = match &turn.outcome {
        Some(ConversationTurnOutcome::Completed { response }) => ReplayOutcome::Completed(
            response
                .choices
                .iter()
                .find(|choice| choice.message.role == ModelGatewayMessageRole::Assistant)
                .expect("assistant checked")
                .message
                .clone(),
        ),
        Some(ConversationTurnOutcome::ToolRequested { response }) => {
            ReplayOutcome::ToolRequested(response.choices[0].message.clone())
        }
        Some(ConversationTurnOutcome::Failed {
            diagnostic,
            partial_text,
        }) => ReplayOutcome::LegacyFailed {
            diagnostic: diagnostic.clone(),
            partial: partial_text.clone(),
        },
        Some(ConversationTurnOutcome::CancelledLocally { partial_text }) => {
            ReplayOutcome::LegacyCancelled {
                partial: partial_text.clone(),
            }
        }
        Some(ConversationTurnOutcome::RejectedBeforeDispatch { diagnostic }) => {
            ReplayOutcome::Failed {
                diagnostic: diagnostic.clone(),
                partial: String::new(),
            }
        }
        Some(ConversationTurnOutcome::Uncertain { reason }) => ReplayOutcome::Uncertain {
            reason: *reason,
            partial: turn.partial_text.clone(),
        },
        None => ReplayOutcome::Unfinished {
            partial: turn.partial_text.clone(),
        },
    };
    ReplayTurn {
        id: turn.id,
        prompt: turn.start.user_text.clone(),
        thinking: turn.partial_thinking.clone(),
        checkpoint_spans: turn.checkpoint_spans.clone(),
        outcome,
    }
}

pub(crate) struct HistoryWindow {
    pub(crate) turns: Vec<ReplayTurn>,
    pub(crate) has_earlier: bool,
    pub(crate) before: Option<ConversationTurnId>,
}

struct HistoryProjection {
    turns: VecDeque<(ReplayTurn, usize)>,
    bytes: usize,
    before: Option<ConversationTurnId>,
    has_earlier: bool,
    max_turns: usize,
    max_bytes: usize,
}

impl HistoryProjection {
    fn new(before: Option<ConversationTurnId>, max_turns: usize, max_bytes: usize) -> Self {
        Self {
            turns: VecDeque::new(),
            bytes: 0,
            before,
            has_earlier: false,
            max_turns,
            max_bytes,
        }
    }

    fn push(&mut self, turn: &ConversationTurn) -> bool {
        if self.before.is_some_and(|before| turn.id >= before) {
            return true;
        }
        let Some(bytes) = projected_bytes(turn) else {
            return false;
        };
        const OMITTED_PROMPT: &str = "[turn omitted: exceeds history page budget]";
        let omitted = bytes > self.max_bytes;
        let bytes = if omitted {
            size_of::<ReplayTurn>() + OMITTED_PROMPT.len()
        } else {
            bytes
        };
        if self.max_turns == 0 || bytes > self.max_bytes {
            return false;
        }
        while self.turns.len() >= self.max_turns
            || self.bytes.saturating_add(bytes) > self.max_bytes
        {
            if let Some((_, removed)) = self.turns.pop_front() {
                self.bytes -= removed;
                self.has_earlier = true;
            } else {
                return false;
            }
        }
        let item = if omitted {
            ReplayTurn {
                id: turn.id,
                prompt: OMITTED_PROMPT.into(),
                thinking: String::new(),
                checkpoint_spans: Vec::new(),
                outcome: ReplayOutcome::Omitted,
            }
        } else {
            project_turn(turn)
        };
        self.bytes += bytes;
        self.turns.push_back((item, bytes));
        true
    }

    fn finish(self) -> HistoryWindow {
        HistoryWindow {
            turns: self.turns.into_iter().map(|(turn, _)| turn).collect(),
            has_earlier: self.has_earlier,
            before: self.before,
        }
    }
}

pub(crate) async fn load_history_window<S: SessionEventStore>(
    storage: S,
    project: ProjectId,
    session: SessionId,
    agent: AgentId,
    before: Option<ConversationTurnId>,
) -> Result<(ConversationJournal<S>, HistoryWindow), ReplayError> {
    load_history_window_with_limits(
        storage,
        project,
        session,
        agent,
        before,
        ReplayLimits {
            turns: HISTORY_WINDOW_TURNS,
            bytes: HISTORY_WINDOW_BYTES,
        },
    )
    .await
}

pub(crate) async fn load_history_window_with_limits<S: SessionEventStore>(
    storage: S,
    project: ProjectId,
    session: SessionId,
    agent: AgentId,
    before: Option<ConversationTurnId>,
    limits: ReplayLimits,
) -> Result<(ConversationJournal<S>, HistoryWindow), ReplayError> {
    let mut projection = HistoryProjection::new(before, limits.turns, limits.bytes);
    let journal = ConversationJournal::open_controlled(storage, project, session, agent, |turn| {
        projection.push(turn)
    })
    .await
    .map_err(ReplayError::Journal)?;
    Ok((journal, projection.finish()))
}

#[derive(Debug)]
pub(crate) enum ReplayError {
    Journal(ConversationJournalError),
    ProjectionLimit { turns: usize, bytes: usize },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Journal(error) => write!(f, "{error}"),
            Self::ProjectionLimit { turns, bytes } => write!(
                f,
                "session exceeds the TUI replay limit ({turns} turns or {bytes} accounted bytes)"
            ),
        }
    }
}
impl Error for ReplayError {}

pub(crate) async fn load_session<S: SessionEventStore>(
    storage: S,
    project: ProjectId,
    session: SessionId,
    agent: AgentId,
) -> Result<(ConversationJournal<S>, Vec<ReplayTurn>), ReplayError> {
    load_session_with_limits(storage, project, session, agent, ReplayLimits::default()).await
}

pub(crate) async fn load_session_with_limits<S: SessionEventStore>(
    storage: S,
    project: ProjectId,
    session: SessionId,
    agent: AgentId,
    limits: ReplayLimits,
) -> Result<(ConversationJournal<S>, Vec<ReplayTurn>), ReplayError> {
    let mut projection = ReplayProjection::new(limits.turns, limits.bytes);
    let result = ConversationJournal::open_controlled(storage, project, session, agent, |turn| {
        projection.push(turn)
    })
    .await;
    match result {
        Ok(journal) => Ok((journal, projection.turns)),
        Err(ConversationJournalError::ReplayStopped) if projection.rejected => {
            Err(ReplayError::ProjectionLimit {
                turns: limits.turns,
                bytes: limits.bytes,
            })
        }
        Err(error) => Err(ReplayError::Journal(error)),
    }
}
