// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::path::Path;

use asp_rust::RustOwnerResponsibility;

use crate::policy::rust_project_harness_config_for_project;

use super::helpers::{
    assert_performance_and_stability_tasks, assert_responsibilities, profile_hint, workspace_root,
};

#[test]
fn tui_policy_targets_binary_entry_and_interactive_hotspots() {
    let root = workspace_root().join("crates/marlin-agent-tui");
    let config = rust_project_harness_config_for_project(&root);
    let hints = &config.verification_policy.profile_hints;
    let entry = profile_hint(hints, "src/main.rs");
    assert_responsibilities(
        entry,
        [
            RustOwnerResponsibility::LatencySensitive,
            RustOwnerResponsibility::AvailabilityCritical,
            RustOwnerResponsibility::ExternalDependency,
        ],
    );
    assert_performance_and_stability_tasks(entry);
    for path in ["src/app.rs", "src/ui_host.rs", "src/render.rs"] {
        assert!(hints.iter().any(|hint| hint.owner_path == Path::new(path)));
    }
}

#[test]
fn shared_ui_policy_targets_protocol_and_effect_boundary() {
    let root = workspace_root().join("crates/marlin-agent-ui");
    let config = rust_project_harness_config_for_project(&root);
    let entry = profile_hint(&config.verification_policy.profile_hints, "src/lib.rs");
    assert_responsibilities(
        entry,
        [
            RustOwnerResponsibility::PublicApi,
            RustOwnerResponsibility::PureDomainLogic,
            RustOwnerResponsibility::SecurityBoundary,
        ],
    );
    assert_performance_and_stability_tasks(entry);
}
