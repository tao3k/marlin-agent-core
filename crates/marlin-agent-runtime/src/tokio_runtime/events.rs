//! Runtime event sink and stream ownership.

use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use tokio::sync::mpsc;

use super::RuntimeEvent;

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

    pub async fn emit(
        &self,
        event: RuntimeEvent,
    ) -> Result<(), mpsc::error::SendError<RuntimeEvent>> {
        let Some((primary, captures)) = self.senders.split_first() else {
            return Err(mpsc::error::SendError(event));
        };

        primary.send(event.clone()).await?;
        for capture in captures {
            let _ = capture.send(event.clone()).await;
        }
        Ok(())
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
