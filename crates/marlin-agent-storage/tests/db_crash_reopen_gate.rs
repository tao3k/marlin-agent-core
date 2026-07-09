#![cfg(feature = "turso")]

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerUpdate, ArtifactRecord,
    EventId, ProjectId, SessionId, StorageError, StorageResult, TursoAgentStorage,
    TursoAgentStorageConfig, TursoMvccMode,
};
use tempfile::tempdir;

fn project_id() -> StorageResult<ProjectId> {
    ProjectId::new("project:rfcdb-crash-reopen")
}

fn session_id() -> StorageResult<SessionId> {
    SessionId::new("session:crash-reopen")
}

fn agent_id() -> StorageResult<AgentId> {
    AgentId::new("agent:crash-reopen")
}

fn event_id(index: usize) -> StorageResult<EventId> {
    EventId::new(format!("event:crash-reopen:{index}"))
}

fn artifact_hash(index: usize) -> StorageResult<ArtifactHash> {
    ArtifactHash::new(format!("sha256:crash-reopen-artifact-{index}"))
}

fn pointer_key() -> StorageResult<ArtifactPointerKey> {
    ArtifactPointerKey::new("pointer:crash-reopen:latest")
}

fn artifact(project_id: &ProjectId, index: usize) -> StorageResult<ArtifactRecord> {
    let body = format!("crash-reopen-artifact-{index}").into_bytes();
    Ok(ArtifactRecord {
        project_id: project_id.clone(),
        artifact_hash: artifact_hash(index)?,
        artifact_kind: "rfcdb.crash-reopen.artifact".to_owned(),
        producer_session_id: session_id()?,
        producer_agent_id: agent_id()?,
        producer_event_id: event_id(index)?,
        media_type: "application/octet-stream".to_owned(),
        size_bytes: body.len() as u64,
        body,
        created_at_unix_ms: 1_800_003_000_000 + index as i64,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_reopen_preserves_pointer_cas_conflict_state() -> StorageResult<()> {
    let project_id = project_id()?;
    let tempdir = tempdir().expect("tempdir should be available");
    let db_path = tempdir.path().join("crash-reopen.turso");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: db_path.clone(),
        mvcc: TursoMvccMode::Required,
    })
    .await?;

    storage.put_artifact(artifact(&project_id, 1)?).await?;
    storage
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: project_id.clone(),
            pointer_key: pointer_key()?,
            expected_artifact_hash: None,
            new_artifact_hash: artifact_hash(1)?,
            updated_by_session_id: session_id()?,
            updated_by_agent_id: agent_id()?,
            updated_by_event_id: event_id(10)?,
            updated_at_unix_ms: 1_800_003_000_100,
        })
        .await?;

    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: db_path,
        mvcc: TursoMvccMode::Required,
    })
    .await?;
    storage_put_second_artifact_and_assert_conflict(&reopened, &project_id).await?;

    let committed = reopened
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: project_id.clone(),
            pointer_key: pointer_key()?,
            expected_artifact_hash: Some(artifact_hash(1)?),
            new_artifact_hash: artifact_hash(2)?,
            updated_by_session_id: session_id()?,
            updated_by_agent_id: agent_id()?,
            updated_by_event_id: event_id(12)?,
            updated_at_unix_ms: 1_800_003_000_300,
        })
        .await?;
    assert_eq!(committed.target_artifact_hash, artifact_hash(2)?);

    Ok(())
}

async fn storage_put_second_artifact_and_assert_conflict(
    storage: &TursoAgentStorage,
    project_id: &ProjectId,
) -> StorageResult<()> {
    storage.put_artifact(artifact(project_id, 2)?).await?;
    let conflict = storage
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: project_id.clone(),
            pointer_key: pointer_key()?,
            expected_artifact_hash: None,
            new_artifact_hash: artifact_hash(2)?,
            updated_by_session_id: session_id()?,
            updated_by_agent_id: agent_id()?,
            updated_by_event_id: event_id(11)?,
            updated_at_unix_ms: 1_800_003_000_200,
        })
        .await;

    match conflict {
        Err(StorageError::ArtifactPointerConflict { actual, .. }) => {
            assert_eq!(actual, Some(artifact_hash(1)?));
        }
        other => panic!("expected artifact pointer conflict after reopen, got {other:?}"),
    }
    Ok(())
}
