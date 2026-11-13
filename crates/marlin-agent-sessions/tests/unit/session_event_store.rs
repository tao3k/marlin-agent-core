// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_protocol::ModelEndpoint;
use marlin_agent_sessions::{
    ConversationEventReader, ConversationJournal, ConversationJournalError,
    ConversationSubmissionId, ConversationTerminalObservation, ConversationTurnOutcome,
    ConversationTurnStart,
};
use marlin_agent_storage::{
    AgentId, InMemoryAgentStorage, ProjectId, SessionEventBatchWriteReceipt, SessionEventCount,
    SessionEventCursor, SessionEventPageRequest, SessionEventRecord, SessionEventStore, SessionId,
    SessionPageRequest, StorageFuture, StoragePage,
};

// Implements only the session contract, demonstrating that a dedicated Org
// backend need not implement unrelated artifact or topology operations.
#[derive(Clone, Default)]
struct SessionOnly(InMemoryAgentStorage);

#[tokio::test]
async fn duplicate_submission_rolls_back_new_ordinal_and_survives_reopen() {
    let store = SessionOnly::default();
    let project = ProjectId::new("project").unwrap();
    let session = SessionId::new("session").unwrap();
    let agent = AgentId::new("agent").unwrap();
    let submission = ConversationSubmissionId::new("send-1".into()).unwrap();
    let start = ConversationTurnStart {
        endpoint: ModelEndpoint::new("openai", "gpt-fixture"),
        user_text: "hello".into(),
    };
    let mut journal = ConversationJournal::open(
        store.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let first = journal
        .start_submission(&submission, start.clone(), 0, 1)
        .await
        .unwrap();
    journal
        .finish(
            first,
            ConversationTerminalObservation::RejectedBeforeDispatch {
                diagnostic: "fixture".into(),
            },
            2,
        )
        .await
        .unwrap();
    assert!(matches!(
        journal
            .start_submission(&submission, start.clone(), 1, 3)
            .await,
        Err(ConversationJournalError::Storage(
            marlin_agent_storage::StorageError::DuplicateSessionEvent { .. }
        ))
    ));
    let second = journal.start(start, 4).await.unwrap();
    assert_eq!(second.get(), 2);
    let reader = ConversationEventReader::new(store, project, session, agent);
    assert_eq!(
        reader
            .resolve_submission(&submission, "hello", 0)
            .await
            .unwrap(),
        Some(first)
    );
}

impl SessionEventStore for SessionOnly {
    fn count_session_events<'a>(
        &'a self,
        p: &'a ProjectId,
        s: &'a SessionId,
    ) -> StorageFuture<'a, SessionEventCount> {
        self.0.count_session_events(p, s)
    }
    fn list_sessions_page<'a>(
        &'a self,
        r: SessionPageRequest,
    ) -> StorageFuture<'a, StoragePage<SessionId, SessionId>> {
        self.0.list_sessions_page(r)
    }
    fn append_session_event<'a>(&'a self, r: SessionEventRecord) -> StorageFuture<'a, ()> {
        self.0.append_session_event(r)
    }
    fn append_session_events_atomically<'a>(
        &'a self,
        r: Vec<SessionEventRecord>,
    ) -> StorageFuture<'a, SessionEventBatchWriteReceipt> {
        self.0.append_session_events_atomically(r)
    }
    fn list_session_events_page<'a>(
        &'a self,
        r: SessionEventPageRequest,
    ) -> StorageFuture<'a, StoragePage<SessionEventRecord, SessionEventCursor>> {
        self.0.list_session_events_page(r)
    }
}

#[derive(Clone)]
struct WrongScope {
    inner: SessionOnly,
    change_project: bool,
}

impl SessionEventStore for WrongScope {
    fn count_session_events<'a>(
        &'a self,
        p: &'a ProjectId,
        s: &'a SessionId,
    ) -> StorageFuture<'a, SessionEventCount> {
        self.inner.count_session_events(p, s)
    }
    fn list_sessions_page<'a>(
        &'a self,
        request: SessionPageRequest,
    ) -> StorageFuture<'a, StoragePage<SessionId, SessionId>> {
        self.inner.list_sessions_page(request)
    }
    fn append_session_event<'a>(&'a self, record: SessionEventRecord) -> StorageFuture<'a, ()> {
        self.inner.append_session_event(record)
    }
    fn append_session_events_atomically<'a>(
        &'a self,
        records: Vec<SessionEventRecord>,
    ) -> StorageFuture<'a, SessionEventBatchWriteReceipt> {
        self.inner.append_session_events_atomically(records)
    }
    fn list_session_events_page<'a>(
        &'a self,
        request: SessionEventPageRequest,
    ) -> StorageFuture<'a, StoragePage<SessionEventRecord, SessionEventCursor>> {
        Box::pin(async move {
            let mut page = self.inner.list_session_events_page(request).await?;
            for record in &mut page.items {
                if self.change_project {
                    record.project_id = ProjectId::new("other-project")?;
                } else {
                    record.session_id = SessionId::new("other-session")?;
                }
            }
            Ok(page)
        })
    }
}

#[tokio::test]
async fn replay_rejects_events_outside_requested_project_or_session() {
    let store = SessionOnly::default();
    let project = ProjectId::new("project").unwrap();
    let session = SessionId::new("session").unwrap();
    let agent = AgentId::new("agent").unwrap();
    let mut journal = ConversationJournal::open(
        store.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    journal
        .start(
            ConversationTurnStart {
                endpoint: ModelEndpoint::new("openai", "gpt-fixture"),
                user_text: "private context".into(),
            },
            1,
        )
        .await
        .unwrap();
    for change_project in [true, false] {
        let mut visited = false;
        let result = ConversationJournal::open(
            WrongScope {
                inner: store.clone(),
                change_project,
            },
            project.clone(),
            session.clone(),
            agent.clone(),
            |_| visited = true,
        )
        .await;
        assert!(
            matches!(result, Err(ConversationJournalError::InvalidEvent(message)) if message.contains("crosses session scope"))
        );
        assert!(!visited);
    }
}

#[tokio::test]
async fn conversation_journal_accepts_session_only_storage() {
    let store = SessionOnly::default();
    let project = ProjectId::new("project").unwrap();
    let session = SessionId::new("session").unwrap();
    let agent = AgentId::new("agent").unwrap();
    let mut journal = ConversationJournal::open(
        store.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let id = journal
        .start(
            ConversationTurnStart {
                endpoint: ModelEndpoint::new("openai", "gpt-fixture"),
                user_text: "hello".into(),
            },
            1,
        )
        .await
        .unwrap();
    journal
        .finish(
            id,
            ConversationTerminalObservation::RejectedBeforeDispatch {
                diagnostic: "fixture".into(),
            },
            2,
        )
        .await
        .unwrap();
    let mut turns = Vec::new();
    ConversationJournal::open(store, project, session, agent, |turn| {
        turns.push(turn.clone())
    })
    .await
    .unwrap();
    assert_eq!(turns.len(), 1);
    assert!(matches!(
        turns[0].outcome,
        Some(ConversationTurnOutcome::RejectedBeforeDispatch { .. })
    ));
}
