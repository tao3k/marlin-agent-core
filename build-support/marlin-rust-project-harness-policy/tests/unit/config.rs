use std::path::{Path, PathBuf};

use marlin_rust_project_harness_policy::{
    MarlinCrateVerificationRole, marlin_crate_verification_role_for_project,
    rust_project_harness_config_for_project,
};
use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationTaskKind,
};

#[test]
fn crate_verification_role_classification_tracks_marlin_boundaries() {
    let root = workspace_root();

    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-agent-core")),
        MarlinCrateVerificationRole::AgentRuntime
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-agent-stream")),
        MarlinCrateVerificationRole::AgentRuntime
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-agent-harness")),
        MarlinCrateVerificationRole::AgentHarness
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-agent-test-support")),
        MarlinCrateVerificationRole::AgentHarness
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-agent-runtime")),
        MarlinCrateVerificationRole::AgentRuntime
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-agent-graph")),
        MarlinCrateVerificationRole::AgentTopology
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-agent-protocol")),
        MarlinCrateVerificationRole::ProtocolContract
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-agent-sessions")),
        MarlinCrateVerificationRole::ProtocolContract
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-workspace-view")),
        MarlinCrateVerificationRole::OrgWorkspace
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-org-memory")),
        MarlinCrateVerificationRole::OrgWorkspace
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-gerbil-scheme")),
        MarlinCrateVerificationRole::GerbilRuntime
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(
            &root.join("build-support/marlin-deck-runtime-native-build"),
        ),
        MarlinCrateVerificationRole::NativeBuildSupport
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(
            &root.join("build-support/marlin-rust-project-harness-policy"),
        ),
        MarlinCrateVerificationRole::BuildHarnessPolicy
    );
    assert_eq!(
        marlin_crate_verification_role_for_project(&root.join("crates/marlin-agent-environment")),
        MarlinCrateVerificationRole::GitOrEnvironmentBoundary
    );
}

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

#[test]
fn agent_harness_crate_receives_harness_verification_policy() {
    let root = workspace_root().join("crates/marlin-agent-harness");
    let config = rust_project_harness_config_for_project(&root);
    let root_hint = profile_hint(&config.verification_policy.profile_hints, "src/lib.rs");

    assert_responsibilities(
        root_hint,
        [
            RustOwnerResponsibility::PublicApi,
            RustOwnerResponsibility::LatencySensitive,
            RustOwnerResponsibility::AvailabilityCritical,
            RustOwnerResponsibility::ExternalDependency,
        ],
    );
    assert_performance_and_stability_tasks(root_hint);
    assert!(
        config
            .verification_policy
            .profile_hints
            .iter()
            .any(|hint| hint.owner_path == Path::new("src/runtime.rs"))
    );
}

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
    let root = workspace_root().join("build-support/marlin-deck-runtime-native-build");
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

#[test]
fn org_memory_crate_receives_persistence_verification_policy() {
    let root = workspace_root().join("crates/marlin-org-memory");
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
            .any(|hint| hint.owner_path == Path::new("src/memory/project_graph.rs"))
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

fn profile_hint<'a>(
    hints: &'a [RustVerificationProfileHint],
    owner_path: &str,
) -> &'a RustVerificationProfileHint {
    hints
        .iter()
        .find(|hint| hint.owner_path == Path::new(owner_path))
        .unwrap_or_else(|| panic!("missing profile hint for {owner_path}"))
}

fn assert_responsibilities<const N: usize>(
    hint: &RustVerificationProfileHint,
    responsibilities: [RustOwnerResponsibility; N],
) {
    for responsibility in responsibilities {
        assert!(
            hint.responsibilities.contains(&responsibility),
            "missing responsibility {responsibility:?} in {:?}",
            hint.responsibilities
        );
    }
}

fn assert_performance_and_stability_tasks(hint: &RustVerificationProfileHint) {
    let task_kinds = hint.task_kinds.as_ref().expect("explicit task kinds");
    assert!(task_kinds.contains(&RustVerificationTaskKind::Performance));
    assert!(task_kinds.contains(&RustVerificationTaskKind::Stability));
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("policy crate should live under workspace/build-support")
        .to_path_buf()
}
