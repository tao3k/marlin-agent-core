// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM29 (ADV-L4) hostile exploit verification suite for `marlin-agent-kernel`:
//! 1. Cedar `forbid ... unless` missing context key fail-open bypass;
//! 2. Same-epoch `policy_revision` rollback wiping ASCENT fences & Cedar forbids;
//! 3. Same-epoch pre-trip durable checkpoint replay silently untripping active E-Stop;
//! 4. `rearm_by_verified_human` resurrecting permanently retracted Bi-Temporal TM derivations;
//! 5. `register_derived_lease` admitting leases while breaker is tripped or stale-epoch.

use std::{collections::BTreeMap, sync::Arc};

use marlin_agent_kernel::{
    BreakerActor, BreakerActorId, BreakerRejectionReason, BreakerTripReason, CedarContextValue,
    CedarEntityRef, CedarEvaluationDecision, CedarEvaluationRequest, CedarPolicyId, DerivationId,
    ExecutionCircuitBreaker, GOVERNANCE_ADMISSION_SCHEMA_VERSION, GovernanceAdmissionError,
    GovernanceAdmissionGate, GovernanceProofDigest, GovernanceProofEnvelope, GovernanceProofEpoch,
    GovernedSubjectName, SynthesizedCedarPolicy, ToolBudgetUnits, ToolExecutionLeaseId,
    ToolIfcContext, compile_cedar_policy, evaluate_compiled_cedar_policies,
};

fn make_digest(ch: char) -> GovernanceProofDigest {
    GovernanceProofDigest::new(format!("sha256:{}", std::iter::repeat_n(ch, 64).collect::<String>()))
        .expect("valid sha256 digest")
}

fn make_envelope(
    epoch: u64,
    policy_revision: u64,
    admitted: bool,
    fenced_actors: Vec<GovernedSubjectName>,
    retracted_conclusions: Vec<DerivationId>,
    cedar_policies: Vec<SynthesizedCedarPolicy>,
) -> GovernanceProofEnvelope {
    GovernanceProofEnvelope {
        schema_version: GOVERNANCE_ADMISSION_SCHEMA_VERSION,
        admitted,
        revocation_epoch: GovernanceProofEpoch::new(epoch).expect("valid epoch"),
        policy_revision,
        assessment_digest: make_digest('a'),
        taint_proof_digest: make_digest('b'),
        fixpoint_proof_digest: make_digest('c'),
        temporal_proof_digest: make_digest('d'),
        invalidation_digest: make_digest('e'),
        rejection_reasons: if admitted {
            Vec::new()
        } else {
            vec!["sm29-incident-denied".to_owned()]
        },
        fenced_actors,
        blocked_tools: Vec::new(),
        retracted_conclusions,
        cedar_policies,
    }
}

#[test]
fn sm29_adv_l4_cedar_forbid_unless_missing_context_key_must_fail_closed() {
    let permit = compile_cedar_policy(&SynthesizedCedarPolicy {
        policy_id: CedarPolicyId::new("marlin.permit.ops-export").unwrap(),
        source: "permit (principal in Marlin::Role::\"VerifiedHuman\", action == Marlin::Action::\"export\", resource) when { context.step_up_fresh == true };".to_owned(),
    })
    .unwrap();
    let forbid = compile_cedar_policy(&SynthesizedCedarPolicy {
        policy_id: CedarPolicyId::new("marlin.forbid.external-unless-ticket").unwrap(),
        source: "forbid (principal, action, resource in Marlin::Scope::\"external\") unless { context.declassified == true && context.approval_ticket == \"T-APPROVE-1\" };".to_owned(),
    })
    .unwrap();
    let policies = vec![permit, forbid];

    // Hostile attack: attacker sets `declassified = true` and `step_up_fresh = true`,
    // but OMITS `approval_ticket` from context so `eval_condition_block` returns `None`.
    // The `forbid` MUST remain active (Deny), NEVER silently waived into `Allow`!
    let hostile_req = CedarEvaluationRequest {
        principal: CedarEntityRef::actor("ops-alice"),
        principal_roles: vec![CedarEntityRef::role("VerifiedHuman")],
        action: CedarEntityRef::action("export"),
        resource: CedarEntityRef::resource("ext-webhook"),
        resource_groups: vec![CedarEntityRef::scope("external")],
        context: BTreeMap::from([
            ("step_up_fresh".to_owned(), CedarContextValue::Bool(true)),
            ("declassified".to_owned(), CedarContextValue::Bool(true)),
        ]),
    };
    let decision = evaluate_compiled_cedar_policies(&policies, &hostile_req);
    assert_eq!(
        decision,
        CedarEvaluationDecision::Deny {
            matching_forbids: vec![
                CedarPolicyId::new("marlin.forbid.external-unless-ticket").unwrap()
            ],
        },
        "SM29 ADV-L4 FAIL-OPEN EXPLOIT: omitting context.approval_ticket silently disabled Cedar forbid-unless and yielded {decision:?}"
    );

    // Control: supplying BOTH `declassified == true` AND `approval_ticket == "T-APPROVE-1"` genuinely waives forbid.
    let mut authorized_req = hostile_req;
    authorized_req.context.insert(
        "approval_ticket".to_owned(),
        CedarContextValue::Str("T-APPROVE-1".to_owned()),
    );
    assert!(matches!(
        evaluate_compiled_cedar_policies(&policies, &authorized_req),
        CedarEvaluationDecision::Allow { .. }
    ));

    // Control 2: `permit ... unless { context.channel_blocked == true }` with missing `channel_blocked` MUST fail closed!
    let permit_unless = compile_cedar_policy(&SynthesizedCedarPolicy {
        policy_id: CedarPolicyId::new("marlin.permit.unless-blocked").unwrap(),
        source: "permit (principal, action, resource) unless { context.channel_blocked == true };"
            .to_owned(),
    })
    .unwrap();
    let missing_permit_ctx = CedarEvaluationRequest {
        principal: CedarEntityRef::actor("ops-alice"),
        principal_roles: Vec::new(),
        action: CedarEntityRef::action("export"),
        resource: CedarEntityRef::resource("workspace"),
        resource_groups: Vec::new(),
        context: BTreeMap::new(),
    };
    assert_eq!(
        evaluate_compiled_cedar_policies(&[permit_unless], &missing_permit_ctx),
        CedarEvaluationDecision::ImplicitDeny
    );
}

