use std::path::Path;

use crate::policy::rust_project_harness_config_for_project;
use asp_rust::RustOwnerResponsibility;

use super::helpers::{
    assert_performance_and_stability_tasks, assert_responsibilities, profile_hint, workspace_root,
};

#[test]
fn agent_harness_crate_receives_harness_verification_policy() {
    let root = workspace_root().join("crates/marlin-agent-harness");
    let config = rust_project_harness_config_for_project(&root);
    let advice_explanation = config
        .cargo_check_advice_allow_explanation
        .as_deref()
        .expect("Marlin should retain non-blocking Info advice in typed receipts");
    let root_hint = profile_hint(&config.verification_policy.profile_hints, "src/lib.rs");

    assert!(
        [
            "scope=",
            "owner=",
            "finding_category=",
            "why_safe_now=",
            "cleanup_trigger=",
        ]
        .into_iter()
        .all(|field| advice_explanation.contains(field))
    );
    assert!(
        config
            .blocking_severities
            .contains(&asp_rust::RustDiagnosticSeverity::Warning)
    );
    assert!(
        config
            .blocking_severities
            .contains(&asp_rust::RustDiagnosticSeverity::Error)
    );
    assert!(
        !config
            .blocking_severities
            .contains(&asp_rust::RustDiagnosticSeverity::Info)
    );

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
