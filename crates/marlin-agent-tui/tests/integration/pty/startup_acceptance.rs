// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    Arc, AtomicBool, Duration, Instant, StopOnDrop, TcpListener, Terminal, contains, fs, mpsc,
    screen_lines, stream_fixture, thread,
};

#[cfg(feature = "linked-ui")]
#[test]
#[ignore = "manual paired startup benchmark; build the current Release binary first"]
fn release_linked_fresh_copy_startup_pair() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = root.join("target/release/marlin-tui");
    linked_fresh_copy_startup_pair(&binary, "Release");
}

#[cfg(feature = "linked-ui")]
#[test]
#[ignore = "manual paired startup benchmark; build the current debug binary first"]
fn debug_linked_fresh_copy_startup_pair() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = root.join("target/debug/marlin-tui");
    linked_fresh_copy_startup_pair(&binary, "debug");
}

#[cfg(feature = "linked-ui")]
fn linked_fresh_copy_startup_pair(binary: &std::path::Path, profile: &str) {
    assert!(binary.is_file(), "build the linked {profile} binary first");
    let directory = tempfile::tempdir().unwrap();
    let copied = directory.path().join("marlin-tui-copy");
    fs::copy(binary, &copied).expect("copy linked binary");
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let absent = directory.path().join("no-gerbil-install");
    let absent = absent.to_string_lossy().into_owned();
    let mut observations = Vec::with_capacity(2);
    for _ in 0..2 {
        let mut terminal = Terminal::start_in_binary(
            &copied,
            &config,
            &[],
            &[
                ("GERBIL_HOME", absent.clone()),
                ("GERBIL_PATH", absent.clone()),
                ("GERBIL_LOADPATH", absent.clone()),
                ("MARLIN_TUI_STARTUP_TRACE", "1".to_owned()),
            ],
            30,
            100,
            directory.path(),
        );
        terminal.wait_for(Duration::from_secs(10), "Release startup pair", |output| {
            contains(output, b"\x1b[?1049h")
                && screen_lines(output)[29].contains("Message: Enter send")
                && screen_lines(output)
                    .iter()
                    .any(|line| line.contains("MARLIN"))
        });
        observations.push((
            terminal.first_output_elapsed.expect("first PTY output"),
            terminal.start.elapsed(),
        ));
        terminal.send(b"\x11");
        terminal.wait_exit(Duration::from_secs(5));
        terminal.assert_restored();
        for line in String::from_utf8_lossy(&terminal.output).split("\r\n") {
            if line.starts_with("marlin-tui startup ") {
                eprintln!("{profile} {line}");
            }
        }
    }
    eprintln!(
        "linked {profile} fresh copy: first output {:?}, workbench {:?}; same file second run: first output {:?}, workbench {:?}",
        observations[0].0, observations[0].1, observations[1].0, observations[1].1
    );
}

#[test]
#[ignore = "diagnostic only: longer windows do not satisfy the acceptance gate"]
fn menu_paste_stream_diagnostic() {
    exercise_pty(Duration::from_secs(90), Duration::from_secs(45));
}

