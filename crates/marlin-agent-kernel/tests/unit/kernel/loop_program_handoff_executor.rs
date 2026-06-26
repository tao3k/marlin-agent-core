use std::sync::Arc;

use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, GenericLoopMachineReceipt,
    GenericLoopMachineStepIndex, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffExecutionStatus, LoopProgramRuntimeHandoffExecutor,
    LoopProgramRuntimeHandoffHandler, LoopProgramRuntimeHandoffPlan,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, LoopProgramToolProcessProgram, LoopProgramToolProcessSpawnRequest,
    StaticLoopProgramRuntimeHandoffHandler, spawn_loop_program_tool_process,
};
use marlin_agent_protocol::{
    AgentFlowMemoryOperation, LoopProgramActionKind, LoopProgramEventKind, LoopProgramId,
    LoopProgramStateId, LoopProgramTransitionId,
};
use marlin_agent_runtime::TokioAgentRuntime;

#[test]
fn default_handoff_router_defers_all_runtime_lanes_with_typed_receipts() {
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("deferred-program"),
        &[
            receipt(1, LoopProgramActionKind::InvokeModel),
            receipt(2, LoopProgramActionKind::DispatchTools),
            receipt(3, LoopProgramActionKind::WriteMemory),
        ],
    );

    let execution = LoopProgramRuntimeHandoffRouter::default().execute_plan(&plan);

    assert_eq!(execution.program_id, LoopProgramId::new("deferred-program"));
    assert_eq!(
        execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Deferred
    );
    assert_eq!(execution.executions.len(), 3);
    assert!(execution.executions.iter().all(|handoff| {
        handoff.status == LoopProgramRuntimeHandoffExecutionStatus::Deferred
            && handoff.owner.as_str() == "kernel.loop-program.deferred"
    }));
    assert!(execution.executions[1].agent_flow_intent.is_some());
    assert!(execution.executions[2].agent_flow_intent.is_some());
}

#[test]
fn handoff_router_dispatches_each_lane_to_its_dedicated_handler() {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        control_handler: handled_by("control"),
        model_handler: handled_by("model"),
        tool_handler: handled_by("tool"),
        memory_handler: handled_by("memory"),
        placement_handler: handled_by("placement"),
        runtime_handler: handled_by("runtime"),
        graph_handler: handled_by("graph"),
        session_handler: handled_by("session"),
        agent_handler: handled_by("agent"),
        verification_handler: handled_by("verification"),
        human_gate_handler: handled_by("human-gate"),
        receipt_handler: handled_by("receipt"),
    };
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("routed-program"),
        &[
            receipt(1, LoopProgramActionKind::Continue),
            receipt(2, LoopProgramActionKind::InvokeModel),
            receipt(3, LoopProgramActionKind::DispatchTools),
            receipt(4, LoopProgramActionKind::ReadMemory),
            receipt(5, LoopProgramActionKind::RequestPlacement),
            receipt(6, LoopProgramActionKind::RuntimeHandoff),
            receipt(7, LoopProgramActionKind::RewriteGraph),
            receipt(8, LoopProgramActionKind::ForkSession),
            receipt(9, LoopProgramActionKind::DelegateAgent),
            receipt(10, LoopProgramActionKind::Verify),
            receipt(11, LoopProgramActionKind::HumanGate),
            receipt(12, LoopProgramActionKind::EmitReceipt),
            receipt(13, LoopProgramActionKind::Stop),
        ],
    );

    let execution = LoopProgramRuntimeHandoffRouter::new(handlers).execute_plan(&plan);

    assert_eq!(
        execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    );
    assert_eq!(
        execution
            .executions
            .iter()
            .map(|execution| execution.owner.as_str())
            .collect::<Vec<_>>(),
        vec![
            "control",
            "model",
            "tool",
            "memory",
            "placement",
            "runtime",
            "graph",
            "session",
            "agent",
            "verification",
            "human-gate",
            "receipt",
            "control",
        ]
    );
}

#[test]
fn handoff_router_reports_denied_when_any_adapter_denies() {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        tool_handler: denied_by("tool-policy"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("denied-program"),
        &[
            receipt(1, LoopProgramActionKind::InvokeModel),
            receipt(2, LoopProgramActionKind::DispatchTools),
        ],
    );

    let execution = LoopProgramRuntimeHandoffRouter::new(handlers).execute_plan(&plan);

    assert_eq!(
        execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Denied
    );
    assert_eq!(
        execution.executions[1].status,
        LoopProgramRuntimeHandoffExecutionStatus::Denied
    );
    assert_eq!(execution.executions[1].owner.as_str(), "tool-policy");
}

