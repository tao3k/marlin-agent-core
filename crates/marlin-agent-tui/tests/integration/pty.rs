// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real Unix PTY, streaming provider, startup and terminal restoration gate.
#![cfg(unix)]

use nix::{
    pty::{Winsize, openpty},
    sys::termios::tcgetattr,
};
#[cfg(feature = "linked-ui")]
use nix::{
    sys::{
        signal::{Signal, kill},
        wait::{WaitPidFlag, WaitStatus, waitpid},
    },
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
#[path = "pty/minibuffer.rs"]
#[cfg(feature = "linked-ui")]
mod minibuffer;
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
            Ok((stream, _)) => {
                stream
                    .set_nonblocking(false)
                    .expect("blocking fixture connection");
                return Some(stream);
            }
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
    #[cfg(feature = "linked-ui")]
    screen: TerminalScreen,
    #[cfg(feature = "linked-ui")]
    screen_offset: usize,
    start: Instant,
    first_output_elapsed: Option<Duration>,
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
        Self::start_in_binary(
            std::path::Path::new(env!("CARGO_BIN_EXE_marlin-tui")),
            config,
            args,
            variables,
            rows,
            columns,
            working_directory,
        )
    }

    fn start_in_binary(
        binary: &std::path::Path,
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
        let pair = openpty(Some(&size), None).unwrap_or_else(|error| {
            let os_error = std::io::Error::last_os_error();
            panic!("open PTY: {error:?}; underlying OS error: {os_error:?}")
        });
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
        let mut command = Command::new(binary);
        command
            .current_dir(working_directory)
            .arg(config)
            .args(args)
            .env("MARLIN_TUI_FIXTURE_KEY", "fixture")
            .env_remove("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
            .env_remove("MARLIN_TUI_HOST_OBSERVATIONS_FILE")
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
        if std::env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some() {
            eprintln!("marlin-pty spawn returned after {:?}", start.elapsed());
        }
        Self {
            child,
            master,
            slave,
            received,
            output: Vec::new(),
            #[cfg(feature = "linked-ui")]
            screen: TerminalScreen::new(),
            #[cfg(feature = "linked-ui")]
            screen_offset: 0,
            start,
            first_output_elapsed: None,
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
        mut predicate: impl FnMut(&[u8]) -> bool,
    ) {
        self.wait_until(timeout, description, |terminal| predicate(&terminal.output));
    }

    #[cfg(feature = "linked-ui")]
    fn wait_for_screen(
        &mut self,
        timeout: Duration,
        description: &str,
        mut predicate: impl FnMut(&TerminalScreen) -> bool,
    ) {
        self.wait_until(timeout, description, |terminal| {
            terminal
                .screen
                .update(&terminal.output[terminal.screen_offset..]);
            terminal.screen_offset = terminal.output.len();
            predicate(&terminal.screen)
        });
    }

    fn wait_until(
        &mut self,
        timeout: Duration,
        description: &str,
        mut predicate: impl FnMut(&mut Self) -> bool,
    ) {
        let deadline = Instant::now() + timeout;
        loop {
            if predicate(self) {
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
                Ok(chunk) => {
                    if self.first_output_elapsed.is_none() {
                        let elapsed = self.start.elapsed();
                        self.first_output_elapsed = Some(elapsed);
                        if std::env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some() {
                            eprintln!("marlin-pty first output after {elapsed:?}");
                        }
                    }
                    self.output.extend_from_slice(&chunk);
                }
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

    fn wait_mx_selected(&mut self, selected: usize) {
        self.wait_for(Duration::from_secs(15), "M-x selection", |output| {
            screen_lines(output)
                .iter()
                .any(|line| mx_selected(line, selected))
        });
    }

    fn wait_mx_last(&mut self) {
        self.wait_for(Duration::from_secs(15), "last M-x selection", |output| {
            screen_lines(output).iter().any(|line| mx_last(line))
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
                    "marlin-tui startup first frame",
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
struct TerminalScreen {
    rows: [[char; 100]; 30],
    row: usize,
    col: usize,
    pending: Vec<u8>,
}

impl TerminalScreen {
    fn new() -> Self {
        Self {
            rows: [[' '; 100]; 30],
            row: 0,
            col: 0,
            pending: Vec::new(),
        }
    }

    fn update(&mut self, output: &[u8]) {
        let mut buffered = std::mem::take(&mut self.pending);
        buffered.extend_from_slice(output);
        let complete = match std::str::from_utf8(&buffered) {
            Err(error) if error.error_len().is_none() => error.valid_up_to(),
            _ => buffered.len(),
        };
        let text = String::from_utf8_lossy(&buffered[..complete]);
        let mut chars = text.char_indices().peekable();
        let mut remaining = text.len();
        let (rows, row, col) = (&mut self.rows, &mut self.row, &mut self.col);
        while let Some((index, ch)) = chars.next() {
            if ch == '\x1b' && chars.peek().is_none() {
                remaining = index;
                break;
            }
            if ch == '\x1b' && chars.peek().is_some_and(|&(_, next)| next == '[') {
                chars.next();
                let Some((end, op)) = chars.find(|&(_, c)| ('\x40'..='\x7e').contains(&c)) else {
                    remaining = index;
                    break;
                };
                let args = &text[index + 2..end];
                let nums: Vec<usize> = args
                    .trim_start_matches('?')
                    .split(';')
                    .map(|n| n.parse().unwrap_or(0))
                    .collect();
                let n = nums.first().copied().unwrap_or(0).max(1);
                match op {
                    'h' if args == "?1049" => {
                        for line in rows.iter_mut() {
                            line.fill(' ');
                        }
                        *row = 0;
                        *col = 0;
                    }
                    'H' | 'f' => {
                        *row = n - 1;
                        *col = nums.get(1).copied().unwrap_or(1).max(1) - 1;
                    }
                    'G' => *col = n - 1,
                    'A' => *row = row.saturating_sub(n),
                    'B' => *row = row.saturating_add(n),
                    'C' => *col = col.saturating_add(n),
                    'D' => *col = col.saturating_sub(n),
                    'J' if nums.first().copied() == Some(2) => {
                        for line in rows.iter_mut() {
                            line.fill(' ');
                        }
                    }
                    'K' if *row < 30 => {
                        let mode = nums.first().copied().unwrap_or(0);
                        let start = if mode == 1 || mode == 2 {
                            0
                        } else {
                            (*col).min(100)
                        };
                        let end = if mode == 1 { (*col + 1).min(100) } else { 100 };
                        rows[*row][start..end].fill(' ');
                    }
                    _ => {}
                }
            } else if ch == '\r' {
                *col = 0;
            } else if ch == '\n' {
                *row = row.saturating_add(1);
            } else if !ch.is_control() {
                let width = ch.width().unwrap_or(0);
                if *row < 30 && *col < 100 && width > 0 {
                    rows[*row][*col] = ch;
                }
                *col = col.saturating_add(width);
            }
        }
        self.pending
            .extend_from_slice(&text.as_bytes()[remaining..]);
        self.pending.extend_from_slice(&buffered[complete..]);
    }

    fn line(&self, row: usize) -> String {
        self.rows[row].iter().collect()
    }

    #[cfg(feature = "linked-ui")]
    fn lines(&self) -> impl Iterator<Item = String> + '_ {
        (0..self.rows.len()).map(|row| self.line(row))
    }
}

fn screen_lines(output: &[u8]) -> Vec<String> {
    let mut screen = TerminalScreen::new();
    screen.update(output);
    (0..screen.rows.len()).map(|row| screen.line(row)).collect()
}

fn mx_position(line: &str) -> Option<(usize, usize)> {
    let (position, _) = line.trim_start().split_once(" M-x")?;
    let (selected, total) = position.split_once('/')?;
    let selected = selected.parse::<usize>().ok()?;
    let total = total.parse::<usize>().ok()?;
    (total > 0 && selected > 0 && selected <= total).then_some((selected, total))
}

fn mx_selected(line: &str, selected: usize) -> bool {
    mx_position(line).is_some_and(|(current, _)| current == selected)
}

fn mx_last(line: &str) -> bool {
    mx_position(line).is_some_and(|(selected, total)| selected == total)
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

    let mut incremental = TerminalScreen::new();
    for byte in output {
        incremental.update(&[*byte]);
    }
    assert_eq!(
        (0..30).map(|row| incremental.line(row)).collect::<Vec<_>>(),
        rows
    );
    incremental.update(b"\x1b[30;2HR");
    assert!(incremental.line(29).contains("Ressage: Enter send"));
    for byte in "\x1b[1;1H🐟".bytes() {
        incremental.update(&[byte]);
    }
    assert!(incremental.line(0).starts_with('🐟'));
}

fn read_request(stream: &mut TcpStream) -> (String, serde_json::Value) {
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    let (headers, body) = read_http_request(stream);
    (headers, serde_json::from_slice(&body).unwrap())
}

fn read_http_request(stream: &mut TcpStream) -> (String, Vec<u8>) {
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
    (
        headers,
        input[body_start..body_start + content_length].to_vec(),
    )
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
    startup_acceptance::exercise_pty(Duration::from_secs(10), Duration::from_secs(5));
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
        let rows = screen_lines(output);
        contains(output, b"\x1b[?1049h")
            && rows[29].contains("Message: Enter send")
            && rows[28].contains("openai/gpt-fixture")
            && rows[0].trim().is_empty()
    });
    terminal.send(b"\x1bx\x1b[B");
    terminal.wait_for(
        Duration::from_secs(5),
        "source-free first M-x Down",
        |output| {
            screen_lines(output)
                .iter()
                .any(|line| line.contains("2/") && line.contains("M-x"))
        },
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn minibuffer_expands_on_hover_and_collapses_on_leave() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "collapsed minibuffer", |output| {
        contains(output, b"\x1b[?1003h")
            && screen_lines(output)[28].contains("MARLIN")
            && screen_lines(output)[29].contains("Message: Enter send")
    });
    terminal.send(b"\x1b[<35;1;30M");
    terminal.wait_for(Duration::from_secs(5), "expanded minibuffer", |output| {
        screen_lines(output)[25].contains("MARLIN")
    });
    terminal.send(b"\x1b[<35;1;1M");
    terminal.wait_for(
        Duration::from_secs(5),
        "collapsed minibuffer again",
        |output| screen_lines(output)[28].contains("MARLIN"),
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
fn ctrl_z_restores_terminal_before_suspend_and_reopens_after_continue() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "first workbench", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    terminal.send(b"draft survives suspension");
    terminal.wait_for(
        Duration::from_secs(5),
        "draft before suspension",
        |output| {
            screen_lines(output)
                .iter()
                .any(|line| line.contains("draft survives suspension"))
        },
    );
    let before_suspend = terminal.output.len();
    terminal.send(b"\x1a");
    terminal.wait_for(
        Duration::from_secs(5),
        "terminal restoration before stop",
        |output| contains(&output[before_suspend..], b"\x1b[?1049l"),
    );
    let pid = Pid::from_raw(terminal.child.id().try_into().unwrap());
    let wait_stopped = || {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match waitpid(pid, Some(WaitPidFlag::WNOHANG | WaitPidFlag::WUNTRACED)).unwrap() {
                WaitStatus::StillAlive if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(10));
                }
                WaitStatus::Stopped(_, Signal::SIGSTOP) => break,
                outcome => panic!("expected terminal-safe stop, observed {outcome:?}"),
            }
        }
    };
    wait_stopped();
    assert_eq!(tcgetattr(&terminal.slave).unwrap(), terminal.initial_mode);
    let before_resume = terminal.output.len();
    kill(pid, Signal::SIGCONT).unwrap();
    terminal.wait_for(
        Duration::from_secs(5),
        "workbench after continue",
        |output| {
            contains(&output[before_resume..], b"\x1b[?1049h")
                && screen_lines(output)
                    .iter()
                    .any(|line| line.contains("draft survives suspension"))
        },
    );

    let before_external_stop = terminal.output.len();
    kill(pid, Signal::SIGTSTP).unwrap();
    terminal.wait_for(
        Duration::from_secs(5),
        "external stop restoration",
        |output| contains(&output[before_external_stop..], b"\x1b[?1049l"),
    );
    wait_stopped();
    assert_eq!(tcgetattr(&terminal.slave).unwrap(), terminal.initial_mode);
    let before_external_resume = terminal.output.len();
    kill(pid, Signal::SIGCONT).unwrap();
    terminal.wait_for(
        Duration::from_secs(5),
        "workbench after external stop",
        |output| {
            contains(&output[before_external_resume..], b"\x1b[?1049h")
                && screen_lines(output)
                    .iter()
                    .any(|line| line.contains("draft survives suspension"))
        },
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
#[ignore = "requires ANTHROPIC_API_KEY and ANTHROPIC_BASE_URL for example.toml"]
fn example_config_linked_debug_startup() {
    let config = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("example.toml");
    let mut terminal = Terminal::start(&config);
    terminal.wait_for(Duration::from_secs(10), "example workbench", |output| {
        contains(output, b"\x1b[?1049h")
            && screen_lines(output)[29].contains("Message: Enter send")
            && screen_lines(output)
                .iter()
                .any(|line| line.contains("MARLIN"))
    });
    eprintln!(
        "example workbench ready after {:?}",
        terminal.start.elapsed()
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
#[ignore = "diagnostic only: set MARLIN_TUI_DIAGNOSTIC_BINARY to an absolute executable path"]
fn example_config_binary_diagnostic() {
    let binary = std::path::PathBuf::from(
        std::env::var_os("MARLIN_TUI_DIAGNOSTIC_BINARY").expect("diagnostic binary path"),
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = root.join("crates/marlin-agent-tui/example.toml");
    let mut terminal = Terminal::start_in_binary(&binary, &config, &[], &[], 30, 100, &root);
    terminal.wait_for(Duration::from_secs(10), "diagnostic workbench", |output| {
        contains(output, b"\x1b[?1049h")
            && screen_lines(output)[29].contains("Message: Enter send")
            && screen_lines(output)
                .iter()
                .any(|line| line.contains("MARLIN"))
    });
    eprintln!(
        "diagnostic workbench ready after {:?} from {}",
        terminal.start.elapsed(),
        binary.display()
    );
    if std::env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some() {
        for line in String::from_utf8_lossy(&terminal.output).split("\r\n") {
            if line.starts_with("marlin-tui startup ") {
                eprintln!("{line}");
            }
        }
    }
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[cfg(feature = "linked-ui")]
#[test]
#[ignore = "requires the linked Release artifact from just tui-aot-build"]
fn release_linked_ui_first_frame_without_a_gerbil_package_tree() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = root.join("target/release/marlin-tui");
    assert!(binary.is_file(), "build the linked Release binary first");
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let absent = directory.path().join("no-gerbil-install");
    let absent = absent.to_string_lossy().into_owned();
    let mut terminal = Terminal::start_in_binary(
        &binary,
        &config,
        &[],
        &[
            ("GERBIL_HOME", absent.clone()),
            ("GERBIL_PATH", absent.clone()),
            ("GERBIL_LOADPATH", absent),
        ],
        30,
        100,
        directory.path(),
    );
    terminal.wait_for(Duration::from_secs(10), "release workbench", |output| {
        contains(output, b"\x1b[?1049h")
            && screen_lines(output)[29].contains("Message: Enter send")
            && screen_lines(output)
                .iter()
                .any(|line| line.contains("MARLIN"))
    });
    eprintln!(
        "release workbench ready after {:?}",
        terminal.start.elapsed()
    );
    terminal.send(b"\x1bx\x1b[B");
    terminal.wait_for(Duration::from_secs(5), "release M-x first Down", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|line| mx_selected(line, 2))
            && rows.iter().any(|line| line.starts_with(" › "))
    });
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}

#[path = "pty/startup_acceptance.rs"]
mod startup_acceptance;

#[path = "pty/turns.rs"]
mod turns;

#[path = "pty/soak.rs"]
mod soak;

#[path = "pty/tool_cleanup.rs"]
mod tool_cleanup;
