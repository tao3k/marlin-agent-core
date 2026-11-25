// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// ASP Rust's default severity contract blocks Warning and Error. The TUI
// still carries Info-only owner-boundary advice; do not turn non-blocking
// advice into a test failure while the owner refactor proceeds.
asp_rust::asp_rust_cargo_test_gate!(advice = allow);

/// Architectural invariants preventing custom Rust UI rendering bypasses.
///
/// Marlin's Ratatui UI mapping framework (`marlin-agent-ui::UiElement` +
/// `crates/marlin-agent-tui/src/ui_elements/`) is the SINGLE generic renderer.
/// New UI features, bars, tabs, overlays, and pure-UI commands MUST be
/// declared in Scheme and projected through `UiElement` + `UiCommandCapability::SchemeAction`
/// without adding bespoke Ratatui widget renderers or domain-specific Rust UI enums.
mod declarative_ui_boundary_guard {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
            .map(|entry| entry.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    #[test]
    fn ratatui_widget_rendering_stays_inside_generic_ui_elements_engine() {
        let tui_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect_rs_files(&tui_src, &mut files);

        // Strictly bounded allowlist of files permitted to touch `ratatui::widgets`
        // or call `.render_widget(`. Zero domain-specific renderers are allowed;
        // every UI feature (including Helm and Task Tabs) projects to `UiElement`s
        // rendered through `ui_elements`.
        let allowed_widget_owners = [
            "ui_elements/core.rs",
            "ui_elements/widgets.rs",
            "render_main.rs",
        ];

        let mut violations = Vec::new();
        for file in files {
            let rel = file
                .strip_prefix(&tui_src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if allowed_widget_owners.contains(&rel.as_str()) {
                continue;
            }
            let src = fs::read_to_string(&file).unwrap();
            if src.contains("ratatui::widgets") || src.contains(".render_widget(") {
                violations.push(format!(
                    "{rel}: direct Ratatui widget rendering bypasses generic `ui_elements` engine"
                ));
            }
            if rel.ends_with("_render.rs")
                || rel.ends_with("_tabs.rs")
                || rel.ends_with("_tab_bar.rs")
                || rel.ends_with("_overlay.rs")
            {
                violations.push(format!(
                    "{rel}: bespoke UI renderer module forbidden; express UI in Scheme `UiElement`s"
                ));
            }
        }

        assert!(
            violations.is_empty(),
            "Custom Rust UI bypass detected! Route UI presentation through Scheme `UiElement`s and `ui_elements::render_with_slots`:\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn render_pipeline_contains_no_domain_specific_ui_bypass() {
        let render_rs = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/render.rs");
        let src = fs::read_to_string(&render_rs).unwrap();
        for forbidden in [
            "ratatui::widgets",
            "task_tabs",
            "render_workbench_tab_strip",
            "split_task_tab_strip_area",
            "TaskActor",
            "graph_engine",
        ] {
            assert!(
                !src.contains(forbidden),
                "render.rs must stay a generic Scheme UiElement layout pipeline; forbidden domain bypass `{forbidden}` found"
            );
        }
    }

    #[test]
    fn ui_command_capability_stays_generic_and_forbids_bespoke_ui_feature_variants() {
        let commands_rs =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../marlin-agent-ui/src/model/commands.rs");
        let src = fs::read_to_string(&commands_rs).unwrap();

        assert!(
            src.contains("SchemeAction"),
            "UiCommandCapability must expose generic `SchemeAction` so pure-UI features never touch Rust enums"
        );
        for forbidden in ["TaskTabs", "TaskWorkbench", "Detour", "GraphTab", "TabBar"] {
            assert!(
                !src.contains(forbidden),
                "UiCommandCapability must NOT add domain-specific UI variant `{forbidden}`; declare it in Scheme with capability `scheme-action`"
            );
        }
    }
}
