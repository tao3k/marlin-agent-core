use std::{
    path::PathBuf,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, DenylistedLoopProgramToolDispatchHandler,
    GenericLoopMachineReceipt, HybridLoopProgramRuntimeHandoffExecutor, LoopProgramEventMapper,
    LoopProgramExecutionReceipt, LoopProgramRuntimeHandoffExecutionReceipt,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffHandler,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, LoopProgramToolProcessCommandTemplate, LoopProgramToolProcessProgram,
    PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor, RetryBudgetToolHandler,
    StaticLoopProgramRuntimeHandoffHandler, StaticLoopProgramToolProcessResolver,
};
use marlin_agent_protocol::{
    AgentFlowMemoryOperation, LoopProgram, LoopProgramActionKind, LoopProgramEventKind,
};
use marlin_gerbil_scheme::{
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverVerticalTraceReceipt,
    project_gerbil_loop_case_driver_loop_action_kind,
    project_gerbil_loop_case_driver_loop_event_kind, project_gerbil_loop_case_driver_loop_program,
    run_gerbil_config_interface_case_driver_smoke,
};

pub(super) fn cap(tag: impl Into<String>) -> GerbilLoopCaseDriverCapability {
    GerbilLoopCaseDriverCapability::new(tag)
}

pub(super) fn run_config_interface_case_driver_smoke() -> String {
    static STDOUT: OnceLock<String> = OnceLock::new();

    STDOUT
        .get_or_init(|| {
            run_gerbil_config_interface_case_driver_smoke()
                .expect("gxi case-driver smoke should produce verified stdout")
        })
        .clone()
}

pub(super) fn unique_scheme_projected_runtime_workspace() -> PathBuf {
    static WORKSPACE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    std::env::temp_dir().join(format!(
        "marlin-scheme-projected-runtime-{}-{}",
        std::process::id(),
        WORKSPACE_COUNTER.fetch_add(1, Ordering::SeqCst)
    ))
}

pub(super) fn scheme_projected_dispatch_tools_shell_resolver(
    script: &'static str,
) -> StaticLoopProgramToolProcessResolver {
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

pub(super) fn scheme_projected_loop_program(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> LoopProgram {
    project_gerbil_loop_case_driver_loop_program(receipt)
        .expect("Scheme vertical receipt should project into a LoopProgram")
}

pub(super) fn scheme_projected_event_mapper(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> SchemeProjectedLoopProgramEventMapper {
    SchemeProjectedLoopProgramEventMapper::new(
        receipt
            .transition_events()
            .map(|event| {
                project_gerbil_loop_case_driver_loop_event_kind(event)
                    .expect("Scheme vertical event should project")
            })
            .collect::<Vec<_>>(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SchemeProjectedLoopProgramEventMapper {
    events: Box<[LoopProgramEventKind]>,
}

impl SchemeProjectedLoopProgramEventMapper {
    fn new(events: impl Into<Box<[LoopProgramEventKind]>>) -> Self {
        Self {
            events: events.into(),
        }
    }
}

impl LoopProgramEventMapper for SchemeProjectedLoopProgramEventMapper {
    fn next_event(
        &self,
        machine_receipt: &GenericLoopMachineReceipt,
        runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> Option<LoopProgramEventKind> {
        if runtime_handoff_execution.status
            == LoopProgramRuntimeHandoffExecutionReportStatus::Denied
        {
            return Some(LoopProgramEventKind::Error);
        }
        if runtime_handoff_execution.status
            != LoopProgramRuntimeHandoffExecutionReportStatus::Completed
        {
            return None;
        }

        self.events
            .get(machine_receipt.step_index.get() as usize)
            .cloned()
    }
}

#[derive(Clone, Debug)]
pub(super) struct SchemeProjectedMemoryRecallDecisionMapper;

impl LoopProgramEventMapper for SchemeProjectedMemoryRecallDecisionMapper {
    fn next_event(
        &self,
        machine_receipt: &GenericLoopMachineReceipt,
        runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> Option<LoopProgramEventKind> {
        if runtime_handoff_execution.status
            != LoopProgramRuntimeHandoffExecutionReportStatus::Completed
        {
            return None;
        }

        match machine_receipt.action {
            LoopProgramActionKind::ReadMemory
                if runtime_handoff_execution
                    .memory_projections
                    .iter()
                    .any(|projection| {
                        projection.intent.operation == AgentFlowMemoryOperation::Recall
                    }) =>
            {
                Some(LoopProgramEventKind::ToolRequest)
            }
            LoopProgramActionKind::DispatchTools => Some(LoopProgramEventKind::ToolReceipt),
            _ => None,
        }
    }
}

pub(super) fn scheme_projected_runtime_executor() -> HybridLoopProgramRuntimeHandoffExecutor {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        model_handler: handled_by("scheme.projected.model"),
        graph_handler: handled_by("scheme.projected.graph"),
        verification_handler: handled_by("scheme.projected.verification"),
        control_handler: handled_by("scheme.projected.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };

    HybridLoopProgramRuntimeHandoffExecutor::new(
        LoopProgramRuntimeHandoffRouter::new(handlers),
        AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
            "scheme.projected.agent-flow",
        )),
    )
}

pub(super) fn scheme_projected_tool_denial_runtime_executor() -> LoopProgramRuntimeHandoffRouter {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        tool_handler: Arc::new(DenylistedLoopProgramToolDispatchHandler::new(
            LoopProgramRuntimeOwner::new("scheme.projected.sandbox-denial"),
            ["loop-program.dispatch-tools"],
        )),
        model_handler: handled_by("scheme.projected.model"),
        graph_handler: handled_by("scheme.projected.graph"),
        verification_handler: handled_by("scheme.projected.verification"),
        control_handler: handled_by("scheme.projected.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };

    LoopProgramRuntimeHandoffRouter::new(handlers)
}

pub(super) fn scheme_projected_retry_budget_runtime_executor()
-> PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        tool_handler: Arc::new(RetryBudgetToolHandler::new(
            LoopProgramRuntimeOwner::new("scheme.projected.retry-budget-tool"),
            1,
        )),
        control_handler: handled_by("scheme.projected.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };

    PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor::new(
        LoopProgramRuntimeHandoffRouter::new(handlers),
        AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
            "scheme.projected.agent-flow.retry-budget-tool",
        )),
    )
}

pub(super) fn loop_program_action_kind(action: &str) -> LoopProgramActionKind {
    project_gerbil_loop_case_driver_loop_action_kind(action)
        .expect("Scheme vertical action should project")
}

pub(super) fn tool_projection_count(receipt: &LoopProgramExecutionReceipt) -> usize {
    receipt
        .steps
        .iter()
        .map(|step| {
            step.runtime_handoff_execution
                .tool_process_projections
                .len()
        })
        .sum()
}

fn handled_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
        LoopProgramRuntimeOwner::new(owner),
    ))
}
