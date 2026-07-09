//! Run receipt artifact rendering for intent-case bundles.

use crate::{
    intent_case_artifact_real_llm::real_llm_run_receipt_lines,
    intent_case_artifact_receipt_header::{artifact_kind_name, render_key_value_artifact_receipt},
};
use marlin_agent_harness_types::{
    IntentCaseArtifactKind, IntentCaseArtifactManifest, IntentCaseRunReceipt, IntentCaseRunStatus,
};
use marlin_gerbil_scheme::GerbilLoopCaseDriverRealLlmCaseReceipt;

pub(crate) fn render_run_receipt_artifact(
    manifest: &IntentCaseArtifactManifest,
    real_llm_case_receipt: Option<&GerbilLoopCaseDriverRealLlmCaseReceipt>,
) -> String {
    let receipt = IntentCaseRunReceipt::passed(manifest.clone());
    let expected_artifacts = receipt.manifest.expected_artifact_kinds();
    let materialized_artifacts = receipt.manifest.present_artifact_kinds();
    let expected_spans = receipt.manifest.expected_span_names();
    let observed_spans = receipt.manifest.observed_span_names();
    let missing_spans = expected_spans
        .iter()
        .filter(|span_name| !observed_spans.contains(span_name))
        .collect::<Vec<_>>();
    let missing_trace_artifact_refs = receipt.manifest.trace_artifact_ref_missing_ids();
    let missing_runtime_owners = receipt.manifest.trace_entries_without_runtime_owner();
    let missing_action_identities = receipt.manifest.trace_entries_without_action_identity();
    let expected_lanes = artifact_lane_names(&expected_artifacts);
    let materialized_lanes = artifact_lane_names(&materialized_artifacts);

    let mut lines = vec![
        format!("run_receipt_schema={}", receipt.schema_id),
        format!("run_receipt_manifest_schema={}", receipt.manifest.schema_id),
        format!(
            "run_receipt_status={}",
            run_receipt_status_name(receipt.status)
        ),
        format!("run_receipt_case_id={}", receipt.manifest.case_id),
        format!("run_receipt_run_id={}", receipt.manifest.run_id),
        format!(
            "run_receipt_policy_digest={}",
            receipt.manifest.policy_digest
        ),
        format!(
            "run_receipt_loop_program_id={}",
            receipt.manifest.loop_program_id
        ),
        format!(
            "run_receipt_expected_artifact_count={}",
            expected_artifacts.len()
        ),
        format!(
            "run_receipt_materialized_artifact_count={}",
            materialized_artifacts.len()
        ),
        format!("run_receipt_expected_artifact_lanes={expected_lanes}"),
        format!("run_receipt_materialized_artifact_lanes={materialized_lanes}"),
        format!("run_receipt_expected_span_count={}", expected_spans.len()),
        format!("run_receipt_observed_span_count={}", observed_spans.len()),
        format!("run_receipt_missing_span_count={}", missing_spans.len()),
        format!(
            "run_receipt_trace_entry_count={}",
            receipt.manifest.trace_index.entries.len()
        ),
        format!(
            "run_receipt_correlation_key_count={}",
            receipt.manifest.correlation_keys().len()
        ),
        format!(
            "run_receipt_missing_trace_artifact_ref_count={}",
            missing_trace_artifact_refs.len()
        ),
        format!(
            "run_receipt_missing_runtime_owner_count={}",
            missing_runtime_owners.len()
        ),
        format!(
            "run_receipt_missing_action_identity_count={}",
            missing_action_identities.len()
        ),
        format!(
            "run_receipt_complete_trace_correlation={}",
            receipt.manifest.has_complete_trace_correlation()
        ),
        format!("run_receipt_diagnostic_count={}", receipt.diagnostics.len()),
        "run_receipt_internal_json_boundary=false".to_owned(),
    ];
    lines.extend(real_llm_run_receipt_lines(real_llm_case_receipt));

    render_key_value_artifact_receipt(manifest, IntentCaseArtifactKind::RunReceipt, lines)
}

fn artifact_lane_names(kinds: &[IntentCaseArtifactKind]) -> String {
    kinds
        .iter()
        .map(|kind| artifact_kind_name(*kind))
        .collect::<Vec<_>>()
        .join(",")
}

fn run_receipt_status_name(status: IntentCaseRunStatus) -> &'static str {
    match status {
        IntentCaseRunStatus::Passed => "passed",
        IntentCaseRunStatus::Failed => "failed",
        IntentCaseRunStatus::Incomplete => "incomplete",
    }
}
