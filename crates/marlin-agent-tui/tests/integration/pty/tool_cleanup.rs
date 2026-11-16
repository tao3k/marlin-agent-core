// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::Terminal;
#[cfg(target_os = "macos")]
use super::{contains, read_http_request};
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    thread,
    time::{Duration, Instant},
};
#[cfg(target_os = "macos")]
use std::{io::ErrorKind, net::TcpListener, path::Path};

struct ChildGuard(Option<Pid>);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            let _ = kill(pid, Signal::SIGKILL);
        }
    }
}

#[test]
#[ignore = "macOS real PTY and controlled workspace git process"]
#[cfg(target_os = "macos")]
fn workspace_tool_cancel_reaps_process() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    let config = path.join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:9/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let git = path.join("git");
    fs::write(
        &git,
        "#!/bin/sh\nprintf '%s' \"$$\" > \"$MARLIN_TEST_GIT_PID\"\nexec /bin/sleep 30\n",
    )
    .unwrap();
    fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
    let pidfile = path.join("git.pid");
    let metrics = path.join("metrics.csv");
    let variables = [
        ("PATH", format!("{}:/usr/bin:/bin", path.display())),
        (
            "MARLIN_TEST_GIT_PID",
            pidfile.to_string_lossy().into_owned(),
        ),
        (
            "MARLIN_TUI_RUNTIME_METRICS_FILE",
            metrics.to_string_lossy().into_owned(),
        ),
    ];
    let mut terminal = Terminal::start_in(&config, &[], &variables, 30, 100, path);
    terminal.wait_status("Message: Enter send");
    for cycle in 1..=20 {
        let _ = fs::remove_file(&pidfile);
        terminal.send(b"\x0f");
        terminal.wait_text("M-x");
        terminal.send(b"status\r");
        let deadline = Instant::now() + Duration::from_secs(10);
        let pid = loop {
            if let Ok(raw) = fs::read_to_string(&pidfile)
                && let Ok(pid) = raw.parse::<i32>()
            {
                break Pid::from_raw(pid);
            }
            if Instant::now() >= deadline {
                terminal.pump(Duration::from_millis(100));
                panic!(
                    "workspace tool did not start; screen: {:?}",
                    super::screen_lines(&terminal.output)
                );
            }
            thread::sleep(Duration::from_millis(20));
        };
        let mut guard = ChildGuard(Some(pid));
        assert!(
            kill(pid, None).is_ok(),
            "tool must be alive before cancellation"
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let visible = fs::read_to_string(&metrics)
                .ok()
                .and_then(|data| {
                    data.lines()
                        .last()
                        .and_then(|line| line.split(',').nth(3))
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .unwrap_or(0);
            if visible > 0 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "live tool was not registered in Runtime"
            );
            thread::sleep(Duration::from_millis(20));
        }
        terminal.send(b"\x1b");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match kill(pid, None) {
                Err(nix::errno::Errno::ESRCH) => break,
                Ok(()) => (),
                Err(error) => panic!("process observation failed: {error}"),
            }
            assert!(
                Instant::now() < deadline,
                "workspace tool {pid} survived cancellation"
            );
            thread::sleep(Duration::from_millis(20));
        }
        guard.0 = None;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let visible = fs::read_to_string(&metrics).ok().and_then(|data| {
                data.lines()
                    .last()
                    .and_then(|line| line.split(',').nth(3))
                    .and_then(|value| value.parse::<usize>().ok())
            });
            if visible == Some(0) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Runtime still reports a process after cancellation"
            );
            thread::sleep(Duration::from_millis(20));
        }
        eprintln!("workspace cancellation cycle={cycle} pid={pid} reaped");
        thread::sleep(Duration::from_millis(200));
    }
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    terminal.assert_restored();
}

