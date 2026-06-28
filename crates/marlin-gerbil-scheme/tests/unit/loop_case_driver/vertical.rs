use marlin_agent_harness_types::{IntentCaseArtifactKind, IntentCaseRunStatus};
use marlin_agent_kernel::{
    LoopProgramExecutionDriver, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus,
};
use marlin_agent_protocol::LoopProgramEventKind;
use marlin_gerbil_scheme::{
    GERBIL_LOOP_CASE_DRIVER_INTENT_CASE_RUNTIME_OWNER,
    GERBIL_LOOP_CASE_DRIVER_RUST_LOOP_RECEIPT_SCHEMA_ID, GerbilLoopCaseCommandKind,
    GerbilLoopCaseRuntimeHandoffStatus, GerbilLoopCaseSchemeBoundary,
    GerbilLoopCaseSerializationBoundary,
    project_gerbil_loop_case_driver_intent_case_artifact_manifest,
    project_gerbil_loop_case_driver_intent_case_run_receipt,
    project_gerbil_loop_case_driver_vertical_trace_rust_loop_receipt,
    verify_gerbil_loop_case_driver_vertical_trace,
};

use super::support::{
    cap, loop_program_action_kind, run_config_interface_case_driver_smoke,
    scheme_projected_event_mapper, scheme_projected_loop_program,
    scheme_projected_runtime_executor,
};

#[test]
fn config_interface_case_driver_scheme_smoke_runs_vertical_policy_cases() {
    let stdout = run_config_interface_case_driver_smoke();
    assert!(stdout.contains("config-interface-case-driver-ok"));
    assert!(stdout.contains("case-driver-receipts=11"));
    assert!(stdout.contains("vertical-case-receipts=7"));
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");
    assert!(
        vertical_receipts
            .iter()
            .any(|receipt| receipt.transition_count() >= 6),
        "at least one vertical case should cover the scripted end-to-end loop: {vertical_receipts:?}"
    );
    assert!(
        vertical_receipts
            .iter()
            .any(|receipt| receipt.mechanism_policy_count() >= 3),
        "at least one vertical case should combine multiple runtime policies: {vertical_receipts:?}"
    );
    for receipt in &vertical_receipts {
        let capability_tags = receipt.capability_tags().collect::<Vec<_>>();
        let module_selection_tags = receipt.module_selection_tags().collect::<Vec<_>>();

        assert_eq!(receipt.compiler_owner(), "gerbil-poo-flow");
        assert_eq!(
            receipt.session_transform(),
            "loop-policy-profile->loop-program"
        );
        assert_eq!(
            receipt.runtime_handoff_kind(),
            "loop-program-runtime-handoff"
        );
        assert_eq!(
            receipt.runtime_receipt_kind(),
            "loop-program-runtime-receipt"
        );
        assert_eq!(
            receipt.derived_session_kind(),
            "derived-session/from-loop-receipt"
        );
        assert_eq!(
            receipt.module_kind(),
            "marlin.config-interface.loop-policy-profile-projection.v1"
        );
        assert_eq!(receipt.module_user_module(), "funflow");
        assert_eq!(module_selection_tags, capability_tags);
        assert_eq!(receipt.module_source_ref(), receipt.profile_ref().as_str());
        assert_eq!(
            receipt.module_entrypoint(),
            "marlinLoopPolicyProfileCompilerReceipts"
        );
        assert!(receipt.module_enabled());
        assert_ne!(receipt.placement_intent_count(), 0);
        assert_ne!(receipt.capability_mask(), 0);
        assert_eq!(
            receipt.mechanism_policy_ids().count(),
            receipt.mechanism_policy_count()
        );
        assert!(
            receipt
                .mechanism_policy_ids()
                .all(|policy_id| !policy_id.trim().is_empty()),
            "mechanism policy ids should be explicit Scheme-projected policy lanes: {receipt:?}"
        );
    }
    let live_repair_receipts = vertical_receipts
        .iter()
        .filter(|receipt| receipt.live_llm_required())
        .collect::<Vec<_>>();
    assert_eq!(live_repair_receipts.len(), 1);
    let live_repair_receipt = live_repair_receipts[0];
    assert_eq!(live_repair_receipt.live_gate_env(), "MARLIN_LIVE_LLM");
    assert!(!live_repair_receipt.live_llm_allowed());
    assert_eq!(
        live_repair_receipt.live_llm_denial_receipt(),
        "deferred-no-live-llm"
    );
    assert_eq!(
        live_repair_receipt.llm_repair_intent(),
        "single-file-repair"
    );
    assert_ne!(live_repair_receipt.tool_intent_count(), 0);
    assert!(live_repair_receipt.has_capability(&cap("+tool-repair")));
    assert!(
        live_repair_receipt
            .transition_actions()
            .any(|action| action == "invoke_model"),
        "live repair receipt should include model invocation before tool repair"
    );
    assert!(
        vertical_receipts
            .iter()
            .filter(|receipt| !receipt.live_llm_required())
            .all(|receipt| {
                receipt.live_gate_env() == "none"
                    && !receipt.live_llm_allowed()
                    && receipt.live_llm_denial_receipt() == "not-required"
                    && receipt.llm_repair_intent() == "none"
            })
    );
    assert!(
        vertical_receipts
            .iter()
            .any(|receipt| receipt.tool_intent_count() > 0),
        "vertical flow should include Scheme-projected ToolIntent lanes: {vertical_receipts:?}"
    );
    assert!(
        vertical_receipts
            .iter()
            .any(|receipt| receipt.memory_intent_count() > 0),
        "vertical flow should include Scheme-projected MemoryIntent lanes: {vertical_receipts:?}"
    );
    assert!(
        vertical_receipts
            .iter()
            .filter(|receipt| receipt.memory_intent_count() > 0)
            .all(|receipt| {
                receipt.has_capability(&cap("+memory"))
                    || receipt.has_capability(&cap("+memory-recall"))
            }),
        "memory intents must stay tied to Scheme-projected memory capabilities: {vertical_receipts:?}"
    );
    assert!(
        vertical_receipts.iter().any(|receipt| receipt
            .mechanism_policy_ids()
            .any(|policy_id| policy_id == "verification-gate")),
        "vertical receipts should include the Scheme-projected verification policy lane: {vertical_receipts:?}"
    );
    for capability in [
        "+scripted-e2e",
        "+tool-repair",
        "+verification",
        "+sandbox",
        "+denylist",
        "+retry-budget",
        "+failure-policy",
        "+maker",
        "+checker",
        "+dynamic-rewrite",
        "+repair",
        "+memory-recall",
        "+memory",
        "+tool-selection",
        "+policy-combination",
        "+rewrite",
    ] {
        assert!(
            vertical_receipts
                .iter()
                .any(|receipt| receipt.has_capability(&cap(capability))),
            "Scheme vertical case matrix should cover capability {capability}: {vertical_receipts:?}"
        );
    }
}

