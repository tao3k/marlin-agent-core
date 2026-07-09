#![cfg(feature = "turso")]

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactRecord, EventId, ProjectId, SessionEventRecord,
    SessionId, StorageResult, TurnId, TursoAgentStorage, TursoAgentStorageConfig, TursoMvccMode,
    VisibilityReceipt,
};
use tempfile::tempdir;

fn project_id() -> ProjectId {
    ProjectId::new("project:ifc-policy-pack-storage").expect("valid project id")
}

fn session_id() -> SessionId {
    SessionId::new("session:ifc-policy-pack-projection").expect("valid session id")
}

fn agent_id() -> AgentId {
    AgentId::new("agent:ifc-policy-pack-projector").expect("valid agent id")
}

fn turn_id() -> TurnId {
    TurnId::new("turn:ifc-policy-pack-projection").expect("valid turn id")
}

fn event_id() -> EventId {
    EventId::new("event:ifc-policy-pack-projection").expect("valid event id")
}

fn artifact_hash() -> ArtifactHash {
    ArtifactHash::new("sha256:ifc-policy-pack-projection-001").expect("valid artifact hash")
}

fn projection_body() -> Vec<u8> {
    [
        "schema_id=marlin.ifc.policy-pack.projection-receipt.v1",
        "policy_pack=real-policy-basic",
        "compiler_receipt=policy-pack-compiler-receipt",
        "slot_merge_receipt=policy-pack-slot-merge-receipt",
        "failure_combination_receipt=policy-pack-failure-combination-receipt",
        "native_abi=json-free-scheme-types-to-rust-types",
    ]
    .join("\n")
    .into_bytes()
}

#[tokio::test]
async fn turso_persists_ifc_policy_pack_projection_as_artifact_and_visibility() -> StorageResult<()>
{
    let tempdir = tempdir().expect("tempdir should be available");
    let db_path = tempdir.path().join("ifc-policy-pack-storage.turso");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: db_path.clone(),
        mvcc: TursoMvccMode::Required,
    })
    .await?;

    let project_id = project_id();
    let session_id = session_id();
    let agent_id = agent_id();
    let turn_id = turn_id();
    let event_id = event_id();
    let artifact_hash = artifact_hash();
    let body = projection_body();

    storage
        .append_session_event(SessionEventRecord {
            project_id: project_id.clone(),
            session_id: session_id.clone(),
            agent_id: agent_id.clone(),
            turn_id,
            event_id: event_id.clone(),
            event_kind: "ifc.policy_pack.projection.compiled".to_owned(),
            causality_parent_event_id: None,
            body: b"policy-pack projection compiled without live LLM".to_vec(),
            created_at_unix_ms: 1_800_001_000_000,
        })
        .await?;

    let artifact = ArtifactRecord {
        project_id: project_id.clone(),
        artifact_hash: artifact_hash.clone(),
        artifact_kind: "ifc.policy_pack.projection_receipt".to_owned(),
        producer_session_id: session_id.clone(),
        producer_agent_id: agent_id.clone(),
        producer_event_id: event_id.clone(),
        media_type: "application/vnd.marlin.ifc.policy-pack-receipt+text".to_owned(),
        size_bytes: body.len() as u64,
        body: body.clone(),
        created_at_unix_ms: 1_800_001_000_001,
    };
    let artifact_outcome = storage.put_artifact(artifact.clone()).await?;
    assert!(artifact_outcome.inserted);
    assert_eq!(artifact_outcome.artifact, artifact);

    storage
        .record_visibility(VisibilityReceipt {
            project_id: project_id.clone(),
            receipt_id: "ifc-policy-pack-projection-storage-001".to_owned(),
            receipt_kind: "ifc.policy_pack.projection.stored".to_owned(),
            body: b"artifact=sha256:ifc-policy-pack-projection-001;schema=v1".to_vec(),
            created_at_unix_ms: 1_800_001_000_002,
        })
        .await?;

    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: db_path,
        mvcc: TursoMvccMode::Required,
    })
    .await?;

    let events = reopened
        .list_session_events(&project_id, &session_id)
        .await?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_kind, "ifc.policy_pack.projection.compiled");
    assert_eq!(events[0].agent_id, agent_id);
    assert_eq!(events[0].event_id, event_id);

    let stored_artifact = reopened
        .get_artifact(&project_id, &artifact_hash)
        .await?
        .expect("IFC projection artifact should persist");
    assert_eq!(
        stored_artifact.artifact_kind,
        "ifc.policy_pack.projection_receipt"
    );
    assert_eq!(stored_artifact.body, body);
    assert_eq!(stored_artifact.producer_session_id, session_id);

    let visibility = reopened.list_visibility(&project_id).await?;
    assert!(visibility.iter().any(|receipt| {
        receipt.receipt_kind == "ifc.policy_pack.projection.stored"
            && String::from_utf8_lossy(&receipt.body).contains("schema=v1")
    }));

    Ok(())
}
