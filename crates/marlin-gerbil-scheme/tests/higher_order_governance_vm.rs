// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Live Embedded Gambit/Gerbil VM -> Zero-JSON Rooted Vector -> Rust Kernel E2E Verification!
//!
//! Executes REAL Gerbil Scheme `scheme/policy/higher-order-governance.ss`
//! (Atomic POO × `gerbil-ascent` Stratified Taint & Multi-Hop Delegation Fixpoint ×
//! `agentic-ai-lab/modules/temporal-causality` Bi-Temporal Truth Maintenance ×
//! Dynamic Cedar Policy Synthesis) inside the live embedded `GerbilRuntime`,
//! decodes the 14-slot rooted Gambit heap vector with ZERO JSON serialization,
//! and drives Rust `HigherOrderGovernanceAdmissionGate` + `ExecutionCircuitBreaker` +
//! `LoopProgramRuntimeSideEffectExecutor` to physically block/allow real `/bin/sh` OS spawns.

use std::sync::Arc;

use gerbil_scheme::{GerbilRuntime, IdentifierKind, PackageResolutionProgress};
use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, BreakerActor, BreakerActorId, CedarPolicyId,
    DerivationId, ExecutionCircuitBreaker, GenericLoopMachineReceipt, GenericLoopMachineStepIndex,
    GovernanceProofDigest, GovernedSubjectName, HigherOrderBridgeError,
    HigherOrderGovernanceAdmissionGate, HigherOrderGovernanceProofEnvelope, HigherOrderProofEpoch,
    LoopProgramRuntimeHandoffExecutor, LoopProgramRuntimeHandoffPlan, LoopProgramRuntimeOwner,
    LoopProgramRuntimeSideEffectExecutor, LoopProgramRuntimeSideEffectStatus,
    LoopProgramToolProcessCommandTemplate, LoopProgramToolProcessProgram,
    StaticLoopProgramToolProcessResolver, SynthesizedCedarPolicy, ToolBudgetUnits,
    ToolExecutionLeaseId, ToolIfcContext,
};
use marlin_agent_protocol::{
    LoopProgramActionKind, LoopProgramEventKind, LoopProgramId, LoopProgramStateId,
    LoopProgramTransitionId,
};
use marlin_agent_runtime::TokioAgentRuntime;
use marlin_gerbil_scheme::{
    HigherOrderGovernanceVectorProjection, HigherOrderGovernanceVectorRequest,
    MARLIN_HIGHER_ORDER_GOVERNANCE_PROJECT_SYMBOL, decode_higher_order_governance_vector,
    encode_higher_order_governance_request, gerbil_package_root,
};

fn into_kernel_envelope(
    proj: &HigherOrderGovernanceVectorProjection,
) -> HigherOrderGovernanceProofEnvelope {
    HigherOrderGovernanceProofEnvelope {
        schema_version: proj.schema_version,
        admitted: proj.admitted,
        revocation_epoch: HigherOrderProofEpoch::new(proj.revocation_epoch).unwrap(),
        policy_revision: proj.policy_revision,
        assessment_digest: GovernanceProofDigest::new(proj.assessment_digest.clone()).unwrap(),
        taint_proof_digest: GovernanceProofDigest::new(proj.taint_proof_digest.clone()).unwrap(),
        fixpoint_proof_digest: GovernanceProofDigest::new(proj.fixpoint_proof_digest.clone())
            .unwrap(),
        temporal_proof_digest: GovernanceProofDigest::new(proj.temporal_proof_digest.clone())
            .unwrap(),
        invalidation_digest: GovernanceProofDigest::new(proj.invalidation_digest.clone()).unwrap(),
        rejection_reasons: proj.rejection_reasons.clone(),
        fenced_actors: proj
            .fenced_actors
            .iter()
            .map(|s| GovernedSubjectName::new(s.clone()).unwrap())
            .collect(),
        blocked_tools: proj
            .blocked_tools
            .iter()
            .map(|s| GovernedSubjectName::new(s.clone()).unwrap())
            .collect(),
        retracted_conclusions: proj
            .retracted_conclusions
            .iter()
            .map(|s| DerivationId::new(s.clone()).unwrap())
            .collect(),
        cedar_policies: proj
            .cedar_policies
            .iter()
            .map(|(id, src)| SynthesizedCedarPolicy {
                policy_id: CedarPolicyId::new(id.clone()).unwrap(),
                source: src.clone(),
            })
            .collect(),
    }
}

