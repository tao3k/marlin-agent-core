use std::{fs, path::PathBuf, sync::Arc};

use super::{
    GatewayRepairDecisionMapper, LoopProgramActionKind, LoopProgramEventKind,
    LoopProgramExecutionDriver, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramFileSandbox, LoopProgramFileWriteSideEffectStatus, LoopProgramFileWriteTemplate,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffRouter,
    LoopProgramRuntimeHandoffRouterHandlers, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramRuntimeSideEffectStatus, ModelGatewayMessageRole,
    RealRepairPolicyGatedHandoffExecutor, RuntimeEdgeModelGateway, RuntimeEdgePolicy,
    StaticLoopProgramFileWriteResolver, StaticLoopProgramToolProcessResolver, StaticRepairGateway,
    TokioAgentRuntime, handled_by, runtime_repair_no_write_llm_loop_program,
    runtime_repair_single_file_loop_program, unique_temp_repair_workspace,
};

#[test]
fn runtime_repair_no_write_llm_repair_receipt_selects_single_file_patch() {
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
        runtime_repair_no_write_llm_loop_program(),
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
fn runtime_repair_single_file_bug_fix_runs_llm_tool_and_verifier_loop() {
    let workspace = unique_temp_repair_workspace();
    let bug_relative_path = PathBuf::from("src/lib.rs");
    let bug_file = workspace.join(&bug_relative_path);
    fs::create_dir_all(bug_file.parent().expect("bug fixture parent"))
        .expect("create bug fixture dir");
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
    let executor = RealRepairPolicyGatedHandoffExecutor::new();
    let driver = LoopProgramExecutionDriver::new(executor).with_event_mapper(mapper);

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        runtime_repair_single_file_loop_program(),
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
        tool_step.runtime_handoff_execution.executions[0]
            .owner
            .as_str(),
        "runtime.policy.repair-tool-admission"
    );
    assert_eq!(
        tool_step.runtime_handoff_execution.tool_process_projections[0]
            .owner
            .as_str(),
        "runtime.agent-flow.repair-tool"
    );
    assert_eq!(
        tool_step
            .runtime_handoff_execution
            .tool_process_projections
            .len(),
        1
    );

    let (runtime, _events) = TokioAgentRuntime::new(4);
    let replay_bundle = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("side-effect runtime")
        .block_on(
            LoopProgramRuntimeSideEffectExecutor::new(
                StaticLoopProgramToolProcessResolver::default(),
            )
            .with_file_write_resolver(StaticLoopProgramFileWriteResolver::new(
                vec![LoopProgramFileWriteTemplate::new(
                    "agent-flow.tool-intent",
                    ["loop-program.dispatch-tools"],
                    bug_relative_path.clone(),
                    b"fn answer() -> i32 { 41 }\n".to_vec(),
                )]
                .into_boxed_slice(),
            ))
            .with_file_sandbox(
                LoopProgramFileSandbox::new(workspace.clone())
                    .with_allowed_relative_paths([bug_relative_path.clone()]),
            )
            .execute_loop_execution(&runtime.context(), &receipt),
        );

    assert_eq!(
        replay_bundle.policy_status,
        super::LoopProgramDerivedSessionPolicyStatus::Ready
    );
    assert!(replay_bundle.allows_replay());
    assert_eq!(replay_bundle.step_replay_bundles.len(), 1);
    assert_eq!(
        replay_bundle.step_replay_bundles[0].side_effects.status,
        LoopProgramRuntimeSideEffectStatus::Completed
    );
    assert!(
        replay_bundle.step_replay_bundles[0]
            .side_effects
            .tool_processes
            .is_empty()
    );
    assert_eq!(
        replay_bundle.step_replay_bundles[0]
            .side_effects
            .file_writes
            .len(),
        1
    );
    let file_write = &replay_bundle.step_replay_bundles[0]
        .side_effects
        .file_writes[0];
    assert_eq!(
        file_write.status,
        LoopProgramFileWriteSideEffectStatus::Completed
    );
    let write_receipt = file_write
        .write_receipt
        .as_ref()
        .expect("real repair file-write receipt");
    assert_eq!(write_receipt.relative_path, bug_relative_path);
    assert_eq!(write_receipt.path, bug_file);
    assert_eq!(
        write_receipt.bytes_written,
        "fn answer() -> i32 { 41 }\n".len()
    );
    assert_ne!(
        write_receipt.before_hash.as_ref().expect("before hash"),
        &write_receipt.after_hash
    );
    assert_eq!(
        fs::read_to_string(&bug_file).expect("read repaired fixture"),
        "fn answer() -> i32 { 41 }\n"
    );
    fs::remove_dir_all(&workspace).expect("remove repair workspace");
}
