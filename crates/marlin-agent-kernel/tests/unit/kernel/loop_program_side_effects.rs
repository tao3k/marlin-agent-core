use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, GenericLoopMachineReceipt,
    GenericLoopMachineStepIndex, LoopProgramDerivedSessionPolicyStatus, LoopProgramExecutionDriver,
    LoopProgramExecutionRequest, LoopProgramExecutionStatus, LoopProgramFileSandbox,
    LoopProgramFileWriteSideEffectStatus, LoopProgramFileWriteTemplate,
    LoopProgramRuntimeHandoffExecutor, LoopProgramRuntimeHandoffPlan, LoopProgramRuntimeOwner,
    LoopProgramRuntimeReplayBundleReceipt, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramRuntimeSideEffectReceipt, LoopProgramRuntimeSideEffectStatus,
    LoopProgramToolProcessCommandTemplate, LoopProgramToolProcessProgram,
    LoopProgramToolProcessSideEffectStatus, ScriptedLoopProgramEventMapper,
    StaticLoopProgramFileWriteResolver, StaticLoopProgramToolProcessResolver,
};
use marlin_agent_protocol::{
    LoopMechanismPolicyId, LoopPolicyDigest, LoopPolicyEpoch, LoopProgram, LoopProgramActionKind,
    LoopProgramEventKind, LoopProgramId, LoopProgramInput, LoopProgramStateId,
    LoopProgramTransition, LoopProgramTransitionId,
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
async fn side_effect_executor_replays_projected_steps_from_pumped_loop_execution() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let driver = LoopProgramExecutionDriver::new(AgentFlowLoopProgramRuntimeHandoffExecutor::new(
        LoopProgramRuntimeOwner::new("agent-flow-runtime"),
    ))
    .with_event_mapper(ScriptedLoopProgramEventMapper::new(vec![(
        LoopProgramActionKind::DispatchTools,
        LoopProgramEventKind::ToolReceipt,
    )]))
    .with_max_steps(8);

    let execution = driver.run(LoopProgramExecutionRequest::new(
        tool_side_effect_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));
    let side_effect_executor =
        LoopProgramRuntimeSideEffectExecutor::new(tool_resolver("printf pumped-side-effect"));

    let bundle = side_effect_executor
        .execute_loop_execution(&runtime.context(), &execution)
        .await;

    assert_eq!(execution.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(execution.steps.len(), 2);
    assert_eq!(
        bundle.program_id,
        LoopProgramId::new("tool-side-effect-loop")
    );
    assert_eq!(bundle.execution_status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(
        bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Ready
    );
    assert!(bundle.allows_replay());
    assert!(!bundle.requires_follow_up());
    assert!(!bundle.blocks_replay());
    assert_eq!(bundle.step_replay_bundles.len(), 1);

    let step_bundle = &bundle.step_replay_bundles[0];
    assert_eq!(
        step_bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Ready
    );
    assert_eq!(
        step_bundle.side_effects.status,
        LoopProgramRuntimeSideEffectStatus::Completed
    );
    let tool_process = &step_bundle.side_effects.tool_processes[0];
    assert_eq!(
        tool_process.status,
        LoopProgramToolProcessSideEffectStatus::Completed
    );
    let spawn_receipt = tool_process
        .spawn_receipt
        .as_ref()
        .expect("pumped loop should spawn resolved tool projection");
    assert_eq!(
        String::from_utf8_lossy(&spawn_receipt.output.stdout),
        "pumped-side-effect"
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

#[tokio::test]
async fn side_effect_executor_writes_allowed_file_projection_through_sandbox() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let workspace = unique_side_effect_workspace();
    let relative_path = PathBuf::from("src/lib.rs");
    let file_path = workspace.join(&relative_path);
    fs::create_dir_all(file_path.parent().expect("fixture parent")).expect("create fixture dir");
    fs::write(&file_path, "fn answer() -> i32 { 40 }\n").expect("write initial fixture");

    let handoff_execution = agent_flow_handoff_execution();
    let receipt = LoopProgramRuntimeSideEffectExecutor::default()
        .with_file_write_resolver(file_write_resolver(
            relative_path.clone(),
            "fn answer() -> i32 { 41 }\n",
        ))
        .with_file_sandbox(
            LoopProgramFileSandbox::new(workspace.clone())
                .with_allowed_relative_paths([relative_path.clone()]),
        )
        .execute(&runtime.context(), &handoff_execution)
        .await;

    assert_eq!(
        receipt.status,
        LoopProgramRuntimeSideEffectStatus::Completed
    );
    assert!(receipt.tool_processes.is_empty());
    assert_eq!(receipt.file_writes.len(), 1);
    let file_write = &receipt.file_writes[0];
    assert_eq!(
        file_write.status,
        LoopProgramFileWriteSideEffectStatus::Completed
    );
    let write_receipt = file_write
        .write_receipt
        .as_ref()
        .expect("file write receipt");
    assert_eq!(write_receipt.relative_path, relative_path);
    assert_eq!(write_receipt.path, file_path);
    assert_eq!(
        write_receipt.bytes_written,
        "fn answer() -> i32 { 41 }\n".len()
    );
    assert_ne!(
        write_receipt.before_hash.as_ref().expect("before hash"),
        &write_receipt.after_hash
    );
    assert_eq!(
        fs::read_to_string(workspace.join("src/lib.rs")).expect("read repaired fixture"),
        "fn answer() -> i32 { 41 }\n"
    );
    fs::remove_dir_all(&workspace).expect("remove file-write workspace");
}

#[tokio::test]
async fn side_effect_executor_blocks_disallowed_file_projection_before_write() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let workspace = unique_side_effect_workspace();
    fs::create_dir_all(&workspace).expect("create workspace");

    let handoff_execution = agent_flow_handoff_execution();
    let receipt = LoopProgramRuntimeSideEffectExecutor::default()
        .with_file_write_resolver(file_write_resolver("secret.rs", "sealed\n"))
        .with_file_sandbox(
            LoopProgramFileSandbox::new(workspace.clone())
                .with_allowed_relative_paths([PathBuf::from("src/lib.rs")]),
        )
        .execute(&runtime.context(), &handoff_execution)
        .await;

    assert_eq!(receipt.status, LoopProgramRuntimeSideEffectStatus::Failed);
    assert!(receipt.tool_processes.is_empty());
    assert_eq!(receipt.file_writes.len(), 1);
    assert_eq!(
        receipt.file_writes[0].status,
        LoopProgramFileWriteSideEffectStatus::Denied
    );
    assert!(
        receipt.file_writes[0]
            .diagnostic
            .as_deref()
            .expect("sandbox diagnostic")
            .contains("sandbox_denied")
    );
    assert!(!workspace.join("secret.rs").exists());
    fs::remove_dir_all(&workspace).expect("remove denied workspace");
}

#[tokio::test]
async fn side_effect_executor_blocks_traversal_file_projection_before_write() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let workspace = unique_side_effect_workspace();
    fs::create_dir_all(&workspace).expect("create workspace");

    let handoff_execution = agent_flow_handoff_execution();
    let receipt = LoopProgramRuntimeSideEffectExecutor::default()
        .with_file_write_resolver(file_write_resolver("../escape.rs", "escape\n"))
        .with_file_sandbox(
            LoopProgramFileSandbox::new(workspace.clone())
                .with_allowed_relative_paths([PathBuf::from("../escape.rs")]),
        )
        .execute(&runtime.context(), &handoff_execution)
        .await;

    assert_eq!(receipt.status, LoopProgramRuntimeSideEffectStatus::Failed);
    assert_eq!(
        receipt.file_writes[0].status,
        LoopProgramFileWriteSideEffectStatus::Denied
    );
    assert!(
        receipt.file_writes[0]
            .diagnostic
            .as_deref()
            .expect("traversal diagnostic")
            .contains("relative_path_denied")
    );
    assert!(
        fs::read_dir(&workspace)
            .expect("read traversal workspace")
            .next()
            .is_none()
    );
    fs::remove_dir_all(&workspace).expect("remove traversal workspace");
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
        file_writes: Vec::new().into_boxed_slice(),
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

fn file_write_resolver(
    relative_path: impl Into<PathBuf>,
    contents: &'static str,
) -> StaticLoopProgramFileWriteResolver {
    StaticLoopProgramFileWriteResolver::new(
        vec![LoopProgramFileWriteTemplate::new(
            "agent-flow.tool-intent",
            ["loop-program.dispatch-tools"],
            relative_path,
            contents.as_bytes().to_vec(),
        )]
        .into_boxed_slice(),
    )
}

fn unique_side_effect_workspace() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "marlin-loop-side-effect-{}-{nanos}",
        std::process::id()
    ))
}

fn tool_side_effect_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("tool-side-effect-loop"),
        policy_epoch: LoopPolicyEpoch::new(3),
        policy_digest: LoopPolicyDigest::from_bytes([3_u8; 32]),
        mechanism_policies: vec![LoopMechanismPolicyId::new("reactive-tool-loop-side-effect")]
            .into_boxed_slice(),
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
                transition_id: LoopProgramTransitionId::new("tool-stop"),
                from: LoopProgramStateId::new("await-tool"),
                event: LoopProgramEventKind::ToolReceipt,
                action: LoopProgramActionKind::Stop,
                to: LoopProgramStateId::new("stopped"),
            },
        ]
        .into_boxed_slice(),
    })
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
