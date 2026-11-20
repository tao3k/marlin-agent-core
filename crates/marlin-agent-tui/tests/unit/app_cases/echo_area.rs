// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{EchoArea, KeySequenceState};
use marlin_agent_ui::UiEchoPolicy;
use proptest::{
    prelude::ProptestConfig, prop_assert, prop_assert_eq, proptest, test_runner::RngSeed,
};
use std::time::{Duration, Instant};

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        rng_seed: RngSeed::Fixed(42),
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn replacement_uses_its_own_deadline(timeout in 100u64..=30_000, offset in 1u64..100) {
        let duration = Duration::from_millis(timeout);
        let now = Instant::now();
        let later = now + Duration::from_millis(offset);
        let mut echo = EchoArea::default();
        echo.configure(&UiEchoPolicy { timeout: duration, ..UiEchoPolicy::default() });
        echo.set_message("old".into(), now);
        echo.set_message("replacement".into(), later);
        prop_assert!(!echo.expire(now + duration));
        prop_assert_eq!(echo.message(), Some("replacement"));
        prop_assert!(echo.expire(later + duration));
        prop_assert_eq!(echo.message(), None);
        prop_assert!(!echo.expire(later + duration));
    }
}

#[test]
fn input_dismisses_feedback_without_interrupting_a_pending_sequence() {
    let duration = Duration::from_millis(731);
    let now = Instant::now();
    let mut echo = EchoArea::default();
    echo.configure(&UiEchoPolicy {
        timeout: duration,
        clear_on_input: true,
        ..UiEchoPolicy::default()
    });
    let keys = ["prefix".into()];
    echo.set_keys(&keys, KeySequenceState::Pending, now);
    echo.set_message("summary".into(), now);
    echo.on_input();
    assert_eq!(echo.message(), None);
    assert!(!echo.expire(now + duration));
    assert!(echo.keys.is_some());
    echo.set_keys(&keys, KeySequenceState::Complete, now + duration);
    echo.on_input();
    assert!(echo.keys.is_none());
}