// Observe the crash boundary separately from graceful cancellation. SIGKILL
// bypasses the TUI's destructors; the fixture guard removes any surviving tool.
#[test]
#[ignore = "macOS real PTY crash with controlled workspace git process"]
#[cfg(target_os = "macos")]
fn workspace_tool_parent_crash_and_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    let config = path.join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:9/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let git = path.join("git");
    fs::write(
        &git,
        "#!/bin/sh\nprintf '%s' \"$$\" > \"$MARLIN_TEST_GIT_PID\"\nexec /bin/sleep 30\n",
    )
    .unwrap();
    fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
    let pidfile = path.join("git.pid");
    let variables = [
        ("PATH", format!("{}:/usr/bin:/bin", path.display())),
        (
            "MARLIN_TEST_GIT_PID",
            pidfile.to_string_lossy().into_owned(),
        ),
    ];
    let mut terminal = Terminal::start_in(&config, &[], &variables, 30, 100, path);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"\x0f");
    terminal.wait_text("M-x");
    terminal.send(b"status\r");
    let deadline = Instant::now() + Duration::from_secs(10);
    let pid = loop {
        if let Ok(raw) = fs::read_to_string(&pidfile)
            && let Ok(pid) = raw.parse::<i32>()
        {
            break Pid::from_raw(pid);
        }
        assert!(Instant::now() < deadline, "workspace tool did not start");
        thread::sleep(Duration::from_millis(20));
    };
    let mut guard = ChildGuard(Some(pid));
    assert!(kill(pid, None).is_ok());
    terminal.child.kill().unwrap();
    assert!(!terminal.wait_exit_status(Duration::from_secs(5)).success());
    thread::sleep(Duration::from_millis(200));
    let survived = kill(pid, None).is_ok();
    eprintln!(
        "workspace tool after parent SIGKILL: pid={pid} survived={survived}; fixture cleanup follows"
    );
    if survived {
        kill(pid, Signal::SIGKILL).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while kill(pid, None).is_ok() {
            assert!(
                Instant::now() < deadline,
                "fixture could not remove tool {pid}"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
    guard.0 = None;
    drop(terminal);
    let mut restarted = Terminal::start_in(&config, &[], &variables, 30, 100, path);
    restarted.wait_status("Message: Enter send");
    restarted.send(b"\x11");
    restarted.wait_exit(Duration::from_secs(10));
    restarted.assert_restored();
    assert!(
        !survived,
        "workspace tool survived TUI SIGKILL; fixture killed it"
    );
}

#[test]
#[ignore = "macOS real PTY shutdown with controlled workspace git process"]
#[cfg(target_os = "macos")]
fn workspace_tool_shutdown_reaps_after_exit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    let config = path.join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:9/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let git = path.join("git");
    fs::write(
        &git,
        "#!/bin/sh\nprintf '%s' \"$$\" > \"$MARLIN_TEST_GIT_PID\"\nexec /bin/sleep 30\n",
    )
    .unwrap();
    fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
    let pidfile = path.join("git.pid");
    let variables = [
        ("PATH", format!("{}:/usr/bin:/bin", path.display())),
        (
            "MARLIN_TEST_GIT_PID",
            pidfile.to_string_lossy().into_owned(),
        ),
    ];
    let mut terminal = Terminal::start_in(&config, &[], &variables, 30, 100, path);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"\x0f");
    terminal.wait_text("M-x");
    terminal.send(b"status\r");
    let deadline = Instant::now() + Duration::from_secs(10);
    let pid = loop {
        if let Ok(raw) = fs::read_to_string(&pidfile)
            && let Ok(pid) = raw.parse::<i32>()
        {
            break Pid::from_raw(pid);
        }
        assert!(Instant::now() < deadline, "workspace tool did not start");
        thread::sleep(Duration::from_millis(20));
    };
    let mut guard = ChildGuard(Some(pid));
    assert!(kill(pid, None).is_ok());
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    terminal.assert_restored();
    match kill(pid, None) {
        Err(nix::errno::Errno::ESRCH) => guard.0 = None,
        Ok(()) => panic!("workspace tool {pid} survived TUI exit"),
        Err(error) => panic!("process observation failed: {error}"),
    }
    eprintln!("workspace tool {pid} absent after TUI exit");
}

