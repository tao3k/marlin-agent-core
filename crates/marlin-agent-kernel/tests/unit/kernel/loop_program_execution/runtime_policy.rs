use std::sync::Arc;

use marlin_agent_kernel::{RuntimePolicyRecommendationPriority, runtime_policy_experiment_receipt};

use super::LoopProgram;
use super::{
    AgentFlowIntent, AgentFlowLoopProgramRuntimeHandoffExecutor, AgentFlowMemoryOperation,
    DenylistedLoopProgramToolDispatchHandler, HybridLoopProgramRuntimeHandoffExecutor,
    LoopProgramActionKind, LoopProgramDerivedSessionPolicyStatus, LoopProgramEventKind,
    LoopProgramExecutionDriver, LoopProgramExecutionReceipt, LoopProgramExecutionRequest,
    LoopProgramExecutionStatus, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffExecutionStatus, LoopProgramRuntimeHandoffRouter,
    LoopProgramRuntimeHandoffRouterHandlers, LoopProgramRuntimeOwner,
    LoopProgramRuntimeSideEffectExecutor, LoopProgramRuntimeSideEffectStatus,
    LoopProgramToolProcessCommandTemplate, LoopProgramToolProcessProgram,
    LoopProgramToolProcessSideEffectStatus, MemoryRecallDecisionMapper,
    PolicyCombinationDecisionMapper, PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor,
    ReceiptDrivenLoopProgramEventMapper, RetryBudgetToolHandler, ScriptedLoopProgramEventMapper,
    StaticLoopProgramToolProcessResolver, TokioAgentRuntime, handled_by, handled_by_with_event,
    memory_model_rewrite_tool_verify_loop_program, memory_recall_then_tool_loop_program,
    model_then_verify_loop_program, rewrite_tool_verify_loop_program,
    single_tool_dispatch_error_stop_loop_program, single_tool_dispatch_receipt_stop_loop_program,
    two_attempt_tool_dispatch_loop_program,
};

fn dispatch_tools_shell_resolver(script: &'static str) -> StaticLoopProgramToolProcessResolver {
    StaticLoopProgramToolProcessResolver::new(
        vec![
            LoopProgramToolProcessCommandTemplate::new(
                "agent-flow.tool-intent",
                ["loop-program.dispatch-tools"],
                LoopProgramToolProcessProgram::new("sh"),
            )
            .with_args(["-c", script]),
        ]
        .into_boxed_slice(),
    )
}