#[test]
fn config_interface_vertical_trace_projects_to_rust_loop_receipts() {
    let stdout = run_config_interface_case_driver_smoke();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");

    let rust_receipts = vertical_receipts
        .iter()
        .map(project_gerbil_loop_case_driver_vertical_trace_rust_loop_receipt)
        .collect::<Vec<_>>();

    assert_eq!(rust_receipts.len(), 7);
    for (vertical_receipt, rust_receipt) in vertical_receipts.iter().zip(&rust_receipts) {
        assert_eq!(
            rust_receipt.schema_id,
            GERBIL_LOOP_CASE_DRIVER_RUST_LOOP_RECEIPT_SCHEMA_ID
        );
        assert_eq!(&rust_receipt.case_id, vertical_receipt.case_id());
        assert_eq!(&rust_receipt.profile_ref, vertical_receipt.profile_ref());
        assert_eq!(
            rust_receipt.command_kind,
            GerbilLoopCaseCommandKind::LoopProgramRun
        );
        assert_eq!(
            rust_receipt.command_vector,
            vec![
                "marlin",
                "loop",
                "program",
                "run",
                "--profile",
                vertical_receipt.profile_ref().as_str(),
            ]
        );
        assert_eq!(
            rust_receipt.input_path.to_string_lossy(),
            vertical_receipt.module_source_ref()
        );
        assert_eq!(
            rust_receipt.runtime_handoff_status,
            if vertical_receipt.live_llm_required() && !vertical_receipt.live_llm_allowed() {
                GerbilLoopCaseRuntimeHandoffStatus::DeferredNoLiveLlm
            } else {
                GerbilLoopCaseRuntimeHandoffStatus::Ready
            }
        );
        assert_eq!(rust_receipt.runtime_execution_owner, "marlin-agent-core");
        assert_eq!(
            rust_receipt.module_kind.as_str(),
            vertical_receipt.module_kind()
        );
        assert_eq!(
            rust_receipt.module_user_module.as_str(),
            vertical_receipt.module_user_module()
        );
        assert_eq!(
            rust_receipt
                .module_selection_tags
                .iter()
                .map(|tag| tag.as_str())
                .collect::<Vec<_>>(),
            vertical_receipt
                .module_selection_tags()
                .map(|tag| tag.as_str())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            rust_receipt.module_source_ref,
            vertical_receipt.module_source_ref()
        );
        assert_eq!(
            rust_receipt.module_entrypoint,
            vertical_receipt.module_entrypoint()
        );
        assert_eq!(
            rust_receipt.module_enabled,
            vertical_receipt.module_enabled()
        );
        assert_eq!(
            rust_receipt.live_llm_required,
            vertical_receipt.live_llm_required()
        );
        assert_eq!(
            rust_receipt.live_llm_allowed,
            vertical_receipt.live_llm_allowed()
        );
        assert!(!rust_receipt.stable_fixture);
        assert_eq!(
            rust_receipt.scheme_boundary,
            GerbilLoopCaseSchemeBoundary::SchemeTypesToRustTypes
        );
        assert_eq!(
            rust_receipt.serialization_boundary,
            GerbilLoopCaseSerializationBoundary::RustOwnedCliTraceCrossProcess
        );
        assert!(
            !format!("{:?}", rust_receipt.scheme_boundary)
                .to_ascii_lowercase()
                .contains("json")
        );
    }

    let deferred_live_receipts = rust_receipts
        .iter()
        .filter(|receipt| receipt.live_llm_required)
        .collect::<Vec<_>>();
    assert_eq!(deferred_live_receipts.len(), 1);
    assert_eq!(
        deferred_live_receipts[0].runtime_handoff_status,
        GerbilLoopCaseRuntimeHandoffStatus::DeferredNoLiveLlm
    );
    assert!(!deferred_live_receipts[0].live_llm_allowed);
}

