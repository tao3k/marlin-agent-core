// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real terminal interactions for commands, Buffers, and themes.

use super::{Duration, Terminal, fs, mx_last, mx_position, mx_selected, screen_lines};

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
    terminal.enter_normal_mode();
    terminal.send(b"\x1bxswb");
    terminal.wait_for(Duration::from_secs(5), "Scheme abbreviation", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| line.contains("1/1 M-x swb"))
            && rows.iter().any(|line| line.contains("› switch-buffer"))
    });
    terminal.send(b"\r");
    terminal.wait_for(Duration::from_secs(5), "scoped buffer menu", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| line.contains(" Switch to: "))
            && rows.iter().any(|line| line.contains("› "))
    });
    terminal.send(b"\x07\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn painted_buffer_tabs_switch_the_window_and_preserve_the_draft() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "Buffer tabs", |output| {
        screen_lines(output)[0].contains(" Messages ")
    });
    terminal.send(b"draft kept by the window");
    terminal.wait_for(Duration::from_secs(5), "message draft", |output| {
        screen_lines(output)
            .iter()
            .any(|line| line.contains("draft kept by the window"))
    });
    let click = |terminal: &mut Terminal, label: &str| {
        let rows = screen_lines(&terminal.output);
        let offset = rows[0].find(label).expect("visible Buffer tab");
        let column = unicode_width::UnicodeWidthStr::width(&rows[0][..offset]) + 1;
        terminal.send(format!("\x1b[<0;{column};1M").as_bytes());
    };
    click(&mut terminal, "Messages");
    terminal.wait_for(Duration::from_secs(5), "Messages selected", |output| {
        screen_lines(output)[0].contains("▎ \u{f0e0} Messages")
    });
    click(&mut terminal, "Models");
    terminal.wait_for(
        Duration::from_secs(5),
        "borderless Models Buffer",
        |output| {
            let rows = screen_lines(output);
            let modeline = rows.iter().position(|line| line.contains("MARLIN"));
            rows[1].starts_with(" *Models*")
                && modeline.is_some_and(|end| {
                    rows[1..end]
                        .iter()
                        .all(|line| !line.starts_with(['┌', '│', '└']))
                })
        },
    );
    click(&mut terminal, "Chat");
    terminal.wait_for(Duration::from_secs(5), "draft remains in Chat", |output| {
        let rows = screen_lines(output);
        !rows[29].contains("*Messages*")
            && rows
                .iter()
                .any(|line| line.contains("draft kept by the window"))
    });
    terminal.resize(30, 32);
    terminal.wait_for(Duration::from_secs(5), "compact Buffer tabs", |output| {
        let rows = screen_lines(output);
        rows[0].contains(" Chat ") && rows[0].contains(" Msgs ")
    });
    terminal.resize(5, 32);
    terminal.wait_for(Duration::from_secs(5), "short terminal", |output| {
        let rows = screen_lines(output);
        !rows[0].contains(" Chat ")
            && rows[..5].iter().any(|line| line.contains("MARLIN"))
            && rows[..5]
                .iter()
                .any(|line| line.contains("draft kept by the"))
    });
    terminal.resize(30, 100);
    terminal.wait_for(Duration::from_secs(5), "restored terminal", |output| {
        let rows = screen_lines(output);
        rows[0].contains(" Messages ")
            && rows[27..]
                .iter()
                .any(|line| line.contains("draft kept by the window"))
    });
    terminal.send(b"\x11");
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
    terminal.enter_normal_mode();
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
    terminal.send(b"\x18b");
    terminal.wait_for(
        Duration::from_secs(5),
        "buffer menu with Messages",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| line.contains("Switch to:"))
                && rows
                    .iter()
                    .any(|line| line.trim_start().starts_with("messages"))
        },
    );
    let rows = screen_lines(&terminal.output);
    let messages_row = rows
        .iter()
        .position(|line| line.trim_start().starts_with("messages"))
        .unwrap();
    terminal.send(format!("\x1b[<0;30;{}M", messages_row + 1).as_bytes());
    terminal.wait_for(
        Duration::from_secs(5),
        "clicked Messages previews without confirmation",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| line.contains("Switch to:"))
                && rows.iter().any(|line| line.contains("› messages"))
                && rows[..28]
                    .iter()
                    .any(|line| line.contains("Use M-x to see available commands"))
        },
    );
    terminal.send(b"\r");
    terminal.wait_for(
        Duration::from_secs(5),
        "full Messages diagnostic",
        |output| {
            let rows = screen_lines(output);
            rows[0].contains("▎ \u{f0e0} Messages")
                && rows.iter().all(|line| !line.contains("Switch to:"))
                && rows[..28]
                    .iter()
                    .any(|line| line.contains("Use M-x to see available commands"))
        },
    );
    terminal.send(b"\x1b");
    terminal.wait_for(Duration::from_secs(5), "conversation restored", |output| {
        let rows = screen_lines(output);
        rows[0].contains("▎ \u{f086} Chat")
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
    terminal.enter_normal_mode();
    terminal.send(b"\x1bxhelp\r");
    terminal.wait_for(Duration::from_secs(5), "Scheme Help buffer", |output| {
        let rows = screen_lines(output);
        rows[..28]
            .iter()
            .any(|line| line.contains("C-r  Search prompts"))
            && rows[0].contains("▎ \u{f059} Help")
            && rows[..28].iter().all(|line| !line.contains("Help: Enter"))
    });
    terminal.send(b"\x1b");
    terminal.wait_for(Duration::from_secs(5), "conversation restored", |output| {
        let rows = screen_lines(output);
        rows[0].contains("▎ \u{f086} Chat")
            && rows[..28]
                .iter()
                .all(|line| !line.contains("C-r  Search prompts"))
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
    terminal.enter_normal_mode();
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
        },
    );
    terminal.send(b"\x1b[B");
    terminal.wait_for(
        Duration::from_secs(5),
        "Down wraps to first command",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| mx_selected(line, 1))
        },
    );
    terminal.send(b"\x1b[B");
    terminal.wait_for(
        Duration::from_secs(5),
        "first Down selects second command",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| mx_selected(line, 2))
        },
    );
    terminal.send(b"\x1b[A");
    terminal.wait_for(
        Duration::from_secs(5),
        "Up selects first command",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| mx_selected(line, 1))
        },
    );
    terminal.send(b"\x1b[<65;30;25M");
    terminal.wait_for(
        Duration::from_secs(5),
        "wheel selects second command",
        |output| screen_lines(output).iter().any(|line| mx_selected(line, 2)),
    );
    terminal.send(b"\x1b[<64;30;25M");
    terminal.wait_for(
        Duration::from_secs(5),
        "wheel restores first command",
        |output| screen_lines(output).iter().any(|line| mx_selected(line, 1)),
    );
    let rows = screen_lines(&terminal.output);
    let first_row = rows
        .iter()
        .position(|line| line.trim_start().starts_with('›'))
        .unwrap();
    terminal.send(format!("\x1b[<0;30;{}M", first_row + 2).as_bytes());
    terminal.wait_for(
        Duration::from_secs(5),
        "click selects the painted second command",
        |output| screen_lines(output).iter().any(|line| mx_selected(line, 2)),
    );
    terminal.send(b"\x1b[A");
    terminal.wait_for(
        Duration::from_secs(5),
        "keyboard returns to first command",
        |output| screen_lines(output).iter().any(|line| mx_selected(line, 1)),
    );
    terminal.send(&b"\x1b[<65;30;25M".repeat(10));
    terminal.wait_for(
        Duration::from_secs(5),
        "trackpad burst advances ten commands",
        |output| {
            screen_lines(output)
                .iter()
                .any(|line| mx_selected(line, 11))
        },
    );
    terminal.send(&b"\x1b[<64;30;25M".repeat(10));
    terminal.wait_for(
        Duration::from_secs(5),
        "reverse burst returns to first",
        |output| screen_lines(output).iter().any(|line| mx_selected(line, 1)),
    );
    let total = screen_lines(&terminal.output)
        .iter()
        .find_map(|line| mx_position(line))
        .unwrap()
        .1;
    assert!(total > 4);
    terminal.send(&b"\x1b[<65;30;25M".repeat(20));
    terminal.wait_for(
        Duration::from_secs(5),
        "large trackpad burst retains every pulse",
        |output| {
            screen_lines(output)
                .iter()
                .any(|line| mx_selected(line, 20 % total + 1))
        },
    );
    terminal.send(&b"\x1b[<64;30;25M".repeat(20));
    terminal.wait_for(
        Duration::from_secs(5),
        "large reverse burst restores selection",
        |output| screen_lines(output).iter().any(|line| mx_selected(line, 1)),
    );
    terminal.send(b"mode");
    terminal.wait_for(Duration::from_secs(5), "filtered command list", |output| {
        screen_lines(output)
            .iter()
            .any(|line| mx_selected(line, 1) && line.contains(" M-x mode"))
    });
    terminal.send(b"\x1b[B");
    terminal.wait_for(
        Duration::from_secs(5),
        "first filtered Down selects next",
        |output| {
            let rows = screen_lines(output);
            rows.iter()
                .any(|line| mx_selected(line, 2) && line.contains(" M-x mode"))
                && rows.iter().any(|line| line.contains("› models"))
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
            rows.iter()
                .any(|line| line.contains("2/") && line.contains(" Switch to: "))
                && rows.iter().any(|line| line.contains("› "))
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
        |output| {
            let rows = screen_lines(output);
            rows[0].contains("▎ \u{f0e0} Messages")
                && rows.iter().all(|line| !line.contains("Switch to:"))
        },
    );
    terminal.send(b"\x1b");
    terminal.wait_for(
        Duration::from_secs(5),
        "conversation buffer restored",
        |output| screen_lines(output)[0].contains("▎ \u{f086} Chat"),
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn mx_switches_tab_style_and_workbench_theme_independently() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "Doom Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.enter_normal_mode();
    terminal.send(b"\x1bxcentaur-tabs-themes-switch\r");
    terminal.wait_for(
        Duration::from_secs(5),
        "Centaur Tabs style menu",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| line.contains("Tab style: "))
                && rows.iter().any(|line| line.contains("box"))
        },
    );
    terminal.send(b"box\r");
    terminal.wait_for(
        Duration::from_secs(5),
        "Box tabs with Doom Modeline",
        |output| {
            let rows = screen_lines(output);
            rows[0].contains("Chat│") && rows[28].contains("MARLIN")
        },
    );
    terminal.send(b"\x1bxswitch-themes\r");
    terminal.wait_for(Duration::from_secs(5), "Scheme theme menu", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| line.contains("Theme: "))
            && rows.iter().any(|line| line.contains("› pi"))
    });
    terminal.send(b"pi\r");
    terminal.wait_for(
        Duration::from_secs(5),
        "Box tabs with Pi Modeline",
        |output| {
            let rows = screen_lines(output);
            rows[0].contains("Chat│") && rows[28].contains("marlin")
        },
    );
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
    terminal.enter_normal_mode();
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
