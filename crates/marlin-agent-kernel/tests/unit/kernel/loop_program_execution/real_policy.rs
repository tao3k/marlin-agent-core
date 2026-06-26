use std::sync::Arc;

use super::{
    AgentFlowIntent, AgentFlowLoopProgramRuntimeHandoffExecutor, AgentFlowMemoryOperation,
    DenylistedLoopProgramToolDispatchHandler, LoopProgramActionKind, LoopProgramEventKind,
    LoopProgramExecutionDriver, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffExecutionStatus,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, LoopProgramToolProcessProgram, LoopProgramToolProcessSpawnRequest,
    MemoryRecallDecisionMapper, PolicyCombinationDecisionMapper, RetryBudgetToolHandler,
    ScriptedLoopProgramEventMapper, TokioAgentRuntime, handled_by,
    policy_combination_matrix_loop_program, real_policy_002_retry_budget_loop_program,
    real_policy_003_maker_checker_loop_program, real_policy_004_dynamic_rewrite_loop_program,
    real_policy_005_memory_recall_loop_program, real_tool_sandbox_loop_program,
    spawn_loop_program_tool_process, tool_error_loop_program,
};

#[test]
fn real_policy_001_sandbox_denylist_blocks_dispatch_tool_intent() {
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

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        tool_error_loop_program(),
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
}

#[cfg(unix)]
#[tokio::test]
async fn real_tool_sandbox_loop_projects_and_spawns_allowed_tool_process() {
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
        real_tool_sandbox_loop_program(),
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

    let spawn_receipt = spawn_loop_program_tool_process(
        &runtime.context(),
        LoopProgramToolProcessSpawnRequest::new(
            tool_step.runtime_handoff_execution.tool_process_projections[0].clone(),
            LoopProgramToolProcessProgram::new("sh"),
        )
        .with_args(
            vec!["-c".to_owned(), "printf real-tool-sandbox-loop".to_owned()].into_boxed_slice(),
        )
        .with_started_at_ms(100)
        .with_observed_at_ms(125),
    )
    .await
    .expect("allowed tool projection should spawn");

    assert!(spawn_receipt.output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&spawn_receipt.output.stdout),
        "real-tool-sandbox-loop"
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
fn real_policy_002_retry_budget_replays_after_first_denied_dispatch() {
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
        real_policy_002_retry_budget_loop_program(),
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

#[test]
fn real_policy_003_maker_checker_routes_model_and_verification_lanes_apart() {
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
        real_policy_003_maker_checker_loop_program(),
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
fn real_policy_004_dynamic_rewrite_runs_before_repair_and_verification() {
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
        real_policy_004_dynamic_rewrite_loop_program(),
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
fn real_policy_005_memory_recall_receipt_changes_next_decision() {
    let driver = LoopProgramExecutionDriver::new(AgentFlowLoopProgramRuntimeHandoffExecutor::new(
        LoopProgramRuntimeOwner::new("runtime.agent-flow.memory-policy"),
    ))
    .with_event_mapper(MemoryRecallDecisionMapper);

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        real_policy_005_memory_recall_loop_program(),
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

#[test]
fn policy_combination_matrix_runs_memory_rewrite_checker_path() {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        memory_handler: handled_by("runtime.memory.recall"),
        model_handler: handled_by("runtime.model.maker"),
        graph_handler: handled_by("runtime.graph.dynamic-rewrite"),
        tool_handler: handled_by("runtime.tool.repair"),
        verification_handler: handled_by("runtime.verification.checker"),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(PolicyCombinationDecisionMapper);

    let receipt = driver.run(LoopProgramExecutionRequest::new(
        policy_combination_matrix_loop_program(),
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
            "runtime.memory.recall",
            "runtime.model.maker",
            "runtime.graph.dynamic-rewrite",
            "runtime.tool.repair",
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
}
