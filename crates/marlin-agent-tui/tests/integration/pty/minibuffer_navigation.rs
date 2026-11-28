// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real terminal regression for the first M-x navigation input.

use super::{Duration, Instant, Terminal, fs, mx_last, mx_position, mx_selected, screen_lines};

#[test]
fn doom_leader_uses_normal_mode_without_stealing_insert_spaces() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"\x1bxa x");
    terminal.wait_text("a x");
    terminal.enter_normal_mode();
    terminal.send(b"\x18");
    terminal.wait_for(
        Duration::from_secs(5),
        "ordinary prefix echoes beside the draft",
        |output| {
            let rows = screen_lines(output);
            rows[28].contains("NORMAL")
                && rows[29].starts_with("a x")
                && rows[29].trim_end().ends_with("C-x-")
        },
    );
    terminal.send(b"\x07");
    terminal.send(b" ");
    terminal.wait_for(
        Duration::from_secs(5),
        "prefix echo before the dashboard",
        |output| {
            let rows = screen_lines(output);
            rows[28].contains("MARLIN")
                && rows[29].trim_start().starts_with("Keypad: SPC-")
                && !rows.iter().any(|line| line.contains("Keymaps"))
        },
    );
    terminal.wait_for(Duration::from_secs(5), "Keypad dashboard", |output| {
        let rows = screen_lines(output);
        rows.iter()
            .any(|line| line.contains("Commands") && line.contains("Keymaps"))
            && rows.iter().any(|line| line.contains("b : Buffers"))
            && rows[29].contains("Keypad: SPC-")
            && rows
                .iter()
                .any(|line| line.contains("MARLIN") && line.contains("NORMAL"))
            && rows
                .iter()
                .any(|line| line.trim_start().starts_with("x   : Open commands"))
            && rows
                .iter()
                .any(|line| line.trim_start().starts_with("SPC : Open commands"))
    });
    let wide_column = screen_lines(&terminal.output)
        .iter()
        .find_map(|line| line.find("Keymaps"))
        .unwrap();
    terminal.resize(30, 32);
    terminal.wait_for(
        Duration::from_secs(5),
        "Keypad fits narrow terminal",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| {
                line.contains("Commands")
                    && line
                        .find("Keymaps")
                        .is_some_and(|column| column < wide_column)
            }) && rows.iter().any(|line| line.contains("b : Buffers"))
        },
    );
    terminal.resize(11, 100);
    terminal.wait_for(
        Duration::from_secs(5),
        "dashboard drops padding on short terminals",
        |output| {
            let rows = screen_lines(output);
            rows[9].contains("b : Buffers")
                && rows[10].contains("Keypad: SPC-")
                && rows[..11]
                    .iter()
                    .any(|line| line.contains("MARLIN") && line.contains("NORMAL"))
        },
    );
    terminal.resize(30, 100);
    terminal.send(b"b");
    terminal.wait_status("Keypad: SPC b-");
    terminal.wait_text("g : buffers group");
    terminal.send(b"\x7f");
    terminal.wait_status("Keypad: SPC-");
    terminal.send(b"\x07");
    terminal.wait_text("NORMAL");
    terminal.send(b" Z");
    terminal.wait_status("SPC Z is undefined");
    terminal.send(b" x");
    terminal.wait_mx_selected(1);
    terminal.send(b"\x07");
    terminal.wait_text("NORMAL");
    terminal.send(b"a b");
    terminal.wait_text("a x b");
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn accepted_command_ranks_first_on_next_mx_and_navigation_uses_that_order() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "initial Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.enter_normal_mode();
    terminal.send(b"\x1bxclear");
    terminal.wait_for(Duration::from_secs(5), "exact command", |output| {
        screen_lines(output)
            .iter()
            .any(|line| line.contains("M-x clear"))
    });
    terminal.send(b"\r");
    terminal.wait_for(Duration::from_secs(5), "command accepted", |output| {
        let rows = screen_lines(output);
        rows[28].contains("MARLIN") && !rows.iter().any(|line| mx_position(line).is_some())
    });
    terminal.send(b"\x1bx");
    terminal.wait_for(Duration::from_secs(5), "recent command first", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| mx_selected(line, 1))
            && rows.iter().any(|line| line.contains("› clear"))
    });
    terminal.send(b"\x1b[B");
    terminal.wait_for(Duration::from_secs(5), "ranked next", |output| {
        screen_lines(output).iter().any(|line| mx_selected(line, 2))
    });
    terminal.send(b"\x1b[A");
    terminal.wait_for(Duration::from_secs(5), "ranked navigation", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| mx_selected(line, 1))
            && rows.iter().any(|line| line.contains("› clear"))
    });
    terminal.send(b"\x07\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn filtering_preserves_a_user_selected_candidate_when_still_visible() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "initial Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.enter_normal_mode();
    terminal.send(b"\x1bxmode\x1b[B");
    terminal.wait_for(Duration::from_secs(5), "selected models", |output| {
        let rows = screen_lines(output);
        rows.iter()
            .any(|line| mx_selected(line, 2) && line.contains(" M-x mode"))
            && rows.iter().any(|line| line.contains("› models"))
            && rows
                .iter()
                .find_map(|line| {
                    line.find("Select a model alias")
                        .map(|index| line[..index].chars().count())
                })
                .zip(rows.iter().find_map(|line| {
                    line.find("List configured models")
                        .map(|index| line[..index].chars().count())
                }))
                .is_some_and(|(left, right)| left == right)
    });
    terminal.send(b"\x7f");
    terminal.wait_for(
        Duration::from_secs(5),
        "locked selection after filtering",
        |output| {
            let rows = screen_lines(output);
            rows.iter()
                .any(|line| mx_selected(line, 2) && line.contains(" M-x mod"))
                && rows.iter().any(|line| line.contains("› models"))
        },
    );
    terminal.send(b"\x07\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn hydra_hint_and_foreign_key_use_the_minibuffer_without_losing_the_draft() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "initial Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.send(b"draft");
    terminal.wait_text("draft");
    terminal.enter_normal_mode();
    terminal.send(b"\x1bxhydra-history");
    terminal.wait_for(Duration::from_secs(5), "M-x hydra command", |output| {
        screen_lines(output)
            .iter()
            .any(|line| line.contains("M-x hydra-history"))
    });
    terminal.send(b"\r");
    terminal.wait_for(Duration::from_secs(5), "hydra hint", |output| {
        let rows = screen_lines(output);
        rows.iter()
            .position(|line| line.contains("MARLIN"))
            .zip(
                rows.iter()
                    .position(|line| line.contains("p : Newest") && line.contains("q : Quit")),
            )
            .is_some_and(|(modeline, hint)| modeline < hint && rows[modeline + 1].trim().is_empty())
            && rows[28].trim().is_empty()
            && rows[29].trim().is_empty()
            && !rows.iter().any(|line| line.starts_with("draft "))
    });
    terminal.send(b"nq");
    terminal.wait_for(Duration::from_secs(5), "hydra exit", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| line.contains("draft"))
            && !rows.iter().any(|line| line.contains("draftn"))
            && !rows.iter().any(|line| line.contains("p : Newest"))
    });
    terminal.send(b"\x1bxhydra-history");
    terminal.wait_for(
        Duration::from_secs(5),
        "M-x hydra command again",
        |output| {
            screen_lines(output)
                .iter()
                .any(|line| line.contains("M-x hydra-history"))
        },
    );
    terminal.send(b"\rz");
    terminal.wait_for(Duration::from_secs(5), "foreign key replay", |output| {
        let rows = screen_lines(output);
        rows[29].starts_with("draft ") && !rows.iter().any(|line| line.contains("p : Newest"))
    });
    terminal.send(b" bg");
    terminal.wait_for(Duration::from_secs(5), "buffers group hint", |output| {
        screen_lines(output)
            .iter()
            .any(|line| line.contains("m : Messages"))
    });
    terminal.send(b"m");
    terminal.wait_for(Duration::from_secs(5), "messages buffer", |output| {
        let rows = screen_lines(output);
        rows[0].contains("▎ \u{f0e0} Messages")
            && rows[29].starts_with("draft ")
            && !rows.iter().any(|line| line.contains("m : Messages"))
    });
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn scheme_editor_keys_keep_multiline_draft_in_the_minibuffer() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "initial Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.send(b"alpha\x0ebeta\x01<\x05>");
    terminal.wait_for(
        Duration::from_secs(5),
        "Scheme-routed multiline edit",
        |output| {
            let rows = screen_lines(output);
            rows[27].contains("MARLIN")
                && rows[28].trim() == "alpha"
                && rows[29].trim() == "<beta>"
                && rows[27].contains("INSERT")
        },
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn first_mx_arrow_is_not_lost_before_the_menu_frame() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "initial Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });

    terminal.enter_normal_mode();
    // One PTY write leaves no opportunity to wait for the first M-x frame.
    terminal.send(b"\x1bx\x1b[B");
    terminal.wait_for(Duration::from_secs(5), "immediate Down", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| mx_selected(line, 2))
    });
    terminal.send(b"\x07");
    terminal.wait_for(Duration::from_secs(5), "closed M-x", |output| {
        let rows = screen_lines(output);
        rows[28].contains("MARLIN") && !rows.iter().any(|line| mx_position(line).is_some())
    });
    terminal.send(b"\x1bx\x1b[A");
    terminal.wait_for(Duration::from_secs(5), "immediate Up", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| mx_last(line))
    });
    terminal.send(b"\x07");
    terminal.wait_for(Duration::from_secs(5), "closed M-x", |output| {
        !screen_lines(output)
            .iter()
            .any(|line| mx_position(line).is_some())
    });
    // Application cursor mode emits SS3 arrows on some terminal emulators.
    terminal.send(b"\x1bx\x1bOB");
    terminal.wait_for(Duration::from_secs(5), "immediate SS3 Down", |output| {
        screen_lines(output).iter().any(|line| mx_selected(line, 2))
    });
    terminal.send(b"\x07");
    terminal.wait_for(Duration::from_secs(5), "closed SS3 M-x", |output| {
        !screen_lines(output)
            .iter()
            .any(|line| mx_position(line).is_some())
    });
    // Other terminals explicitly encode the unmodified CSI modifier. Test
    // the key after the first menu frame as well as the batched SS3/CSI path.
    terminal.send(b"\x1bx");
    terminal.wait_for(Duration::from_secs(5), "first M-x frame", |output| {
        screen_lines(output).iter().any(|line| mx_selected(line, 1))
    });
    terminal.send(b"\x1b[1;1B");
    terminal.wait_for(Duration::from_secs(5), "explicit CSI Down", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| mx_selected(line, 2))
    });
    terminal.send(b"\x1b[1;1A");
    terminal.wait_for(Duration::from_secs(5), "explicit CSI Up", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| mx_selected(line, 1))
    });
    terminal.send(b"\x07");
    terminal.wait_for(
        Duration::from_secs(5),
        "closed explicit CSI M-x",
        |output| {
            !screen_lines(output)
                .iter()
                .any(|line| mx_position(line).is_some())
        },
    );
    terminal.send(b"\x1bx\x1b[1;1B");
    terminal.wait_for(
        Duration::from_secs(5),
        "immediate explicit CSI Down",
        |output| screen_lines(output).iter().any(|line| mx_selected(line, 2)),
    );
    terminal.send(b"\x07");
    terminal.wait_for(
        Duration::from_secs(5),
        "closed immediate explicit CSI M-x",
        |output| {
            !screen_lines(output)
                .iter()
                .any(|line| mx_position(line).is_some())
        },
    );
    terminal.send(b"\x1bx\x1bOA");
    terminal.wait_for(Duration::from_secs(5), "immediate SS3 Up", |output| {
        screen_lines(output).iter().any(|line| mx_last(line))
    });
    terminal.send(b"\x07");
    terminal.wait_for(Duration::from_secs(5), "closed SS3 M-x", |output| {
        !screen_lines(output)
            .iter()
            .any(|line| mx_position(line).is_some())
    });
    // Filtering and the first navigation can also arrive before either menu
    // frame is painted. The new candidate set must be selected first.
    terminal.send(b"\x1bxmode\x1b[B");
    terminal.wait_for(
        Duration::from_secs(5),
        "immediate filtered Down",
        |output| {
            let rows = screen_lines(output);
            rows.iter()
                .any(|line| mx_selected(line, 2) && line.contains(" M-x mode"))
                && rows.iter().any(|line| line.contains("› models"))
        },
    );
    terminal.send(b"\x07");
    terminal.wait_for(Duration::from_secs(5), "closed filtered M-x", |output| {
        !screen_lines(output)
            .iter()
            .any(|line| mx_position(line).is_some())
    });
    // The first arrow must change the executable choice, not only its paint.
    terminal.send(b"\x1bxmode\x1b[B\r");
    terminal.wait_for(
        Duration::from_secs(5),
        "first-arrow command executed",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| line.contains("gpt-fixture"))
                && !rows.iter().any(|line| mx_position(line).is_some())
        },
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
#[ignore = "manual wall-clock benchmark of the linked VM, render and PTY round trip"]
fn mx_navigation_pty_latency() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for_screen(Duration::from_secs(10), "initial Modeline", |screen| {
        screen.line(28).contains("MARLIN")
    });
    terminal.enter_normal_mode();
    terminal.send(b"\x1bx");
    terminal.wait_for_screen(Duration::from_secs(5), "M-x", |screen| {
        screen.lines().any(|line| mx_selected(&line, 1))
    });

    let mut samples = Vec::with_capacity(30);
    for index in 0..30 {
        let started = Instant::now();
        terminal.send(match index % 6 {
            0 => b"\x1b[B",
            1 => b"\x1b[A",
            2 => b"\x1bOB",
            3 => b"\x1bOA",
            4 => b"\x1b[1;1B",
            _ => b"\x1b[1;1A",
        });
        let expected = if index % 2 == 0 { 2 } else { 1 };
        terminal.wait_for_screen(Duration::from_secs(5), "arrow redraw", |screen| {
            screen.lines().any(|line| mx_selected(&line, expected))
        });
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    eprintln!(
        "M-x mixed CSI/SS3/explicit CSI arrow PTY round trip: n={} p50={:?} p95={:?} max={:?}",
        samples.len(),
        samples[samples.len() / 2],
        samples[(samples.len() * 95).div_ceil(100) - 1],
        samples[samples.len() - 1]
    );

    let mut first_arrow_samples = Vec::with_capacity(30);
    for index in 0..30 {
        terminal.send(b"\x07");
        terminal.wait_for_screen(Duration::from_secs(5), "closed M-x", |screen| {
            screen.line(28).contains("MARLIN")
                && !screen.lines().any(|line| mx_position(&line).is_some())
        });
        let started = Instant::now();
        terminal.send(match index % 3 {
            0 => b"\x1bx\x1b[B",
            1 => b"\x1bx\x1bOB",
            _ => b"\x1bx\x1b[1;1B",
        });
        terminal.wait_for_screen(Duration::from_secs(5), "first arrow redraw", |screen| {
            screen.lines().any(|line| mx_selected(&line, 2))
        });
        first_arrow_samples.push(started.elapsed());
    }
    first_arrow_samples.sort_unstable();
    eprintln!(
        "M-x open plus first mixed CSI/SS3/explicit CSI arrow PTY round trip: n={} p50={:?} p95={:?} max={:?}",
        first_arrow_samples.len(),
        first_arrow_samples[first_arrow_samples.len() / 2],
        first_arrow_samples[(first_arrow_samples.len() * 95).div_ceil(100) - 1],
        first_arrow_samples[first_arrow_samples.len() - 1]
    );
    // Editing the query crosses the Scheme candidate projection once per
    // change; measure that path independently of arrow-only navigation.
    let mut query_samples = Vec::with_capacity(30);
    for index in 0..30 {
        let started = Instant::now();
        terminal.send(if index % 2 == 0 { b"m" } else { b"\x7f" });
        let filtered = index % 2 == 0;
        terminal.wait_for_screen(Duration::from_secs(5), "query redraw", |screen| {
            screen
                .lines()
                .any(|line| mx_selected(&line, 1) && line.contains(" M-x m") == filtered)
        });
        query_samples.push(started.elapsed());
    }
    query_samples.sort_unstable();
    eprintln!(
        "M-x query edit and Scheme projection PTY round trip: n={} p50={:?} p95={:?} max={:?}",
        query_samples.len(),
        query_samples[query_samples.len() / 2],
        query_samples[(query_samples.len() * 95).div_ceil(100) - 1],
        query_samples[query_samples.len() - 1]
    );
    terminal.send(b"\x07");
    terminal.wait_for_screen(Duration::from_secs(5), "closed M-x", |screen| {
        !screen.lines().any(|line| mx_position(&line).is_some())
    });
    let mut abbreviation_samples = Vec::with_capacity(30);
    for _ in 0..30 {
        let started = Instant::now();
        terminal.send(b"\x1bxswb");
        terminal.wait_for_screen(Duration::from_secs(5), "subsequence candidate", |screen| {
            screen.lines().any(|line| line.contains("1/1 M-x swb"))
        });
        abbreviation_samples.push(started.elapsed());
        terminal.send(b"\x07");
        terminal.wait_for_screen(
            Duration::from_secs(5),
            "closed abbreviation menu",
            |screen| !screen.lines().any(|line| line.contains("M-x swb")),
        );
    }
    abbreviation_samples.sort_unstable();
    eprintln!(
        "M-x open and Scheme subsequence abbreviation PTY round trip: n={} p50={:?} p95={:?} max={:?}",
        abbreviation_samples.len(),
        abbreviation_samples[abbreviation_samples.len() / 2],
        abbreviation_samples[(abbreviation_samples.len() * 95).div_ceil(100) - 1],
        abbreviation_samples[abbreviation_samples.len() - 1]
    );
    let mut scoped_samples = Vec::with_capacity(30);
    for _ in 0..30 {
        let started = Instant::now();
        terminal.send(b"\x18b\x1b[B");
        terminal.wait_for_screen(Duration::from_secs(5), "first scoped arrow", |screen| {
            screen
                .lines()
                .any(|line| line.contains("2/") && line.contains(" Switch to: "))
        });
        scoped_samples.push(started.elapsed());
        terminal.send(b"\x07");
        terminal.wait_for_screen(Duration::from_secs(5), "closed scoped menu", |screen| {
            !screen.lines().any(|line| line.contains("Switch to: "))
        });
    }
    scoped_samples.sort_unstable();
    eprintln!(
        "C-x b plus first Down scoped PTY round trip: n={} p50={:?} p95={:?} max={:?}",
        scoped_samples.len(),
        scoped_samples[scoped_samples.len() / 2],
        scoped_samples[(scoped_samples.len() * 95).div_ceil(100) - 1],
        scoped_samples[scoped_samples.len() - 1]
    );
    let mut scoped_open_samples = Vec::with_capacity(30);
    let mut scoped_arrow_samples = Vec::with_capacity(30);
    for _ in 0..30 {
        let started = Instant::now();
        terminal.send(b"\x18b");
        terminal.wait_for_screen(Duration::from_secs(5), "scoped menu open", |screen| {
            screen
                .lines()
                .any(|line| line.contains("1/") && line.contains(" Switch to: "))
        });
        scoped_open_samples.push(started.elapsed());

        let started = Instant::now();
        terminal.send(b"\x1b[B");
        terminal.wait_for_screen(Duration::from_secs(5), "scoped menu Down", |screen| {
            screen
                .lines()
                .any(|line| line.contains("2/") && line.contains(" Switch to: "))
        });
        scoped_arrow_samples.push(started.elapsed());
        terminal.send(b"\x07");
        terminal.wait_for_screen(Duration::from_secs(5), "closed scoped menu", |screen| {
            !screen.lines().any(|line| line.contains(" Switch to: "))
        });
    }
    for (label, samples) in [
        ("C-x b scoped menu open", &mut scoped_open_samples),
        (
            "C-x b scoped menu Down after open",
            &mut scoped_arrow_samples,
        ),
    ] {
        samples.sort_unstable();
        eprintln!(
            "{label} PTY round trip: n={} p50={:?} p95={:?} max={:?}",
            samples.len(),
            samples[samples.len() / 2],
            samples[(samples.len() * 95).div_ceil(100) - 1],
            samples[samples.len() - 1]
        );
    }
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();

    // Each sample uses a new process and its first menu opening. The terminal
    // is already visible, so startup and disk-cache effects are reported apart.
    let mut first_process_samples = Vec::with_capacity(10);
    for index in 0..10 {
        let mut terminal = Terminal::start(&config);
        terminal.wait_for_screen(Duration::from_secs(10), "initial Modeline", |screen| {
            screen.line(28).contains("MARLIN")
        });
        terminal.enter_normal_mode();
        let started = Instant::now();
        terminal.send(match index % 3 {
            0 => b"\x1bx\x1b[B",
            1 => b"\x1bx\x1bOB",
            _ => b"\x1bx\x1b[1;1B",
        });
        terminal.wait_for_screen(
            Duration::from_secs(5),
            "new-process first arrow",
            |screen| screen.lines().any(|line| mx_selected(&line, 2)),
        );
        first_process_samples.push(started.elapsed());
        terminal.send(b"\x11");
        terminal.wait_exit(Duration::from_secs(5));
        terminal.assert_restored();
    }
    first_process_samples.sort_unstable();
    eprintln!(
        "M-x first mixed CSI/SS3/explicit CSI arrow in new process after first frame: n={} p50={:?} p95={:?} max={:?}",
        first_process_samples.len(),
        first_process_samples[first_process_samples.len() / 2],
        first_process_samples[(first_process_samples.len() * 95).div_ceil(100) - 1],
        first_process_samples[first_process_samples.len() - 1]
    );
}
