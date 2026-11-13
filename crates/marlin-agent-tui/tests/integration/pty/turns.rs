// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    StopOnDrop, Terminal, accept_until, contains, mx_selected, read_request, screen_lines,
};
use nix::{
    sys::signal::{Signal, kill},
    sys::termios::tcgetattr,
    unistd::Pid,
};
use std::{
    fs,
    io::Write,
    net::TcpListener,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[test]
#[ignore = "real PTY, linked Scheme tool catalog, loopback provider and local journal"]
fn scheme_bash_ask_continues_with_one_committed_result() {
    use marlin_agent_sessions::{ConversationJournal, ConversationTurnOutcome, ConversationTurnSource};
    use marlin_agent_storage::{
        AgentId, ProjectId, SessionId, TursoAgentStorage, TursoAgentStorageConfig,
        TursoBatchTransactionMode, TursoOptimizationProfile,
    };

    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let server_stop = stop.clone();
    let server = thread::spawn(move || {
        let Some(mut stream) = accept_until(&listener, &server_stop) else {
            return;
        };
        let (_, request) = read_request(&mut stream);
        assert_eq!(request["tools"][0]["name"], "bash");
        let response = serde_json::json!({"type":"response.completed","response":{
            "status":"completed","id":"resp-tool","model":"gpt-fixture",
            "output":[{"type":"function_call","id":"fc-1","call_id":"call-1",
                "name":"bash","arguments":"{\"command\":\"printf marlin-tool-ok\"}",
                "status":"completed"}]
        }});
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {response}\n\n").unwrap();
        let Some(mut continuation) = accept_until(&listener, &server_stop) else {
            panic!("committed tool result did not trigger provider continuation");
        };
        let (_, request) = read_request(&mut continuation);
        let output = request["input"].as_array().unwrap().last().unwrap();
        assert_eq!(output["type"], "function_call_output");
        assert_eq!(output["call_id"], "call-1");
        assert!(output["output"].as_str().unwrap().contains("marlin-tool-ok"));
        let response = serde_json::json!({"type":"response.completed","response":{
            "status":"completed","id":"resp-answer","model":"gpt-fixture",
            "output":[{"type":"message","content":[{"type":"output_text","text":"marker observed"}]}]
        }});
        write!(continuation, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {response}\n\n").unwrap();
    });
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("session.db");
    let config = directory.path().join("config.toml");
    fs::write(&config, format!(
        "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n[session]\npath={}\nproject=\"project:tool\"\nsession=\"session:tool\"\nagent=\"agent:tool\"\n",
        serde_json::to_string(&database.to_string_lossy()).unwrap(),
    )).unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"run tool\r");
    terminal.wait_for(Duration::from_secs(20), "Scheme Ask", |output| {
        contains(output, b"Tool request:") && contains(output, b"marlin-tool-ok")
    });
    terminal.send(b"\x1b[B");
    terminal.wait_for(Duration::from_secs(5), "second Ask choice", |output| {
        screen_lines(output)
            .iter()
            .any(|line| line.contains("2/2 Tool request:"))
    });
    terminal.send(b"\x1b[A");
    terminal.wait_for(Duration::from_secs(5), "first Ask choice", |output| {
        screen_lines(output)
            .iter()
            .any(|line| line.contains("1/2 Tool request:"))
    });
    terminal.send(b"\r");
    terminal.wait_for(Duration::from_secs(20), "committed tool result", |output| {
        screen_lines(output)
            .iter()
            .any(|line| line.contains("Tool results committed"))
    });
    terminal.wait_text("marker observed");
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    terminal.assert_restored();
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let turns = runtime.block_on(async {
        let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
            path: database,
            optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
            batch_transaction_mode: TursoBatchTransactionMode::Immediate,
        })
        .await
        .unwrap();
        let mut turns = Vec::new();
        let journal = ConversationJournal::open(
            storage,
            ProjectId::new("project:tool").unwrap(),
            SessionId::new("session:tool").unwrap(),
            AgentId::new("agent:tool").unwrap(),
            |turn| turns.push(turn.clone()),
        )
        .await
        .unwrap();
        assert!(journal.pending_tool_request().is_none());
        turns
    });
    assert_eq!(turns.len(), 2);
    assert!(
        format!("{:?}", turns[0].tool_results.as_ref().unwrap().results()).contains("marlin-tool-ok")
    );
    assert!(matches!(turns[1].start, ConversationTurnSource::ToolResults(_)));
    assert!(matches!(turns[1].outcome, Some(ConversationTurnOutcome::Completed { .. })));
}

