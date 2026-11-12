// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real Unix PTY, streaming provider, startup and terminal restoration gate.
#![cfg(unix)]

use nix::{
    pty::{Winsize, openpty},
    sys::signal::{Signal, kill},
    sys::termios::tcgetattr,
    unistd::Pid,
};
use std::{
    fs::{self, File},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};
use unicode_width::UnicodeWidthChar;

#[path = "pty/live_provider.rs"]
mod live_provider;
#[path = "pty/session_replay.rs"]
mod session_replay;

struct StopOnDrop(Arc<AtomicBool>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

fn accept_until(listener: &TcpListener, stop: &AtomicBool) -> Option<TcpStream> {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => return Some(stream),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => panic!("fixture accept: {error}"),
        }
    }
    None
}

struct Terminal {
    child: Child,
    master: File,
    slave: File,
    received: Receiver<Vec<u8>>,
    output: Vec<u8>,
    start: Instant,
    initial_mode: nix::sys::termios::Termios,
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

impl Terminal {
    fn start(config: &std::path::Path) -> Self {
        Self::start_with_args(config, &[])
    }

    fn start_with_args(config: &std::path::Path, args: &[&str]) -> Self {
        Self::start_with_env(config, args, &[])
    }

    fn start_with_env(
        config: &std::path::Path,
        args: &[&str],
        variables: &[(&str, String)],
    ) -> Self {
        Self::start_sized(config, args, variables, 30, 100)
    }

    fn start_sized(
        config: &std::path::Path,
        args: &[&str],
        variables: &[(&str, String)],
        rows: u16,
        columns: u16,
    ) -> Self {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        Self::start_in(config, args, variables, rows, columns, &root)
    }

    fn start_in(
        config: &std::path::Path,
        args: &[&str],
        variables: &[(&str, String)],
        rows: u16,
        columns: u16,
        working_directory: &std::path::Path,
    ) -> Self {
        let size = Winsize {
            ws_row: rows,
            ws_col: columns,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let pair = openpty(Some(&size), None).expect("open PTY");
        let master = File::from(pair.master);
        let slave = File::from(pair.slave);
        let initial_mode = tcgetattr(&slave).expect("capture terminal mode");
        let mut reader = master.try_clone().expect("clone PTY master");
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
            .current_dir(working_directory)
            .arg(config)
            .args(args)
            .env("MARLIN_TUI_FIXTURE_KEY", "fixture")
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave.try_clone().unwrap()));
        command.envs(variables.iter().map(|(name, value)| (*name, value)));
        if std::env::var_os("MARLIN_PTY_LOCAL_PACKAGE").as_deref()
            == Some(std::ffi::OsStr::new("1"))
        {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            if working_directory.canonicalize().ok() == root.canonicalize().ok() {
                command.env_remove("GERBIL_PATH");
            } else {
                command.env("GERBIL_PATH", root.join("scheme/.gerbil"));
            }
            command.env_remove("GERBIL_LOADPATH");
        }
        let start = Instant::now();
        let child = command.spawn().expect("start TUI");
        Self {
            child,
            master,
            slave,
            received,
            output: Vec::new(),
            start,
            initial_mode,
        }
    }

    fn send(&mut self, input: &[u8]) {
        self.master.write_all(input).expect("write PTY");
    }

    fn resize(&self, rows: u16, columns: u16) {
        rustix::termios::tcsetwinsize(
            &self.slave,
            rustix::termios::Winsize {
                ws_row: rows,
                ws_col: columns,
                ws_xpixel: 0,
                ws_ypixel: 0,
            },
        )
        .expect("resize PTY");
    }

