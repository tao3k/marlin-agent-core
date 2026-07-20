#![cfg(feature = "turso")]

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactRecord, EventId, ProjectId, SessionEventRecord,
    SessionId, StorageResult, TurnId, TursoAgentStorage, TursoAgentStorageConfig,
    VisibilityReceipt,
};
use serde::Deserialize;
use tempfile::tempdir;

#[derive(Debug, Deserialize)]
struct DbPressureDocument {
    db_pressure: DbPressureScenario,
}

#[derive(Debug, Deserialize)]
struct DbPressureScenario {
    sessions: usize,
    events_per_session: usize,
    artifacts: usize,
    visibility_receipts: usize,
}

fn db_pressure_scenario() -> DbPressureScenario {
    toml::from_str::<DbPressureDocument>(include_str!(
        "fixtures/db_pressure/scenarios/multi_session_reopen.toml"
    ))
    .expect("db pressure scenario fixture must be valid TOML")
    .db_pressure
}

fn project_id() -> ProjectId {
    ProjectId::new("project:storage-db-pressure").expect("valid project id")
}

fn session_id(index: usize) -> SessionId {
    SessionId::new(format!("session:{index:02}")).expect("valid session id")
}

fn agent_id(index: usize) -> AgentId {
    AgentId::new(format!("agent:{index:02}")).expect("valid agent id")
}

fn turn_id(session: usize, event: usize) -> TurnId {
    TurnId::new(format!("turn:{session:02}:{event:03}")).expect("valid turn id")
}

fn event_id(session: usize, event: usize) -> EventId {
    EventId::new(format!("event:{session:02}:{event:03}")).expect("valid event id")
}

fn artifact_hash(index: usize) -> ArtifactHash {
    ArtifactHash::new(format!("sha256:artifact-{index:03}")).expect("valid artifact hash")
}

fn session_event(project_id: &ProjectId, session: usize, event: usize) -> SessionEventRecord {
    SessionEventRecord {
        project_id: project_id.clone(),
        session_id: session_id(session),
        agent_id: agent_id(session),
        turn_id: turn_id(session, event),
        event_id: event_id(session, event),
        event_kind: "storage.db_pressure.session_event".to_owned(),
        causality_parent_event_id: (event > 0).then(|| event_id(session, event - 1)),
        body: format!("session={session};event={event}").into_bytes(),
        created_at_unix_ms: 1_800_000_000_000 + ((session * 1_000 + event) as i64),
    }
}

fn artifact(project_id: &ProjectId, index: usize) -> ArtifactRecord {
    let body = format!("artifact-body-{index:03}").into_bytes();
    ArtifactRecord {
        project_id: project_id.clone(),
        artifact_hash: artifact_hash(index),
        artifact_kind: "storage.db_pressure.artifact".to_owned(),
        producer_session_id: session_id(index % 3),
        producer_agent_id: agent_id(index % 3),
        producer_event_id: event_id(index % 3, index),
        media_type: "application/octet-stream".to_owned(),
        size_bytes: body.len() as u64,
        body,
        created_at_unix_ms: 1_800_000_100_000 + index as i64,
    }
}

fn visibility_receipt(project_id: &ProjectId, index: usize) -> VisibilityReceipt {
    VisibilityReceipt {
        project_id: project_id.clone(),
        receipt_id: format!("db-pressure-receipt-{index:03}"),
        receipt_kind: "storage.db_pressure.visibility".to_owned(),
        body: format!("receipt={index}").into_bytes(),
        created_at_unix_ms: 1_800_000_200_000 + index as i64,
    }
}

#[derive(Debug)]
struct TursoDbPressurePerformanceReceipt {
    optimization: marlin_agent_storage::TursoOptimizationReceipt,
    cold_open_ms: u128,
    concurrent_event_writes_ms: u128,
    concurrent_artifact_writes_ms: u128,
    concurrent_visibility_writes_ms: u128,
    warm_reopen_ms: u128,
    indexed_reads_ms: u128,
    total_ms: u128,
    resident_memory_bytes: Option<u64>,
    database_bytes: u64,
    session_count: usize,
    event_count: usize,
    artifact_count: usize,
    workload_visibility_receipt_count: usize,
}

