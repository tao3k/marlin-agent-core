//! Verifies the typed denial receipt when live model access is unavailable.

use super::{
    LIVE_LLM_MODEL_ENV, LIVE_LLM_PROVIDER_ENV, LoopProgramActionKind, LoopProgramEventKind,
    LoopProgramExecutionDriver, LoopProgramExecutionRequest,
    LoopProgramRuntimeHandoffExecutionReportStatus, NoLiveRepairDecisionMapper,
    RuntimeLiveRepairGateStatus, RuntimeRepairCaseId, RuntimeRepairCount,
    RuntimeRepairDenialReason, RuntimeRepairHandoffStatus, RuntimeRepairLiveGateStatus,
    RuntimeRepairNoLiveCaseReceipt, RuntimeRepairNoLiveCaseReceiptRequest,
    RuntimeRepairNoLiveHandoffExecutor, RuntimeRepairProfileRef, SCHEME_REAL_REPAIR_CASE_ID,
    SCHEME_REAL_REPAIR_PROFILE_REF, SCHEME_REAL_REPAIR_PROGRAM_ID,
    runtime_live_repair_gate_receipt_from_lookup, runtime_repair_gate_status,
    runtime_repair_handoff_status, scheme_projected_real_repair_loop_case,
};

#[test]
fn runtime_live_repair_no_live_gate_denial_runs_typed_loop_receipt() {
    let gate_receipt = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_PROVIDER_ENV => Some("openai".to_owned()),
        LIVE_LLM_MODEL_ENV => Some("gpt-repair-policy".to_owned()),
        "OPENAI_API_KEY" => Some("redacted".to_owned()),
        _ => None,
    });
    assert_eq!(gate_receipt.status, RuntimeLiveRepairGateStatus::Disabled);
    let denial_reason = gate_receipt
        .denial_reason
        .clone()
        .expect("disabled no-live gate denial reason");

    let driver = LoopProgramExecutionDriver::new(RuntimeRepairNoLiveHandoffExecutor::new())
        .with_event_mapper(NoLiveRepairDecisionMapper);
    let scheme_case = scheme_projected_real_repair_loop_case();
    let scheme_receipt = scheme_case.receipt();
    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        scheme_case.loop_program().clone(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![LoopProgramActionKind::InvokeModel]
    );

    let model_step = execution_receipt.steps.first().expect("model denial step");
    assert_eq!(
        model_step.machine_receipt.action,
        LoopProgramActionKind::InvokeModel
    );
    assert_eq!(
        model_step.runtime_handoff_execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Denied
    );
    assert!(
        execution_receipt.error.is_some(),
        "Scheme-projected repair loop should not synthesize a Rust stop branch for denied LLM: {execution_receipt:?}"
    );

    let no_live_receipt =
        RuntimeRepairNoLiveCaseReceipt::new(RuntimeRepairNoLiveCaseReceiptRequest {
            case_id: RuntimeRepairCaseId::new(scheme_receipt.case_id().as_str()),
            profile_ref: RuntimeRepairProfileRef::new(scheme_receipt.profile_ref().as_str()),
            program_id: execution_receipt.program_id.clone(),
            gate_status: runtime_repair_gate_status(gate_receipt.status),
            denial_reason: RuntimeRepairDenialReason::new(denial_reason),
            live_llm_allowed: false,
            action_count: RuntimeRepairCount::new(execution_receipt.steps.len()),
            model_handoff_status: runtime_repair_handoff_status(
                &model_step.runtime_handoff_execution.status,
            ),
        });

    assert_eq!(no_live_receipt.case_id.as_str(), SCHEME_REAL_REPAIR_CASE_ID);
    assert_eq!(
        no_live_receipt.profile_ref.as_str(),
        SCHEME_REAL_REPAIR_PROFILE_REF
    );
    assert_eq!(
        no_live_receipt.program_id.as_str(),
        SCHEME_REAL_REPAIR_PROGRAM_ID
    );
    assert_eq!(
        no_live_receipt.gate_status,
        RuntimeRepairLiveGateStatus::Disabled
    );
    assert_eq!(
        no_live_receipt.denial_reason.as_str(),
        "live LLM gate is disabled"
    );
    assert!(!no_live_receipt.live_llm_allowed);
    assert_eq!(no_live_receipt.action_count.get(), 1);
    assert_eq!(
        no_live_receipt.model_handoff_status,
        RuntimeRepairHandoffStatus::Denied
    );
}
