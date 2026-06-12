//! Runtime event sink and stream ownership.

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use tokio::sync::mpsc;

use super::RuntimeEvent;

/// Cloneable Tokio mpsc sender for runtime observations and receipts.
#[derive(Clone, Debug)]
pub struct RuntimeEventSink {
    sender: mpsc::Sender<RuntimeEvent>,
}

impl RuntimeEventSink {
    pub fn channel(event_buffer: usize) -> (Self, RuntimeEventStream) {
        let (sender, receiver) = mpsc::channel(event_buffer);
        (Self { sender }, RuntimeEventStream::new(receiver))
    }

    pub async fn emit(
        &self,
        event: RuntimeEvent,
    ) -> Result<(), mpsc::error::SendError<RuntimeEvent>> {
        self.sender.send(event).await
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
