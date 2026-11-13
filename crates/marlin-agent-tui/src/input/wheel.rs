// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded trackpad batching with coordinates retained for hit testing.

use std::{collections::VecDeque, sync::Mutex};

/// Collapse high-rate trackpad pulses before they enter the bounded key queue.
#[derive(Default)]
pub(crate) struct WheelAccumulator(Mutex<VecDeque<WheelBatch>>);

#[derive(Clone, Copy)]
struct WheelBatch {
    column: u16,
    row: u16,
    steps: i32,
}

const MAX_WHEEL_BATCHES: usize = 64;

impl WheelAccumulator {
    /// A true result means the event loop needs one wake notification.
    pub(crate) fn push(&self, up: bool, column: u16, row: u16) -> bool {
        let step = if up { 1 } else { -1 };
        let mut batches = self.0.lock().expect("wheel accumulator poisoned");
        let wake = batches.is_empty();
        if let Some(last) = batches.back_mut()
            && last.column == column
            && last.row == row
        {
            last.steps = last.steps.saturating_add(step);
            if last.steps == 0 {
                batches.pop_back();
            }
        } else if batches.len() < MAX_WHEEL_BATCHES {
            batches.push_back(WheelBatch {
                column,
                row,
                steps: step,
            });
        }
        wake && !batches.is_empty()
    }

    /// Bound one frame's scroll while retaining direction and small gestures.
    pub(crate) fn take_lines(&self) -> Option<(bool, usize, u16, u16)> {
        let batch = self
            .0
            .lock()
            .expect("wheel accumulator poisoned")
            .pop_front()?;
        Some((
            batch.steps > 0,
            batch.steps.unsigned_abs().min(16) as usize * 3,
            batch.column,
            batch.row,
        ))
    }

    pub(crate) fn has_pending(&self) -> bool {
        !self
            .0
            .lock()
            .expect("wheel accumulator poisoned")
            .is_empty()
    }
}
