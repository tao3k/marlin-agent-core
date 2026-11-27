// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::sync::Arc;
use std::time::Duration;

use marlin_agent_protocol::{
    ModelEndpoint, ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, assistant_gateway_message,
    user_gateway_message,
};
use marlin_agent_runtime::{
    AtomicEffect, AtomicEffectReceipt, CasGetEffectPayload, CasPutEffectPayload,
    LlmCompleteEffectPayload, SandboxRunEffectPayload, run_atomic_effect_batch,
};
use marlin_agent_storage::{
    AgentId, ProjectId, SessionId, TursoAgentStorage, TursoAgentStorageConfig,
    TursoBatchTransactionMode, TursoOptimizationProfile,
};

#[cfg(feature = "turso-host")]
use marlin_agent_app_server::atomic_effect_substrate::StorageCasStore;
use marlin_agent_app_server::atomic_effect_substrate::{
    AllowlistSandboxRunner, AtomicSandboxRunner, BoundedTokioSandboxRunner,
    CompositeAtomicEffectExecutor,
};

struct EchoModelGateway;

impl ModelGateway for EchoModelGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        let prompt = request
            .messages()
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        Box::pin(async move {
            Ok(ModelGatewayCompletionResponse::new(
                "echo-resp-1",
                "gpt-marlin-echo",
                vec![ModelGatewayCompletionChoice::new(
                    0,
                    assistant_gateway_message(format!("echo:{prompt}")),
                    None,
                )],
            ))
        })
    }
}

#[tokio::test]
async fn composite_executor_runs_all_four_primitives_on_real_turso_wal_and_survives_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = TursoAgentStorageConfig {
        path: dir.path().join("atomic-substrate.db"),
        optimization_profile: TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: TursoBatchTransactionMode::Concurrent,
    };
    let project = ProjectId::new("proj-s11").unwrap();
    let session = SessionId::new("sess-s11").unwrap();
    let agent = AgentId::new("agent-s11").unwrap();

    // Boot #1 with default fail-closed DenySandboxRunner
    let storage1 = TursoAgentStorage::open_local(cfg.clone())
        .await
        .expect("open turso #1");
    let exec1 = Arc::new(CompositeAtomicEffectExecutor::new(
        storage1,
        EchoModelGateway,
        project.clone(),
        session.clone(),
        agent.clone(),
    ));

    let batch1 = vec![
        AtomicEffect::CasGet(CasGetEffectPayload {
            effect_id: "e-miss".to_owned(),
            key: "k:s11:wire".to_owned(),
        }),
        AtomicEffect::LlmComplete(LlmCompleteEffectPayload {
            effect_id: "e-llm".to_owned(),
            request: ModelGatewayRequest::new(
                ModelEndpoint::new("openai", "gpt-marlin-echo"),
                vec![user_gateway_message("ping-s11")],
            ),
        }),
        AtomicEffect::SandboxRun(SandboxRunEffectPayload {
            effect_id: "e-sb-denied".to_owned(),
            program: "/bin/echo".to_owned(),
            args: vec!["blocked".to_owned()],
            cwd: None,
        }),
    ];
    let rcpts1 = run_atomic_effect_batch(Arc::clone(&exec1), batch1).await;
    assert!(matches!(rcpts1[0], AtomicEffectReceipt::CasMiss(ref id) if id == "e-miss"));
    assert!(matches!(
        rcpts1[1],
        AtomicEffectReceipt::LlmCompleted(ref p) if p.effect_id == "e-llm" && p.content == "echo:ping-s11"
    ));
    assert!(matches!(
        rcpts1[2],
        AtomicEffectReceipt::EffectError(ref p) if p.effect_id == "e-sb-denied"
    ));

    // Store bytes via CasPut on real Turso WAL, then drop Boot #1 completely!
    let put_rcpts = run_atomic_effect_batch(
        Arc::clone(&exec1),
        vec![AtomicEffect::CasPut(CasPutEffectPayload {
            effect_id: "e-put".to_owned(),
            key: "k:s11:wire".to_owned(),
            bytes: b"(fleet-wire-v1 (round . 11))".to_vec(),
        })],
    )
    .await;
    assert!(
        matches!(put_rcpts.as_slice(), [AtomicEffectReceipt::CasStored(id)] if id == "e-put")
    );
    drop(exec1);

    // Boot #2: Reopen Turso WAL + mount BoundedTokioSandboxRunner
    let storage2 = TursoAgentStorage::open_local(cfg)
        .await
        .expect("open turso #2");
    let exec2 = Arc::new(
        CompositeAtomicEffectExecutor::new(storage2, EchoModelGateway, project, session, agent)
            .with_sandbox(BoundedTokioSandboxRunner::new(Duration::from_secs(5), 4096)),
    );
    let batch2 = vec![
        AtomicEffect::CasGet(CasGetEffectPayload {
            effect_id: "e-hit".to_owned(),
            key: "k:s11:wire".to_owned(),
        }),
        AtomicEffect::SandboxRun(SandboxRunEffectPayload {
            effect_id: "e-sb-ok".to_owned(),
            program: "/bin/echo".to_owned(),
            args: vec!["hello-atomic-sandbox".to_owned()],
            cwd: None,
        }),
    ];
    let rcpts2 = run_atomic_effect_batch(Arc::clone(&exec2), batch2).await;
    assert!(matches!(
        rcpts2[0],
        AtomicEffectReceipt::CasHit(ref p)
            if p.effect_id == "e-hit" && p.bytes == b"(fleet-wire-v1 (round . 11))"
    ));
    assert!(matches!(
        rcpts2[1],
        AtomicEffectReceipt::SandboxExited(ref p)
            if p.effect_id == "e-sb-ok"
                && p.exit_code == 0
                && String::from_utf8_lossy(&p.stdout).trim() == "hello-atomic-sandbox"
    ));

    // Boot #2 CasPut on the exact same key must NOT collide on Turso visibility_receipts!
    let put2 = run_atomic_effect_batch(
        exec2,
        vec![AtomicEffect::CasPut(CasPutEffectPayload {
            effect_id: "e-put-boot2".to_owned(),
            key: "k:s11:wire".to_owned(),
            bytes: b"(fleet-wire-v1 (round . 16))".to_vec(),
        })],
    )
    .await;
    assert!(
        matches!(put2.as_slice(), [AtomicEffectReceipt::CasStored(id)] if id == "e-put-boot2")
    );
}

