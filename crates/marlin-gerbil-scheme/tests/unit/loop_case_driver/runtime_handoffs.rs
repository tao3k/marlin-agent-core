use marlin_agent_kernel::{
    LoopProgramDerivedSessionPolicyStatus, LoopProgramExecutionDriver, LoopProgramExecutionRequest,
    LoopProgramExecutionStatus, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffExecutionStatus, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramRuntimeSideEffectStatus, LoopProgramToolProcessSideEffectStatus,
    ReceiptDrivenLoopProgramEventMapper,
};
use marlin_agent_protocol::{LoopProgramActionKind, LoopProgramEventKind};
use marlin_agent_runtime::TokioAgentRuntime;
use marlin_gerbil_scheme::verify_gerbil_loop_case_driver_vertical_trace;

use super::support::{
    cap, run_config_interface_case_driver_smoke, scheme_projected_dispatch_tools_shell_resolver,
    scheme_projected_event_mapper, scheme_projected_loop_program,
    scheme_projected_retry_budget_runtime_executor, scheme_projected_runtime_executor,
    scheme_projected_tool_denial_runtime_executor, tool_projection_count,
};

#[test]
fn config_interface_vertical_trace_drives_projected_tool_denial() {
    let stdout = run_config_interface_case_driver_smoke();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");
    let receipt = vertical_receipts
        .iter()
        .find(|receipt| {
            receipt.has_capability(&cap("+denylist"))
                && receipt
                    .transition_actions()
                    .any(|action| action == "dispatch_tools")
                && receipt.transition_events().any(|event| event == "error")
        })
        .expect("Scheme vertical trace should include a denylist dispatch shape");
    let loop_program = scheme_projected_loop_program(receipt);
    let driver = LoopProgramExecutionDriver::new(scheme_projected_tool_denial_runtime_executor())
        .with_event_mapper(scheme_projected_event_mapper(receipt))
        .with_max_steps(receipt.transition_count() + 2);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert!(execution_receipt.error.is_none());
    let denied_step = execution_receipt
        .steps
        .iter()
        .find(|step| {
            step.runtime_handoff_execution.status
                == LoopProgramRuntimeHandoffExecutionReportStatus::Denied
        })
        .expect("Scheme-projected denylist shape should deny a dispatch handoff");
    assert_eq!(
        denied_step.machine_receipt.action,
        LoopProgramActionKind::DispatchTools
    );
    assert_eq!(
        denied_step.generated_event,
        Some(LoopProgramEventKind::Error)
    );
    let denied_execution = denied_step
        .runtime_handoff_execution
        .executions
        .iter()
        .find(|execution| execution.status == LoopProgramRuntimeHandoffExecutionStatus::Denied)
        .expect("denied report should preserve a denied runtime execution");
    assert_eq!(
        denied_execution.owner.as_str(),
        "scheme.projected.sandbox-denial"
    );
    assert!(
        denied_execution.agent_flow_intent.is_some(),
        "Scheme-projected denied dispatch should preserve a typed Agent-Flow intent"
    );
    assert!(
        execution_receipt.steps.iter().any(|step| {
            step.machine_receipt.action == LoopProgramActionKind::Stop
                && step.runtime_handoff_execution.status
                    == LoopProgramRuntimeHandoffExecutionReportStatus::Completed
        }),
        "Scheme-projected denial should transition through an error receipt to stop: {execution_receipt:?}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn config_interface_vertical_trace_drives_projected_tool_side_effect_spawn() {
    let stdout = run_config_interface_case_driver_smoke();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");
    let receipt = vertical_receipts
        .iter()
        .find(|receipt| receipt.has_capability(&cap("+policy-combination")))
        .expect("Scheme vertical trace should include a policy-combination tool path");
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
        LoopProgramExecutionStatus::Stopped
    );
    let projected_tool_count = tool_projection_count(&execution_receipt);
    assert!(
        projected_tool_count > 0,
        "Scheme-projected policy-combination loop should emit tool projections: {execution_receipt:?}"
    );

    let (runtime, _events) = TokioAgentRuntime::new(4);
    let replay_bundle =
        LoopProgramRuntimeSideEffectExecutor::new(scheme_projected_dispatch_tools_shell_resolver(
            "printf scheme-projected-policy-combination-tool-spawn",
        ))
        .with_started_at_ms(700)
        .with_observed_at_ms(725)
        .execute_loop_execution(&runtime.context(), &execution_receipt)
        .await;

    assert_eq!(
        replay_bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Ready
    );
    assert!(replay_bundle.allows_replay());
    assert!(
        replay_bundle
            .step_replay_bundles
            .iter()
            .any(
                |bundle| bundle.side_effects.status == LoopProgramRuntimeSideEffectStatus::Empty
                    && !bundle.handoff_execution.memory_projections.is_empty()
            ),
        "Scheme-projected memory handoff should stay typed and side-effect empty"
    );
    let tool_processes = replay_bundle
        .step_replay_bundles
        .iter()
        .flat_map(|bundle| bundle.side_effects.tool_processes.iter())
        .collect::<Vec<_>>();
    assert_eq!(tool_processes.len(), projected_tool_count);
    let tool_process = tool_processes[0];
    assert_eq!(
        tool_process.status,
        LoopProgramToolProcessSideEffectStatus::Completed
    );
    let spawn_receipt = tool_process
        .spawn_receipt
        .as_ref()
        .expect("Scheme-projected tool handoff should spawn");
    assert!(spawn_receipt.output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&spawn_receipt.output.stdout),
        "scheme-projected-policy-combination-tool-spawn"
    );
    assert!(spawn_receipt.output.stderr.is_empty());
    assert!(
        runtime
            .context()
            .process_registry()
            .get(spawn_receipt.pid)
            .is_none()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn config_interface_vertical_trace_drives_projected_retry_budget_admission() {
    let stdout = run_config_interface_case_driver_smoke();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");
    let receipt = vertical_receipts
        .iter()
        .find(|receipt| {
            receipt.has_capability(&cap("+retry-budget"))
                && receipt.has_capability(&cap("+failure-policy"))
                && receipt
                    .transition_actions()
                    .filter(|action| *action == "dispatch_tools")
                    .count()
                    >= 2
                && receipt.transition_events().any(|event| event == "error")
                && receipt
                    .transition_events()
                    .any(|event| event == "tool_receipt")
        })
        .expect("Scheme vertical trace should include a retry-budget dispatch shape");
    let loop_program = scheme_projected_loop_program(receipt);
    let driver = LoopProgramExecutionDriver::new(scheme_projected_retry_budget_runtime_executor())
        .with_event_mapper(ReceiptDrivenLoopProgramEventMapper)
        .with_max_steps(receipt.transition_count() + 3);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert!(execution_receipt.error.is_none());
    assert_eq!(execution_receipt.steps.len(), receipt.transition_count());
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Stop,
        ]
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.runtime_handoff_execution.status.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramRuntimeHandoffExecutionReportStatus::Denied,
            LoopProgramRuntimeHandoffExecutionReportStatus::Completed,
            LoopProgramRuntimeHandoffExecutionReportStatus::Completed,
        ]
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.generated_event.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(LoopProgramEventKind::Error),
            Some(LoopProgramEventKind::ToolReceipt),
            None,
        ]
    );
    assert_eq!(
        execution_receipt.steps[0]
            .runtime_handoff_execution
            .tool_process_projections
            .len(),
        0
    );
    assert_eq!(
        execution_receipt.steps[1]
            .runtime_handoff_execution
            .tool_process_projections
            .len(),
        1
    );
    assert_eq!(
        execution_receipt.steps[1]
            .runtime_handoff_execution
            .executions[0]
            .owner
            .as_str(),
        "scheme.projected.retry-budget-tool"
    );
    assert_eq!(
        execution_receipt.steps[1]
            .runtime_handoff_execution
            .tool_process_projections[0]
            .owner
            .as_str(),
        "scheme.projected.agent-flow.retry-budget-tool"
    );

    let (runtime, _events) = TokioAgentRuntime::new(4);
    let replay_bundle =
        LoopProgramRuntimeSideEffectExecutor::new(scheme_projected_dispatch_tools_shell_resolver(
            "printf scheme-projected-retry-budget-admitted-tool-spawn",
        ))
        .with_started_at_ms(900)
        .with_observed_at_ms(925)
        .execute_loop_execution(&runtime.context(), &execution_receipt)
        .await;

    assert_eq!(
        replay_bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Ready
    );
    assert!(replay_bundle.allows_replay());
    assert_eq!(replay_bundle.step_replay_bundles.len(), 1);
    let tool_process = &replay_bundle.step_replay_bundles[0]
        .side_effects
        .tool_processes[0];
    assert_eq!(
        tool_process.status,
        LoopProgramToolProcessSideEffectStatus::Completed
    );
    let spawn_receipt = tool_process
        .spawn_receipt
        .as_ref()
        .expect("Scheme-projected retry-admitted tool should spawn");
    assert!(spawn_receipt.output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&spawn_receipt.output.stdout),
        "scheme-projected-retry-budget-admitted-tool-spawn"
    );
    assert!(spawn_receipt.output.stderr.is_empty());
    assert!(
        runtime
            .context()
            .process_registry()
            .get(spawn_receipt.pid)
            .is_none()
    );
}