#[test]
#[ignore = "macOS real PTY and unresponsive loopback OTLP collector"]
#[cfg(target_os = "macos")]
fn stalled_collector_does_not_delay_runtime_drain_or_block_tui_exit() {
    let collector = TcpListener::bind("127.0.0.1:0").unwrap();
    collector.set_nonblocking(true).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    let config = write_group_tool_fixture(path);
    let metrics = path.join("metrics.csv");
    let pidfile = path.join("git.pids");
    let variables = [
        ("PATH", format!("{}:/usr/bin:/bin", path.display())),
        (
            "MARLIN_TEST_GIT_PID",
            pidfile.to_string_lossy().into_owned(),
        ),
        (
            "MARLIN_TUI_RUNTIME_METRICS_FILE",
            metrics.to_string_lossy().into_owned(),
        ),
        ("MARLIN_TUI_STARTUP_TRACE", "1".to_owned()),
        (
            "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
            format!("http://{}/v1/traces", collector.local_addr().unwrap()),
        ),
        // Make this a shutdown export regardless of periodic batching on the host.
        ("OTEL_BSP_SCHEDULE_DELAY", "60000".to_owned()),
        ("OTEL_BSP_MAX_QUEUE_SIZE", "2048".to_owned()),
        ("OTEL_BSP_MAX_EXPORT_BATCH_SIZE", "512".to_owned()),
    ];
    let mut terminal = Terminal::start_in(&config, &[], &variables, 30, 100, path);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"\x0f");
    terminal.wait_text("M-x");
    terminal.send(b"status\r");
    let startup_deadline = Instant::now() + Duration::from_secs(10);
    let (tool, descendant) = wait_for_fixture(startup_deadline, "workspace process group", || {
        let pids = fs::read_to_string(&pidfile).ok()?;
        let (tool, descendant) = pids.trim().split_once(' ')?;
        Some((
            Pid::from_raw(tool.parse().ok()?),
            Pid::from_raw(descendant.parse().ok()?),
        ))
    });
    let mut tool_guard = ChildGuard(Some(tool));
    let mut descendant_guard = ChildGuard(Some(descendant));
    assert_eq!(nix::unistd::getpgid(Some(tool)).unwrap(), tool);
    assert_eq!(nix::unistd::getpgid(Some(descendant)).unwrap(), tool);
    wait_for_fixture(startup_deadline, "active Runtime tool", || {
        let (tasks, processes) = runtime_counts(&metrics)?;
        (tasks > 0 && processes == 1).then_some(())
    });

    let quit = Instant::now();
    let deadline = quit + Duration::from_secs(10);
    terminal.send(b"\x11");
    let mut connection = wait_for_fixture(deadline, "OTLP shutdown request", || {
        match collector.accept() {
            Ok((stream, _)) => Some(stream),
            Err(error) if error.kind() == ErrorKind::WouldBlock => None,
            Err(error) => panic!("accept OTLP shutdown request: {error}"),
        }
    });
    connection.set_nonblocking(false).unwrap();
    connection
        .set_read_timeout(Some(deadline.saturating_duration_since(Instant::now())))
        .unwrap();
    let (headers, body) = read_http_request(&mut connection);
    assert!(headers.starts_with("POST /v1/traces HTTP/1.1\r\n"));
    assert!(contains(&body, b"marlin-tui") && contains(&body, b"runtime.task"));
    let flush_started = quit.elapsed();

    // Check at the collector boundary while the TUI is alive and still waiting for a response.
    assert!(terminal.child.try_wait().unwrap().is_none());
    assert_eq!(runtime_counts(&metrics), Some((0, 0)));
    for pid in [tool, descendant, Pid::from_raw(-tool.as_raw())] {
        assert_eq!(
            kill(pid, None),
            Err(nix::errno::Errno::ESRCH),
            "live resource {pid} during OTLP export"
        );
    }
    tool_guard.0 = None;
    descendant_guard.0 = None;
    terminal.wait_for(
        deadline.saturating_duration_since(Instant::now()),
        "Runtime dropped before export",
        |output| contains(output, b"marlin-tui startup Tokio runtime dropped"),
    );
    terminal.assert_restored();

    // No response or collector-side close may unblock the TUI.
    terminal.wait_exit(deadline.saturating_duration_since(Instant::now()));
    assert!(
        Instant::now() < deadline,
        "TUI exceeded the shutdown watchdog"
    );
    assert!(contains(
        &terminal.output,
        b"telemetry shutdown: failed to export 1 OTLP trace batch"
    ));
    drop(connection);
    eprintln!(
        "stalled collector: group={tool} descendant={descendant} reaped before OTLP at {flush_started:?}; TUI exited in {:?}",
        quit.elapsed()
    );
}

