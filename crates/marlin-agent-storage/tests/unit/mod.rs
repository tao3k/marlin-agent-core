use std::sync::Arc;

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerUpdate, ArtifactRecord,
    EventId, InMemoryAgentStorage, ProjectId, SessionEventRecord, SessionId, StorageError, TurnId,
    VisibilityReceipt,
};
#[cfg(feature = "turso")]
use marlin_agent_storage::{TursoAgentStorage, TursoAgentStorageConfig, TursoMvccMode};
use serde::Deserialize;

fn id(value: &str) -> String {
    value.to_string()
}

fn project() -> ProjectId {
    ProjectId::new("project").unwrap()
}

fn session(value: &str) -> SessionId {
    SessionId::new(value).unwrap()
}

fn agent(value: &str) -> AgentId {
    AgentId::new(value).unwrap()
}

fn turn(value: &str) -> TurnId {
    TurnId::new(value).unwrap()
}

fn event(value: &str) -> EventId {
    EventId::new(value).unwrap()
}

fn artifact_hash(value: &str) -> ArtifactHash {
    ArtifactHash::new(value).unwrap()
}

fn pointer_key(value: &str) -> ArtifactPointerKey {
    ArtifactPointerKey::new(value).unwrap()
}

fn session_event(
    session_id: SessionId,
    agent_id: AgentId,
    turn_id: TurnId,
    event_id: EventId,
) -> SessionEventRecord {
    SessionEventRecord {
        project_id: project(),
        session_id,
        agent_id,
        turn_id,
        event_id,
        event_kind: id("tool.output"),
        causality_parent_event_id: None,
        body: b"event".to_vec(),
        created_at_unix_ms: 1,
    }
}

fn artifact(hash: ArtifactHash, event_id: EventId, body: &[u8]) -> ArtifactRecord {
    ArtifactRecord {
        project_id: project(),
        artifact_hash: hash,
        artifact_kind: id("test.receipt"),
        producer_session_id: session("session-a"),
        producer_agent_id: agent("agent-a"),
        producer_event_id: event_id,
        media_type: id("application/octet-stream"),
        size_bytes: body.len() as u64,
        body: body.to_vec(),
        created_at_unix_ms: 1,
    }
}

#[derive(Debug, Deserialize)]
struct StoragePressureScenarioDocument {
    storage_pressure: StoragePressureScenario,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct StoragePressureScenario {
    sessions: usize,
    agents: usize,
    events: usize,
    pointer_contention: usize,
    visibility_receipts: bool,
}

fn storage_pressure_scenario() -> StoragePressureScenario {
    let source = include_str!("../db_pressure/scenarios/storage_pressure.toml");
    let document: StoragePressureScenarioDocument = toml::from_str(source).unwrap();
    let scenario = document.storage_pressure;
    assert!(scenario.sessions > 0);
    assert!(scenario.agents > 0);
    assert!(scenario.events >= scenario.sessions);
    assert!(scenario.pointer_contention > 1);
    scenario
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stores_append_only_session_events_from_multiple_agents() {
    let storage = Arc::new(InMemoryAgentStorage::new());
    let mut handles = Vec::new();

    for index in 0..64 {
        let storage = Arc::clone(&storage);
        handles.push(tokio::spawn(async move {
            let session_id = session(&format!("session-{}", index % 4));
            let agent_id = agent(&format!("agent-{index}"));
            let turn_id = turn(&format!("turn-{index}"));
            let event_id = event(&format!("event-{index}"));
            storage
                .append_session_event(session_event(session_id, agent_id, turn_id, event_id))
                .await
                .unwrap();
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let events = storage
        .list_session_events(&project(), &session("session-0"))
        .await
        .unwrap();
    assert_eq!(events.len(), 16);
    assert!(
        events
            .iter()
            .all(|event| event.session_id == session("session-0"))
    );
}

#[tokio::test]
async fn preserves_immutable_artifacts_and_detects_pointer_conflicts() {
    let storage = InMemoryAgentStorage::new();
    let pointer = pointer_key("latest-test-receipt");
    let first_hash = artifact_hash("hash:first");
    let second_hash = artifact_hash("hash:second");
    let third_hash = artifact_hash("hash:third");

    assert!(
        storage
            .put_artifact(artifact(first_hash.clone(), event("event-1"), b"first"))
            .await
            .unwrap()
            .inserted
    );
    assert!(
        storage
            .put_artifact(artifact(second_hash.clone(), event("event-2"), b"second"))
            .await
            .unwrap()
            .inserted
    );
    assert!(
        storage
            .put_artifact(artifact(third_hash.clone(), event("event-3"), b"third"))
            .await
            .unwrap()
            .inserted
    );

    storage
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: project(),
            pointer_key: pointer.clone(),
            expected_artifact_hash: None,
            new_artifact_hash: first_hash.clone(),
            updated_by_session_id: session("session-a"),
            updated_by_agent_id: agent("agent-a"),
            updated_by_event_id: event("event-1"),
            updated_at_unix_ms: 1,
        })
        .await
        .unwrap();

    storage
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: project(),
            pointer_key: pointer.clone(),
            expected_artifact_hash: Some(first_hash.clone()),
            new_artifact_hash: second_hash.clone(),
            updated_by_session_id: session("session-b"),
            updated_by_agent_id: agent("agent-b"),
            updated_by_event_id: event("event-2"),
            updated_at_unix_ms: 2,
        })
        .await
        .unwrap();

    let conflict = storage
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: project(),
            pointer_key: pointer.clone(),
            expected_artifact_hash: Some(first_hash.clone()),
            new_artifact_hash: third_hash.clone(),
            updated_by_session_id: session("session-c"),
            updated_by_agent_id: agent("agent-c"),
            updated_by_event_id: event("event-3"),
            updated_at_unix_ms: 3,
        })
        .await
        .unwrap_err();

