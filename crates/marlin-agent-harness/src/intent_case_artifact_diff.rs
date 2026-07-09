//! Diff artifact rendering for intent-case bundles.

use crate::intent_case_artifact_receipt_header::render_patch_artifact_receipt_header;
use marlin_agent_harness_types::{
    IntentCaseArtifactKind, IntentCaseArtifactManifest, RuntimeRepairCaseReceipt,
};
use marlin_agent_kernel::LoopProgramExecutionReplayBundleReceipt;

pub(crate) fn render_diff_artifact(
    manifest: &IntentCaseArtifactManifest,
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
    runtime_repair_receipt: Option<&RuntimeRepairCaseReceipt>,
) -> String {
    if let Some(replay_bundle) = side_effect_replay_bundle {
        return render_side_effect_diff_artifact(manifest, replay_bundle);
    }
    if let Some(receipt) = runtime_repair_receipt {
        return render_runtime_repair_diff_artifact(manifest, receipt);
    }

    let mut lines = vec!["diff --git a/scripted-intent-case b/scripted-intent-case".to_owned()];
    lines.extend(render_patch_artifact_receipt_header(
        manifest,
        IntentCaseArtifactKind::DiffPatch,
    ));
    lines.push("# scripted run did not apply a live model patch".to_owned());
    format!("{}\n", lines.join("\n"))
}

fn render_runtime_repair_diff_artifact(
    manifest: &IntentCaseArtifactManifest,
    receipt: &RuntimeRepairCaseReceipt,
) -> String {
    let mut lines = vec![format!(
        "diff --git a/runtime-repair/{} b/runtime-repair/{}",
        manifest.case_id, manifest.case_id
    )];
    lines.extend(render_patch_artifact_receipt_header(
        manifest,
        IntentCaseArtifactKind::DiffPatch,
    ));
    match receipt {
        RuntimeRepairCaseReceipt::Live(receipt) => {
            lines.push("# runtime_repair_patch_projection=live-digest-summary".to_owned());
            lines.push(format!(
                "# runtime_repair_patch_tool_success={}",
                receipt.patch_tool_success
            ));
            lines.push(format!(
                "# runtime_repair_tool_projection_count={}",
                receipt.tool_projection_count.get()
            ));
            lines.push(format!(
                "# runtime_repair_verification_success={}",
                receipt.verification_success
            ));
            lines.push(format!(
                "# runtime_repair_repaired_content_digest={}",
                receipt.repaired_content.digest
            ));
            lines.push(format!(
                "# runtime_repair_repaired_content_bytes={}",
                receipt.repaired_content.byte_count.get()
            ));
            lines.push("# runtime_repair_internal_json_boundary=false".to_owned());
        }
        RuntimeRepairCaseReceipt::NoLive(receipt) => {
            lines.push("# runtime_repair_patch_projection=blocked".to_owned());
            lines.push(format!(
                "# runtime_repair_gate_status={:?}",
                receipt.gate_status
            ));
            lines.push(format!(
                "# runtime_repair_denial_reason={}",
                receipt.denial_reason
            ));
            lines.push("# runtime_repair_internal_json_boundary=false".to_owned());
        }
    }
    lines.join("\n") + "\n"
}

fn render_side_effect_diff_artifact(
    manifest: &IntentCaseArtifactManifest,
    replay_bundle: &LoopProgramExecutionReplayBundleReceipt,
) -> String {
    let mut lines = vec![format!(
        "diff --git a/intent-case/{} b/intent-case/{}",
        manifest.case_id, manifest.case_id
    )];
    lines.extend(render_patch_artifact_receipt_header(
        manifest,
        IntentCaseArtifactKind::DiffPatch,
    ));
    for step_bundle in &replay_bundle.step_replay_bundles {
        for file_write in &step_bundle.side_effects.file_writes {
            if let Some(write_receipt) = file_write.write_receipt.as_ref() {
                lines.push(format!(
                    "# file={} status={:?} before_hash={} after_hash={} bytes_written={}",
                    write_receipt.relative_path.display(),
                    file_write.status,
                    write_receipt.before_hash.as_deref().unwrap_or("none"),
                    write_receipt.after_hash,
                    write_receipt.bytes_written
                ));
            } else {
                lines.push(format!(
                    "# file={} status={:?} diagnostic={}",
                    file_write.relative_path.display(),
                    file_write.status,
                    file_write.diagnostic.as_deref().unwrap_or("none")
                ));
            }
        }
    }
    if lines.len() == 1 {
        lines.push("# side-effect replay did not include file writes".to_owned());
    }
    lines.join("\n") + "\n"
}
