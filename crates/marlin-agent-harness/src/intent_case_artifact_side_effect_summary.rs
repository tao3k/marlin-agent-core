//! Side-effect summary lines for intent-case sandbox artifacts.

use marlin_agent_harness_types::IntentCaseArtifactManifest;
use marlin_agent_kernel::{
    LoopProgramExecutionReplayBundleReceipt, LoopProgramFileWriteSideEffectStatus,
    LoopProgramRuntimeReplayBundleReceipt, LoopProgramToolProcessSideEffectStatus,
};

pub(crate) fn render_side_effect_sandbox_summary_lines(
    replay_bundle: &LoopProgramExecutionReplayBundleReceipt,
) -> Vec<String> {
    let mut tool_process_count = 0_usize;
    let mut completed_tool_process_count = 0_usize;
    let mut failed_tool_process_count = 0_usize;
    let mut skipped_tool_process_count = 0_usize;
    let mut file_write_count = 0_usize;
    let mut completed_file_write_count = 0_usize;
    let mut denied_file_write_count = 0_usize;
    let mut failed_file_write_count = 0_usize;

    for step_bundle in &replay_bundle.step_replay_bundles {
        for tool_process in &step_bundle.side_effects.tool_processes {
            tool_process_count += 1;
            match tool_process.status {
                LoopProgramToolProcessSideEffectStatus::Completed => {
                    completed_tool_process_count += 1;
                }
                LoopProgramToolProcessSideEffectStatus::Failed => {
                    failed_tool_process_count += 1;
                }
                LoopProgramToolProcessSideEffectStatus::Skipped => {
                    skipped_tool_process_count += 1;
                }
            }
        }
        for file_write in &step_bundle.side_effects.file_writes {
            file_write_count += 1;
            match file_write.status {
                LoopProgramFileWriteSideEffectStatus::Completed => {
                    completed_file_write_count += 1;
                }
                LoopProgramFileWriteSideEffectStatus::Denied => {
                    denied_file_write_count += 1;
                }
                LoopProgramFileWriteSideEffectStatus::Failed => {
                    failed_file_write_count += 1;
                }
            }
        }
    }

    vec![
        format!("side_effect_tool_process_count={tool_process_count}"),
        format!("side_effect_completed_tool_process_count={completed_tool_process_count}"),
        format!("side_effect_failed_tool_process_count={failed_tool_process_count}"),
        format!("side_effect_skipped_tool_process_count={skipped_tool_process_count}"),
        format!("side_effect_file_write_count={file_write_count}"),
        format!("side_effect_completed_file_write_count={completed_file_write_count}"),
        format!("side_effect_denied_file_write_count={denied_file_write_count}"),
        format!("side_effect_failed_file_write_count={failed_file_write_count}"),
        format!(
            "side_effect_sandbox_enforcement={}",
            if file_write_count > 0 {
                "file-write"
            } else {
                "no-file-write-projection"
            }
        ),
    ]
}

pub(crate) fn render_tool_process_sandbox_projection_lines(
    manifest: &IntentCaseArtifactManifest,
    step_bundle: &LoopProgramRuntimeReplayBundleReceipt,
) -> Vec<String> {
    step_bundle
        .side_effects
        .tool_processes
        .iter()
        .map(|tool_process| {
            let step_index = tool_process.projection.step_index.get();
            format!(
                "tool_process_sandbox_projection step={} tool_call_id={} resource_key={} sandbox_profile={} owner={} status={:?} sandbox_scope=profile-projection diagnostic={}",
                step_index,
                tool_call_id_for_step(manifest, step_index).unwrap_or("none"),
                resource_key_for_step(manifest, step_index).unwrap_or("none"),
                sandbox_profile_for_step(manifest, step_index).unwrap_or("none"),
                tool_process.projection.owner.as_str(),
                tool_process.status,
                tool_process.diagnostic.as_deref().unwrap_or("none")
            )
        })
        .collect()
}

fn tool_call_id_for_step(manifest: &IntentCaseArtifactManifest, step_index: u64) -> Option<&str> {
    manifest
        .trace_index
        .entries
        .iter()
        .find(|entry| entry.step_index == step_index && entry.action == "dispatch_tools")
        .and_then(|entry| entry.tool_call_id.as_ref())
        .map(|id| id.as_str())
}

fn resource_key_for_step(manifest: &IntentCaseArtifactManifest, step_index: u64) -> Option<&str> {
    manifest
        .trace_index
        .entries
        .iter()
        .find(|entry| entry.step_index == step_index && entry.action == "dispatch_tools")
        .and_then(|entry| entry.resource_key.as_ref())
        .map(|id| id.as_str())
}

fn sandbox_profile_for_step(
    manifest: &IntentCaseArtifactManifest,
    step_index: u64,
) -> Option<&str> {
    manifest
        .trace_index
        .entries
        .iter()
        .find(|entry| entry.step_index == step_index && entry.action == "dispatch_tools")
        .and_then(|entry| entry.sandbox_profile.as_ref())
        .map(|id| id.as_str())
}