#[test]
fn runtime_policy_tool_denylist_blocks_dispatch_tool_intent() {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        tool_handler: Arc::new(DenylistedLoopProgramToolDispatchHandler::new(
            LoopProgramRuntimeOwner::new("runtime.sandbox.denylist"),
            ["loop-program.dispatch-tools"],
        )),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(ScriptedLoopProgramEventMapper::default());

    let loop_program = single_tool_dispatch_error_stop_loop_program();
    let receipt = driver.run(LoopProgramExecutionRequest::new(
        loop_program.clone(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(receipt.steps.len(), 2);
    let denied_execution = &receipt.steps[0].runtime_handoff_execution.executions[0];
    assert_eq!(denied_execution.owner.as_str(), "runtime.sandbox.denylist");
    assert_eq!(
        denied_execution.status,
        LoopProgramRuntimeHandoffExecutionStatus::Denied
    );
    assert_eq!(
        receipt.steps[0].generated_event,
        Some(LoopProgramEventKind::Error)
    );
    let Some(AgentFlowIntent::Tool(tool_intent)) = &denied_execution.agent_flow_intent else {
        panic!("denylisted dispatch should preserve the typed tool intent");
    };
    assert_eq!(
        tool_intent.tool_name.as_str(),
        "loop-program.dispatch-tools"
    );

    let experiment_receipt =
        runtime_policy_experiment_receipt("runtime-tool-denylist", &loop_program, &receipt);
    assert_eq!(
        experiment_receipt.program_id.as_str(),
        "kernel-fixture-tool-dispatch-error-stop"
    );
    assert_eq!(
        experiment_receipt.policy_ids[0].as_str(),
        "kernel-fixture-tool-dispatch-error"
    );
    assert_eq!(experiment_receipt.denied_handoff_count.get(), 1);
    assert!(
        experiment_receipt
            .improvement_recommendations
            .iter()
            .any(
                |recommendation| recommendation.target.as_str() == "runtime.sandbox.denylist"
                    && recommendation.priority == RuntimePolicyRecommendationPriority::P0
            )
    );
}

#[cfg(unix)]
#[tokio::test]
async fn tool_sandbox_loop_projects_and_spawns_allowed_tool_process() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let driver = LoopProgramExecutionDriver::new(AgentFlowLoopProgramRuntimeHandoffExecutor::new(
        LoopProgramRuntimeOwner::new("runtime.agent-flow.tool-sandbox"),
    ))
    .with_event_mapper(ScriptedLoopProgramEventMapper::new(
        vec![(
            LoopProgramActionKind::DispatchTools,
            LoopProgramEventKind::ToolReceipt,
        )]
        .into_boxed_slice(),
    ));

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        single_tool_dispatch_receipt_stop_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(receipt.steps.len(), 2);
    let tool_step = &receipt.steps[0];
    assert_eq!(
        tool_step.runtime_handoff_execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    );
    assert_eq!(
        tool_step
            .runtime_handoff_execution
            .agent_flow_receipt
            .as_ref()
            .expect("Agent-Flow handoff receipt")
            .handoff
            .intents
            .len(),
        1
    );
    assert_eq!(
        tool_step
            .runtime_handoff_execution
            .tool_process_projections
            .len(),
        1
    );

    let replay_bundle = LoopProgramRuntimeSideEffectExecutor::new(dispatch_tools_shell_resolver(
        "printf runtime-tool-sandbox-loop",
    ))
    .with_started_at_ms(100)
    .with_observed_at_ms(125)
    .execute_loop_execution(&runtime.context(), &receipt)
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
        .expect("allowed tool projection should spawn");
    assert!(spawn_receipt.output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&spawn_receipt.output.stdout),
        "runtime-tool-sandbox-loop"
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

#[test]
fn runtime_policy_retry_budget_replays_after_first_denied_dispatch() {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        tool_handler: Arc::new(RetryBudgetToolHandler::new(
            LoopProgramRuntimeOwner::new("runtime.retry-budget.tool"),
            1,
        )),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(ScriptedLoopProgramEventMapper::new(
            vec![(
                LoopProgramActionKind::DispatchTools,
                LoopProgramEventKind::ToolReceipt,
            )]
            .into_boxed_slice(),
        ))
        .with_max_steps(8);

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        two_attempt_tool_dispatch_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(receipt.steps.len(), 3);
    assert_eq!(
        receipt
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
        receipt
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
        receipt
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
}

#[cfg(unix)]
#[tokio::test]
async fn runtime_policy_retry_budget_gates_agent_flow_tool_projection_before_spawn() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        tool_handler: Arc::new(RetryBudgetToolHandler::new(
            LoopProgramRuntimeOwner::new("runtime.retry-budget.tool"),
            1,
        )),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(
        PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor::new(
            LoopProgramRuntimeHandoffRouter::new(handlers),
            AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
                "runtime.agent-flow.retry-budget-tool",
            )),
        ),
    )
    .with_event_mapper(ReceiptDrivenLoopProgramEventMapper)
    .with_max_steps(8);

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        two_attempt_tool_dispatch_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(receipt.steps.len(), 3);
    assert_eq!(
        receipt
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
        receipt
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
        receipt.steps[0]
            .runtime_handoff_execution
            .tool_process_projections
            .len(),
        0
    );
    assert_eq!(
        receipt.steps[1]
            .runtime_handoff_execution
            .tool_process_projections
            .len(),
        1
    );
    assert_eq!(
        receipt.steps[1].runtime_handoff_execution.executions[0]
            .owner
            .as_str(),
        "runtime.retry-budget.tool"
    );
    assert_eq!(
        receipt.steps[1].runtime_handoff_execution.executions[0].next_event,
        Some(LoopProgramEventKind::ToolReceipt)
    );
    assert_eq!(
        receipt.steps[1]
            .runtime_handoff_execution
            .tool_process_projections[0]
            .owner
            .as_str(),
        "runtime.agent-flow.retry-budget-tool"
    );

    let replay_bundle = LoopProgramRuntimeSideEffectExecutor::new(dispatch_tools_shell_resolver(
        "printf retry-budget-admitted-tool-spawn",
    ))
    .with_started_at_ms(400)
    .with_observed_at_ms(425)
    .execute_loop_execution(&runtime.context(), &receipt)
    .await;

    assert_eq!(
        replay_bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Ready
    );
    assert!(replay_bundle.allows_replay());
    assert_eq!(replay_bundle.step_replay_bundles.len(), 1);
    let tool_bundle = &replay_bundle.step_replay_bundles[0];
    assert_eq!(
        tool_bundle.side_effects.status,
        LoopProgramRuntimeSideEffectStatus::Completed
    );
    let tool_process = &tool_bundle.side_effects.tool_processes[0];
    assert_eq!(
        tool_process.status,
        LoopProgramToolProcessSideEffectStatus::Completed
    );
    let spawn_receipt = tool_process
        .spawn_receipt
        .as_ref()
        .expect("retry-admitted tool projection should spawn");
    assert!(spawn_receipt.output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&spawn_receipt.output.stdout),
        "retry-budget-admitted-tool-spawn"
    );
    assert!(spawn_receipt.output.stderr.is_empty());
}

