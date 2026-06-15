//! Runtime receipt for configured sub-agent spawn profiles.

use marlin_agent_protocol::{SubAgentSpawnConfig, SubAgentSpawnProfile};
use marlin_agent_sessions::{RuntimeFanoutJoinPolicy, SessionId, SessionIsolationReceipt};

/// Runtime receipt for a configured sub-agent spawn profile.
#[derive(Clone, Debug)]
pub struct SubAgentSpawnReceipt {
    config: SubAgentSpawnConfig,
    isolation_receipt: SessionIsolationReceipt,
}

impl SubAgentSpawnReceipt {
    pub fn new(config: SubAgentSpawnConfig, isolation_receipt: SessionIsolationReceipt) -> Self {
        Self {
            config,
            isolation_receipt,
        }
    }

    pub fn config(&self) -> &SubAgentSpawnConfig {
        &self.config
    }

    pub fn isolation_receipt(&self) -> &SessionIsolationReceipt {
        &self.isolation_receipt
    }

    pub fn profile_id(&self) -> &str {
        self.config.profile_id.as_str()
    }

    pub fn agent_type(&self) -> &str {
        self.config.agent_type.as_str()
    }

    pub fn role(&self) -> &str {
        self.config.role.as_str()
    }

    pub fn nickname(&self) -> Option<&str> {
        self.config.nickname.as_deref()
    }

    pub fn child_session_id(&self) -> &SessionId {
        self.isolation_receipt.child_session_id()
    }

    pub fn activity_profile(&self) -> SubAgentSpawnProfile {
        SubAgentSpawnProfile::from_config(&self.config)
    }
}

/// Completion status for one runtime fanout task.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeFanoutTaskStatus {
    Completed,
    JoinError,
    CancelledBeforeStart,
}

/// Receipt for one task in a runtime fanout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeFanoutTaskReceipt {
    input_index: Option<usize>,
    status: RuntimeFanoutTaskStatus,
    join_error: Option<String>,
}

impl RuntimeFanoutTaskReceipt {
    pub fn completed(input_index: usize) -> Self {
        Self {
            input_index: Some(input_index),
            status: RuntimeFanoutTaskStatus::Completed,
            join_error: None,
        }
    }

    pub fn join_error(join_error: impl Into<String>) -> Self {
        Self {
            input_index: None,
            status: RuntimeFanoutTaskStatus::JoinError,
            join_error: Some(join_error.into()),
        }
    }

    pub fn cancelled_before_start(input_index: usize) -> Self {
        Self {
            input_index: Some(input_index),
            status: RuntimeFanoutTaskStatus::CancelledBeforeStart,
            join_error: None,
        }
    }

    pub fn input_index(&self) -> Option<usize> {
        self.input_index
    }

    pub fn status(&self) -> &RuntimeFanoutTaskStatus {
        &self.status
    }

    pub fn join_error_message(&self) -> Option<&str> {
        self.join_error.as_deref()
    }
}

/// Typed receipt for a runtime fanout join.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeFanoutReceipt {
    policy: RuntimeFanoutJoinPolicy,
    task_receipts: Vec<RuntimeFanoutTaskReceipt>,
}

impl RuntimeFanoutReceipt {
    pub fn new(policy: RuntimeFanoutJoinPolicy) -> Self {
        Self {
            policy,
            task_receipts: Vec::new(),
        }
    }

    pub fn push_task_receipt(&mut self, receipt: RuntimeFanoutTaskReceipt) {
        self.task_receipts.push(receipt);
    }

    pub fn policy(&self) -> &RuntimeFanoutJoinPolicy {
        &self.policy
    }

    pub fn task_receipts(&self) -> &[RuntimeFanoutTaskReceipt] {
        self.task_receipts.as_slice()
    }

    pub fn completed_count(&self) -> usize {
        self.count_status(&RuntimeFanoutTaskStatus::Completed)
    }

    pub fn join_error_count(&self) -> usize {
        self.count_status(&RuntimeFanoutTaskStatus::JoinError)
    }