    fn pump(&mut self, duration: Duration) {
        let deadline = Instant::now() + duration;
        while let Ok(chunk) = self
            .received
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            self.output.extend_from_slice(&chunk);
            if Instant::now() >= deadline {
                break;
            }
        }
    }

    fn wait_for(
        &mut self,
        timeout: Duration,
        description: &str,
        predicate: impl Fn(&[u8]) -> bool,
    ) {
        let deadline = Instant::now() + timeout;
        loop {
            if predicate(&self.output) {
                return;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "timed out waiting for {description}; screen: {:?}; output tail: {:?}",
                screen_lines(&self.output),
                String::from_utf8_lossy(&self.output[self.output.len().saturating_sub(1200)..])
            );
            match self.received.recv_timeout(remaining) {
                Ok(chunk) => self.output.extend_from_slice(&chunk),
                Err(mpsc::RecvTimeoutError::Timeout) => panic!(
                    "timed out waiting for {description}; screen: {:?}; output tail: {:?}",
                    screen_lines(&self.output),
                    String::from_utf8_lossy(&self.output[self.output.len().saturating_sub(1200)..])
                ),
                Err(error) => panic!("PTY ended waiting for {description}: {error}"),
            }
        }
    }

    fn wait_status(&mut self, expected: &str) {
        self.wait_for(Duration::from_secs(15), expected, |output| {
            screen_lines(output)[29].contains(expected)
        });
    }

    fn wait_text(&mut self, expected: &str) {
        self.wait_for(Duration::from_secs(15), expected, |output| {
            screen_lines(output)
                .iter()
                .any(|line| line.contains(expected))
        });
    }

    fn wait_raw(&mut self, expected: &[u8]) {
        self.wait_for(Duration::from_secs(15), "terminal output", |output| {
            contains(output, expected)
        });
    }

    fn wait_exit(&mut self, timeout: Duration) {
        let status = self.wait_exit_status(timeout);
        assert!(status.success(), "TUI exited with {status}");
    }

    fn wait_exit_status(&mut self, timeout: Duration) -> std::process::ExitStatus {
        let deadline = Instant::now() + timeout;
        let status = loop {
            while let Ok(chunk) = self.received.try_recv() {
                self.output.extend_from_slice(&chunk);
            }
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "TUI did not exit within {timeout:?}; output tail: {:?}",
                String::from_utf8_lossy(&self.output[self.output.len().saturating_sub(800)..])
            );
            thread::sleep(Duration::from_millis(20));
        };
        while let Ok(chunk) = self.received.recv_timeout(Duration::from_millis(100)) {
            self.output.extend_from_slice(&chunk);
        }
        if std::env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some() {
            for line in String::from_utf8_lossy(&self.output).split("\r\n") {
                for marker in [
                    "marlin-tui startup event loop exit",
                    "marlin-tui startup terminal restored",
                    "marlin-tui startup gateway dropped",
                    "marlin-tui startup Tokio runtime dropped",
                    "marlin-tui startup UI session dropped",
                    "marlin-tui startup Scheme VM dropped",
                ] {
                    if let Some(index) = line.find(marker) {
                        eprintln!("{}", line[index..].split('\x1b').next().unwrap_or(""));
                    }
                }
            }
        }
        status
    }

    fn assert_restored(&self) {
        assert_eq!(
            tcgetattr(&self.slave).unwrap(),
            self.initial_mode,
            "terminal mode not restored"
        );
        for sequence in [
            b"\x1b[?1049h".as_slice(),
            b"\x1b[?1049l",
            b"\x1b[?2004h",
            b"\x1b[?2004l",
        ] {
            assert!(
                contains(&self.output, sequence),
                "missing terminal mode transition"
            );
        }
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

// Ratatui sends cell deltas rather than contiguous strings. Reconstruct the
// screen from cursor operations so checks do not depend on escape chunking.
fn screen_lines(output: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(output);
    let mut chars = text.chars().peekable();
    let mut rows = vec![vec![' '; 100]; 30];
    let (mut row, mut col) = (0usize, 0usize);
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            let mut args = String::new();
            let op = loop {
                match chars.next() {
                    Some(c) if c.is_ascii_alphabetic() => break Some(c),
                    Some(c) => args.push(c),
                    None => break None,
                }
            };
            let nums: Vec<usize> = args
                .trim_start_matches('?')
                .split(';')
                .map(|n| n.parse().unwrap_or(0))
                .collect();
            let n = nums.first().copied().unwrap_or(0).max(1);
            match op {
                Some('h') if args == "?1049" => {
                    for line in &mut rows {
                        line.fill(' ');
                    }
                    row = 0;
                    col = 0;
                }
                Some('H' | 'f') => {
                    row = n - 1;
                    col = nums.get(1).copied().unwrap_or(1).max(1) - 1;
                }
                Some('G') => col = n - 1,
                Some('A') => row = row.saturating_sub(n),
                Some('B') => row = row.saturating_add(n),
                Some('C') => col = col.saturating_add(n),
                Some('D') => col = col.saturating_sub(n),
                Some('J') if nums.first().copied() == Some(2) => {
                    for line in &mut rows {
                        line.fill(' ');
                    }
                }
                Some('K') if row < 30 => {
                    let mode = nums.first().copied().unwrap_or(0);
                    let start = if mode == 1 || mode == 2 {
                        0
                    } else {
                        col.min(100)
                    };
                    let end = if mode == 1 { (col + 1).min(100) } else { 100 };
                    rows[row][start..end].fill(' ');
                }
                _ => {}
            }
        } else if ch == '\r' {
            col = 0;
        } else if ch == '\n' {
            row = row.saturating_add(1);
        } else if !ch.is_control() {
            let width = ch.width().unwrap_or(0);
            if row < 30 && col < 100 && width > 0 {
                rows[row][col] = ch;
            }
            col = col.saturating_add(width);
        }
    }
    rows.into_iter()
        .map(|row| row.into_iter().collect())
        .collect()
}

#[test]
fn terminal_screen_reconstructs_partial_cell_updates() {
    let output = b"\x1b[3;1Hanswer-two\x1b[3;1Han\x1b[3;4Hwer-one\x1b[30;2HMessage: Enter send";
    let rows = screen_lines(output);
    assert!(rows[2].starts_with("answer-one"));
    assert!(rows[29].contains("Message: Enter send"));
    let cleared = screen_lines(b"\x1b[3;1Hstale\x1b[2J\x1b[3;1Hfresh");
    assert!(cleared[2].starts_with("fresh"));
    assert!(!cleared[2].contains("stale"));
    let alternate = screen_lines(b"startup\r\n\x1b[?1049h\x1b[1;1Hworkbench");
    assert!(alternate[0].starts_with("workbench"));
    assert!(!alternate.iter().any(|line| line.contains("startup")));
}

