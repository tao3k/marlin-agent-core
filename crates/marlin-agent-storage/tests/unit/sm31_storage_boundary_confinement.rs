// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM31 (ADV-L4) hostile verification suite for storage deserialization parity,
//! content-addressed artifact integrity, TLS-mandatory sync transport, and
//! idempotent CAS audit receipts.

use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerUpdate, ArtifactRecord,
    EventId, InMemoryAgentStorage, ProjectId, SessionId, StorageError, StoragePageLimit, TurnId,
    VisibilityReceipt,
};
#[cfg(feature = "turso")]
use marlin_agent_storage::{
    TursoAgentStorage, TursoAgentStorageConfig, TursoBatchTransactionMode, TursoOptimizationProfile,
};

const LEGITIMATE_POLICY_BODY: &[u8] = b"LEGITIMATE_POLICY";
// Exact FIPS 180-4 SHA-256 of b"LEGITIMATE_POLICY" (verified via `printf 'LEGITIMATE_POLICY' | shasum -a 256`).
const LEGITIMATE_POLICY_SHA256: &str =
    "sha256:7395dbcedfa2d13e04ee1c2c2f68542e83a1477211a71aef27b4e3746418d67a";

fn make_artifact(hash: &str, body: &[u8], size_bytes: u64) -> ArtifactRecord {
    ArtifactRecord {
        project_id: ProjectId::new("proj:sm31").unwrap(),
        artifact_hash: ArtifactHash::new(hash).unwrap(),
        artifact_kind: "governance.policy".to_owned(),
        producer_session_id: SessionId::new("session:sm31").unwrap(),
        producer_agent_id: AgentId::new("agent:sm31").unwrap(),
        producer_event_id: EventId::new("event:sm31-1").unwrap(),
        media_type: "application/octet-stream".to_owned(),
        size_bytes,
        body: body.to_vec(),
        created_at_unix_ms: 1_800_000_000_000,
    }
}

