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
async fn rfcdb_turso_pointer_promotion_reopen_persistence() -> StorageResult<()> {
    let project_id = project_id()?;
    let tempdir = tempdir().expect("tempdir should be available");
    let db_path = tempdir.path().join("crash-reopen.turso");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig { path: db_path.clone(), optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental, batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent })
    .await?;
    assert_eq!(
        storage.optimization_receipt(),
        marlin_agent_storage::TursoOptimizationReceipt {
            async_io: marlin_agent_storage::TursoAsyncIoMode::Enabled,
            mvcc: TursoMvccMode::Required,
            mvcc_checkpoint: marlin_agent_storage::TursoMvccCheckpointMode::PassiveExperimental,
            connection_lanes: 4,
            statement_cache:
                marlin_agent_storage::TursoStatementCacheMode::PreparedCachedPerConnection,
        }
    );

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
    assert_eq!(
        storage
            .get_artifact_pointer(&project_id, &pointer_key()?)
            .await?
            .expect("initial artifact pointer should exist")
            .target_artifact_hash,
        artifact_hash(1)?
    );

    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig { path: db_path.clone(), optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental, batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent })
    .await?;
    assert_eq!(
        reopened.optimization_receipt(),
        marlin_agent_storage::TursoOptimizationReceipt {
            async_io: marlin_agent_storage::TursoAsyncIoMode::Enabled,
            mvcc: TursoMvccMode::Required,
            mvcc_checkpoint: marlin_agent_storage::TursoMvccCheckpointMode::PassiveExperimental,
            connection_lanes: 4,
            statement_cache:
                marlin_agent_storage::TursoStatementCacheMode::PreparedCachedPerConnection,
        }
    );
    let operational = reopened.operational_receipt();
    assert_eq!(operational.optimization, reopened.optimization_receipt());
    assert_eq!(
        operational.sync_feature,
        if cfg!(feature = "turso-sync") {
            marlin_agent_storage::TursoSyncFeatureStatus::Compiled
        } else {
            marlin_agent_storage::TursoSyncFeatureStatus::NotCompiled
        }
    );
    assert_eq!(
        operational.checkpoint_mode,
        marlin_agent_storage::TursoMvccCheckpointMode::PassiveExperimental
    );
    assert_eq!(
        operational.checkpoint_stats,
        marlin_agent_storage::TursoSdkTelemetryStatus::NotExposedBySdk
    );
    assert_eq!(
        operational.database_stats,
        marlin_agent_storage::TursoSdkTelemetryStatus::NotExposedBySdk
    );
    storage_put_second_artifact_and_assert_conflict(&reopened, &project_id).await?;
    assert_eq!(
        reopened
            .get_artifact_pointer(&project_id, &pointer_key()?)
            .await?
            .expect("conflict must preserve current pointer")
            .target_artifact_hash,
        artifact_hash(1)?
    );

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

    let transaction_receipts = reopened
        .list_visibility_page(marlin_agent_storage::VisibilityPageRequest::new(
            project_id.clone(),
            marlin_agent_storage::StoragePageLimit::MAXIMUM,
        ))
        .await?
        .items;
    let committed_body = marlin_agent_storage::TursoTransactionReceipt {
        operation: marlin_agent_storage::TursoTransactionOperation::ArtifactPointerCompareAndSwap,
        status: marlin_agent_storage::TursoTransactionStatus::Committed,
        retry_count: 0,
        rows_affected: 1,
    }
    .visibility_body();
    let conflict_body = marlin_agent_storage::TursoTransactionReceipt {
        operation: marlin_agent_storage::TursoTransactionOperation::ArtifactPointerCompareAndSwap,
        status: marlin_agent_storage::TursoTransactionStatus::Conflict,
        retry_count: 0,
        rows_affected: 0,
    }
    .visibility_body();
    assert_eq!(
        transaction_receipts
            .iter()
            .filter(|receipt| receipt.receipt_kind == "storage.turso.transaction")
            .map(|receipt| receipt.body.clone())
            .collect::<Vec<_>>(),
        vec![committed_body.clone(), conflict_body, committed_body],
        "transaction receipts must preserve promotion order"
    );

    drop(reopened);
    let final_reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: db_path,
        optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    })
    .await?;
    assert_eq!(
        final_reopened
            .get_artifact_pointer(&project_id, &pointer_key()?)
            .await?
            .expect("promoted artifact pointer should survive second reopen")
            .target_artifact_hash,
        artifact_hash(2)?
    );

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
