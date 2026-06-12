//! Tokio task handle wrappers.

use tokio::task::JoinError;

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
