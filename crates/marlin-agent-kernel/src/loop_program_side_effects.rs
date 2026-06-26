//! Runtime side-effect execution for typed `LoopProgram` handoff projections.

use std::{io, sync::Arc};

use marlin_agent_protocol::{AgentFlowReceipt, LoopProgramId};
use marlin_agent_runtime::RuntimeContext;

use crate::{
    LoopProgramExecutionReceipt, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReceipt, LoopProgramToolProcessProgram,
    LoopProgramToolProcessProjectionReceipt, LoopProgramToolProcessSpawnReceipt,
    LoopProgramToolProcessSpawnRequest, spawn_loop_program_tool_process,
};

/// Aggregate status for runtime side effects derived from one handoff execution receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramRuntimeSideEffectStatus {
    Empty,
    Completed,
    Partial,
    Skipped,
    Failed,
}

/// Status for one tool-process side effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramToolProcessSideEffectStatus {
    Completed,
    Skipped,
    Failed,
}

/// Derived-session replay policy status after runtime side effects are observed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramDerivedSessionPolicyStatus {
    Ready,
    Deferred,
    Blocked,
    MissingDerivedSession,
    ReceiptMismatch,
}

impl LoopProgramDerivedSessionPolicyStatus {
    pub fn allows_replay(&self) -> bool {
        matches!(self, Self::Ready)
    }

    pub fn requires_follow_up(&self) -> bool {
        matches!(self, Self::Deferred)
    }

    pub fn blocks_replay(&self) -> bool {
        matches!(
            self,
            Self::Blocked | Self::MissingDerivedSession | Self::ReceiptMismatch
        )
    }
}

/// Tool-process side-effect receipt with typed projection provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramToolProcessSideEffectReceipt {
    pub projection: LoopProgramToolProcessProjectionReceipt,
    pub status: LoopProgramToolProcessSideEffectStatus,
    pub spawn_receipt: Option<LoopProgramToolProcessSpawnReceipt>,
    pub diagnostic: Option<String>,
}

impl LoopProgramToolProcessSideEffectReceipt {
    fn completed(spawn_receipt: LoopProgramToolProcessSpawnReceipt) -> Self {
        Self {
            projection: spawn_receipt.projection.clone(),
            status: if spawn_receipt.output.status.success() {
                LoopProgramToolProcessSideEffectStatus::Completed
            } else {
                LoopProgramToolProcessSideEffectStatus::Failed
            },
            diagnostic: (!spawn_receipt.output.status.success()).then(|| {
                format!(
                    "loop_program.tool_process.exit_status={:?}",
                    spawn_receipt.output.status.code()
                )
            }),
            spawn_receipt: Some(spawn_receipt),
        }
    }

    fn skipped(projection: LoopProgramToolProcessProjectionReceipt) -> Self {
        Self {
            projection,
            status: LoopProgramToolProcessSideEffectStatus::Skipped,
            spawn_receipt: None,
            diagnostic: Some("loop_program.tool_process.unresolved_projection".to_owned()),
        }
    }

    fn failed(projection: LoopProgramToolProcessProjectionReceipt, error: io::Error) -> Self {
        Self {
            projection,
            status: LoopProgramToolProcessSideEffectStatus::Failed,
            spawn_receipt: None,
            diagnostic: Some(format!("loop_program.tool_process.spawn_failed:{error}")),
        }
    }
}

/// Run-level side-effect receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramRuntimeSideEffectReceipt {
    pub program_id: LoopProgramId,
    pub status: LoopProgramRuntimeSideEffectStatus,
    pub tool_processes: Box<[LoopProgramToolProcessSideEffectReceipt]>,
}

impl LoopProgramRuntimeSideEffectReceipt {
    fn new(
        program_id: LoopProgramId,
        tool_processes: Vec<LoopProgramToolProcessSideEffectReceipt>,
    ) -> Self {
        let status = side_effect_status(&tool_processes);
        Self {
            program_id,
            status,
            tool_processes: tool_processes.into_boxed_slice(),
        }
    }
}

/// Replay bundle that keeps Agent-Flow handoff evidence separate from runtime side effects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramRuntimeReplayBundleReceipt {
    pub program_id: LoopProgramId,
    pub policy_status: LoopProgramDerivedSessionPolicyStatus,
    pub handoff_execution: LoopProgramRuntimeHandoffExecutionReceipt,
    pub side_effects: LoopProgramRuntimeSideEffectReceipt,
}

impl LoopProgramRuntimeReplayBundleReceipt {
    pub fn from_runtime_receipts(
        handoff_execution: LoopProgramRuntimeHandoffExecutionReceipt,
        side_effects: LoopProgramRuntimeSideEffectReceipt,
    ) -> Self {
        let policy_status = derived_session_policy_status(&handoff_execution, &side_effects);
        Self {
            program_id: handoff_execution.program_id.clone(),
            policy_status,
            handoff_execution,
            side_effects,
        }
    }

