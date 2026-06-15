//! Agent-readable crate-role verification policy receipts.

use std::path::Path;

use serde::Serialize;

use rust_lang_project_harness::{
    RustHarnessConfig, RustVerificationPlan, RustVerificationTaskKind,
};

use crate::marlin_crate_verification_role_for_project;

/// Agent-readable receipt describing why this crate has its verification shape.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessVerificationPolicyReceipt {
    pub schema_id: String,
    pub schema_version: String,
    pub package_name: String,
    pub crate_role: String,
    pub owner_profiles: Vec<RustProjectHarnessVerificationOwnerProfileReceipt>,
    pub performance_task_count: usize,
    pub stability_task_count: usize,
    pub performance_report_obligation: bool,
    pub stability_report_obligation: bool,
}

/// One owner-specific verification policy projection for agents.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessVerificationOwnerProfileReceipt {
    pub owner_path: String,
    pub responsibilities: Vec<String>,
    pub task_kinds: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// Builds the agent-readable Marlin verification policy receipt for a crate.
pub fn build_verification_policy_receipt(
    package_name: impl Into<String>,
    project_root: &Path,
    config: &RustHarnessConfig,
    plan: &RustVerificationPlan,
) -> RustProjectHarnessVerificationPolicyReceipt {
    let package_name = package_name.into();
    let role = marlin_crate_verification_role_for_project(project_root);
    let mut owner_profiles = config
        .verification_policy
        .profile_hints
        .iter()
        .map(|hint| RustProjectHarnessVerificationOwnerProfileReceipt {
            owner_path: hint.owner_path.display().to_string(),
            responsibilities: hint
                .responsibilities
                .iter()
                .map(debug_kebab_label)
                .collect(),
            task_kinds: hint
                .task_kinds
                .as_ref()
                .map(|task_kinds| task_kinds.iter().map(debug_kebab_label).collect())
                .unwrap_or_default(),
            rationale: hint.rationale.clone(),
        })
        .collect::<Vec<_>>();
    owner_profiles.sort_by(|left, right| left.owner_path.cmp(&right.owner_path));

    RustProjectHarnessVerificationPolicyReceipt {
        schema_id: "marlin.rust-project-harness.verification-policy".to_owned(),
        schema_version: "1".to_owned(),
        package_name,
        crate_role: role.as_str().to_owned(),
        owner_profiles,
        performance_task_count: active_verification_task_count(
            plan,
            RustVerificationTaskKind::Performance,
        ),
        stability_task_count: active_verification_task_count(
            plan,
            RustVerificationTaskKind::Stability,
        ),
        performance_report_obligation: has_report_obligation(plan, "performance_index_json"),
        stability_report_obligation: has_report_obligation(plan, "stability_index_json"),
    }
}

fn active_verification_task_count(
    plan: &RustVerificationPlan,
    task_kind: RustVerificationTaskKind,
) -> usize {
    plan.tasks
        .iter()
        .filter(|task| task.kind == task_kind && task.is_active())
        .count()
}

fn has_report_obligation(plan: &RustVerificationPlan, report_obligation_key: &str) -> bool {
    plan.report_obligations
        .iter()
        .any(|obligation| obligation.key == report_obligation_key)
}

fn debug_kebab_label(value: impl std::fmt::Debug) -> String {
    let debug_label = format!("{value:?}");
    let mut label = String::with_capacity(debug_label.len());

    for (index, character) in debug_label.chars().enumerate() {
        if character.is_uppercase() {
            if index > 0 {
                label.push('-');
            }
            for lower in character.to_lowercase() {
                label.push(lower);
            }
        } else if character == '_' {
            label.push('-');
        } else {
            label.push(character);
        }
    }

    label
}