#[test]
fn runtime_policy_maker_checker_routes_model_and_verification_lanes_apart() {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        model_handler: handled_by("runtime.model.maker"),
        verification_handler: handled_by("runtime.verification.checker"),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(ScriptedLoopProgramEventMapper::new(
            vec![
                (
                    LoopProgramActionKind::InvokeModel,
                    LoopProgramEventKind::ModelEvent,
                ),
                (
                    LoopProgramActionKind::Verify,
                    LoopProgramEventKind::VerificationReceipt,
                ),
            ]
            .into_boxed_slice(),
        ));

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        model_then_verify_loop_program(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(
        receipt
            .steps
            .iter()
            .map(|step| step.runtime_handoff_execution.executions[0].owner.as_str())
            .collect::<Vec<_>>(),
        vec![
            "runtime.model.maker",
            "runtime.verification.checker",
            "runtime.control",
        ]
    );
    assert_eq!(
        receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::Verify,
            LoopProgramActionKind::Stop,
        ]
    );
}

#[test]
fn runtime_policy_dynamic_rewrite_runs_before_repair_and_verification() {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        graph_handler: handled_by("runtime.graph.dynamic-rewrite"),
        tool_handler: handled_by("runtime.tool.repair"),
        verification_handler: handled_by("runtime.verification.post-rewrite"),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(ScriptedLoopProgramEventMapper::new(
            vec![
                (
                    LoopProgramActionKind::RewriteGraph,
                    LoopProgramEventKind::RuntimeReceipt,
                ),
                (
                    LoopProgramActionKind::DispatchTools,
                    LoopProgramEventKind::ToolReceipt,
                ),
                (
                    LoopProgramActionKind::Verify,
                    LoopProgramEventKind::VerificationReceipt,
                ),
            ]
            .into_boxed_slice(),
        ));

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        rewrite_tool_verify_loop_program(),
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
            LoopProgramActionKind::RewriteGraph,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Verify,
            LoopProgramActionKind::Stop,
        ]
    );
    assert_eq!(
        receipt.steps[0].runtime_handoff_execution.executions[0]
            .owner
            .as_str(),
        "runtime.graph.dynamic-rewrite"
    );
}

#[test]
fn runtime_policy_memory_recall_receipt_changes_next_decision() {
    let driver = LoopProgramExecutionDriver::new(AgentFlowLoopProgramRuntimeHandoffExecutor::new(
        LoopProgramRuntimeOwner::new("runtime.agent-flow.memory-policy"),
    ))
    .with_event_mapper(MemoryRecallDecisionMapper);

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        memory_recall_then_tool_loop_program(),
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
            LoopProgramActionKind::ReadMemory,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Stop,
        ]
    );
    assert_eq!(
        receipt.steps[0]
            .runtime_handoff_execution
            .memory_projections[0]
            .intent
            .operation,
        AgentFlowMemoryOperation::Recall
    );
    assert_eq!(
        receipt.steps[0].generated_event,
        Some(LoopProgramEventKind::ToolRequest)
    );
}

