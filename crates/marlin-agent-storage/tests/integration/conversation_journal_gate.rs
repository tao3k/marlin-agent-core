// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(feature = "turso")]
use marlin_agent_storage::SessionEventStore;

use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    assistant_gateway_message,
};
use marlin_agent_sessions::{
    ConversationJournal, ConversationJournalError, ConversationTurnOutcome, ConversationTurnStart,
};
use marlin_agent_storage::{
    AgentId, EventId, ProjectId, SessionEventRecord, SessionId, SessionPageRequest, StorageError,
    StoragePageLimit, TurnId, TursoAgentStorage, TursoAgentStorageConfig,
    TursoBatchTransactionMode, TursoOptimizationProfile,
};

fn start(text: &str) -> ConversationTurnStart {
    ConversationTurnStart {
        endpoint: ModelEndpoint::new("openai", "gpt-fixture"),
        user_text: text.into(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_replay_rejects_an_append_between_pages() {
    use std::{sync::mpsc, time::Duration};
    let temp = tempfile::tempdir().unwrap();
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: temp.path().join("concurrent.db"),
        optimization_profile:
            TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
        batch_transaction_mode: TursoBatchTransactionMode::Concurrent,
    })
    .await
    .unwrap();
    let project = ProjectId::new("project:race").unwrap();
    let session = SessionId::new("session:race").unwrap();
    let agent = AgentId::new("agent:race").unwrap();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    for index in 0..10 {
        let id = journal.start(start("prompt"), index).await.unwrap();
        let response = ModelGatewayCompletionResponse::new(
            "id",
            "model",
            vec![ModelGatewayCompletionChoice::new(
                0,
                assistant_gateway_message("answer"),
                None,
            )],
        );
        journal
            .finish(
                id,
                marlin_agent_sessions::ConversationTerminalObservation::Completed { response },
                index,
            )
            .await
            .unwrap();
    }
    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let write_storage = storage.clone();
    let record = SessionEventRecord {
        project_id: project.clone(),
        session_id: session.clone(),
        agent_id: agent.clone(),
        turn_id: TurnId::new("z-unrelated").unwrap(),
        event_id: EventId::new("event").unwrap(),
        event_kind: "other.event".into(),
        causality_parent_event_id: None,
        body: Vec::new(),
        created_at_unix_ms: 1,
    };
    let writer = tokio::spawn(async move {
        trigger_rx.await.unwrap();
        let result = write_storage.append_session_event(record).await;
        done_tx.send(result).unwrap();
    });
    let mut trigger = Some(trigger_tx);
    let result = ConversationJournal::open(storage, project, session, agent, |_| {
        if let Some(trigger) = trigger.take() {
            trigger.send(()).unwrap();
            done_rx
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
        }
    })
    .await;
    writer.await.unwrap();
    assert!(matches!(
        result,
        Err(ConversationJournalError::ReplayInconsistent)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "manual paired local diagnostic, not an end-to-end performance gate"]
async fn compare_turso_replay_count_check_cost() {
    use marlin_agent_storage::SessionEventPageRequest;
    use std::time::Instant;
    let temp = tempfile::tempdir().unwrap();
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: temp.path().join("performance.db"),
        optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        batch_transaction_mode: TursoBatchTransactionMode::Immediate,
    })
    .await
    .unwrap();
    let project = ProjectId::new("project:perf").unwrap();
    let session = SessionId::new("session:perf").unwrap();
    let agent = AgentId::new("agent:perf").unwrap();
    let records = (0..1000)
        .map(|index| SessionEventRecord {
            project_id: project.clone(),
            session_id: session.clone(),
            agent_id: agent.clone(),
            turn_id: TurnId::new(format!("other:{index:020}")).unwrap(),
            event_id: EventId::new("event").unwrap(),
            event_kind: "other".into(),
            causality_parent_event_id: None,
            body: vec![0; 128],
            created_at_unix_ms: index,
        })
        .collect();
    storage
        .append_session_events_atomically(records)
        .await
        .unwrap();
    let mut baseline = Vec::new();
    let mut checked = Vec::new();
    for iteration in 0..8 {
        for with_check in [iteration % 2 == 0, iteration % 2 != 0] {
            let begin = Instant::now();
            if with_check {
                ConversationJournal::open(
                    storage.clone(),
                    project.clone(),
                    session.clone(),
                    agent.clone(),
                    |_| {},
                )
                .await
                .unwrap();
            } else {
                let mut cursor = None;
                let mut count = 0;
                loop {
                    let mut request = SessionEventPageRequest::new(
                        project.clone(),
                        session.clone(),
                        StoragePageLimit::new(16).unwrap(),
                    );
                    if let Some(after) = cursor {
                        request = request.after(after);
                    }
                    let page = storage.list_session_events_page(request).await.unwrap();
                    count += page.items.len();
                    match page.next_cursor {
                        Some(next) => cursor = Some(next),
                        None => break,
                    }
                }
                assert_eq!(count, 1000);
            }
            let elapsed = begin.elapsed().as_micros();
            if with_check {
                checked.push(elapsed);
            } else {
                baseline.push(elapsed);
            }
        }
    }
    eprintln!(
        "1000-event local Turso scan: baseline_us={baseline:?} checked_replay_us={checked:?}"
    );
}

#[tokio::test]
async fn turso_session_listing_is_scoped_paginated_and_survives_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let config = TursoAgentStorageConfig {
        path: temp.path().join("sessions.db"),
        optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        batch_transaction_mode: TursoBatchTransactionMode::Immediate,
    };
    let storage = TursoAgentStorage::open_local(config.clone()).await.unwrap();
    let project = ProjectId::new("project:catalog").unwrap();
    let agent = AgentId::new("agent:catalog").unwrap();
    for name in ["b", "a", "c"] {
        let mut journal = ConversationJournal::open(
            storage.clone(),
            project.clone(),
            SessionId::new(name).unwrap(),
            agent.clone(),
            |_| {},
        )
        .await
        .unwrap();
        journal.start(start(name), 1).await.unwrap();
    }
    let mut foreign = ConversationJournal::open(
        storage.clone(),
        ProjectId::new("other").unwrap(),
        SessionId::new("hidden").unwrap(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    foreign.start(start("other"), 1).await.unwrap();
    drop(foreign);
    let mut other_agent = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        SessionId::new("hidden-agent").unwrap(),
        AgentId::new("agent:other").unwrap(),
        |_| {},
    )
    .await
    .unwrap();
    other_agent.start(start("other agent"), 1).await.unwrap();
    drop(other_agent);
    drop(storage);
    let storage = TursoAgentStorage::open_local(config).await.unwrap();
    let limit = StoragePageLimit::new(2).unwrap();
    let first = storage
        .list_sessions_page(SessionPageRequest::new(
            project.clone(),
            agent.clone(),
            limit,
        ))
        .await
        .unwrap();
    assert_eq!(
        first
            .items
            .iter()
            .map(SessionId::as_str)
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    let second = storage
        .list_sessions_page(
            SessionPageRequest::new(project, agent, limit).after(first.next_cursor.unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(
        second
            .items
            .iter()
            .map(SessionId::as_str)
            .collect::<Vec<_>>(),
        ["c"]
    );
    assert!(second.next_cursor.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conversation_turns_survive_reopen_and_stale_claims_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let config = TursoAgentStorageConfig {
        path: temp.path().join("conversation.turso"),
        optimization_profile:
            TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
        batch_transaction_mode: TursoBatchTransactionMode::Concurrent,
    };
    let project = ProjectId::new("project:conversation").unwrap();
    let session = SessionId::new("session:conversation").unwrap();
    let agent = AgentId::new("agent:conversation").unwrap();
    let storage = TursoAgentStorage::open_local(config.clone()).await.unwrap();
    let mut first = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let mut stale = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let id = first.start(start("hello"), 1).await.unwrap();
    assert!(matches!(
        stale.start(start("must not dispatch"), 2).await,
        Err(ConversationJournalError::Storage(
            StorageError::DuplicateSessionEvent { .. }
        ))
    ));
    first.checkpoint_thinking(id, "reasoned ", 2).await.unwrap();
    first.checkpoint(id, "world", 2).await.unwrap();
    first.checkpoint_thinking(id, "more", 2).await.unwrap();
    let response = ModelGatewayCompletionResponse::new(
        "fixture",
        "gpt-fixture",
        vec![ModelGatewayCompletionChoice::new(
            0,
            assistant_gateway_message("world"),
            Some("stop".into()),
        )],
    );
    first
        .finish(
            id,
            marlin_agent_sessions::ConversationTerminalObservation::Completed { response },
            3,
        )
        .await
        .unwrap();
    drop((first, stale, storage));
    let storage = TursoAgentStorage::open_local(config.clone()).await.unwrap();
    let mut replay = Vec::new();
    let mut reopened = ConversationJournal::open(
        storage,
        project.clone(),
        session.clone(),
        agent.clone(),
        |turn| replay.push(turn.clone()),
    )
    .await
    .unwrap();
    assert_eq!(replay.len(), 1);
    assert_eq!(replay[0].start.user_text(), Some("hello"));
    assert_eq!(replay[0].partial_thinking, "reasoned more");
    assert_eq!(replay[0].partial_text, "world");
    use marlin_agent_sessions::ConversationCheckpointChannel::{Text, Thinking};
    assert_eq!(
        replay[0]
            .checkpoint_spans
            .iter()
            .map(|span| span.channel)
            .collect::<Vec<_>>(),
        [Thinking, Text, Thinking]
    );
    assert!(
        matches!(&replay[0].outcome, Some(ConversationTurnOutcome::Completed { response }) if response.choices[0].message.content == "world")
    );
    let unfinished = reopened.start(start("unfinished"), 4).await.unwrap();
    reopened
        .checkpoint_thinking(unfinished, "pending", 4)
        .await
        .unwrap();
    drop(reopened);
    let storage = TursoAgentStorage::open_local(config).await.unwrap();
    let mut observed = Vec::new();
    let mut reopened = ConversationJournal::open(storage, project, session, agent, |turn| {
        observed.push(turn.clone())
    })
    .await
    .unwrap();
    assert_eq!(observed.len(), 2);
    assert_eq!(observed[1].id, unfinished);
    assert_eq!(observed[1].partial_thinking, "pending");
    assert!(observed[1].outcome.is_none());
    assert!(matches!(
        reopened.start(start("not resent"), 5).await,
        Err(ConversationJournalError::UnfinishedTurn)
    ));
    assert!(matches!(
        reopened
            .finish(
                unfinished,
                marlin_agent_sessions::ConversationTerminalObservation::RejectedBeforeDispatch {
                    diagnostic: "not dispatched".into()
                },
                6
            )
            .await,
        Err(ConversationJournalError::UnownedTurn)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "child helper; launched by the process-termination test"]
async fn conversation_crash_child() {
    use std::io::Write;
    let Some(path) = std::env::var_os("MARLIN_CONVERSATION_CRASH_DB") else {
        return;
    };
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: path.into(),
        optimization_profile:
            TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
        batch_transaction_mode: TursoBatchTransactionMode::Concurrent,
    })
    .await
    .unwrap();
    let mut journal = ConversationJournal::open(
        storage,
        ProjectId::new("project:crash").unwrap(),
        SessionId::new("session:crash").unwrap(),
        AgentId::new("agent:crash").unwrap(),
        |_| {},
    )
    .await
    .unwrap();
    journal
        .start(start("persist before dispatch"), 1)
        .await
        .unwrap();
    println!("CONVERSATION_CHILD_READY");
    std::io::stdout().flush().unwrap();
    std::future::pending::<()>().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn killed_process_leaves_unfinished_turn_without_automatic_retry() {
    use std::{
        io::{BufRead, BufReader},
        process::{Command, Stdio},
        sync::mpsc,
        time::Duration,
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("terminated.turso");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "conversation_journal_gate::conversation_crash_child",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env("MARLIN_CONVERSATION_CRASH_DB", &path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if line.contains("CONVERSATION_CHILD_READY") {
                let _ = tx.send(());
                break;
            }
        }
    });
    if rx.recv_timeout(Duration::from_secs(30)).is_err() {
        let _ = child.kill();
        let _ = child.wait();
        panic!("child did not persist its start event");
    }
    child.kill().unwrap();
    let status = child.wait().unwrap();
    assert!(!status.success());
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path,
        optimization_profile:
            TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
        batch_transaction_mode: TursoBatchTransactionMode::Concurrent,
    })
    .await
    .unwrap();
    let mut replay = Vec::new();
    let mut journal = ConversationJournal::open(
        storage,
        ProjectId::new("project:crash").unwrap(),
        SessionId::new("session:crash").unwrap(),
        AgentId::new("agent:crash").unwrap(),
        |turn| replay.push(turn.clone()),
    )
    .await
    .unwrap();
    assert_eq!(replay.len(), 1);
    assert_eq!(replay[0].start.user_text(), Some("persist before dispatch"));
    assert!(replay[0].outcome.is_none());
    assert!(matches!(
        journal.start(start("must not dispatch"), 2).await,
        Err(ConversationJournalError::UnfinishedTurn)
    ));
}
