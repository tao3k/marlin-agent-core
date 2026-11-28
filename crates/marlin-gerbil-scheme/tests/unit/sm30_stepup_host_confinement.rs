// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM30 (ADV-L4) hostile exploit suite for `marlin-gerbil-scheme` governance host:
//! 1. RFC 9470 Step-Up MFA attestation spoofing (worker-*/scheme-sandbox-*, weak acr=none/password,
//!    duplicate actor=/acr= parameter smuggling) must NEVER yield VerifiedHuman or re-arm E-Stop;
//! 2. Runtime fact-graph mutations must enforce exact capacity parity with MGF1's 4096-item
//!    deserialization cap (preventing self-inflicted restart deserialization bombs);
//! 3. Durable Step-Up Re-Arm must fail closed (re-trip breaker) if Turso CAS persistence fails
//!    instead of swallowing errors via `let _ =`.

use std::sync::Arc;

use marlin_agent_app_server::{
    StorageCasStore,
    ui_grpc::MyGoBridgeHost,
    wire::marlin::gate::v1::GraphHumanDecision,
};
use marlin_agent_kernel::BreakerActor;
use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId, SessionId};
use marlin_gerbil_scheme::governance_host::{
    GovernanceFactStore, GovernanceHostController, GovernanceUiCommand, SharedProjectionRecords,
    parse_verified_step_up_actor,
};

#[test]
fn sm30_adv_l4_stepup_parser_rejects_worker_sandbox_weak_acr_and_duplicate_params() {
    // 1. Agent-role prefixes from `infer_actor_roles` (`worker-*`, `scheme-sandbox-*`, `system-*`)
    //    MUST NOT be classified as `VerifiedHuman`!
    for agent_note in [
        "stepup-verified:actor=worker-autonomous:acr=urn:ietf:params:oauth:acr:mfa",
        "stepup-verified:actor=scheme-sandbox-worker:acr=urn:ietf:params:oauth:acr:mfa",
        "stepup-verified:actor=system-daemon:acr=urn:ietf:params:oauth:acr:phr",
        "stepup-verified:actor=service-mesh:acr=urn:ietf:params:oauth:acr:mfa",
    ] {
        let parsed = parse_verified_step_up_actor(Some(agent_note)).unwrap();
        assert!(
            matches!(parsed, BreakerActor::Agent(_)),
            "SM30 ADV-L4 AGENT IMPERSONATION: `{agent_note}` was classified as {parsed:?} instead of Agent"
        );
    }

    // 2. Weak/bogus `acr=` strings MUST NOT pass as `VerifiedHuman`!
    for weak_note in [
        "stepup-verified:actor=ops-alice:acr=none",
        "stepup-verified:actor=ops-alice:acr=password",
        "stepup-verified:actor=ops-alice:acr=0",
        "stepup-verified:actor=ops-alice:acr=fake",
        "stepup-verified:actor=ops-alice:acr=urn:untrusted:low",
    ] {
        let parsed = parse_verified_step_up_actor(Some(weak_note)).unwrap();
        assert!(
            matches!(parsed, BreakerActor::UnverifiedActor(_)),
            "SM30 ADV-L4 WEAK ACR BYPASS: `{weak_note}` was classified as {parsed:?} instead of UnverifiedActor"
        );
    }

    // 3. Duplicate `actor=` or `acr=` parameter smuggling MUST fail closed!
    for smuggled in [
        "stepup-verified:actor=agent-rogue:actor=ops-alice:acr=urn:ietf:params:oauth:acr:mfa",
        "stepup-verified:actor=ops-alice:acr=urn:ietf:params:oauth:acr:mfa:acr=none",
        "stepup-verified:junk:actor=ops-alice:acr=urn:ietf:params:oauth:acr:mfa",
    ] {
        let parsed = parse_verified_step_up_actor(Some(smuggled)).unwrap();
        assert!(
            !matches!(parsed, BreakerActor::VerifiedHuman(_)),
            "SM30 ADV-L4 PARAM SMUGGLING: `{smuggled}` was classified as {parsed:?}"
        );
    }

    // 4. Legitimate RFC 9470 / NIST MFA URNs continue to pass cleanly.
    for valid in [
        "stepup-verified:actor=alice:acr=urn:ietf:params:oauth:acr:mfa",
        "stepup-verified:actor=ops-alice:acr=urn:ietf:params:oauth:acr:phr",
        "stepup-verified:actor=ops-alice:acr=urn:ietf:params:oauth:acr:phrh",
        "stepup-verified:actor=ops-alice:acr=urn:nist:800-63:aal:3",
    ] {
        let parsed = parse_verified_step_up_actor(Some(valid)).unwrap();
        assert!(
            matches!(parsed, BreakerActor::VerifiedHuman(_)),
            "valid RFC 9470 note `{valid}` must classify as VerifiedHuman, got {parsed:?}"
        );
    }
}