impl TursoDbPressurePerformanceReceipt {
    fn visibility_body(&self) -> Vec<u8> {
        format!(
            "optimization={:?}\ncold_open_ms={}\nconcurrent_event_writes_ms={}\nconcurrent_artifact_writes_ms={}\nconcurrent_visibility_writes_ms={}\nwarm_reopen_ms={}\nindexed_reads_ms={}\ntotal_ms={}\nresident_memory_bytes={:?}\ndatabase_bytes={}\nsession_count={}\nevent_count={}\nartifact_count={}\nworkload_visibility_receipt_count={}\n",
            self.optimization,
            self.cold_open_ms,
            self.concurrent_event_writes_ms,
            self.concurrent_artifact_writes_ms,
            self.concurrent_visibility_writes_ms,
            self.warm_reopen_ms,
            self.indexed_reads_ms,
            self.total_ms,
            self.resident_memory_bytes,
            self.database_bytes,
            self.session_count,
            self.event_count,
            self.artifact_count,
            self.workload_visibility_receipt_count,
        )
        .into_bytes()
    }
}

fn current_resident_memory_bytes() -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    output.status.success().then_some(())?;
    String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(|rss_kib| rss_kib * 1024)
}

const fn batch_transaction_mode_for_profile(
    profile: marlin_agent_storage::TursoOptimizationProfile,
) -> marlin_agent_storage::TursoBatchTransactionMode {
    match profile {
        marlin_agent_storage::TursoOptimizationProfile::AsyncIoOnlyCompatibility => {
            marlin_agent_storage::TursoBatchTransactionMode::Immediate
        }
        marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvcc
        | marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental => {
            marlin_agent_storage::TursoBatchTransactionMode::Concurrent
        }
    }
}