    assert!(matches!(
        conflict,
        StorageError::ArtifactPointerConflict { .. }
    ));

    let current = storage
        .get_artifact_pointer(&project(), &pointer)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.target_artifact_hash, second_hash);

    assert!(
        storage
            .get_artifact(&project(), &first_hash)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        storage
            .get_artifact(&project(), &third_hash)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn records_visibility_without_touching_artifact_pointers() {
    let storage = InMemoryAgentStorage::new();
    storage
        .record_visibility(VisibilityReceipt {
            project_id: project(),
            receipt_id: id("receipt-1"),
            receipt_kind: id("storage.transaction"),
            body: br#"{"status":"committed"}"#.to_vec(),
            created_at_unix_ms: 1,
        })
        .await
        .unwrap();

    let receipts = storage.list_visibility(&project()).await.unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].receipt_kind, "storage.transaction");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pressure_mixed_workload_confines_conflicts_to_artifact_pointers() {
    let storage = Arc::new(InMemoryAgentStorage::new());
    let mut handles = Vec::new();
    let scenario = storage_pressure_scenario();
    let workload_size = scenario.events;

    for index in 0..workload_size {
        let storage = Arc::clone(&storage);
        handles.push(tokio::spawn(async move {
            let event_id = event(&format!("event-{index}"));
            storage
                .append_session_event(session_event(
                    session(&format!("session-{}", index % scenario.sessions)),
                    agent(&format!("agent-{}", index % scenario.agents)),
                    turn(&format!("turn-{index}")),
                    event_id.clone(),
                ))
                .await
                .unwrap();

            storage
                .put_artifact(artifact(
                    artifact_hash(&format!("hash:{index}")),
                    event_id,
                    format!("artifact-body-{index}").as_bytes(),
                ))
                .await
                .unwrap();

            if scenario.visibility_receipts {
                storage
                    .record_visibility(VisibilityReceipt {
                        project_id: project(),
                        receipt_id: id(&format!("receipt-{index}")),
                        receipt_kind: id("storage.pressure.write"),
                        body: format!(r#"{{"index":{index}}}"#).into_bytes(),
                        created_at_unix_ms: index as i64,
                    })
                    .await
                    .unwrap();
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let pointer = pointer_key("latest-pressure-artifact");
    let initial_hash = artifact_hash("hash:0");
    storage
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: project(),
            pointer_key: pointer.clone(),
            expected_artifact_hash: None,
            new_artifact_hash: initial_hash.clone(),
            updated_by_session_id: session("session-0"),
            updated_by_agent_id: agent("agent-0"),
            updated_by_event_id: event("event-0"),
            updated_at_unix_ms: 0,
        })
        .await
        .unwrap();

    let mut pointer_handles = Vec::new();
    for index in 1..scenario.pointer_contention {
        let storage = Arc::clone(&storage);
        let pointer = pointer.clone();
        let initial_hash = initial_hash.clone();
        pointer_handles.push(tokio::spawn(async move {
            storage
                .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
                    project_id: project(),
                    pointer_key: pointer,
                    expected_artifact_hash: Some(initial_hash),
                    new_artifact_hash: artifact_hash(&format!("hash:{index}")),
                    updated_by_session_id: session(&format!("session-{}", index % 8)),
                    updated_by_agent_id: agent(&format!("agent-{}", index % 16)),
                    updated_by_event_id: event(&format!("event-{index}")),
                    updated_at_unix_ms: index as i64,
                })
                .await
        }));
    }

    let mut accepted_pointer_updates = 0;
    let mut pointer_conflicts = 0;
    for handle in pointer_handles {
        match handle.await.unwrap() {
            Ok(_) => accepted_pointer_updates += 1,
            Err(StorageError::ArtifactPointerConflict { .. }) => pointer_conflicts += 1,
            Err(error) => panic!("unexpected storage error: {error}"),
        }
    }

    assert_eq!(accepted_pointer_updates, 1);
    assert_eq!(pointer_conflicts, scenario.pointer_contention - 2);

    let receipts = storage.list_visibility(&project()).await.unwrap();
    assert_eq!(
        receipts.len(),
        if scenario.visibility_receipts {
            workload_size
        } else {
            0
        }
    );

    for index in 0..workload_size {
        assert!(
            storage
                .get_artifact(&project(), &artifact_hash(&format!("hash:{index}")))
                .await
                .unwrap()
                .is_some(),
            "artifact {index} should survive pointer contention"
        );
    }
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_local_backend_bootstraps_mvcc_schema() {
    let tempdir = tempfile::tempdir().unwrap();
    let path = tempdir.path().join("agent-storage.db");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: path.clone(),
        mvcc: TursoMvccMode::Required,
    })
    .await
    .unwrap();

    assert_eq!(storage.config().path, path);
    assert_eq!(storage.config().mvcc, TursoMvccMode::Required);
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_local_backend_persists_first_domain_slice_across_reopen() {
    let tempdir = tempfile::tempdir().unwrap();
    let path = tempdir.path().join("agent-storage.db");

    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: path.clone(),
        mvcc: TursoMvccMode::Required,
    })
    .await
    .unwrap();

    let pointer = pointer_key("latest-reopen-artifact");
    let first_hash = artifact_hash("hash:reopen-first");
    let second_hash = artifact_hash("hash:reopen-second");

    storage
        .append_session_event(session_event(
            session("session-reopen"),
            agent("agent-reopen"),
            turn("turn-reopen"),
            event("event-reopen"),
        ))
        .await
        .unwrap();
    storage
        .put_artifact(artifact(
            first_hash.clone(),
            event("event-reopen"),
            b"reopen-first",
        ))
        .await
        .unwrap();
    storage
        .put_artifact(artifact(
            second_hash.clone(),
            event("event-reopen-2"),
            b"reopen-second",
        ))
        .await
        .unwrap();
    storage
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: project(),
            pointer_key: pointer.clone(),
            expected_artifact_hash: None,
            new_artifact_hash: first_hash.clone(),
            updated_by_session_id: session("session-reopen"),
            updated_by_agent_id: agent("agent-reopen"),
            updated_by_event_id: event("event-reopen"),
            updated_at_unix_ms: 1,
        })
        .await
        .unwrap();
    storage
        .record_visibility(VisibilityReceipt {
            project_id: project(),
            receipt_id: id("receipt-reopen"),
            receipt_kind: id("storage.transaction"),
            body: br#"{"status":"committed"}"#.to_vec(),
            created_at_unix_ms: 1,
        })
        .await
        .unwrap();
    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path,
        mvcc: TursoMvccMode::Required,
    })
    .await
    .unwrap();

    let events = reopened
        .list_session_events(&project(), &session("session-reopen"))
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, event("event-reopen"));

    let first_artifact = reopened
        .get_artifact(&project(), &first_hash)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first_artifact.body, b"reopen-first");
    assert!(
        reopened
            .get_artifact(&project(), &second_hash)
            .await
            .unwrap()
            .is_some()
    );

    let pointer_record = reopened
        .get_artifact_pointer(&project(), &pointer)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pointer_record.target_artifact_hash, first_hash);

    let receipts = reopened.list_visibility(&project()).await.unwrap();
    assert_eq!(receipts.len(), 2);
    assert!(
        receipts
            .iter()
            .any(|receipt| receipt.receipt_id == "receipt-reopen")
    );
    assert!(
        receipts
            .iter()
            .any(|receipt| receipt.receipt_kind == "storage.turso.transaction")
    );
}

