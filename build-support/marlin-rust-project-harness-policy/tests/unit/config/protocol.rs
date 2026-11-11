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
fn protocol_crate_receives_protocol_contract_verification_policy() {
    let root = workspace_root().join("crates/marlin-agent-protocol");
    let config = rust_project_harness_config_for_project(&root);
    let root_hint = profile_hint(&config.verification_policy.profile_hints, "src/lib.rs");

    assert_responsibilities(
        root_hint,
        [
            RustOwnerResponsibility::PublicApi,
            RustOwnerResponsibility::PureDomainLogic,
        ],
    );
    assert_performance_and_stability_tasks(root_hint);
    assert!(
        config
            .verification_policy
            .profile_hints
            .iter()
            .any(|hint| hint.owner_path == Path::new("src/project_runtime/mod.rs"))
    );
}

#[test]
fn session_crate_receives_protocol_contract_verification_policy() {
    let root = workspace_root().join("crates/marlin-agent-sessions");
    let config = rust_project_harness_config_for_project(&root);
    let root_hint = profile_hint(&config.verification_policy.profile_hints, "src/lib.rs");

    assert_responsibilities(
        root_hint,
        [
            RustOwnerResponsibility::PublicApi,
            RustOwnerResponsibility::PureDomainLogic,
        ],
    );
    assert_performance_and_stability_tasks(root_hint);
}
