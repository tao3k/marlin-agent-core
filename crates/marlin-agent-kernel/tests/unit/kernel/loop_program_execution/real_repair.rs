use std::{fs, sync::Arc};

use super::{
    GatewayRepairDecisionMapper, LoopProgramActionKind, LoopProgramEventKind,
    LoopProgramExecutionDriver, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffRouter,
    LoopProgramRuntimeHandoffRouterHandlers, LoopProgramToolProcessProgram,
    LoopProgramToolProcessSpawnRequest, ModelGatewayMessageRole, RealRepairHybridHandoffExecutor,
    RuntimeEdgeModelGateway, RuntimeEdgePolicy, StaticRepairGateway, TokioAgentRuntime, handled_by,
    real_repair_001_no_write_llm_loop_program, real_repair_001_single_file_loop_program,
    spawn_loop_program_tool_process, unique_temp_repair_file,
};

#[test]
fn real_repair_001_no_write_llm_repair_receipt_selects_single_file_patch() {
    let gateway = StaticRepairGateway::new("PATCH_INTENT:single-file-add-one");
    let edge_gateway = RuntimeEdgeModelGateway::new(
        gateway.clone(),
        RuntimeEdgePolicy::new()
            .with_concurrency_limit(1)
            .with_timeout_ms(5_000),
    )
    .expect("runtime edge model gateway");
    let mapper = GatewayRepairDecisionMapper::new(Arc::new(edge_gateway));
    let completion_receipts = mapper.completion_receipts();
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        model_handler: handled_by("runtime.model.gateway.repair-planner"),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(mapper);

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        real_repair_001_no_write_llm_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(
        receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::Stop,
        ]
    );
    assert_eq!(gateway.requests().len(), 1);
    assert_eq!(gateway.requests()[0].endpoint().provider.as_str(), "openai");
    assert_eq!(
        gateway.requests()[0].endpoint().model.as_str(),
        "gpt-5-mini"
    );
    assert_eq!(
        gateway.requests()[0].messages()[0].role,
        ModelGatewayMessageRole::System
    );
    assert_eq!(
        completion_receipts.lock().expect("completion receipts")[0].choices[0]
            .message
            .content,
        "PATCH_INTENT:single-file-add-one"
    );
}

#[cfg(unix)]
#[test]
fn real_repair_001_single_file_bug_fix_runs_llm_tool_and_verifier_loop() {
    let bug_file = unique_temp_repair_file();
    fs::write(&bug_file, "fn answer() -> i32 { 40 }\n").expect("write bug fixture");

    let gateway = StaticRepairGateway::new("PATCH_INTENT:single-file-add-one");
    let edge_gateway = RuntimeEdgeModelGateway::new(
        gateway.clone(),
        RuntimeEdgePolicy::new()
            .with_concurrency_limit(1)
            .with_timeout_ms(5_000),
    )
    .expect("runtime edge model gateway");
    let mapper = GatewayRepairDecisionMapper::new(Arc::new(edge_gateway))
        .with_tool_event(LoopProgramEventKind::ToolReceipt)
        .with_verification_event(LoopProgramEventKind::VerificationReceipt);
    let completion_receipts = mapper.completion_receipts();
    let executor = RealRepairHybridHandoffExecutor::new();
    let driver = LoopProgramExecutionDriver::new(executor).with_event_mapper(mapper);

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        real_repair_001_single_file_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(
        receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Verify,
            LoopProgramActionKind::Stop,
        ]
    );
    assert_eq!(gateway.requests().len(), 1);
    assert_eq!(
        completion_receipts.lock().expect("completion receipts")[0].choices[0]
            .message
            .content,
        "PATCH_INTENT:single-file-add-one"
    );

    let tool_step = receipt
        .steps
        .iter()
        .find(|step| step.machine_receipt.action == LoopProgramActionKind::DispatchTools)
        .expect("tool dispatch step");
    assert_eq!(
        tool_step.runtime_handoff_execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    );
    assert_eq!(
        tool_step
            .runtime_handoff_execution
            .tool_process_projections
            .len(),
        1
    );

    let (runtime, _events) = TokioAgentRuntime::new(4);
    let spawn_receipt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tool process runtime")
        .block_on(spawn_loop_program_tool_process(
            &runtime.context(),
            LoopProgramToolProcessSpawnRequest::new(
                tool_step.runtime_handoff_execution.tool_process_projections[0].clone(),
                LoopProgramToolProcessProgram::new("sh"),
            )
            .with_args(
                vec![
                    "-c".to_owned(),
                    "printf 'fn answer() -> i32 { 41 }\\n' > \"$1\"".to_owned(),
                    "real-repair-001".to_owned(),
                    bug_file.to_string_lossy().into_owned(),
                ]
                .into_boxed_slice(),
            )
            .with_started_at_ms(200)
            .with_observed_at_ms(225),
        ))
        .expect("repair tool should spawn");

    assert!(spawn_receipt.output.status.success());
    assert_eq!(
        fs::read_to_string(&bug_file).expect("read repaired fixture"),
        "fn answer() -> i32 { 41 }\n"
    );
    fs::remove_file(&bug_file).expect("remove repair fixture");
}
