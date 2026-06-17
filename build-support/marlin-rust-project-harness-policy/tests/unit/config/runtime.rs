use std::path::Path;

use marlin_rust_project_harness_policy::rust_project_harness_config_for_project;
use rust_lang_project_harness::RustOwnerResponsibility;

use super::helpers::{
    assert_performance_and_stability_tasks, assert_responsibilities, profile_hint, workspace_root,
};

#[test]
fn runtime_crate_receives_runtime_specific_verification_policy() {
    let root = workspace_root().join("crates/marlin-agent-runtime");
    let config = rust_project_harness_config_for_project(&root);
    let root_hint = profile_hint(&config.verification_policy.profile_hints, "src/lib.rs");

    assert_responsibilities(
        root_hint,
        [
            RustOwnerResponsibility::PublicApi,
            RustOwnerResponsibility::LatencySensitive,
            RustOwnerResponsibility::AvailabilityCritical,
        ],
    );
    assert_performance_and_stability_tasks(root_hint);
    assert!(
        config
            .verification_policy
            .profile_hints
            .iter()
            .any(|hint| hint.owner_path == Path::new("src/graph_loop.rs"))
    );
}

#[test]
fn gerbil_crate_receives_native_runtime_verification_policy() {
    let root = workspace_root().join("crates/marlin-gerbil-scheme");
    let config = rust_project_harness_config_for_project(&root);
    let root_hint = profile_hint(&config.verification_policy.profile_hints, "src/lib.rs");

    assert_responsibilities(
        root_hint,
        [
            RustOwnerResponsibility::ExternalDependency,
            RustOwnerResponsibility::SecurityBoundary,
            RustOwnerResponsibility::LatencySensitive,
            RustOwnerResponsibility::AvailabilityCritical,
        ],
    );
    assert_performance_and_stability_tasks(root_hint);
    assert!(
        config
            .verification_policy
            .profile_hints
            .iter()
            .any(|hint| hint.owner_path == Path::new("src/working_copy_policy.rs"))
    );
}