#[test]
fn sm30_adv_l4_host_rearm_rejects_spoofed_worker_and_weak_acr_end_to_end() {
    let host = MyGoBridgeHost::new_default_trust(|_| Ok(()));
    let ctrl = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
    ctrl.apply_operator_estop(&host, "ops-alice", "sm30-estop")
        .unwrap();
    assert!(ctrl.gate().snapshot().unwrap().breaker.tripped);

    // Hostile attack: `worker-autonomous` with `acr=none` attempts to re-arm E-Stop!
    assert!(ctrl.handle_claimed_gate_decision(
        &host,
        "gov-rearm-epoch-1",
        GraphHumanDecision::Approve as i32,
        Some("stepup-verified:actor=worker-autonomous:acr=none"),
    ));
    assert!(
        ctrl.gate().snapshot().unwrap().breaker.tripped,
        "SM30 ADV-L4 RE-ARM EXPLOIT: spoofed worker-autonomous + acr=none un-tripped active E-Stop!"
    );
}

#[test]
fn sm30_adv_l4_fact_store_mutation_enforces_mgf1_deserialization_cap_parity() {
    let ctrl = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
    for i in 0..4096usize {
        ctrl.mutate_facts_and_build_request(&GovernanceUiCommand::RevokeActor {
            actor: format!("revoked-actor-{i}"),
        })
        .expect("mutations within 4096 cap must succeed");
    }

    // 4,097th unique revoked actor MUST be rejected BEFORE overflowing MGF1's 4096 deserialization cap!
    let overflow = ctrl.mutate_facts_and_build_request(&GovernanceUiCommand::RevokeActor {
        actor: "revoked-actor-overflow-4096".to_owned(),
    });
    assert!(
        overflow.is_err(),
        "SM30 ADV-L4 DESER BOMB: 4097th fact mutation succeeded and would brick MGF1 restore on reboot!"
    );

    // Verify exported MGF1 blob at exact cap=4096 still round-trips cleanly!
    let (req, _, _) = ctrl
        .mutate_facts_and_build_request(&GovernanceUiCommand::RevokeActor {
            actor: "revoked-actor-0".to_owned(),
        })
        .expect("idempotent re-insertion of existing fact at cap must succeed")
        .unwrap();
    let mut store = GovernanceFactStore::default();
    store.reset_from_request(&req);
    let blob = store.to_canonical_bytes();
    assert!(
        GovernanceFactStore::from_canonical_bytes(&blob).is_ok(),
        "MGF1 blob at cap=4096 must deserialize cleanly"
    );
}

#[derive(Clone, Default)]
struct FailingPutArtifactStorage(InMemoryAgentStorage);

impl marlin_agent_storage::SessionEventStore for FailingPutArtifactStorage {
    fn latest_session_event_header<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a SessionId,
        from_turn: &'a marlin_agent_storage::TurnId,
        before_turn: &'a marlin_agent_storage::TurnId,
    ) -> marlin_agent_storage::StorageFuture<'a, Option<marlin_agent_storage::SessionEventHeader>>
    {
        self.0
            .latest_session_event_header(project, session, from_turn, before_turn)
    }

    fn count_session_events<'a>(
        &'a self,
        project: &'a ProjectId,
        session: &'a SessionId,
    ) -> marlin_agent_storage::StorageFuture<'a, marlin_agent_storage::SessionEventCount> {
        self.0.count_session_events(project, session)
    }

    fn list_sessions_page<'a>(
        &'a self,
        request: marlin_agent_storage::SessionPageRequest,
    ) -> marlin_agent_storage::StorageFuture<
        'a,
        marlin_agent_storage::StoragePage<SessionId, SessionId>,
    > {
        self.0.list_sessions_page(request)
    }

    fn append_session_event<'a>(
        &'a self,
        record: marlin_agent_storage::SessionEventRecord,
    ) -> marlin_agent_storage::StorageFuture<'a, ()> {
        self.0.append_session_event(record)
    }

    fn append_session_events_atomically<'a>(
        &'a self,
        records: Vec<marlin_agent_storage::SessionEventRecord>,
    ) -> marlin_agent_storage::StorageFuture<
        'a,
        marlin_agent_storage::SessionEventBatchWriteReceipt,
    > {
        self.0.append_session_events_atomically(records)
    }

    fn list_session_events_page<'a>(
        &'a self,
        request: marlin_agent_storage::SessionEventPageRequest,
    ) -> marlin_agent_storage::StorageFuture<
        'a,
        marlin_agent_storage::StoragePage<
            marlin_agent_storage::SessionEventRecord,
            marlin_agent_storage::SessionEventCursor,
        >,
    > {
        self.0.list_session_events_page(request)
    }
}

