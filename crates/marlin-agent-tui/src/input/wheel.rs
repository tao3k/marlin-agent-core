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
const MAX_PENDING_STEPS: i32 = 256;
const MAX_FRAME_STEPS: u32 = 16;

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
            last.steps = last
                .steps
                .saturating_add(step)
                .clamp(-MAX_PENDING_STEPS, MAX_PENDING_STEPS);
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

    /// Bound one event-loop turn's pulse count while retaining direction and coordinates.
    pub(crate) fn take_steps(&self) -> Option<(bool, usize, u16, u16)> {
        let mut batches = self.0.lock().expect("wheel accumulator poisoned");
        let batch = batches.front_mut()?;
        let up = batch.steps > 0;
        let count = batch.steps.unsigned_abs().min(MAX_FRAME_STEPS);
        batch.steps -= if up { count as i32 } else { -(count as i32) };
        let (column, row) = (batch.column, batch.row);
        if batch.steps == 0 {
            batches.pop_front();
        }
        Some((up, count as usize, column, row))
    }

    pub(crate) fn has_pending(&self) -> bool {
        !self
            .0
            .lock()
            .expect("wheel accumulator poisoned")
            .is_empty()
    }
}