fn dispatch_handoff() -> marlin_agent_kernel::LoopProgramRuntimeHandoffExecutionReceipt {
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("vm-ho-gov-e2e"),
        &[GenericLoopMachineReceipt {
            program_id: LoopProgramId::new("vm-ho-gov-e2e"),
            step_index: GenericLoopMachineStepIndex::new(1),
            transition_id: LoopProgramTransitionId::new("t-1"),
            from: LoopProgramStateId::new("start"),
            event: LoopProgramEventKind::RuntimeReceipt,
            action: LoopProgramActionKind::DispatchTools,
            to: LoopProgramStateId::new("done"),
            stopped: false,
        }],
    );
    AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
        "agent-flow-runtime",
    ))
    .execute_plan(&plan)
}

#[cfg(unix)]
#[tokio::test]
async fn live_gerbil_vm_higher_order_governance_drives_rust_breaker_and_blocks_real_os_spawn() {
    let runtime = GerbilRuntime::initialize().expect("initialize embedded Gambit/Gerbil VM");
    let _ = runtime.set_package_resolution_progress(PackageResolutionProgress::Quiet);

    let scheme_dir = gerbil_package_root();
    let scheme_dir_str = scheme_dir.to_str().expect("utf8 scheme dir");
    let module_sym = runtime
        .intern_identifier(
            IdentifierKind::Symbol,
            ":marlin-agent-core/policy/higher-order-governance",
        )
        .unwrap();
    let proc_sym = runtime
        .intern_identifier(
            IdentifierKind::Symbol,
            MARLIN_HIGHER_ORDER_GOVERNANCE_PROJECT_SYMBOL,
        )
        .unwrap();
    let scheme_project = runtime
        .resolve_package_export(scheme_dir_str, &module_sym, &proc_sym)
        .expect("resolve live Scheme marlin-higher-order-governance-project-vector");

    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(4)));
    let gate = HigherOrderGovernanceAdmissionGate::new(Arc::clone(&breaker));

    let temp_dir = tempfile::tempdir().expect("temp workspace");
    let sentinel = temp_dir.path().join("vm-gov-os-process.sentinel");
    let resolver = StaticLoopProgramToolProcessResolver::new(
        vec![
            LoopProgramToolProcessCommandTemplate::new(
                "agent-flow.tool-intent",
                ["loop-program.dispatch-tools"],
                LoopProgramToolProcessProgram::new("/bin/sh"),
            )
            .with_args(["-c".to_owned(), format!("touch '{}'", sentinel.display())]),
        ]
        .into_boxed_slice(),
    );
    let executor = LoopProgramRuntimeSideEffectExecutor::new(resolver)
        .with_execution_breaker(Arc::clone(&breaker));
    let handoff = dispatch_handoff();
    let (tokio_rt, _events) = TokioAgentRuntime::new(4);

    // Pre-register two active causal derivations in Rust before retraction arrives from Scheme:
    //   - D-TOOL-EXEC (depends on human-session-alice via D-OBO-TICKET -> WILL BE RETRACTED!)
    //   - D-PUBLIC-DOC (depends only on public-workspace-policy -> WILL SURVIVE!)
    let d_tool = DerivationId::new("D-TOOL-EXEC").unwrap();
    let d_pub = DerivationId::new("D-PUBLIC-DOC").unwrap();
    let lease_tool = breaker
        .acquire_lease(ToolExecutionLeaseId::new("lease-tool-1").unwrap(), "os-shell")
        .unwrap();
    let lease_pub = breaker
        .acquire_lease(ToolExecutionLeaseId::new("lease-pub-1").unwrap(), "doc-read")
        .unwrap();
    gate.register_derived_lease(d_tool.clone(), lease_tool)
        .unwrap();
    gate.register_derived_lease(d_pub.clone(), lease_pub)
        .unwrap();

    // -------------------------------------------------------------------------
    // Scenario 1: Live Gerbil Scheme ASCENT detects multi-hop CaMeL taint AND
    // Bi-Temporal Truth Maintenance retracts `human-session-alice`!
    // -------------------------------------------------------------------------
    let req_attack = HigherOrderGovernanceVectorRequest {
        delegation_edges: vec![("alice".into(), "agent-sub".into())],
        revoked_actors: vec!["alice".into()],
        taint_edges: vec![
            ("chat-ext".into(), "shared-ctx".into()),
            ("shared-ctx".into(), "os-shell".into()),
        ],
        taint_sources: vec!["chat-ext".into()],
        taint_sinks: vec!["os-shell".into()],
        taint_sanitizers: vec![],
        retracted_subjects: vec!["human-session-alice".into()],
        revocation_epoch: 2,
        policy_revision: 1,
        ..Default::default()
    };
    let in_attack = encode_higher_order_governance_request(&runtime, &req_attack).unwrap();
    let out_attack = runtime.call_rooted(&scheme_project, &in_attack).unwrap();
    let proj_attack = decode_higher_order_governance_vector(out_attack).unwrap();

    assert!(!proj_attack.admitted);
    assert!(
        proj_attack
            .rejection_reasons
            .iter()
            .any(|r| r == "ascent-camel-taint-alert")
    );
    assert!(
        proj_attack
            .rejection_reasons
            .iter()
            .any(|r| r == "bitemporal-premise-retracted")
    );
    assert_eq!(
        proj_attack.retracted_conclusions,
        vec!["D-OBO-TICKET".to_owned(), "D-TOOL-EXEC".to_owned()]
    );

    let env_attack = into_kernel_envelope(&proj_attack);
    let sync_attack = gate.synchronize_proof_envelope(&env_attack).unwrap();
    assert!(!sync_attack.admitted);
    assert!(sync_attack.breaker_auto_tripped);
    assert_eq!(sync_attack.invalidated_active_derivations, vec![d_tool.clone()]);
    assert!(gate.is_derivation_retracted(&d_tool));
    assert!(!gate.is_derivation_retracted(&d_pub));
    assert!(breaker.snapshot().unwrap().tripped);

    // Physical OS side-effect cutoff: `/bin/sh` MUST NOT run!
    let blocked_receipt = executor.execute(&tokio_rt.context(), &handoff).await;
    assert_eq!(blocked_receipt.status, LoopProgramRuntimeSideEffectStatus::Failed);
    assert!(
        !sentinel.exists(),
        "live Scheme ASCENT taint + Truth Maintenance retraction MUST physically block OS process spawn"
    );

    // -------------------------------------------------------------------------
    // Scenario 2: Live Scheme clears taint with `human-gate` sanitizer at epoch 3,
    // while still exporting ASCENT transitive fencing & Cedar forbid for `agent-sub`!
    // -------------------------------------------------------------------------
    let req_clean = HigherOrderGovernanceVectorRequest {
        delegation_edges: vec![("alice".into(), "agent-sub".into())],
        revoked_actors: vec!["alice".into()],
        taint_edges: vec![
            ("chat-ext".into(), "human-gate".into()),
            ("human-gate".into(), "os-shell".into()),
        ],
        taint_sources: vec!["chat-ext".into()],
        taint_sinks: vec!["os-shell".into()],
        taint_sanitizers: vec!["human-gate".into()],
        retracted_subjects: vec![],
        revocation_epoch: 3,
        policy_revision: 2,
        ..Default::default()
    };
    let in_clean = encode_higher_order_governance_request(&runtime, &req_clean).unwrap();
    let out_clean = runtime.call_rooted(&scheme_project, &in_clean).unwrap();
    let proj_clean = decode_higher_order_governance_vector(out_clean).unwrap();

    assert!(proj_clean.admitted);
    assert!(proj_clean.fenced_actors.iter().any(|a| a == "agent-sub"));
    assert!(
        proj_clean
            .cedar_policies
            .iter()
            .any(|(id, _)| id == "marlin.ascent.forbid.fenced.agent-sub")
    );

    let env_clean = into_kernel_envelope(&proj_clean);
    let sync_clean = gate.synchronize_proof_envelope(&env_clean).unwrap();
    assert!(sync_clean.admitted);
    assert!(!sync_clean.breaker_auto_tripped);

    // Replaying the older epoch-2 proof envelope MUST fail closed (`StaleProofEpoch`)!
    assert!(matches!(
        gate.synchronize_proof_envelope(&env_attack),
        Err(HigherOrderBridgeError::StaleProofEpoch { .. })
    ));

    // Verified human re-arms breaker (epoch 2) -> register fresh derivation ->
    // `agent-sub` is STILL blocked by live Scheme ASCENT/Cedar forbid, while clean `ops-bob` succeeds!
    let human = BreakerActor::VerifiedHuman(BreakerActorId::new("ops-human").unwrap());
    breaker.rearm(&human).unwrap();

    let d_fresh = DerivationId::new("D-FRESH-EXEC").unwrap();
    let fresh_lease = breaker
        .acquire_lease(ToolExecutionLeaseId::new("lease-fresh-1").unwrap(), "os-shell")
        .unwrap();
    gate.register_derived_lease(d_fresh.clone(), fresh_lease)
        .unwrap();

    let fenced_agent = GovernedSubjectName::new("agent-sub").unwrap();
    let clean_actor = GovernedSubjectName::new("ops-bob").unwrap();
    let shell_tool = GovernedSubjectName::new("os-shell").unwrap();

    assert!(matches!(
        gate.commit_derived_effect(
            &d_fresh,
            &fenced_agent,
            &shell_tool,
            &ToolIfcContext::clean_trusted(),
        ),
        Err(HigherOrderBridgeError::AscentFencedActorDenied(_))
    ));

    let committed = gate
        .commit_derived_effect(
            &d_fresh,
            &clean_actor,
            &shell_tool,
            &ToolIfcContext::clean_trusted(),
        )
        .expect("verified clean actor with sanitized proof and armed breaker commits cleanly");
    assert_eq!(committed.trip_epoch.get(), 2);

    // Physical OS side effect via LoopProgramRuntimeSideEffectExecutor::with_higher_order_gate:
    // 1. Invoked as ASCENT-fenced `agent-sub` -> physically blocks `/bin/sh` OS spawn!
    let gate_arc = Arc::new(gate);
    let fenced_executor = executor.clone().with_higher_order_gate(
        Arc::clone(&gate_arc),
        fenced_agent,
        Some(d_fresh.clone()),
    );
    let fenced_os_receipt = fenced_executor.execute(&tokio_rt.context(), &handoff).await;
    assert_eq!(fenced_os_receipt.status, LoopProgramRuntimeSideEffectStatus::Failed);
    assert!(
        fenced_os_receipt.tool_processes[0]
            .diagnostic
            .as_deref()
            .unwrap_or("")
            .contains("higher_order_blocked")
    );
    assert!(
        !sentinel.exists(),
        "ASCENT-fenced actor MUST be physically blocked from spawning /bin/sh by side-effect executor"
    );

    // 2. Invoked as clean `ops-bob` -> physically executes `/bin/sh` cleanly!
    let clean_executor = executor.with_higher_order_gate(gate_arc, clean_actor, Some(d_fresh));
    let ok_receipt = clean_executor.execute(&tokio_rt.context(), &handoff).await;
    assert_eq!(ok_receipt.status, LoopProgramRuntimeSideEffectStatus::Completed);
    assert!(sentinel.exists(), "real OS process should succeed after verified recovery");
}
