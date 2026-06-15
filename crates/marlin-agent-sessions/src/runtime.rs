//! Tokio runtime capability policies carried by session context.

use std::time::Duration;

use crate::SessionId;
use serde::{Deserialize, Serialize};

/// Tokio runtime flavor selected for a session-owned runtime boundary.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum TokioRuntimeFlavor {
    #[default]
    MultiThread,
    CurrentThread,
}

/// Diagnostics that may be requested for a Tokio runtime.
///
/// Unstable Tokio diagnostics remain requests here. Runtime construction owns
/// feature gates and platform checks before any unstable API is used.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokioRuntimeDiagnosticsPolicy {
    enabled: bool,
    taskdump_requested: bool,
    poll_time_histogram_requested: bool,
}

impl TokioRuntimeDiagnosticsPolicy {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn stable_metrics() -> Self {
        Self {
            enabled: true,
            taskdump_requested: false,
            poll_time_histogram_requested: false,
        }
    }

    pub fn with_taskdump(mut self) -> Self {
        self.enabled = true;
        self.taskdump_requested = true;
        self
    }

    pub fn with_poll_time_histogram(mut self) -> Self {
        self.enabled = true;
        self.poll_time_histogram_requested = true;
        self
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn taskdump_requested(&self) -> bool {
        self.taskdump_requested
    }

    pub fn poll_time_histogram_requested(&self) -> bool {
        self.poll_time_histogram_requested
    }

    pub fn unstable_requested(&self) -> bool {
        self.taskdump_requested || self.poll_time_histogram_requested
    }
}

/// Stable Tokio runtime construction policy owned by Marlin.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokioRuntimePolicy {
    flavor: TokioRuntimeFlavor,
    worker_threads: Option<usize>,
    max_blocking_threads: Option<usize>,
    thread_name_prefix: Option<String>,
    thread_keep_alive_ms: Option<u64>,
    event_interval: Option<u32>,
    global_queue_interval: Option<u32>,
    shutdown_timeout_ms: Option<u64>,
    diagnostics: TokioRuntimeDiagnosticsPolicy,
}

impl TokioRuntimePolicy {
    pub fn production_default() -> Self {
        Self {
            flavor: TokioRuntimeFlavor::MultiThread,
            worker_threads: None,
            max_blocking_threads: None,
            thread_name_prefix: None,
            thread_keep_alive_ms: None,
            event_interval: None,
            global_queue_interval: None,
            shutdown_timeout_ms: None,
            diagnostics: TokioRuntimeDiagnosticsPolicy::disabled(),
        }
    }

    pub fn current_thread() -> Self {
        Self {
            flavor: TokioRuntimeFlavor::CurrentThread,
            ..Self::production_default()
        }
    }

    pub fn with_worker_threads(mut self, worker_threads: Option<usize>) -> Self {
        self.worker_threads = worker_threads;
        self
    }

    pub fn with_max_blocking_threads(mut self, max_blocking_threads: Option<usize>) -> Self {
        self.max_blocking_threads = max_blocking_threads;
        self
    }

    pub fn with_thread_name_prefix(mut self, thread_name_prefix: Option<String>) -> Self {
        self.thread_name_prefix = thread_name_prefix;
        self
    }

    pub fn with_thread_keep_alive_ms(mut self, thread_keep_alive_ms: Option<u64>) -> Self {
        self.thread_keep_alive_ms = thread_keep_alive_ms;
        self
    }

    pub fn with_event_interval(mut self, event_interval: Option<u32>) -> Self {
        self.event_interval = event_interval;
        self
    }

    pub fn with_global_queue_interval(mut self, global_queue_interval: Option<u32>) -> Self {
        self.global_queue_interval = global_queue_interval;
        self
    }

    pub fn with_shutdown_timeout_ms(mut self, shutdown_timeout_ms: Option<u64>) -> Self {
        self.shutdown_timeout_ms = shutdown_timeout_ms;
        self
    }

    pub fn with_diagnostics(mut self, diagnostics: TokioRuntimeDiagnosticsPolicy) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub fn flavor(&self) -> &TokioRuntimeFlavor {
        &self.flavor
    }

    pub fn worker_threads(&self) -> Option<usize> {
        self.worker_threads
    }

    pub fn max_blocking_threads(&self) -> Option<usize> {
        self.max_blocking_threads
    }

