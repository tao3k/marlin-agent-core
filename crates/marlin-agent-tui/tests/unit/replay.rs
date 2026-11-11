use super::replay::{
    ReplayLimits, ReplayOutcome, ReplayProjection, load_history_window_with_limits, load_session,
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    assistant_gateway_message,
};
use marlin_agent_sessions::{ConversationJournal, ConversationJournalError, ConversationTurnStart};
use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId, SessionId};

fn ids() -> (ProjectId, SessionId, AgentId) {
    (
        ProjectId::new("project:replay").unwrap(),
        SessionId::new("session:replay").unwrap(),
        AgentId::new("agent:replay").unwrap(),
    )
}

#[tokio::test]
async fn thinking_checkpoint_is_preserved_and_counts_against_replay_budget() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
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
                endpoint: ModelEndpoint::new("provider", "model"),
                user_text: "prompt".into(),
            },
            1,
        )
        .await
        .unwrap();
    journal
        .checkpoint_thinking(id, &"t".repeat(1024), 2)
        .await
        .unwrap();
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
            3,
        )
        .await
        .unwrap();
    let (_, turns) = load_session(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
    )
    .await
    .unwrap();
    assert_eq!(turns[0].thinking, "t".repeat(1024));
    let mut projection = ReplayProjection::new(1, 512);
    assert!(
        ConversationJournal::open_controlled(storage, project, session, agent, |turn| projection
            .push(turn))
        .await
        .is_err()
    );
}

#[tokio::test]
async fn replay_limits_stop_without_returning_an_incomplete_journal() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    for index in 0..3 {
        let id = journal
            .start(
                ConversationTurnStart {
                    endpoint: ModelEndpoint::new("provider", "model"),
                    user_text: format!("prompt-{index}"),
                },
                index,
            )
            .await
            .unwrap();
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
    let mut projection = ReplayProjection::new(1, 1024);
    let mut visited = 0;
    let result = ConversationJournal::open_controlled(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |turn| {
            visited += 1;
            projection.push(turn)
        },
    )
    .await;
    assert!(matches!(
        result,
        Err(ConversationJournalError::ReplayStopped)
    ));
    assert_eq!(visited, 2, "replay must stop at the first rejected turn");
    let mut tiny = ReplayProjection::new(100, 1);
    assert!(matches!(
        ConversationJournal::open_controlled(
            storage.clone(),
            project.clone(),
            session.clone(),
            agent.clone(),
            |turn| tiny.push(turn)
        )
        .await,
        Err(ConversationJournalError::ReplayStopped)
    ));
    let (_, turns) = load_session(storage, project, session, agent)
        .await
        .unwrap();
    assert_eq!(turns.len(), 3);
    assert!(
        matches!(&turns[0].outcome, ReplayOutcome::Completed(message) if message.content == "answer")
    );
}

#[tokio::test]
async fn replay_projects_only_required_completion_fields() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
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
                endpoint: ModelEndpoint::new("provider", "model"),
                user_text: "short".into(),
            },
            1,
        )
        .await
        .unwrap();
    let response = ModelGatewayCompletionResponse::new(
        "metadata".repeat(1024),
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
            2,
        )
        .await
        .unwrap();
    let mut small_projection = ReplayProjection::new(1, 1024);
    assert!(
        ConversationJournal::open_controlled(storage, project, session, agent, |turn| {
            small_projection.push(turn)
        })
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn history_pages_are_ordered_bounded_and_preserve_oversized_source() {
    let storage = InMemoryAgentStorage::new();
    let (project, session, agent) = ids();
    let mut journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    for index in 0..5 {
        let id = journal
            .start(
                ConversationTurnStart {
                    endpoint: ModelEndpoint::new("provider", "model"),
                    user_text: format!("prompt-{index}"),
                },
                index,
            )
            .await
            .unwrap();
        let answer = if index == 2 {
            "x".repeat(2048)
        } else {
            "answer".into()
        };
        let response = ModelGatewayCompletionResponse::new(
            "id",
            "model",
            vec![ModelGatewayCompletionChoice::new(
                0,
                assistant_gateway_message(answer),
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
    let limits = ReplayLimits {
        turns: 2,
        bytes: 1024,
    };
    let (_, newest) = load_history_window_with_limits(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        None,
        limits,
    )
    .await
    .unwrap();
    assert_eq!(
        newest
            .turns
            .iter()
            .map(|t| t.prompt.as_str())
            .collect::<Vec<_>>(),
        ["prompt-3", "prompt-4"]
    );
    assert!(newest.has_earlier);
    let (_, middle) = load_history_window_with_limits(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        Some(newest.turns[0].id),
        limits,
    )
    .await
    .unwrap();
    assert_eq!(middle.turns[0].prompt, "prompt-1");
    assert!(matches!(middle.turns[1].outcome, ReplayOutcome::Omitted));
    assert!(middle.has_earlier);
    let (_, oldest) = load_history_window_with_limits(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        Some(middle.turns[0].id),
        limits,
    )
    .await
    .unwrap();
    assert_eq!(oldest.turns.len(), 1);
    assert_eq!(oldest.turns[0].prompt, "prompt-0");
    assert!(!oldest.has_earlier);
    let (_, full) = load_session(storage, project, session, agent)
        .await
        .unwrap();
    assert!(
        matches!(&full[2].outcome, ReplayOutcome::Completed(message) if message.content.len() == 2048)
    );
}