#[cfg(feature = "turso")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_local_backend_confines_concurrent_pointer_contention() {
    let tempdir = tempfile::tempdir().unwrap();
    let path = tempdir.path().join("agent-storage.db");
    let storage = Arc::new(
        TursoAgentStorage::open_local(TursoAgentStorageConfig {
            path,
            mvcc: TursoMvccMode::Required,
        })
        .await
        .unwrap(),
    );
    let scenario = storage_pressure_scenario();
    let workload_size = scenario.pointer_contention;
    let pointer = pointer_key("latest-contention-artifact");
    let initial_hash = artifact_hash("hash:contention-0");

    for index in 0..workload_size {
        storage
            .put_artifact(artifact(
                artifact_hash(&format!("hash:contention-{index}")),
                event(&format!("event-contention-{index}")),
                format!("contention-body-{index}").as_bytes(),
            ))
            .await
            .unwrap();
    }

    storage
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: project(),
            pointer_key: pointer.clone(),
            expected_artifact_hash: None,
            new_artifact_hash: initial_hash.clone(),
            updated_by_session_id: session("session-contention-0"),
            updated_by_agent_id: agent("agent-contention-0"),
            updated_by_event_id: event("event-contention-0"),
            updated_at_unix_ms: 0,
        })
        .await
        .unwrap();

    let mut handles = Vec::new();
    for index in 1..workload_size {
        let storage = Arc::clone(&storage);
        let pointer = pointer.clone();
        let initial_hash = initial_hash.clone();
        handles.push(tokio::spawn(async move {
            storage
                .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
                    project_id: project(),
                    pointer_key: pointer,
                    expected_artifact_hash: Some(initial_hash),
                    new_artifact_hash: artifact_hash(&format!("hash:contention-{index}")),
                    updated_by_session_id: session(&format!("session-contention-{index}")),
                    updated_by_agent_id: agent(&format!("agent-contention-{index}")),
                    updated_by_event_id: event(&format!("event-contention-{index}")),
                    updated_at_unix_ms: index as i64,
                })
                .await
        }));
    }

    let mut accepted_pointer_updates = 0;
    let mut pointer_conflicts = 0;
    for handle in handles {
        match handle.await.unwrap() {
            Ok(_) => accepted_pointer_updates += 1,
            Err(StorageError::ArtifactPointerConflict { .. }) => pointer_conflicts += 1,
            Err(error) => panic!("unexpected storage error: {error}"),
        }
    }

    assert_eq!(accepted_pointer_updates, 1);
    assert_eq!(pointer_conflicts, workload_size - 2);

    let receipts = storage.list_visibility(&project()).await.unwrap();
    assert_eq!(receipts.len(), workload_size);
    assert!(receipts.iter().any(|receipt| {
        receipt.receipt_kind == "storage.turso.transaction"
            && String::from_utf8_lossy(&receipt.body).contains("status=conflict")
    }));
    assert!(receipts.iter().any(|receipt| {
        receipt.receipt_kind == "storage.turso.transaction"
            && String::from_utf8_lossy(&receipt.body).contains("status=committed")
    }));

    for index in 0..workload_size {
        assert!(
            storage
                .get_artifact(
                    &project(),
                    &artifact_hash(&format!("hash:contention-{index}"))
                )
                .await
                .unwrap()
                .is_some(),
            "artifact {index} should survive pointer contention"
        );
    }
}
