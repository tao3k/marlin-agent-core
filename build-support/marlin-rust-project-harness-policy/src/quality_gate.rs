//! Package-level Rust harness quality gate receipts.

use rust_lang_project_harness::{RustVerificationPlan, RustVerificationTaskKind};

/// Structured package-level gate receipt for required Rust harness evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustProjectHarnessGateReceipt {
    /// Cargo package name evaluated by the build gate.
    pub package_name: String,
    /// Whether an active performance verification task is configured.
    pub performance_gate: bool,
    /// Whether an active stability verification task is configured.
    pub stability_gate: bool,
    /// Whether the performance report obligation is configured.
    pub performance_report_obligation: bool,
    /// Whether the stability report obligation is configured.
    pub stability_report_obligation: bool,
}

impl RustProjectHarnessGateReceipt {
    /// Returns true when all required package-level quality gates are present.
    pub fn is_success(&self) -> bool {
        self.performance_gate
            && self.stability_gate
            && self.performance_report_obligation
            && self.stability_report_obligation
    }
}

/// Evaluates required package-level performance and stability gates.
pub fn evaluate_performance_and_stability_gate(
    plan: &RustVerificationPlan,
    package_name: impl Into<String>,
) -> RustProjectHarnessGateReceipt {
    let package_name = package_name.into();
    RustProjectHarnessGateReceipt {
        package_name,
        performance_gate: has_active_verification_task(plan, RustVerificationTaskKind::Performance),
        stability_gate: has_active_verification_task(plan, RustVerificationTaskKind::Stability),
        performance_report_obligation: has_report_obligation(plan, "performance_index_json"),
        stability_report_obligation: has_report_obligation(plan, "stability_index_json"),
    }
}

fn has_active_verification_task(
    plan: &RustVerificationPlan,
    task_kind: RustVerificationTaskKind,
) -> bool {
    plan.tasks
        .iter()
        .any(|task| task.kind == task_kind && task.is_active())
}

fn has_report_obligation(plan: &RustVerificationPlan, report_obligation_key: &str) -> bool {
    plan.report_obligations
        .iter()
        .any(|obligation| obligation.key == report_obligation_key)
}