#[tokio::test]
async fn allowlist_sandbox_runner_enforces_program_allowlist_and_cwd_confinement() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sub = dir.path().join("work");
    std::fs::create_dir_all(&sub).expect("create work subdir");
    let outside = tempfile::tempdir().expect("outside tempdir");

    let runner = AllowlistSandboxRunner::new(Duration::from_secs(5), 4096, ["/bin/echo"])
        .with_confined_root(dir.path());

    // 1. Allowed program inside confined subdir succeeds.
    let ok = runner
        .run_sandbox(SandboxRunEffectPayload {
            effect_id: "sb-ok".to_owned(),
            program: "/bin/echo".to_owned(),
            args: vec!["confined-ok".to_owned()],
            cwd: Some("work".to_owned()),
        })
        .await;
    assert!(matches!(
        ok,
        AtomicEffectReceipt::SandboxExited(ref p)
            if p.effect_id == "sb-ok"
                && p.exit_code == 0
                && String::from_utf8_lossy(&p.stdout).trim() == "confined-ok"
    ));

    // 2. Disallowed program fails closed.
    let denied_prog = runner
        .run_sandbox(SandboxRunEffectPayload {
            effect_id: "sb-deny-prog".to_owned(),
            program: "/bin/ls".to_owned(),
            args: vec![],
            cwd: None,
        })
        .await;
    assert!(matches!(
        denied_prog,
        AtomicEffectReceipt::EffectError(ref p)
            if p.effect_id == "sb-deny-prog" && p.message.contains("allowlist")
    ));

    // 3. Escaping confined root via absolute outside path or `..` fails closed.
    let escaped_abs = runner
        .run_sandbox(SandboxRunEffectPayload {
            effect_id: "sb-escape-abs".to_owned(),
            program: "/bin/echo".to_owned(),
            args: vec!["nope".to_owned()],
            cwd: Some(outside.path().to_string_lossy().into_owned()),
        })
        .await;
    assert!(matches!(
        escaped_abs,
        AtomicEffectReceipt::EffectError(ref p)
            if p.effect_id == "sb-escape-abs" && p.message.contains("escapes confined root")
    ));

    let escaped_rel = runner
        .run_sandbox(SandboxRunEffectPayload {
            effect_id: "sb-escape-rel".to_owned(),
            program: "/bin/echo".to_owned(),
            args: vec!["nope".to_owned()],
            cwd: Some("..".to_owned()),
        })
        .await;
    assert!(matches!(
        escaped_rel,
        AtomicEffectReceipt::EffectError(ref p)
            if p.effect_id == "sb-escape-rel" && p.message.contains("escapes confined root")
    ));
}

