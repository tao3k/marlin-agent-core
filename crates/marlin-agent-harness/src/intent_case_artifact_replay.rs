//! Replay script receipt rendering for intent-case bundles.

use crate::intent_case_artifact_receipt_header::{
    artifact_kind_name, artifact_receipt_header_lines,
};
use marlin_agent_harness_types::{
    IntentCaseArtifactKind, IntentCaseArtifactManifest, IntentCaseSpanName,
};

pub(crate) const INTENT_CASE_REPLAY_RECEIPT_SCHEMA_ID: &str =
    "marlin.intent-case.replay-receipt.v1";

pub(crate) fn render_replay_script_artifact(manifest: &IntentCaseArtifactManifest) -> String {
    let expected_artifacts = manifest
        .expected_artifact_kinds()
        .into_iter()
        .map(artifact_kind_name)
        .collect::<Vec<_>>()
        .join(",");
    let expected_spans = manifest
        .expected_span_names()
        .iter()
        .map(IntentCaseSpanName::as_str)
        .collect::<Vec<_>>()
        .join(",");

    let mut lines = vec![
        "#!/usr/bin/env sh".to_owned(),
        "set -eu".to_owned(),
        "# replay-intent-case".to_owned(),
    ];
    lines.extend(artifact_receipt_header_lines(
        manifest,
        IntentCaseArtifactKind::ReplayScript,
    ));
    lines.extend([
        format!("replay_receipt_schema={INTENT_CASE_REPLAY_RECEIPT_SCHEMA_ID}"),
        format!("replay_case_id={}", manifest.case_id),
        format!("replay_run_id={}", manifest.run_id),
        format!("replay_policy_epoch={}", manifest.policy_epoch),
        format!("replay_policy_digest={}", manifest.policy_digest),
        format!("replay_loop_program_id={}", manifest.loop_program_id),
        format!(
            "replay_expected_artifact_count={}",
            manifest.expected_artifact_kinds().len()
        ),
        format!("replay_expected_artifact_lanes={expected_artifacts}"),
        format!("replay_expected_span_count={}", manifest.expected_span_names().len()),
        format!("replay_expected_span_names={expected_spans}"),
        format!("replay_trace_entry_count={}", manifest.trace_index.entries.len()),
        format!("replay_correlation_key_count={}", manifest.correlation_keys().len()),
        "replay_internal_json_boundary=false".to_owned(),
        "replay_command='direnv exec . rtk --ultra-compact cargo test -p marlin-agent-harness intent_case'".to_owned(),
    ]);

    format!("{}\n", lines.join("\n"))
}
