use marlin_agent_kernel::{
    LoopProgramExecutionDriver, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus,
};
use marlin_agent_protocol::LoopProgramEventKind;
use marlin_gerbil_scheme::verify_gerbil_loop_case_driver_vertical_trace;

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
