// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    Arc, AtomicBool, Duration, Ordering, Pid, Signal, StopOnDrop, TcpListener, Terminal, Write,
    accept_until, contains, fs, kill, mpsc, read_request, screen_lines, thread,
};

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
    let (resume_tx, resume_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        for (turn_index, answer) in ["answer-fixture", "answer-live", "answer-restart"]
            .into_iter()
            .enumerate()
        {
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
            for (event_index, event) in events.into_iter().enumerate() {
                write!(stream, "data: {event}\n\n").unwrap();
                if turn_index == 0 && event_index == 2 {
                    resume_rx.recv_timeout(Duration::from_secs(20)).unwrap();
                }
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
            screen_lines(output)
                .iter()
                .any(|line| line.contains("MARLIN"))
        });
        if first {
            terminal.send(b"hello\r");
        }
        terminal.wait_for(Duration::from_secs(20), "folded thinking", |output| {
            let rows = screen_lines(output);
            let Some(user) = rows.iter().position(|line| line.contains("hello")) else {
                return false;
            };
            let end = if first {
                "Waiting for gateway"
            } else {
                "answer-fixture"
            };
            let Some(next) = rows.iter().position(|line| line.contains(end)) else {
                return false;
            };
            if next <= user + 1 {
                return false;
            }
            rows[user + 1..next]
                .iter()
                .any(|line| !line.trim().is_empty())
        });
        terminal.send(b"\x1bx");
        terminal.wait_for(Duration::from_secs(20), "thinking command", |output| {
            screen_lines(output)
                .iter()
                .any(|line| line.contains("toggle-thinking"))
        });
        assert!(
            !screen_lines(&terminal.output)
                .iter()
                .any(|line| line.contains("reasoning-fixture"))
        );
        terminal.send(b"toggle-thinking\r");
        terminal.wait_for(Duration::from_secs(20), "expanded thinking", |output| {
            let rows = screen_lines(output);
            rows.iter().any(|line| line.contains("reasoning-fixture"))
        });
        if first {
            assert!(
                !screen_lines(&terminal.output)
                    .iter()
                    .any(|line| line.contains("answer-fixture"))
            );
            resume_tx.send(()).unwrap();
        }
        terminal.wait_for(Duration::from_secs(20), "first answer", |output| {
            contains(output, b"answer-fixture")
        });
        if first {
            let rows = screen_lines(&terminal.output);
            assert!(rows.iter().any(|line| line.contains("hello")));
            assert!(
                !rows
                    .iter()
                    .any(|line| matches!(line.trim(), "You" | "Assistant"))
            );
        }
        thread::sleep(Duration::from_millis(500));
        let offset = terminal.output.len();
        terminal.send(b"follow-up\r");
        let expected: &[u8] = if first { b"answer-live" } else { b"restart" };
        terminal.wait_for(Duration::from_secs(20), "follow-up outcome", |output| {
            if changed {
                screen_lines(output)
                    .iter()
                    .any(|line| line.contains("Request not dispatched"))
            } else {
                contains(&output[offset..], expected)
            }
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
