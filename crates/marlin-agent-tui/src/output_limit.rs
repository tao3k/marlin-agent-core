// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Per-turn output accounting at the TUI stream-consumption boundary.

use marlin_agent_protocol::{
    ModelGatewayCompletionResponse, ModelGatewayError, ModelGatewayMessageRole,
    ModelStreamInterruption,
};
use marlin_agent_stream::ModelStreamEvent;

pub(crate) const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_COMPLETION_HEAP_BYTES: usize = 32 * 1024 * 1024;

pub(crate) fn completion_heap_limit(
    response: &ModelGatewayCompletionResponse,
) -> Option<ModelStreamInterruption> {
    let actual_bytes = response.heap_capacity();
    (actual_bytes > MAX_COMPLETION_HEAP_BYTES).then_some(
        ModelStreamInterruption::CompletionHeapLimit {
            limit_bytes: MAX_COMPLETION_HEAP_BYTES,
            actual_bytes,
        },
    )
}

pub(crate) struct OutputLimit {
    accepted_bytes: usize,
    saw_answer: bool,
    max_bytes: usize,
}

impl OutputLimit {
    pub(crate) fn new(max_bytes: usize) -> Self {
        Self {
            accepted_bytes: 0,
            saw_answer: false,
            max_bytes,
        }
    }

    /// Reject an entire event before it enters the UI queue if its text would
    /// exceed the limit. Earlier chunks remain visible as provisional output.
    pub(crate) fn check(&mut self, event: &ModelStreamEvent) -> Result<(), ModelGatewayError> {
        if let ModelStreamEvent::Completed { response } = event
            && let Some(reason) = completion_heap_limit(response)
        {
            return Err(ModelGatewayError::StreamInterrupted(reason));
        }
        let too_large = match event {
            ModelStreamEvent::Started { .. } => false,
            ModelStreamEvent::Chunk(chunk) => {
                match self.accepted_bytes.checked_add(chunk.content.len()) {
                    Some(total) if total <= self.max_bytes => {
                        self.accepted_bytes = total;
                        self.saw_answer |= !chunk.content.is_empty();
                        false
                    }
                    _ => true,
                }
            }
            ModelStreamEvent::Thinking(chunk) => {
                match self.accepted_bytes.checked_add(chunk.content.len()) {
                    Some(total) if total <= self.max_bytes => {
                        self.accepted_bytes = total;
                        false
                    }
                    _ => true,
                }
            }
            ModelStreamEvent::Completed { response } => response
                .choices
                .iter()
                .filter(|choice| choice.message.role == ModelGatewayMessageRole::Assistant)
                .any(|choice| {
                    if self.saw_answer {
                        choice.message.content.len() > self.max_bytes
                    } else {
                        self.accepted_bytes
                            .checked_add(choice.message.content.len())
                            .is_none_or(|total| total > self.max_bytes)
                    }
                }),
        };
        if too_large {
            Err(ModelGatewayError::StreamInterrupted(
                ModelStreamInterruption::OutputLimit {
                    limit_bytes: self.max_bytes,
                },
            ))
        } else {
            Ok(())
        }
    }
}
