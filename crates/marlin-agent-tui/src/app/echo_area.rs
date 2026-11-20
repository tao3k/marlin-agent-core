// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Store transient feedback for the Scheme-selected echo area.

use crate::ui_host::KeySequenceState;
use marlin_agent_ui::UiEchoPolicy;
use std::time::{Duration, Instant};

#[cfg(test)]
#[path = "../../tests/unit/app_cases/echo_area.rs"]
mod tests;

pub(crate) struct KeyEcho {
    pub(crate) text: String,
    pub(crate) state: KeySequenceState,
    until: Instant,
}

#[derive(Default)]
pub(crate) struct EchoArea {
    pub(crate) keys: Option<KeyEcho>,
    message: Option<(String, Instant)>,
    timeout: Duration,
    clear_on_input: bool,
}

impl EchoArea {
    pub(super) fn configure(&mut self, policy: &UiEchoPolicy) {
        self.timeout = policy.timeout;
        self.clear_on_input = policy.clear_on_input;
    }

    pub(crate) fn message(&self) -> Option<&str> {
        self.message.as_ref().map(|(text, _)| text.as_str())
    }

    pub(crate) fn set_message(&mut self, text: String, now: Instant) {
        self.message = Some((text, now + self.timeout));
    }

    pub(super) fn on_input(&mut self) {
        if self.clear_on_input {
            self.message = None;
            if self
                .keys
                .as_ref()
                .is_some_and(|keys| keys.state != KeySequenceState::Pending)
            {
                self.keys = None;
            }
        }
    }

    pub(super) fn set_keys(&mut self, keys: &[String], state: KeySequenceState, now: Instant) {
        self.keys = (!keys.is_empty()).then(|| KeyEcho {
            text: keys.join(" "),
            state,
            until: now + self.timeout,
        });
    }

    pub(super) fn expire(&mut self, now: Instant) -> bool {
        let message = self
            .message
            .as_ref()
            .is_some_and(|(_, until)| now >= *until);
        let keys = self
            .keys
            .as_ref()
            .is_some_and(|keys| keys.state != KeySequenceState::Pending && now >= keys.until);
        if message {
            self.message = None;
        }
        if keys {
            self.keys = None;
        }
        message || keys
    }
}
