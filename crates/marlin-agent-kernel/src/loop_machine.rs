//! Provider-neutral `LoopProgram` state machine for kernel runtime handoff.

use marlin_agent_protocol::{
    LoopMechanismPolicyId, LoopProgram, LoopProgramActionKind, LoopProgramEventKind, LoopProgramId,
    LoopProgramStateId, LoopProgramTransitionId,
};

/// Provider-neutral state machine for compiled LoopProgram artifacts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericLoopMachine {
    program: LoopProgram,
    current_state: LoopProgramStateId,
    step_index: GenericLoopMachineStepIndex,
    stopped: bool,
}

impl GenericLoopMachine {
    pub fn new(program: LoopProgram) -> Self {
        let current_state = program.initial_state.clone();
        Self {
            program,
            current_state,
            step_index: GenericLoopMachineStepIndex::new(0),
            stopped: false,
        }
    }

    pub fn program(&self) -> &LoopProgram {
        &self.program
    }

    pub fn current_state(&self) -> &LoopProgramStateId {
        &self.current_state
    }

    pub fn step_index(&self) -> GenericLoopMachineStepIndex {
        self.step_index
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    pub fn uses_policy(&self, policy_id: &LoopMechanismPolicyId) -> bool {
        self.program.uses_policy(policy_id)
    }

    pub fn apply_event(
        &mut self,
        event: LoopProgramEventKind,
    ) -> Result<GenericLoopMachineStep, GenericLoopMachineError> {
        if self.stopped {
            return Err(GenericLoopMachineError::AlreadyStopped {
                program_id: self.program.program_id.clone(),
                state: self.current_state.clone(),
            });
        }

        let transition = self
            .program
            .transitions
            .iter()
            .find(|transition| transition.from == self.current_state && transition.event == event)
            .cloned()
            .ok_or_else(|| GenericLoopMachineError::NoTransition {
                program_id: self.program.program_id.clone(),
                state: self.current_state.clone(),
                event: event.clone(),
            })?;

        let from = self.current_state.clone();
        self.current_state = transition.to.clone();
        self.step_index = self.step_index.next();
        self.stopped = matches!(transition.action, LoopProgramActionKind::Stop);

        Ok(GenericLoopMachineStep {
            action: transition.action.clone(),
            receipt: GenericLoopMachineReceipt {
                program_id: self.program.program_id.clone(),
                step_index: self.step_index,
                transition_id: transition.transition_id,
                from,
                event,
                action: transition.action,
                to: self.current_state.clone(),
                stopped: self.stopped,
            },
        })
    }
}

/// Monotonic in-memory step index for one GenericLoopMachine run.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct GenericLoopMachineStepIndex(u64);

impl GenericLoopMachineStepIndex {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// One state-machine step with the runtime action and replayable receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericLoopMachineStep {
    pub action: LoopProgramActionKind,
    pub receipt: GenericLoopMachineReceipt,
}

/// Replayable receipt emitted for each GenericLoopMachine transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericLoopMachineReceipt {
    pub program_id: LoopProgramId,
    pub step_index: GenericLoopMachineStepIndex,
    pub transition_id: LoopProgramTransitionId,
    pub from: LoopProgramStateId,
    pub event: LoopProgramEventKind,
    pub action: LoopProgramActionKind,
    pub to: LoopProgramStateId,
    pub stopped: bool,
}

/// Deterministic transition errors for provider-neutral loop execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenericLoopMachineError {
    NoTransition {
        program_id: LoopProgramId,
        state: LoopProgramStateId,
        event: LoopProgramEventKind,
    },
    AlreadyStopped {
        program_id: LoopProgramId,
        state: LoopProgramStateId,
    },
}
