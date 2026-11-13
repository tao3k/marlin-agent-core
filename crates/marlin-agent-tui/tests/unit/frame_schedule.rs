// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::time::{Duration, Instant};

use super::frame_schedule::FrameSchedule;

#[test]
fn idle_frames_are_suppressed_and_dirty_changes_coalesce() {
    let start = Instant::now();
    let mut schedule = FrameSchedule::new();
    assert!(schedule.should_draw(start));
    schedule.drawn(start);
    assert!(!schedule.should_draw(start + Duration::from_secs(1)));
    schedule.mark_dirty();
    schedule.mark_dirty();
    assert!(!schedule.should_draw(start + Duration::from_millis(5)));
    assert_eq!(
        schedule.wait_duration(start + Duration::from_millis(5)),
        Duration::from_millis(11)
    );
    assert!(schedule.should_draw(start + Duration::from_millis(16)));
    schedule.drawn(start + Duration::from_millis(16));
    assert!(!schedule.should_draw(start + Duration::from_millis(50)));
}
