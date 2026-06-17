use std::path::Path;

use marlin_rust_project_harness_policy::rust_project_harness_config_for_project;
use rust_lang_project_harness::RustOwnerResponsibility;

use super::helpers::{
    assert_performance_and_stability_tasks, assert_responsibilities, profile_hint, workspace_root,
};

#[test]
fn environment_crate_receives_git_environment_boundary_policy() {
    let root = workspace_root().join("crates/marlin-agent-environment");
    let config = rust_project_harness_config_for_project(&root);
    let root_hint = profile_hint(&config.verification_policy.profile_hints, "src/lib.rs");

    assert_responsibilities(
        root_hint,
        [
            RustOwnerResponsibility::ExternalDependency,
            RustOwnerResponsibility::SecurityBoundary,
            RustOwnerResponsibility::AvailabilityCritical,
        ],
    );
    assert_performance_and_stability_tasks(root_hint);
    assert!(
        config
            .verification_policy
            .profile_hints
            .iter()
            .any(|hint| hint.owner_path == Path::new("src/working_copy.rs"))
    );
}

#[test]
fn native_build_crate_receives_external_build_policy() {
    let root = workspace_root().join("build-support/marlin-gerbil-native-build");
    let config = rust_project_harness_config_for_project(&root);
    let root_hint = profile_hint(&config.verification_policy.profile_hints, "src/lib.rs");

    assert_responsibilities(
        root_hint,
        [
            RustOwnerResponsibility::ExternalDependency,
            RustOwnerResponsibility::SecurityBoundary,
        ],
    );
    assert_performance_and_stability_tasks(root_hint);
}
