use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, GenericLoopMachineReceipt,
    GenericLoopMachineStepIndex, LoopProgramDerivedSessionPolicyStatus,
    LoopProgramRuntimeHandoffExecutor, LoopProgramRuntimeHandoffPlan, LoopProgramRuntimeOwner,
    LoopProgramRuntimeReplayBundleReceipt, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramRuntimeSideEffectReceipt, LoopProgramRuntimeSideEffectStatus,
    LoopProgramToolProcessCommandTemplate, LoopProgramToolProcessProgram,
    LoopProgramToolProcessSideEffectStatus, StaticLoopProgramToolProcessResolver,
};
use marlin_agent_protocol::{
    LoopProgramActionKind, LoopProgramEventKind, LoopProgramId, LoopProgramStateId,
    LoopProgramTransitionId,
};
use marlin_agent_runtime::TokioAgentRuntime;

#[cfg(unix)]
#[tokio::test]
async fn side_effect_executor_spawns_resolved_tool_projection_through_runtime_registry() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let handoff_execution = agent_flow_handoff_execution();
    let executor =
        LoopProgramRuntimeSideEffectExecutor::new(tool_resolver("printf loop-side-effect"))
            .with_started_at_ms(11)
            .with_observed_at_ms(22);

    let receipt = executor
        .execute(&runtime.context(), &handoff_execution)
        .await;

    assert_eq!(
        receipt.status,
        LoopProgramRuntimeSideEffectStatus::Completed
    );
    assert_eq!(receipt.tool_processes.len(), 1);
    let tool_process = &receipt.tool_processes[0];
    assert_eq!(
        tool_process.status,
        LoopProgramToolProcessSideEffectStatus::Completed
    );
    let spawn_receipt = tool_process
        .spawn_receipt
        .as_ref()
        .expect("tool process spawn receipt");
    assert!(spawn_receipt.output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&spawn_receipt.output.stdout),
        "loop-side-effect"
    );
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
async fn side_effect_executor_marks_unresolved_tool_projection_as_skipped() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let handoff_execution = agent_flow_handoff_execution();

    let receipt = LoopProgramRuntimeSideEffectExecutor::default()
        .execute(&runtime.context(), &handoff_execution)
        .await;

    assert_eq!(receipt.status, LoopProgramRuntimeSideEffectStatus::Skipped);
    assert_eq!(receipt.tool_processes.len(), 1);
    assert_eq!(
        receipt.tool_processes[0].status,
        LoopProgramToolProcessSideEffectStatus::Skipped
    );
    assert!(receipt.tool_processes[0].spawn_receipt.is_none());
    assert!(
        runtime
            .context()
            .process_registry()
            .active_processes()
            .is_empty()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn side_effect_executor_reports_failed_tool_process_exit() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let handoff_execution = agent_flow_handoff_execution();

    let receipt = LoopProgramRuntimeSideEffectExecutor::new(tool_resolver("exit 7"))
        .execute(&runtime.context(), &handoff_execution)
        .await;

    assert_eq!(receipt.status, LoopProgramRuntimeSideEffectStatus::Failed);
    assert_eq!(receipt.tool_processes.len(), 1);
    assert_eq!(
        receipt.tool_processes[0].status,
        LoopProgramToolProcessSideEffectStatus::Failed
    );
    let spawn_receipt = receipt.tool_processes[0]
        .spawn_receipt
        .as_ref()
        .expect("failed process still has a runtime spawn receipt");
    assert_eq!(spawn_receipt.output.status.code(), Some(7));
    assert!(!spawn_receipt.output.status.success());
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
async fn replay_bundle_marks_completed_side_effects_ready_without_mutating_agent_flow_receipt() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let handoff_execution = agent_flow_handoff_execution();
    let side_effects = LoopProgramRuntimeSideEffectExecutor::new(tool_resolver("printf ready"))
        .execute(&runtime.context(), &handoff_execution)
        .await;

    let bundle = LoopProgramRuntimeReplayBundleReceipt::from_runtime_receipts(
        handoff_execution.clone(),
        side_effects.clone(),
    );

    assert_eq!(
        bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Ready
    );
    assert!(bundle.allows_replay());
    assert!(!bundle.requires_follow_up());
    assert!(!bundle.blocks_replay());
    assert_eq!(bundle.handoff_execution, handoff_execution);
    assert_eq!(bundle.side_effects, side_effects);
    assert_eq!(
        bundle
            .agent_flow_receipt()
            .expect("Agent-Flow receipt")
            .derived_session
            .session
            .generation,
        1
    );
}

