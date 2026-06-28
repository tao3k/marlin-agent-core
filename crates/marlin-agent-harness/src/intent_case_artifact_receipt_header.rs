//! Shared typed receipt headers for intent-case artifacts.

use marlin_agent_harness_types::{IntentCaseArtifactKind, IntentCaseArtifactManifest};

pub(crate) const INTENT_CASE_ARTIFACT_RECEIPT_SCHEMA_ID: &str =
    "marlin.intent-case.artifact-receipt.v1";

pub(crate) fn render_key_value_artifact_receipt(
    manifest: &IntentCaseArtifactManifest,
    kind: IntentCaseArtifactKind,
    body_lines: impl IntoIterator<Item = String>,
) -> String {
    let mut lines = artifact_receipt_header_lines(manifest, kind);
    lines.extend(body_lines);
    format!("{}\n", lines.join("\n"))
}

pub(crate) fn render_patch_artifact_receipt_header(
    manifest: &IntentCaseArtifactManifest,
    kind: IntentCaseArtifactKind,
) -> Vec<String> {
    artifact_receipt_header_lines(manifest, kind)
        .into_iter()
        .map(|line| format!("# {line}"))
        .collect()
}

pub(crate) fn render_org_artifact_receipt_properties(
    manifest: &IntentCaseArtifactManifest,
    kind: IntentCaseArtifactKind,
) -> String {
    artifact_receipt_header_lines(manifest, kind)
        .into_iter()
        .map(|line| {
            let (key, value) = line
                .split_once('=')
                .expect("artifact receipt header lines use key=value shape");
            let property_key = key.replace('-', "_").to_ascii_uppercase();
            format!(":{property_key}: {value}\n")
        })
        .collect()
}

pub(crate) fn artifact_kind_name(kind: IntentCaseArtifactKind) -> &'static str {
    match kind {
        IntentCaseArtifactKind::Intent => "intent",
        IntentCaseArtifactKind::PolicyPack => "policy-pack",
        IntentCaseArtifactKind::PolicyMergeReceipts => "policy-merge-receipts",
        IntentCaseArtifactKind::LoopProgram => "loop-program",
        IntentCaseArtifactKind::VerticalTrace => "vertical-trace",
        IntentCaseArtifactKind::ExecutionTrace => "execution-trace",
        IntentCaseArtifactKind::ModelEvents => "model-events",
        IntentCaseArtifactKind::ToolCalls => "tool-calls",
        IntentCaseArtifactKind::SandboxReceipts => "sandbox-receipts",
        IntentCaseArtifactKind::MemoryReceipts => "memory-receipts",
        IntentCaseArtifactKind::DiffPatch => "diff-patch",
        IntentCaseArtifactKind::TestBefore => "test-before",
        IntentCaseArtifactKind::TestAfter => "test-after",
        IntentCaseArtifactKind::VerifierReceipt => "verifier-receipt",
        IntentCaseArtifactKind::PolicyExplanation => "policy-explanation",
        IntentCaseArtifactKind::ReplayScript => "replay-script",
        IntentCaseArtifactKind::RunReceipt => "run-receipt",
    }
}

pub(crate) fn artifact_receipt_header_lines(
    manifest: &IntentCaseArtifactManifest,
    kind: IntentCaseArtifactKind,
) -> Vec<String> {
    vec![
        format!("artifact_receipt_schema={INTENT_CASE_ARTIFACT_RECEIPT_SCHEMA_ID}"),
        format!("artifact_kind={}", artifact_kind_name(kind)),
        format!(
            "artifact_id={}",
            artifact_id_for_kind(manifest, kind).unwrap_or("none")
        ),
        format!("case_id={}", manifest.case_id),
        format!("run_id={}", manifest.run_id),
        format!("policy_epoch={}", manifest.policy_epoch),
        format!("policy_digest={}", manifest.policy_digest),
        format!("loop_program_id={}", manifest.loop_program_id),
        format!("trace_entry_count={}", manifest.trace_index.entries.len()),
        format!(
            "correlation_key_count={}",
            manifest.correlation_keys().len()
        ),
        "internal_json_boundary=false".to_owned(),
    ]
}

fn artifact_id_for_kind(
    manifest: &IntentCaseArtifactManifest,
    kind: IntentCaseArtifactKind,
) -> Option<&str> {
    manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == kind && artifact.present)
        .map(|artifact| artifact.artifact_id.as_str())
}
