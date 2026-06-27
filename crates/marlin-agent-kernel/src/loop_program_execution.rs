//! Provider-neutral event pump for `LoopProgram` execution.

use std::{collections::VecDeque, sync::Arc};

use marlin_agent_protocol::{
    LoopProgram, LoopProgramActionKind, LoopProgramEventKind, LoopProgramId,
};

use crate::{
    GenericLoopMachine, GenericLoopMachineError, GenericLoopMachineReceipt,
    LoopProgramRuntimeHandoffExecutionReceipt, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffExecutor, LoopProgramRuntimeHandoffPlan,
    LoopProgramRuntimeHandoffRouter,
};

/// Explicit request for driving a compiled `LoopProgram` through a runtime event pump.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramExecutionRequest {
    pub program: LoopProgram,
    pub initial_events: Box<[LoopProgramEventKind]>,
}

impl LoopProgramExecutionRequest {
    pub fn new(
        program: LoopProgram,
        initial_events: impl Into<Box<[LoopProgramEventKind]>>,
    ) -> Self {
        Self {
            program,
            initial_events: initial_events.into(),
        }
    }
}

/// Terminal status for one event-pumped `LoopProgram` execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramExecutionStatus {
    Completed,
    Stopped,
    Rejected,
    StepLimitExceeded,
}

/// Step-level receipt joining the pure machine step with its runtime handoff result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramExecutionStepReceipt {
    pub machine_receipt: GenericLoopMachineReceipt,
    pub runtime_handoff_plan: LoopProgramRuntimeHandoffPlan,
    pub runtime_handoff_execution: LoopProgramRuntimeHandoffExecutionReceipt,
    pub generated_event: Option<LoopProgramEventKind>,
}

/// Run-level receipt emitted by the provider-neutral event pump.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramExecutionReceipt {
    pub program_id: LoopProgramId,
    pub status: LoopProgramExecutionStatus,
    pub steps: Box<[LoopProgramExecutionStepReceipt]>,
    pub error: Option<GenericLoopMachineError>,
}

impl LoopProgramExecutionReceipt {
    fn new(
        program_id: LoopProgramId,
        status: LoopProgramExecutionStatus,
        steps: Vec<LoopProgramExecutionStepReceipt>,
        error: Option<GenericLoopMachineError>,
    ) -> Self {
        Self {
            program_id,
            status,
            steps: steps.into_boxed_slice(),
            error,
        }
    }
}

/// Converts a handled runtime handoff receipt into the next machine event.
pub trait LoopProgramEventMapper: Send + Sync + 'static {
    fn next_event(
        &self,
        machine_receipt: &GenericLoopMachineReceipt,
        runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> Option<LoopProgramEventKind>;
}

/// Mapper that never fabricates follow-up events.
#[derive(Clone, Debug, Default)]
pub struct TerminalLoopProgramEventMapper;

impl LoopProgramEventMapper for TerminalLoopProgramEventMapper {
    fn next_event(
        &self,
        _machine_receipt: &GenericLoopMachineReceipt,
        _runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> Option<LoopProgramEventKind> {
        None
    }
}

/// Mapper that advances the loop from runtime-owned handoff execution receipts.
#[derive(Clone, Debug, Default)]
pub struct ReceiptDrivenLoopProgramEventMapper;

impl LoopProgramEventMapper for ReceiptDrivenLoopProgramEventMapper {
    fn next_event(
        &self,
        _machine_receipt: &GenericLoopMachineReceipt,
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

        unambiguous_receipt_next_event(runtime_handoff_execution)
    }
}

/// Scripted mapper for tests and deterministic smoke fixtures.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScriptedLoopProgramEventMapper {
    mappings: Box<[(LoopProgramActionKind, LoopProgramEventKind)]>,
}

impl ScriptedLoopProgramEventMapper {
    pub fn new(mappings: impl Into<Box<[(LoopProgramActionKind, LoopProgramEventKind)]>>) -> Self {
        Self {
            mappings: mappings.into(),
        }
    }
}

impl LoopProgramEventMapper for ScriptedLoopProgramEventMapper {
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

        self.mappings
            .iter()
            .find_map(|(action, event)| (action == &machine_receipt.action).then(|| event.clone()))
    }
}

/// Runtime event pump around the pure `GenericLoopMachine`.
#[derive(Clone)]
pub struct LoopProgramExecutionDriver {
    handoff_executor: Arc<dyn LoopProgramRuntimeHandoffExecutor>,
    event_mapper: Arc<dyn LoopProgramEventMapper>,
    max_steps: usize,
}

