// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::path::Path;

use crate::policy::rust_project_harness_config_for_project;
use asp_rust::RustOwnerResponsibility;

use super::helpers::{
    assert_performance_and_stability_tasks, assert_responsibilities, profile_hint, workspace_root,
};

#[test]
fn context_crate_receives_persistence_verification_policy() {
    let root = workspace_root().join("crates/marlin-agent-context");
    let config = rust_project_harness_config_for_project(&root);
    let root_hint = profile_hint(&config.verification_policy.profile_hints, "src/lib.rs");

    assert_responsibilities(
        root_hint,
        [
            RustOwnerResponsibility::PureDomainLogic,
            RustOwnerResponsibility::Persistence,
        ],
    );
    assert_performance_and_stability_tasks(root_hint);
    assert!(
        config
            .verification_policy
            .profile_hints
            .iter()
            .any(|hint| hint.owner_path == Path::new("src/org/memory/project_graph.rs"))
    );
}
