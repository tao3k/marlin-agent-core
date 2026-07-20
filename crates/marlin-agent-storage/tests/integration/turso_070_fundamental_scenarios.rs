#![cfg(feature = "turso")]

use std::time::Instant;

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerUpdate, ArtifactRecord,
    EventId, MemoryKey, ProjectId, SessionEventRecord, SessionId, StorageError, StorageResult,
    TurnId, TursoAgentStorage, TursoAgentStorageConfig, TursoBatchTransactionMode,
    TursoMemoryEmbedding, TursoMemoryEmbeddingRecord, TursoMemorySearchLimit,
    TursoMemorySearchRequest, TursoOptimizationProfile, TursoStatementCacheMode,
};
use tempfile::tempdir;

fn project_id() -> ProjectId {
    ProjectId::new("project:turso-070-fundamental").expect("valid project id")
}

fn session_id(session: usize) -> SessionId {
    SessionId::new(format!("session:{session:02}")).expect("valid session id")
}

fn event_id(session: usize, event: usize) -> EventId {
    EventId::new(format!("event:{session:02}:{event:04}")).expect("valid event id")
}

fn session_event(session: usize, event: usize) -> SessionEventRecord {
    SessionEventRecord {
        project_id: project_id(),
        session_id: session_id(session),
        agent_id: AgentId::new(format!("agent:{session:02}")).expect("valid agent id"),
        turn_id: TurnId::new(format!("turn:{session:02}:{event:04}")).expect("valid turn id"),
        event_id: event_id(session, event),
        event_kind: "storage.turso070.batch".to_owned(),
        causality_parent_event_id: (event > 0).then(|| event_id(session, event - 1)),
        body: format!("session={session};event={event}").into_bytes(),
        created_at_unix_ms: 1_900_000_000_000 + ((session * 10_000 + event) as i64),
    }
}

fn config(path: std::path::PathBuf) -> TursoAgentStorageConfig {
    config_with_profile(
        path,
        TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
        TursoBatchTransactionMode::Immediate,
    )
}

fn config_with_profile(
    path: std::path::PathBuf,
    optimization_profile: TursoOptimizationProfile,
    batch_transaction_mode: TursoBatchTransactionMode,
) -> TursoAgentStorageConfig {
    TursoAgentStorageConfig {
        path,
        optimization_profile,
        batch_transaction_mode,
    }
}

const fn batch_transaction_mode_for_profile(
    profile: TursoOptimizationProfile,
) -> TursoBatchTransactionMode {
    match profile {
        TursoOptimizationProfile::AsyncIoOnlyCompatibility => TursoBatchTransactionMode::Immediate,
        TursoOptimizationProfile::AsyncIoWithMvcc
        | TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental => {
            TursoBatchTransactionMode::Concurrent
        }
    }
}