#[cfg(feature = "ui-grpc")]
#[tokio::test]
async fn mygo_bridge_host_executes_atomic_effect_batch_over_mounted_turso_executor() {
    use marlin_agent_runtime::plugin::ReleaseTrust;
    use std::collections::BTreeMap;

    let unmounted =
        marlin_agent_app_server::ui_grpc::MyGoBridgeHost::new(ReleaseTrust::new(BTreeMap::new()), |_| Ok(()));
    let denied = unmounted
        .run_atomic_effects(vec![AtomicEffect::CasGet(CasGetEffectPayload {
            effect_id: "e-unmounted".to_owned(),
            key: "k:missing".to_owned(),
        })])
        .await
        .unwrap_err();
    assert_eq!(denied.code(), tonic::Code::FailedPrecondition);

    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = TursoAgentStorageConfig {
        path: dir.path().join("mygo-host-atomic.db"),
        optimization_profile: TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: TursoBatchTransactionMode::Concurrent,
    };
    let storage = TursoAgentStorage::open_local(cfg)
        .await
        .expect("open turso");
    let exec = Arc::new(CompositeAtomicEffectExecutor::new(
        storage,
        EchoModelGateway,
        ProjectId::new("proj-host-s13").unwrap(),
        SessionId::new("sess-host-s13").unwrap(),
        AgentId::new("mygo-host-s13").unwrap(),
    ));
    let host =
        marlin_agent_app_server::ui_grpc::MyGoBridgeHost::new(ReleaseTrust::new(BTreeMap::new()), |_| Ok(()))
            .with_atomic_executor(exec);

    let put_rcpts = host
        .run_atomic_effects(vec![
            AtomicEffect::CasPut(CasPutEffectPayload {
                effect_id: "h-put".to_owned(),
                key: "k:host:s13".to_owned(),
                bytes: b"host-wal-bytes".to_vec(),
            }),
            AtomicEffect::LlmComplete(LlmCompleteEffectPayload {
                effect_id: "h-llm".to_owned(),
                request: ModelGatewayRequest::new(
                    ModelEndpoint::new("openai", "gpt-marlin-echo"),
                    vec![user_gateway_message("host-ping")],
                ),
            }),
        ])
        .await
        .expect("host atomic batch");
    assert!(matches!(put_rcpts[0], AtomicEffectReceipt::CasStored(ref id) if id == "h-put"));
    assert!(matches!(
        put_rcpts[1],
        AtomicEffectReceipt::LlmCompleted(ref p) if p.content == "echo:host-ping"
    ));

    let get_rcpts = host
        .run_atomic_effects(vec![AtomicEffect::CasGet(CasGetEffectPayload {
            effect_id: "h-get".to_owned(),
            key: "k:host:s13".to_owned(),
        })])
        .await
        .expect("host cas get");
    assert!(matches!(
        get_rcpts.as_slice(),
        [AtomicEffectReceipt::CasHit(p)] if p.bytes == b"host-wal-bytes"
    ));
}