    pub fn agent_flow_receipt(&self) -> Option<&AgentFlowReceipt> {
        self.handoff_execution.agent_flow_receipt.as_ref()
    }

    pub fn allows_replay(&self) -> bool {
        self.policy_status.allows_replay()
    }

    pub fn requires_follow_up(&self) -> bool {
        self.policy_status.requires_follow_up()
    }

    pub fn blocks_replay(&self) -> bool {
        self.policy_status.blocks_replay()
    }
}

/// Replay bundle for side effects derived from a full event-pumped loop execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramExecutionReplayBundleReceipt {
    pub program_id: LoopProgramId,
    pub execution_status: LoopProgramExecutionStatus,
    pub policy_status: LoopProgramDerivedSessionPolicyStatus,
    pub step_replay_bundles: Box<[LoopProgramRuntimeReplayBundleReceipt]>,
}

impl LoopProgramExecutionReplayBundleReceipt {
    fn new(
        program_id: LoopProgramId,
        execution_status: LoopProgramExecutionStatus,
        step_replay_bundles: Vec<LoopProgramRuntimeReplayBundleReceipt>,
    ) -> Self {
        let policy_status = execution_replay_policy_status(&step_replay_bundles);
        Self {
            program_id,
            execution_status,
            policy_status,
            step_replay_bundles: step_replay_bundles.into_boxed_slice(),
        }
    }

    pub fn allows_replay(&self) -> bool {
        self.policy_status.allows_replay()
    }

    pub fn requires_follow_up(&self) -> bool {
        self.policy_status.requires_follow_up()
    }

    pub fn blocks_replay(&self) -> bool {
        self.policy_status.blocks_replay()
    }
}

/// Resolves a typed tool projection into an explicit runtime process request.
pub trait LoopProgramToolProcessResolver: Send + Sync + 'static {
    fn resolve(
        &self,
        projection: &LoopProgramToolProcessProjectionReceipt,
    ) -> Option<LoopProgramToolProcessSpawnRequest>;
}

/// Static projection resolver for tests, smoke fixtures, and TOML-backed configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StaticLoopProgramToolProcessResolver {
    templates: Box<[LoopProgramToolProcessCommandTemplate]>,
}

impl StaticLoopProgramToolProcessResolver {
    pub fn new(templates: impl Into<Box<[LoopProgramToolProcessCommandTemplate]>>) -> Self {
        Self {
            templates: templates.into(),
        }
    }
}

impl LoopProgramToolProcessResolver for StaticLoopProgramToolProcessResolver {
    fn resolve(
        &self,
        projection: &LoopProgramToolProcessProjectionReceipt,
    ) -> Option<LoopProgramToolProcessSpawnRequest> {
        self.templates
            .iter()
            .find(|template| template.matches(projection))
            .map(|template| template.spawn_request(projection.clone()))
    }
}

/// One static mapping from a projected command observation to an executable process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramToolProcessCommandTemplate {
    command_kind: String,
    argv: Box<[String]>,
    program: LoopProgramToolProcessProgram,
    args: Box<[String]>,
}

impl LoopProgramToolProcessCommandTemplate {
    pub fn new<I, S>(
        command_kind: impl Into<String>,
        argv: I,
        program: LoopProgramToolProcessProgram,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            command_kind: command_kind.into(),
            argv: argv.into_iter().map(Into::into).collect(),
            program,
            args: Box::new([]),
        }
    }

    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    fn matches(&self, projection: &LoopProgramToolProcessProjectionReceipt) -> bool {
        projection.command.command_kind.as_str() == self.command_kind.as_str()
            && projection.command.argv.as_slice() == self.argv.as_ref()
    }

    fn spawn_request(
        &self,
        projection: LoopProgramToolProcessProjectionReceipt,
    ) -> LoopProgramToolProcessSpawnRequest {
        LoopProgramToolProcessSpawnRequest::new(projection, self.program.clone())
            .with_args(self.args.clone())
    }
}

/// Async side-effect executor for handoff projections that require runtime ownership.
#[derive(Clone)]
pub struct LoopProgramRuntimeSideEffectExecutor {
    tool_process_resolver: Arc<dyn LoopProgramToolProcessResolver>,
    started_at_ms: u64,
    observed_at_ms: u64,
}

impl LoopProgramRuntimeSideEffectExecutor {
    pub fn new<R>(tool_process_resolver: R) -> Self
    where
        R: LoopProgramToolProcessResolver,
    {
        Self {
            tool_process_resolver: Arc::new(tool_process_resolver),
            started_at_ms: 0,
            observed_at_ms: 0,
        }
    }

    pub fn with_started_at_ms(mut self, started_at_ms: u64) -> Self {
        self.started_at_ms = started_at_ms;
        self
    }

    pub fn with_observed_at_ms(mut self, observed_at_ms: u64) -> Self {
        self.observed_at_ms = observed_at_ms;
        self
    }

