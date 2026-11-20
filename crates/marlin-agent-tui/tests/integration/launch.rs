// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Diagnostic for time before Marlin can open a terminal or initialize Scheme.
//! The PTY first-frame test remains the end-to-end startup gate.

#[cfg(target_os = "macos")]
#[test]
#[ignore = "manual fresh-file launch diagnostic; build target/debug/marlin-tui first"]
fn linked_debug_pre_ui_launch_pair() {
    linked_pre_ui_launch_pair("debug");
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "manual fresh-file launch diagnostic; build target/release/marlin-tui first"]
fn linked_release_pre_ui_launch_pair() {
    linked_pre_ui_launch_pair("release");
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "manual fresh-file launch diagnostic; build target/release-size/marlin-tui first"]
fn linked_release_size_pre_ui_launch_pair() {
    linked_pre_ui_launch_pair("release-size");
}

#[cfg(target_os = "macos")]
fn linked_pre_ui_launch_pair(profile: &str) {
    use std::{
        fs,
        io::{BufRead, BufReader},
        path::Path,
        process::{Command, Stdio},
        time::Instant,
    };

    static LAUNCH_BENCH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _serial = LAUNCH_BENCH_LOCK.lock().expect("launch benchmark lock");

    let binary = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target")
        .join(profile)
        .join("marlin-tui");
    assert!(binary.is_file(), "build the linked {profile} binary first");
    let directory = tempfile::tempdir().expect("private launch directory");
    let copied = directory.path().join("marlin-tui");
    fs::copy(&binary, &copied).expect("fresh executable copy");

    for launch in ["fresh", "same-file"] {
        let start = Instant::now();
        let mut child = Command::new(&copied)
            .arg(directory.path().join("absent-config.toml"))
            .env("MARLIN_TUI_STARTUP_TRACE", "1")
            .env("DYLD_PRINT_LIBRARIES", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("launch copied Marlin executable");
        let spawned = start.elapsed();
        let mut first_image = None;
        let mut main = None;
        let mut terminal_error = false;
        let stderr = child.stderr.take().expect("launch stderr");
        for line in BufReader::new(stderr).lines() {
            let line = line.expect("launch diagnostics");
            if line.starts_with("dyld[") {
                first_image.get_or_insert_with(|| start.elapsed());
            } else if line == "marlin-tui startup Rust main" {
                main = Some(start.elapsed());
            } else if line.contains("requires a terminal") {
                terminal_error = true;
            }
        }
        assert!(!child.wait().expect("pre-UI exit").success());
        assert!(terminal_error, "pre-UI exit must reject a missing terminal");
        let main = main.expect("Rust entry marker");
        // Dyld tracing can be suppressed by the platform's signing policy.
        let loader_to_main = first_image.and_then(|image| main.checked_sub(image));
        eprintln!(
            "linked {profile} {launch} pre-UI launch: spawn={spawned:?}, first image={first_image:?}, Rust main={main:?}, image to main={loader_to_main:?}, exit={:?}",
            start.elapsed()
        );
    }

    let control = directory.path().join("true");
    fs::copy("/usr/bin/true", &control).expect("fresh control executable");
    for launch in ["fresh", "same-file"] {
        let start = Instant::now();
        let output = Command::new(&control)
            .output()
            .expect("launch copied control executable");
        assert!(output.status.success());
        eprintln!("small executable {launch} control: {:?}", start.elapsed());
    }
}