#[test]
#[ignore = "macOS real PTY and quiet managed workspace tool"]
#[cfg(target_os = "macos")]
fn quiet_workspace_tool_survives_idle_observation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    let config = write_group_tool_fixture(path);
    let git = path.join("git");
    fs::write(
        &git,
        "#!/bin/sh\nprintf '%s' \"$$\" > \"$MARLIN_TEST_GIT_PID\"\nexec /bin/sleep 30\n",
    )
    .unwrap();
    let pidfile = path.join("git.pid");
    let variables = [
        ("PATH", format!("{}:/usr/bin:/bin", path.display())),
        (
            "MARLIN_TEST_GIT_PID",
            pidfile.to_string_lossy().into_owned(),
        ),
    ];
    let mut terminal = Terminal::start_in(&config, &[], &variables, 30, 100, path);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"\x0f");
    terminal.wait_text("M-x");
    terminal.send(b"status\r");
    let pid = wait_for_fixture(
        Instant::now() + Duration::from_secs(5),
        "workspace tool PID",
        || {
            Some(Pid::from_raw(
                fs::read_to_string(&pidfile).ok()?.parse().ok()?,
            ))
        },
    );
    let mut guard = ChildGuard(Some(pid));
    terminal.wait_for(
        Duration::from_secs(8),
        "workspace idle advisory",
        |output| super::screen_lines(output)[29].contains("Workspace output idle"),
    );
    assert_eq!(
        kill(pid, None),
        Ok(()),
        "quiet tool must survive the advisory"
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    terminal.assert_restored();
    assert_eq!(kill(pid, None), Err(nix::errno::Errno::ESRCH));
    guard.0 = None;
}

#[test]
#[ignore = "macOS linked M-x process list and Runtime-owned stop"]
#[cfg(target_os = "macos")]
fn process_buffer_stops_a_managed_generation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    let config = write_group_tool_fixture(path);
    let pidfile = path.join("git.pid");
    let variables = [
        ("PATH", format!("{}:/usr/bin:/bin", path.display())),
        (
            "MARLIN_TEST_GIT_PID",
            pidfile.to_string_lossy().into_owned(),
        ),
    ];
    let mut terminal = Terminal::start_in(&config, &[], &variables, 30, 100, path);
    terminal.wait_status("Message: Enter send");
    let command = |terminal: &mut Terminal, input: &str| {
        terminal.send(b"\x0f");
        terminal.wait_text("M-x");
        terminal.send(format!("{input}\r").as_bytes());
    };
    command(&mut terminal, "status");
    let pids = wait_for_fixture(
        Instant::now() + Duration::from_secs(5),
        "managed tool PIDs",
        || {
            let text = fs::read_to_string(&pidfile).ok()?;
            let mut pids = text.split_whitespace();
            Some([
                Pid::from_raw(pids.next()?.parse().ok()?),
                Pid::from_raw(pids.next()?.parse().ok()?),
            ])
        },
    );
    let mut guards = pids.map(|pid| ChildGuard(Some(pid)));
    command(&mut terminal, "list-processes");
    terminal.wait_text("*Processes*");
    terminal.wait_for(
        Duration::from_secs(5),
        "running process in *Processes*",
        |output| {
            super::screen_lines(output)
                .iter()
                .any(|line| line.contains("Running") && line.contains("workspace.status"))
        },
    );
    let generation = super::screen_lines(&terminal.output)
        .into_iter()
        .find(|line| line.contains("Running") && line.contains("workspace.status"))
        .and_then(|line| {
            line.split_whitespace()
                .find_map(|field| field.parse::<u64>().ok())
        })
        .expect("process generation in table");
    command(&mut terminal, &format!("kill-process {generation}"));
    terminal.wait_text("Exited (failed)");
    for guard in &mut guards {
        assert_eq!(kill(guard.0.unwrap(), None), Err(nix::errno::Errno::ESRCH));
        guard.0 = None;
    }
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    terminal.assert_restored();
}

