use std::sync::Arc;

use marlin_agent_kernel::{
    LoopProgramEventMapper, LoopProgramExecutionDriver, LoopProgramExecutionRequest,
    LoopProgramExecutionStatus, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffExecutionStatus, LoopProgramRuntimeHandoffHandler,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, ScriptedLoopProgramEventMapper,
    StaticLoopProgramRuntimeHandoffHandler,
};
use marlin_agent_protocol::{
    LoopMechanismPolicyId, LoopPolicyDigest, LoopPolicyEpoch, LoopProgram, LoopProgramActionKind,
    LoopProgramEventKind, LoopProgramId, LoopProgramInput, LoopProgramStateId,
    LoopProgramTransition, LoopProgramTransitionId,
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

fn loop_script() -> impl LoopProgramEventMapper {
    ScriptedLoopProgramEventMapper::new(
        vec![
            (
                LoopProgramActionKind::InvokeModel,
                LoopProgramEventKind::ToolRequest,
            ),
            (
                LoopProgramActionKind::DispatchTools,
                LoopProgramEventKind::ToolReceipt,
            ),
            (
                LoopProgramActionKind::Continue,
                LoopProgramEventKind::ModelEvent,
            ),
            (
                LoopProgramActionKind::RewriteGraph,
                LoopProgramEventKind::RuntimeReceipt,
            ),
            (
                LoopProgramActionKind::Verify,
                LoopProgramEventKind::VerificationReceipt,
            ),
        ]
        .into_boxed_slice(),
    )
}

fn handled_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
        LoopProgramRuntimeOwner::new(owner),
    ))
}

fn denied_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::denied(
        LoopProgramRuntimeOwner::new(owner),
    ))
}

fn sample_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("execution-driver-loop"),
        policy_epoch: LoopPolicyEpoch::new(8),
        policy_digest: LoopPolicyDigest::from_bytes([7_u8; 32]),
        mechanism_policies: vec![LoopMechanismPolicyId::new("reactive-tool-loop-base")]
            .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("start-model"),
                from: LoopProgramStateId::new("start"),
                event: LoopProgramEventKind::Start,
                action: LoopProgramActionKind::InvokeModel,
                to: LoopProgramStateId::new("await-model"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("model-tools"),
                from: LoopProgramStateId::new("await-model"),
                event: LoopProgramEventKind::ToolRequest,
                action: LoopProgramActionKind::DispatchTools,
                to: LoopProgramStateId::new("await-tools"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("tools-continue"),
                from: LoopProgramStateId::new("await-tools"),
                event: LoopProgramEventKind::ToolReceipt,
                action: LoopProgramActionKind::Continue,
                to: LoopProgramStateId::new("await-model"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("dynamic-rewrite"),
                from: LoopProgramStateId::new("await-model"),
                event: LoopProgramEventKind::ModelEvent,
                action: LoopProgramActionKind::RewriteGraph,
                to: LoopProgramStateId::new("rewritten"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("verify-rewrite"),
                from: LoopProgramStateId::new("rewritten"),
                event: LoopProgramEventKind::RuntimeReceipt,
                action: LoopProgramActionKind::Verify,
                to: LoopProgramStateId::new("verifying"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("verification-stop"),
                from: LoopProgramStateId::new("verifying"),
                event: LoopProgramEventKind::VerificationReceipt,
                action: LoopProgramActionKind::Stop,
                to: LoopProgramStateId::new("stopped"),
            },
        ]
        .into_boxed_slice(),
    })
}

fn tool_error_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("execution-driver-tool-error-loop"),
        policy_epoch: LoopPolicyEpoch::new(9),
        policy_digest: LoopPolicyDigest::from_bytes([8_u8; 32]),
        mechanism_policies: vec![LoopMechanismPolicyId::new("tool-error-route")].into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("start-tool"),
                from: LoopProgramStateId::new("start"),
                event: LoopProgramEventKind::Start,
                action: LoopProgramActionKind::DispatchTools,
                to: LoopProgramStateId::new("await-tool"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("tool-error-stop"),
                from: LoopProgramStateId::new("await-tool"),
                event: LoopProgramEventKind::Error,
                action: LoopProgramActionKind::Stop,
                to: LoopProgramStateId::new("stopped"),
            },
        ]
        .into_boxed_slice(),
    })
}