#[cfg(feature = "turso-host")]
#[tokio::test]
async fn allowlist_run_command_and_storage_cas_store_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let runner = AllowlistSandboxRunner::new(Duration::from_secs(5), 1024, ["/bin/echo"])
        .with_confined_root(dir.path());
    let (code, stdout, _) = runner
        .run_command(
            "/bin/echo".to_owned(),
            vec!["direct-ok".to_owned()],
            Some(dir.path().to_string_lossy().into_owned()),
        )
        .await
        .expect("allowlisted direct command must succeed");
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8_lossy(&stdout).trim(), "direct-ok");
    assert!(
        runner
            .run_command(
                "/bin/sh".to_owned(),
                vec!["-c".to_owned(), "true".to_owned()],
                None
            )
            .await
            .is_err()
    );
    assert!(
        runner
            .run_command(
                "/bin/echo".to_owned(),
                vec!["escape".to_owned()],
                Some("/".to_owned())
            )
            .await
            .is_err()
    );

    let cfg = TursoAgentStorageConfig {
        path: dir.path().join("cas-store-s27.db"),
        optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
        batch_transaction_mode: TursoBatchTransactionMode::Immediate,
    };
    let storage = TursoAgentStorage::open_local(cfg)
        .await
        .expect("open turso");
    let store = StorageCasStore::new(
        storage,
        ProjectId::new("proj-cas-s27").unwrap(),
        SessionId::new("sess-cas-s27").unwrap(),
        AgentId::new("agent-cas-s27").unwrap(),
    );
    assert_eq!(store.get_blob("k:s27").await.unwrap(), None);
    store
        .put_blob("k:s27", b"cas-s27-bytes".to_vec())
        .await
        .unwrap();
    assert_eq!(
        store.get_blob("k:s27").await.unwrap(),
        Some(b"cas-s27-bytes".to_vec())
    );
}

