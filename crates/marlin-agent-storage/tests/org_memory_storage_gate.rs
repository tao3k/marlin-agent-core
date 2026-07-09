#![cfg(feature = "turso")]

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactRecord, EventId, ProjectId, SessionId,
    StorageResult, TursoAgentStorage, TursoAgentStorageConfig, TursoMvccMode, VisibilityReceipt,
};
use marlin_org_memory::{
    PROJECT_MEMORY_CONTENT_ID_PROPERTY, PROJECT_MEMORY_ID_PROPERTY,
    PROJECT_MEMORY_RECALL_QUERY_PROPERTY, PROJECT_MEMORY_SESSION_ID_PROPERTY,
    SESSION_FACT_CONTEXT_PACK_ID_PROPERTY, SESSION_FACT_SESSION_ID_PROPERTY, TOPOLOGY_ID_PROPERTY,
    TOPOLOGY_SCOPE_PROPERTY,
};
use tempfile::tempdir;

fn project_id() -> StorageResult<ProjectId> {
    ProjectId::new("project:rfcdb-org-memory")
}

fn session_id() -> StorageResult<SessionId> {
    SessionId::new("session:org-memory")
}

fn agent_id() -> StorageResult<AgentId> {
    AgentId::new("agent:org-memory")
}

fn event_id() -> StorageResult<EventId> {
    EventId::new("event:org-memory")
}

fn memory_artifact_hash() -> StorageResult<ArtifactHash> {
    ArtifactHash::new("sha256:org-memory-contract-artifact")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_persists_org_memory_artifact_and_contract_visibility() -> StorageResult<()> {
    let project_id = project_id()?;
    let session_id = session_id()?;
    let org_body = format!(
        "* Project memory\n:PROPERTIES:\n:{PROJECT_MEMORY_ID_PROPERTY}: memory:rfcdb\n:{PROJECT_MEMORY_CONTENT_ID_PROPERTY}: content:rfcdb\n:{PROJECT_MEMORY_SESSION_ID_PROPERTY}: {}\n:{PROJECT_MEMORY_RECALL_QUERY_PROPERTY}: storage substrate\n:{SESSION_FACT_SESSION_ID_PROPERTY}: {}\n:{SESSION_FACT_CONTEXT_PACK_ID_PROPERTY}: context-pack:rfcdb\n:{TOPOLOGY_ID_PROPERTY}: topology:rfcdb\n:{TOPOLOGY_SCOPE_PROPERTY}: project\n:END:\n",
        session_id, session_id
    )
    .into_bytes();

    let tempdir = tempdir().expect("tempdir should be available");
    let db_path = tempdir.path().join("org-memory.turso");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: db_path.clone(),
        mvcc: TursoMvccMode::Required,
    })
    .await?;

    let artifact = ArtifactRecord {
        project_id: project_id.clone(),
        artifact_hash: memory_artifact_hash()?,
        artifact_kind: "rfcdb.org-memory.contract".to_owned(),
        producer_session_id: session_id.clone(),
        producer_agent_id: agent_id()?,
        producer_event_id: event_id()?,
        media_type: "text/org".to_owned(),
        size_bytes: org_body.len() as u64,
        body: org_body,
        created_at_unix_ms: 1_800_002_000_000,
    };
    storage.put_artifact(artifact.clone()).await?;
    storage
        .record_visibility(VisibilityReceipt {
            project_id: project_id.clone(),
            receipt_id: "org-memory-contract-properties".to_owned(),
            receipt_kind: "rfcdb.org-memory.contract.visibility".to_owned(),
            body: serde_json::to_vec(&serde_json::json!({
                "memory_id_property": PROJECT_MEMORY_ID_PROPERTY,
                "content_id_property": PROJECT_MEMORY_CONTENT_ID_PROPERTY,
                "recall_query_property": PROJECT_MEMORY_RECALL_QUERY_PROPERTY,
                "topology_id_property": TOPOLOGY_ID_PROPERTY,
            }))
            .expect("org memory contract visibility should serialize"),
            created_at_unix_ms: 1_800_002_000_100,
        })
        .await?;

    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: db_path,
        mvcc: TursoMvccMode::Required,
    })
    .await?;
    let stored = reopened
        .get_artifact(&project_id, &memory_artifact_hash()?)
        .await?
        .expect("org memory artifact should persist");
    let stored_body = String::from_utf8_lossy(&stored.body);
    let visibility = reopened.list_visibility(&project_id).await?;

    assert_eq!(stored, artifact);
    assert!(stored_body.contains(PROJECT_MEMORY_ID_PROPERTY));
    assert!(stored_body.contains(PROJECT_MEMORY_RECALL_QUERY_PROPERTY));
    assert!(stored_body.contains(TOPOLOGY_SCOPE_PROPERTY));
    assert!(visibility.iter().any(|receipt| {
        receipt.receipt_kind == "rfcdb.org-memory.contract.visibility"
            && String::from_utf8_lossy(&receipt.body).contains(PROJECT_MEMORY_ID_PROPERTY)
    }));

    Ok(())
}
