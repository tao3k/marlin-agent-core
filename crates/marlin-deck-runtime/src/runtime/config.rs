//! Bounds for the single VM and each cooperative actor turn.

use std::num::NonZeroU16;

/// Admission and execution limits for one owner-local VM.
#[derive(Clone, Copy, Debug)]
pub struct DeckRuntimeConfig {
    pub(crate) queue_capacity: NonZeroU16,
    pub(crate) actors: NonZeroU16,
    pub(crate) batch_capacity: NonZeroU16,
    pub(crate) actor_quantum: u16,
}

impl DeckRuntimeConfig {
    /// Use bounded ready-work batching and one yield opportunity per actor item.
    pub fn new(queue_capacity: NonZeroU16, actors: NonZeroU16) -> Self {
        Self {
            queue_capacity,
            actors,
            batch_capacity: NonZeroU16::new(64).unwrap(),
            actor_quantum: 1,
        }
    }

    /// Limit both requests held by the owner and Context rows per VM entry.
    pub fn with_batch_capacity(mut self, capacity: NonZeroU16) -> Self {
        self.batch_capacity = capacity;
        self
    }

    /// Yield after this many items per Scheme actor. Zero disables explicit yields.
    pub fn with_actor_quantum(mut self, quantum: u16) -> Self {
        self.actor_quantum = quantum;
        self
    }
}
