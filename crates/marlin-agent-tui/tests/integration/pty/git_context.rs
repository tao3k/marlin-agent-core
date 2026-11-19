// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{Command, Duration, Instant, Terminal, fs, screen_lines};
use std::{os::unix::fs::PermissionsExt, path::Path, thread};

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn commit_context(root: &Path, name: &str) -> String {
    for (path, anchor) in [("index.org", "index"), ("topology.org", "topology")] {
        fs::write(
            root.join(path),
            format!(":PROPERTIES:\n:ID: {anchor}\n:CONTRACT_ID: context.{name}.{anchor}.v1\n:CONTRACT_REVISION: 1\n:END:\n"),
        ).unwrap();
    }
    git(root, &["add", "index.org", "topology.org"]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "--no-gpg-sign",
            "-m",
            name,
        ],
    );
    git(root, &["rev-parse", "HEAD"])
}

fn messages(terminal: &mut Terminal) {
    terminal.wait_status("Message: Enter send");
    terminal.send(b"\x18b");
    terminal.wait_text("Switch to:");
    terminal.send(b"messages\r");
    terminal.wait_status("*Messages*");
}

fn context_ready(terminal: &mut Terminal, name: &str, commit: &str) {
    terminal.wait_for(Duration::from_secs(20), "bound Git Context", |output| {
        let rows = screen_lines(output).join("\n");
        rows.contains("Git Context ready")
            && rows.contains(commit)
            && rows.contains(&format!("context.{name}.index.v1"))
            && rows.contains(&format!("context.{name}.topology.v1"))
    });
}

#[test]
#[ignore = "requires real Worktrunk and the linked Scheme programme"]
fn committed_worktree_context_restores_and_fences_a_late_read() {
    let fixture = tempfile::tempdir().unwrap();
    let directory = fixture.path().canonicalize().unwrap();
    let repo = directory.join("repo");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    let first = commit_context(&repo, "start");
    git(&repo, &["switch", "--quiet", "-c", "review"]);
    let second = commit_context(&repo, "review");
    git(&repo, &["switch", "--quiet", "main"]);
    let nested = repo.join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(repo.join("index.org"), "dirty bytes must not be read").unwrap();

    let config = directory.join("tui.toml");
    fs::write(&config, format!(
        "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n[git_context]\nrepository={:?}\n",
        repo.display().to_string(),
    )).unwrap();
    let prefs = directory.join("worktrunk.toml");
    fs::write(
        &prefs,
        "worktree-path = \"{{ repo_path }}/../{{ repo }}-{{ branch }}\"\n",
    )
    .unwrap();
    let base = [
        ("GAMBOPT", "max-heap=2G,debug=q".to_owned()),
        ("WORKTRUNK_CONFIG_PATH", prefs.display().to_string()),
    ];
    for _ in 0..2 {
        let mut terminal = Terminal::start_in(&config, &[], &base, 30, 100, &nested);
        messages(&mut terminal);
        context_ready(&mut terminal, "start", &first);
        terminal.send(b"\x11");
        terminal.wait_exit(Duration::from_secs(5));
        terminal.assert_restored();
    }

    let tool = directory.join("bin");
    fs::create_dir(&tool).unwrap();
    let search = std::env::var_os("PATH").unwrap();
    let real_git = std::env::split_paths(&search)
        .map(|p| p.join("git"))
        .find(|p| p.is_file())
        .unwrap();
    let wrapper = tool.join("git");
    let started = directory.join("old-read-started");
    let release = directory.join("release-old-read");
    fs::write(
        &wrapper,
        format!(
            r#"#!/usr/bin/env python3
import os, pathlib, subprocess, sys, time
git = {git:?}
argv = sys.argv[1:]
if argv[-3:] == ["cat-file", "blob", {request:?}]:
    output = subprocess.check_output([git, *argv])
    pathlib.Path({started:?}).touch()
    while not pathlib.Path({release:?}).exists():
        time.sleep(0.01)
    sys.stdout.buffer.write(output)
else:
    os.execv(git, [git, *argv])
"#,
            git = real_git.to_str().unwrap(),
            request = format!("{first}:index.org"),
            started = started.to_str().unwrap(),
            release = release.to_str().unwrap()
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let search =
        std::env::join_paths(std::iter::once(tool).chain(std::env::split_paths(&search))).unwrap();
    let timed = [
        base[0].clone(),
        base[1].clone(),
        ("PATH", search.into_string().unwrap()),
    ];
    let mut terminal = Terminal::start_in(&config, &[], &timed, 30, 100, &nested);
    messages(&mut terminal);
    let deadline = Instant::now() + Duration::from_secs(15);
    while !started.exists() {
        assert!(Instant::now() < deadline, "old committed read never opened");
        thread::sleep(Duration::from_millis(20));
    }
    terminal.send(b"\x1bxworktree-switch review\r");
    context_ready(&mut terminal, "review", &second);
    fs::write(release, "").unwrap();
    terminal.send(b"\x1bxstatus\r");
    terminal.wait_text("Worktree clean");
    assert!(
        !screen_lines(&terminal.output)
            .join("\n")
            .contains("context.start.index.v1")
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}