fn read_request(stream: &mut TcpStream) -> (String, serde_json::Value) {
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    let mut input = Vec::new();
    let mut chunk = [0; 4096];
    let (body_start, content_length) = loop {
        let count = stream.read(&mut chunk).expect("read fixture headers");
        assert!(count > 0, "provider closed before headers");
        input.extend_from_slice(&chunk[..count]);
        if let Some(index) = input.windows(4).position(|w| w == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&input[..index]);
            let length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .expect("content length");
            break (index + 4, length);
        }
    };
    while input.len() < body_start + content_length {
        let count = stream.read(&mut chunk).expect("read fixture body");
        assert!(count > 0, "provider closed before body");
        input.extend_from_slice(&chunk[..count]);
    }
    let headers = String::from_utf8(input[..body_start - 4].to_vec()).unwrap();
    let body = serde_json::from_slice(&input[body_start..body_start + content_length]).unwrap();
    (headers, body)
}

fn stream_fixture(
    listener: TcpListener,
    release: Receiver<()>,
    requests: mpsc::Sender<serde_json::Value>,
    stop: Arc<AtomicBool>,
) {
    let Some(mut stream) = accept_until(&listener, &stop) else {
        return;
    };
    let (_, body) = read_request(&mut stream);
    requests.send(body).unwrap();
    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"fixture-live\"}\n\n").unwrap();
    release
        .recv_timeout(Duration::from_secs(10))
        .expect("release streaming fixture");
    stream.write_all(b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"done\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"fixture\",\"model\":\"gpt-fixture\",\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"fixture-livedone\"}]}]}}\n\n").unwrap();
}

#[test]
#[ignore = "real PTY and loopback acceptance; requires the built Scheme package"]
fn startup_menu_paste_stream_and_terminal_restoration() {
    exercise_pty(Duration::from_secs(10), Duration::from_secs(5));
}

#[cfg(feature = "linked-ui")]
#[test]
fn linked_ui_opens_without_a_gerbil_package_tree() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let absent = directory.path().join("no-gerbil-install");
    let absent = absent.to_string_lossy().into_owned();
    let mut terminal = Terminal::start_with_env(
        &config,
        &[],
        &[
            ("GERBIL_HOME", absent.clone()),
            ("GERBIL_PATH", absent.clone()),
            ("GERBIL_LOADPATH", absent),
        ],
    );
    terminal.wait_for(Duration::from_secs(10), "linked workbench", |output| {
        contains(output, b"\x1b[?1049h") && screen_lines(output)[29].contains("Message: Enter send")
    });
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[test]
#[ignore = "diagnostic only: longer windows do not satisfy the acceptance gate"]
fn menu_paste_stream_diagnostic() {
    exercise_pty(Duration::from_secs(90), Duration::from_secs(45));
}