#[test]
fn sm31_adv_l4_serde_rejects_out_of_range_page_limits_and_empty_storage_ids() {
    assert!(
        serde_json::from_str::<StoragePageLimit>("0").is_err(),
        "StoragePageLimit::Deserialize must reject 0"
    );
    assert!(
        serde_json::from_str::<StoragePageLimit>("1001").is_err(),
        "StoragePageLimit::Deserialize must reject values above MAX (1000)"
    );
    assert!(
        serde_json::from_str::<StoragePageLimit>("65535").is_err(),
        "StoragePageLimit::Deserialize must reject u16::MAX"
    );
    assert_eq!(
        serde_json::from_str::<StoragePageLimit>("1").unwrap().get(),
        1
    );
    assert_eq!(
        serde_json::from_str::<StoragePageLimit>("1000")
            .unwrap()
            .get(),
        1000
    );

    assert!(
        serde_json::from_str::<ProjectId>(r#""""#).is_err(),
        "ProjectId::Deserialize must reject empty string"
    );
    assert!(
        serde_json::from_str::<SessionId>(r#""   ""#).is_err(),
        "SessionId::Deserialize must reject whitespace-only string"
    );
    assert!(
        serde_json::from_str::<TurnId>(r#""""#).is_err(),
        "TurnId::Deserialize must reject empty string"
    );
    assert!(
        serde_json::from_str::<EventId>(r#""\t\n""#).is_err(),
        "EventId::Deserialize must reject whitespace-only string"
    );
    assert!(
        serde_json::from_str::<ArtifactHash>(r#""""#).is_err(),
        "ArtifactHash::Deserialize must reject empty string"
    );
    assert_eq!(
        serde_json::from_str::<ProjectId>(r#""proj:ok""#)
            .unwrap()
            .as_str(),
        "proj:ok"
    );
}

#[tokio::test]
async fn sm31_adv_l4_put_artifact_rejects_size_lie_i64_wrap_and_sha256_cas_poisoning() {
    let memory = InMemoryAgentStorage::new();

    // 1. Forged size_bytes=0 on non-empty body must be refused.
    let err_zero = memory
        .put_artifact(make_artifact("sha256:synthetic-size-0", b"payload-16-bytes", 0))
        .await
        .expect_err("forged size_bytes=0 must be rejected");
    assert!(
        !matches!(err_zero, StorageError::MissingArtifact { .. }),
        "expected size integrity error, got {err_zero:?}"
    );

    // 2. Forged size_bytes=u64::MAX (wraps to -1 as i64 in SQLite) must be refused.
    assert!(
        memory
            .put_artifact(make_artifact(
                "sha256:synthetic-size-max",
                b"payload",
                u64::MAX
            ))
            .await
            .is_err(),
        "forged size_bytes=u64::MAX must be rejected"
    );

    // 3. Poisoned payload under a real 64-hex sha256:<hex> digest must be refused.
    let poisoned = b"POISONED_PAYLOAD!";
    let err_poison = memory
        .put_artifact(make_artifact(
            LEGITIMATE_POLICY_SHA256,
            poisoned,
            poisoned.len() as u64,
        ))
        .await
        .expect_err("mismatched sha256:<64-hex> payload must be refused");
    assert!(
        matches!(err_poison, StorageError::ArtifactHashCollision { .. }),
        "expected ArtifactHashCollision on sha256 digest mismatch, got {err_poison:?}"
    );

    // 4. Legitimate payload matching both size_bytes and exact sha256 digest must succeed.
    let ok = memory
        .put_artifact(make_artifact(
            LEGITIMATE_POLICY_SHA256,
            LEGITIMATE_POLICY_BODY,
            LEGITIMATE_POLICY_BODY.len() as u64,
        ))
        .await
        .expect("legitimate sha256 artifact must succeed");
    assert!(ok.inserted);

    #[cfg(feature = "turso")]
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let turso = TursoAgentStorage::open_local(TursoAgentStorageConfig {
            path: dir.path().join("sm31-cas.db"),
            optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
            batch_transaction_mode: TursoBatchTransactionMode::Immediate,
        })
        .await
        .expect("open turso");

        assert!(
            turso
                .put_artifact(make_artifact("sha256:turso-wrap", b"abc", u64::MAX))
                .await
                .is_err(),
            "Turso put_artifact must reject u64::MAX size_bytes before i64 wrap"
        );
        let err_turso_poison = turso
            .put_artifact(make_artifact(
                LEGITIMATE_POLICY_SHA256,
                poisoned,
                poisoned.len() as u64,
            ))
            .await
            .expect_err("Turso put_artifact must reject poisoned sha256:<64-hex> payload");
        assert!(matches!(
            err_turso_poison,
            StorageError::ArtifactHashCollision { .. }
        ));
        let ok_turso = turso
            .put_artifact(make_artifact(
                LEGITIMATE_POLICY_SHA256,
                LEGITIMATE_POLICY_BODY,
                LEGITIMATE_POLICY_BODY.len() as u64,
            ))
            .await
            .expect("Turso legitimate sha256 artifact must succeed");
        assert!(ok_turso.inserted);
    }
}

#[cfg(feature = "turso-sync")]
#[test]
fn sm31_adv_l4_turso_sync_remote_url_blocks_cleartext_remote_http_userinfo_and_crlf() {
    use marlin_agent_storage::TursoSyncRemoteUrl;

    for hostile in [
        "http://evil-remote.example/db",
        "http://10.0.0.5:8080/db",
        "https://trusted.turso.io@attacker.example/db",
        "libsql://user:pass@attacker.example",
        "https://turso.io\r\nAuthorization: Bearer evil",
        "https://turso.io/path with space",
        "https://",
        "libsql://",
        "http://",
        "file:///etc/passwd",
    ] {
        assert!(
            matches!(
                TursoSyncRemoteUrl::new(hostile),
                Err(StorageError::InvalidSyncConfiguration { .. })
            ),
            "hostile sync remote URL must be refused: {hostile:?}"
        );
    }

    for allowed in [
        "https://my-db.turso.io",
        "libsql://my-db.turso.io:443",
        "http://127.0.0.1:8080",
        "http://localhost:8080",
        "http://[::1]:8080",
    ] {
        assert!(
            TursoSyncRemoteUrl::new(allowed).is_ok(),
            "legitimate sync remote URL must be admitted: {allowed:?}"
        );
    }
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn sm31_adv_l4_turso_cas_pointer_repeated_conflict_and_commit_audit_receipts_are_idempotent()
{
    let dir = tempfile::tempdir().expect("tempdir");
    let turso = TursoAgentStorage::open_local(TursoAgentStorageConfig {
        path: dir.path().join("sm31-cas-retry.db"),
        optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        batch_transaction_mode: TursoBatchTransactionMode::Immediate,
    })
    .await
    .expect("open turso");

    let proj = ProjectId::new("proj:sm31").unwrap();
    let ptr = ArtifactPointerKey::new("checkpoint:main").unwrap();
    let h1 = ArtifactHash::new("sha256:cas-v1").unwrap();
    let h2 = ArtifactHash::new("sha256:cas-v2").unwrap();
    turso
        .put_artifact(make_artifact(h1.as_str(), b"v1", 2))
        .await
        .unwrap();
    turso
        .put_artifact(make_artifact(h2.as_str(), b"v2", 2))
        .await
        .unwrap();

    // Seed initial pointer -> h1.
    turso
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: proj.clone(),
            pointer_key: ptr.clone(),
            expected_artifact_hash: None,
            new_artifact_hash: h1.clone(),
            updated_by_session_id: SessionId::new("session:sm31").unwrap(),
            updated_by_agent_id: AgentId::new("agent:sm31").unwrap(),
            updated_by_event_id: EventId::new("ev:seed").unwrap(),
            updated_at_unix_ms: 100,
        })
        .await
        .expect("initial CAS commit");

    // Retry the SAME conflicting CAS event twice: BOTH must return ArtifactPointerConflict
    // (never StorageError::Backend("UNIQUE constraint failed: visibility_receipts...")).
    for attempt in 0..2 {
        let err = turso
            .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
                project_id: proj.clone(),
                pointer_key: ptr.clone(),
                expected_artifact_hash: None,
                new_artifact_hash: h2.clone(),
                updated_by_session_id: SessionId::new("session:sm31").unwrap(),
                updated_by_agent_id: AgentId::new("agent:sm31").unwrap(),
                updated_by_event_id: EventId::new("ev:retry-conflict").unwrap(),
                updated_at_unix_ms: 200 + attempt,
            })
            .await
            .expect_err("stale expected hash must conflict");
        assert!(
            matches!(err, StorageError::ArtifactPointerConflict { .. }),
            "attempt {attempt}: expected ArtifactPointerConflict, got {err:?}"
        );
    }

    // Pre-insert a colliding committed visibility receipt for ev:commit-2 and perform valid CAS:
    // pointer update MUST commit cleanly without split-brain Backend error.
    turso
        .record_visibility(VisibilityReceipt {
            project_id: proj.clone(),
            receipt_id: format!("turso:artifact-pointer-cas:{}:ev:commit-2:committed", ptr),
            receipt_kind: "storage.turso.transaction".to_owned(),
            body: b"preexisting".to_vec(),
            created_at_unix_ms: 250,
        })
        .await
        .unwrap();
    let committed = turso
        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
            project_id: proj.clone(),
            pointer_key: ptr.clone(),
            expected_artifact_hash: Some(h1.clone()),
            new_artifact_hash: h2.clone(),
            updated_by_session_id: SessionId::new("session:sm31").unwrap(),
            updated_by_agent_id: AgentId::new("agent:sm31").unwrap(),
            updated_by_event_id: EventId::new("ev:commit-2").unwrap(),
            updated_at_unix_ms: 300,
        })
        .await
        .expect("valid CAS must succeed idempotently even if audit receipt already exists");
    assert_eq!(committed.target_artifact_hash, h2);
}