pub(super) fn exercise_pty(startup_limit: Duration, exit_limit: Duration) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback fixture");
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (release_tx, release_rx) = mpsc::channel();
    let (request_tx, request_rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let server = thread::spawn(move || stream_fixture(listener, release_rx, request_tx, stop));
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, format!("provider = \"openai\"\nmodel = \"gpt-fixture\"\nendpoint = \"http://{address}/v1/responses\"\nprotocol = \"open-ai-responses\"\ncredential_env = \"MARLIN_TUI_FIXTURE_KEY\"\n")).unwrap();
    let mut terminal = Terminal::start(&config);
    let remaining = startup_limit.saturating_sub(terminal.start.elapsed());
    terminal.wait_for(remaining, "workbench", |output| {
        contains(output, b"\x1b[?1049h")
            && screen_lines(output)[28].contains("MARLIN")
            && screen_lines(output)[29].contains("Message: Enter send")
    });
    assert!(terminal.start.elapsed() < startup_limit);
    eprintln!("workbench ready after {:?}", terminal.start.elapsed());
    if std::env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some() {
        for line in String::from_utf8_lossy(&terminal.output).split("\r\n") {
            if line.starts_with("marlin-tui startup ")
                || line.starts_with("marlin-tui UI resolve ")
                || line.starts_with("marlin-tui UI open ")
            {
                eprintln!("{line}");
            }
        }
    }
    assert!(!contains(&terminal.output, b"GERBIL-BUILD"));
    terminal.send(b"\x1b[Z");
    terminal.wait_status("Transcript: scroll to browse");
    terminal.send(b"\t");
    terminal.wait_status("Message: Enter send");
    terminal.send(b"\x1bx");
    terminal.wait_mx_selected(1);
    terminal.send(b"\x1b[A");
    terminal.wait_mx_last();
    terminal.send(b"\x1b[B");
    terminal.wait_mx_selected(1);
    terminal.send(b"\r");
    terminal.wait_text("*Help*");
    terminal.send(b"\x1b");
    terminal.wait_status("Message: Enter send");
    terminal.send(b"\x1bxsessions\r");
    terminal.wait_text("Session listing requires a persistent session configuration.");
    terminal.wait_status("Message: Enter send");
    assert!(
        matches!(request_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "command sent a model request"
    );
    terminal.send(
        "\x1b[200~hello 中\r\nworld\r\nline-three\r\nline-four\r\nline-five\x1b[201~".as_bytes(),
    );
    terminal.wait_raw(b"line-five");
    terminal.send(b"\t");
    terminal.wait_status("Commands: Enter open");
    terminal.send(b"\r");
    terminal.wait_mx_selected(1);
    terminal.send(b"\x07");
    terminal.wait_status("Commands: Enter open");
    terminal.send(b"\t");
    terminal.wait_status("Transcript: scroll to browse");
    terminal.send(b"\t");
    terminal.wait_status("Message: Enter send");
    terminal.wait_for(
        Duration::from_secs(5),
        "cursor after long draft",
        |output| {
            output
                .windows(b"line-five".len())
                .rposition(|window| window == b"line-five")
                .is_some_and(|start| contains(&output[start..], b"\x1b[?25h"))
        },
    );
    assert!(
        matches!(request_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "paste submitted before Enter"
    );
    terminal.send(b"\x1b[A!\x1b[B?\x01^\x05$\x1b[H<\x1b[F>");
    thread::sleep(Duration::from_millis(300));
    terminal.send(b"\r");
    terminal.wait_raw(b"fixture-live");
    let request = request_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        request["input"][0]["content"],
        "hello 中\nworld\nline-three\nline-four!\n<^line-five?$>"
    );
    assert_eq!(request["stream"], true);
    release_tx.send(()).unwrap();
    terminal.wait_raw(b"done");
    terminal.send(b"\x11");
    terminal.wait_exit(exit_limit);
    terminal.assert_restored();
    server.join().expect("fixture server");
}

#[test]
#[ignore = "real PTY acceptance; requires the built Scheme package"]
fn ctrl_c_confirmation_restores_terminal() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"unsent-draft\x03");
    terminal.wait_status("Press Ctrl-C again to quit");
    assert!(
        terminal.child.try_wait().unwrap().is_none(),
        "first Ctrl-C exited"
    );
    terminal.send(b"\x03");
    terminal.wait_exit(Duration::from_secs(10));
    terminal.assert_restored();
}

#[test]
#[ignore = "idle-output observation; requires the built Scheme package"]
fn idle_output_probe() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:9/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    for trial in 1..=3 {
        let mut terminal = Terminal::start(&config);
        terminal.wait_status("Message: Enter send");
        let settle = Instant::now() + Duration::from_millis(600);
        while let Ok(chunk) = terminal
            .received
            .recv_timeout(settle.saturating_duration_since(Instant::now()))
        {
            terminal.output.extend_from_slice(&chunk);
            if Instant::now() >= settle {
                break;
            }
        }
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut bytes = 0usize;
        while let Ok(chunk) = terminal
            .received
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            bytes += chunk.len();
            terminal.output.extend_from_slice(&chunk);
            if Instant::now() >= deadline {
                break;
            }
        }
        terminal.send(b"\x11");
        terminal.wait_exit(Duration::from_secs(5));
        println!("trial {trial}: {bytes} idle bytes");
    }
}

#[test]
#[ignore = "real PTY acceptance; requires the built Scheme package"]
fn runtime_policy_receipt() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:9/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\ntimeout_seconds=7\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"/runtime\r");
    for text in [
        "Stream-open timeout: 7000 ms",
        "does not cover stream duration",
    ] {
        terminal.wait_for(Duration::from_secs(15), text, |output| {
            screen_lines(output).join(" ").contains(text)
        });
    }
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
}

#[test]
#[ignore = "real PTY acceptance; requires the built Scheme package"]
fn missing_session_configuration_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:9/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\ntimeout_seconds=7\n").unwrap();
    let mut invalid = Terminal::start_with_args(&config, &["--session", "session:missing"]);
    invalid.wait_raw(b"--session requires a [session] storage configuration");
    assert_eq!(
        invalid.wait_exit_status(Duration::from_secs(5)).code(),
        Some(1)
    );
}
