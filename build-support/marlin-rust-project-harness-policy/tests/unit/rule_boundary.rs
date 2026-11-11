// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::fs;

use crate::policy::{
    run_marlin_rust_project_harness_for_package, rust_project_harness_config_for_project,
};
use asp_rust::AspRustReport;

enum ModuleLayout {
    Flat,
    Directory,
    Both,
}

fn report_for_layout(layout: ModuleLayout) -> AspRustReport {
    let project = tempfile::tempdir().expect("temporary Rust project");
    let root = project.path();
    fs::create_dir_all(root.join("src")).expect("create source root");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"module-layout-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write package manifest");
    fs::write(root.join("src/lib.rs"), "mod domain;\n").expect("write crate root");

    if matches!(layout, ModuleLayout::Flat | ModuleLayout::Both) {
        fs::write(root.join("src/domain.rs"), "pub struct Domain;\n").expect("write flat module");
    }
    if matches!(layout, ModuleLayout::Directory | ModuleLayout::Both) {
        fs::create_dir_all(root.join("src/domain")).expect("create directory module");
        fs::write(root.join("src/domain/mod.rs"), "pub struct Domain;\n")
            .expect("write directory module");
    }

    let config = rust_project_harness_config_for_project(root);
    run_marlin_rust_project_harness_for_package(root, &config).expect("Marlin harness report")
}

#[test]
fn module_source_layout_boundary() {
    for (layout, expected) in [
        (ModuleLayout::Flat, 0),
        (ModuleLayout::Directory, 0),
        (ModuleLayout::Both, 1),
    ] {
        let report = report_for_layout(layout);
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|finding| finding.rule_id == "RUST-MOD-R007")
                .count(),
            expected,
            "module layout report: {:?}",
            report.findings,
        );
    }
}
