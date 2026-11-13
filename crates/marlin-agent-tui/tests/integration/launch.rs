// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Diagnostic for time before Marlin can open a terminal or initialize Scheme.
//! The PTY first-frame test remains the end-to-end startup gate.

#[cfg(target_os = "macos")]
#[test]
#[ignore = "manual fresh-file launch diagnostic; build target/release/marlin-tui first"]
fn linked_release_pre_ui_launch_pair() {
    use std::{fs, path::Path, process::Command, time::Instant};

    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/release/marlin-tui");
    assert!(binary.is_file(), "build the linked Release binary first");
    let directory = tempfile::tempdir().expect("private launch directory");
    let copied = directory.path().join("marlin-tui");
    fs::copy(&binary, &copied).expect("fresh executable copy");

    for launch in ["fresh", "same-file"] {
        let start = Instant::now();
        let output = Command::new(&copied)
            .arg(directory.path().join("absent-config.toml"))
            .output()
            .expect("launch copied Marlin executable");
        let elapsed = start.elapsed();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("requires a terminal"),
            "early exit changed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        eprintln!("linked Release {launch} pre-UI launch: {elapsed:?}");
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