    pub fn cancelled_before_start_count(&self) -> usize {
        self.count_status(&RuntimeFanoutTaskStatus::CancelledBeforeStart)
    }

    pub fn has_join_errors(&self) -> bool {
        self.join_error_count() > 0
    }

    fn count_status(&self, status: &RuntimeFanoutTaskStatus) -> usize {
        self.task_receipts
            .iter()
            .filter(|receipt| receipt.status() == status)
            .count()
    }
}

/// Successful fanout outputs plus their typed join receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeFanoutOutput<T> {
    outputs: Vec<T>,
    receipt: RuntimeFanoutReceipt,
}

impl<T> RuntimeFanoutOutput<T> {
    pub fn new(outputs: Vec<T>, receipt: RuntimeFanoutReceipt) -> Self {
        Self { outputs, receipt }
    }

    pub fn outputs(&self) -> &[T] {
        self.outputs.as_slice()
    }

    pub fn into_outputs(self) -> Vec<T> {
        self.outputs
    }

    pub fn receipt(&self) -> &RuntimeFanoutReceipt {
        &self.receipt
    }

    pub fn into_parts(self) -> (Vec<T>, RuntimeFanoutReceipt) {
        (self.outputs, self.receipt)
    }
}

/// Result returned by runtime fanout helpers.
pub type RuntimeFanoutResult<T> = Result<RuntimeFanoutOutput<T>, RuntimeFanoutReceipt>;

/// Shutdown status for a runtime task tracker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeTaskShutdownStatus {
    Completed,
    TimedOut,
}

/// Request mode represented by a runtime task shutdown receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeTaskShutdownRequest {
    WaitForTasks,
    CancelTasksAndWait,
}

impl RuntimeTaskShutdownRequest {
    pub fn cancellation_requested(&self) -> bool {
        matches!(self, Self::CancelTasksAndWait)
    }
}

/// Tracker state represented by a runtime task shutdown receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeTaskTrackerShutdownState {
    Open,
    Closed,
}

impl RuntimeTaskTrackerShutdownState {
    pub fn tracker_closed(&self) -> bool {
        matches!(self, Self::Closed)
    }
}

/// Named input for constructing runtime task shutdown receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeTaskShutdownReceiptInput {
    pub status: RuntimeTaskShutdownStatus,
    pub request: RuntimeTaskShutdownRequest,
    pub tracker_state: RuntimeTaskTrackerShutdownState,
    pub timeout_ms: Option<u64>,
    pub tracked_task_count_at_shutdown: usize,
    pub remaining_task_count: usize,
}

/// Receipt emitted when a runtime closes and waits for tracked tasks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeTaskShutdownReceipt {
    status: RuntimeTaskShutdownStatus,
    cancellation_requested: bool,
    tracker_closed: bool,
    timeout_ms: Option<u64>,
    tracked_task_count_at_shutdown: usize,
    remaining_task_count: usize,
}

impl RuntimeTaskShutdownReceipt {
    pub fn from_input(input: RuntimeTaskShutdownReceiptInput) -> Self {
        Self {
            status: input.status,
            cancellation_requested: input.request.cancellation_requested(),
            tracker_closed: input.tracker_state.tracker_closed(),
            timeout_ms: input.timeout_ms,
            tracked_task_count_at_shutdown: input.tracked_task_count_at_shutdown,
            remaining_task_count: input.remaining_task_count,
        }
    }

    pub fn status(&self) -> &RuntimeTaskShutdownStatus {
        &self.status
    }

    pub fn cancellation_requested(&self) -> bool {
        self.cancellation_requested
    }

    pub fn tracker_closed(&self) -> bool {
        self.tracker_closed
    }

    pub fn timeout_ms(&self) -> Option<u64> {
        self.timeout_ms
    }

    pub fn tracked_task_count_at_shutdown(&self) -> usize {
        self.tracked_task_count_at_shutdown
    }

    pub fn remaining_task_count(&self) -> usize {
        self.remaining_task_count
    }
}