#[cfg(unix)]
#[tokio::test]
async fn replay_bundle_blocks_derived_session_policy_on_failed_side_effects() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let handoff_execution = agent_flow_handoff_execution();
    let side_effects = LoopProgramRuntimeSideEffectExecutor::new(tool_resolver("exit 9"))
        .execute(&runtime.context(), &handoff_execution)
        .await;

    let bundle = LoopProgramRuntimeReplayBundleReceipt::from_runtime_receipts(
        handoff_execution,
        side_effects,
    );

    assert_eq!(
        bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Blocked
    );
    assert!(!bundle.allows_replay());
    assert!(!bundle.requires_follow_up());
    assert!(bundle.blocks_replay());
    assert!(bundle.agent_flow_receipt().is_some());
}

#[cfg(unix)]
#[tokio::test]
async fn replay_bundle_defers_policy_when_side_effect_projection_is_unresolved() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let handoff_execution = agent_flow_handoff_execution();
    let side_effects = LoopProgramRuntimeSideEffectExecutor::default()
        .execute(&runtime.context(), &handoff_execution)
        .await;

    let bundle = LoopProgramRuntimeReplayBundleReceipt::from_runtime_receipts(
        handoff_execution,
        side_effects,
    );

    assert_eq!(
        bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Deferred
    );
    assert!(!bundle.allows_replay());
    assert!(bundle.requires_follow_up());
    assert!(!bundle.blocks_replay());
    assert!(bundle.agent_flow_receipt().is_some());
}

#[tokio::test]
async fn replay_bundle_reports_missing_derived_session_without_side_effect_failure() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let handoff_execution = non_agent_flow_handoff_execution();
    let side_effects = LoopProgramRuntimeSideEffectExecutor::default()
        .execute(&runtime.context(), &handoff_execution)
        .await;

    let bundle = LoopProgramRuntimeReplayBundleReceipt::from_runtime_receipts(
        handoff_execution,
        side_effects,
    );

    assert_eq!(
        bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::MissingDerivedSession
    );
    assert!(!bundle.allows_replay());
    assert!(!bundle.requires_follow_up());
    assert!(bundle.blocks_replay());
    assert!(bundle.agent_flow_receipt().is_none());
}

#[tokio::test]
async fn replay_bundle_blocks_replay_when_runtime_receipts_disagree_on_program_id() {
    let handoff_execution = agent_flow_handoff_execution();
    let side_effects = LoopProgramRuntimeSideEffectReceipt {
        program_id: LoopProgramId::new("other-program"),
        status: LoopProgramRuntimeSideEffectStatus::Empty,
        tool_processes: Vec::new().into_boxed_slice(),
    };

    let bundle = LoopProgramRuntimeReplayBundleReceipt::from_runtime_receipts(
        handoff_execution,
        side_effects,
    );

    assert_eq!(
        bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::ReceiptMismatch
    );
    assert!(!bundle.allows_replay());
    assert!(!bundle.requires_follow_up());
    assert!(bundle.blocks_replay());
    assert!(bundle.agent_flow_receipt().is_some());
}

fn agent_flow_handoff_execution() -> marlin_agent_kernel::LoopProgramRuntimeHandoffExecutionReceipt
{
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("side-effect-program"),
        &[receipt(1, LoopProgramActionKind::DispatchTools)],
    );
    AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
        "agent-flow-runtime",
    ))
    .execute_plan(&plan)
}

fn non_agent_flow_handoff_execution()
-> marlin_agent_kernel::LoopProgramRuntimeHandoffExecutionReceipt {
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("side-effect-program"),
        &[receipt(1, LoopProgramActionKind::InvokeModel)],
    );
    AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
        "agent-flow-runtime",
    ))
    .execute_plan(&plan)
}

fn tool_resolver(script: &'static str) -> StaticLoopProgramToolProcessResolver {
    StaticLoopProgramToolProcessResolver::new(
        vec![
            LoopProgramToolProcessCommandTemplate::new(
                "agent-flow.tool-intent",
                ["loop-program.dispatch-tools"],
                LoopProgramToolProcessProgram::new("/bin/sh"),
            )
            .with_args(["-c", script]),
        ]
        .into_boxed_slice(),
    )
}

fn receipt(step: u64, action: LoopProgramActionKind) -> GenericLoopMachineReceipt {
    GenericLoopMachineReceipt {
        program_id: LoopProgramId::new("program"),
        step_index: GenericLoopMachineStepIndex::new(step),
        transition_id: LoopProgramTransitionId::new(format!("transition-{step}")),
        from: LoopProgramStateId::new("from"),
        event: LoopProgramEventKind::RuntimeReceipt,
        action,
        to: LoopProgramStateId::new("to"),
        stopped: false,
    }
}
