use std::fs;

use marlin_rust_project_harness_policy::{
    run_marlin_rust_project_harness_for_package, rust_project_harness_config_for_project,
};
use rust_lang_project_harness::RustHarnessReport;

enum ModuleLayout {
    Flat,
    Directory,
    Both,
}

fn report_for_layout(layout: ModuleLayout) -> RustHarnessReport {
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

rust_lang_project_harness::rust_project_harness_rule_boundary_tests! {
    suite = module_source_layout_boundary,
    rule = "RUST-MOD-R007",
    cases = {
        flat_module_source_is_clean: {
            report = report_for_layout(ModuleLayout::Flat),
            expected_findings = 0,
        },
        directory_module_source_is_clean: {
            report = report_for_layout(ModuleLayout::Directory),
            expected_findings = 0,
        },
        file_and_directory_module_sources_conflict: {
            report = report_for_layout(ModuleLayout::Both),
            expected_findings = 1,
        },
    },
}
