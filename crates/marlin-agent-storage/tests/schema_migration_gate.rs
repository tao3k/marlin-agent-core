#![cfg(feature = "turso")]

use marlin_agent_storage::{
    STORAGE_SCHEMA_V1_MIGRATION_ID, TursoAgentStorage, TursoAgentStorageConfig, TursoMvccMode,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn turso_bootstrap_records_schema_version_and_tables() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let database_path = tempdir.path().join("agent-storage.db");

    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: database_path.clone(),
        mvcc: TursoMvccMode::DisabledForCompatibility,
    })
    .await
    .expect("open turso storage");

    let snapshot = storage.schema_snapshot().await.expect("schema snapshot");
    assert!(snapshot.has_migration(STORAGE_SCHEMA_V1_MIGRATION_ID));
    for table_name in [
        "storage_schema_migrations",
        "session_events",
        "artifacts",
        "artifact_pointers",
        "visibility_receipts",
        "memory_proposals",
        "topology_edges",
    ] {
        assert!(snapshot.has_table(table_name), "missing table {table_name}");
    }

    drop(storage);

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: database_path,
        mvcc: TursoMvccMode::DisabledForCompatibility,
    })
    .await
    .expect("reopen turso storage");
    let reopened_snapshot = reopened
        .schema_snapshot()
        .await
        .expect("reopened schema snapshot");
    let migration_count = reopened_snapshot
        .migrations
        .iter()
        .filter(|migration| migration.migration_id == STORAGE_SCHEMA_V1_MIGRATION_ID)
        .count();
    assert_eq!(migration_count, 1);
}
