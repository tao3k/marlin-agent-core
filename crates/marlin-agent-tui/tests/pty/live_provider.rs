// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::*;

#[test]
#[ignore = "explicit opt-in: sends one real request using the current Anthropic environment"]
fn configured_anthropic_provider_responds() {
    assert_eq!(std::env::var("MARLIN_LIVE_PROVIDER").as_deref(), Ok("1"));
    assert!(std::env::var_os("ANTHROPIC_API_KEY").is_some());
    assert!(std::env::var_os("ANTHROPIC_BASE_URL").is_some());
    let config = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("example.toml");
    let mut terminal = Terminal::start(&config);
    // This is a live-connectivity diagnostic. It deliberately does not replace
    // the separate, strict ten-second startup acceptance test.
    terminal.wait_for(Duration::from_secs(90), "workbench", |output| {
        screen_lines(output)[29].contains("Message: Enter send")
    });
    terminal.send(b"Answer using digits only: 71821 + 389 = ?\r");
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let screen = screen_lines(&terminal.output);
        if screen.iter().any(|line| line.contains("72210"))
            && screen.iter().any(|line| line.contains("Idle"))
        {
            eprintln!("real provider response completed");
            break;
        }
        if screen.iter().any(|line| line.trim() == "Error") {
            panic!("real provider returned an error; transcript omitted");
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "timed out waiting for real provider response"
        );
        match terminal.received.recv_timeout(remaining) {
            Ok(chunk) => terminal.output.extend_from_slice(&chunk),
            Err(_) => panic!("provider response did not complete before the deadline"),
        }
    }
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
}