#[cfg(unix)]
#[tokio::test]
async fn sm20_bounded_sandbox_runner_caps_heap_before_truncate_and_avoids_pipe_deadlock() {
    // Spawn a real subprocess writing 2 MiB (32x the 64 KiB OS pipe buffer)
    // with max_output_bytes = 4096.
    // SM20 requires:
    // 1. Retained output <= 4096 bytes AND peak allocated Vec capacity <= 8192
    //    (proving bytes were NOT buffered to 2 MiB before post-hoc truncate()).
    // 2. Child exits cleanly within timeout (proving pipe was drained to sink
    //    rather than deadlocking the writer after 4096 bytes).
    let runner = BoundedTokioSandboxRunner::new(Duration::from_secs(5), 4096);
    let receipt = runner
        .run_sandbox(SandboxRunEffectPayload {
            effect_id: "sm20-pipe-flood".to_owned(),
            program: "/bin/sh".to_owned(),
            args: vec![
                "-c".to_owned(),
                "dd if=/dev/zero bs=65536 count=32 2>/dev/null | tr '\\0' 'A'".to_owned(),
            ],
            cwd: None,
        })
        .await;

    match receipt {
        AtomicEffectReceipt::SandboxExited(payload) => {
            assert_eq!(payload.exit_code, 0, "child must exit cleanly without pipe deadlock");
            assert_eq!(payload.stdout.len(), 4096);
            assert!(
                payload.stdout.capacity() <= 8192,
                "SM20 buffer-before-truncate OOM hazard: stdout.capacity() spiked to {} bytes for a 4096-byte cap!",
                payload.stdout.capacity()
            );
        }
        other => panic!("expected SandboxExited without timeout/deadlock, got {other:?}"),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn sm24_adv_l4_sandbox_runner_scrubs_ambient_secret_env_vars() {
    // ADV-L4 hostile exploit: ambient host credentials in the parent process
    // MUST NOT leak into sandboxed tool subprocesses via inherited environment.
    let secrets = [
        ("SM24_OPENAI_API_KEY", "sk-live-sm24-sandbox-exfil-1111"),
        ("SM24_MARLIN_BRIDGE_TOKEN", "marlin-bridge-secret-sm24-2222"),
        ("SM24_AWS_SECRET_ACCESS_KEY", "wJalrXUtnFEMI/SM24/SANDBOX3333"),
        ("SM24_CUSTOM_HOST_SECRET", "custom-ambient-secret-sm24-4444"),
    ];
    for (k, v) in secrets {
        // SAFETY: unique test-scoped env var names; cleaned up immediately below.
        unsafe {
            std::env::set_var(k, v);
        }
    }
    let runner = BoundedTokioSandboxRunner::new(Duration::from_secs(5), 4096);
    let receipt = runner
        .run_sandbox(SandboxRunEffectPayload {
            effect_id: "sm24-env-exfil-probe".to_owned(),
            program: "/bin/sh".to_owned(),
            args: vec![
                "-c".to_owned(),
                "printf '%s|%s|%s|%s' \"$SM24_OPENAI_API_KEY\" \"$SM24_MARLIN_BRIDGE_TOKEN\" \"$SM24_AWS_SECRET_ACCESS_KEY\" \"$SM24_CUSTOM_HOST_SECRET\"".to_owned(),
            ],
            cwd: None,
        })
        .await;
    for (k, _) in secrets {
        unsafe {
            std::env::remove_var(k);
        }
    }

    match receipt {
        AtomicEffectReceipt::SandboxExited(payload) => {
            assert_eq!(payload.exit_code, 0);
            let leaked = String::from_utf8_lossy(&payload.stdout);
            assert_eq!(
                leaked, "|||",
                "ADV-L4 SECRET EXFILTRATION PROVEN: sandboxed child inherited ambient host secrets: {leaked:?}"
            );
        }
        other => panic!("expected SandboxExited, got {other:?}"),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn sm25_adv_l4_prod_graph_executor_must_enforce_estop_and_cedar_governance_gate() {
    // ADV-L4 REAL-REPO SPLIT-BRAIN EXPLOIT:
    // Operator pulls E-Stop on GovernanceAdmissionGate (previously wired ONLY
    // onto the legacy LoopProgramRuntimeSideEffectExecutor, NEVER onto the
    // production GraphEngine CompositeAtomicEffectExecutor!).
    use marlin_agent_kernel::{
        ExecutionCircuitBreaker, GovernanceAdmissionGate, ToolBudgetUnits,
    };
    use marlin_agent_runtime::AtomicEffectExecutor;
    use marlin_agent_storage::InMemoryAgentStorage;

    let dir = tempfile::tempdir().expect("tempdir");
    let marker = dir.path().join("sm25-estop-bypassed.marker");

    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
    let gate = Arc::new(GovernanceAdmissionGate::new(breaker));
    gate.trip_operator_estop("operator-alice", "sm25-emergency-stop")
        .expect("trip operator estop");
    assert!(gate.snapshot().unwrap().breaker.tripped);

    // Production GraphEngine executor wired to the authoritative GovernanceAdmissionGate (SM25).
    use marlin_agent_kernel::{
        BreakerActor, BreakerActorId, CedarPolicyId, GOVERNANCE_ADMISSION_SCHEMA_VERSION,
        GovernanceProofDigest, GovernanceProofEnvelope, GovernanceProofEpoch,
        GovernedSubjectName, SynthesizedCedarPolicy,
    };
    let exec = CompositeAtomicEffectExecutor::new(
        InMemoryAgentStorage::default(),
        EchoModelGateway,
        ProjectId::new("proj-sm25").unwrap(),
        SessionId::new("sess-sm25").unwrap(),
        AgentId::new("agent-sm25").unwrap(),
    )
    .with_sandbox(BoundedTokioSandboxRunner::new(Duration::from_secs(5), 4096))
    .with_governance_gate(
        Arc::clone(&gate),
        GovernedSubjectName::new("agent-sm25").unwrap(),
        None,
    );

    // 1. Active operator E-Stop blocks BOTH SandboxRun AND CasPut fail-closed.
    let receipt = exec
        .execute_atomic(AtomicEffect::SandboxRun(SandboxRunEffectPayload {
            effect_id: "sm25-estop-probe".to_owned(),
            program: "/bin/sh".to_owned(),
            args: vec![
                "-c".to_owned(),
                format!("echo BYPASSED > '{}'", marker.display()),
            ],
            cwd: None,
        }))
        .await;

    assert!(
        !marker.exists(),
        "ADV-L4 DISCONNECTED GOVERNANCE BYPASS PROVEN: production CompositeAtomicEffectExecutor executed /bin/sh and wrote marker file despite active E-Stop! receipt={receipt:?}"
    );
    match &receipt {
        AtomicEffectReceipt::EffectError(err) => {
            assert!(
                err.message.contains("governance_admission_denied"),
                "expected governance_admission_denied diagnostic, got: {}",
                err.message
            );
        }
        other => panic!("expected EffectError under E-Stop, got {other:?}"),
    }

    let cas_blocked = exec
        .execute_atomic(AtomicEffect::CasPut(CasPutEffectPayload {
            effect_id: "sm25-cas-estop".to_owned(),
            key: "sm25:blocked".to_owned(),
            bytes: b"must-not-store".to_vec(),
        }))
        .await;
    assert!(matches!(cas_blocked, AtomicEffectReceipt::EffectError(_)));

    // 2. After human re-arm, Scheme-synthesized Cedar forbid STILL blocks /bin/sh fail-closed.
    let human = BreakerActor::VerifiedHuman(BreakerActorId::new("operator-alice").unwrap());
    gate.rearm_by_verified_human(human).expect("human rearm");
    let digest = GovernanceProofDigest::new(format!("sha256:{}", "b".repeat(64))).unwrap();
    gate.synchronize_proof_envelope(&GovernanceProofEnvelope {
        schema_version: GOVERNANCE_ADMISSION_SCHEMA_VERSION,
        admitted: true,
        revocation_epoch: GovernanceProofEpoch::new(2).unwrap(),
        policy_revision: 1,
        assessment_digest: digest.clone(),
        taint_proof_digest: digest.clone(),
        fixpoint_proof_digest: digest.clone(),
        temporal_proof_digest: digest.clone(),
        invalidation_digest: digest.clone(),
        rejection_reasons: Vec::new(),
        fenced_actors: Vec::new(),
        blocked_tools: Vec::new(),
        retracted_conclusions: Vec::new(),
        cedar_policies: vec![SynthesizedCedarPolicy {
            policy_id: CedarPolicyId::new("forbid-sm25-shell").unwrap(),
            source: r#"forbid(principal == Marlin::Actor::"agent-sm25", action == Marlin::Action::"atomic_effect:sandbox_run", resource);"#.to_owned(),
        }],
    })
    .expect("sync cedar forbid");

    let cedar_receipt = exec
        .execute_atomic(AtomicEffect::SandboxRun(SandboxRunEffectPayload {
            effect_id: "sm25-cedar-probe".to_owned(),
            program: "/bin/sh".to_owned(),
            args: vec![
                "-c".to_owned(),
                format!("echo CEDAR_BYPASSED > '{}'", marker.display()),
            ],
            cwd: None,
        }))
        .await;
    assert!(!marker.exists());
    assert!(matches!(cedar_receipt, AtomicEffectReceipt::EffectError(_)));

    // 3. Clean proof envelope permits governed execution and emits SandboxExited(0).
    gate.synchronize_proof_envelope(&GovernanceProofEnvelope {
        schema_version: GOVERNANCE_ADMISSION_SCHEMA_VERSION,
        admitted: true,
        revocation_epoch: GovernanceProofEpoch::new(3).unwrap(),
        policy_revision: 2,
        assessment_digest: digest.clone(),
        taint_proof_digest: digest.clone(),
        fixpoint_proof_digest: digest.clone(),
        temporal_proof_digest: digest.clone(),
        invalidation_digest: digest,
        rejection_reasons: Vec::new(),
        fenced_actors: Vec::new(),
        blocked_tools: Vec::new(),
        retracted_conclusions: Vec::new(),
        cedar_policies: Vec::new(),
    })
    .expect("sync clean envelope");

    let clean_receipt = exec
        .execute_atomic(AtomicEffect::SandboxRun(SandboxRunEffectPayload {
            effect_id: "sm25-clean-run".to_owned(),
            program: "/bin/sh".to_owned(),
            args: vec![
                "-c".to_owned(),
                format!("echo GOVERNED_OK > '{}'", marker.display()),
            ],
            cwd: None,
        }))
        .await;
    assert!(matches!(
        clean_receipt,
        AtomicEffectReceipt::SandboxExited(ref p) if p.exit_code == 0
    ));
    assert!(marker.exists());
}