fn exercise_pty(startup_limit: Duration, exit_limit: Duration) {
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
            && contains(output, b"Marlin")
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
    terminal.wait_status("Transcript: Up/Down scroll");
    terminal.send(b"\t");
    terminal.wait_status("Message: Enter send");
    terminal.send(b"/");
    terminal.wait_status("Command: Enter run");
    terminal.send(b"\x1b[A");
    terminal.wait_status("Selected: /clear");
    terminal.send(b"\x1b[B");
    terminal.wait_status("Selected: /help");
    terminal.send(b"\r");
    terminal.wait_status("Message: Enter send");
    terminal.send(b"/sessions");
    terminal.wait_status("Command: Enter run");
    terminal.send(b"\x1b[B");
    terminal.wait_status("Selected: /sessions");
    terminal.send(b"\r");
    terminal.wait_text("Session listing requires a persistent session configuration.");
    terminal.wait_status("Message: Enter send");
    terminal.send(b"/hel");
    terminal.wait_status("Command: Enter run");
    terminal.wait_raw(b"Keyboard shortcuts");
    terminal.send(b"\t\r");
    terminal.wait_status("Message: Enter send");
    terminal.wait_raw(b"stream-open");
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
    terminal.wait_status("Enter run command");
    terminal.wait_raw(b"Keyboard shortcuts");
    terminal.send(b"\x0f\t");
    terminal.wait_status("Transcript: Up/Down scroll");
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

#[test]
#[ignore = "real PTY and loopback acceptance; requires the built Scheme package"]
fn anthropic_base_url_and_two_turns() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let server = thread::spawn(move || {
        for answer in ["fixture-connected", "fixture-second"] {
            let Some(mut stream) = accept_until(&listener, &stop) else {
                return;
            };
            let request = read_request(&mut stream);
            sender.send(request).unwrap();
            let events = [
                serde_json::json!({"type":"message_start", "message":{"id":"fixture", "model":"deepseek-v4-pro[1m]", "usage":{"input_tokens":1,"output_tokens":0}}}),
                serde_json::json!({"type":"content_block_start", "index":0, "content_block":{"type":"text", "text":""}}),
                serde_json::json!({"type":"content_block_delta", "index":0, "delta":{"type":"text_delta", "text":answer}}),
                serde_json::json!({"type":"content_block_stop", "index":0}),
                serde_json::json!({"type":"message_delta", "delta":{"stop_reason":"end_turn"}, "usage":{"output_tokens":1}}),
                serde_json::json!({"type":"message_stop"}),
            ];
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n").unwrap();
            for event in events {
                write!(stream, "data: {event}\n\n").unwrap();
            }
        }
    });
    let config = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("example.toml");
    let variables = [
        ("ANTHROPIC_BASE_URL", format!("http://{address}/anthropic")),
        ("ANTHROPIC_API_KEY", "fixture-key".into()),
    ];
    let mut terminal = Terminal::start_with_env(&config, &[], &variables);
    terminal.wait_for(Duration::from_secs(20), "workbench", |output| {
        screen_lines(output)[29].contains("Message: Enter send")
    });
    terminal.send(b"hello\r");
    terminal.wait_for(
        Duration::from_secs(20),
        "first provider response",
        |output| contains(output, b"fixture-connected"),
    );
    thread::sleep(Duration::from_millis(500));
    terminal.send(b"follow-up\r");
    terminal.wait_for(
        Duration::from_secs(20),
        "second provider response",
        |output| contains(output, b"fixture-second"),
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    let (header, first) = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    let (_, second) = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(header.starts_with("POST /anthropic/v1/messages "));
    assert!(
        header
            .to_ascii_lowercase()
            .contains("x-api-key: fixture-key")
    );
    assert_eq!(first["model"], "deepseek-v4-pro[1m]");
    assert_eq!(first["messages"][0]["content"], "hello");
    assert_eq!(second["messages"][0]["content"], "hello");
    assert_eq!(
        second["messages"][1]["content"][0]["text"],
        "fixture-connected"
    );
    assert_eq!(second["messages"][2]["content"], "follow-up");
    server.join().unwrap();
}

#[test]
#[ignore = "real PTY and loopback flood; requires the built Scheme package"]
fn streaming_flood_can_be_cancelled() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (connected_tx, connected_rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let server_stop = stop.clone();
    let server = thread::spawn(move || {
        let Some(mut stream) = accept_until(&listener, &server_stop) else {
            return;
        };
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let _ = read_request(&mut stream);
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
        connected_tx.send(()).unwrap();
        let events =
            b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"PIPE \"}\n\n".repeat(64);
        while !server_stop.load(Ordering::Relaxed) {
            if stream.write_all(&events).is_err() {
                break;
            }
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    let mut body = format!(
        "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n"
    );
    let persistent =
        std::env::var_os("MARLIN_PTY_PERSIST").as_deref() == Some(std::ffi::OsStr::new("1"));
    if persistent {
        let path =
            serde_json::to_string(&directory.path().join("session.db").to_string_lossy()).unwrap();
        body.push_str(&format!("[session]\npath={path}\nproject=\"project:flood\"\nsession=\"session:flood\"\nagent=\"agent:flood\"\n"));
    }
    fs::write(&config, body).unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"flood test\r");
    connected_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("provider connection");
    terminal.wait_for(Duration::from_secs(10), "stream output", |output| {
        contains(output, b"PIPE")
    });
    let started = Instant::now();
    terminal.send(b"\x03");
    terminal.wait_for(Duration::from_secs(10), "local cancellation", |output| {
        screen_lines(output)
            .join(" ")
            .contains("Request cancelled locally.")
    });
    eprintln!(
        "local cancellation after {:?}; persistent={persistent}",
        started.elapsed()
    );
    stop.store(true, Ordering::Relaxed);
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
    server.join().unwrap();
}

#[test]
#[ignore = "persistent PTY and loopback acceptance; requires the built Scheme package"]
fn uncertain_open_remains_unresolved_after_restart() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let server_stop = stop.clone();
    let (sender, receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let mut open_streams = Vec::new();
        while !server_stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let (_, request) = read_request(&mut stream);
                    sender.send(request).unwrap();
                    open_streams.push(stream);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(20))
                }
                Err(error) => panic!("fixture accept: {error}"),
            }
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    let path =
        serde_json::to_string(&directory.path().join("session.db").to_string_lossy()).unwrap();
    fs::write(&config, format!("provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\ntimeout_seconds=1\n[session]\npath={path}\nproject=\"project:uncertain\"\nsession=\"session:uncertain\"\nagent=\"agent:uncertain\"\n")).unwrap();
    let mut first = Terminal::start(&config);
    first.wait_for(Duration::from_secs(20), "workbench", |output| {
        screen_lines(output)[29].contains("Message: Enter send")
    });
    first.send(b"first\r");
    first.wait_for(
        Duration::from_secs(20),
        "unknown remote outcome",
        |output| {
            screen_lines(output)
                .join(" ")
                .contains("remote outcome is unknown")
        },
    );
    let request = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(request["input"][0]["content"], "first");
    first.send(b"\x11");
    first.wait_exit(Duration::from_secs(10));
    drop(first);

    let mut reopened = Terminal::start(&config);
    reopened.wait_for(
        Duration::from_secs(20),
        "unresolved previous turn",
        |output| {
            screen_lines(output)
                .join(" ")
                .contains("remote outcome and ownership are unknown")
        },
    );
    reopened.send(b"second\r");
    reopened.wait_for(
        Duration::from_secs(20),
        "blocked uncertain session",
        |output| {
            screen_lines(output)
                .join(" ")
                .contains("Session state is uncertain")
        },
    );
    thread::sleep(Duration::from_millis(500));
    assert!(
        matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "uncertain session dispatched another request"
    );
    reopened.send(b"\x11");
    reopened.wait_exit(Duration::from_secs(10));
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
}

