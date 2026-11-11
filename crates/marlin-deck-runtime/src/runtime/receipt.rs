//! Owner observations published after the Context service and the VM retire.

/// Why the single VM owner stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeckRuntimeStopReason {
    Drained,
    SchemeCloseFailed,
    ProtocolFailed,
    OwnerPanicked,
}

/// Counters for dequeued requests and actual Scheme actor submissions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DeckRuntimeMetrics {
    /// Requests removed from the admission channel by this owner.
    pub requests_received: u64,
    /// Owner-qualified successful outcomes offered to their reply slots.
    pub requests_succeeded: u64,
    /// Requests retired after cancellation or a dropped reply receiver.
    pub requests_cancelled: u64,
    /// Requests retired after their admission deadline.
    pub requests_expired: u64,
    /// Other rejected requests, including invalid Context projections.
    pub requests_failed: u64,
    /// Largest ready request batch held by the owner before projection.
    pub largest_request_batch: usize,
    /// Context batch attempts, including fallible Scheme value construction.
    pub scheme_batches: u64,
    /// Context rows submitted to those batch attempts.
    pub scheme_items: u64,
    /// Largest attempted Scheme Context batch.
    pub largest_scheme_batch: usize,
}

/// Scheme-owned mailbox lifetimes, observed after draining and joining Actors.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DeckMailboxMetrics {
    pub actors_created: u64,
    pub actors_retired: u64,
    pub messages_submitted: u64,
    pub results_collected: u64,
}

/// Terminal observation after the Scheme runtime has been dropped on its owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeckRuntimeShutdownReceipt {
    pub reason: DeckRuntimeStopReason,
    pub vm_processors: u32,
    /// The Scheme Context service acknowledged retirement.
    pub context_service_closed: bool,
    pub mailbox: DeckMailboxMetrics,
    pub metrics: DeckRuntimeMetrics,
}
