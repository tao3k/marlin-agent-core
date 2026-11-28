// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{Command, Duration, Terminal, fs};

#[test]
#[ignore = "linked terminal with real Git, direnv and an isolated authorization store"]
fn mx_envrc_controls_the_current_git_worktree() {
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(path)
            .status()
            .unwrap()
            .success()
    );
    fs::write(path.join(".envrc"), "export MARLIN_PTY_ENVRC=ready\n").unwrap();
    let config = path.join("tui.toml");
    fs::write(
        &config,
        concat!(
            "provider=\"openai\"\n",
            "model=\"gpt-fixture\"\n",
            "endpoint=\"http://127.0.0.1:1/v1/responses\"\n",
            "protocol=\"open-ai-responses\"\n",
            "credential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n",
        ),
    )
    .unwrap();
    let variables = [
        (
            "XDG_DATA_HOME",
            path.join("data").to_str().unwrap().to_owned(),
        ),
        (
            "XDG_CONFIG_HOME",
            path.join("config").to_str().unwrap().to_owned(),
        ),
    ];
    let mut terminal = Terminal::start_in(&config, &[], &variables, 30, 100, path);
    terminal.wait_status("Worktree ENVRC blocked");
    let command = |terminal: &mut Terminal, name: &str, expected: &str| {
        terminal.send(b"\x0f");
        terminal.wait_text("M-x");
        terminal.send(format!("{name}\r").as_bytes());
        terminal.wait_status(expected);
    };
    command(&mut terminal, "envrc-allow", "Worktree ENVRC ready");
    command(&mut terminal, "envrc-status", "Worktree ENVRC ready");
    command(&mut terminal, "envrc-reload", "Worktree ENVRC ready");
    command(&mut terminal, "envrc-deny", "Worktree ENVRC denied");
    command(&mut terminal, "envrc-reload", "Worktree ENVRC blocked");
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    terminal.assert_restored();
}
