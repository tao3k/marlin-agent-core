use marlin_rust_project_harness_policy::{
    MarlinCrateVerificationRole, marlin_crate_verification_role_for_project,
};

use super::helpers::workspace_root;

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
        marlin_crate_verification_role_for_project(
            &root.join("crates/marlin-agent-policy-routing-native"),
        ),
        MarlinCrateVerificationRole::GerbilRuntime
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
            &root.join("build-support/marlin-gerbil-native-build"),
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
