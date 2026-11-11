//! An activated Module owns its Actor and timer lifetimes inside the shared VM.
use super::*;
use std::num::NonZeroU64;
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeckModuleInstanceId(pub(super) NonZeroU64);
impl DeckModuleInstanceId {
    pub fn get(self) -> u64 {
        self.0.get()
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeckTimerId(pub(super) NonZeroU64);
impl DeckTimerId {
    pub fn get(self) -> u64 {
        self.0.get()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeckTimerReceipt {
    pub module: DeckModuleInstanceId,
    pub timer: DeckTimerId,
    pub due_monotonic_us: u64,
    pub observed_monotonic_us: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeckModuleRetirementReceipt {
    pub module: DeckModuleInstanceId,
    pub mailbox: DeckMailboxMetrics,
    pub timers_retired: u64,
}
pub(super) struct ModuleState {
    pub id: DeckModuleInstanceId,
    // Sending and closing admission share this short lock; it is never held
    // across await, VM entry or reply publication. Earlier jobs are already queued.
    pub closed: Mutex<bool>,
    pub finished: watch::Sender<Option<DeckModuleRetirementReceipt>>,
}
impl ModuleState {
    pub(super) fn new(id: DeckModuleInstanceId) -> Arc<Self> {
        Arc::new(Self {
            id,
            closed: Mutex::new(false),
            finished: watch::channel(None).0,
        })
    }
}
/// Retire explicitly; dropping this handle abandons no native work. Runtime
/// shutdown remains the fallback owner of every activated Module.
#[derive(Clone)]
pub struct DeckContextModule {
    pub(super) runtime: DeckRuntime,
    pub(super) state: Arc<ModuleState>,
}
impl DeckContextModule {
    pub fn id(&self) -> DeckModuleInstanceId {
        self.state.id
    }
    pub async fn admit_context(
        &self,
        request: ContextAdmissionRequest,
        budget: Duration,
        cancellation: CancellationToken,
    ) -> Outcome {
        self.runtime
            .admit_in(&self.state, request, budget, cancellation)
            .await
    }
    /// A Module-owned observation. Dropping the waiting future does not cancel
    /// its native timer. Retirement joins it and closes its reply slot.
    pub async fn timer(&self, delay: Duration) -> Result<DeckTimerReceipt, DeckRuntimeRejection> {
        let microseconds = delay.as_nanos().div_ceil(1000);
        let delay_us = u64::try_from(microseconds)
            .ok()
            .filter(|delay| *delay > 0)
            .ok_or(DeckRuntimeRejection::TimerDelay)?;
        let (reply, result) = oneshot::channel();
        self.runtime.offer(
            &self.state,
            OwnerMessage::Timer {
                module: self.id(),
                delay_us,
                reply,
            },
        )?;
        result
            .await
            .map_err(|_| DeckRuntimeRejection::Unavailable)?
    }
    pub async fn retire(&self) -> Result<DeckModuleRetirementReceipt, DeckRuntimeRejection> {
        {
            let mut closed = self
                .state
                .closed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if !*closed {
                if self.runtime.stopping.load(Ordering::Acquire) {
                    return Err(DeckRuntimeRejection::Unavailable);
                }
                self.runtime
                    .queue
                    .try_send(OwnerMessage::Retire(Arc::clone(&self.state)))
                    .map_err(queue_error)?;
                *closed = true;
                self.runtime
                    .wake
                    .notify()
                    .map_err(|_| DeckRuntimeRejection::Unavailable)?;
            }
        }
        let mut finished = self.state.finished.subscribe();
        let mut stopped = self.runtime.stopped.clone();
        loop {
            if let Some(receipt) = *finished.borrow_and_update() {
                return Ok(receipt);
            }
            tokio::select! {
                changed = finished.changed() => { changed.map_err(|_| DeckRuntimeRejection::Unavailable)?; }
                _ = stopped.changed() => return Err(DeckRuntimeRejection::Unavailable),
            }
        }
    }
}
pub(super) fn queue_error<T>(error: mpsc::error::TrySendError<T>) -> DeckRuntimeRejection {
    match error {
        mpsc::error::TrySendError::Full(_) => DeckRuntimeRejection::Backpressure,
        mpsc::error::TrySendError::Closed(_) => DeckRuntimeRejection::Unavailable,
    }
}