    pub fn thread_name_prefix(&self) -> Option<&str> {
        self.thread_name_prefix.as_deref()
    }

    pub fn thread_keep_alive_ms(&self) -> Option<u64> {
        self.thread_keep_alive_ms
    }

    pub fn event_interval(&self) -> Option<u32> {
        self.event_interval
    }

    pub fn global_queue_interval(&self) -> Option<u32> {
        self.global_queue_interval
    }

    pub fn shutdown_timeout_ms(&self) -> Option<u64> {
        self.shutdown_timeout_ms
    }

    pub fn diagnostics(&self) -> &TokioRuntimeDiagnosticsPolicy {
        &self.diagnostics
    }
}

impl Default for TokioRuntimePolicy {
    fn default() -> Self {
        Self::production_default()
    }
}

/// Policy for tasks spawned through a session runtime boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeTaskTrackerPolicy {
    cancel_on_shutdown: bool,
    shutdown_timeout_ms: Option<u64>,
}

impl RuntimeTaskTrackerPolicy {
    pub fn cancel_on_shutdown(shutdown_timeout_ms: Option<u64>) -> Self {
        Self {
            cancel_on_shutdown: true,
            shutdown_timeout_ms,
        }
    }

    pub fn wait_on_shutdown(shutdown_timeout_ms: Option<u64>) -> Self {
        Self {
            cancel_on_shutdown: false,
            shutdown_timeout_ms,
        }
    }

    pub fn cancel_on_shutdown_enabled(&self) -> bool {
        self.cancel_on_shutdown
    }

    pub fn shutdown_timeout_ms(&self) -> Option<u64> {
        self.shutdown_timeout_ms
    }

    pub fn shutdown_timeout_duration(&self) -> Option<Duration> {
        self.shutdown_timeout_ms.map(Duration::from_millis)
    }
}

impl Default for RuntimeTaskTrackerPolicy {
    fn default() -> Self {
        Self::cancel_on_shutdown(None)
    }
}

/// Policy for fanout joins spawned from a session runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeFanoutJoinPolicy {
    max_parallelism: Option<usize>,
    cancel_on_first_error: bool,
    preserve_input_order: bool,
    retain_completion_order: bool,
    shutdown_timeout_ms: Option<u64>,
}

impl RuntimeFanoutJoinPolicy {
    pub fn bounded(max_parallelism: usize) -> Self {
        Self {
            max_parallelism: Some(max_parallelism),
            ..Self::default()
        }
    }

    pub fn with_cancel_on_first_error(mut self, cancel_on_first_error: bool) -> Self {
        self.cancel_on_first_error = cancel_on_first_error;
        self
    }

    pub fn with_preserve_input_order(mut self, preserve_input_order: bool) -> Self {
        self.preserve_input_order = preserve_input_order;
        self
    }

    pub fn with_retain_completion_order(mut self, retain_completion_order: bool) -> Self {
        self.retain_completion_order = retain_completion_order;
        self
    }

    pub fn with_shutdown_timeout_ms(mut self, shutdown_timeout_ms: Option<u64>) -> Self {
        self.shutdown_timeout_ms = shutdown_timeout_ms;
        self
    }

    pub fn max_parallelism(&self) -> Option<usize> {
        self.max_parallelism
    }

    pub fn cancel_on_first_error(&self) -> bool {
        self.cancel_on_first_error
    }

    pub fn preserve_input_order(&self) -> bool {
        self.preserve_input_order
    }

    pub fn retain_completion_order(&self) -> bool {
        self.retain_completion_order
    }

    pub fn shutdown_timeout_ms(&self) -> Option<u64> {
        self.shutdown_timeout_ms
    }
}

impl Default for RuntimeFanoutJoinPolicy {
    fn default() -> Self {
        Self {
            max_parallelism: None,
            cancel_on_first_error: true,
            preserve_input_order: true,
            retain_completion_order: false,
            shutdown_timeout_ms: None,
        }
    }
}

/// Strategy used when a session must bridge synchronous and async work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeBlockingBridgeStrategy {
    MultiThreadBlockInPlace,
    SpawnBlocking,
    HelperThread,
    OutsideRuntimeDirect,
}

/// Policy for runtime blocking bridges.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeBlockingBridgePolicy {
    strategy: RuntimeBlockingBridgeStrategy,
    blocking_non_cancellable: bool,
}

