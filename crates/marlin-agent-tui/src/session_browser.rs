// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Asynchronous session-page requests for the terminal client.

use super::replay::{
    HistoryWindow, ReplayError, ReplayLimits, ReplayTurn, load_history_window,
    load_session_with_limits,
};
use marlin_agent_sessions::{ConversationJournal, ConversationTurnId};
use marlin_agent_storage::{
    AgentId, ProjectId, SessionEventStore, SessionId, SessionPageRequest, StorageError,
    StoragePage, StoragePageLimit,
};
use tokio::{runtime::Runtime, sync::oneshot};

const PAGE_SIZE: u16 = 20;
pub(crate) type SessionPage = StoragePage<SessionId, SessionId>;

pub(crate) fn validate_page(
    page: SessionPage,
    after: Option<&SessionId>,
) -> Result<SessionPage, StorageError> {
    let invalid = page.items.len() > usize::from(PAGE_SIZE)
        || page.items.windows(2).any(|pair| pair[0] >= pair[1])
        || page
            .items
            .first()
            .is_some_and(|first| after.is_some_and(|cursor| first <= cursor))
        || page
            .next_cursor
            .as_ref()
            .is_some_and(|cursor| page.items.last() != Some(cursor));
    if invalid {
        return Err(StorageError::Backend {
            message: "invalid session page ordering or cursor".into(),
        });
    }
    Ok(page)
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SessionPageRequestError {
    Busy,
    NoNextPage,
}

pub(crate) struct SessionBrowser<S> {
    storage: S,
    project: ProjectId,
    agent: AgentId,
    pending: Option<oneshot::Receiver<Result<SessionPage, StorageError>>>,
    next: Option<SessionId>,
    opening: Option<PendingOpen<S>>,
    replay_limits: ReplayLimits,
}

struct PendingOpen<S> {
    receiver: oneshot::Receiver<Result<OpenedSession<S>, ReplayError>>,
    task: tokio::task::JoinHandle<()>,
}

pub(crate) struct OpenedSession<S> {
    pub(crate) id: SessionId,
    pub(crate) journal: ConversationJournal<S>,
    pub(crate) view: OpenedView,
}

pub(crate) enum OpenedView {
    Full(Vec<ReplayTurn>),
    History(HistoryWindow),
}

#[derive(Debug)]
pub(crate) enum SessionOpenError {
    Replay(ReplayError),
    TaskClosed,
}

impl std::fmt::Display for SessionOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Replay(error) => write!(f, "{error}"),
            Self::TaskClosed => f.write_str("session loading task ended without a result"),
        }
    }
}

impl<S: SessionEventStore + Clone + 'static> SessionBrowser<S> {
    pub(crate) fn new(storage: S, project: ProjectId, agent: AgentId) -> Self {
        Self {
            storage,
            project,
            agent,
            pending: None,
            next: None,
            opening: None,
            replay_limits: ReplayLimits::default(),
        }
    }

    #[cfg(test)]
    #[allow(dead_code, reason = "used by the external unit-test harness")]
    pub(crate) fn with_replay_limits(mut self, limits: ReplayLimits) -> Self {
        self.replay_limits = limits;
        self
    }

    pub(crate) fn is_opening(&self) -> bool {
        self.opening.is_some()
    }

    pub(crate) fn open(
        &mut self,
        runtime: &Runtime,
        id: SessionId,
    ) -> Result<(), SessionPageRequestError> {
        if self.opening.is_some() {
            return Err(SessionPageRequestError::Busy);
        }
        let storage = self.storage.clone();
        let project = self.project.clone();
        let agent = self.agent.clone();
        let limits = self.replay_limits;
        let (sender, receiver) = oneshot::channel();
        let task = runtime.spawn(async move {
            let result = load_session_with_limits(storage, project, id.clone(), agent, limits)
                .await
                .map(|(journal, turns)| OpenedSession {
                    id,
                    journal,
                    view: OpenedView::Full(turns),
                });
            let _ = sender.send(result);
        });
        self.opening = Some(PendingOpen { receiver, task });
        Ok(())
    }

    pub(crate) fn inspect(
        &mut self,
        runtime: &Runtime,
        id: SessionId,
        before: Option<ConversationTurnId>,
    ) -> Result<(), SessionPageRequestError> {
        if self.opening.is_some() {
            return Err(SessionPageRequestError::Busy);
        }
        let storage = self.storage.clone();
        let project = self.project.clone();
        let agent = self.agent.clone();
        let (sender, receiver) = oneshot::channel();
        let task = runtime.spawn(async move {
            let result = load_history_window(storage, project, id.clone(), agent, before)
                .await
                .map(|(journal, window)| OpenedSession {
                    id,
                    journal,
                    view: OpenedView::History(window),
                });
            let _ = sender.send(result);
        });
        self.opening = Some(PendingOpen { receiver, task });
        Ok(())
    }

    pub(crate) fn cancel_open(&mut self) -> bool {
        if let Some(pending) = self.opening.take() {
            pending.task.abort();
            true
        } else {
            false
        }
    }

    pub(crate) fn poll_open(&mut self) -> Option<Result<OpenedSession<S>, SessionOpenError>> {
        let result = match self.opening.as_mut()?.receiver.try_recv() {
            Ok(result) => result.map_err(SessionOpenError::Replay),
            Err(oneshot::error::TryRecvError::Empty) => return None,
            Err(oneshot::error::TryRecvError::Closed) => Err(SessionOpenError::TaskClosed),
        };
        self.opening = None;
        Some(result)
    }

    pub(crate) fn request(
        &mut self,
        runtime: &Runtime,
        next: bool,
    ) -> Result<(), SessionPageRequestError> {
        if self.pending.is_some() {
            return Err(SessionPageRequestError::Busy);
        }
        let cursor = if next {
            Some(
                self.next
                    .clone()
                    .ok_or(SessionPageRequestError::NoNextPage)?,
            )
        } else {
            None
        };
        let storage = self.storage.clone();
        let mut request = SessionPageRequest::new(
            self.project.clone(),
            self.agent.clone(),
            StoragePageLimit::new(PAGE_SIZE).expect("valid session page size"),
        );
        if let Some(cursor) = cursor.clone() {
            request = request.after(cursor);
        }
        let (sender, receiver) = oneshot::channel();
        self.pending = Some(receiver);
        runtime.spawn(async move {
            let result = storage
                .list_sessions_page(request)
                .await
                .and_then(|page| validate_page(page, cursor.as_ref()));
            let _ = sender.send(result);
        });
        Ok(())
    }

    pub(crate) fn poll(&mut self) -> Option<Result<SessionPage, StorageError>> {
        let result = match self.pending.as_mut()?.try_recv() {
            Ok(result) => result,
            Err(oneshot::error::TryRecvError::Empty) => return None,
            Err(oneshot::error::TryRecvError::Closed) => Err(StorageError::Backend {
                message: "session listing task ended without a result".into(),
            }),
        };
        self.pending = None;
        if let Ok(page) = &result {
            self.next = page.next_cursor.clone();
        }
        Some(result)
    }
}