impl LoopProgramExecutionDriver {
    pub fn new<E>(handoff_executor: E) -> Self
    where
        E: LoopProgramRuntimeHandoffExecutor,
    {
        Self {
            handoff_executor: Arc::new(handoff_executor),
            event_mapper: Arc::new(TerminalLoopProgramEventMapper),
            max_steps: 128,
        }
    }

    pub fn with_event_mapper<M>(mut self, event_mapper: M) -> Self
    where
        M: LoopProgramEventMapper,
    {
        self.event_mapper = Arc::new(event_mapper);
        self
    }

    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    pub fn run(&self, request: LoopProgramExecutionRequest) -> LoopProgramExecutionReceipt {
        let program_id = request.program.program_id.clone();
        let step_capacity = execution_step_capacity(
            self.max_steps,
            &request.program,
            request.initial_events.len(),
        );
        let initial_events = request.initial_events;
        let mut machine = GenericLoopMachine::new(request.program);
        let mut events = initial_event_queue(initial_events);
        let mut steps = Vec::with_capacity(step_capacity);

        self.run_event_pump(program_id, &mut machine, &mut events, &mut steps)
    }

    fn run_event_pump(
        &self,
        program_id: LoopProgramId,
        machine: &mut GenericLoopMachine,
        events: &mut VecDeque<LoopProgramEventKind>,
        steps: &mut Vec<LoopProgramExecutionStepReceipt>,
    ) -> LoopProgramExecutionReceipt {
        while let Some(event) = events.pop_front() {
            if steps.len() >= self.max_steps {
                return LoopProgramExecutionReceipt::new(
                    program_id,
                    LoopProgramExecutionStatus::StepLimitExceeded,
                    std::mem::take(steps),
                    None,
                );
            }

            let step = match machine.apply_event(event) {
                Ok(step) => step,
                Err(error) => {
                    return LoopProgramExecutionReceipt::new(
                        program_id,
                        LoopProgramExecutionStatus::Rejected,
                        std::mem::take(steps),
                        Some(error),
                    );
                }
            };
            let runtime_handoff_plan = LoopProgramRuntimeHandoffPlan::from_receipts(
                program_id.clone(),
                std::slice::from_ref(&step.receipt),
            );
            let runtime_handoff_execution =
                self.handoff_executor.execute_plan(&runtime_handoff_plan);
            let stopped = machine.is_stopped();
            let generated_event = (!stopped)
                .then(|| {
                    self.event_mapper
                        .next_event(&step.receipt, &runtime_handoff_execution)
                })
                .flatten();
            if let Some(event) = generated_event.clone() {
                events.push_back(event);
            }

            steps.push(LoopProgramExecutionStepReceipt {
                machine_receipt: step.receipt,
                runtime_handoff_plan,
                runtime_handoff_execution,
                generated_event,
            });

            if stopped {
                return LoopProgramExecutionReceipt::new(
                    program_id,
                    LoopProgramExecutionStatus::Stopped,
                    std::mem::take(steps),
                    None,
                );
            }
        }

        LoopProgramExecutionReceipt::new(
            program_id,
            LoopProgramExecutionStatus::Completed,
            std::mem::take(steps),
            None,
        )
    }
}

impl Default for LoopProgramExecutionDriver {
    fn default() -> Self {
        Self::new(LoopProgramRuntimeHandoffRouter::default())
    }
}

fn execution_step_capacity(
    max_steps: usize,
    program: &LoopProgram,
    initial_event_count: usize,
) -> usize {
    max_steps.min(
        program
            .transitions
            .len()
            .saturating_add(initial_event_count)
            .max(1),
    )
}

fn initial_event_queue(
    initial_events: Box<[LoopProgramEventKind]>,
) -> VecDeque<LoopProgramEventKind> {
    let mut events = VecDeque::with_capacity(initial_events.len().max(1));
    events.extend(initial_events);
    events
}

fn unambiguous_receipt_next_event(
    runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
) -> Option<LoopProgramEventKind> {
    let mut next_events = runtime_handoff_execution
        .executions
        .iter()
        .filter_map(|execution| execution.next_event.as_ref());
    let first = next_events.next()?.clone();
    if next_events.all(|event| event == &first) {
        Some(first)
    } else {
        None
    }
}
