use std::sync::Arc;

use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, GraphLoopController, GraphLoopExecutionBudget,
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphLoopNextAction, GraphLoopRunRequest,
    GraphLoopStopPolicy, LoopGraph, LoopProgramRunRequest, LoopProgramRunStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffExecutionStatus,
    LoopProgramRuntimeHandoffHandler, LoopProgramRuntimeHandoffKind,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, StaticLoopProgramRuntimeHandoffHandler,
};
use marlin_agent_protocol::{
    AgentFlowIntent, LoopMechanismPolicyId, LoopPolicyDigest, LoopPolicyEpoch, LoopProgram,
    LoopProgramActionKind, LoopProgramEventKind, LoopProgramId, LoopProgramInput,
    LoopProgramStateId, LoopProgramTransition, LoopProgramTransitionId,
};
use marlin_agent_runtime::{GraphLoopRunStatus, TokioAgentRuntime};

use super::{controller, edge, node};

#[tokio::test]
async fn controller_runs_initial_graph_and_reports_terminal_action() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(1));
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.iteration, 0);
    assert_eq!(
        report.execution_result.status,
        GraphLoopExecutionStatus::Completed
    );
    assert_eq!(report.execution_result.visited_nodes, vec!["plan"]);
    assert!(report.execution_result.node_receipts.is_empty());
    assert!(report.trace.is_none());
    assert_eq!(report.next_action, GraphLoopNextAction::StopCompleted);
}

#[tokio::test]
async fn controller_runs_loop_program_through_generic_loop_machine() {
    let (runtime, _events) = TokioAgentRuntime::new(8);
    let request = LoopProgramRunRequest::new(
        sample_loop_program(),
        vec![
            LoopProgramEventKind::Start,
            LoopProgramEventKind::ToolRequest,
            LoopProgramEventKind::ToolReceipt,
            LoopProgramEventKind::ModelEvent,
            LoopProgramEventKind::RuntimeReceipt,
            LoopProgramEventKind::VerificationReceipt,
        ],
    );

    let receipt = controller()
        .spawn_loop_program(request, &runtime)
        .join()
        .await
        .expect("loop program task should join");

    assert_eq!(
        receipt.program_id,
        LoopProgramId::new("controller-loop-program")
    );
    assert_eq!(receipt.status, LoopProgramRunStatus::Stopped);
    assert_eq!(receipt.action_receipts.len(), 6);
    assert_eq!(receipt.last_action, Some(LoopProgramActionKind::Stop));
    assert!(receipt.error.is_none());
    assert_eq!(
        receipt.action_receipts[0].action,
        LoopProgramActionKind::InvokeModel
    );
    assert_eq!(
        receipt.action_receipts[1].action,
        LoopProgramActionKind::DispatchTools
    );
    assert_eq!(
        receipt.action_receipts[3].action,
        LoopProgramActionKind::RewriteGraph
    );
    assert!(receipt.action_receipts[5].stopped);

    assert_eq!(receipt.runtime_handoff_plan.handoffs.len(), 6);
    assert_eq!(receipt.runtime_handoff_execution.executions.len(), 6);
    assert_eq!(
        receipt.runtime_handoff_execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Deferred
    );
    assert_eq!(
        receipt.runtime_handoff_execution.executions[0].status,
        LoopProgramRuntimeHandoffExecutionStatus::Deferred
    );
    assert_eq!(
        receipt.runtime_handoff_execution.executions[0]
            .owner
            .as_str(),
        "kernel.loop-program.deferred"
    );
    assert_eq!(
        receipt.runtime_handoff_plan.handoffs[0].kind,
        LoopProgramRuntimeHandoffKind::ModelInvocation
    );
    assert_eq!(
        receipt.runtime_handoff_plan.handoffs[1].kind,
        LoopProgramRuntimeHandoffKind::ToolDispatch
    );
    assert_eq!(
        receipt.runtime_handoff_plan.handoffs[3].kind,
        LoopProgramRuntimeHandoffKind::GraphRewrite
    );
    assert_eq!(
        receipt.runtime_handoff_plan.handoffs[5].kind,
        LoopProgramRuntimeHandoffKind::Stop
    );

    let agent_flow_intents = receipt.runtime_handoff_plan.agent_flow_intents();
    assert_eq!(agent_flow_intents.len(), 1);
    let AgentFlowIntent::Tool(tool_intent) = &agent_flow_intents[0] else {
        panic!("dispatch-tools should produce a tool intent");
    };
    assert_eq!(
        tool_intent.tool_name.as_str(),
        "loop-program.dispatch-tools"
    );
}

#[tokio::test]
async fn controller_uses_injected_runtime_handoff_executor() {
    let (runtime, _events) = TokioAgentRuntime::new(8);
    let request = LoopProgramRunRequest::new(
        sample_loop_program(),
        vec![
            LoopProgramEventKind::Start,
            LoopProgramEventKind::ToolRequest,
            LoopProgramEventKind::ToolReceipt,
            LoopProgramEventKind::ModelEvent,
            LoopProgramEventKind::RuntimeReceipt,
            LoopProgramEventKind::VerificationReceipt,
        ],
    );
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        control_handler: handled_by("test-control-runtime"),
        model_handler: handled_by("test-model-runtime"),
        tool_handler: handled_by("test-tool-runtime"),
        graph_handler: handled_by("test-graph-runtime"),
        verification_handler: handled_by("test-verification-runtime"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };

    let receipt = controller()
        .with_loop_program_handoff_executor(LoopProgramRuntimeHandoffRouter::new(handlers))
        .spawn_loop_program(request, &runtime)
        .join()
        .await
        .expect("loop program task should join");

    assert_eq!(
        receipt.runtime_handoff_execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    );
    assert_eq!(
        receipt
            .runtime_handoff_execution
            .executions
            .iter()
            .map(|execution| execution.owner.as_str())
            .collect::<Vec<_>>(),
        vec![
            "test-model-runtime",
            "test-tool-runtime",
            "test-control-runtime",
            "test-graph-runtime",
            "test-verification-runtime",
            "test-control-runtime",
        ]
    );
}

