// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Admission budget for complete model context; no history is truncated.

use std::mem::size_of;

use marlin_agent_protocol::ModelGatewayMessage;

const MAX_CONTEXT_BYTES: usize = 64 * 1024 * 1024;
const MAX_CONTEXT_TURNS: usize = 10_000;

#[derive(Clone, Copy)]
pub(crate) struct ContextBudget {
    pub(crate) bytes: usize,
    pub(crate) turns: usize,
}

pub(crate) struct ContextUsage {
    pub(crate) completed_turns: usize,
    pub(crate) request_bytes: usize,
    pub(crate) allowed: bool,
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            bytes: MAX_CONTEXT_BYTES,
            turns: MAX_CONTEXT_TURNS,
        }
    }
}

impl ContextBudget {
    /// Check the existing complete exchanges plus the next prompt before
    /// cloning history or dispatching a provider request.
    pub(crate) fn allows(self, history: &[ModelGatewayMessage], prompt: &str) -> bool {
        if history.len() / 2 >= self.turns {
            return false;
        }
        self.measure(history, prompt)
            .is_some_and(|usage| usage.allowed)
    }

    /// Account exactly the values used by admission, without cloning the context.
    pub(crate) fn measure(
        self,
        history: &[ModelGatewayMessage],
        prompt: &str,
    ) -> Option<ContextUsage> {
        let initial = size_of::<ModelGatewayMessage>().checked_add(prompt.len())?;
        let total = history.iter().try_fold(initial, |sum, message| {
            sum.checked_add(size_of::<ModelGatewayMessage>())?
                .checked_add(message.content.len())?
                .checked_add(message.name.as_ref().map_or(0, String::len))?
                .checked_add(
                    message
                        .continuation
                        .as_ref()
                        .map_or(0, |state| state.accounted_bytes()),
                )
        });
        let request_bytes = total?;
        let completed_turns = history.len() / 2;
        Some(ContextUsage {
            completed_turns,
            request_bytes,
            allowed: completed_turns < self.turns && request_bytes <= self.bytes,
        })
    }
}
