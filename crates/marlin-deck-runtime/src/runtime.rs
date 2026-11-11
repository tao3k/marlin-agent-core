//! Bounded Tokio admission into one Scheme VM.

mod config;
mod module;
mod owner;
mod wake;
pub use module::{
    DeckContextModule, DeckModuleInstanceId, DeckModuleRetirementReceipt, DeckTimerId,
    DeckTimerReceipt,
};
use module::{ModuleState, queue_error};
#[cfg(feature = "context-profiling")]
mod profile;
mod receipt;
mod vm;

use gerbil_scheme::NativeError as SchemeVmError;
use marlin_agent_context::gerbil::ContextBridgeError;
use marlin_agent_context::{CONTEXT_BATCH_MAX_ROWS, ContextAdmissionRequest, ContextScopeDigest};

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot, watch},
    time::Instant,
};
use tokio_util::sync::CancellationToken;

pub use config::DeckRuntimeConfig;
pub use receipt::{
    DeckMailboxMetrics, DeckRuntimeMetrics, DeckRuntimeShutdownReceipt, DeckRuntimeStopReason,
};

/// Invalid limits, or failure while establishing the unique VM owner.
#[derive(Debug, thiserror::Error)]
pub enum DeckRuntimeStartupError {
    #[error("Deck batch capacity must be between 1 and {CONTEXT_BATCH_MAX_ROWS}")]
    BatchCapacity,
    #[error("cannot start Deck VM owner: {0}")]
    Owner(#[from] std::io::Error),
    #[error("Deck VM startup failed: {0}")]
    Scheme(#[from] SchemeVmError),
    #[error("Deck VM owner exited during startup")]
    OwnerExited,
}

/// A terminal rejection without a Context scope admission.
#[derive(Clone, Debug, thiserror::Error)]
pub enum DeckRuntimeRejection {
    #[error("Deck Context admission queue is full")]
    Backpressure,
    #[error("Deck Module has retired")]
    ModuleRetired,
    #[error("Deck timer delay must be positive and fit u64 microseconds")]
    TimerDelay,
    #[error("Deck Context admission was cancelled")]
    Cancelled,
    #[error("Deck Context admission exceeded its latency budget")]
    Deadline,
    #[error("Deck VM owner is unavailable")]
    Unavailable,
    #[error("Deck Scheme call failed: {0}")]
    Scheme(#[from] SchemeVmError),
    #[error("Deck returned an invalid typed projection")]
    InvalidProjection,
    #[error("The Context Actor rejected this message")]
    ContextActor,
    #[error(transparent)]
    Context(#[from] ContextBridgeError),
}

/// The owner could not publish or join its terminal state.
#[derive(Debug, thiserror::Error)]
#[error("Deck VM owner exited without a shutdown receipt")]
pub struct DeckRuntimeShutdownError;

type Outcome = Result<ContextScopeDigest, DeckRuntimeRejection>;

struct Job {
    module: DeckModuleInstanceId,
    #[cfg(feature = "context-profiling")]
    enqueued_at: std::time::Instant,
    request: ContextAdmissionRequest,
    cancellation: CancellationToken,
    deadline: Instant,
    reply: oneshot::Sender<Outcome>,
}

impl Job {
    fn check_active(&self) -> Result<(), DeckRuntimeRejection> {
        if self.cancellation.is_cancelled() {
            Err(DeckRuntimeRejection::Cancelled)
        } else if Instant::now() >= self.deadline {
            Err(DeckRuntimeRejection::Deadline)
        } else if self.reply.is_closed() {
            Err(DeckRuntimeRejection::Cancelled)
        } else {
            Ok(())
        }
    }
}

enum OwnerMessage {
    Admit(Job),
    Activate(oneshot::Sender<Result<Arc<ModuleState>, DeckRuntimeRejection>>),
    Retire(Arc<ModuleState>),
    Timer {
        module: DeckModuleInstanceId,
        delay_us: u64,
        reply: oneshot::Sender<Result<DeckTimerReceipt, DeckRuntimeRejection>>,
    },
    Stop,
}

/// Clones share one VM, one bounded admission channel.
#[derive(Clone)]
pub struct DeckRuntime {
    queue: mpsc::Sender<OwnerMessage>,
    wake: Arc<wake::Wake>,
    default_module: Arc<ModuleState>,
    stopping: Arc<AtomicBool>,
    stopped: watch::Receiver<Option<DeckRuntimeShutdownReceipt>>,
    owner: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
}

impl DeckRuntime {
    /// Start the single-processor VM and the Scheme Context batch service.
    pub fn start(config: DeckRuntimeConfig) -> Result<Self, DeckRuntimeStartupError> {
        if usize::from(config.batch_capacity.get()) > CONTEXT_BATCH_MAX_ROWS {
            return Err(DeckRuntimeStartupError::BatchCapacity);
        }
        let wake = Arc::new(wake::Wake::new()?);
        let worker_wake = Arc::clone(&wake);
        let default_module = ModuleState::new(DeckModuleInstanceId(std::num::NonZeroU64::MIN));
        let (queue, jobs) = mpsc::channel(usize::from(config.queue_capacity.get()));
        let (ready, started) = std::sync::mpsc::sync_channel(1);
        let (finished, stopped) = watch::channel(None);
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stopping = Arc::clone(&stopping);
        let thread = std::thread::Builder::new()
            .name("marlin-deck-vm".into())
            .spawn(move || {
                owner::run(config, jobs, worker_stopping, worker_wake, ready, finished);
            })?;
        match started.recv() {
            Ok(Ok(())) => Ok(Self {
                queue,
                wake,
                default_module,
                stopping,
                stopped,
                owner: Arc::new(Mutex::new(Some(thread))),
            }),
            result => {
                let _ = thread.join();
                match result {
                    Ok(Err(error)) => Err(error.into()),
                    _ => Err(DeckRuntimeStartupError::OwnerExited),
                }
            }
        }
    }

    /// Reserve one reply slot with each admitted request. Ready work never waits
    /// for a fuller batch. Cancellation is checked at each VM boundary.
    pub async fn admit_context(
        &self,
        request: ContextAdmissionRequest,
        budget: Duration,
        cancellation: CancellationToken,
    ) -> Outcome {
        self.admit_in(&self.default_module, request, budget, cancellation)
            .await
    }

    pub fn context_module(&self) -> DeckContextModule {
        DeckContextModule {
            runtime: self.clone(),
            state: Arc::clone(&self.default_module),
        }
    }
    pub async fn activate_context_module(&self) -> Result<DeckContextModule, DeckRuntimeRejection> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(DeckRuntimeRejection::Unavailable);
        }
        let (reply, result) = oneshot::channel();
        self.queue
            .try_send(OwnerMessage::Activate(reply))
            .map_err(queue_error)?;
        self.wake
            .notify()
            .map_err(|_| DeckRuntimeRejection::Unavailable)?;
        let state = result
            .await
            .map_err(|_| DeckRuntimeRejection::Unavailable)??;
        Ok(DeckContextModule {
            runtime: self.clone(),
            state,
        })
    }
    fn offer(
        &self,
        module: &ModuleState,
        message: OwnerMessage,
    ) -> Result<(), DeckRuntimeRejection> {
        let closed = module
            .closed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if *closed {
            return Err(DeckRuntimeRejection::ModuleRetired);
        }
        if self.stopping.load(Ordering::Acquire) {
            return Err(DeckRuntimeRejection::Unavailable);
        }
        self.queue.try_send(message).map_err(queue_error)?;
        self.wake
            .notify()
            .map_err(|_| DeckRuntimeRejection::Unavailable)
    }
    async fn admit_in(
        &self,
        module: &ModuleState,
        request: ContextAdmissionRequest,
        budget: Duration,
        cancellation: CancellationToken,
    ) -> Outcome {
        if self.stopping.load(Ordering::Acquire) {
            return Err(DeckRuntimeRejection::Unavailable);
        }
        let deadline = Instant::now()
            .checked_add(budget)
            .ok_or(DeckRuntimeRejection::Deadline)?;
        let (reply, result) = oneshot::channel();
        let job = Job {
            module: module.id,
            #[cfg(feature = "context-profiling")]
            enqueued_at: std::time::Instant::now(),
            request,
            cancellation: cancellation.clone(),
            deadline,
            reply,
        };
        job.check_active()?;
        self.offer(module, OwnerMessage::Admit(job))?;
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(DeckRuntimeRejection::Cancelled),
            () = tokio::time::sleep_until(deadline) => Err(DeckRuntimeRejection::Deadline),
            receipt = result => receipt.map_err(|_| DeckRuntimeRejection::Unavailable)?,
        }
    }

    /// Close admission for all clones, drain accepted work, retire the actors,
    /// drop the VM on its owner, and join the owner thread.
    pub async fn shutdown(&self) -> Result<DeckRuntimeShutdownReceipt, DeckRuntimeShutdownError> {
        if !self.stopping.swap(true, Ordering::AcqRel) {
            // A full queue already wakes the owner. An idle owner needs Stop.
            let _ = self.queue.try_send(OwnerMessage::Stop);
            let _ = self.wake.notify();
        }
        let mut stopped = self.stopped.clone();
        let receipt = loop {
            if let Some(receipt) = *stopped.borrow_and_update() {
                break receipt;
            }
            stopped
                .changed()
                .await
                .map_err(|_| DeckRuntimeShutdownError)?;
        };
        let owner = self
            .owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(owner) = owner {
            tokio::task::spawn_blocking(move || owner.join())
                .await
                .map_err(|_| DeckRuntimeShutdownError)?
                .map_err(|_| DeckRuntimeShutdownError)?;
        }
        Ok(receipt)
    }
}

#[cfg(test)]
#[path = "../tests/unit/runtime_queue.rs"]
mod queue_tests;

impl Drop for DeckRuntime {
    fn drop(&mut self) {
        if self.queue.strong_count() == 1 {
            self.stopping.store(true, Ordering::Release);
            let _ = self.wake.notify();
        }
    }
}