async fn run_turso_db_pressure_once(
    optimization_profile: marlin_agent_storage::TursoOptimizationProfile,
    batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode,
) -> StorageResult<TursoDbPressurePerformanceReceipt> {
    let total_started = std::time::Instant::now();
    let scenario = db_pressure_scenario();
    let project_id = project_id();
    let tempdir = tempdir().expect("tempdir should be available");
    let db_path = tempdir.path().join("agent-storage.turso");
    let config = TursoAgentStorageConfig {
        path: db_path.clone(),
        optimization_profile,
        batch_transaction_mode,
    };
    let cold_open_started = std::time::Instant::now();
    let storage = TursoAgentStorage::open_local(config).await?;
    assert_eq!(
        storage.optimization_receipt(),
        optimization_profile.receipt()
    );
    let cold_open_ms = cold_open_started.elapsed().as_millis();

    let concurrent_event_writes_started = std::time::Instant::now();
    let mut writers = tokio::task::JoinSet::new();
    for session in 0..scenario.sessions {
        let writer_storage = storage.clone();
        let writer_project_id = project_id.clone();
        let events_per_session = scenario.events_per_session;
        writers.spawn(async move {
            let records = (0..events_per_session)
                .map(|event| session_event(&writer_project_id, session, event))
                .collect();
            let receipt = writer_storage.append_session_events_batch(records).await?;
            assert_eq!(receipt.item_count, events_per_session);
            assert_eq!(receipt.rows_affected, events_per_session as u64);
            assert_eq!(receipt.transaction_mode, batch_transaction_mode);
            Ok::<(), marlin_agent_storage::StorageError>(())
        });
    }
    while let Some(result) = writers.join_next().await {
        result.expect("concurrent session writer should not panic")?;
    }
    let concurrent_event_writes_ms = concurrent_event_writes_started.elapsed().as_millis();

    let concurrent_artifact_writes_started = std::time::Instant::now();
    let mut artifact_writers = tokio::task::JoinSet::new();
    for index in 0..scenario.artifacts {
        let writer_storage = storage.clone();
        let writer_project_id = project_id.clone();
        artifact_writers.spawn(async move {
            let record = artifact(&writer_project_id, index);
            let outcome = writer_storage.put_artifact(record.clone()).await?;
            assert!(outcome.inserted, "first artifact write should insert");
            assert_eq!(outcome.artifact, record);
            Ok::<(), marlin_agent_storage::StorageError>(())
        });
    }
    while let Some(result) = artifact_writers.join_next().await {
        result.expect("concurrent artifact writer should not panic")?;
    }
    let concurrent_artifact_writes_ms = concurrent_artifact_writes_started.elapsed().as_millis();

    let concurrent_visibility_writes_started = std::time::Instant::now();
    let mut visibility_writers = tokio::task::JoinSet::new();
    for index in 0..scenario.visibility_receipts {
        let writer_storage = storage.clone();
        let writer_project_id = project_id.clone();
        visibility_writers.spawn(async move {
            writer_storage
                .record_visibility(visibility_receipt(&writer_project_id, index))
                .await
        });
    }
    while let Some(result) = visibility_writers.join_next().await {
        result.expect("concurrent visibility writer should not panic")?;
    }
    let concurrent_visibility_writes_ms =
        concurrent_visibility_writes_started.elapsed().as_millis();

    drop(storage);
    let warm_reopen_started = std::time::Instant::now();

    let reopened = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: db_path.clone(),
        optimization_profile,
        batch_transaction_mode,
    })
    .await?;
    assert_eq!(
        reopened.optimization_receipt(),
        optimization_profile.receipt()
    );
    let warm_reopen_ms = warm_reopen_started.elapsed().as_millis();
    let indexed_reads_started = std::time::Instant::now();

    for session in 0..scenario.sessions {
        let events = reopened
            .list_session_events_page(marlin_agent_storage::SessionEventPageRequest::new(
                project_id.clone(),
                session_id(session),
                marlin_agent_storage::StoragePageLimit::MAXIMUM,
            ))
            .await?
            .items;
        assert_eq!(events.len(), scenario.events_per_session);
        for (event, record) in events.iter().enumerate() {
            assert_eq!(record.event_id, event_id(session, event));
            assert_eq!(
                record.causality_parent_event_id,
                (event > 0).then(|| event_id(session, event - 1))
            );
        }
    }

    for index in 0..scenario.artifacts {
        let stored = reopened
            .get_artifact(&project_id, &artifact_hash(index))
            .await?
            .expect("artifact should persist across reopen");
        assert_eq!(stored, artifact(&project_id, index));
    }

    let receipts = reopened
        .list_visibility_page(marlin_agent_storage::VisibilityPageRequest::new(
            project_id.clone(),
            marlin_agent_storage::StoragePageLimit::MAXIMUM,
        ))
        .await?
        .items;
    assert_eq!(receipts.len(), scenario.visibility_receipts);
    for index in 0..scenario.visibility_receipts {
        assert!(
            receipts
                .iter()
                .any(|receipt| receipt.receipt_id == format!("db-pressure-receipt-{index:03}")),
            "missing visibility receipt {index}"
        );
    }

    let indexed_reads_ms = indexed_reads_started.elapsed().as_millis();
    let performance_receipt = TursoDbPressurePerformanceReceipt {
        optimization: reopened.optimization_receipt(),
        cold_open_ms,
        concurrent_event_writes_ms,
        concurrent_artifact_writes_ms,
        concurrent_visibility_writes_ms,
        warm_reopen_ms,
        indexed_reads_ms,
        total_ms: total_started.elapsed().as_millis(),
        resident_memory_bytes: current_resident_memory_bytes(),
        database_bytes: std::fs::metadata(&db_path)
            .map(|metadata| metadata.len())
            .unwrap_or_default(),
        session_count: scenario.sessions,
        event_count: scenario.sessions * scenario.events_per_session,
        artifact_count: scenario.artifacts,
        workload_visibility_receipt_count: scenario.visibility_receipts,
    };
    assert_eq!(
        performance_receipt.optimization,
        optimization_profile.receipt()
    );
    assert!(performance_receipt.database_bytes > 0);
    let performance_body = performance_receipt.visibility_body();
    reopened
        .record_visibility(VisibilityReceipt {
            project_id: project_id.clone(),
            receipt_id: "rfcdb-turso-db-pressure-performance".to_owned(),
            receipt_kind: "rfcdb.turso.performance".to_owned(),
            body: performance_body.clone(),
            created_at_unix_ms: 1_800_006_100_000,
        })
        .await?;
    assert!(
        reopened
            .list_visibility_page(marlin_agent_storage::VisibilityPageRequest::new(
                project_id.clone(),
                marlin_agent_storage::StoragePageLimit::MAXIMUM,
            ))
            .await?
            .items
            .iter()
            .any(|receipt| {
                receipt.receipt_kind == "rfcdb.turso.performance"
                    && receipt.body == performance_body
            })
    );
    eprintln!(
        "turso db pressure performance receipt: optimization={:?} cold_open_ms={} concurrent_event_writes_ms={} concurrent_artifact_writes_ms={} concurrent_visibility_writes_ms={} warm_reopen_ms={} indexed_reads_ms={} total_ms={} resident_memory_bytes={:?} database_bytes={} session_count={} event_count={} artifact_count={} workload_visibility_receipt_count={}",
        performance_receipt.optimization,
        performance_receipt.cold_open_ms,
        performance_receipt.concurrent_event_writes_ms,
        performance_receipt.concurrent_artifact_writes_ms,
        performance_receipt.concurrent_visibility_writes_ms,
        performance_receipt.warm_reopen_ms,
        performance_receipt.indexed_reads_ms,
        performance_receipt.total_ms,
        performance_receipt.resident_memory_bytes,
        performance_receipt.database_bytes,
        performance_receipt.session_count,
        performance_receipt.event_count,
        performance_receipt.artifact_count,
        performance_receipt.workload_visibility_receipt_count,
    );

    Ok(performance_receipt)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turso_persists_multi_session_events_artifacts_and_visibility_across_reopen()