#[tokio::test]
async fn controller_can_project_agent_flow_runtime_receipt_from_loop_program() {
    let (runtime, _events) = TokioAgentRuntime::new(8);
    let request = LoopProgramRunRequest::new(
        sample_loop_program(),
        vec![
            LoopProgramEventKind::Start,
            LoopProgramEventKind::ToolRequest,
            LoopProgramEventKind::ToolReceipt,
            LoopProgramEventKind::ModelEvent,
            LoopProgramEventKind::RuntimeReceipt,
            LoopProgramEventKind::VerificationReceipt,
        ],
    );

    let receipt = controller()
        .with_loop_program_handoff_executor(
            AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
                "agent-flow-runtime",
            ))
            .with_admitted_at_ms(2048),
        )
        .spawn_loop_program(request, &runtime)
        .join()
        .await
        .expect("loop program task should join");

    assert_eq!(
        receipt.runtime_handoff_execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Deferred
    );
    assert_eq!(
        receipt.runtime_handoff_execution.executions[1].status,
        LoopProgramRuntimeHandoffExecutionStatus::Handled
    );
    assert_eq!(
        receipt.runtime_handoff_execution.executions[1]
            .owner
            .as_str(),
        "agent-flow-runtime"
    );

    let agent_flow_receipt = receipt
        .runtime_handoff_execution
        .agent_flow_receipt
        .as_ref()
        .expect("Agent-Flow runtime receipt");
    assert_eq!(agent_flow_receipt.handoff.intents.len(), 1);
    assert_eq!(agent_flow_receipt.handoff.admitted_at_ms, 2048);
    assert_eq!(
        agent_flow_receipt.handoff.handoff_id.as_str(),
        "loop-program:controller-loop-program:agent-flow-handoff"
    );
    assert_eq!(
        receipt
            .runtime_handoff_execution
            .tool_process_projections
            .len(),
        1
    );
    assert_eq!(
        receipt.runtime_handoff_execution.tool_process_projections[0]
            .command
            .command_kind
            .as_str(),
        "agent-flow.tool-intent"
    );
}

#[tokio::test]
async fn controller_rejects_invalid_loop_program_event_sequence() {
    let (runtime, _events) = TokioAgentRuntime::new(8);
    let request = LoopProgramRunRequest::new(
        sample_loop_program(),
        vec![LoopProgramEventKind::ToolReceipt],
    );

    let receipt = controller()
        .spawn_loop_program(request, &runtime)
        .join()
        .await
        .expect("loop program task should join");

    assert_eq!(
        receipt.program_id,
        LoopProgramId::new("controller-loop-program")
    );
    assert_eq!(receipt.status, LoopProgramRunStatus::Rejected);
    assert!(receipt.action_receipts.is_empty());
    assert!(receipt.runtime_handoff_plan.handoffs.is_empty());
    assert!(receipt.runtime_handoff_execution.executions.is_empty());
    assert_eq!(
        receipt.runtime_handoff_execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Empty
    );
    assert!(receipt.last_action.is_none());
    assert!(receipt.error.is_some());
}

fn handled_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
        LoopProgramRuntimeOwner::new(owner),
    ))
}

#[tokio::test]
async fn controller_records_loop_run_lifecycle_in_runtime_registry() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(1));
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    let snapshot = runtime
        .graph_loop_runs()
        .read_registry(|registry| registry.snapshot(1));

    assert_eq!(snapshot.run_count, 1);
    assert_eq!(snapshot.active_count, 0);
    let observation = snapshot.runs.first().expect("run observation");
    assert_eq!(observation.run_id.as_str(), "run");
    assert_eq!(observation.graph_id.as_str(), "graph");
    assert_eq!(observation.status, GraphLoopRunStatus::Completed);
    assert_eq!(
        observation.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert_eq!(
        observation
            .current_iteration_id
            .expect("current iteration id")
            .get(),
        0
    );
}

fn sample_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("controller-loop-program"),
        policy_epoch: LoopPolicyEpoch::new(7),
        policy_digest: LoopPolicyDigest::from_bytes([5_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("reactive-tool-loop-base"),
            LoopMechanismPolicyId::new("claude-style-dynamic-graph-rewrite"),
        ]
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

#[tokio::test]
async fn controller_honors_zero_iteration_stop_policy_without_execution() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan")],
            edges: Vec::new(),
        },
    ))
    .with_stop_policy(GraphLoopStopPolicy::max_iterations(0));
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert!(reports.is_empty());
}

#[tokio::test]
async fn controller_applies_iteration_budget_to_kernel_execution() {
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new(
        "run",
        LoopGraph {
            graph_id: "graph".to_owned(),
            nodes: vec![node("plan"), node("apply")],
            edges: vec![edge("plan", "apply")],
        },
    ))
    .with_iteration_budget(GraphLoopExecutionBudget::max_node_executions(1));
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let reports = controller()
        .spawn_loop(request, &runtime)
        .join()
        .await
        .expect("controller task should join");

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(
        report.execution_result.status,
        GraphLoopExecutionStatus::Failed
    );
    assert_eq!(report.next_action, GraphLoopNextAction::StopFailed);
    assert_eq!(
        report.execution_result.diagnostics,
        vec!["graph execution budget exceeded: planned node executions 2 > max 1"]
    );
}
