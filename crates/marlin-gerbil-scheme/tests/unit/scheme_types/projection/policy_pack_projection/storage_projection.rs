use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactRecord, EventId, ProjectId, SessionEventRecord,
    SessionId, StorageResult, TurnId, TursoAgentStorage, TursoAgentStorageConfig,
    VisibilityReceipt,
};
use tempfile::tempdir;

use super::{
    decode_gerbil_policy_mixin_stack_compiler_receipt,
    policy_combination_mixin_stack_compiler_fixture, policy_mixin_stack_compiler_envelope,
    policy_mixin_stack_compiler_registry,
};

fn project_id() -> ProjectId {
    ProjectId::new("project:ifc-real-policy-pack-projection").expect("valid project id")
}

fn session_id() -> SessionId {
    SessionId::new("session:ifc-real-policy-pack-projection").expect("valid session id")
}

fn agent_id() -> AgentId {
    AgentId::new("agent:ifc-real-policy-pack-projector").expect("valid agent id")
}

fn turn_id() -> TurnId {
    TurnId::new("turn:ifc-real-policy-pack-projection").expect("valid turn id")
}

fn event_id() -> EventId {
    EventId::new("event:ifc-real-policy-pack-projection").expect("valid event id")
}

fn artifact_hash() -> ArtifactHash {
    ArtifactHash::new("sha256:ifc-real-policy-pack-projection-001").expect("valid artifact hash")
}

#[tokio::test]
async fn real_policy_pack_projection_receipt_persists_as_storage_artifact_and_visibility()
-> StorageResult<()> {
    let registry = policy_mixin_stack_compiler_registry();
    let envelope =
        policy_mixin_stack_compiler_envelope(policy_combination_mixin_stack_compiler_fixture());
    let receipt = decode_gerbil_policy_mixin_stack_compiler_receipt(&registry, &envelope)
        .expect("real policy-pack mixin-stack receipt decodes");
    assert!(receipt.has_current_schema());
    assert_eq!(receipt.mixin_count, receipt.mixin_definitions.len());
    assert_eq!(
        receipt.slot_merge_receipts.len(),
        receipt.slot_merge_laws.len()
    );

    let body =
        serde_json::to_vec(&receipt).expect("typed IFC receipt serializes as Rust-owned artifact");
    let tempdir = tempdir().expect("tempdir should be available");
    let db_path = tempdir.path().join("ifc-real-policy-pack-projection.turso");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: db_path.clone(),
        optimization_profile:
            marlin_agent_storage::TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Immediate,
    })
    .await?;

    let project_id = project_id();
    let session_id = session_id();
    let agent_id = agent_id();
    let event_id = event_id();
    let artifact_hash = artifact_hash();

    storage
        .append_session_event(SessionEventRecord {
            project_id: project_id.clone(),
            session_id: session_id.clone(),
            agent_id: agent_id.clone(),
            turn_id: turn_id(),
            event_id: event_id.clone(),
            event_kind: "ifc.policy_pack.mixin_stack.projected".to_owned(),
            causality_parent_event_id: None,
            body: format!(
                "profile={};mixins={};slot_merges={}",
                receipt.profile_id.as_str(),
                receipt.mixin_count,
                receipt.slot_merge_receipts.len()
            )
            .into_bytes(),
            created_at_unix_ms: 1_800_002_000_000,
        })
        .await?;

    let artifact = ArtifactRecord {
        project_id: project_id.clone(),
        artifact_hash: artifact_hash.clone(),
        artifact_kind: "ifc.policy_pack.mixin_stack_projection_receipt".to_owned(),
        producer_session_id: session_id.clone(),
        producer_agent_id: agent_id.clone(),
        producer_event_id: event_id.clone(),
        media_type: "application/vnd.marlin.ifc.mixin-stack-receipt+json".to_owned(),
        size_bytes: body.len() as u64,
        body: body.clone(),
        created_at_unix_ms: 1_800_002_000_001,
    };
    let outcome = storage.put_artifact(artifact.clone()).await?;
    assert!(outcome.inserted);
    assert_eq!(outcome.artifact, artifact);

    storage
        .record_visibility(VisibilityReceipt {
            project_id: project_id.clone(),
            receipt_id: "ifc-real-policy-pack-projection-storage-001".to_owned(),
            receipt_kind: "ifc.policy_pack.mixin_stack_projection.stored".to_owned(),
            body: format!(
                "profile={};artifact={};native_abi=scheme-types-to-rust-types",
                receipt.profile_id.as_str(),
                artifact_hash.as_str()
            )
            .into_bytes(),
            created_at_unix_ms: 1_800_002_000_002,
        })
        .await?;

    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: db_path,
        optimization_profile:
            marlin_agent_storage::TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Immediate,
    })
    .await?;

    let events = reopened
        .list_session_events_page(marlin_agent_storage::SessionEventPageRequest::new(
            project_id.clone(),
            session_id.clone(),
            marlin_agent_storage::StoragePageLimit::MAXIMUM,
        ))
        .await?;
    assert_eq!(events.items.len(), 1);
    assert_eq!(
        events.items[0].event_kind,
        "ifc.policy_pack.mixin_stack.projected"
    );

    let stored_artifact = reopened
        .get_artifact(&project_id, &artifact_hash)
        .await?
        .expect("real IFC projection artifact should persist");
    assert_eq!(
        stored_artifact.artifact_kind,
        "ifc.policy_pack.mixin_stack_projection_receipt"
    );
    assert_eq!(stored_artifact.body, body);
    assert_eq!(stored_artifact.producer_event_id, event_id);

    let visibility = reopened
        .list_visibility_page(marlin_agent_storage::VisibilityPageRequest::new(
            project_id.clone(),
            marlin_agent_storage::StoragePageLimit::MAXIMUM,
        ))
        .await?;
    assert!(visibility.items.iter().any(|receipt| {
        receipt.receipt_kind == "ifc.policy_pack.mixin_stack_projection.stored"
            && String::from_utf8_lossy(&receipt.body)
                .contains("native_abi=scheme-types-to-rust-types")
    }));

    Ok(())
}
