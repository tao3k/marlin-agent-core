//! Builds Marlin's project-owned `rust-lang-project-harness` policy config.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use rust_lang_project_harness::{
    RustHarnessConfig, RustOwnerResponsibility, RustProjectHarnessDownstreamPolicy,
    RustVerificationPolicy, RustVerificationProfileHint, RustVerificationStabilityPictureConfig,
    RustVerificationTaskKind,
};

/// Marlin-specific crate role used to specialize Rust verification policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarlinCrateVerificationRole {
    AgentRuntime,
    AgentTopology,
    AgentHarness,
    ProtocolContract,
    OrgWorkspace,
    GerbilRuntime,
    NativeBuildSupport,
    BuildHarnessPolicy,
    GitOrEnvironmentBoundary,
    GeneralLibrary,
}

/// Classify a package root into the Marlin verification role used for policy injection.
pub fn marlin_crate_verification_role_for_project(
    project_root: &Path,
) -> MarlinCrateVerificationRole {
    match marlin_project_harness_package_name(project_root).as_str() {
        "marlin-agent-core"
        | "marlin-agent-kernel"
        | "marlin-agent-runtime"
        | "marlin-agent-stream" => MarlinCrateVerificationRole::AgentRuntime,
        "marlin-agent-graph" => MarlinCrateVerificationRole::AgentTopology,
        "marlin-agent-harness" | "marlin-agent-harness-types" | "marlin-agent-test-support" => {
            MarlinCrateVerificationRole::AgentHarness
        }
        "marlin-agent-protocol" | "marlin-agent-sessions" | "marlin-workspace-protocol" => {
            MarlinCrateVerificationRole::ProtocolContract
        }
        "marlin-org-memory"
        | "marlin-org-model"
        | "marlin-org-patch"
        | "marlin-org-store"
        | "marlin-org-workflow"
        | "marlin-org-workspace"
        | "marlin-workspace-patch"
        | "marlin-workspace-query"
        | "marlin-workspace-status"
        | "marlin-workspace-view" => MarlinCrateVerificationRole::OrgWorkspace,
        "marlin-deck-runtime-native" | "marlin-gerbil-ir" | "marlin-gerbil-scheme" => {
            MarlinCrateVerificationRole::GerbilRuntime
        }
        "marlin-deck-runtime-native-build" => MarlinCrateVerificationRole::NativeBuildSupport,
        "marlin-rust-project-harness-policy" => MarlinCrateVerificationRole::BuildHarnessPolicy,
        "marlin-agent-environment" | "marlin-agent-hooks" | "marlin-git-utils" => {
            MarlinCrateVerificationRole::GitOrEnvironmentBoundary
        }
        _ => MarlinCrateVerificationRole::GeneralLibrary,
    }
}

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
    let package_name = marlin_project_harness_package_name(project_root);
    let role = marlin_crate_verification_role_for_project(project_root);

    push_or_merge_profile_hint(
        &mut policy,
        role.root_verification_profile_hint(&package_name),
    );
    for hint in role.additional_verification_profile_hints(project_root, &package_name) {
        push_or_merge_profile_hint(&mut policy, hint);
    }
    if policy.stability_picture.is_none() {
        policy.stability_picture = Some(RustVerificationStabilityPictureConfig::default());
    }

    policy
}

fn marlin_project_harness_gate_label(project_root: &Path) -> String {
    let package_name = marlin_project_harness_package_name(project_root);
    format!("marlin::{package_name}")
}

fn marlin_project_harness_package_name(project_root: &Path) -> String {
    let package_name = project_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace crate");
    package_name.to_owned()
}