fn artifact(hash: &str, event: usize) -> ArtifactRecord {
    let body = format!("artifact={hash}").into_bytes();
    ArtifactRecord {
        project_id: project_id(),
        artifact_hash: ArtifactHash::new(hash).expect("valid artifact hash"),
        artifact_kind: "storage.turso070.cas-target".to_owned(),
        producer_session_id: session_id(0),
        producer_agent_id: AgentId::new("agent:00").expect("valid agent id"),
        producer_event_id: event_id(0, event),
        media_type: "application/octet-stream".to_owned(),
        size_bytes: body.len() as u64,
        body,
        created_at_unix_ms: 1_900_000_100_000 + event as i64,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_070_batch_transaction_is_atomic_and_survives_reopen() -> StorageResult<()> {
    let tempdir = tempdir().expect("temporary directory");
    let db_path = tempdir.path().join("batch-reopen.turso");
    let storage = TursoAgentStorage::open_local(config(db_path.clone())).await?;
    let records = (0..128).map(|event| session_event(0, event)).collect();

    let storage_contract: &dyn AgentStorage = &storage;
    let receipt = storage_contract
        .append_session_events_atomically(records)
        .await?;
    assert_eq!(receipt.item_count, 128);
    assert_eq!(receipt.rows_affected, 128);

    drop(storage);
    let reopened = TursoAgentStorage::open_local(config(db_path)).await?;
    let persisted = reopened
        .list_session_events_page(marlin_agent_storage::SessionEventPageRequest::new(
            project_id(),
            session_id(0),
            marlin_agent_storage::StoragePageLimit::MAXIMUM,
        ))
        .await?
        .items;
    assert_eq!(persisted.len(), 128);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_070_duplicate_rolls_back_the_entire_batch() -> StorageResult<()> {
    let tempdir = tempdir().expect("temporary directory");
    let storage =
        TursoAgentStorage::open_local(config(tempdir.path().join("batch-rollback.turso"))).await?;
    let duplicate = session_event(0, 1);

    let error = storage
        .append_session_events_batch(vec![session_event(0, 0), duplicate.clone(), duplicate])
        .await
        .expect_err("duplicate in one transaction must fail the batch");
    assert!(matches!(error, StorageError::DuplicateSessionEvent { .. }));
    let persisted = storage
        .list_session_events_page(marlin_agent_storage::SessionEventPageRequest::new(
            project_id(),
            session_id(0),
            marlin_agent_storage::StoragePageLimit::MAXIMUM,
        ))
        .await?
        .items;
    assert!(persisted.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn turso_070_mvcc_lanes_commit_non_overlapping_batches_concurrently() -> StorageResult<()> {
    let tempdir = tempdir().expect("temporary directory");
    let storage =
        TursoAgentStorage::open_local(config(tempdir.path().join("concurrent-batches.turso")))
            .await?;
    let optimization =
        TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental.receipt();
    assert_eq!(optimization.connection_lanes, 4);
    assert_eq!(
        optimization.statement_cache,
        TursoStatementCacheMode::PreparedCachedPerConnection
    );

    let mut tasks = Vec::new();
    for session in 0..8 {
        let storage = storage.clone();
        tasks.push(tokio::spawn(async move {
            let records = (0..32).map(|event| session_event(session, event)).collect();
            storage.append_session_events_batch(records).await
        }));
    }
    for task in tasks {
        let receipt = task.await.expect("batch task must join")?;
        assert_eq!(receipt.item_count, 32);
        assert_eq!(receipt.rows_affected, 32);
    }
    for session in 0..8 {
        let persisted = storage
            .list_session_events_page(marlin_agent_storage::SessionEventPageRequest::new(
                project_id(),
                session_id(session),
                marlin_agent_storage::StoragePageLimit::MAXIMUM,
            ))
            .await?
            .items;
        assert_eq!(persisted.len(), 32);
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_070_single_lane_cache_does_not_deadlock_nested_artifact_validation()
-> StorageResult<()> {
    let tempdir = tempdir().expect("temporary directory");
    let storage = TursoAgentStorage::open_local(config_with_profile(
        tempdir.path().join("compatibility-single-lane.turso"),
        TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        TursoBatchTransactionMode::Immediate,
    ))
    .await?;
    let record = artifact("sha256:single-lane", 0);
    storage.put_artifact(record.clone()).await?;

    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        storage.put_artifact(record),
    )
    .await
    .expect("single-lane duplicate validation must release its write lease")?;
    assert!(!outcome.inserted);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn turso_070_cas_storm_has_one_commit_and_typed_conflicts() -> StorageResult<()> {
    let tempdir = tempdir().expect("temporary directory");
    let storage =
        TursoAgentStorage::open_local(config(tempdir.path().join("cas-storm.turso"))).await?;
    let old_artifact = artifact("sha256:cas-old", 0);
    let new_artifact = artifact("sha256:cas-new", 1);
    storage.put_artifact(old_artifact.clone()).await?;
    storage.put_artifact(new_artifact.clone()).await?;
    let pointer_key = ArtifactPointerKey::new("pointer:cas-storm").expect("valid pointer key");
    storage
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: project_id(),
            pointer_key: pointer_key.clone(),
            expected_artifact_hash: None,
            new_artifact_hash: old_artifact.artifact_hash.clone(),
            updated_by_session_id: session_id(0),
            updated_by_agent_id: AgentId::new("agent:00").expect("valid agent id"),
            updated_by_event_id: event_id(0, 2),
            updated_at_unix_ms: 1_900_000_100_002,
        })
        .await?;

    let mut tasks = Vec::new();
    for contender in 0..16 {
        let storage = storage.clone();
        let pointer_key = pointer_key.clone();
        let expected = old_artifact.artifact_hash.clone();
        let target = new_artifact.artifact_hash.clone();
        tasks.push(tokio::spawn(async move {
            storage
                .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
                    project_id: project_id(),
                    pointer_key,
                    expected_artifact_hash: Some(expected),
                    new_artifact_hash: target,
                    updated_by_session_id: session_id(0),
                    updated_by_agent_id: AgentId::new("agent:00").expect("valid agent id"),
                    updated_by_event_id: event_id(0, 100 + contender),
                    updated_at_unix_ms: 1_900_000_200_000 + contender as i64,
                })
                .await
        }));
    }

    let mut committed = 0;
    let mut conflicts = 0;
    for task in tasks {
        match task.await.expect("CAS contender must join") {
            Ok(_) => committed += 1,
            Err(StorageError::ArtifactPointerConflict { .. }) => conflicts += 1,
            Err(error) => return Err(error),
        }
    }
    assert_eq!(committed, 1);
    assert_eq!(conflicts, 15);
    let pointer = storage
        .get_artifact_pointer(&project_id(), &pointer_key)
        .await?
        .expect("pointer must remain present");
    assert_eq!(pointer.target_artifact_hash, new_artifact.artifact_hash);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_070_large_blob_round_trips_through_cached_connection_lane() -> StorageResult<()> {
    let tempdir = tempdir().expect("temporary directory");
    let db_path = tempdir.path().join("large-blob.turso");
    let storage = TursoAgentStorage::open_local(config(db_path.clone())).await?;
    let mut record = artifact("sha256:large-blob", 0);
    record.body = vec![0xA5; 1024 * 1024];
    record.size_bytes = record.body.len() as u64;
    storage.put_artifact(record.clone()).await?;
    drop(storage);

    let reopened = TursoAgentStorage::open_local(config(db_path)).await?;
    let persisted = reopened
        .get_artifact(&record.project_id, &record.artifact_hash)
        .await?
        .expect("large artifact must persist");
    assert_eq!(persisted.size_bytes, 1024 * 1024);
    assert_eq!(persisted.body, record.body);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_070_vector8_memory_search_is_project_and_dimension_bounded() -> StorageResult<()> {
    let tempdir = tempdir().expect("temporary directory");
    let db_path = tempdir.path().join("vector-memory.turso");
    let storage = TursoAgentStorage::open_local(config(db_path.clone())).await?;
    for (key, values, timestamp) in [
        ("memory:exact", vec![1.0, 0.0, 0.0, 0.0], 10),
        ("memory:near", vec![0.9, 0.1, 0.0, 0.0], 20),
        ("memory:far", vec![0.0, 1.0, 0.0, 0.0], 30),
        ("memory:other-dimension", vec![1.0, 0.0, 0.0], 40),
    ] {
        storage
            .put_memory_embedding(TursoMemoryEmbeddingRecord {
                project_id: project_id(),
                memory_key: MemoryKey::new(key).expect("valid memory key"),
                embedding: TursoMemoryEmbedding::new(values)?,
                updated_at_unix_ms: 1_900_000_300_000 + timestamp,
            })
            .await?;
    }
    drop(storage);

    let reopened = TursoAgentStorage::open_local(config(db_path)).await?;
    let matches = reopened
        .search_memory_embeddings(TursoMemorySearchRequest {
            project_id: project_id(),
            embedding: TursoMemoryEmbedding::new(vec![1.0, 0.0, 0.0, 0.0])?,
            limit: TursoMemorySearchLimit::new(2)?,
        })
        .await?;
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].memory_key.as_str(), "memory:exact");
    assert_eq!(matches[1].memory_key.as_str(), "memory:near");
    assert!(matches[0].cosine_distance <= matches[1].cosine_distance);
    Ok(())
}

#[test]
fn turso_070_vector_protocol_rejects_ambiguous_inputs() {
    assert!(matches!(
        TursoMemoryEmbedding::new(Vec::new()),
        Err(StorageError::InvalidEmbedding { .. })
    ));
    assert!(matches!(
        TursoMemoryEmbedding::new(vec![f32::NAN]),
        Err(StorageError::InvalidEmbedding { .. })
    ));
    assert!(matches!(
        TursoMemorySearchLimit::new(0),
        Err(StorageError::InvalidMemorySearchLimit { limit: 0 })
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "manual Turso 0.7 profile by concurrent-batch scenario matrix"]
async fn turso_070_profile_by_valid_batch_mode_matrix() -> StorageResult<()> {
    const PROFILES: [TursoOptimizationProfile; 3] = [
        TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        TursoOptimizationProfile::AsyncIoWithMvcc,
        TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
    ];
    let tempdir = tempdir().expect("temporary directory");
    for profile in PROFILES {
        let mut samples_ms = Vec::new();
        for sample in 0..3 {
            let storage = TursoAgentStorage::open_local(config_with_profile(
                tempdir
                    .path()
                    .join(format!("batch-matrix-{profile:?}-{sample}.turso")),
                profile,
                batch_transaction_mode_for_profile(profile),
            ))
            .await?;
            let started = Instant::now();
            let mut tasks = Vec::new();
            for session in 0..8 {
                let storage = storage.clone();
                tasks.push(tokio::spawn(async move {
                    storage
                        .append_session_events_batch(
                            (0..64).map(|event| session_event(session, event)).collect(),
                        )
                        .await
                }));
            }
            for task in tasks {
                let receipt = task.await.expect("batch matrix task must join")?;
                assert_eq!(receipt.rows_affected, 64);
            }
            samples_ms.push(started.elapsed().as_millis());
        }
        samples_ms.sort_unstable();
        eprintln!(
            "turso_070_profile_batch_matrix profile={profile:?} optimization={:?} samples_ms={samples_ms:?} median_ms={}",
            profile.receipt(),
            samples_ms[1]
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "manual Turso 0.7 Immediate atomic-batch latency gate with row-at-a-time observation"]
async fn turso_070_atomic_immediate_batch_latency_gate() -> StorageResult<()> {
    const SAMPLE_COUNT: usize = 5;
    const BATCH_SAMPLE_BUDGET_MS: u128 = 310;
    const REQUIRED_FAST_BATCH_SAMPLES: usize = 3;
    let tempdir = tempdir().expect("temporary directory");
    let mut row_at_a_time_samples = Vec::with_capacity(SAMPLE_COUNT);
    let mut immediate_batch_samples = Vec::with_capacity(SAMPLE_COUNT);

    for sample in 0..SAMPLE_COUNT {
        let sequential = TursoAgentStorage::open_local(config(
            tempdir.path().join(format!("row-at-a-time-{sample}.turso")),
        ))
        .await?;
        let started = Instant::now();
        for event in 0..256 {
            sequential
                .append_session_event(session_event(sample, event))
                .await?;
        }
        row_at_a_time_samples.push(started.elapsed().as_millis());

        let batched = TursoAgentStorage::open_local(config(
            tempdir.path().join(format!("batch-{sample}.turso")),
        ))
        .await?;
        let started = Instant::now();
        let receipt = batched
            .append_session_events_batch(
                (0..256)
                    .map(|event| session_event(SAMPLE_COUNT + sample, event))
                    .collect(),
            )
            .await?;
        immediate_batch_samples.push(started.elapsed().as_millis());
        assert_eq!(receipt.rows_affected, 256);
        assert_eq!(
            receipt.transaction_mode,
            TursoBatchTransactionMode::Immediate
        );
    }
    row_at_a_time_samples.sort_unstable();
    immediate_batch_samples.sort_unstable();
    let row_at_a_time_median = row_at_a_time_samples[SAMPLE_COUNT / 2];
    let immediate_batch_median = immediate_batch_samples[SAMPLE_COUNT / 2];
    let fast_batch_samples = immediate_batch_samples
        .iter()
        .filter(|elapsed_ms| **elapsed_ms <= BATCH_SAMPLE_BUDGET_MS)
        .count();
    eprintln!(
        "turso_070_atomic_immediate_batch row_at_a_time_samples_ms={row_at_a_time_samples:?} immediate_batch_samples_ms={immediate_batch_samples:?} row_at_a_time_median_ms={row_at_a_time_median} immediate_batch_median_ms={immediate_batch_median} batch_sample_budget_ms={BATCH_SAMPLE_BUDGET_MS} fast_batch_samples={fast_batch_samples} required_fast_batch_samples={REQUIRED_FAST_BATCH_SAMPLES}"
    );
    assert!(
        fast_batch_samples >= REQUIRED_FAST_BATCH_SAMPLES,
        "expected at least {REQUIRED_FAST_BATCH_SAMPLES}/{SAMPLE_COUNT} Immediate atomic batches within {BATCH_SAMPLE_BUDGET_MS}ms: immediate_batch_samples_ms={immediate_batch_samples:?}"
    );
    Ok(())
}
