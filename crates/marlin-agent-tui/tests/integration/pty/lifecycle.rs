// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    Command, Duration, File, Instant, Read, Stdio, Terminal, Winsize, contains, mpsc, openpty,
    screen_lines, tcgetattr, thread,
};

fn startup_timeout() -> Duration {
    #[cfg(feature = "linked-ui")]
    {
        Duration::from_secs(10)
    }
    #[cfg(not(feature = "linked-ui"))]
    {
        Duration::from_secs(30)
    }
}

fn exit_timeout() -> Duration {
    #[cfg(feature = "linked-ui")]
    {
        Duration::from_secs(5)
    }
    #[cfg(not(feature = "linked-ui"))]
    {
        Duration::from_secs(15)
    }
}

#[test]
fn example_config_starts_stays_alive_and_exits_cleanly_offline() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = root.join("crates/marlin-agent-tui/example.toml");
    let polluted_gerbil = tempfile::tempdir().expect("polluted gerbil dir");
    let polluted = polluted_gerbil.path().to_string_lossy().into_owned();

    let mut terminal = Terminal::start_in(
        &config,
        &[],
        &[
            (
                "ANTHROPIC_BASE_URL",
                "http://127.0.0.1:1/anthropic".to_owned(),
            ),
            ("ANTHROPIC_API_KEY", "fixture-key".to_owned()),
            ("GERBIL_PATH", polluted),
            ("MARLIN_TUI_STARTUP_TRACE", "1".to_owned()),
        ],
        30,
        100,
        &root,
    );

    terminal.wait_for(startup_timeout(), "example.toml first frame", |output| {
        let rows = screen_lines(output);
        contains(output, b"\x1b[?1049h")
            && rows[29].contains("Message: Enter send")
            && rows
                .iter()
                .any(|line| line.contains("MARLIN") && line.contains("deepseek/deepseek-flash"))
    });

    // Exercise resize (SIGWINCH) and steady-state liveness across the
    // post-first-frame window while background worktree/envrc/index tasks run.
    terminal.resize(32, 110);
    terminal.pump(Duration::from_millis(1200));
    terminal.resize(30, 100);
    terminal.pump(Duration::from_millis(1300));
    assert!(
        terminal
            .child
            .try_wait()
            .expect("poll steady-state child")
            .is_none(),
        "marlin-tui exited unexpectedly during steady-state window; output tail: {:?}",
        String::from_utf8_lossy(&terminal.output[terminal.output.len().saturating_sub(1200)..])
    );

    terminal.send(b"\x11");
    terminal.wait_exit(exit_timeout());
    terminal.assert_restored();
    let trace = String::from_utf8_lossy(&terminal.output);
    assert!(
        trace.contains("event loop exit (ctrl-q,"),
        "expected explicit ctrl-q exit trace; output tail: {:?}",
        &trace[trace.len().saturating_sub(800)..]
    );
}