impl MarlinCrateVerificationRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AgentRuntime => "agent-runtime",
            Self::AgentTopology => "agent-topology",
            Self::AgentHarness => "agent-harness",
            Self::ProtocolContract => "protocol-contract",
            Self::OrgWorkspace => "org-workspace",
            Self::GerbilRuntime => "gerbil-runtime",
            Self::NativeBuildSupport => "native-build-support",
            Self::BuildHarnessPolicy => "build-harness-policy",
            Self::GitOrEnvironmentBoundary => "git-or-environment-boundary",
            Self::GeneralLibrary => "general-library",
        }
    }

    fn root_verification_profile_hint(self, package_name: &str) -> RustVerificationProfileHint {
        verification_profile_hint(
            "src/lib.rs",
            self.root_responsibilities(),
            format!(
                "{package_name} owns {role} verification evidence; require performance and stability receipts",
                role = self.agent_label(),
            ),
        )
    }

    fn additional_verification_profile_hints(
        self,
        project_root: &Path,
        package_name: &str,
    ) -> Vec<RustVerificationProfileHint> {
        self.additional_owner_profiles()
            .into_iter()
            .filter(|profile| project_root.join(profile.owner_path).is_file())
            .map(|profile| {
                verification_profile_hint(
                    profile.owner_path,
                    profile.responsibilities,
                    format!(
                        "{package_name} {role} hotspot {path} needs role-specific verification evidence",
                        role = self.agent_label(),
                        path = profile.owner_path,
                    ),
                )
            })
            .collect()
    }

    fn root_responsibilities(self) -> &'static [RustOwnerResponsibility] {
        match self {
            Self::AgentRuntime => &[
                RustOwnerResponsibility::PublicApi,
                RustOwnerResponsibility::LatencySensitive,
                RustOwnerResponsibility::AvailabilityCritical,
            ],
            Self::AgentTopology => &[
                RustOwnerResponsibility::PublicApi,
                RustOwnerResponsibility::PureDomainLogic,
            ],
            Self::AgentHarness => &[
                RustOwnerResponsibility::PublicApi,
                RustOwnerResponsibility::LatencySensitive,
                RustOwnerResponsibility::AvailabilityCritical,
                RustOwnerResponsibility::ExternalDependency,
            ],
            Self::ProtocolContract => &[
                RustOwnerResponsibility::PublicApi,
                RustOwnerResponsibility::PureDomainLogic,
            ],
            Self::OrgWorkspace => &[
                RustOwnerResponsibility::PureDomainLogic,
                RustOwnerResponsibility::Persistence,
            ],
            Self::GerbilRuntime => &[
                RustOwnerResponsibility::ExternalDependency,
                RustOwnerResponsibility::SecurityBoundary,
                RustOwnerResponsibility::LatencySensitive,
                RustOwnerResponsibility::AvailabilityCritical,
            ],
            Self::NativeBuildSupport => &[
                RustOwnerResponsibility::ExternalDependency,
                RustOwnerResponsibility::SecurityBoundary,
            ],
            Self::BuildHarnessPolicy => &[
                RustOwnerResponsibility::PublicApi,
                RustOwnerResponsibility::PureDomainLogic,
                RustOwnerResponsibility::SecurityBoundary,
            ],
            Self::GitOrEnvironmentBoundary => &[
                RustOwnerResponsibility::ExternalDependency,
                RustOwnerResponsibility::SecurityBoundary,
                RustOwnerResponsibility::AvailabilityCritical,
            ],
            Self::GeneralLibrary => &[
                RustOwnerResponsibility::PublicApi,
                RustOwnerResponsibility::PureDomainLogic,
            ],
        }
    }

    fn additional_owner_profiles(self) -> Vec<MarlinVerificationOwnerProfile> {
        match self {
            Self::AgentRuntime => vec![
                MarlinVerificationOwnerProfile::new(
                    "src/graph_loop.rs",
                    &[
                        RustOwnerResponsibility::LatencySensitive,
                        RustOwnerResponsibility::AvailabilityCritical,
                    ],
                ),
                MarlinVerificationOwnerProfile::new(
                    "src/model_route/session.rs",
                    &[RustOwnerResponsibility::PublicApi],
                ),
                MarlinVerificationOwnerProfile::new(
                    "src/tokio_runtime/handle.rs",
                    &[
                        RustOwnerResponsibility::LatencySensitive,
                        RustOwnerResponsibility::AvailabilityCritical,
                    ],
                ),
            ],
            Self::AgentTopology => vec![MarlinVerificationOwnerProfile::new(
                "src/lib.rs",
                &[
                    RustOwnerResponsibility::PublicApi,
                    RustOwnerResponsibility::PureDomainLogic,
                ],
            )],
            Self::AgentHarness => vec![
                MarlinVerificationOwnerProfile::new(
                    "src/runtime.rs",
                    &[
                        RustOwnerResponsibility::LatencySensitive,
                        RustOwnerResponsibility::AvailabilityCritical,
                    ],
                ),
                MarlinVerificationOwnerProfile::new(
                    "src/stability.rs",
                    &[RustOwnerResponsibility::AvailabilityCritical],
                ),
            ],
            Self::ProtocolContract => vec![
                MarlinVerificationOwnerProfile::new(
                    "src/project_runtime/mod.rs",
                    &[
                        RustOwnerResponsibility::PublicApi,
                        RustOwnerResponsibility::PureDomainLogic,
                    ],
                ),
                MarlinVerificationOwnerProfile::new(
                    "src/graph/loop_event.rs",
                    &[
                        RustOwnerResponsibility::PublicApi,
                        RustOwnerResponsibility::PureDomainLogic,
                    ],
                ),
            ],
            Self::OrgWorkspace => vec![
                MarlinVerificationOwnerProfile::new(
                    "src/memory/project_graph.rs",
                    &[
                        RustOwnerResponsibility::PureDomainLogic,
                        RustOwnerResponsibility::Persistence,
                    ],
                ),
                MarlinVerificationOwnerProfile::new(
                    "src/memory/workspace.rs",
                    &[RustOwnerResponsibility::Persistence],
                ),
            ],
            Self::GerbilRuntime => vec![
                MarlinVerificationOwnerProfile::new(
                    "src/resident_runtime.rs",
                    &[
                        RustOwnerResponsibility::ExternalDependency,
                        RustOwnerResponsibility::AvailabilityCritical,
                    ],
                ),
                MarlinVerificationOwnerProfile::new(
                    "src/working_copy_policy.rs",
                    &[RustOwnerResponsibility::SecurityBoundary],
                ),
            ],
            Self::GitOrEnvironmentBoundary => vec![
                MarlinVerificationOwnerProfile::new(
                    "src/working_copy.rs",
                    &[
                        RustOwnerResponsibility::ExternalDependency,
                        RustOwnerResponsibility::AvailabilityCritical,
                    ],
                ),
                MarlinVerificationOwnerProfile::new(
                    "src/config.rs",
                    &[RustOwnerResponsibility::SecurityBoundary],
                ),
            ],
            Self::NativeBuildSupport | Self::BuildHarnessPolicy | Self::GeneralLibrary => {
                Vec::new()
            }
        }
    }

    fn agent_label(self) -> &'static str {
        match self {
            Self::AgentRuntime => "agent runtime",
            Self::AgentTopology => "agent topology",
            Self::AgentHarness => "agent harness",
            Self::ProtocolContract => "protocol contract",
            Self::OrgWorkspace => "Org workspace",
            Self::GerbilRuntime => "Gerbil/native runtime",
            Self::NativeBuildSupport => "native build support",
            Self::BuildHarnessPolicy => "Rust harness policy",
            Self::GitOrEnvironmentBoundary => "Git/environment boundary",
            Self::GeneralLibrary => "general library",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MarlinVerificationOwnerProfile {
    owner_path: &'static str,
    responsibilities: &'static [RustOwnerResponsibility],
}

impl MarlinVerificationOwnerProfile {
    const fn new(
        owner_path: &'static str,
        responsibilities: &'static [RustOwnerResponsibility],
    ) -> Self {
        Self {
            owner_path,
            responsibilities,
        }
    }
}

fn verification_profile_hint(
    owner_path: impl Into<PathBuf>,
    responsibilities: &'static [RustOwnerResponsibility],
    rationale: impl Into<String>,
) -> RustVerificationProfileHint {
    RustVerificationProfileHint::new(owner_path, responsibilities.iter().copied())
        .with_task_kinds([
            RustVerificationTaskKind::Performance,
            RustVerificationTaskKind::Stability,
        ])
        .with_rationale(rationale)
}

fn push_or_merge_profile_hint(
    policy: &mut RustVerificationPolicy,
    hint: RustVerificationProfileHint,
) {
    if let Some(existing) = policy
        .profile_hints
        .iter_mut()
        .find(|existing| existing.owner_path == hint.owner_path)
    {
        existing.responsibilities.extend(hint.responsibilities);
        merge_task_kinds(&mut existing.task_kinds, hint.task_kinds);
        if existing.rationale.is_none() {
            existing.rationale = hint.rationale;
        }
    } else {
        policy.profile_hints.push(hint);
    }
}

fn merge_task_kinds(
    existing: &mut Option<BTreeSet<RustVerificationTaskKind>>,
    incoming: Option<BTreeSet<RustVerificationTaskKind>>,
) {
    let Some(incoming) = incoming else {
        return;
    };
    match existing {
        Some(existing) => existing.extend(incoming),
        None => *existing = Some(incoming),
    }
}
