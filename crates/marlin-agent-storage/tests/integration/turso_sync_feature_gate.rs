#![cfg(feature = "turso-sync")]

use marlin_agent_storage::{
    AgentStorage, ProjectId, StorageResult, TursoAgentStorage, TursoAgentStorageConfig,
    TursoMvccMode, VisibilityReceipt,
};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rfcdb_turso_sync_checkpoint_stats_receipt_gate() -> StorageResult<()> {
    let project_id = ProjectId::new("project:rfcdb-turso-sync")?;
    let tempdir = tempdir().expect("tempdir should be available");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig { path: tempdir.path().join("sync-feature.turso"), optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental, batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent })
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
    assert_eq!(
        storage.operational_receipt(),
        marlin_agent_storage::TursoOperationalReceipt {
            optimization: storage.optimization_receipt(),
            sync_feature: marlin_agent_storage::TursoSyncFeatureStatus::Compiled,
            checkpoint_mode: marlin_agent_storage::TursoMvccCheckpointMode::PassiveExperimental,
            checkpoint_stats: marlin_agent_storage::TursoSdkTelemetryStatus::NotExposedBySdk,
            database_stats: marlin_agent_storage::TursoSdkTelemetryStatus::NotExposedBySdk,
        }
    );

    storage
        .record_visibility(VisibilityReceipt {
            project_id: project_id.clone(),
            receipt_id: "turso-sync-feature-compiled".to_owned(),
            receipt_kind: "rfcdb.turso-sync.feature".to_owned(),
            body: b"turso-sync feature compiled with local storage receipts".to_vec(),
            created_at_unix_ms: 1_800_005_000_000,
        })
        .await?;

    let receipts = storage
        .list_visibility_page(marlin_agent_storage::VisibilityPageRequest::new(
            project_id.clone(),
            marlin_agent_storage::StoragePageLimit::MAXIMUM,
        ))
        .await?
        .items;
    assert!(
        receipts
            .iter()
            .any(|receipt| receipt.receipt_kind == "rfcdb.turso-sync.feature")
    );

    Ok(())
}

#[test]
fn turso_sync_configuration_is_typed_and_redacts_credentials() -> StorageResult<()> {
    assert!(matches!(
        marlin_agent_storage::TursoSyncRemoteUrl::new("file:local.db"),
        Err(marlin_agent_storage::StorageError::InvalidSyncConfiguration { .. })
    ));
    assert!(matches!(
        marlin_agent_storage::TursoSyncAuthToken::new(""),
        Err(marlin_agent_storage::StorageError::InvalidSyncConfiguration { .. })
    ));
    let token = marlin_agent_storage::TursoSyncAuthToken::new("secret-test-token")?;
    assert_eq!(format!("{token:?}"), "TursoSyncAuthToken([REDACTED])");
    assert!(!format!("{token:?}").contains("secret-test-token"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_storage_rejects_sync_operations_without_sync_authority() -> StorageResult<()> {
    let tempdir = tempdir().expect("tempdir should be available");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: tempdir.path().join("local-only.turso"),
        optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    })
    .await?;
    assert_eq!(
        storage.sync_push().await,
        Err(marlin_agent_storage::StorageError::SyncNotConfigured)
    );
    assert_eq!(
        storage.sync_pull().await,
        Err(marlin_agent_storage::StorageError::SyncNotConfigured)
    );
    assert_eq!(
        storage.sync_checkpoint().await,
        Err(marlin_agent_storage::StorageError::SyncNotConfigured)
    );
    assert_eq!(
        storage.sync_stats().await,
        Err(marlin_agent_storage::StorageError::SyncNotConfigured)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "manual Turso 0.7 remote push/pull/checkpoint/stats scenario; requires isolated TURSO_TEST_* database"]
async fn turso_070_remote_sync_operational_scenario() -> StorageResult<()> {
    let remote_url = std::env::var("TURSO_TEST_DATABASE_URL").map_err(|_| {
        marlin_agent_storage::StorageError::InvalidSyncConfiguration {
            reason: "TURSO_TEST_DATABASE_URL is required",
        }
    })?;
    let auth_token = std::env::var("TURSO_TEST_AUTH_TOKEN").map_err(|_| {
        marlin_agent_storage::StorageError::InvalidSyncConfiguration {
            reason: "TURSO_TEST_AUTH_TOKEN is required",
        }
    })?;
    let tempdir = tempdir().expect("tempdir should be available");
    let storage = TursoAgentStorage::open_sync(marlin_agent_storage::TursoSyncAgentStorageConfig {
        path: tempdir.path().join("sync-client.turso"),
        remote_url: marlin_agent_storage::TursoSyncRemoteUrl::new(remote_url)?,
        auth_token: marlin_agent_storage::TursoSyncAuthToken::new(auth_token)?,
        bootstrap_if_empty: true,
    })
    .await?;
    storage.sync_push().await?;
    let _changed = storage.sync_pull().await?;
    storage.sync_checkpoint().await?;
    let stats = storage.sync_stats().await?;
    assert!(stats.main_wal_size < u64::MAX);
    Ok(())
}