#[test]
#[ignore = "persistent PTY and loopback acceptance; requires the built Scheme package"]
fn interrupted_stream_retains_prefix_after_restart() {
    let oversized =
        std::env::var_os("MARLIN_PTY_OVERSIZE").as_deref() == Some(std::ffi::OsStr::new("1"));
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let server_stop = stop.clone();
    let (sender, receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        while let Some(mut stream) = accept_until(&listener, &server_stop) {
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let (_, request) = read_request(&mut stream);
            if sender.send(request).is_err() {
                break;
            }
            if stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"observed-prefix\"}\n\n").is_err() { continue; }
            if oversized {
                thread::sleep(Duration::from_millis(200));
                let mut event =
                    b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"".to_vec();
                event.extend(std::iter::repeat_n(b'x', 8 * 1024 * 1024 + 1));
                event.extend_from_slice(b"\"}\n\n");
                if let Err(error) = stream.write_all(&event) {
                    eprintln!("oversized fixture write ended: {error}");
                }
            } else {
                let _ = stream.write_all(
                    b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"-tail\"}\n\n",
                );
            }
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    let path =
        serde_json::to_string(&directory.path().join("session.db").to_string_lossy()).unwrap();
    fs::write(&config, format!("provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\ntimeout_seconds=5\n[session]\npath={path}\nproject=\"project:interrupted\"\nsession=\"session:interrupted\"\nagent=\"agent:interrupted\"\n")).unwrap();
    let mut first = Terminal::start(&config);
    first.wait_for(Duration::from_secs(20), "workbench", |output| {
        screen_lines(output)[29].contains("Message: Enter send")
    });
    first.send(b"first\r");
    first.wait_for(
        Duration::from_secs(20),
        "unknown remote outcome",
        |output| {
            screen_lines(output)
                .join(" ")
                .contains("remote outcome is unknown")
        },
    );
    if oversized {
        first.wait_for(Duration::from_secs(20), "provider byte limit", |output| {
            screen_lines(output)
                .join(" ")
                .contains("provider response exceeded its byte limit")
        });
    }
    let _request = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    first.send(b"\x11");
    first.wait_exit(Duration::from_secs(10));
    drop(first);

    let mut reopened = Terminal::start(&config);
    reopened.wait_for(
        Duration::from_secs(20),
        "unresolved previous turn",
        |output| {
            screen_lines(output)
                .join(" ")
                .contains("remote outcome and ownership are unknown")
        },
    );
    let expected = if oversized {
        "observed-prefix"
    } else {
        "observed-prefix-tail"
    };
    reopened.wait_for(Duration::from_secs(20), "recovered prefix", |output| {
        screen_lines(output).join(" ").contains(expected)
    });
    if oversized {
        reopened.wait_for(
            Duration::from_secs(20),
            "recovered byte-limit error",
            |output| {
                screen_lines(output)
                    .join(" ")
                    .contains("provider response exceeded its byte limit")
            },
        );
        assert!(
            !screen_lines(&reopened.output)
                .join("")
                .contains(&"x".repeat(100)),
            "rejected output leaked into recovered prefix"
        );
    }
    reopened.send(b"second\r");
    reopened.wait_for(
        Duration::from_secs(20),
        "blocked uncertain session",
        |output| {
            screen_lines(output)
                .join(" ")
                .contains("Session state is uncertain")
        },
    );
    thread::sleep(Duration::from_millis(500));
    assert!(
        matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "uncertain session dispatched another request"
    );
    reopened.send(b"\x11");
    reopened.wait_exit(Duration::from_secs(10));
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
}

#[test]
#[ignore = "persistent PTY and loopback acceptance; requires the built Scheme package"]
fn crash_recovery_blocks_retry_and_allows_another_session() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let server_stop = stop.clone();
    let (sender, receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let mut open_streams = Vec::new();
        while let Some(mut stream) = accept_until(&listener, &server_stop) {
            let (_, request) = read_request(&mut stream);
            if sender.send(request).is_err() {
                break;
            }
            if stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"held-output\"}\n\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"-timed\"}\n\n").is_ok() {
                open_streams.push(stream);
            }
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    let path =
        serde_json::to_string(&directory.path().join("session.db").to_string_lossy()).unwrap();
    fs::write(&config, format!("provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n[session]\npath={path}\nproject=\"project:crash\"\nsession=\"session:crash\"\nagent=\"agent:crash\"\n")).unwrap();
    let mut first = Terminal::start(&config);
    first.wait_for(Duration::from_secs(20), "workbench", |output| {
        screen_lines(output)[29].contains("Message: Enter send")
    });
    first.send(b"first\r");
    first.wait_for(Duration::from_secs(20), "stream checkpoint", |output| {
        contains(output, b"-timed")
    });
    let first_request = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(first_request["input"][0]["content"], "first");
    thread::sleep(Duration::from_millis(1500));
    first.child.kill().unwrap();
    assert!(!first.wait_exit_status(Duration::from_secs(5)).success());
    drop(first);

    let mut reopened = Terminal::start(&config);
    reopened.wait_for(Duration::from_secs(20), "unfinished turn", |output| {
        screen_lines(output)
            .join(" ")
            .contains("Unfinished prior turn")
    });
    reopened.wait_for(Duration::from_secs(20), "persisted prefix", |output| {
        screen_lines(output).join("").contains("held-output-timed")
    });
    reopened.send(b"second\r");
    reopened.wait_for(
        Duration::from_secs(20),
        "blocked uncertain session",
        |output| {
            screen_lines(output)
                .join(" ")
                .contains("Session state is uncertain")
        },
    );
    thread::sleep(Duration::from_secs(1));
    assert!(
        matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "unfinished turn was retried"
    );
    reopened.send(b"\x7f\x7f\x7f\x7f\x7f\x7f/session \"session:separate\"\r");
    thread::sleep(Duration::from_millis(500));
    let offset = reopened.output.len();
    reopened.send(b"new conversation\r");
    reopened.wait_for(
        Duration::from_secs(20),
        "separate session response",
        |output| contains(&output[offset..], b"-timed"),
    );
    let separate = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(separate["input"].as_array().unwrap().len(), 1);
    assert_eq!(separate["input"][0]["content"], "new conversation");
    reopened.send(b"\x11");
    reopened.wait_exit(Duration::from_secs(5));
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
}

#[test]
#[ignore = "real PTY, resize and loopback acceptance; requires the built Scheme package"]
fn resize_stream_scroll_and_history_search() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server_listener = listener.try_clone().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let (release_tx, release_rx) = mpsc::channel();
    let (initial_tx, initial_rx) = mpsc::channel();
    let (sent_tx, sent_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let Some(mut stream) = accept_until(&server_listener, &stop) else {
            return;
        };
        let _ = read_request(&mut stream);
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"INITIAL\"}\n\n").unwrap();
        initial_tx.send(()).unwrap();
        release_rx
            .recv_timeout(Duration::from_secs(15))
            .expect("release scroll fixture");
        let chunk = format!(
            "\n{}",
            (0..30)
                .map(|i| format!("FRESH_ROW_{i:02}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let events = [
            serde_json::json!({"type":"response.output_text.delta", "delta":chunk}),
            serde_json::json!({"type":"response.completed", "response":{"id":"fixture", "model":"gpt-fixture", "status":"completed", "output":[{"type":"message", "content":[{"type":"output_text", "text":format!("INITIAL{chunk}")}]}]}}),
        ];
        for event in events {
            write!(stream, "data: {event}\n\n").unwrap();
        }
        sent_tx.send(()).unwrap();
    });
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, format!("provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n")).unwrap();
    let mut terminal = Terminal::start_sized(&config, &[], &[], 20, 80);
    terminal.wait_raw(b"Marlin");
    let prompt = (0..50)
        .map(|i| format!("HISTORY_ROW_{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    terminal.send(format!("\x1b[200~{prompt}\x1b[201~\r").as_bytes());
    initial_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("initial stream chunk");
    terminal.wait_raw(b"Assistant");
    for (rows, cols) in [(12, 32), (8, 12), (24, 100), (20, 80)] {
        terminal.resize(rows, cols);
        terminal.pump(Duration::from_millis(250));
        assert!(
            terminal.child.try_wait().unwrap().is_none(),
            "TUI exited during resize to {cols}x{rows}"
        );
    }
    let scrolled = terminal.output.len();
    terminal.send(b"\x1b[5~");
    terminal.wait_for(Duration::from_secs(15), "scrolled history", |output| {
        contains(&output[scrolled..], b"HISTORY_ROW_")
    });
    terminal.pump(Duration::from_millis(300));
    let before_tail = terminal.output.len();
    release_tx.send(()).unwrap();
    sent_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("stream tail");
    terminal.pump(Duration::from_secs(1));
    assert!(
        !contains(&terminal.output[before_tail..], b"FRESH_ROW_"),
        "new output replaced scrolled history"
    );
    let before_scroll_down = terminal.output.len();
    for _ in 0..6 {
        terminal.send(b"\x1b[6~");
        terminal.pump(Duration::from_millis(100));
    }
    terminal.wait_for(
        Duration::from_secs(15),
        "stream tail while scrolling down",
        |output| contains(&output[before_scroll_down..], b"FRESH_ROW_00"),
    );
    terminal.send(b"draft before search");
    terminal.pump(Duration::from_millis(200));
    terminal.send(b"\x12");
    terminal.wait_text("Search:");
    terminal.send(b"HISTORY_ROW_05");
    terminal.pump(Duration::from_millis(300));
    terminal.send(b"\x1b");
    terminal.wait_text("draft before search");
    terminal.send(b"\x12HISTORY_ROW_05");
    terminal.wait_text("Search:");
    terminal.send(b"\r");
    terminal.pump(Duration::from_millis(300));
    assert!(
        terminal.child.try_wait().unwrap().is_none(),
        "TUI exited while searching history"
    );
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "search dispatched a second request"
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    server.join().unwrap();
}

#[test]
#[ignore = "real PTY, Git and loopback acceptance; requires the built Scheme package"]
fn workspace_reads_remain_outside_model_context() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let (sender, receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let Some(mut stream) = accept_until(&listener, &stop) else {
            return;
        };
        let (_, request) = read_request(&mut stream);
        sender.send(request).unwrap();
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"workspace-answer\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"fixture\",\"model\":\"gpt-fixture\",\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"workspace-answer\"}]}]}}\n\n").unwrap();
    });
    let directory = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .arg(directory.path())
            .status()
            .unwrap()
            .success()
    );
    let index = directory.path().join(".git/index");
    assert!(!index.exists());
    let tracked = directory.path().join("tracked-fixture.txt");
    fs::write(&tracked, "staged-diff-content\n").unwrap();
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(directory.path())
            .args(["add", "--", "tracked-fixture.txt"])
            .status()
            .unwrap()
            .success()
    );
    let index_before = fs::read(&index).unwrap();
    fs::write(&tracked, "worktree-diff-content\n").unwrap();
    let untracked = directory.path().join("workspace-fixture.txt");
    fs::write(&untracked, "untracked").unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, format!("provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n")).unwrap();
    let mut terminal = Terminal::start_in(&config, &[], &[], 24, 100, directory.path());
    terminal.wait_raw(b"Marlin");
    terminal.send(b"/status\r");
    terminal.wait_raw(b"workspace-fixture.txt");
    let mark = terminal.output.len();
    terminal.send(b"/diff \"tracked-fixture.txt\"\r");
    terminal.wait_for(Duration::from_secs(15), "worktree diff", |output| {
        contains(&output[mark..], b"worktree-diff-content")
    });
    let mark = terminal.output.len();
    terminal.send(b"/diff staged\r");
    terminal.wait_for(Duration::from_secs(15), "staged diff", |output| {
        contains(&output[mark..], b"staged-diff-content")
    });
    terminal.send(b"hello\x03");
    thread::sleep(Duration::from_millis(300));
    terminal.send(b"\r");
    terminal.wait_raw(b"workspace-answer");
    let request = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(request["input"].as_array().unwrap().len(), 1);
    assert_eq!(request["input"][0]["content"], "hello");
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    assert_eq!(tcgetattr(&terminal.slave).unwrap(), terminal.initial_mode);
    assert!(untracked.exists());
    assert_eq!(
        fs::read(index).unwrap(),
        index_before,
        "workspace reads changed the Git index"
    );
    server.join().unwrap();
}

