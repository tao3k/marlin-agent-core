//! Controller handoff surface for provider-neutral `LoopProgram` runs.

use marlin_agent_protocol::{
    LoopProgram, LoopProgramActionKind, LoopProgramEventKind, LoopProgramId,
};
use marlin_agent_runtime::{RuntimeContext, RuntimeTask, TokioAgentRuntime};

use crate::{
    GenericLoopMachine, GenericLoopMachineError, GenericLoopMachineReceipt,
    LoopProgramRuntimeHandoffExecutionReceipt, LoopProgramRuntimeHandoffExecutor,
    LoopProgramRuntimeHandoffPlan, LoopProgramRuntimeReplayBundleReceipt,
    LoopProgramRuntimeSideEffectExecutor, TokioGraphLoopController,
};

/// Explicit controller request for running a compiled `LoopProgram`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramRunRequest {
    pub program: LoopProgram,
    pub events: Box<[LoopProgramEventKind]>,
}

impl LoopProgramRunRequest {
    pub fn new(program: LoopProgram, events: impl Into<Box<[LoopProgramEventKind]>>) -> Self {
        Self {
            program,
            events: events.into(),
        }
    }
}

/// Controller-level receipt for a provider-neutral `LoopProgram` run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramRunReceipt {
    pub program_id: LoopProgramId,
    pub status: LoopProgramRunStatus,
    pub action_receipts: Box<[GenericLoopMachineReceipt]>,
    pub runtime_handoff_plan: LoopProgramRuntimeHandoffPlan,
    pub runtime_handoff_execution: LoopProgramRuntimeHandoffExecutionReceipt,
    pub runtime_replay_bundle: LoopProgramRuntimeReplayBundleReceipt,
    pub last_action: Option<LoopProgramActionKind>,
    pub error: Option<GenericLoopMachineError>,
}

/// Terminal status for a controller-owned `LoopProgram` run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramRunStatus {
    Completed,
    Stopped,
    Rejected,
}

impl TokioGraphLoopController {
    pub fn spawn_loop_program(
        &self,
        request: LoopProgramRunRequest,
        runtime: &TokioAgentRuntime,
    ) -> RuntimeTask<LoopProgramRunReceipt> {
        let controller_runtime = runtime.child_runtime();
        let handoff_executor = self.loop_program_handoff_executor();
        let side_effect_executor = self.loop_program_side_effect_executor();
        let runtime_context = runtime.context().clone();
        controller_runtime.spawn(async move {
            run_loop_program(
                request,
                handoff_executor,
                side_effect_executor,
                runtime_context,
            )
            .await
        })
    }
}

async fn run_loop_program(
    request: LoopProgramRunRequest,
    handoff_executor: std::sync::Arc<dyn LoopProgramRuntimeHandoffExecutor>,
    side_effect_executor: LoopProgramRuntimeSideEffectExecutor,
    runtime_context: RuntimeContext,
) -> LoopProgramRunReceipt {
    let program_id = request.program.program_id.clone();
    let mut machine = GenericLoopMachine::new(request.program);
    let mut action_receipts = Vec::new();
    let mut last_action = None;

    for event in request.events {
        match machine.apply_event(event) {
            Ok(step) => {
                last_action = Some(step.action);
                action_receipts.push(step.receipt);
                if machine.is_stopped() {
                    return build_run_receipt(
                        program_id,
                        LoopProgramRunStatus::Stopped,
                        action_receipts,
                        last_action,
                        None,
                        handoff_executor.as_ref(),
                        &side_effect_executor,
                        &runtime_context,
                    )
                    .await;
                }
            }
            Err(error) => {
                return build_run_receipt(
                    program_id,
                    LoopProgramRunStatus::Rejected,
                    action_receipts,
                    last_action,
                    Some(error),
                    handoff_executor.as_ref(),
                    &side_effect_executor,
                    &runtime_context,
                )
                .await;
            }
        }
    }

    build_run_receipt(
        program_id,
        LoopProgramRunStatus::Completed,
        action_receipts,
        last_action,
        None,
        handoff_executor.as_ref(),
        &side_effect_executor,
        &runtime_context,
    )
    .await
}

async fn build_run_receipt(
    program_id: LoopProgramId,
    status: LoopProgramRunStatus,
    action_receipts: Vec<GenericLoopMachineReceipt>,
    last_action: Option<LoopProgramActionKind>,
    error: Option<GenericLoopMachineError>,
    handoff_executor: &dyn LoopProgramRuntimeHandoffExecutor,
    side_effect_executor: &LoopProgramRuntimeSideEffectExecutor,
    runtime_context: &RuntimeContext,
) -> LoopProgramRunReceipt {
    let runtime_handoff_plan =
        LoopProgramRuntimeHandoffPlan::from_receipts(program_id.clone(), &action_receipts);
    let runtime_handoff_execution = handoff_executor.execute_plan(&runtime_handoff_plan);
    let side_effects = side_effect_executor
        .execute(runtime_context, &runtime_handoff_execution)
        .await;
    let runtime_replay_bundle = LoopProgramRuntimeReplayBundleReceipt::from_runtime_receipts(
        runtime_handoff_execution.clone(),
        side_effects,
    );
    LoopProgramRunReceipt {
        program_id,
        status,
        action_receipts: action_receipts.into_boxed_slice(),
        runtime_handoff_plan,
        runtime_handoff_execution,
        runtime_replay_bundle,
        last_action,
        error,
    }
}
