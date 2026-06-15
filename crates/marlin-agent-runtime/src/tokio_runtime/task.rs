//! Tokio task handle wrappers.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::{sync::Notify, task::JoinError};

use super::{
    RuntimeTaskShutdownReceipt, RuntimeTaskShutdownReceiptInput, RuntimeTaskShutdownRequest,
    RuntimeTaskShutdownStatus, RuntimeTaskTrackerShutdownState,
};

/// Tokio task handle with a stable marlin-owned name.
#[derive(Debug)]
pub struct RuntimeTask<T> {
    handle: tokio::task::JoinHandle<T>,
}

/// Outcome for work spawned through Tokio cancellation-aware helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeTaskOutcome<T> {
    Completed(T),
    Cancelled,
}

impl<T> RuntimeTask<T> {
    pub fn new(handle: tokio::task::JoinHandle<T>) -> Self {
        Self { handle }
    }

    pub fn abort(&self) {
        self.handle.abort();
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    pub async fn join(self) -> Result<T, JoinError> {
        self.handle.await
    }
}

/// Tracks tasks spawned through a runtime boundary.
#[derive(Clone, Debug)]
pub struct RuntimeTaskTracker {
    state: Arc<RuntimeTaskTrackerState>,
}

#[derive(Debug)]
struct RuntimeTaskTrackerState {
    active_tasks: AtomicUsize,
    closed: AtomicBool,
    notify: Notify,
}

#[derive(Debug)]
pub(crate) struct RuntimeTaskGuard {
    tracker: RuntimeTaskTracker,
}

impl RuntimeTaskTracker {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RuntimeTaskTrackerState {
                active_tasks: AtomicUsize::new(0),
                closed: AtomicBool::new(false),
                notify: Notify::new(),
            }),
        }
    }

    pub fn active_task_count(&self) -> usize {
        self.state.active_tasks.load(Ordering::SeqCst)
    }

    pub fn is_closed(&self) -> bool {
        self.state.closed.load(Ordering::SeqCst)
    }

    pub fn close(&self) {
        self.state.closed.store(true, Ordering::SeqCst);
        if self.active_task_count() == 0 {
            self.state.notify.notify_waiters();
        }
    }

    pub async fn wait(&self) {
        loop {
            let notified = self.state.notify.notified();
            if self.active_task_count() == 0 {
                break;
            }
            notified.await;
        }
    }

    pub async fn close_and_wait(
        &self,
        timeout_duration: Option<Duration>,
        request: RuntimeTaskShutdownRequest,
    ) -> RuntimeTaskShutdownReceipt {
        let tracked_task_count_at_shutdown = self.active_task_count();
        self.close();
        let status = match timeout_duration {
            Some(duration) => match tokio::time::timeout(duration, self.wait()).await {
                Ok(()) => RuntimeTaskShutdownStatus::Completed,
                Err(_) => RuntimeTaskShutdownStatus::TimedOut,
            },
            None => {
                self.wait().await;
                RuntimeTaskShutdownStatus::Completed
            }
        };
        RuntimeTaskShutdownReceipt::from_input(RuntimeTaskShutdownReceiptInput {
            status,
            request,
            tracker_state: if self.is_closed() {
                RuntimeTaskTrackerShutdownState::Closed
            } else {
                RuntimeTaskTrackerShutdownState::Open
            },
            timeout_ms: duration_to_millis(timeout_duration),
            tracked_task_count_at_shutdown,
            remaining_task_count: self.active_task_count(),
        })
    }

    pub(crate) fn track_task(&self) -> RuntimeTaskGuard {
        self.state.active_tasks.fetch_add(1, Ordering::SeqCst);
        RuntimeTaskGuard {
            tracker: self.clone(),
        }
    }
}

impl Default for RuntimeTaskTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RuntimeTaskGuard {
    fn drop(&mut self) {
        let previous = self
            .tracker
            .state
            .active_tasks
            .fetch_sub(1, Ordering::SeqCst);
        if previous == 1 {
            self.tracker.state.notify.notify_waiters();
        }
    }
}

fn duration_to_millis(duration: Option<Duration>) -> Option<u64> {
    duration.map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
}