impl marlin_agent_storage::AgentStorage for FailingPutArtifactStorage {
    fn put_artifact<'a>(
        &'a self,
        _record: marlin_agent_storage::ArtifactRecord,
    ) -> marlin_agent_storage::StorageFuture<'a, marlin_agent_storage::ArtifactPutOutcome> {
        Box::pin(async {
            Err(marlin_agent_storage::StorageError::Backend {
                message: "injected cas disk full".to_owned(),
            })
        })
    }

    fn get_artifact<'a>(
        &'a self,
        project_id: &'a ProjectId,
        artifact_hash: &'a marlin_agent_storage::ArtifactHash,
    ) -> marlin_agent_storage::StorageFuture<'a, Option<marlin_agent_storage::ArtifactRecord>> {
        self.0.get_artifact(project_id, artifact_hash)
    }

    fn compare_and_swap_artifact_pointer<'a>(
        &'a self,
        update: marlin_agent_storage::ArtifactPointerUpdate,
    ) -> marlin_agent_storage::StorageFuture<'a, marlin_agent_storage::ArtifactPointerRecord> {
        self.0.compare_and_swap_artifact_pointer(update)
    }

    fn get_artifact_pointer<'a>(
        &'a self,
        project_id: &'a ProjectId,
        pointer_key: &'a marlin_agent_storage::ArtifactPointerKey,
    ) -> marlin_agent_storage::StorageFuture<'a, Option<marlin_agent_storage::ArtifactPointerRecord>>
    {
        self.0.get_artifact_pointer(project_id, pointer_key)
    }

    fn record_visibility<'a>(
        &'a self,
        receipt: marlin_agent_storage::VisibilityReceipt,
    ) -> marlin_agent_storage::StorageFuture<'a, ()> {
        self.0.record_visibility(receipt)
    }

    fn list_visibility_page<'a>(
        &'a self,
        request: marlin_agent_storage::VisibilityPageRequest,
    ) -> marlin_agent_storage::StorageFuture<
        'a,
        marlin_agent_storage::StoragePage<
            marlin_agent_storage::VisibilityReceipt,
            marlin_agent_storage::VisibilityCursor,
        >,
    > {
        self.0.list_visibility_page(request)
    }

    fn put_memory_proposal<'a>(
        &'a self,
        record: marlin_agent_storage::MemoryProposalRecord,
    ) -> marlin_agent_storage::StorageFuture<'a, ()> {
        self.0.put_memory_proposal(record)
    }

    fn list_memory_proposals_page<'a>(
        &'a self,
        request: marlin_agent_storage::MemoryProposalPageRequest,
    ) -> marlin_agent_storage::StorageFuture<
        'a,
        marlin_agent_storage::StoragePage<
            marlin_agent_storage::MemoryProposalRecord,
            marlin_agent_storage::MemoryProposalCursor,
        >,
    > {
        self.0.list_memory_proposals_page(request)
    }

    fn append_topology_edge<'a>(
        &'a self,
        record: marlin_agent_storage::TopologyEdgeRecord,
    ) -> marlin_agent_storage::StorageFuture<'a, ()> {
        self.0.append_topology_edge(record)
    }

    fn list_topology_edges_page<'a>(
        &'a self,
        request: marlin_agent_storage::TopologyEdgePageRequest,
    ) -> marlin_agent_storage::StorageFuture<
        'a,
        marlin_agent_storage::StoragePage<
            marlin_agent_storage::TopologyEdgeRecord,
            marlin_agent_storage::TopologyEdgeCursor,
        >,
    > {
        self.0.list_topology_edges_page(request)
    }
}

