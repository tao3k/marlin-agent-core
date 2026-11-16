// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{Duration, Terminal, fs, mx_position, screen_lines};

#[test]
#[ignore = "requires the compiled independent ui-extension Scheme package in the package environment"]
fn compiled_scheme_package_adds_a_buffer_command_and_key_without_native_handlers() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    fs::write(&config, format!(
        "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n[tui_ui_package]\ndirectory={:?}\nmodule=\":marlin-tui-ui-test/fixture\"\nexport=\"test-publication\"\n", root.to_str().unwrap()
    )).unwrap();
    let mut terminal = Terminal::start(&config);
    terminal.wait_status("Message: Enter send");
    terminal.send(b"\x1bz");
    terminal.wait_for(Duration::from_secs(5), "package buffer", |output| {
        screen_lines(output)
            .iter()
            .any(|row| row.contains("Scheme package window"))
    });
    terminal.send(b"\x1b[<0;3;4M\r");
    terminal.wait_for(
        Duration::from_secs(5),
        "scheme row method focused commands",
        |output| {
            screen_lines(output)
                .iter()
                .any(|row| mx_position(row).is_some())
        },
    );
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
}
