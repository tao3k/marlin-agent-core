use super::{
    LoopProgramActionKind, LoopProgramEventKind, LoopProgramExecutionDriver,
    LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffRouter,
    LoopProgramRuntimeHandoffRouterHandlers, ReceiptDrivenLoopProgramEventMapper,
    decode_gerbil_poo_loop_program_compiler_receipt, handled_by, handled_by_with_event,
    poo_loop_program_compiler_envelope, poo_loop_program_compiler_fixture,
    poo_loop_program_compiler_registry, runtime_reactive_tool_loop_script,
};

#[test]
fn poo_loop_program_compiler_receipt_runs_scripted_loop_through_kernel_driver() {
    let registry = poo_loop_program_compiler_registry();
    let envelope = poo_loop_program_compiler_envelope(poo_loop_program_compiler_fixture([7; 32]));
    let compiler_receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect("POO loop program compiler receipt decodes");

    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        control_handler: handled_by("runtime.control"),
        model_handler: handled_by("runtime.model"),
        tool_handler: handled_by("runtime.tool"),
        graph_handler: handled_by("runtime.graph"),
        verification_handler: handled_by("runtime.verification"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(runtime_reactive_tool_loop_script())
        .with_max_steps(16);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        compiler_receipt.loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert!(execution_receipt.error.is_none());
    assert_eq!(execution_receipt.steps.len(), 6);
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Continue,
            LoopProgramActionKind::RewriteGraph,
            LoopProgramActionKind::Verify,
            LoopProgramActionKind::Stop,
        ]
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.generated_event.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(LoopProgramEventKind::ToolRequest),
            Some(LoopProgramEventKind::ToolReceipt),
            Some(LoopProgramEventKind::ModelEvent),
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::VerificationReceipt),
            None,
        ]
    );
    assert!(execution_receipt.steps.iter().all(|step| {
        step.runtime_handoff_plan.handoffs.len() == 1
            && step.runtime_handoff_execution.status
                == LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    }));
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.runtime_handoff_execution.executions[0].owner.as_str())
            .collect::<Vec<_>>(),
        vec![
            "runtime.model",
            "runtime.tool",
            "runtime.control",
            "runtime.graph",
            "runtime.verification",
            "runtime.control",
        ]
    );
}

#[test]
fn poo_loop_program_compiler_receipt_runs_runtime_loop_from_runtime_receipts() {
    let registry = poo_loop_program_compiler_registry();
    let envelope = poo_loop_program_compiler_envelope(poo_loop_program_compiler_fixture([7; 32]));
    let compiler_receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect("POO loop program compiler receipt decodes");

    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        control_handler: handled_by_with_event("runtime.control", LoopProgramEventKind::ModelEvent),
        model_handler: handled_by_with_event("runtime.model", LoopProgramEventKind::ToolRequest),
        tool_handler: handled_by_with_event("runtime.tool", LoopProgramEventKind::ToolReceipt),
        graph_handler: handled_by_with_event("runtime.graph", LoopProgramEventKind::RuntimeReceipt),
        verification_handler: handled_by_with_event(
            "runtime.verification",
            LoopProgramEventKind::VerificationReceipt,
        ),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(ReceiptDrivenLoopProgramEventMapper)
        .with_max_steps(16);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        compiler_receipt.loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert!(execution_receipt.error.is_none());
    assert_eq!(execution_receipt.steps.len(), 6);
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Continue,
            LoopProgramActionKind::RewriteGraph,
            LoopProgramActionKind::Verify,
            LoopProgramActionKind::Stop,
        ]
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.generated_event.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(LoopProgramEventKind::ToolRequest),
            Some(LoopProgramEventKind::ToolReceipt),
            Some(LoopProgramEventKind::ModelEvent),
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::VerificationReceipt),
            None,
        ]
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .filter_map(|step| step.runtime_handoff_execution.executions[0]
                .next_event
                .clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramEventKind::ToolRequest,
            LoopProgramEventKind::ToolReceipt,
            LoopProgramEventKind::ModelEvent,
            LoopProgramEventKind::RuntimeReceipt,
            LoopProgramEventKind::VerificationReceipt,
            LoopProgramEventKind::ModelEvent,
        ]
    );
}