#[test]
#[ignore = "real PTY and controlled loopback stream"]
fn streaming_command_navigation_stays_within_admitted_candidates() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let server_stop = stop.clone();
    let (connected_tx, connected_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let Some(mut stream) = accept_until(&listener, &server_stop) else {
            return;
        };
        let _ = read_request(&mut stream);
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"stream-active\"}\n\n").unwrap();
        connected_tx.send(()).unwrap();
        let _ = release_rx.recv_timeout(Duration::from_secs(20));
    });
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, format!("provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n")).unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"stream test\r");
    connected_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    terminal.wait_for(Duration::from_secs(10), "stream active", |output| {
        contains(output, b"stream-active")
    });
    terminal.send(b"\x1bx");
    terminal.wait_for(
        Duration::from_secs(5),
        "restricted first candidate",
        |output| screen_lines(output).iter().any(|line| mx_selected(line, 1)),
    );
    terminal.send(b"\x1b[B");
    terminal.wait_for(
        Duration::from_secs(5),
        "restricted next candidate",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| mx_selected(line, 2))
                && rows
                    .iter()
                    .all(|line| !line.contains("Command menu failed"))
        },
    );
    terminal.send(b"\x07\x03");
    let _ = release_tx.send(());
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
    server.join().unwrap();
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
            .contains("Request cancelled locally")
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
        "model request failure summary",
        |output| screen_lines(output)[29].contains("Model request failed"),
    );
    first.send(b"\x18bmessages\r");
    first.wait_for(
        Duration::from_secs(5),
        "unknown remote outcome detail",
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
    reopened.wait_for(Duration::from_secs(20), "recovery required", |output| {
        screen_lines(output)[28].contains("Recovery required")
    });
    reopened.send(b"\x18bmessages\r");
    reopened.wait_for(
        Duration::from_secs(5),
        "unresolved previous turn",
        |output| {
            let text = screen_lines(output).join(" ");
            text.contains("Uncertain prior turn") && text.contains("Further turns are blocked")
        },
    );
    reopened.send(b"\x1b");
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
        "model request failure summary",
        |output| screen_lines(output)[29].contains("Model request failed"),
    );
    first.send(b"\x18bmessages\r");
    first.wait_for(
        Duration::from_secs(5),
        "unknown remote outcome detail",
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
    reopened.wait_for(Duration::from_secs(20), "recovery required", |output| {
        screen_lines(output)[28].contains("Recovery required")
    });
    reopened.send(b"\x18bmessages\r");
    reopened.wait_for(
        Duration::from_secs(5),
        "unresolved previous turn",
        |output| {
            let text = screen_lines(output).join(" ");
            text.contains("Uncertain prior turn") && text.contains("Further turns are blocked")
        },
    );
    if oversized {
        assert!(
            screen_lines(&reopened.output)
                .join(" ")
                .contains("provider response exceeded its byte limit"),
            "recovered byte-limit diagnostic belongs in Messages"
        );
    }
    reopened.send(b"\x1b");
    let expected = if oversized {
        "observed-prefix"
    } else {
        "observed-prefix-tail"
    };
    reopened.wait_for(Duration::from_secs(20), "recovered prefix", |output| {
        screen_lines(output).join(" ").contains(expected)
    });
    if oversized {
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
    // Open the command surface explicitly: the rejected prompt remains a
    // draft, so typing a slash into it does not open the command palette.
    reopened.send(b"\x0f");
    reopened.wait_text("M-x");
    reopened.send(b"session \"session:separate\"\r");
    reopened.wait_for(Duration::from_secs(20), "separate session", |output| {
        screen_lines(output).join(" ").contains("session:separate")
    });
    reopened.send(b"\x7f\x7f\x7f\x7f\x7f\x7f");
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
    terminal.wait_text("MARLIN");
    let prompt = (0..50)
        .map(|i| format!("HISTORY_ROW_{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    terminal.send(format!("\x1b[200~{prompt}\x1b[201~\r").as_bytes());
    initial_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("initial stream chunk");
    terminal.wait_text("INITIAL");
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
    let before_wheel = screen_lines(&terminal.output);
    terminal.send(b"\x1b[<64;40;5M");
    terminal.wait_for(
        Duration::from_secs(5),
        "wheel up reveals older rows",
        |output| screen_lines(output)[..18] != before_wheel[..18],
    );
    terminal.send(b"\x1b[<65;40;5M");
    terminal.wait_for(
        Duration::from_secs(5),
        "wheel down restores rows",
        |output| screen_lines(output)[..18] == before_wheel[..18],
    );
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

#[path = "turns/signal_and_thinking.rs"]
mod signal_and_thinking;