fn send_signal(terminal: &Terminal, signal: Signal) {
    let pid = i32::try_from(terminal.child.id()).expect("child PID fits platform PID");
    kill(Pid::from_raw(pid), signal).expect("send terminal signal");
}

#[test]
#[ignore = "Unix signal PTY acceptance; requires the built Scheme package"]
fn unix_signals_restore_terminal_modes() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    for signal in [Signal::SIGINT, Signal::SIGTERM, Signal::SIGHUP] {
        let mut terminal = Terminal::start(&config);
        terminal.wait_for(Duration::from_secs(20), "workbench", |output| {
            contains(output, b"Marlin")
        });
        send_signal(&terminal, signal);
        assert_eq!(
            terminal.wait_exit_status(Duration::from_secs(10)).code(),
            Some(128 + signal as i32)
        );
        terminal.assert_restored();
    }
}

#[test]
#[ignore = "persistent Unix signal PTY acceptance; requires the built Scheme package"]
fn signal_preserves_unfinished_turn_and_blocks_retry() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let extra_connections = listener.try_clone().unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let server_stop = stop.clone();
    let (sender, receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let Some(mut stream) = accept_until(&listener, &server_stop) else {
            return;
        };
        let (_, request) = read_request(&mut stream);
        if sender.send(request).is_err() {
            return;
        }
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"signal-prefix\"}\n\n").unwrap();
        while !server_stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(20));
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    let path =
        serde_json::to_string(&directory.path().join("session.db").to_string_lossy()).unwrap();
    fs::write(&config, format!("provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n[session]\npath={path}\nproject=\"project:signal\"\nsession=\"session:signal\"\nagent=\"agent:signal\"\n")).unwrap();
    let mut first = Terminal::start(&config);
    first.wait_for(Duration::from_secs(20), "workbench", |output| {
        contains(output, b"Marlin")
    });
    first.send(b"first\r");
    first.wait_for(Duration::from_secs(20), "partial stream", |output| {
        contains(output, b"signal-prefix")
    });
    let request = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(request["input"][0]["content"], "first");
    send_signal(&first, Signal::SIGTERM);
    assert_eq!(
        first.wait_exit_status(Duration::from_secs(10)).code(),
        Some(128 + Signal::SIGTERM as i32)
    );
    first.assert_restored();
    drop(first);
    let mut reopened = Terminal::start(&config);
    reopened.wait_for(Duration::from_secs(20), "unfinished prior turn", |output| {
        screen_lines(output)
            .join(" ")
            .contains("Unfinished prior turn")
    });
    assert!(
        screen_lines(&reopened.output)
            .join("")
            .contains("signal-prefix")
    );
    reopened.send(b"second\r");
    reopened.wait_for(
        Duration::from_secs(20),
        "blocked uncertain session",
        |output| {
            screen_lines(output)
                .join(" ")
                .contains("Session state is uncertain")
        },
    );
    thread::sleep(Duration::from_millis(500));
    assert!(
        matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "uncertain session dispatched another request"
    );
    assert!(
        matches!(extra_connections.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "uncertain session opened another connection"
    );
    reopened.send(b"\x11");
    reopened.wait_exit(Duration::from_secs(10));
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
}

