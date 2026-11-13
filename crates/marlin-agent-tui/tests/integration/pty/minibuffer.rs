// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real terminal regression for the first M-x navigation input.
use super::{Duration, Instant, Terminal, fs, mx_last, mx_position, mx_selected, screen_lines};

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
    terminal.send(b"draft\x18h");
    terminal.wait_for(Duration::from_secs(5), "hydra hint", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| line.contains("draft"))
            && rows.iter().any(|line| line.contains("History"))
            && rows
                .iter()
                .any(|line| line.contains("p newest  n older  q quit"))
    });
    terminal.send(b"nq");
    terminal.wait_for(Duration::from_secs(5), "hydra exit", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| line.contains("draft"))
            && !rows.iter().any(|line| line.contains("draftn"))
            && !rows
                .iter()
                .any(|line| line.contains("p newest  n older  q quit"))
    });
    terminal.send(b"\x18hz");
    terminal.wait_for(Duration::from_secs(5), "foreign key replay", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| line.contains("draftz"))
            && !rows
                .iter()
                .any(|line| line.contains("p newest  n older  q quit"))
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
            rows.iter().any(|line| line.contains("alpha"))
                && rows.iter().any(|line| line.contains("<beta>"))
                && rows.iter().any(|line| line.contains("Message: Enter send"))
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

    // One PTY write leaves no opportunity to wait for the first M-x frame.
    terminal.send(b"\x1bx\x1b[B");
    terminal.wait_for(Duration::from_secs(5), "immediate Down", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| mx_selected(line, 2))
            && rows.iter().any(|line| line.contains("› models"))
    });
    terminal.send(b"\x07");
    terminal.wait_for(Duration::from_secs(5), "closed M-x", |output| {
        let rows = screen_lines(output);
        rows[28].contains("MARLIN") && !rows.iter().any(|line| mx_position(line).is_some())
    });
    terminal.send(b"\x1bx\x1b[A");
    terminal.wait_for(Duration::from_secs(5), "immediate Up", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| mx_last(line)) && rows.iter().any(|line| line.contains("› clear"))
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
            && rows.iter().any(|line| line.contains("› models"))
    });
    terminal.send(b"\x1b[1;1A");
    terminal.wait_for(Duration::from_secs(5), "explicit CSI Up", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| mx_selected(line, 1))
            && rows.iter().any(|line| line.contains("› help"))
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
    terminal.send(b"\x1bxm\x1b[B");
    terminal.wait_for(
        Duration::from_secs(5),
        "immediate filtered Down",
        |output| {
            let rows = screen_lines(output);
            rows.iter()
                .any(|line| mx_selected(line, 2) && line.contains(" M-x m"))
                && rows.iter().any(|line| line.contains("› model"))
        },
    );
    terminal.send(b"\x07");
    terminal.wait_for(Duration::from_secs(5), "closed filtered M-x", |output| {
        !screen_lines(output)
            .iter()
            .any(|line| mx_position(line).is_some())
    });
    // The first arrow must change the executable choice, not only its paint.
    terminal.send(b"\x1bx\x1b[B\r");
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
fn mx_abbreviation_opens_the_scheme_declared_buffer_menu() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "initial Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.send(b"\x1bxswb");
    terminal.wait_for(Duration::from_secs(5), "Scheme abbreviation", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| line.contains("1/1 M-x swb"))
            && rows.iter().any(|line| line.contains("› switch-buffer"))
    });
    terminal.send(b"\r");
    terminal.wait_for(Duration::from_secs(5), "scoped buffer menu", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| line.contains("1/3 Switch to: "))
            && rows.iter().any(|line| line.contains("› conversation"))
    });
    terminal.send(b"\x07\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn invalid_mx_command_uses_messages_buffer_without_changing_conversation() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "initial Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.send(b"\x1bxno-such-command\r");
    terminal.wait_for(
        Duration::from_secs(5),
        "command diagnostic summary",
        |output| {
            let rows = screen_lines(output);
            rows[29].contains("Unknown command or arguments")
                && rows[..28]
                    .iter()
                    .all(|line| !line.contains("Unknown command"))
        },
    );
    terminal.send(b"\x18bmessages\r");
    terminal.wait_for(
        Duration::from_secs(5),
        "full Messages diagnostic",
        |output| {
            let rows = screen_lines(output);
            rows[29].contains("*Messages*")
                && rows[..28]
                    .iter()
                    .any(|line| line.contains("Use M-x to see available commands"))
        },
    );
    terminal.send(b"\x1b");
    terminal.wait_for(Duration::from_secs(5), "conversation restored", |output| {
        let rows = screen_lines(output);
        rows[29].contains("Message: Enter send")
            && rows[..28]
                .iter()
                .all(|line| !line.contains("Unknown command"))
    });
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn mx_help_switches_to_scheme_buffer_without_appending_conversation() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "initial Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.send(b"\x1bxhelp\r");
    terminal.wait_for(Duration::from_secs(5), "Scheme Help buffer", |output| {
        let rows = screen_lines(output);
        rows[..28]
            .iter()
            .any(|line| line.contains("Ctrl-R  Search prompts"))
            && rows[29].contains("*Help*")
            && rows[..28].iter().all(|line| !line.contains("Help: Enter"))
    });
    terminal.send(b"\x1b");
    terminal.wait_for(Duration::from_secs(5), "conversation restored", |output| {
        let rows = screen_lines(output);
        rows[29].contains("Message: Enter send")
            && rows[..28]
                .iter()
                .all(|line| !line.contains("Ctrl-R  Search prompts"))
    });
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn first_mx_arrows_move_the_visible_selection_and_c_x_b_opens_buffers() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "initial Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.send(b"\x1bx");
    terminal.wait_for(Duration::from_secs(5), "M-x first candidate", |output| {
        screen_lines(output).iter().any(|line| mx_selected(line, 1))
    });
    terminal.send(b"\x1b[A");
    terminal.wait_for(
        Duration::from_secs(5),
        "first Up wraps to last command",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| mx_last(line))
                && rows.iter().any(|line| line.contains("› clear"))
        },
    );
    terminal.send(b"\x1b[B");
    terminal.wait_for(
        Duration::from_secs(5),
        "Down wraps to first command",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| mx_selected(line, 1))
                && rows.iter().any(|line| line.contains("› help"))
        },
    );
    terminal.send(b"\x1b[B");
    terminal.wait_for(
        Duration::from_secs(5),
        "first Down selects second command",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| mx_selected(line, 2))
                && rows.iter().any(|line| line.contains("› models"))
        },
    );
    terminal.send(b"\x1b[A");
    terminal.wait_for(
        Duration::from_secs(5),
        "Up selects first command",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| mx_selected(line, 1))
                && rows.iter().any(|line| line.contains("› help"))
        },
    );
    terminal.send(b"\x1b[<65;30;5M");
    terminal.wait_for(
        Duration::from_secs(5),
        "wheel selects second command",
        |output| screen_lines(output).iter().any(|line| mx_selected(line, 2)),
    );
    terminal.send(b"\x1b[<64;30;5M");
    terminal.wait_for(
        Duration::from_secs(5),
        "wheel restores first command",
        |output| screen_lines(output).iter().any(|line| mx_selected(line, 1)),
    );
    terminal.send(&b"\x1b[<65;30;5M".repeat(10));
    terminal.wait_for(
        Duration::from_secs(5),
        "trackpad burst advances ten commands",
        |output| {
            screen_lines(output)
                .iter()
                .any(|line| mx_selected(line, 11))
        },
    );
    terminal.send(&b"\x1b[<64;30;5M".repeat(10));
    terminal.wait_for(
        Duration::from_secs(5),
        "reverse burst returns to first",
        |output| screen_lines(output).iter().any(|line| mx_selected(line, 1)),
    );
    terminal.send(b"m");
    terminal.wait_for(Duration::from_secs(5), "filtered command list", |output| {
        screen_lines(output)
            .iter()
            .any(|line| mx_selected(line, 1) && line.contains(" M-x m"))
    });
    terminal.send(b"\x1b[B");
    terminal.wait_for(
        Duration::from_secs(5),
        "first filtered Down selects next",
        |output| {
            let rows = screen_lines(output);
            rows.iter()
                .any(|line| mx_selected(line, 2) && line.contains(" M-x m"))
                && rows.iter().any(|line| line.contains("› model"))
        },
    );
    terminal.send(b"\x07");
    terminal.wait_for(Duration::from_secs(5), "C-g closes M-x", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    // Prefix command and first arrow may arrive before the scoped menu paints.
    terminal.send(b"\x18b\x1b[B");
    terminal.wait_for(
        Duration::from_secs(5),
        "C-x b opens and navigates buffer selection",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| line.contains("2/3 Switch to: "))
                && rows.iter().any(|line| line.contains("› messages"))
        },
    );
    terminal.send(b"mess");
    terminal.wait_for(Duration::from_secs(5), "scoped buffer query", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| line.contains("1/1 Switch to: mess"))
            && rows.iter().any(|line| line.contains("› messages"))
            && rows.iter().all(|line| !line.contains("buffer-messages"))
    });
    terminal.send(b"ages\r");
    terminal.wait_for(
        Duration::from_secs(5),
        "Messages buffer selected",
        |output| screen_lines(output)[29].contains("*Messages*"),
    );
    terminal.send(b"\x1b");
    terminal.wait_for(
        Duration::from_secs(5),
        "conversation buffer restored",
        |output| screen_lines(output)[29].contains("Message: Enter send"),
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn mx_profile_switch_uses_the_selected_scheme_modeline() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "Doom Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.send(b"\x1bxswitch-themes\r");
    terminal.wait_for(Duration::from_secs(5), "Scheme theme menu", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| line.contains("Theme: "))
            && rows.iter().any(|line| line.contains("› doom-one"))
    });
    terminal.send(b"pi\r");
    terminal.wait_for(Duration::from_secs(5), "pi Modeline", |output| {
        screen_lines(output)[28].contains("marlin")
    });
    terminal.send(b"\x1bxtheme-doom-one\r");
    terminal.wait_for(Duration::from_secs(5), "restored Doom Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn narrow_mx_profile_switch_changes_optional_modeline_priority() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start_sized(&config, &[], &[], 30, 44);
    terminal.wait_for(Duration::from_secs(10), "narrow Doom Modeline", |output| {
        let line = &screen_lines(output)[28];
        line.contains("openai/gpt-fixture") && !line.contains("temporary")
    });
    terminal.send(b"\x1bxtheme-pi\r");
    terminal.wait_for(Duration::from_secs(5), "narrow pi Modeline", |output| {
        let line = &screen_lines(output)[28];
        line.contains("marlin")
            && line.contains("temporary")
            && !line.contains("openai/gpt-fixture")
    });
    terminal.send(b"\x1bxtheme-doom-one\r");
    terminal.wait_for(
        Duration::from_secs(5),
        "restored narrow Doom Modeline",
        |output| {
            let line = &screen_lines(output)[28];
            line.contains("MARLIN")
                && line.contains("openai/gpt-fixture")
                && !line.contains("temporary")
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
    terminal.wait_for(Duration::from_secs(10), "initial Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.send(b"\x1bx");
    terminal.wait_for(Duration::from_secs(5), "M-x", |output| {
        screen_lines(output).iter().any(|line| mx_selected(line, 1))
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
        terminal.wait_for(Duration::from_secs(5), "arrow redraw", |output| {
            screen_lines(output)
                .iter()
                .any(|line| mx_selected(line, expected))
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
        terminal.wait_for(Duration::from_secs(5), "closed M-x", |output| {
            let rows = screen_lines(output);
            rows[28].contains("MARLIN") && !rows.iter().any(|line| mx_position(line).is_some())
        });
        let started = Instant::now();
        terminal.send(match index % 3 {
            0 => b"\x1bx\x1b[B",
            1 => b"\x1bx\x1bOB",
            _ => b"\x1bx\x1b[1;1B",
        });
        terminal.wait_for(Duration::from_secs(5), "first arrow redraw", |output| {
            screen_lines(output).iter().any(|line| mx_selected(line, 2))
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
        terminal.wait_for(Duration::from_secs(5), "query redraw", |output| {
            screen_lines(output)
                .iter()
                .any(|line| mx_selected(line, 1) && (!filtered || line.contains(" M-x m")))
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
    terminal.wait_for(Duration::from_secs(5), "closed M-x", |output| {
        !screen_lines(output)
            .iter()
            .any(|line| mx_position(line).is_some())
    });
    let mut abbreviation_samples = Vec::with_capacity(30);
    for _ in 0..30 {
        let started = Instant::now();
        terminal.send(b"\x1bxswb");
        terminal.wait_for(Duration::from_secs(5), "subsequence candidate", |output| {
            screen_lines(output)
                .iter()
                .any(|line| line.contains("1/1 M-x swb"))
        });
        abbreviation_samples.push(started.elapsed());
        terminal.send(b"\x07");
        terminal.wait_for(
            Duration::from_secs(5),
            "closed abbreviation menu",
            |output| {
                !screen_lines(output)
                    .iter()
                    .any(|line| line.contains("M-x swb"))
            },
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
        terminal.wait_for(Duration::from_secs(5), "first scoped arrow", |output| {
            screen_lines(output)
                .iter()
                .any(|line| line.contains("2/3 Switch to: "))
        });
        scoped_samples.push(started.elapsed());
        terminal.send(b"\x07");
        terminal.wait_for(Duration::from_secs(5), "closed scoped menu", |output| {
            !screen_lines(output)
                .iter()
                .any(|line| line.contains("Switch to: "))
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
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();

    // Each sample uses a new process and its first menu opening. The terminal
    // is already visible, so startup and disk-cache effects are reported apart.
    let mut first_process_samples = Vec::with_capacity(10);
    for index in 0..10 {
        let mut terminal = Terminal::start(&config);
        terminal.wait_for(Duration::from_secs(10), "initial Modeline", |output| {
            screen_lines(output)[28].contains("MARLIN")
        });
        let started = Instant::now();
        terminal.send(match index % 3 {
            0 => b"\x1bx\x1b[B",
            1 => b"\x1bx\x1bOB",
            _ => b"\x1bx\x1b[1;1B",
        });
        terminal.wait_for(
            Duration::from_secs(5),
            "new-process first arrow",
            |output| screen_lines(output).iter().any(|line| mx_selected(line, 2)),
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