    pub async fn execute(
        &self,
        context: &RuntimeContext,
        execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> LoopProgramRuntimeSideEffectReceipt {
        let mut tool_processes = Vec::with_capacity(execution.tool_process_projections.len());
        for projection in execution.tool_process_projections.iter().cloned() {
            let Some(request) = self.tool_process_resolver.resolve(&projection) else {
                tool_processes.push(LoopProgramToolProcessSideEffectReceipt::skipped(projection));
                continue;
            };
            let request = request
                .with_started_at_ms(self.started_at_ms)
                .with_observed_at_ms(self.observed_at_ms);
            match spawn_loop_program_tool_process(context, request).await {
                Ok(receipt) => {
                    tool_processes
                        .push(LoopProgramToolProcessSideEffectReceipt::completed(receipt));
                }
                Err(error) => {
                    tool_processes.push(LoopProgramToolProcessSideEffectReceipt::failed(
                        projection, error,
                    ));
                }
            }
        }

        LoopProgramRuntimeSideEffectReceipt::new(execution.program_id.clone(), tool_processes)
    }

    pub async fn execute_loop_execution(
        &self,
        context: &RuntimeContext,
        execution: &LoopProgramExecutionReceipt,
    ) -> LoopProgramExecutionReplayBundleReceipt {
        let mut step_replay_bundles = Vec::new();
        for step in execution.steps.iter() {
            if !has_projected_runtime_side_effects(&step.runtime_handoff_execution) {
                continue;
            }
            let side_effects = self.execute(context, &step.runtime_handoff_execution).await;
            step_replay_bundles.push(
                LoopProgramRuntimeReplayBundleReceipt::from_runtime_receipts(
                    step.runtime_handoff_execution.clone(),
                    side_effects,
                ),
            );
        }

        LoopProgramExecutionReplayBundleReceipt::new(
            execution.program_id.clone(),
            execution.status.clone(),
            step_replay_bundles,
        )
    }
}

impl Default for LoopProgramRuntimeSideEffectExecutor {
    fn default() -> Self {
        Self::new(StaticLoopProgramToolProcessResolver::default())
    }
}

fn side_effect_status(
    tool_processes: &[LoopProgramToolProcessSideEffectReceipt],
) -> LoopProgramRuntimeSideEffectStatus {
    if tool_processes.is_empty() {
        return LoopProgramRuntimeSideEffectStatus::Empty;
    }
    let (failed, skipped) =
        tool_processes
            .iter()
            .fold(
                (false, 0_usize),
                |(failed, skipped), receipt| match receipt.status {
                    LoopProgramToolProcessSideEffectStatus::Failed => (true, skipped),
                    LoopProgramToolProcessSideEffectStatus::Skipped => (failed, skipped + 1),
                    LoopProgramToolProcessSideEffectStatus::Completed => (failed, skipped),
                },
            );
    if failed {
        return LoopProgramRuntimeSideEffectStatus::Failed;
    }
    if skipped == tool_processes.len() {
        LoopProgramRuntimeSideEffectStatus::Skipped
    } else if skipped > 0 {
        LoopProgramRuntimeSideEffectStatus::Partial
    } else {
        LoopProgramRuntimeSideEffectStatus::Completed
    }
}

fn derived_session_policy_status(
    handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    side_effects: &LoopProgramRuntimeSideEffectReceipt,
) -> LoopProgramDerivedSessionPolicyStatus {
    if handoff_execution.program_id != side_effects.program_id {
        return LoopProgramDerivedSessionPolicyStatus::ReceiptMismatch;
    }
    if handoff_execution.agent_flow_receipt.is_none() {
        return LoopProgramDerivedSessionPolicyStatus::MissingDerivedSession;
    }

    match side_effects.status {
        LoopProgramRuntimeSideEffectStatus::Empty
        | LoopProgramRuntimeSideEffectStatus::Completed => {
            LoopProgramDerivedSessionPolicyStatus::Ready
        }
        LoopProgramRuntimeSideEffectStatus::Partial
        | LoopProgramRuntimeSideEffectStatus::Skipped => {
            LoopProgramDerivedSessionPolicyStatus::Deferred
        }
        LoopProgramRuntimeSideEffectStatus::Failed => {
            LoopProgramDerivedSessionPolicyStatus::Blocked
        }
    }
}

fn has_projected_runtime_side_effects(
    execution: &LoopProgramRuntimeHandoffExecutionReceipt,
) -> bool {
    execution.agent_flow_receipt.is_some()
        || !execution.tool_process_projections.is_empty()
        || !execution.memory_projections.is_empty()
}

fn execution_replay_policy_status(
    step_replay_bundles: &[LoopProgramRuntimeReplayBundleReceipt],
) -> LoopProgramDerivedSessionPolicyStatus {
    if step_replay_bundles
        .iter()
        .any(|bundle| bundle.blocks_replay())
    {
        return LoopProgramDerivedSessionPolicyStatus::Blocked;
    }
    if step_replay_bundles
        .iter()
        .any(|bundle| bundle.requires_follow_up())
    {
        return LoopProgramDerivedSessionPolicyStatus::Deferred;
    }
    LoopProgramDerivedSessionPolicyStatus::Ready
}
