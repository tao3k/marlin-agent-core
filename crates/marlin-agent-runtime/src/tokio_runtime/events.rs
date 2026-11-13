// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runtime event sink and stream ownership.

use std::{
    fmt,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use tokio::sync::mpsc;

use super::RuntimeEvent;

/// Maximum heap retained by one queued runtime observation.
pub const MAX_RUNTIME_EVENT_HEAP_BYTES: usize = 64 * 1024;

/// The event remains owned by the caller when admission fails.
#[derive(Debug)]
pub enum RuntimeEventSendError {
    Closed(RuntimeEvent),
    Oversized {
        event: RuntimeEvent,
        actual_bytes: usize,
        limit_bytes: usize,
    },
}

impl fmt::Display for RuntimeEventSendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed(_) => formatter.write_str("runtime event stream closed"),
            Self::Oversized {
                actual_bytes,
                limit_bytes,
                ..
            } => write!(
                formatter,
                "runtime event retains {actual_bytes} bytes; limit is {limit_bytes} bytes"
            ),
        }
    }
}

impl std::error::Error for RuntimeEventSendError {}

/// Cloneable Tokio mpsc sender for runtime observations and receipts.
#[derive(Clone, Debug)]
pub struct RuntimeEventSink {
    senders: Arc<Vec<mpsc::Sender<RuntimeEvent>>>,
}

impl RuntimeEventSink {
    pub fn channel(event_buffer: usize) -> (Self, RuntimeEventStream) {
        let (sender, receiver) = mpsc::channel(event_buffer);
        (
            Self {
                senders: Arc::new(vec![sender]),
            },
            RuntimeEventStream::new(receiver),
        )
    }

    pub fn with_capture(&self, event_buffer: usize) -> (Self, RuntimeEventStream) {
        let (sender, receiver) = mpsc::channel(event_buffer);
        let mut senders = self.senders.as_ref().clone();
        senders.push(sender);
        (
            Self {
                senders: Arc::new(senders),
            },
            RuntimeEventStream::new(receiver),
        )
    }

    pub async fn emit(&self, event: RuntimeEvent) -> Result<(), RuntimeEventSendError> {
        let actual_bytes = event
            .topic
            .capacity()
            .saturating_add(event.message.capacity());
        if actual_bytes > MAX_RUNTIME_EVENT_HEAP_BYTES {
            return Err(RuntimeEventSendError::Oversized {
                event,
                actual_bytes,
                limit_bytes: MAX_RUNTIME_EVENT_HEAP_BYTES,
            });
        }
        let Some((primary, captures)) = self.senders.split_first() else {
            return Err(RuntimeEventSendError::Closed(event));
        };
        if captures.is_empty() {
            return primary
                .send(event)
                .await
                .map_err(|error| RuntimeEventSendError::Closed(error.0));
        }

        // Reserve every live destination before publishing to any of them.
        // Dropping a pending emit releases its reservations without exposing
        // an event to only a prefix of the subscribers.
        let primary_permit = primary.reserve().await.ok();
        let primary_closed = primary_permit.is_none();
        // One capture is the normal graph-evidence case; keep its permit on
        // the stack so the fanout hot path does not allocate per event.
        let first_capture_permit = captures[0].reserve().await.ok();
        let mut capture_permits = Vec::with_capacity(captures.len() - 1);
        for capture in &captures[1..] {
            if let Ok(permit) = capture.reserve().await {
                capture_permits.push(permit);
            }
        }
        if let Some(permit) = primary_permit {
            permit.send(event.clone());
        }
        if let Some(permit) = first_capture_permit {
            permit.send(event.clone());
        }
        for permit in capture_permits {
            permit.send(event.clone());
        }
        if primary_closed {
            Err(RuntimeEventSendError::Closed(event))
        } else {
            Ok(())
        }
    }
}

/// Tokio stream of runtime observations and receipts.
#[derive(Debug)]
pub struct RuntimeEventStream {
    receiver: mpsc::Receiver<RuntimeEvent>,
}

impl RuntimeEventStream {
    /// Wrap a Tokio event receiver in the Marlin runtime event stream boundary.
    pub fn new(receiver: mpsc::Receiver<RuntimeEvent>) -> Self {
        Self { receiver }
    }

    /// Return one already-buffered runtime event without waiting.
    pub fn try_next(&mut self) -> Option<RuntimeEvent> {
        self.receiver.try_recv().ok()
    }

    /// Wait for the next event, returning `None` after the stream closes.
    pub async fn recv(&mut self) -> Option<RuntimeEvent> {
        self.receiver.recv().await
    }

    /// Close the stream so no further runtime events can be received.
    pub fn close(&mut self) {
        self.receiver.close();
    }
}

impl tokio_stream::Stream for RuntimeEventStream {
    type Item = RuntimeEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

/// Compatibility alias for the runtime event stream boundary.
pub type EventStream = RuntimeEventStream;
