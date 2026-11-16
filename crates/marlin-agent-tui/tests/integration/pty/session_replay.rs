// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{StopOnDrop, Terminal, accept_until, contains, read_request, screen_lines};
use std::{
    fs,
    io::Write,
    net::TcpListener,
    process::{Command, Stdio},
    sync::{Arc, Mutex, atomic::AtomicBool},
    thread,
    time::{Duration, Instant},
};

fn wait_raw(terminal: &mut Terminal, expected: &[u8]) {
    let text = std::str::from_utf8(expected).unwrap();
    terminal.wait_for(Duration::from_secs(20), text, |output| {
        contains(output, expected) || screen_lines(output).iter().any(|line| line.contains(text))
    });
}

fn run_turn(config: &std::path::Path, session: &str, prompt: &[u8], answer: &[u8]) {
    let mut terminal = Terminal::start_with_args(config, &["--session", session]);
    terminal.wait_for(Duration::from_secs(20), "Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    wait_raw(&mut terminal, session.as_bytes());
    terminal.send(prompt);
    terminal.send(b"\r");
    wait_raw(&mut terminal, answer);
    thread::sleep(Duration::from_millis(500));
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
}

fn listing(config: &std::path::Path, options: &[&str]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_marlin-tui"))
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .arg(config)
        .args(options)
        .env_remove("MARLIN_TUI_FIXTURE_KEY")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("session listing exceeded ten seconds");
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.wait_with_output().unwrap()
}

fn text_content(message: &serde_json::Value) -> String {
    let content = &message["content"];
    if let Some(text) = content.as_str() {
        return text.to_owned();
    }
    content
        .as_array()
        .unwrap()
        .iter()
        .filter(|part| part["type"] == "output_text")
        .map(|part| part["text"].as_str().unwrap())
        .collect()
}

#[test]
#[ignore = "persistent multi-session PTY and loopback acceptance; requires the built Scheme package"]
fn separate_sessions_preserve_history_and_route_authority() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let requests = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    let observed = requests.clone();
    let server = thread::spawn(move || {
        for answer in [
            "answer-one",
            "answer-foreign",
            "answer-two",
            "answer-probe",
            "answer-three",
            "answer-four",
            "answer-five",
        ] {
            let Some(mut stream) = accept_until(&listener, &stop) else {
                return;
            };
            let (_, request) = read_request(&mut stream);
            observed.lock().unwrap().push(request);
            let events = [
                serde_json::json!({"type":"response.output_text.delta", "delta":answer}),
                serde_json::json!({"type":"response.completed", "response":{"id":"fixture", "model":"gpt-fixture", "status":"completed", "output":[{"type":"message", "role":"assistant", "content":[{"type":"output_text", "text":answer}]}]}}),
            ];
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n").unwrap();
            for event in events {
                write!(stream, "data: {event}\n\n").unwrap();
            }
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    let database = directory.path().join("session.db");
    let db = serde_json::to_string(&database.to_string_lossy()).unwrap();
    let contents = format!(
        "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n[session]\npath={db}\nproject=\"project:pty\"\nsession=\"session:pty\"\nagent=\"agent:pty\"\n[models.alt]\nprovider=\"openai\"\nmodel=\"gpt-fixture-alt\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n"
    );
    fs::write(&config, &contents).unwrap();
    let foreign = directory.path().join("foreign.toml");
    fs::write(
        &foreign,
        contents.replace("agent=\"agent:pty\"", "agent=\"agent:foreign\""),
    )
    .unwrap();
    run_turn(&config, "session:A", b"first", b"answer-one");
    run_turn(&foreign, "session:foreign", b"foreign", b"answer-foreign");

    let mut terminal = Terminal::start_with_args(&config, &["--session", "session:B"]);
    terminal.wait_for(Duration::from_secs(20), "Modeline", |output| {
        screen_lines(output)[28].contains("MARLIN")
    });
    wait_raw(&mut terminal, b"session:B");
    terminal.send(b"second\r");
    wait_raw(&mut terminal, b"answer-two");
    thread::sleep(Duration::from_millis(500));
    terminal.send(b"\x0fsession \"session:foreign\"\r");
    terminal.wait_for(Duration::from_secs(20), "failed session switch", |output| {
        screen_lines(output)[29].contains("Session switch failed")
    });
    terminal.send(b"\x18bmessages\r");
    terminal.wait_for(
        Duration::from_secs(20),
        "retained session diagnostic",
        |output| {
            let rows = screen_lines(output);
            rows[29].contains("*Messages*")
                && rows[..28]
                    .iter()
                    .any(|line| line.contains("Current session retained"))
        },
    );
    terminal.send(b"\x07");
    terminal.wait_for(Duration::from_secs(20), "session B restored", |output| {
        screen_lines(output)[29].contains("Message: Enter send")
    });
    terminal.send(b"probe\r");
    wait_raw(&mut terminal, b"answer-probe");
    thread::sleep(Duration::from_millis(500));
    terminal.send(b"\x0fsessions\r");
    terminal.wait_for(Duration::from_secs(20), "Sessions Buffer", |output| {
        let rows = screen_lines(output);
        rows[29].contains("*Sessions*") && rows[..28].iter().any(|line| line.contains("session:A"))
    });
    terminal.send(b"\x0fsession \"session:A\"\r");
    wait_raw(&mut terminal, b"answer-one");
    terminal.send(b"third\x0fcontext-budget\r");
    terminal.wait_for(Duration::from_secs(20), "context budget notice", |output| {
        screen_lines(output)[29].contains("Context budget")
    });
    terminal.send(b"\x18bmessages\r");
    terminal.wait_for(
        Duration::from_secs(20),
        "context budget details",
        |output| {
            let rows = screen_lines(output);
            rows[29].contains("*Messages*")
                && rows[..28]
                    .iter()
                    .any(|line| line.contains("Next request accounting"))
        },
    );
    terminal.send(b"\x07");
    assert_eq!(
        requests.lock().unwrap().len(),
        4,
        "context command dispatched a request"
    );
    terminal.send(b"\r");
    wait_raw(&mut terminal, b"answer-three");
    thread::sleep(Duration::from_millis(500));
    terminal.send(b"blocked\x0finspect \"session:A\"\r");
    wait_raw(&mut terminal, b"Read-only");
    terminal.send(b"\r");
    terminal.wait_for(
        Duration::from_secs(20),
        "read-only history notice",
        |output| screen_lines(output)[29].contains("Read-only history"),
    );
    thread::sleep(Duration::from_millis(500));
    assert_eq!(
        requests.lock().unwrap().len(),
        5,
        "read-only inspection dispatched a request"
    );
    terminal.send(b"\x7f\x7f\x7f\x7f\x7f\x7f\x7f\x0fsession \"session:A\"\r");
    thread::sleep(Duration::from_millis(500));
    terminal.send(b"fourth\r");
    wait_raw(&mut terminal, b"answer-four");
    thread::sleep(Duration::from_millis(500));
    let offset = terminal.output.len();
    terminal.send(b"draft-b\x0fsession \"session:B\"\r");
    terminal.wait_for(Duration::from_secs(20), "session B replay", |output| {
        contains(&output[offset..], b"probe")
    });
    let offset = terminal.output.len();
    terminal.send(b"\x0fmodel alt\r\r");
    terminal.wait_for(
        Duration::from_secs(20),
        "incompatible continuation",
        |output| screen_lines(output)[29].contains("Incompatible continuation"),
    );
    terminal.send(b"\x18bmessages\r");
    terminal.wait_for(
        Duration::from_secs(20),
        "continuation diagnostic",
        |output| {
            let rows = screen_lines(output);
            rows[29].contains("*Messages*")
                && rows[..28]
                    .iter()
                    .any(|line| line.contains("draft retained"))
        },
    );
    terminal.send(b"\x07");
    terminal.wait_for(Duration::from_secs(20), "conversation restored", |output| {
        screen_lines(output)[29].contains("Message: Enter send")
    });
    assert_eq!(
        requests.lock().unwrap().len(),
        6,
        "incompatible continuation dispatched a request"
    );
    terminal.send(b"\x0fmodel default\r\r");
    terminal.wait_for(Duration::from_secs(20), "final answer", |output| {
        contains(&output[offset..], b"five")
    });
    thread::sleep(Duration::from_millis(500));
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    server.join().unwrap();

    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 7);
    for (index, expected) in [
        (2, vec!["second"]),
        (3, vec!["second", "answer-two", "probe"]),
        (4, vec!["first", "answer-one", "third"]),
        (
            5,
            vec!["first", "answer-one", "third", "answer-three", "fourth"],
        ),
        (
            6,
            vec!["second", "answer-two", "probe", "answer-probe", "draft-b"],
        ),
    ] {
        let actual: Vec<String> = requests[index]["input"]
            .as_array()
            .unwrap()
            .iter()
            .map(text_content)
            .collect();
        assert_eq!(actual, expected);
    }
    assert_eq!(requests[5]["model"], "gpt-fixture");
    assert_eq!(requests[6]["model"], "gpt-fixture");
    let list = listing(&config, &["--list-sessions"]);
    assert!(list.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&list.stdout).unwrap(),
        serde_json::json!({"items":["session:A","session:B"], "next_cursor":null})
    );
    let after = listing(&config, &["--list-sessions", "--after", "session:A"]);
    assert!(after.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&after.stdout).unwrap(),
        serde_json::json!({"items":["session:B"], "next_cursor":null})
    );
    let missing_db = directory.path().join("missing.db");
    let missing = directory.path().join("missing.toml");
    fs::write(
        &missing,
        contents.replace(
            &db,
            &serde_json::to_string(&missing_db.to_string_lossy()).unwrap(),
        ),
    )
    .unwrap();
    let result = listing(&missing, &["--list-sessions"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(contains(&result.stderr, b"session database does not exist"));
    assert!(!missing_db.exists());
}
