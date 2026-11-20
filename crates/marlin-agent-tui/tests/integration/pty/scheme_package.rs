// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{Duration, Instant, Terminal, fs, screen_lines};
use std::path::Path;

#[test]
#[ignore = "requires the compiled independent ui-extension Scheme package in the package environment"]
fn compiled_scheme_package_adds_a_buffer_command_and_key_without_native_handlers() {
    verify_compiled_scheme_package(Path::new(env!("CARGO_BIN_EXE_marlin-tui")), b"\x1bz");
}

#[cfg(feature = "linked-ui")]
#[test]
#[ignore = "requires the linked Release binary and the compiled independent Scheme package"]
fn release_linked_compiled_scheme_package_loads_actions_on_demand() {
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/release/marlin-tui");
    verify_compiled_scheme_package(&binary, b"\x1bxopen-notes\r");
    verify_compiled_scheme_package(&binary, b"\x1bz");
    verify_compiled_scheme_package(&binary, b" z");
}

fn verify_compiled_scheme_package(binary: &Path, lookup: &[u8]) {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    let receipt = directory.path().join("row-action.receipt");
    let publication = directory.path().join("publication.receipt");
    let copy = directory.path().join("marlin-tui");
    fs::copy(binary, &copy).expect("copy linked TUI");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    fs::write(&config, format!(
        "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n[tui_ui_package]\ndirectory={:?}\nmodule=\":marlin-tui-ui-test/fixture\"\nexport=\"test-publication\"\n", root.to_str().unwrap()
    )).unwrap();
    let variables = [
        (
            "MARLIN_TEST_UI_AUTOLOAD_RECEIPT",
            receipt.to_str().unwrap().to_owned(),
        ),
        (
            "MARLIN_TEST_UI_PUBLICATION_RECEIPT",
            publication.to_str().unwrap().to_owned(),
        ),
    ];
    let startup_limit = Duration::from_secs(10);
    let mut terminal = Terminal::start_in_binary(&copy, &config, &[], &variables, 30, 100, &root);
    terminal.wait_for(
        startup_limit.saturating_sub(terminal.start.elapsed()),
        "compiled package first frame",
        |output| screen_lines(output)[29].contains("Message: Enter send"),
    );
    let first_frame = terminal.start.elapsed();
    assert!(first_frame < startup_limit);
    eprintln!("Scheme package cold first frame: {first_frame:?}");
    assert!(!publication.exists(), "publication loaded before first use");
    assert!(!receipt.exists(), "row action loaded before first use");
    terminal.enter_normal_mode();
    assert!(
        !publication.exists(),
        "an ordinary mode change loaded the package"
    );
    let activation_start = Instant::now();
    terminal.send(lookup);
    terminal.wait_for(Duration::from_secs(5), "package buffer", |output| {
        screen_lines(output)
            .iter()
            .any(|row| row.contains("Scheme package window"))
    });
    eprintln!(
        "Scheme package first lookup: {:?}",
        activation_start.elapsed()
    );
    assert_eq!(
        fs::read_to_string(&publication).unwrap(),
        "UI-PUBLICATION-LOADED\n"
    );
    assert!(!receipt.exists(), "row action loaded before first use");
    let mut expected_receipt = String::from("ROW-ACTION-LOADED\n");
    for call in ["first", "repeat"] {
        let row = screen_lines(&terminal.output)
            .iter()
            .position(|line| line.contains("First Scheme row"))
            .expect("first package row is visible")
            + 1;
        let start = Instant::now();
        expected_receipt.push_str("ROW-ACTION-CALLED\n");
        terminal.send(format!("\x1b[<0;3;{row}M").as_bytes());
        terminal.wait_until(Duration::from_secs(5), "package action receipt", |_| {
            fs::read_to_string(&receipt).as_deref().ok() == Some(expected_receipt.as_str())
        });
        eprintln!("Scheme package {call} action: {:?}", start.elapsed());
    }
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(5));
    terminal.assert_restored();
    assert_eq!(
        fs::read_to_string(publication).unwrap(),
        "UI-PUBLICATION-LOADED\n"
    );
    assert_eq!(fs::read_to_string(receipt).unwrap(), expected_receipt);
}
