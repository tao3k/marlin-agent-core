// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{Duration, Instant, Terminal, fs, mx_position, screen_lines};
use std::path::Path;

#[test]
#[ignore = "requires the compiled independent ui-extension Scheme package in the package environment"]
fn compiled_scheme_package_adds_a_buffer_command_and_key_without_native_handlers() {
    verify_compiled_scheme_package(Path::new(env!("CARGO_BIN_EXE_marlin-tui")), b"\x1bz", false);
}

#[cfg(feature = "linked-ui")]
#[test]
#[ignore = "requires the linked Release binary and the compiled independent Scheme package"]
fn release_linked_compiled_scheme_package_loads_actions_on_demand() {
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/release/marlin-tui");
    verify_compiled_scheme_package(&binary, b"\x1bx", false);
    verify_compiled_scheme_package(&binary, b"\x1bz", false);
    verify_compiled_scheme_package(&binary, b" z", false);
}

#[cfg(feature = "linked-ui")]
#[test]
#[ignore = "requires the linked Release binary and both compiled package declarations"]
fn release_linked_command_catalog_ab() {
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/release/marlin-tui");
    for eager in [true, false, false, true, true, false] {
        verify_compiled_scheme_package(&binary, b"\x1bx", eager);
    }
}

fn verify_compiled_scheme_package(binary: &Path, lookup: &[u8], eager: bool) {
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
    let (module, export) = if eager {
        ("eager-catalog", "eager-catalog")
    } else {
        ("catalog", "test-catalog")
    };
    fs::write(&config, format!(
        "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://127.0.0.1:1/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n[tui_ui_package]\ndirectory={:?}\nmodule=\":marlin-tui-ui-test/{module}\"\nexport=\"{export}\"\n", root.to_str().unwrap()
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
    let activation_start = if lookup == b"\x1bx" {
        let menu_start = Instant::now();
        terminal.send(lookup);
        terminal.wait_for(Duration::from_secs(5), "first M-x menu", |output| {
            screen_lines(output)
                .iter()
                .any(|row| mx_position(row).is_some())
        });
        eprintln!(
            "M-x first painted menu ({module}): {:?}",
            menu_start.elapsed()
        );
        if eager {
            assert_eq!(
                fs::read_to_string(&publication).unwrap(),
                "UI-PUBLICATION-LOADED\n"
            );
        } else {
            assert!(!publication.exists(), "opening M-x initialized the package");
        }
        terminal.send(b"open-notes");
        terminal.wait_for(
            Duration::from_secs(5),
            "declared command and description",
            |output| {
                screen_lines(output)
                    .iter()
                    .any(|row| row.contains("open-notes") && row.contains("Open package notes"))
            },
        );
        if !eager {
            assert!(
                !publication.exists(),
                "completing a name initialized the package"
            );
        }
        let now = Instant::now();
        terminal.send(b"\r");
        now
    } else {
        let now = Instant::now();
        terminal.send(lookup);
        now
    };
    terminal.wait_for(Duration::from_secs(5), "package buffer", |output| {
        screen_lines(output)
            .iter()
            .any(|row| row.contains("Scheme package window"))
    });
    eprintln!(
        "Scheme package first execution ({module}): {:?}",
        activation_start.elapsed()
    );
    assert_eq!(
        fs::read_to_string(&publication).unwrap(),
        "UI-PUBLICATION-LOADED\n"
    );
    if lookup == b"\x1bx" {
        terminal.send(b"\x1bxopen-notes");
        terminal.wait_for(Duration::from_secs(5), "repeat command menu", |output| {
            let rows = screen_lines(output);
            rows.iter().any(|row| mx_position(row).is_some())
                && rows.iter().any(|row| row.contains("Open package notes"))
        });
        terminal.send(b"\r");
        terminal.wait_for(
            Duration::from_secs(5),
            "repeat command completion",
            |output| {
                let rows = screen_lines(output);
                rows.iter().all(|row| mx_position(row).is_none())
                    && rows.iter().any(|row| row.contains("Scheme package window"))
            },
        );
        assert_eq!(
            fs::read_to_string(&publication).unwrap(),
            "UI-PUBLICATION-LOADED\n"
        );
    }
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