fn run_memory_rewrite_checker_loop() -> (LoopProgram, LoopProgramExecutionReceipt) {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        model_handler: handled_by("runtime.model.maker"),
        graph_handler: handled_by("runtime.graph.dynamic-rewrite"),
        verification_handler: handled_by("runtime.verification.checker"),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(HybridLoopProgramRuntimeHandoffExecutor::new(
        LoopProgramRuntimeHandoffRouter::new(handlers),
        AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
            "runtime.agent-flow.policy-combination",
        )),
    ))
    .with_event_mapper(PolicyCombinationDecisionMapper);

    let loop_program = memory_model_rewrite_tool_verify_loop_program();
    let receipt = driver.run(LoopProgramExecutionRequest::new(
        loop_program.clone(),
        vec![LoopProgramEventKind::Start],
    ));

    (loop_program, receipt)
}

fn run_receipt_driven_memory_rewrite_checker_loop() -> (LoopProgram, LoopProgramExecutionReceipt) {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        model_handler: handled_by_with_event(
            "runtime.model.maker",
            LoopProgramEventKind::ModelEvent,
        ),
        graph_handler: handled_by_with_event(
            "runtime.graph.dynamic-rewrite",
            LoopProgramEventKind::RuntimeReceipt,
        ),
        verification_handler: handled_by_with_event(
            "runtime.verification.checker",
            LoopProgramEventKind::VerificationReceipt,
        ),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(HybridLoopProgramRuntimeHandoffExecutor::new(
        LoopProgramRuntimeHandoffRouter::new(handlers),
        AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
            "runtime.agent-flow.policy-combination",
        )),
    ))
    .with_event_mapper(ReceiptDrivenLoopProgramEventMapper);

    let loop_program = memory_model_rewrite_tool_verify_loop_program();
    let receipt = driver.run(LoopProgramExecutionRequest::new(
        loop_program.clone(),
        vec![LoopProgramEventKind::Start],
    ));

    (loop_program, receipt)
}

#[test]
fn memory_rewrite_checker_combination_runs_path() {
    let (loop_program, receipt) = run_memory_rewrite_checker_loop();

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(
        receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::ReadMemory,
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::RewriteGraph,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Verify,
            LoopProgramActionKind::Stop,
        ]
    );
    assert_eq!(
        receipt
            .steps
            .iter()
            .map(|step| step.runtime_handoff_execution.executions[0].owner.as_str())
            .collect::<Vec<_>>(),
        vec![
            "runtime.agent-flow.policy-combination",
            "runtime.model.maker",
            "runtime.graph.dynamic-rewrite",
            "runtime.agent-flow.policy-combination",
            "runtime.verification.checker",
            "runtime.control",
        ]
    );
    let Some(AgentFlowIntent::Memory(memory_intent)) =
        &receipt.steps[0].runtime_handoff_execution.executions[0].agent_flow_intent
    else {
        panic!("memory combination case should preserve typed memory intent");
    };
    assert_eq!(memory_intent.operation, AgentFlowMemoryOperation::Recall);
    assert_eq!(
        receipt.steps[0]
            .runtime_handoff_execution
            .memory_projections
            .len(),
        1
    );
    assert_eq!(
        receipt.steps[3]
            .runtime_handoff_execution
            .tool_process_projections
            .len(),
        1
    );

    let experiment_receipt =
        runtime_policy_experiment_receipt("runtime-policy-combination", &loop_program, &receipt);
    assert_eq!(
        experiment_receipt.program_id.as_str(),
        "kernel-fixture-memory-model-rewrite-tool-verify"
    );
    assert_eq!(
        experiment_receipt
            .policy_ids
            .iter()
            .map(|policy| policy.as_str())
            .collect::<Vec<_>>(),
        vec![
            "kernel-fixture-model-verify",
            "kernel-fixture-rewrite-tool",
            "kernel-fixture-memory-recall",
        ]
    );
    assert_eq!(experiment_receipt.policy_digest.as_str(), "0f".repeat(32));
    assert!(
        experiment_receipt
            .loop_program_digest
            .as_str()
            .starts_with("fnv1a64:")
    );
    assert!(
        experiment_receipt
            .runtime_behavior_digest
            .as_str()
            .starts_with("fnv1a64:")
    );
    assert!(
        experiment_receipt
            .receipt_digest
            .as_str()
            .starts_with("fnv1a64:")
    );
    assert_ne!(
        experiment_receipt.loop_program_digest,
        experiment_receipt.runtime_behavior_digest
    );
    assert_ne!(
        experiment_receipt.receipt_digest,
        experiment_receipt.runtime_behavior_digest
    );
    assert_eq!(experiment_receipt.agent_flow_intent_count.get(), 2);
    assert_eq!(experiment_receipt.memory_projection_count.get(), 1);
    assert_eq!(experiment_receipt.tool_projection_count.get(), 1);
    assert!(
        !experiment_receipt
            .improvement_recommendations
            .iter()
            .any(|recommendation| recommendation.target.as_str()
                == "runtime.agent-flow.memory-projection"
                || recommendation.target.as_str() == "runtime.tool-sandbox.spawn")
    );
    assert!(
        experiment_receipt
            .improvement_recommendations
            .iter()
            .any(|recommendation| recommendation.target.as_str()
                == "gerbil.config-interface.policy-pack"
                && recommendation.priority == RuntimePolicyRecommendationPriority::P2)
    );
}