fn session_event_bodies(database_path: &std::path::Path, sql: &str) -> Vec<Vec<u8>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let path = database_path.to_str().unwrap();
        let database = turso::Builder::new_local(path).build().await.unwrap();
        let connection = database.connect().unwrap();
        let mut rows = connection.query(sql, ()).await.unwrap();
        let mut bodies = Vec::new();
        while let Some(row) = rows.next().await.unwrap() {
            match row.get_value(0).unwrap() {
                turso::Value::Blob(bytes) => bodies.push(bytes),
                turso::Value::Text(text) => bodies.push(text.into_bytes()),
                value => panic!("unexpected event body: {value:?}"),
            }
        }
        bodies
    })
}

#[test]
#[ignore = "persistent Anthropic PTY and loopback acceptance; requires the built Scheme package"]
fn anthropic_thinking_continuation_survives_restart() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let extra_connections = listener.try_clone().unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let (sender, receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        for answer in ["answer-fixture", "answer-live", "answer-restart"] {
            let Some(mut stream) = accept_until(&listener, &stop) else {
                return;
            };
            let (_, request) = read_request(&mut stream);
            if sender.send(request).is_err() {
                return;
            }
            let events = [
                serde_json::json!({"type":"message_start", "message":{"id":"fixture", "model":"fixture", "usage":{"input_tokens":1,"output_tokens":0}}}),
                serde_json::json!({"type":"content_block_start", "index":0, "content_block":{"type":"thinking", "thinking":""}}),
                serde_json::json!({"type":"content_block_delta", "index":0, "delta":{"type":"thinking_delta", "thinking":"reasoning-fixture"}}),
                serde_json::json!({"type":"content_block_delta", "index":0, "delta":{"type":"signature_delta", "signature":"signature-fixture"}}),
                serde_json::json!({"type":"content_block_stop", "index":0}),
                serde_json::json!({"type":"content_block_start", "index":1, "content_block":{"type":"text", "text":""}}),
                serde_json::json!({"type":"content_block_delta", "index":1, "delta":{"type":"text_delta", "text":answer}}),
                serde_json::json!({"type":"content_block_stop", "index":1}),
                serde_json::json!({"type":"message_delta", "delta":{"stop_reason":"end_turn"}, "usage":{"output_tokens":1}}),
                serde_json::json!({"type":"message_stop"}),
            ];
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n").unwrap();
            for event in events {
                write!(stream, "data: {event}\n\n").unwrap();
            }
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("session.db");
    let config = directory.path().join("config.toml");
    let path = serde_json::to_string(&database.to_string_lossy()).unwrap();
    fs::write(&config, format!("provider=\"fixture\"\nmodel=\"fixture\"\nprotocol=\"anthropic\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n[anthropic]\nthinking={{type=\"adaptive\"}}\neffort=\"low\"\nmax_tokens=8192\n[session]\npath={path}\nproject=\"project:pty\"\nsession=\"session:pty\"\nagent=\"agent:pty\"\n")).unwrap();
    let base = format!("http://{address}");
    let run = |first: bool, changed: bool| {
        let variables = [
            ("ANTHROPIC_BASE_URL", base.clone()),
            (
                "MARLIN_TUI_FIXTURE_KEY",
                if changed { "changed" } else { "fixture" }.into(),
            ),
        ];
        let mut terminal = Terminal::start_with_env(&config, &[], &variables);
        terminal.wait_for(Duration::from_secs(20), "workbench", |output| {
            contains(output, b"Marlin")
        });
        if first {
            terminal.send(b"hello\r");
        }
        terminal.wait_for(Duration::from_secs(20), "thinking", |output| {
            contains(output, b"reasoning-fixture")
        });
        terminal.wait_for(Duration::from_secs(20), "first answer", |output| {
            contains(output, b"answer-fixture")
        });
        thread::sleep(Duration::from_millis(500));
        let offset = terminal.output.len();
        terminal.send(b"follow-up\r");
        let expected: &[u8] = if changed {
            b"restored"
        } else if first {
            b"answer-live"
        } else {
            b"restart"
        };
        terminal.wait_for(Duration::from_secs(20), "follow-up outcome", |output| {
            contains(&output[offset..], expected)
        });
        thread::sleep(Duration::from_millis(500));
        terminal.send(b"\x11");
        terminal.wait_exit(Duration::from_secs(10));
    };
    run(true, false);
    let bodies = session_event_bodies(&database, "SELECT body FROM session_events");
    assert!(
        bodies.iter().any(
            |body| contains(body, b"signature-fixture") && contains(body, b"reasoning-fixture")
        ),
        "signed continuation not stored"
    );
    run(false, false);
    server.join().unwrap();
    run(false, true);
    thread::sleep(Duration::from_millis(500));
    assert!(
        matches!(extra_connections.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "credential mismatch dispatched a request"
    );
    let finished = session_event_bodies(
        &database,
        "SELECT body FROM session_events WHERE event_kind = 'marlin.conversation.turn.finished.v1'",
    );
    assert!(
        finished
            .iter()
            .any(|body| contains(body, b"RejectedBeforeDispatch"))
    );
    let requests: Vec<serde_json::Value> = receiver.try_iter().collect();
    assert_eq!(requests.len(), 3);
    for request in &requests {
        assert_eq!(request["thinking"], serde_json::json!({"type":"adaptive"}));
        assert_eq!(request["output_config"]["effort"], "low");
        assert_eq!(request["max_tokens"], 8192);
    }
    for request in &requests[1..] {
        assert_eq!(
            request["messages"][1]["content"][0],
            serde_json::json!({"type":"thinking", "thinking":"reasoning-fixture", "signature":"signature-fixture"})
        );
        assert_eq!(
            request["messages"][1]["content"][1]["text"],
            "answer-fixture"
        );
    }
}
