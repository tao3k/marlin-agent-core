//! Replay receipts that keep `AgentFlow` evidence separate from side effects.

use marlin_agent_protocol::{AgentFlowReceipt, LoopProgramId};

use crate::{LoopProgramExecutionStatus, LoopProgramRuntimeHandoffExecutionReceipt};

use super::{
    LoopProgramDerivedSessionPolicyStatus, LoopProgramFileWriteSideEffectReceipt,
    LoopProgramFileWriteSideEffectStatus, LoopProgramRuntimeSideEffectStatus,
    LoopProgramToolProcessSideEffectReceipt, LoopProgramToolProcessSideEffectStatus,
};

/// Run-level side-effect receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramRuntimeSideEffectReceipt {
    pub program_id: LoopProgramId,
    pub status: LoopProgramRuntimeSideEffectStatus,
    pub tool_processes: Box<[LoopProgramToolProcessSideEffectReceipt]>,
    pub file_writes: Box<[LoopProgramFileWriteSideEffectReceipt]>,
}

impl LoopProgramRuntimeSideEffectReceipt {
    pub(crate) fn new(
        program_id: LoopProgramId,
        tool_processes: Vec<LoopProgramToolProcessSideEffectReceipt>,
        file_writes: Vec<LoopProgramFileWriteSideEffectReceipt>,
    ) -> Self {
        let status = side_effect_status(&tool_processes, &file_writes);
        Self {
            program_id,
            status,
            tool_processes: tool_processes.into_boxed_slice(),
            file_writes: file_writes.into_boxed_slice(),
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
    pub(crate) fn new(
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

fn side_effect_status(
    tool_processes: &[LoopProgramToolProcessSideEffectReceipt],
    file_writes: &[LoopProgramFileWriteSideEffectReceipt],
) -> LoopProgramRuntimeSideEffectStatus {
    let total = tool_processes.len() + file_writes.len();
    if total == 0 {
        return LoopProgramRuntimeSideEffectStatus::Empty;
    }
    let (tool_failed, skipped) =
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
    let file_failed = file_writes.iter().any(|receipt| {
        matches!(
            receipt.status,
            LoopProgramFileWriteSideEffectStatus::Denied
                | LoopProgramFileWriteSideEffectStatus::Failed
        )
    });
    let failed = tool_failed || file_failed;
    if failed {
        return LoopProgramRuntimeSideEffectStatus::Failed;
    }
    if skipped == total {
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