#[test]
fn config_interface_vertical_trace_projects_to_intent_case_artifact_bundles() {
    let stdout = run_config_interface_case_driver_smoke();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");

    for receipt in &vertical_receipts {
        let run_id = format!("scripted-smoke-{}", receipt.case_id().as_str());
        let manifest =
            project_gerbil_loop_case_driver_intent_case_artifact_manifest(receipt, run_id.clone());
        let run_receipt =
            project_gerbil_loop_case_driver_intent_case_run_receipt(receipt, run_id.clone());

        assert!(manifest.is_supported_schema());
        assert_eq!(manifest.case_id.as_str(), receipt.case_id().as_str());
        assert_eq!(manifest.run_id.as_str(), run_id);
        assert_eq!(manifest.policy_epoch, receipt.policy_epoch());
        assert_eq!(
            manifest.policy_digest.as_str().len(),
            receipt.policy_digest_octets().len() * 2
        );
        assert_eq!(
            manifest.loop_program_id.as_str(),
            receipt.loop_program_id().as_str()
        );
        assert!(manifest.has_core_artifact_bundle());
        assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::Intent));
        assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::PolicyPack));
        assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::LoopProgram));
        assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::VerticalTrace));
        assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::ExecutionTrace));
        assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::PolicyExplanation));
        assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::ReplayScript));
        if receipt.live_llm_required() {
            assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::ModelEvents));
            assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::DiffPatch));
            assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::TestBefore));
            assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::TestAfter));
        }
        if receipt.tool_intent_count() > 0 {
            assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::ToolCalls));
        }
        if receipt.memory_intent_count() > 0 {
            assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::MemoryReceipts));
        }
        if receipt.has_capability(&cap("+sandbox")) || receipt.has_capability(&cap("+denylist")) {
            assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::SandboxReceipts));
        }
        if receipt.has_capability(&cap("+verification")) {
            assert!(manifest.has_artifact_kind(IntentCaseArtifactKind::VerifierReceipt));
        }
        assert_eq!(
            manifest.trace_index.entries.len(),
            receipt.transition_count()
        );
        assert!(
            manifest.trace_entries_without_action_identity().is_empty(),
            "model/tool trace steps must carry typed action identities: {manifest:?}"
        );
        assert!(
            manifest
                .artifacts
                .iter()
                .filter_map(|artifact| artifact.path.as_deref())
                .all(|path| !path.ends_with(".json") && !path.ends_with(".jsonl")),
            "intent-case artifacts should not reintroduce JSON as an internal Scheme boundary: {manifest:?}"
        );

        for (index, ((trace_entry, action), event)) in manifest
            .trace_index
            .entries
            .iter()
            .zip(receipt.transition_actions())
            .zip(receipt.transition_events())
            .enumerate()
        {
            assert_eq!(trace_entry.step_index, (index + 1) as u64);
            assert_eq!(trace_entry.action, action);
            assert_eq!(trace_entry.event, event);
            assert!(
                trace_entry
                    .transition_id
                    .as_str()
                    .starts_with(receipt.loop_program_id().as_str())
            );
            assert_eq!(
                trace_entry
                    .runtime_owner
                    .as_ref()
                    .expect("runtime owner is recorded")
                    .as_str(),
                GERBIL_LOOP_CASE_DRIVER_INTENT_CASE_RUNTIME_OWNER
            );
            assert!(
                trace_entry.artifact_refs.len() >= 2,
                "trace entry should correlate vertical and execution artifacts: {trace_entry:?}"
            );
            if action == "invoke_model" && receipt.live_llm_required() {
                assert!(
                    trace_entry
                        .model_invocation_id
                        .as_ref()
                        .is_some_and(|id| id.as_str().contains(":model-invocation-")),
                    "model transition should carry model invocation id: {trace_entry:?}"
                );
                assert!(
                    trace_entry_has_artifact_kind(
                        &manifest,
                        trace_entry,
                        IntentCaseArtifactKind::ModelEvents,
                    ),
                    "model transition should reference model events artifact: {trace_entry:?}"
                );
            }
            if action == "dispatch_tools" && receipt.tool_intent_count() > 0 {
                assert!(
                    trace_entry
                        .tool_call_id
                        .as_ref()
                        .is_some_and(|id| id.as_str().contains(":tool-call-")),
                    "tool transition should carry tool call id: {trace_entry:?}"
                );
                assert!(
                    trace_entry_has_artifact_kind(
                        &manifest,
                        trace_entry,
                        IntentCaseArtifactKind::ToolCalls,
                    ),
                    "tool transition should reference tool calls artifact: {trace_entry:?}"
                );
            }
            if action == "dispatch_tools"
                && (receipt.has_capability(&cap("+sandbox"))
                    || receipt.has_capability(&cap("+denylist")))
            {
                assert!(
                    trace_entry_has_artifact_kind(
                        &manifest,
                        trace_entry,
                        IntentCaseArtifactKind::SandboxReceipts,
                    ),
                    "sandboxed tool transition should reference sandbox receipts artifact: {trace_entry:?}"
                );
            }
            if action == "verify" && receipt.has_capability(&cap("+verification")) {
                assert!(
                    trace_entry_has_artifact_kind(
                        &manifest,
                        trace_entry,
                        IntentCaseArtifactKind::VerifierReceipt,
                    ),
                    "verify transition should reference verifier artifact: {trace_entry:?}"
                );
            }
        }

        assert!(run_receipt.is_supported_schema());
        assert_eq!(run_receipt.status, IntentCaseRunStatus::Passed);
        assert!(run_receipt.diagnostics.is_empty());
        assert_eq!(run_receipt.manifest, manifest);
    }
}