-> StorageResult<()> {
    let performance_receipt = run_turso_db_pressure_once(
        marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
        marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    )
    .await?;
    assert_eq!(performance_receipt.session_count, 6);
    assert_eq!(performance_receipt.event_count, 72);
    assert_eq!(performance_receipt.artifact_count, 18);
    assert_eq!(performance_receipt.workload_visibility_receipt_count, 9);
    Ok(())
}

fn median_u128(mut values: Vec<u128>) -> u128 {
    assert!(!values.is_empty(), "median requires at least one sample");
    values.sort_unstable();
    values[values.len() / 2]
}

fn median_u64(mut values: Vec<u64>) -> u64 {
    assert!(!values.is_empty(), "median requires at least one sample");
    values.sort_unstable();
    values[values.len() / 2]
}

fn assert_live_phase_within_budget(
    benchmark: &marlin_rust_project_harness_policy::RustScenarioBenchmarkReceipt,
    phase: &str,
    median_ms: u128,
) {
    let observed = benchmark
        .benchmark
        .observed_timings
        .get(phase)
        .unwrap_or_else(|| panic!("missing canonical observation for phase {phase}"));
    let phase_max = observed.0 + benchmark.benchmark.regression_budget.0;
    let live = std::time::Duration::from_millis(
        u64::try_from(median_ms).expect("phase median should fit u64 milliseconds"),
    );
    assert!(
        live <= phase_max,
        "live phase {phase} median {median_ms}ms exceeded observed {:?} plus regression {:?}",
        observed,
        benchmark.benchmark.regression_budget,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "manual release-only live Turso performance regression gate"]
async fn rfcdb_turso_db_pressure_live_performance_regression_gate() -> StorageResult<()> {
    if cfg!(debug_assertions) {
        panic!("live Turso performance gates must run with `cargo test --release`");
    }
    const SAMPLE_COUNT: usize = 5;

    let scenario_root =
        crate::paths::crate_root().join("tests/unit/scenarios/performance_baseline");
    let benchmark =
        marlin_rust_project_harness_policy::validate_rust_scenario_benchmark(&scenario_root)
            .expect("validate Turso performance baseline before live measurement");
    let optimization_profile =
        marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental;
    let warmup = run_turso_db_pressure_once(
        optimization_profile,
        batch_transaction_mode_for_profile(optimization_profile),
    )
    .await?;
    eprintln!(
        "turso live performance warmup: total_ms={} cold_open_ms={} warm_reopen_ms={}",
        warmup.total_ms, warmup.cold_open_ms, warmup.warm_reopen_ms
    );
    let mut receipts = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        receipts.push(
            run_turso_db_pressure_once(
                optimization_profile,
                batch_transaction_mode_for_profile(optimization_profile),
            )
            .await?,
        );
    }

    let total_samples_ms = receipts
        .iter()
        .map(|receipt| receipt.total_ms)
        .collect::<Vec<_>>();
    let median_total_ms = median_u128(total_samples_ms.clone());
    let slowest_total_ms = *total_samples_ms
        .iter()
        .max()
        .expect("live gate should produce total samples");
    let max_total_ms = benchmark.benchmark.max_total.0.as_millis();
    let total_samples_within_max = total_samples_ms
        .iter()
        .filter(|sample| **sample <= max_total_ms)
        .count();
    let resident_memory_samples = receipts
        .iter()
        .filter_map(|receipt| receipt.resident_memory_bytes)
        .collect::<Vec<_>>();
    let median_resident_memory_bytes = median_u64(resident_memory_samples.clone());
    let peak_resident_memory_bytes = *resident_memory_samples
        .iter()
        .max()
        .expect("live gate should produce resident memory samples");
    let median_cold_open_ms = median_u128(
        receipts
            .iter()
            .map(|receipt| receipt.cold_open_ms)
            .collect(),
    );
    let median_concurrent_event_writes_ms = median_u128(
        receipts
            .iter()
            .map(|receipt| receipt.concurrent_event_writes_ms)
            .collect(),
    );
    let median_concurrent_artifact_writes_ms = median_u128(
        receipts
            .iter()
            .map(|receipt| receipt.concurrent_artifact_writes_ms)
            .collect(),
    );
    let median_concurrent_visibility_writes_ms = median_u128(
        receipts
            .iter()
            .map(|receipt| receipt.concurrent_visibility_writes_ms)
            .collect(),
    );
    let median_warm_reopen_ms = median_u128(
        receipts
            .iter()
            .map(|receipt| receipt.warm_reopen_ms)
            .collect(),
    );
    let median_indexed_reads_ms = median_u128(
        receipts
            .iter()
            .map(|receipt| receipt.indexed_reads_ms)
            .collect(),
    );

    assert!(
        std::time::Duration::from_millis(
            u64::try_from(median_total_ms).expect("median total should fit u64 milliseconds"),
        ) <= benchmark.benchmark.max_total.0,
        "live median total {median_total_ms}ms exceeded baseline max {:?}",
        benchmark.benchmark.max_total
    );
    assert!(
        total_samples_within_max >= SAMPLE_COUNT - 1,
        "live tail gate requires at least {} of {SAMPLE_COUNT} samples within {}ms, observed {total_samples_ms:?}",
        SAMPLE_COUNT - 1,
        max_total_ms
    );
    assert!(
        median_resident_memory_bytes <= benchmark.benchmark.memory_budget_bytes.0,
        "live median RSS {median_resident_memory_bytes} exceeded budget {:?}",
        benchmark.benchmark.memory_budget_bytes
    );
    assert!(
        peak_resident_memory_bytes <= benchmark.benchmark.memory_budget_bytes.0,
        "live peak RSS {peak_resident_memory_bytes} exceeded budget {:?}",
        benchmark.benchmark.memory_budget_bytes
    );
    assert!(
        receipts
            .iter()
            .all(|receipt| { receipt.optimization == optimization_profile.receipt() })
    );
    assert_live_phase_within_budget(&benchmark, "cold_open", median_cold_open_ms);
    assert_live_phase_within_budget(
        &benchmark,
        "concurrent_event_writes",
        median_concurrent_event_writes_ms,
    );
    assert_live_phase_within_budget(
        &benchmark,
        "concurrent_artifact_writes",
        median_concurrent_artifact_writes_ms,
    );
    assert_live_phase_within_budget(
        &benchmark,
        "concurrent_visibility_writes",
        median_concurrent_visibility_writes_ms,
    );
    assert_live_phase_within_budget(&benchmark, "warm_reopen", median_warm_reopen_ms);
    assert_live_phase_within_budget(&benchmark, "indexed_reads", median_indexed_reads_ms);

    eprintln!(
        "turso live performance gate: samples={SAMPLE_COUNT} total_samples_ms={total_samples_ms:?} median_total_ms={median_total_ms} slowest_total_ms={slowest_total_ms} total_samples_within_max={total_samples_within_max} required_samples_within_max={} max_total_ms={max_total_ms} median_resident_memory_bytes={median_resident_memory_bytes} peak_resident_memory_bytes={peak_resident_memory_bytes} median_cold_open_ms={median_cold_open_ms} median_concurrent_event_writes_ms={median_concurrent_event_writes_ms} median_concurrent_artifact_writes_ms={median_concurrent_artifact_writes_ms} median_concurrent_visibility_writes_ms={median_concurrent_visibility_writes_ms} median_warm_reopen_ms={median_warm_reopen_ms} median_indexed_reads_ms={median_indexed_reads_ms}",
        SAMPLE_COUNT - 1
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "manual release-only live Turso optimization profile comparison"]
async fn rfcdb_turso_optimization_profiles_live_comparison() -> StorageResult<()> {
    if cfg!(debug_assertions) {
        panic!("live Turso performance gates must run with `cargo test --release`");
    }
    const SAMPLE_COUNT: usize = 3;
    const PROFILES: [marlin_agent_storage::TursoOptimizationProfile; 3] = [
        marlin_agent_storage::TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvcc,
        marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvccAndPassiveCheckpointExperimental,
    ];

    let mut comparison_receipts = Vec::with_capacity(PROFILES.len());
    for profile in PROFILES {
        let mut receipts = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            receipts.push(
                run_turso_db_pressure_once(profile, batch_transaction_mode_for_profile(profile))
                    .await?,
            );
        }
        assert!(
            receipts
                .iter()
                .all(|receipt| receipt.optimization == profile.receipt())
        );

        let median_total_ms =
            median_u128(receipts.iter().map(|receipt| receipt.total_ms).collect());
        let median_resident_memory_bytes = median_u64(
            receipts
                .iter()
                .filter_map(|receipt| receipt.resident_memory_bytes)
                .collect(),
        );
        let median_cold_open_ms = median_u128(
            receipts
                .iter()
                .map(|receipt| receipt.cold_open_ms)
                .collect(),
        );
        let median_warm_reopen_ms = median_u128(
            receipts
                .iter()
                .map(|receipt| receipt.warm_reopen_ms)
                .collect(),
        );
        comparison_receipts.push((profile, profile.receipt()));
        eprintln!(
            "turso profile comparison: profile={profile:?} optimization={:?} samples={SAMPLE_COUNT} median_total_ms={median_total_ms} median_resident_memory_bytes={median_resident_memory_bytes} median_cold_open_ms={median_cold_open_ms} median_warm_reopen_ms={median_warm_reopen_ms}",
            profile.receipt(),
        );
    }

    assert_eq!(comparison_receipts.len(), PROFILES.len());
    assert!(
        comparison_receipts
            .windows(2)
            .all(|pair| pair[0].1 != pair[1].1)
    );

    Ok(())
}