impl RuntimeBlockingBridgePolicy {
    pub fn for_runtime_flavor(flavor: &TokioRuntimeFlavor) -> Self {
        match flavor {
            TokioRuntimeFlavor::MultiThread => {
                Self::new(RuntimeBlockingBridgeStrategy::MultiThreadBlockInPlace, true)
            }
            TokioRuntimeFlavor::CurrentThread => {
                Self::new(RuntimeBlockingBridgeStrategy::SpawnBlocking, true)
            }
        }
    }

    pub fn new(strategy: RuntimeBlockingBridgeStrategy, blocking_non_cancellable: bool) -> Self {
        Self {
            strategy,
            blocking_non_cancellable,
        }
    }

    pub fn strategy(&self) -> &RuntimeBlockingBridgeStrategy {
        &self.strategy
    }

    pub fn blocking_non_cancellable(&self) -> bool {
        self.blocking_non_cancellable
    }
}

impl Default for RuntimeBlockingBridgePolicy {
    fn default() -> Self {
        Self::for_runtime_flavor(&TokioRuntimeFlavor::MultiThread)
    }
}

/// Runtime capability snapshot attached to a session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionRuntimeSnapshot {
    session_id: SessionId,
    tokio_policy: TokioRuntimePolicy,
    task_tracker: RuntimeTaskTrackerPolicy,
    fanout_join: RuntimeFanoutJoinPolicy,
    blocking_bridge: RuntimeBlockingBridgePolicy,
}

impl SessionRuntimeSnapshot {
    pub fn new(session_id: impl Into<SessionId>, tokio_policy: TokioRuntimePolicy) -> Self {
        let blocking_bridge =
            RuntimeBlockingBridgePolicy::for_runtime_flavor(tokio_policy.flavor());
        Self {
            session_id: session_id.into(),
            tokio_policy,
            task_tracker: RuntimeTaskTrackerPolicy::default(),
            fanout_join: RuntimeFanoutJoinPolicy::default(),
            blocking_bridge,
        }
    }

    pub fn with_task_tracker(mut self, task_tracker: RuntimeTaskTrackerPolicy) -> Self {
        self.task_tracker = task_tracker;
        self
    }

    pub fn with_fanout_join(mut self, fanout_join: RuntimeFanoutJoinPolicy) -> Self {
        self.fanout_join = fanout_join;
        self
    }

    pub fn with_blocking_bridge(mut self, blocking_bridge: RuntimeBlockingBridgePolicy) -> Self {
        self.blocking_bridge = blocking_bridge;
        self
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn tokio_policy(&self) -> &TokioRuntimePolicy {
        &self.tokio_policy
    }

    pub fn task_tracker(&self) -> &RuntimeTaskTrackerPolicy {
        &self.task_tracker
    }

    pub fn fanout_join(&self) -> &RuntimeFanoutJoinPolicy {
        &self.fanout_join
    }

    pub fn blocking_bridge(&self) -> &RuntimeBlockingBridgePolicy {
        &self.blocking_bridge
    }
}

/// Audit record for the effective Tokio runtime policy selected for a session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokioRuntimePolicyReceipt {
    session_id: SessionId,
    effective_policy: TokioRuntimePolicy,
    diagnostics_enabled: bool,
    unstable_diagnostics_requested: bool,
    shutdown_timeout_ms: Option<u64>,
}

impl TokioRuntimePolicyReceipt {
    pub fn from_snapshot(snapshot: &SessionRuntimeSnapshot) -> Self {
        let policy = snapshot.tokio_policy().clone();
        Self {
            session_id: snapshot.session_id().clone(),
            diagnostics_enabled: policy.diagnostics().enabled(),
            unstable_diagnostics_requested: policy.diagnostics().unstable_requested(),
            shutdown_timeout_ms: policy.shutdown_timeout_ms(),
            effective_policy: policy,
        }
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn effective_policy(&self) -> &TokioRuntimePolicy {
        &self.effective_policy
    }

    pub fn diagnostics_enabled(&self) -> bool {
        self.diagnostics_enabled
    }

    pub fn unstable_diagnostics_requested(&self) -> bool {
        self.unstable_diagnostics_requested
    }

    pub fn shutdown_timeout_ms(&self) -> Option<u64> {
        self.shutdown_timeout_ms
    }
}
