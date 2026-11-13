// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Coalesces visible changes while keeping idle redraws disabled.

use std::time::{Duration, Instant};

const FRAME_INTERVAL: Duration = Duration::from_millis(16);
const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(crate) struct FrameSchedule {
    dirty: bool,
    last_draw: Option<Instant>,
}

impl FrameSchedule {
    pub(crate) fn new() -> Self {
        Self {
            dirty: true,
            last_draw: None,
        }
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn should_draw(&self, now: Instant) -> bool {
        self.dirty
            && self
                .last_draw
                .is_none_or(|last| now.saturating_duration_since(last) >= FRAME_INTERVAL)
    }

    pub(crate) fn drawn(&mut self, now: Instant) {
        self.dirty = false;
        self.last_draw = Some(now);
    }

    pub(crate) fn wait_duration(&self, now: Instant) -> Duration {
        if !self.dirty {
            return INPUT_POLL_INTERVAL;
        }
        self.last_draw.map_or(Duration::ZERO, |last| {
            FRAME_INTERVAL
                .saturating_sub(now.saturating_duration_since(last))
                .min(INPUT_POLL_INTERVAL)
        })
    }
}