#[test]
fn sm29_adv_l4_same_epoch_policy_revision_rollback_cannot_wipe_fences_or_forbids() {
    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
    let gate = GovernanceAdmissionGate::new(breaker);
    let rogue = GovernedSubjectName::new("agent-rogue").unwrap();

    // Install epoch=2, policy_revision=5 with ASCENT fence + Cedar forbid for `agent-rogue`.
    let latest_env = make_envelope(
        2,
        5,
        true,
        vec![rogue.clone()],
        Vec::new(),
        vec![SynthesizedCedarPolicy {
            policy_id: CedarPolicyId::new("marlin.forbid.rogue").unwrap(),
            source: "forbid (principal == Marlin::Actor::\"agent-rogue\", action, resource);"
                .to_owned(),
        }],
    );
    gate.synchronize_proof_envelope(&latest_env).unwrap();
    assert!(gate.is_actor_fenced(&rogue));

    // Hostile attack: replay stale same-epoch envelope (epoch=2, policy_revision=1) with EMPTY fences!
    let stale_rev_env = make_envelope(2, 1, true, Vec::new(), Vec::new(), Vec::new());
    let err = gate
        .synchronize_proof_envelope(&stale_rev_env)
        .expect_err("SM29 ADV-L4 ROLLBACK EXPLOIT: same-epoch lower policy_revision wiped active fences!");
    assert!(
        matches!(err, GovernanceAdmissionError::ProofEnvelopeRejected(_)),
        "expected ProofEnvelopeRejected on same-epoch revision rollback, got {err:?}"
    );
    assert!(
        gate.is_actor_fenced(&rogue),
        "fenced actor must remain fenced after rejected rollback"
    );
    assert_eq!(gate.snapshot().unwrap().policy_revision, 5);
}

#[test]
fn sm29_adv_l4_same_epoch_pre_trip_checkpoint_replay_cannot_untrip_active_estop() {
    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
    let gate = GovernanceAdmissionGate::new(Arc::clone(&breaker));
    let actor = GovernedSubjectName::new("ops-bob").unwrap();
    let tool = GovernedSubjectName::new("os-shell").unwrap();

    // 1. Advance to epoch=2 armed and capture a valid pre-trip durable checkpoint at trip_epoch=1.
    gate.synchronize_proof_envelope(&make_envelope(
        2,
        1,
        true,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ))
    .unwrap();
    let pre_trip_bytes = gate.export_durable_checkpoint_bytes().unwrap();

    // 2. Operator triggers live E-Stop!
    let tripped_snap = gate
        .trip_operator_estop("ops-alice", "active-incident")
        .unwrap();
    assert!(tripped_snap.breaker.tripped);

    // 3. Hostile attack: replay the legitimately signed pre-trip MGV1 checkpoint at the SAME epoch!
    let err = gate
        .restore_durable_checkpoint_bytes(&pre_trip_bytes)
        .expect_err("SM29 ADV-L4 UNTRIP EXPLOIT: pre-trip same-epoch checkpoint replay silently cleared active E-Stop!");
    assert!(
        matches!(
            err,
            GovernanceAdmissionError::ExecutionBreaker(BreakerRejectionReason::BreakerTripped { .. })
        ),
        "expected BreakerTripped refusal on same-epoch pre-trip checkpoint replay, got {err:?}"
    );
    assert!(
        gate.snapshot().unwrap().breaker.tripped,
        "E-Stop latch MUST remain tripped after rejected checkpoint replay"
    );
    assert!(
        gate.authorize_side_effect(None, &actor, &tool, &ToolIfcContext::clean_trusted())
            .is_err()
    );
}

