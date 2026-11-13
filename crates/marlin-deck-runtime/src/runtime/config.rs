// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounds for Context admission through the single VM.

use std::num::NonZeroU16;

/// Rust admission and C ABI transfer limits for Context requests.
#[derive(Clone, Copy, Debug)]
pub struct DeckRuntimeConfig {
    pub(crate) queue_capacity: NonZeroU16,
    pub(crate) batch_capacity: NonZeroU16,
}

impl DeckRuntimeConfig {
    /// Configure the bounded Rust admission queue.
    pub fn new(queue_capacity: NonZeroU16) -> Self {
        Self {
            queue_capacity,
            batch_capacity: NonZeroU16::new(64).unwrap(),
        }
    }

    /// Limit both requests held by the owner and Context rows per VM entry.
    pub fn with_batch_capacity(mut self, capacity: NonZeroU16) -> Self {
        self.batch_capacity = capacity;
        self
    }
}
