#![cfg(feature = "turso-sync")]

use marlin_agent_storage::{
    AgentStorage, ProjectId, StorageResult, TursoAgentStorage, TursoAgentStorageConfig,
    TursoMvccMode, VisibilityReceipt,
};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_sync_feature_combination_keeps_local_storage_receiptable() -> StorageResult<()> {
    let project_id = ProjectId::new("project:rfcdb-turso-sync")?;
    let tempdir = tempdir().expect("tempdir should be available");
    let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: tempdir.path().join("sync-feature.turso"),
        mvcc: TursoMvccMode::Required,
    })
    .await?;

    storage
        .record_visibility(VisibilityReceipt {
            project_id: project_id.clone(),
            receipt_id: "turso-sync-feature-compiled".to_owned(),
            receipt_kind: "rfcdb.turso-sync.feature".to_owned(),
            body: b"turso-sync feature compiled with local storage receipts".to_vec(),
            created_at_unix_ms: 1_800_005_000_000,
        })
        .await?;

    let receipts = storage.list_visibility(&project_id).await?;
    assert!(
        receipts
            .iter()
            .any(|receipt| receipt.receipt_kind == "rfcdb.turso-sync.feature")
    );

    Ok(())
}
