//! Manifest validation and rendering for intent-case artifact bundles.

use crate::intent_case_artifact_error::IntentCaseArtifactBundleMaterializationError;
use marlin_agent_harness_types::{
    IntentCaseArtifactCompletenessReceipt, IntentCaseArtifactManifest,
};

pub(crate) fn ensure_trace_correlation_integrity(
    manifest: &IntentCaseArtifactManifest,
) -> Result<(), IntentCaseArtifactBundleMaterializationError> {
    if let Some(trace_id) = manifest
        .trace_entries_without_runtime_owner()
        .into_iter()
        .next()
    {
        return Err(
            IntentCaseArtifactBundleMaterializationError::MissingTraceRuntimeOwner { trace_id },
        );
    }
    if let Some(trace_id) = manifest
        .trace_entries_without_action_identity()
        .into_iter()
        .next()
    {
        return Err(
            IntentCaseArtifactBundleMaterializationError::MissingTraceActionIdentity { trace_id },
        );
    }

    for entry in &manifest.trace_index.entries {
        if let Some(artifact_id) = entry
            .artifact_refs
            .iter()
            .find(|artifact_id| !manifest.has_present_artifact_id(artifact_id))
        {
            return Err(
                IntentCaseArtifactBundleMaterializationError::UnknownTraceArtifactRef {
                    trace_id: entry.trace_id.clone(),
                    artifact_id: artifact_id.clone(),
                },
            );
        }
    }

    if manifest.correlation_keys().is_empty() {
        return Err(IntentCaseArtifactBundleMaterializationError::EmptyTraceCorrelationIndex);
    }

    Ok(())
}

pub(crate) fn render_manifest_receipt(
    manifest: &IntentCaseArtifactManifest,
    completeness_receipt: &IntentCaseArtifactCompletenessReceipt,
) -> String {
    let mut lines = vec![
        "schema=marlin.intent-case.artifact-materialization.v1".to_owned(),
        format!("manifest_schema={}", manifest.schema_id),
        format!("completeness_schema={}", completeness_receipt.schema_id),
        format!("case_id={}", manifest.case_id),
        format!("run_id={}", manifest.run_id),
        format!("policy_epoch={}", manifest.policy_epoch),
        format!("policy_digest={}", manifest.policy_digest),
        format!("loop_program_id={}", manifest.loop_program_id),
        format!("artifact_count={}", manifest.artifacts.len()),
        format!(
            "expected_artifact_count={}",
            completeness_receipt.expected_artifacts.len()
        ),
        format!(
            "materialized_artifact_count={}",
            completeness_receipt.materialized_artifacts.len()
        ),
        format!(
            "missing_artifact_count={}",
            completeness_receipt.missing_artifacts.len()
        ),
        format!(
            "expected_span_count={}",
            completeness_receipt.expected_spans.len()
        ),
        format!(
            "observed_span_count={}",
            completeness_receipt.observed_spans.len()
        ),
        format!(
            "missing_span_count={}",
            completeness_receipt.missing_spans.len()
        ),
        format!(
            "completeness_status={}",
            completeness_receipt.status.as_str()
        ),
        format!("trace_entry_count={}", manifest.trace_index.entries.len()),
        format!(
            "correlation_key_count={}",
            manifest.correlation_keys().len()
        ),
    ];
    lines.extend(manifest.artifacts.iter().map(|artifact| {
        format!(
            "artifact id={} kind={:?} present={} path={}",
            artifact.artifact_id,
            artifact.kind,
            artifact.present,
            artifact.path.as_deref().unwrap_or("none")
        )
    }));
    lines.extend(
        completeness_receipt
            .expected_artifacts
            .iter()
            .map(|kind| format!("expected_artifact kind={kind:?}")),
    );
    lines.extend(
        completeness_receipt
            .expected_spans
            .iter()
            .map(|span_name| format!("expected_span name={span_name}")),
    );
    lines.extend(
        completeness_receipt
            .observed_spans
            .iter()
            .map(|span_name| format!("observed_span name={span_name}")),
    );
    lines.extend(
        completeness_receipt
            .missing_spans
            .iter()
            .map(|span_name| format!("missing_span name={span_name}")),
    );
    lines.extend(manifest.correlation_keys().iter().map(|key| {
        format!(
            "correlation case_id={} run_id={} policy_epoch={} policy_digest={} loop_program_id={} trace_id={} step_index={} transition_id={} action={} event={} runtime_owner={} model_invocation_id={} tool_call_id={} resource_key={} sandbox_profile={} artifact_id={}",
            key.case_id,
            key.run_id,
            key.policy_epoch,
            key.policy_digest,
            key.loop_program_id,
            key.trace_id,
            key.step_index,
            key.transition_id,
            key.action,
            key.event,
            key.runtime_owner,
            key.model_invocation_id
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or("none"),
            key.tool_call_id
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or("none"),
            key.resource_key
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or("none"),
            key.sandbox_profile
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or("none"),
            key.artifact_id
        )
    }));
    lines.join("\n") + "\n"
}
