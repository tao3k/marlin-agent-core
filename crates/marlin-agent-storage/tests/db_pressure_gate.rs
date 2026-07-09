#![cfg(feature = "turso")]

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactRecord, EventId, ProjectId, SessionEventRecord,
    SessionId, StorageResult, TurnId, TursoAgentStorage, TursoAgentStorageConfig, TursoMvccMode,
    VisibilityReceipt,
};
use serde::Deserialize;
use tempfile::tempdir;

#[derive(Debug, Deserialize)]
struct DbPressureDocument {
    db_pressure: DbPressureScenario,
}

#[derive(Debug, Deserialize)]
struct DbPressureScenario {
    sessions: usize,
    events_per_session: usize,
    artifacts: usize,
    visibility_receipts: usize,
}

fn db_pressure_scenario() -> DbPressureScenario {
    toml::from_str::<DbPressureDocument>(include_str!(
        "db_pressure/scenarios/multi_session_reopen.toml"
    ))
    .expect("db pressure scenario fixture must be valid TOML")
    .db_pressure
}

fn project_id() -> ProjectId {
    ProjectId::new("project:storage-db-pressure").expect("valid project id")
}

fn session_id(index: usize) -> SessionId {
    SessionId::new(format!("session:{index:02}")).expect("valid session id")
}

fn agent_id(index: usize) -> AgentId {
    AgentId::new(format!("agent:{index:02}")).expect("valid agent id")
}

fn turn_id(session: usize, event: usize) -> TurnId {
    TurnId::new(format!("turn:{session:02}:{event:03}")).expect("valid turn id")
}

fn event_id(session: usize, event: usize) -> EventId {
    EventId::new(format!("event:{session:02}:{event:03}")).expect("valid event id")
}

fn artifact_hash(index: usize) -> ArtifactHash {
    ArtifactHash::new(format!("sha256:artifact-{index:03}")).expect("valid artifact hash")
}

fn session_event(project_id: &ProjectId, session: usize, event: usize) -> SessionEventRecord {
    SessionEventRecord {
        project_id: project_id.clone(),
        session_id: session_id(session),
        agent_id: agent_id(session),
        turn_id: turn_id(session, event),
        event_id: event_id(session, event),
        event_kind: "storage.db_pressure.session_event".to_owned(),
        causality_parent_event_id: (event > 0).then(|| event_id(session, event - 1)),
        body: format!("session={session};event={event}").into_bytes(),
        created_at_unix_ms: 1_800_000_000_000 + ((session * 1_000 + event) as i64),
    }
}

fn artifact(project_id: &ProjectId, index: usize) -> ArtifactRecord {
    let body = format!("artifact-body-{index:03}").into_bytes();
    ArtifactRecord {
        project_id: project_id.clone(),
        artifact_hash: artifact_hash(index),
        artifact_kind: "storage.db_pressure.artifact".to_owned(),
        producer_session_id: session_id(index % 3),
        producer_agent_id: agent_id(index % 3),
        producer_event_id: event_id(index % 3, index),
        media_type: "application/octet-stream".to_owned(),
        size_bytes: body.len() as u64,
        body,
        created_at_unix_ms: 1_800_000_100_000 + index as i64,
    }
}

fn visibility_receipt(project_id: &ProjectId, index: usize) -> VisibilityReceipt {
    VisibilityReceipt {
        project_id: project_id.clone(),
        receipt_id: format!("db-pressure-receipt-{index:03}"),
        receipt_kind: "storage.db_pressure.visibility".to_owned(),
        body: format!("receipt={index}").into_bytes(),
        created_at_unix_ms: 1_800_000_200_000 + index as i64,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_persists_multi_session_events_artifacts_and_visibility_across_reopen()
-> StorageResult<()> {
    let scenario = db_pressure_scenario();
    let project_id = project_id();
    let tempdir = tempdir().expect("tempdir should be available");
    let db_path = tempdir.path().join("agent-storage.turso");
    let config = TursoAgentStorageConfig {
        path: db_path.clone(),
        mvcc: TursoMvccMode::Required,
    };
    let storage = TursoAgentStorage::open_local(config).await?;

    for session in 0..scenario.sessions {
        for event in 0..scenario.events_per_session {
            storage
                .append_session_event(session_event(&project_id, session, event))
                .await?;
        }
    }

    for index in 0..scenario.artifacts {
        let record = artifact(&project_id, index);
        let outcome = storage.put_artifact(record.clone()).await?;
        assert!(outcome.inserted, "first artifact write should insert");
        assert_eq!(outcome.artifact, record);
    }

    for index in 0..scenario.visibility_receipts {
        storage
            .record_visibility(visibility_receipt(&project_id, index))
            .await?;
    }

    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: db_path,
        mvcc: TursoMvccMode::Required,
    })
    .await?;

    for session in 0..scenario.sessions {
        let events = reopened
            .list_session_events(&project_id, &session_id(session))
            .await?;
        assert_eq!(events.len(), scenario.events_per_session);
        for (event, record) in events.iter().enumerate() {
            assert_eq!(record.event_id, event_id(session, event));
            assert_eq!(
                record.causality_parent_event_id,
                (event > 0).then(|| event_id(session, event - 1))
            );
        }
    }

    for index in 0..scenario.artifacts {
        let stored = reopened
            .get_artifact(&project_id, &artifact_hash(index))
            .await?
            .expect("artifact should persist across reopen");
        assert_eq!(stored, artifact(&project_id, index));
    }

    let receipts = reopened.list_visibility(&project_id).await?;
    assert_eq!(receipts.len(), scenario.visibility_receipts);
    for index in 0..scenario.visibility_receipts {
        assert!(
            receipts
                .iter()
                .any(|receipt| receipt.receipt_id == format!("db-pressure-receipt-{index:03}")),
            "missing visibility receipt {index}"
        );
    }

    Ok(())
}