#[test]
fn sm29_adv_l4_human_rearm_preserves_retracted_derivations_and_blocks_resurrection() {
    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
    let gate = GovernanceAdmissionGate::new(Arc::clone(&breaker));
    let revoked_deriv = DerivationId::new("D-REVOKED-PREMISE").unwrap();
    let actor = GovernedSubjectName::new("ops-bob").unwrap();
    let tool = GovernedSubjectName::new("file-write").unwrap();

    // 1. Bi-Temporal TM retracts `D-REVOKED-PREMISE` and trips breaker.
    let env = make_envelope(
        2,
        1,
        false,
        Vec::new(),
        vec![revoked_deriv.clone()],
        Vec::new(),
    );
    gate.synchronize_proof_envelope(&env).unwrap();
    assert!(gate.is_derivation_retracted(&revoked_deriv));

    // 2. Verified human re-arms breaker to resume clean operations on NEW derivations.
    let human = BreakerActor::VerifiedHuman(BreakerActorId::new("ops-alice").unwrap());
    let (_receipt, snap) = gate.rearm_by_verified_human(human).unwrap();
    assert!(!snap.breaker.tripped);

    // 3. Hostile attack: attempt to resurrect `D-REVOKED-PREMISE` after human re-arm!
    assert!(
        gate.is_derivation_retracted(&revoked_deriv),
        "SM29 ADV-L4 RESURRECTION EXPLOIT: rearm_by_verified_human wiped retracted_derivations!"
    );
    let err = gate
        .authorize_side_effect(
            Some(&revoked_deriv),
            &actor,
            &tool,
            &ToolIfcContext::clean_trusted(),
        )
        .expect_err("retracted derivation MUST remain denied after human re-arm");
    assert!(matches!(
        err,
        GovernanceAdmissionError::DerivationRetractedDenied(_)
    ));

    // 4. Registering a derived lease for `D-REVOKED-PREMISE` MUST also fail closed.
    let lease = breaker
        .acquire_lease(ToolExecutionLeaseId::new("lease-resurrect").unwrap(), "file-write")
        .unwrap();
    let reg_err = gate
        .register_derived_lease(revoked_deriv, lease)
        .expect_err("register_derived_lease must deny permanently retracted derivation");
    assert!(matches!(
        reg_err,
        GovernanceAdmissionError::DerivationRetractedDenied(_)
    ));
}

#[test]
fn sm29_adv_l4_register_derived_lease_rejects_tripped_breaker_and_stale_epoch() {
    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
    let gate = GovernanceAdmissionGate::new(Arc::clone(&breaker));

    let lease = breaker
        .acquire_lease(ToolExecutionLeaseId::new("lease-pre-trip").unwrap(), "tool-a")
        .unwrap();
    breaker
        .trip(BreakerTripReason::new("estop-before-register").unwrap())
        .unwrap();

    // Registering a pre-trip lease while breaker is tripped MUST fail closed!
    let err = gate
        .register_derived_lease(DerivationId::new("D-PRE-TRIP").unwrap(), lease.clone())
        .expect_err("SM29 ADV-L4: register_derived_lease must reject leases while breaker is tripped");
    assert!(matches!(
        err,
        GovernanceAdmissionError::ExecutionBreaker(BreakerRejectionReason::BreakerTripped { .. })
    ));

    // After human re-arm advances trip_epoch 1 -> 2, registering the stale epoch-1 lease MUST fail closed!
    let human = BreakerActor::VerifiedHuman(BreakerActorId::new("ops-alice").unwrap());
    gate.rearm_by_verified_human(human).unwrap();
    let stale_err = gate
        .register_derived_lease(DerivationId::new("D-STALE-EPOCH").unwrap(), lease)
        .expect_err("SM29 ADV-L4: register_derived_lease must reject stale-epoch leases");
    assert!(matches!(
        stale_err,
        GovernanceAdmissionError::ExecutionBreaker(BreakerRejectionReason::StaleLeaseEpoch { .. })
    ));
}
