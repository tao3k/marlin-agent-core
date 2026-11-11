// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded model-event delivery with byte-accounted backpressure.

use std::{fmt, sync::Arc};

use marlin_agent_protocol::ModelGatewayError;
use marlin_agent_stream::{MODEL_STREAM_MAX_CHUNK_BYTES, ModelStreamEvent};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};

pub(crate) type Reply = Result<ModelStreamEvent, ReplyError>;

#[derive(Debug)]
pub(crate) enum ReplyError {
    Gateway(ModelGatewayError),
    QueueLimit {
        limit_bytes: usize,
        actual_bytes: usize,
    },
}

impl fmt::Display for ReplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gateway(error) => error.fmt(formatter),
            Self::QueueLimit {
                limit_bytes,
                actual_bytes,
            } => write!(
                formatter,
                "Model event exceeded the UI queue byte limit ({actual_bytes} bytes; limit {limit_bytes})."
            ),
        }
    }
}

pub(crate) struct QueuedReply {
    pub(crate) id: u64,
    pub(crate) reply: Reply,
    _bytes: OwnedSemaphorePermit,
}

impl QueuedReply {
    pub(crate) fn into_parts(self) -> (u64, Reply) {
        (self.id, self.reply)
    }
}

#[derive(Clone)]
pub(crate) struct ReplySender {
    sender: mpsc::Sender<QueuedReply>,
    bytes: Arc<Semaphore>,
    max_bytes: u32,
}

pub(crate) fn reply_channel(
    max_items: usize,
    max_bytes: u32,
) -> (ReplySender, mpsc::Receiver<QueuedReply>) {
    assert!(max_items > 0 && max_bytes >= 128);
    let (sender, receiver) = mpsc::channel(max_items);
    (
        ReplySender {
            sender,
            bytes: Arc::new(Semaphore::new(max_bytes as usize)),
            max_bytes,
        },
        receiver,
    )
}

impl ReplySender {
    /// Wait for byte and item capacity. The permit follows the queued event.
    pub(crate) async fn send(
        &self,
        id: u64,
        reply: Result<ModelStreamEvent, ModelGatewayError>,
    ) -> Result<bool, ()> {
        let mut reply = reply.map_err(ReplyError::Gateway);
        let mut bytes = reply_bytes(&reply);
        let chunk_is_bounded = match &reply {
            Ok(ModelStreamEvent::Chunk(chunk)) => {
                chunk.content.capacity() <= MODEL_STREAM_MAX_CHUNK_BYTES
            }
            Ok(ModelStreamEvent::Thinking(chunk)) => {
                chunk.content.capacity() <= MODEL_STREAM_MAX_CHUNK_BYTES
            }
            _ => true,
        };
        let accepted = bytes <= self.max_bytes as usize && chunk_is_bounded;
        if !accepted {
            // Report a failed turn instead of silently dropping or admitting
            // an event larger than the entire queue's byte budget.
            reply = Err(ReplyError::QueueLimit {
                limit_bytes: if chunk_is_bounded {
                    self.max_bytes as usize
                } else {
                    MODEL_STREAM_MAX_CHUNK_BYTES.min(self.max_bytes as usize)
                },
                actual_bytes: bytes,
            });
            bytes = reply_bytes(&reply);
        }
        if bytes > self.max_bytes as usize {
            return Err(());
        }
        let permit = self
            .bytes
            .clone()
            .acquire_many_owned(bytes as u32)
            .await
            .map_err(|_| ())?;
        self.sender
            .send(QueuedReply {
                id,
                reply,
                _bytes: permit,
            })
            .await
            .map_err(|_| ())?;
        Ok(accepted)
    }
}

fn reply_bytes(reply: &Reply) -> usize {
    match reply {
        Err(ReplyError::Gateway(error)) => error.heap_capacity(),
        Err(ReplyError::QueueLimit { .. }) => 0,
        Ok(ModelStreamEvent::Started { .. }) => 0,
        Ok(ModelStreamEvent::Chunk(chunk)) => chunk.content.capacity(),
        Ok(ModelStreamEvent::Thinking(chunk)) => chunk.content.capacity(),
        Ok(ModelStreamEvent::Completed { response }) => response.heap_capacity(),
    }
}