#[test]
#[ignore = "macOS real PTY and independent Darwin host observation"]
#[cfg(target_os = "macos")]
fn independent_observer_distinguishes_stall_from_exit() {
    use marlin_agent_telemetry::host_watchdog::{
        HostHeartbeatSource::{Tokio, Tui},
        HostObservation::{Exited, HeartbeatMissed, HeartbeatResumed, HeartbeatStarted},
        HostObservationRecord,
    };
    use std::io::Write;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    let config = write_group_tool_fixture(path);
    let sample = marlin_agent_protocol::ProcessMemorySampler::sample(
        &marlin_agent_telemetry::MacOsProcessMemorySampler,
        &[],
    )
    .unwrap();
    assert!(sample.host_bytes > 0);
    assert_eq!(sample.child_bytes, 0);
    let reservation = sample.host_bytes;
    // This test measures host observation with admission enabled. Leave room
    // for the separately linked Scheme VM and Tokio workers in the TUI.
    let budget = sample.host_bytes.checked_add(1 << 30).unwrap();
    writeln!(
        fs::OpenOptions::new().append(true).open(&config).unwrap(),
        "[process_budget]\nconcurrent = 4\nmemory_limit_bytes = {budget}\ncommand_reservation_bytes = {reservation}",
    ).unwrap();
    let pidfile = path.join("git.pid");
    let observations = path.join("host.jsonl");
    let variables = [
        ("PATH", format!("{}:/usr/bin:/bin", path.display())),
        (
            "MARLIN_TEST_GIT_PID",
            pidfile.to_string_lossy().into_owned(),
        ),
        (
            "MARLIN_TUI_HOST_OBSERVATIONS_FILE",
            observations.to_string_lossy().into_owned(),
        ),
    ];
    let mut terminal = Terminal::start_in(&config, &[], &variables, 30, 120, path);
    terminal.wait_status("Message: Enter send");
    let host = terminal.child.id();
    let host_pid = Pid::from_raw(host.try_into().unwrap());
    let reports = || -> Option<Vec<_>> {
        Some(
            fs::read_to_string(&observations)
                .ok()?
                .lines()
                .filter_map(|line| {
                    let record: HostObservationRecord = serde_json::from_str(line).ok()?;
                    assert_eq!(record.host_pid, host);
                    Some(record.observation)
                })
                .collect(),
        )
    };
    let observe = |expected: &[_]| {
        wait_for_fixture(
            Instant::now() + Duration::from_secs(5),
            "host evidence",
            || {
                let seen = reports()?;
                expected
                    .iter()
                    .all(|item| seen.contains(item))
                    .then_some(seen)
            },
        )
    };
    #[derive(serde::Deserialize)]
    struct Trace {
        states: Vec<Frame>,
    }
    #[derive(serde::Deserialize)]
    struct Frame {
        s: State,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct State {
        event: String,
        host_alive: bool,
        tool_alive: bool,
        tui: String,
        tokio: String,
        exit_observed: bool,
    }
    let trace =
        std::env::var_os("MARLIN_HOST_OBSERVER_TRACE").expect("generate the Quint ITF first");
    let trace: Trace = serde_json::from_slice(&fs::read(trace).unwrap()).unwrap();
    assert_eq!(trace.states.len(), 5);
    let mut tools = None;
    let alive = |pid| match kill(pid, None) {
        Ok(()) => true,
        Err(nix::errno::Errno::ESRCH) => false,
        Err(error) => panic!("process observation failed: {error}"),
    };
    for Frame { s } in trace.states {
        match s.event.as_str() {
            "attach" => (),
            "start" => {
                terminal.send(b"\x0f");
                terminal.wait_text("M-x");
                terminal.send(b"status\r");
                let pids = wait_for_fixture(
                    Instant::now() + Duration::from_secs(5),
                    "managed tool PIDs",
                    || {
                        let text = fs::read_to_string(&pidfile).ok()?;
                        let mut pids = text.split_whitespace();
                        Some([
                            Pid::from_raw(pids.next()?.parse().ok()?),
                            Pid::from_raw(pids.next()?.parse().ok()?),
                        ])
                    },
                );
                tools = Some(pids.map(|pid| ChildGuard(Some(pid))));
            }
            "stop" => kill(host_pid, Signal::SIGSTOP).unwrap(),
            "resume" => kill(host_pid, Signal::SIGCONT).unwrap(),
            "quit" => {
                terminal.send(b"\x11");
                terminal.wait_exit(Duration::from_secs(10));
                terminal.assert_restored();
            }
            event => panic!("unknown host model event {event}"),
        }
        let heartbeat = |phase: &str, source| match phase {
            "started" => HeartbeatStarted(source),
            "missed" => HeartbeatMissed(source),
            "resumed" => HeartbeatResumed(source),
            other => panic!("unknown heartbeat state {other}"),
        };
        let mut expected = vec![heartbeat(&s.tui, Tui), heartbeat(&s.tokio, Tokio)];
        if s.exit_observed {
            expected.push(Exited);
        }
        let seen = observe(&expected);
        assert_eq!(seen.contains(&Exited), s.exit_observed);
        assert_eq!(alive(host_pid), s.host_alive);
        if let Some(guards) = &mut tools {
            for guard in guards {
                assert_eq!(alive(guard.0.unwrap()), s.tool_alive, "{}", s.event);
                if !s.tool_alive {
                    guard.0 = None;
                }
            }
        } else {
            assert!(!s.tool_alive);
            assert!(!pidfile.exists());
        }
    }
    println!("PASS: 5 Quint states matched macOS host and tool observations");
}

#[cfg(target_os = "macos")]
fn write_group_tool_fixture(path: &Path) -> std::path::PathBuf {
    let config = path.join("config.toml");
    fs::write(&config, "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:9/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n").unwrap();
    let git = path.join("git");
    fs::write(
        &git,
        "#!/bin/sh\n/bin/sleep 30 &\nprintf '%s %s\\n' \"$$\" \"$!\" > \"$MARLIN_TEST_GIT_PID\"\nwait\n",
    )
    .unwrap();
    fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
    config
}

#[cfg(target_os = "macos")]
fn runtime_counts(path: &Path) -> Option<(usize, usize)> {
    let data = fs::read_to_string(path).ok()?;
    let mut columns = data.lines().last()?.split(',');
    Some((columns.nth(2)?.parse().ok()?, columns.next()?.parse().ok()?))
}

#[cfg(target_os = "macos")]
fn wait_for_fixture<T>(
    deadline: Instant,
    description: &str,
    mut observe: impl FnMut() -> Option<T>,
) -> T {
    loop {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        if let Some(value) = observe() {
            return value;
        }
        thread::sleep(Duration::from_millis(10));
    }
}
