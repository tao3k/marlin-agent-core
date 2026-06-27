//! Tool-process projection and spawn receipts for `LoopProgram` handoffs.

use std::{io, process::Stdio};

use marlin_agent_protocol::LoopProgramId;
use marlin_agent_runtime::{
    RuntimeContext,
    observability::{
        AsyncManagedChildProcess, ManagedChildProcessSpec, RuntimeCommandObservation,
        RuntimeProcessKind, RuntimeProcessOutput,
    },
};
use tokio::process::Command as AsyncCommand;

use crate::{GenericLoopMachineStepIndex, LoopProgramRuntimeHandoff};

use super::LoopProgramRuntimeOwner;

/// Tool-process projection derived from a handled Agent-Flow tool intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramToolProcessProjectionReceipt {
    pub program_id: LoopProgramId,
    pub step_index: GenericLoopMachineStepIndex,
    pub owner: LoopProgramRuntimeOwner,
    pub command: RuntimeCommandObservation,
}

impl LoopProgramToolProcessProjectionReceipt {
    pub fn new(
        owner: LoopProgramRuntimeOwner,
        handoff: &LoopProgramRuntimeHandoff,
        command: RuntimeCommandObservation,
    ) -> Self {
        Self {
            program_id: handoff.receipt.program_id.clone(),
            step_index: handoff.receipt.step_index,
            owner,
            command,
        }
    }
}

/// Explicit executable used when a tool-process projection is allowed to spawn.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LoopProgramToolProcessProgram(String);

impl LoopProgramToolProcessProgram {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Named request for executing a projected tool process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramToolProcessSpawnRequest {
    pub projection: LoopProgramToolProcessProjectionReceipt,
    pub program: LoopProgramToolProcessProgram,
    pub args: Box<[String]>,
    pub started_at_ms: u64,
    pub observed_at_ms: u64,
}

impl LoopProgramToolProcessSpawnRequest {
    pub fn new(
        projection: LoopProgramToolProcessProjectionReceipt,
        program: LoopProgramToolProcessProgram,
    ) -> Self {
        Self {
            projection,
            program,
            args: Box::new([]),
            started_at_ms: 0,
            observed_at_ms: 0,
        }
    }

    pub fn with_args(mut self, args: impl Into<Box<[String]>>) -> Self {
        self.args = args.into();
        self
    }

    pub fn with_started_at_ms(mut self, started_at_ms: u64) -> Self {
        self.started_at_ms = started_at_ms;
        self
    }

    pub fn with_observed_at_ms(mut self, observed_at_ms: u64) -> Self {
        self.observed_at_ms = observed_at_ms;
        self
    }
}

/// Receipt emitted after executing a projected tool process through runtime process ownership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramToolProcessSpawnReceipt {
    pub projection: LoopProgramToolProcessProjectionReceipt,
    pub pid: u32,
    pub output: RuntimeProcessOutput,
}

/// Executes an explicitly approved tool-process projection through runtime process ownership.
pub async fn spawn_loop_program_tool_process(
    context: &RuntimeContext,
    request: LoopProgramToolProcessSpawnRequest,
) -> io::Result<LoopProgramToolProcessSpawnReceipt> {
    let mut command = AsyncCommand::new(request.program.as_str());
    command
        .args(request.args.iter().map(String::as_str))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let owner_reference = format!(
        "loop-program:{}:tool-process:{}",
        request.projection.program_id.as_str(),
        request.projection.step_index.get()
    );
    let child = AsyncManagedChildProcess::spawn_with_spec(
        context,
        command,
        ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, owner_reference)
            .with_command(request.projection.command.clone())
            .with_started_at_ms(request.started_at_ms),
    )
    .await?;
    let pid = child.pid();
    let output = child
        .wait_with_output_observed_at(request.observed_at_ms)
        .await?;

    Ok(LoopProgramToolProcessSpawnReceipt {
        projection: request.projection,
        pid,
        output,
    })
}
