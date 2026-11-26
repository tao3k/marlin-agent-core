// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Live Embedded Gambit/Gerbil VM -> Zero-JSON Rooted Vector -> Rust Kernel E2E Verification!
//!
//! Executes REAL Gerbil Scheme `scheme/policy/governance.ss`
//! (Atomic POO × `gerbil-ascent` Stratified Taint & Multi-Hop Delegation Fixpoint ×
//! `agentic-ai-lab/modules/temporal-causality` Bi-Temporal Truth Maintenance ×
//! Dynamic Cedar Policy Synthesis) inside the live embedded `GerbilRuntime`,
//! decodes the 14-slot rooted Gambit heap vector with ZERO JSON serialization,
//! and drives Rust `GovernanceAdmissionGate` + `ExecutionCircuitBreaker` +
//! `LoopProgramRuntimeSideEffectExecutor` to physically block/allow real `/bin/sh` OS spawns.

use std::sync::Arc;

use gerbil_scheme::{GerbilRuntime, IdentifierKind, PackageResolutionProgress};
use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, BreakerActor, BreakerActorId, CedarPolicyId,
    DerivationId, ExecutionCircuitBreaker, GenericLoopMachineReceipt, GenericLoopMachineStepIndex,
    GovernanceProofDigest, GovernedSubjectName, GovernanceAdmissionError,
    GovernanceAdmissionGate, GovernanceProofEnvelope, GovernanceProofEpoch,
    LoopProgramFileSandbox, LoopProgramFileWriteSideEffectStatus, LoopProgramFileWriteTemplate,
    LoopProgramRuntimeHandoffExecutor, LoopProgramRuntimeHandoffPlan, LoopProgramRuntimeOwner,
    LoopProgramRuntimeSideEffectExecutor, LoopProgramRuntimeSideEffectStatus,
    LoopProgramToolProcessCommandTemplate, LoopProgramToolProcessProgram,
    StaticLoopProgramFileWriteResolver, StaticLoopProgramToolProcessResolver,
    SynthesizedCedarPolicy, ToolBudgetUnits, ToolExecutionLeaseId, ToolIfcContext,
};
use marlin_agent_protocol::{
    LoopProgramActionKind, LoopProgramEventKind, LoopProgramId, LoopProgramStateId,
    LoopProgramTransitionId,
};
use marlin_agent_runtime::TokioAgentRuntime;
use marlin_gerbil_scheme::{
    GovernancePolicyVectorProjection, GovernancePolicyVectorRequest,
    MARLIN_GOVERNANCE_PROJECT_SYMBOL, decode_governance_policy_vector,
    encode_governance_policy_request, gerbil_package_root,
};

