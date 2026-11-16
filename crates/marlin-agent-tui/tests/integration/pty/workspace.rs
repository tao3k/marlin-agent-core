// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{Command, Duration, Terminal, fs, screen_lines};

#[test]
fn mx_diff_and_status_use_the_entire_launch_worktree() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let root = directory.path().join("project");
    let nested = root.join("nested");
    fs::create_dir_all(&nested).unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(output.status.success(), "{args:?}: {output:?}");
    };
    git(&["init", "-q"]);
    git(&["config", "diff.relative", "true"]);
    fs::write(root.join("root.txt"), "index-only-text\n").unwrap();
    git(&["add", "--", "root.txt"]);
    fs::write(root.join("root.txt"), "worktree-only-text\n").unwrap();
    fs::write(root.join("untracked.txt"), "new file\n").unwrap();

    let mut terminal = Terminal::start_in(&config, &[], &[], 30, 100, &nested);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"\x1bxstatus\r");
    terminal.wait_for(Duration::from_secs(5), "whole worktree status", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|row| row.contains("AM root.txt"))
            && rows.iter().any(|row| row.contains("?? untracked.txt"))
    });
    terminal.send(b"\x18b");
    terminal.wait_text("Switch to:");
    terminal.send(b"messages\r");
    terminal.wait_status("*Messages*");

    terminal.send(b"\x1bxdiff\r");
    terminal.wait_for(Duration::from_secs(5), "unstaged root diff", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|row| row.contains("Unstaged diff"))
            && rows.iter().any(|row| row.contains("+worktree-only-text"))
            && rows.iter().any(|row| row.contains("-index-only-text"))
    });
    terminal.send(b"\x1b");
    terminal.wait_for(
        Duration::from_secs(5),
        "Messages buffer restored",
        |output| {
            let rows = screen_lines(output);
            rows[..28]
                .iter()
                .any(|row| row.contains("Reading workspace…"))
                && rows[..28].iter().all(|row| !row.contains("Unstaged diff"))
        },
    );

    terminal.send(b"\x1bxdiff staged\r");
    terminal.wait_for(Duration::from_secs(5), "staged root diff", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|row| row.contains("Staged diff"))
            && rows.iter().any(|row| row.contains("+index-only-text"))
            && rows.iter().all(|row| !row.contains("worktree-only-text"))
    });
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}
