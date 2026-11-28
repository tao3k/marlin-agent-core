// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounds for Context admission through the single VM.

use std::num::NonZeroU16;

/// Default maximum concurrent activated Modules per single-VM DeckRuntime.
pub const DEFAULT_MAX_ACTIVE_MODULES: usize = 256;
/// Hard ceiling on configurable active Modules per single-VM DeckRuntime.
pub const MAX_ACTIVE_MODULES_LIMIT: usize = 4096;
/// Default maximum concurrent armed native timers per single-VM DeckRuntime.
pub const DEFAULT_MAX_ACTIVE_TIMERS: usize = 1024;
/// Hard ceiling on configurable active native timers per single-VM DeckRuntime.
pub const MAX_ACTIVE_TIMERS_LIMIT: usize = 16384;
/// Hard ceiling on any single native Deck timer delay (24 hours in microseconds).
pub const MAX_TIMER_DELAY_US: u64 = 86_400 * 1_000_000;

/// Rust admission and C ABI transfer limits for Context requests.
#[derive(Clone, Copy, Debug)]
pub struct DeckRuntimeConfig {
    pub(crate) queue_capacity: NonZeroU16,
    pub(crate) batch_capacity: NonZeroU16,
    pub(crate) max_active_modules: NonZeroU16,
    pub(crate) max_active_timers: NonZeroU16,
}

impl DeckRuntimeConfig {
    /// Configure the bounded Rust admission queue.
    pub fn new(queue_capacity: NonZeroU16) -> Self {
        Self {
            queue_capacity,
            batch_capacity: NonZeroU16::new(64).unwrap(),
            max_active_modules: NonZeroU16::new(DEFAULT_MAX_ACTIVE_MODULES as u16).unwrap(),
            max_active_timers: NonZeroU16::new(DEFAULT_MAX_ACTIVE_TIMERS as u16).unwrap(),
        }
    }

    /// Limit both requests held by the owner and Context rows per VM entry.
    pub fn with_batch_capacity(mut self, capacity: NonZeroU16) -> Self {
        self.batch_capacity = capacity;
        self
    }

    /// Limit concurrent activated Modules owned by this single VM.
    pub fn with_max_active_modules(mut self, capacity: NonZeroU16) -> Self {
        self.max_active_modules = capacity;
        self
    }

    /// Limit concurrent armed native timers owned by this single VM.
    pub fn with_max_active_timers(mut self, capacity: NonZeroU16) -> Self {
        self.max_active_timers = capacity;
        self
    }
}