fn into_kernel_envelope(
    proj: &GovernancePolicyVectorProjection,
) -> GovernanceProofEnvelope {
    GovernanceProofEnvelope {
        schema_version: proj.schema_version,
        admitted: proj.admitted,
        revocation_epoch: GovernanceProofEpoch::new(proj.revocation_epoch).unwrap(),
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
async fn live_gerbil_vm_governance_policy_drives_rust_breaker_and_blocks_real_os_spawn() {
    let runtime = GerbilRuntime::initialize().expect("initialize embedded Gambit/Gerbil VM");
    let _ = runtime.set_package_resolution_progress(PackageResolutionProgress::Quiet);

    let scheme_dir = gerbil_package_root();
    let scheme_dir_str = scheme_dir.to_str().expect("utf8 scheme dir");
    let module_sym = runtime
        .intern_identifier(
            IdentifierKind::Symbol,
            ":marlin-agent-core/policy/governance",
        )
        .unwrap();
    let proc_sym = runtime
        .intern_identifier(
            IdentifierKind::Symbol,
            MARLIN_GOVERNANCE_PROJECT_SYMBOL,
        )
        .unwrap();
    let scheme_project = runtime
        .resolve_package_export(scheme_dir_str, &module_sym, &proc_sym)
        .expect("resolve live Scheme marlin-governance-project-vector");

    let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(4)));
    let gate = GovernanceAdmissionGate::new(Arc::clone(&breaker));

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
    let req_attack = GovernancePolicyVectorRequest {
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
    let in_attack = encode_governance_policy_request(&runtime, &req_attack).unwrap();
    let out_attack = runtime.call_rooted(&scheme_project, &in_attack).unwrap();
    let proj_attack = decode_governance_policy_vector(out_attack).unwrap();

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
    let req_clean = GovernancePolicyVectorRequest {
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
    let in_clean = encode_governance_policy_request(&runtime, &req_clean).unwrap();
    let out_clean = runtime.call_rooted(&scheme_project, &in_clean).unwrap();
    let proj_clean = decode_governance_policy_vector(out_clean).unwrap();

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
        Err(GovernanceAdmissionError::StaleProofEpoch { .. })
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
        Err(GovernanceAdmissionError::AscentFencedActorDenied(_))
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

    // Physical OS side effect via LoopProgramRuntimeSideEffectExecutor::with_governance_gate:
    // 1. Invoked as ASCENT-fenced `agent-sub` -> physically blocks `/bin/sh` OS spawn!
    let gate_arc = Arc::new(gate);
    let fenced_executor = executor.clone().with_governance_gate(
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
            .contains("governance_blocked")
    );
    assert!(
        !sentinel.exists(),
        "ASCENT-fenced actor MUST be physically blocked from spawning /bin/sh by side-effect executor"
    );

    // 2. Invoked as clean `ops-bob` -> physically executes `/bin/sh` cleanly!
    let clean_executor = executor.with_governance_gate(
        Arc::clone(&gate_arc),
        clean_actor.clone(),
        Some(d_fresh),
    );
    let ok_receipt = clean_executor.execute(&tokio_rt.context(), &handoff).await;
    assert_eq!(ok_receipt.status, LoopProgramRuntimeSideEffectStatus::Completed);
    assert!(sentinel.exists(), "real OS process should succeed after verified recovery");

    // -------------------------------------------------------------------------
    // Scenario 3: Live Gambit VM synthesizes resource-scoped Cedar exfiltration
    // `forbid (principal, action, resource == Marlin::Resource::"ext-webhook.txt");`
    // -> LoopProgramRuntimeSideEffectExecutor PHYSICALLY blocks disk write to
    // `ext-webhook.txt` while allowing `safe-report.md`, then admits `ext-webhook.txt`
    // once Scheme evaluates explicit declassification!
    // -------------------------------------------------------------------------
    let req_exfil = GovernancePolicyVectorRequest {
        context_entries: vec![("secret-doc".into(), "private".into())],
        context_flows: vec![("secret-doc".into(), "ext-webhook.txt".into())],
        target_scopes: vec![("ext-webhook.txt".into(), "external".into())],
        declassified_entries: vec![],
        revocation_epoch: 4,
        policy_revision: 3,
        ..Default::default()
    };
    let in_exfil = encode_governance_policy_request(&runtime, &req_exfil).unwrap();
    let out_exfil = runtime.call_rooted(&scheme_project, &in_exfil).unwrap();
    let proj_exfil = decode_governance_policy_vector(out_exfil).unwrap();
    assert!(!proj_exfil.admitted);
    assert!(
        proj_exfil
            .cedar_policies
            .iter()
            .any(|(id, _)| id == "marlin.ascent.forbid.exfil.secret-doc.ext-webhook.txt"),
        "expected Scheme ASCENT to synthesize resource-scoped Cedar exfil forbid, got {:?}",
        proj_exfil.cedar_policies
    );

    let env_exfil = into_kernel_envelope(&proj_exfil);
    gate_arc.synchronize_proof_envelope(&env_exfil).unwrap();
    // Re-arm breaker so ONLY the compiled Scheme Cedar resource forbid guards disk writes!
    breaker.rearm(&human).unwrap();
    assert!(!breaker.snapshot().unwrap().tripped);

    let sandbox = LoopProgramFileSandbox::new(temp_dir.path())
        .with_allowed_relative_paths(["ext-webhook.txt", "safe-report.md"]);

    let make_write_executor = |target_file: &str, body: &[u8]| {
        let empty_proc_resolver = StaticLoopProgramToolProcessResolver::default();
        let file_resolver = StaticLoopProgramFileWriteResolver::new(vec![
            LoopProgramFileWriteTemplate::new(
                "agent-flow.tool-intent",
                ["loop-program.dispatch-tools"],
                target_file,
                body.to_vec(),
            ),
        ]);
        LoopProgramRuntimeSideEffectExecutor::new(empty_proc_resolver)
            .with_file_write_resolver(file_resolver)
            .with_file_sandbox(sandbox.clone())
            .with_governance_gate(Arc::clone(&gate_arc), clean_actor.clone(), None)
    };

    // (a) Writing Cedar-forbidden `ext-webhook.txt` MUST be physically blocked before touching disk!
    let denied_write = make_write_executor("ext-webhook.txt", b"leaked-secret")
        .execute(&tokio_rt.context(), &handoff)
        .await;
    assert_eq!(denied_write.file_writes.len(), 1);
    assert_eq!(
        denied_write.file_writes[0].status,
        LoopProgramFileWriteSideEffectStatus::Denied
    );
    assert!(
        denied_write.file_writes[0]
            .diagnostic
            .as_deref()
            .unwrap_or("")
            .contains("marlin.ascent.forbid.exfil.secret-doc.ext-webhook.txt"),
        "expected compiled Cedar exfil forbid diagnostic, got {:?}",
        denied_write.file_writes[0].diagnostic
    );
    assert!(
        !temp_dir.path().join("ext-webhook.txt").exists(),
        "Cedar-forbidden resource MUST NOT be written to disk"
    );

    // (b) Writing unrelated `safe-report.md` succeeds cleanly!
    let safe_write = make_write_executor("safe-report.md", b"public-summary")
        .execute(&tokio_rt.context(), &handoff)
        .await;
    assert_eq!(
        safe_write.file_writes[0].status,
        LoopProgramFileWriteSideEffectStatus::Completed
    );
    assert_eq!(
        std::fs::read(temp_dir.path().join("safe-report.md")).unwrap(),
        b"public-summary"
    );

    // (c) Explicit declassification in live Gambit VM clears exfil Cedar forbid -> write admitted!
    let req_declass = GovernancePolicyVectorRequest {
        declassified_entries: vec!["secret-doc".into()],
        revocation_epoch: 5,
        policy_revision: 4,
        ..req_exfil
    };
    let in_declass = encode_governance_policy_request(&runtime, &req_declass).unwrap();
    let out_declass = runtime.call_rooted(&scheme_project, &in_declass).unwrap();
    let proj_declass = decode_governance_policy_vector(out_declass).unwrap();
    assert!(proj_declass.admitted);
    assert!(
        !proj_declass
            .cedar_policies
            .iter()
            .any(|(id, _)| id == "marlin.ascent.forbid.exfil.secret-doc.ext-webhook.txt")
    );
    gate_arc
        .synchronize_proof_envelope(&into_kernel_envelope(&proj_declass))
        .unwrap();

    let admitted_write = make_write_executor("ext-webhook.txt", b"declassified-summary")
        .execute(&tokio_rt.context(), &handoff)
        .await;
    assert_eq!(
        admitted_write.file_writes[0].status,
        LoopProgramFileWriteSideEffectStatus::Completed
    );
    assert_eq!(
        std::fs::read(temp_dir.path().join("ext-webhook.txt")).unwrap(),
        b"declassified-summary"
    );
}