#[tokio::test]
async fn sm30_adv_l4_durable_rearm_fails_closed_when_cas_persistence_errors() {
    let cas = StorageCasStore::new(
        FailingPutArtifactStorage::default(),
        ProjectId::new("default").unwrap(),
        SessionId::new("sess-sm30-fail").unwrap(),
        AgentId::new("agent-sm30-fail").unwrap(),
    );
    let host = MyGoBridgeHost::new_default_trust(|_| Ok(()));
    let ctrl = GovernanceHostController::new(8, Arc::new(SharedProjectionRecords::new()));
    ctrl.apply_operator_estop(&host, "ops-alice", "pre-fail-estop")
        .unwrap();
    assert!(ctrl.gate().snapshot().unwrap().breaker.tripped);

    // Inject genuine `StorageError::Backend("injected cas disk full")` on durable CAS write!
    // Host MUST NOT swallow persist error via `let _ =` and leave live breaker armed while WAL is stale!
    assert!(
        ctrl.handle_claimed_gate_decision_durable(
            &host,
            Some(&cas),
            Some(marlin_gerbil_scheme::governance_host::DEFAULT_GOVERNANCE_CHECKPOINT_KEY),
            "gov-rearm-epoch-1",
            GraphHumanDecision::Approve as i32,
            Some("stepup-verified:actor=ops-alice:acr=urn:ietf:params:oauth:acr:mfa"),
        )
        .await
    );
    assert!(
        ctrl.gate().snapshot().unwrap().breaker.tripped,
        "SM30 ADV-L4 PERSIST DESYNC: handle_claimed_gate_decision_durable swallowed CAS failure and left live breaker armed!"
    );
}

#[tokio::test]
async fn sm30_adv_l4_scheme_atomic_effect_blocks_estop_llm_and_reserved_governance_cas_put() {
    use marlin_agent_protocol::{ModelEndpoint, ModelGatewayRequest};
    use marlin_gerbil_scheme::{
        SchemeAtomicEffect, SchemeAtomicReceipt,
        governance_host::DEFAULT_GOVERNANCE_CHECKPOINT_KEY,
        scheme_host_runtime::{is_reserved_governance_cas_key, run_one_atomic_effect},
    };

    let host = MyGoBridgeHost::new_default_trust(|_| Ok(()));
    let ctrl = Arc::new(GovernanceHostController::new(
        8,
        Arc::new(SharedProjectionRecords::new()),
    ));
    let loopback: Option<Arc<str>> = Some(Arc::from("loopback-llm-answer"));
    let req = || {
        ModelGatewayRequest::new(ModelEndpoint::new("openai", "gpt-4o-mini"), Vec::new())
    };

    // 1. Armed: LlmComplete succeeds via loopback.
    let armed_rcpt = run_one_atomic_effect(
        None,
        None,
        Arc::clone(&ctrl),
        None,
        loopback.clone(),
        SchemeAtomicEffect::LlmComplete {
            effect_id: "eff-llm-armed".to_owned(),
            request: req(),
        },
    )
    .await;
    assert!(matches!(
        armed_rcpt,
        SchemeAtomicReceipt::LlmCompleted { .. }
    ));

    // 2. Active E-Stop: LlmComplete MUST fail closed (EffectError), NEVER returning LLM output!
    ctrl.apply_operator_estop(&host, "ops-alice", "sm30-llm-estop")
        .unwrap();
    let estop_rcpt = run_one_atomic_effect(
        None,
        None,
        Arc::clone(&ctrl),
        None,
        loopback,
        SchemeAtomicEffect::LlmComplete {
            effect_id: "eff-llm-estop".to_owned(),
            request: req(),
        },
    )
    .await;
    assert!(
        matches!(estop_rcpt, SchemeAtomicReceipt::EffectError { ref message, .. } if message.contains("governance denied")),
        "SM30 ADV-L4 E-STOP LLM BYPASS: expected governance denial, got {estop_rcpt:?}"
    );

    // 3. CasPut targeting reserved governance checkpoint keys MUST fail closed!
    for reserved in [
        DEFAULT_GOVERNANCE_CHECKPOINT_KEY,
        "checkpoint:governance-authority:v1:facts",
        "checkpoint:governance-custom",
        "tenant:custom:facts",
    ] {
        assert!(is_reserved_governance_cas_key(reserved));
        let cas_rcpt = run_one_atomic_effect(
            None,
            None,
            Arc::clone(&ctrl),
            None,
            None,
            SchemeAtomicEffect::CasPut {
                effect_id: "eff-cas-hijack".to_owned(),
                key: reserved.to_owned(),
                bytes: b"forged-clean-mgv1".to_vec(),
            },
        )
        .await;
        assert!(
            matches!(cas_rcpt, SchemeAtomicReceipt::EffectError { ref message, .. } if message.contains("cas-put-denied")),
            "SM30 ADV-L4 RESERVED CAS OVERWRITE: key {reserved} was not denied: {cas_rcpt:?}"
        );
    }
    assert!(!is_reserved_governance_cas_key("checkpoint:scheme-fleet:v1"));
}
