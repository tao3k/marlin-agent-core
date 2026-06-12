//! Deterministic `chunk` gating primitives for stream tests.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::Semaphore;

/// Deterministic test gate for scripted stream chunk delivery.
#[derive(Clone, Debug)]
pub struct ChunkGate {
    admitted_chunks: Arc<AtomicU64>,
    permits: Arc<Semaphore>,
}

impl Default for ChunkGate {
    fn default() -> Self {
        Self {
            admitted_chunks: Arc::new(AtomicU64::new(0)),
            permits: Arc::new(Semaphore::new(0)),
        }
    }
}

impl ChunkGate {
    /// Creates a gate with no chunk permits.
    pub fn closed() -> Self {
        Self::default()
    }

    /// Releases one pending chunk.
    pub fn release_next(&self) {
        self.permits.add_permits(1);
    }

    /// Releases several pending chunks.
    pub fn release_many(&self, permits: usize) {
        self.permits.add_permits(permits);
    }

    /// Waits until this gate admits the next chunk.
    pub async fn wait_for_next(&self) -> ChunkGatePermit {
        let permit = self
            .permits
            .acquire()
            .await
            .expect("chunk gate semaphore is not externally closed");
        permit.forget();
        let sequence = self.admitted_chunks.fetch_add(1, Ordering::SeqCst) + 1;
        ChunkGatePermit { sequence }
    }

    /// Returns how many chunks have passed through this gate.
    pub fn admitted_chunks(&self) -> u64 {
        self.admitted_chunks.load(Ordering::SeqCst)
    }
}

/// Receipt returned when a chunk is admitted by a gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkGatePermit {
    sequence: u64,
}

impl ChunkGatePermit {
    /// Sequence number of the admitted chunk, starting at one.
    pub fn sequence(self) -> u64 {
        self.sequence
    }
}