#[test]
fn unexpected_stdin_eof_restores_terminal_and_reports_diagnostic() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = root.join("crates/marlin-agent-tui/example.toml");
    let size = Winsize {
        ws_row: 30,
        ws_col: 100,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let stdin_pair = openpty(Some(&size), None).expect("open stdin PTY");
    let io_pair = openpty(Some(&size), None).expect("open stdout/stderr PTY");
    let stdin_master = File::from(stdin_pair.master);
    let stdin_slave = File::from(stdin_pair.slave);
    let io_master = File::from(io_pair.master);
    let io_slave = File::from(io_pair.slave);
    {
        use std::os::fd::AsRawFd;
        // Prevent the spawned child from inheriting PTY masters; otherwise
        // dropping `stdin_master` in the parent cannot deliver EOF to stdin.
        for fd in [stdin_master.as_raw_fd(), io_master.as_raw_fd()] {
            // SAFETY: `fd` is a valid open file descriptor owned by this test.
            let rc = unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
            assert_eq!(rc, 0, "set FD_CLOEXEC on PTY master");
        }
    }
    let initial_mode = tcgetattr(&io_slave).expect("capture initial terminal mode");

    let mut reader = io_master.try_clone().expect("clone io PTY master");
    let (sender, received) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = [0; 8192];
        while let Ok(count) = reader.read(&mut buffer) {
            if count == 0 || sender.send(buffer[..count].to_vec()).is_err() {
                break;
            }
        }
    });

    let mut command = Command::new(env!("CARGO_BIN_EXE_marlin-tui"));
    command
        .current_dir(&root)
        .arg(&config)
        .env("ANTHROPIC_BASE_URL", "http://127.0.0.1:1/anthropic")
        .env("ANTHROPIC_API_KEY", "fixture-key")
        .env("MARLIN_TUI_STARTUP_TRACE", "1")
        .env_remove("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
        .env_remove("MARLIN_TUI_HOST_OBSERVATIONS_FILE")
        .stdin(Stdio::from(stdin_slave))
        .stdout(Stdio::from(io_slave.try_clone().expect("clone stdout")))
        .stderr(Stdio::from(io_slave.try_clone().expect("clone stderr")));

    let mut child = command.spawn().expect("spawn marlin-tui with split PTYs");
    let mut output = Vec::new();
    let deadline = Instant::now() + startup_timeout();
    while !contains(&output, b"\x1b[?1049h")
        || !screen_lines(&output)
            .iter()
            .any(|line| line.contains("MARLIN"))
    {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "timed out waiting for first frame; status={:?}; tail={:?}",
            child.try_wait(),
            String::from_utf8_lossy(&output[output.len().saturating_sub(800)..])
        );
        match received.recv_timeout(remaining) {
            Ok(chunk) => output.extend_from_slice(&chunk),
            Err(error) => panic!("failed waiting for first frame: {error:?}"),
        }
    }

    // Close the master end of the stdin PTY to trigger a real EOF on stdin.
    drop(stdin_master);

    let exit_limit = exit_timeout();
    let exit_deadline = Instant::now() + exit_limit;
    let status = loop {
        while let Ok(chunk) = received.try_recv() {
            output.extend_from_slice(&chunk);
        }
        if let Some(status) = child.try_wait().expect("poll child after stdin EOF") {
            break status;
        }
        assert!(
            Instant::now() < exit_deadline,
            "marlin-tui did not exit within {exit_limit:?} after stdin EOF; output tail: {:?}",
            String::from_utf8_lossy(&output[output.len().saturating_sub(1200)..])
        );
        thread::sleep(Duration::from_millis(20));
    };
    while let Ok(chunk) = received.recv_timeout(Duration::from_millis(100)) {
        output.extend_from_slice(&chunk);
    }

    assert_eq!(
        status.code(),
        Some(1),
        "unexpected stdin EOF must exit non-zero (not silent 0); output tail: {:?}",
        String::from_utf8_lossy(&output[output.len().saturating_sub(800)..])
    );
    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("terminal input closed unexpectedly (stdin EOF)"),
        "expected explicit stdin EOF diagnostic; output tail: {:?}",
        &text[text.len().saturating_sub(800)..]
    );
    assert!(
        text.contains("event loop exit (stdin-eof,"),
        "expected stdin-eof trace marker; output tail: {:?}",
        &text[text.len().saturating_sub(800)..]
    );
    assert!(
        contains(&output, b"\x1b[?1049l"),
        "alternate screen must be exited on stdin EOF"
    );
    let restored = tcgetattr(&io_slave).expect("capture restored terminal mode");
    #[cfg(target_os = "macos")]
    let (restored, initial_mode) = {
        let strip_pendin = |mode: nix::sys::termios::Termios| {
            let mut raw: nix::libc::termios = mode.into();
            raw.c_lflag &= !nix::libc::PENDIN;
            nix::sys::termios::Termios::from(raw)
        };
        (strip_pendin(restored), strip_pendin(initial_mode))
    };
    assert_eq!(
        restored, initial_mode,
        "terminal mode must be restored on stdin EOF"
    );
    drop(io_master);
}
