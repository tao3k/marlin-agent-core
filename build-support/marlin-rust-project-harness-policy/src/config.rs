//! Builds Marlin's project-owned `rust-lang-project-harness` policy config.

use std::path::{Path, PathBuf};

use rust_lang_project_harness::{
    RustHarnessConfig, RustOwnerResponsibility, RustProjectHarnessDownstreamPolicy,
    RustVerificationPolicy, RustVerificationProfileHint, RustVerificationStabilityPictureConfig,
    RustVerificationTaskKind,
};

/// Build Marlin's complete downstream project harness policy.
pub fn rust_project_harness_policy_for_project(
    project_root: &Path,
) -> RustProjectHarnessDownstreamPolicy {
    RustProjectHarnessDownstreamPolicy::new(
        marlin_project_harness_gate_label(project_root),
        rust_project_harness_config_for_project(project_root),
    )
}

/// Build the Rust project harness config with Marlin's workspace verification policy.
pub fn rust_project_harness_config_for_project(project_root: &Path) -> RustHarnessConfig {
    let mut config = rust_lang_project_harness::rust_harness_config_for_project(project_root);
    config.verification_policy =
        with_marlin_project_verification_policy(config.verification_policy, project_root);
    config
}

fn with_marlin_project_verification_policy(
    mut policy: RustVerificationPolicy,
    project_root: &Path,
) -> RustVerificationPolicy {
    let owner_path = PathBuf::from("src/lib.rs");
    let has_crate_root_gate = policy.profile_hints.iter().any(|hint| {
        hint.owner_path == owner_path
            && hint.task_kinds.as_ref().is_some_and(|task_kinds| {
                task_kinds.contains(&RustVerificationTaskKind::Performance)
                    && task_kinds.contains(&RustVerificationTaskKind::Stability)
            })
    });

    if !has_crate_root_gate {
        policy
            .profile_hints
            .push(crate_root_verification_profile_hint(
                project_root,
                owner_path,
            ));
    }
    if policy.stability_picture.is_none() {
        policy.stability_picture = Some(RustVerificationStabilityPictureConfig::default());
    }

    policy
}

fn marlin_project_harness_gate_label(project_root: &Path) -> String {
    let package_name = project_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace crate");
    format!("marlin::{package_name}")
}

fn crate_root_verification_profile_hint(
    project_root: &Path,
    owner_path: PathBuf,
) -> RustVerificationProfileHint {
    let package_name = project_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace crate");

    RustVerificationProfileHint::new(
        owner_path,
        [
            RustOwnerResponsibility::LatencySensitive,
            RustOwnerResponsibility::AvailabilityCritical,
        ],
    )
    .with_task_kinds([
        RustVerificationTaskKind::Performance,
        RustVerificationTaskKind::Stability,
    ])
    .with_rationale(format!(
        "{package_name} owns crate-level performance and long-run stability evidence",
    ))
}
