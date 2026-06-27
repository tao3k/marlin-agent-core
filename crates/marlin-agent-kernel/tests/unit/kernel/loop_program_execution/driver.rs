use super::{
    LoopProgramActionKind, LoopProgramEventKind, LoopProgramExecutionDriver,
    LoopProgramExecutionRequest, LoopProgramExecutionStatus, LoopProgramId,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffExecutionStatus,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    ReceiptDrivenLoopProgramEventMapper, ScriptedLoopProgramEventMapper, denied_by, handled_by,
    handled_by_with_event, loop_script, sample_loop_program, tool_error_loop_program,
};

#[test]
fn execution_driver_pumps_runtime_receipts_into_follow_up_events_until_stop() {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        control_handler: handled_by("runtime.control"),
        model_handler: handled_by("runtime.model"),
        tool_handler: handled_by("runtime.tool"),
        graph_handler: handled_by("runtime.graph"),
        verification_handler: handled_by("runtime.verification"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(loop_script())
        .with_max_steps(16);

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        sample_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        receipt.program_id,
        LoopProgramId::new("execution-driver-loop")
    );
    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert!(receipt.error.is_none());
    assert_eq!(receipt.steps.len(), 6);
    assert_eq!(
        receipt
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
        receipt
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
    assert!(receipt.steps.iter().all(|step| {
        step.runtime_handoff_plan.handoffs.len() == 1
            && step.runtime_handoff_execution.status
                == LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    }));
    assert_eq!(
        receipt
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
fn execution_driver_can_pump_events_from_runtime_handoff_receipts() {
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

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        sample_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert!(receipt.error.is_none());
    assert_eq!(
        receipt
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
        receipt.steps[0].runtime_handoff_execution.executions[0].next_event,
        Some(LoopProgramEventKind::ToolRequest)
    );
    assert_eq!(
        receipt.steps[5].runtime_handoff_execution.executions[0].next_event,
        Some(LoopProgramEventKind::ModelEvent)
    );
    assert_eq!(receipt.steps[5].generated_event, None);
}

#[test]
fn execution_driver_does_not_fabricate_next_events_for_deferred_runtime_work() {
    let driver = LoopProgramExecutionDriver::default().with_event_mapper(loop_script());

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        sample_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Completed);
    assert_eq!(receipt.steps.len(), 1);
    assert_eq!(
        receipt.steps[0].runtime_handoff_execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Deferred
    );
    assert_eq!(receipt.steps[0].generated_event, None);
}

#[test]
fn execution_driver_routes_denied_runtime_receipts_to_error_without_scripted_events() {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        tool_handler: denied_by("runtime.tool-policy"),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(ReceiptDrivenLoopProgramEventMapper);

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        tool_error_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(
        receipt.steps[0].runtime_handoff_execution.executions[0].status,
        LoopProgramRuntimeHandoffExecutionStatus::Denied
    );
    assert_eq!(
        receipt.steps[0].generated_event,
        Some(LoopProgramEventKind::Error)
    );
    assert_eq!(
        receipt.steps[1].machine_receipt.action,
        LoopProgramActionKind::Stop
    );
}

#[test]
fn execution_driver_can_route_denied_runtime_work_through_error_transition() {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        tool_handler: denied_by("runtime.tool-policy"),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(ScriptedLoopProgramEventMapper::default());

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        tool_error_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(receipt.steps.len(), 2);
    assert_eq!(
        receipt.steps[0].runtime_handoff_execution.executions[0].status,
        LoopProgramRuntimeHandoffExecutionStatus::Denied
    );
    assert_eq!(
        receipt.steps[0].generated_event,
        Some(LoopProgramEventKind::Error)
    );
    assert_eq!(
        receipt.steps[1].machine_receipt.action,
        LoopProgramActionKind::Stop
    );
}