fn trace_entry_has_artifact_kind(
    manifest: &marlin_agent_harness_types::IntentCaseArtifactManifest,
    trace_entry: &marlin_agent_harness_types::IntentCaseTraceEntry,
    kind: IntentCaseArtifactKind,
) -> bool {
    let artifact_id = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == kind && artifact.present)
        .map(|artifact| &artifact.artifact_id);
    artifact_id
        .map(|artifact_id| {
            trace_entry
                .artifact_refs
                .iter()
                .any(|trace_artifact_id| trace_artifact_id == artifact_id)
        })
        .unwrap_or(false)
}

#[test]
fn config_interface_vertical_trace_drives_kernel_loop_programs() {
    let stdout = run_config_interface_case_driver_smoke();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");

    for receipt in &vertical_receipts {
        let loop_program = scheme_projected_loop_program(receipt);
        let driver = LoopProgramExecutionDriver::new(scheme_projected_runtime_executor())
            .with_event_mapper(scheme_projected_event_mapper(receipt))
            .with_max_steps(receipt.transition_count() + 2);

        let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
            loop_program,
            vec![LoopProgramEventKind::Start],
        ));

        assert_eq!(
            execution_receipt.status,
            LoopProgramExecutionStatus::Stopped,
            "Scheme-projected loop {} should stop cleanly: {execution_receipt:?}",
            receipt.loop_program_id()
        );
        assert!(execution_receipt.error.is_none());
        assert_eq!(execution_receipt.steps.len(), receipt.transition_count());
        let manifest = project_gerbil_loop_case_driver_intent_case_artifact_manifest(
            receipt,
            format!("kernel-execution-{}", receipt.case_id().as_str()),
        );
        assert_eq!(
            manifest.trace_index.entries.len(),
            execution_receipt.steps.len()
        );
        for (trace_entry, execution_step) in manifest
            .trace_index
            .entries
            .iter()
            .zip(&execution_receipt.steps)
        {
            assert_eq!(
                trace_entry.step_index,
                execution_step.machine_receipt.step_index.get()
            );
        }
        assert_eq!(
            execution_receipt
                .steps
                .iter()
                .map(|step| step.machine_receipt.action.clone())
                .collect::<Vec<_>>(),
            receipt
                .transition_actions()
                .map(loop_program_action_kind)
                .collect::<Vec<_>>()
        );
        assert!(execution_receipt.steps.iter().all(|step| {
            step.runtime_handoff_execution.status
                == LoopProgramRuntimeHandoffExecutionReportStatus::Completed
        }));
    }
}
