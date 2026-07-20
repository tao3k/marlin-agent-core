#![cfg(feature = "turso")]

use marlin_agent_storage::{TursoAgentStorage, TursoAgentStorageConfig, TursoOptimizationProfile};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn turso_bootstrap_reports_development_baseline_and_tables() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let database_path = tempdir.path().join("agent-storage.db");

    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: database_path.clone(),
        optimization_profile:
            marlin_agent_storage::TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Immediate,
    })
    .await
    .expect("open turso storage");
    assert_eq!(
        storage.optimization_receipt(),
        TursoOptimizationProfile::AsyncIoOnlyCompatibility.receipt()
    );

    let snapshot = storage.schema_snapshot().await.expect("schema snapshot");
    assert_eq!(
        snapshot.lifecycle,
        marlin_agent_storage::StorageSchemaLifecycle::DevelopmentBaseline
    );
    for table_name in [
        "session_events",
        "artifacts",
        "artifact_pointers",
        "visibility_receipts",
        "memory_proposals",
        "memory_embeddings",
        "topology_edges",
    ] {
        assert!(snapshot.has_table(table_name), "missing table {table_name}");
    }

    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: database_path,
        optimization_profile:
            marlin_agent_storage::TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Immediate,
    })
    .await
    .expect("reopen turso storage");
    assert_eq!(
        reopened.optimization_receipt(),
        TursoOptimizationProfile::AsyncIoOnlyCompatibility.receipt()
    );
    let reopened_snapshot = reopened
        .schema_snapshot()
        .await
        .expect("reopened schema snapshot");
    assert_eq!(
        reopened_snapshot.lifecycle,
        marlin_agent_storage::StorageSchemaLifecycle::DevelopmentBaseline
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn concurrent_development_baseline_bootstrap_is_idempotent() {
    const OPEN_COUNT: usize = 6;

    let tempdir = tempfile::tempdir().expect("create tempdir");
    let database_path = tempdir.path().join("concurrent-bootstrap.db");
    let profile = TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental;
    let mut open_tasks = tokio::task::JoinSet::new();
    for _ in 0..OPEN_COUNT {
        let path = database_path.clone();
        open_tasks.spawn(async move {
            TursoAgentStorage::open_local(TursoAgentStorageConfig {
                path,
                optimization_profile: profile,
                batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
            })
            .await
        });
    }

    let mut storages = Vec::with_capacity(OPEN_COUNT);
    while let Some(result) = open_tasks.join_next().await {
        let storage = result
            .expect("concurrent bootstrap task should not panic")
            .expect("concurrent bootstrap should retry lock contention");
        assert_eq!(storage.optimization_receipt(), profile.receipt());
        storages.push(storage);
    }
    assert_eq!(storages.len(), OPEN_COUNT);

    for storage in storages {
        let snapshot = storage
            .schema_snapshot()
            .await
            .expect("concurrent bootstrap schema snapshot");
        assert_eq!(
            snapshot.lifecycle,
            marlin_agent_storage::StorageSchemaLifecycle::DevelopmentBaseline
        );
        assert!(!snapshot.has_table("storage_schema_migrations"));
    }
}
