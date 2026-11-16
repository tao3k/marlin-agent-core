// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::super::{DeckModuleInstanceId, DeckTimerId, DeckTimerReceipt};
use super::*;
use std::num::NonZeroU64;
fn module(id: u64) -> DeckModuleInstanceId {
    DeckModuleInstanceId(NonZeroU64::new(id).unwrap())
}
fn timer(id: u64) -> DeckTimerId {
    DeckTimerId(NonZeroU64::new(id).unwrap())
}
#[test]
fn timer_events_preserve_exact_module_and_message_ownership() {
    let mut controls = Controls::new();
    let (reply, mut result) = tokio::sync::oneshot::channel();
    controls.timers.insert(
        timer(1),
        TimerSlot {
            module: module(1),
            reply,
        },
    );
    let valid = DeckTimerReceipt {
        module: module(1),
        timer: timer(1),
        due_monotonic_us: 10,
        observed_monotonic_us: 11,
    };
    let foreign = DeckTimerReceipt {
        module: module(2),
        ..valid
    };
    let unknown = DeckTimerReceipt {
        timer: timer(2),
        ..valid
    };
    assert!(controls.validate_timers(&[valid, foreign]).is_err());
    assert!(controls.validate_timers(&[valid, valid]).is_err());
    assert!(controls.validate_timers(&[unknown]).is_err());
    assert!(result.try_recv().is_err());
    assert_eq!(controls.timers.len(), 1);
    controls.validate_timers(&[valid]).unwrap();
    controls.publish_timers(vec![valid]);
    assert_eq!(result.try_recv().unwrap().unwrap(), valid);
    assert!(controls.timers.is_empty());
    assert!(controls.validate_timers(&[valid]).is_err());
}
