// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::resolve_local_scheme_prefix;
use std::fs;

fn seed_scheme_package(root: &std::path::Path) -> std::path::PathBuf {
    let package = root.join("scheme");
    let prefix = package.join(".gerbil");
    fs::create_dir_all(prefix.join("lib/marlin-agent-core")).expect("create scheme prefix");
    fs::write(package.join("gerbil.pkg"), "(package: marlin-agent-core)\n").expect("write pkg");
    fs::write(prefix.join("lib/marlin-agent-core/init.ssi"), "prelude:\n").expect("write init");
    prefix
}

#[test]
fn polluted_gerbil_path_falls_back_to_workspace_scheme_prefix() {
    let dir = tempfile::tempdir().expect("tempdir");
    let prefix = seed_scheme_package(dir.path());
    let foreign = dir.path().join("foreign-gerbil");
    fs::create_dir_all(&foreign).expect("foreign dir");

    assert_eq!(
        resolve_local_scheme_prefix(Some(&foreign), dir.path(), None),
        Some(prefix)
    );
}

#[test]
fn valid_gerbil_path_does_not_reexec() {
    let dir = tempfile::tempdir().expect("tempdir");
    let prefix = seed_scheme_package(dir.path());

    assert_eq!(
        resolve_local_scheme_prefix(Some(&prefix), dir.path(), None),
        None
    );
}

#[test]
fn resolves_repo_scheme_prefix_relative_to_target_binary_when_cwd_differs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let prefix = seed_scheme_package(dir.path());
    let exe = dir.path().join("target/debug/marlin-tui");
    let outside = tempfile::tempdir().expect("outside cwd");

    assert_eq!(
        resolve_local_scheme_prefix(None, outside.path(), Some(&exe)),
        Some(prefix)
    );
}
