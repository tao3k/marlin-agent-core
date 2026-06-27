use marlin_agent_kernel::{
    LoopProgramExecutionDriver, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus,
};
use marlin_agent_protocol::LoopProgramEventKind;
use marlin_gerbil_scheme::{
    GERBIL_LOOP_CASE_DRIVER_RUST_LOOP_RECEIPT_SCHEMA_ID, GerbilLoopCaseCommandKind,
    GerbilLoopCaseRuntimeHandoffStatus, GerbilLoopCaseSchemeBoundary,
    GerbilLoopCaseSerializationBoundary,
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