#[test]
fn memory_rewrite_checker_combination_pumps_runtime_receipts_without_scripted_mapper() {
    let (_loop_program, receipt) = run_receipt_driven_memory_rewrite_checker_loop();

    assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(
        receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::ReadMemory,
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::RewriteGraph,
            LoopProgramActionKind::DispatchTools,
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
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::ModelEvent),
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::ToolReceipt),
            Some(LoopProgramEventKind::VerificationReceipt),
            None,
        ]
    );
    assert_eq!(
        receipt
            .steps
            .iter()
            .map(|step| {
                step.runtime_handoff_execution.executions[0]
                    .next_event
                    .clone()
            })
            .collect::<Vec<_>>(),
        vec![
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::ModelEvent),
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::ToolReceipt),
            Some(LoopProgramEventKind::VerificationReceipt),
            None,
        ]
    );
    assert_eq!(
        receipt.steps[0]
            .runtime_handoff_execution
            .memory_projections
            .len(),
        1
    );
    assert_eq!(
        receipt.steps[3]
            .runtime_handoff_execution
            .tool_process_projections
            .len(),
        1
    );
}

#[cfg(unix)]
#[tokio::test]
async fn memory_rewrite_checker_combination_spawns_projected_tool_process() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let (_loop_program, receipt) = run_memory_rewrite_checker_loop();
    let tool_step = &receipt.steps[3];

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

    let replay_bundle = LoopProgramRuntimeSideEffectExecutor::new(dispatch_tools_shell_resolver(
        "printf policy-combination-tool-spawn",
    ))
    .with_started_at_ms(300)
    .with_observed_at_ms(325)
    .execute_loop_execution(&runtime.context(), &receipt)
    .await;

    assert_eq!(
        replay_bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Ready
    );
    assert!(replay_bundle.allows_replay());
    assert_eq!(replay_bundle.step_replay_bundles.len(), 2);
    let memory_bundle = &replay_bundle.step_replay_bundles[0];
    assert_eq!(
        memory_bundle.side_effects.status,
        LoopProgramRuntimeSideEffectStatus::Empty
    );
    assert_eq!(memory_bundle.handoff_execution.memory_projections.len(), 1);
    let tool_bundle = &replay_bundle.step_replay_bundles[1];
    assert_eq!(
        tool_bundle.side_effects.status,
        LoopProgramRuntimeSideEffectStatus::Completed
    );
    let tool_process = &tool_bundle.side_effects.tool_processes[0];
    assert_eq!(
        tool_process.status,
        LoopProgramToolProcessSideEffectStatus::Completed
    );
    let spawn_receipt = tool_process
        .spawn_receipt
        .as_ref()
        .expect("policy combination tool projection should spawn");
    assert!(spawn_receipt.output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&spawn_receipt.output.stdout),
        "policy-combination-tool-spawn"
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
