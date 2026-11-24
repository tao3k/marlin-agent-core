// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{Command, Duration, Instant, Terminal, fs, screen_lines};
use std::{
    env,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

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
    terminal.enter_normal_mode();
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

fn configure_context(config: &Path, approved: &Path) {
    fs::write(
        config,
        format!(
            "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n[git_context]\nrepository={approved:?}\n"
        ),
    )
    .unwrap();
}

fn enroll(config: &Path, home: &Path) -> String {
    enroll_with(Path::new(env!("CARGO_BIN_EXE_marlin-tui")), config, home)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

fn enroll_with(binary: &Path, config: &Path, home: &Path) -> PathBuf {
    let output = Command::new(binary)
        .args([config.as_os_str(), "--enroll-context".as_ref()])
        .env("HOME", home)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let root = String::from_utf8(output.stdout).unwrap();
    let path = Path::new(root.trim());
    assert!(path.starts_with(home.join("marlin-ai/repos")));
    path.to_owned()
}

#[test]
#[ignore = "requires the dynamic artifacts_workspace producer and a compiled Scheme package"]
fn linked_terminal_recovers_and_verifies_artifacts_without_a_scheme_install() {
    verify_linked_artifacts(false);
}

#[test]
#[ignore = "requires the dynamic artifacts_workspace producer and the linked DEV Scheme programme"]
fn linked_dev_terminal_recovers_primary_worktree_artifacts() {
    verify_linked_artifacts(true);
}

fn verify_linked_artifacts(development: bool) {
    let seed = env::var_os("MARLIN_ARTIFACT_FIXTURE_PRODUCER").expect("producer test binary");
    let binary = env::var_os("MARLIN_ARTIFACT_FIXTURE_TUI")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_marlin-tui")));
    let fixture = tempfile::tempdir().unwrap();
    let home = fixture.path().canonicalize().unwrap();
    let primary = home.join("primary");
    let branch = home.join("branch");
    fs::create_dir(&primary).unwrap();
    git(&primary, &["init", "-q", "--initial-branch", "main"]);
    fs::write(primary.join("file"), "base\n").unwrap();
    git(&primary, &["add", "file"]);
    let commit = commit_context(&primary, "main");
    git(
        &primary,
        &["worktree", "add", "--detach", branch.to_str().unwrap()],
    );
    fs::write(branch.join("file"), "updated\n").unwrap();
    let config = home.join("tui.toml");
    configure_context(&config, &primary);
    let root = if development {
        primary.join(".data/marlin")
    } else {
        enroll_with(&binary, &config, &home)
    };
    let mode = if development { "1" } else { "" };

    let output = Command::new(seed)
        .args(["--exact", "artifact_process", "--ignored", "--nocapture"])
        .env("MARLIN_ARTIFACT_TEST_DIRECTORY", &branch)
        .env("MARLIN_ARTIFACT_TEST_ACTION", "record")
        .env("HOME", &home)
        .env("MARLIN_AGENT_CORE_DEV", mode)
        .env("GAMBOPT", "max-heap=2G,debug=q")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let org = root.join("context.org");
    let original = fs::read(&org).unwrap();
    fs::remove_file(&org).unwrap();

    let absent = home.join("no-gerbil-install").display().to_string();
    let variables = [
        ("GAMBOPT", "max-heap=2G,debug=q".to_owned()),
        ("HOME", home.display().to_string()),
        ("MARLIN_AGENT_CORE_DEV", mode.to_owned()),
        ("GERBIL_HOME", absent.clone()),
        ("GERBIL_LOADPATH", absent.clone()),
        ("GERBIL_PATH", absent),
    ];
    let mut terminal =
        Terminal::start_in_binary(&binary, &config, &[], &variables, 30, 100, &primary);
    messages(&mut terminal);
    context_ready(&mut terminal, "main", &commit);
    terminal.wait_text("Git Artifacts restored");
    terminal.wait_text("2 verified file(s)");
    assert_eq!(fs::read(&org).unwrap(), original);
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();

    let artifact = fs::read_dir(root.join("artifacts"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(artifact, "corrupted payload").unwrap();
    let mut terminal =
        Terminal::start_in_binary(&binary, &config, &[], &variables, 30, 100, &branch);
    messages(&mut terminal);
    context_ready(&mut terminal, "main", &commit);
    terminal.wait_text("Git Artifacts unavailable");
    terminal.wait_text("Context artifact digest mismatch");
    assert_eq!(fs::read(org).unwrap(), original);
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[test]
fn unapproved_clone_is_refused_before_opening_context_blobs() {
    let fixture = tempfile::tempdir().unwrap();
    let directory = fixture.path().canonicalize().unwrap();
    let approved = directory.join("approved");
    let clone = directory.join("clone");
    let remote = "https://example.invalid/context/shared.git";
    fs::create_dir(&approved).unwrap();
    git(&approved, &["init", "-q", "-b", "main"]);
    let commit = commit_context(&approved, "shared");
    git(&approved, &["remote", "add", "origin", remote]);
    git(
        &directory,
        &["clone", "-q", "--no-local", "approved", "clone"],
    );
    git(&clone, &["remote", "set-url", "origin", remote]);
    assert_eq!(git(&clone, &["rev-parse", "HEAD"]), commit);
    assert_eq!(git(&clone, &["remote", "get-url", "origin"]), remote);

    let config = directory.join("tui.toml");
    let trace = directory.join("git-trace.jsonl");
    configure_context(&config, &approved);
    let variables = [
        ("GAMBOPT", "max-heap=2G,debug=q".to_owned()),
        ("HOME", directory.display().to_string()),
        ("MARLIN_AGENT_CORE_DEV", String::new()),
        ("GIT_TRACE2_EVENT", trace.display().to_string()),
    ];
    let mut terminal = Terminal::start_in(&config, &[], &variables, 30, 100, &approved);
    messages(&mut terminal);
    terminal.wait_text("Git Context unavailable");
    terminal.wait_text("--enroll-context");
    assert!(!directory.join("marlin-ai").exists());
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
    let owner = enroll(&config, &directory);
    assert_eq!(enroll(&config, &directory), owner);

    let mut terminal = Terminal::start_in(&config, &[], &variables, 30, 100, &clone);
    messages(&mut terminal);
    terminal.wait_text("Git Context unavailable");
    terminal.wait_text("approved in git_context");
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();

    let trace = fs::read_to_string(trace).unwrap();
    let mut observed_checkout = false;
    for entry in trace
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
    {
        let Some(arguments) = entry["argv"].as_array() else {
            continue;
        };
        observed_checkout |= arguments.iter().any(|argument| argument == "rev-parse");
        assert!(!arguments.iter().any(|argument| argument == "cat-file"));
    }
    assert!(
        observed_checkout,
        "Git trace must include checkout discovery"
    );
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
    configure_context(&config, &repo);
    let prefs = directory.join("worktrunk.toml");
    fs::write(
        &prefs,
        "worktree-path = \"{{ repo_path }}/../{{ repo }}-{{ branch }}\"\n",
    )
    .unwrap();
    let base = [
        ("GAMBOPT", "max-heap=2G,debug=q".to_owned()),
        ("WORKTRUNK_CONFIG_PATH", prefs.display().to_string()),
        ("HOME", directory.display().to_string()),
        ("MARLIN_AGENT_CORE_DEV", String::new()),
    ];
    let owner = enroll(&config, &directory);
    for _ in 0..2 {
        let mut terminal = Terminal::start_in(&config, &[], &base, 30, 100, &nested);
        messages(&mut terminal);
        context_ready(&mut terminal, "start", &first);
        terminal.wait_text(&owner);
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
# Apple's /usr/bin/git may look up Git through PATH.
environment = os.environ.copy()
environment["PATH"] = {search:?}
if argv[-3:] == ["cat-file", "blob", {request:?}]:
    output = subprocess.check_output([git, *argv], env=environment)
    pathlib.Path({started:?}).touch()
    while not pathlib.Path({release:?}).exists():
        time.sleep(0.01)
    sys.stdout.buffer.write(output)
else:
    os.execve(git, [git, *argv], environment)
"#,
            git = real_git.to_str().unwrap(),
            search = search.to_str().unwrap(),
            request = format!("{first}:index.org"),
            started = started.to_str().unwrap(),
            release = release.to_str().unwrap()
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let search =
        std::env::join_paths(std::iter::once(tool).chain(std::env::split_paths(&search))).unwrap();
    let mut timed = base.into_iter().collect::<Vec<_>>();
    timed.push(("PATH", search.into_string().unwrap()));
    let mut terminal = Terminal::start_in(&config, &[], &timed, 30, 100, &nested);
    messages(&mut terminal);
    wait_for_git_read(&mut terminal, &started, "old committed Git read");
    terminal.send(b"\x1bxworktree-switch review\r");
    context_ready(&mut terminal, "review", &second);
    terminal.wait_text(&owner);
    fs::write(&release, "").unwrap();
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

    fs::remove_file(&started).unwrap();
    fs::remove_file(&release).unwrap();
    let mut terminal = Terminal::start_in(&config, &[], &timed, 30, 100, &nested);
    messages(&mut terminal);
    wait_for_git_read(&mut terminal, &started, "Git read at the revocation fence");
    fs::remove_file(repo.join(".git/marlin-owner-ref")).unwrap();
    fs::write(release, "").unwrap();
    terminal.wait_text("Git Context unavailable");
    terminal.wait_text("--enroll-context");
    let rows = screen_lines(&terminal.output).join("\n");
    assert!(!rows.contains("Git Context ready"));
    assert!(!rows.contains("context.start.index.v1"));
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

fn wait_for_git_read(terminal: &mut Terminal, started: &Path, description: &str) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !started.exists() {
        if Instant::now() >= deadline {
            terminal.fail_wait(description, "Git fixture did not observe a committed read");
        }
        terminal.pump(Duration::from_millis(20));
    }
}