#[test]
fn agent_flow_executor_projects_intents_through_runtime_receipt() {
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("agent-flow-program"),
        &[
            receipt(1, LoopProgramActionKind::DispatchTools),
            receipt(2, LoopProgramActionKind::ReadMemory),
            receipt(3, LoopProgramActionKind::RequestPlacement),
        ],
    );

    let execution = AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
        "agent-flow-runtime",
    ))
    .with_admitted_at_ms(512)
    .execute_plan(&plan);

    assert_eq!(
        execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    );
    assert!(execution.executions.iter().all(|handoff| {
        handoff.status == LoopProgramRuntimeHandoffExecutionStatus::Handled
            && handoff.owner.as_str() == "agent-flow-runtime"
    }));

    let agent_flow_receipt = execution
        .agent_flow_receipt
        .as_ref()
        .expect("Agent-Flow receipt");
    assert_eq!(
        agent_flow_receipt.handoff.handoff_id.as_str(),
        "loop-program:agent-flow-program:agent-flow-handoff"
    );
    assert_eq!(agent_flow_receipt.handoff.admitted_at_ms, 512);
    assert_eq!(agent_flow_receipt.handoff.intents.len(), 3);
    assert_eq!(
        agent_flow_receipt
            .derived_session
            .session
            .session_id
            .as_str(),
        "loop-program:agent-flow-program:agent-flow-session"
    );
    assert_eq!(agent_flow_receipt.derived_session.session.generation, 1);
    assert_eq!(execution.tool_process_projections.len(), 1);
    let tool_projection = &execution.tool_process_projections[0];
    assert_eq!(tool_projection.owner.as_str(), "agent-flow-runtime");
    assert_eq!(
        tool_projection.command.command_kind.as_str(),
        "agent-flow.tool-intent"
    );
    assert_eq!(
        tool_projection.command.argv,
        vec!["loop-program.dispatch-tools"]
    );
    assert_eq!(execution.memory_projections.len(), 1);
    let memory_projection = &execution.memory_projections[0];
    assert_eq!(memory_projection.owner.as_str(), "agent-flow-runtime");
    assert_eq!(
        memory_projection.intent.target.as_str(),
        "loop-program.memory"
    );
    assert_eq!(
        memory_projection.intent.operation,
        AgentFlowMemoryOperation::Recall
    );
}

#[test]
fn agent_flow_executor_projects_each_memory_operation_as_typed_receipt() {
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("memory-agent-flow-program"),
        &[
            receipt(1, LoopProgramActionKind::ReadMemory),
            receipt(2, LoopProgramActionKind::WriteMemory),
            receipt(3, LoopProgramActionKind::CompactContext),
        ],
    );

    let execution = AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
        "agent-flow-runtime",
    ))
    .execute_plan(&plan);

    assert_eq!(
        execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    );
    assert!(execution.tool_process_projections.is_empty());
    assert_eq!(
        execution
            .memory_projections
            .iter()
            .map(|projection| &projection.intent.operation)
            .collect::<Vec<_>>(),
        vec![
            &AgentFlowMemoryOperation::Recall,
            &AgentFlowMemoryOperation::Store,
            &AgentFlowMemoryOperation::Compact,
        ]
    );
    assert!(execution.memory_projections.iter().all(|projection| {
        projection.owner.as_str() == "agent-flow-runtime"
            && projection.intent.target.as_str() == "loop-program.memory"
    }));
}

#[test]
fn agent_flow_executor_defers_non_agent_flow_lanes_after_runtime_projection() {
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("mixed-agent-flow-program"),
        &[
            receipt(1, LoopProgramActionKind::InvokeModel),
            receipt(2, LoopProgramActionKind::DispatchTools),
        ],
    );

    let execution = AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
        "agent-flow-runtime",
    ))
    .execute_plan(&plan);

    assert_eq!(
        execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Deferred
    );
    assert_eq!(
        execution.executions[0].status,
        LoopProgramRuntimeHandoffExecutionStatus::Deferred
    );
    assert_eq!(
        execution.executions[1].status,
        LoopProgramRuntimeHandoffExecutionStatus::Handled
    );
    assert_eq!(
        execution
            .agent_flow_receipt
            .as_ref()
            .expect("Agent-Flow receipt")
            .handoff
            .intents
            .len(),
        1
    );
    assert_eq!(execution.tool_process_projections.len(), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn tool_process_projection_can_spawn_fixture_command() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("spawn-agent-flow-program"),
        &[receipt(1, LoopProgramActionKind::DispatchTools)],
    );
    let execution = AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
        "agent-flow-runtime",
    ))
    .execute_plan(&plan);
    let projection = execution.tool_process_projections[0].clone();

    let receipt = spawn_loop_program_tool_process(
        &runtime.context(),
        LoopProgramToolProcessSpawnRequest::new(
            projection,
            LoopProgramToolProcessProgram::new("sh"),
        )
        .with_args(vec!["-c".to_owned(), "printf loop-tool-spawn".to_owned()].into_boxed_slice())
        .with_started_at_ms(10)
        .with_observed_at_ms(20),
    )
    .await
    .expect("fixture command should spawn");

    assert!(receipt.output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&receipt.output.stdout),
        "loop-tool-spawn"
    );
    assert!(receipt.output.stderr.is_empty());
    assert!(
        runtime
            .context()
            .process_registry()
            .get(receipt.pid)
            .is_none()
    );
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
