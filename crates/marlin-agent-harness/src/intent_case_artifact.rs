//! Materialization of typed intent-case artifact bundles.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{
    intent_case_artifact_error::IntentCaseArtifactBundleMaterializationError,
    intent_case_artifact_manifest::{ensure_trace_correlation_integrity, render_manifest_receipt},
    intent_case_artifact_runtime_repair::render_runtime_repair_case_receipt,
    intent_case_observed_span::IntentCaseObservedSpanSource,
};
use marlin_agent_harness_types::{
    IntentCaseArtifactCompletenessReceipt, IntentCaseArtifactId, IntentCaseArtifactKind,
    IntentCaseArtifactManifest, IntentCaseArtifactRef, IntentCaseRunId, RuntimeRepairCaseReceipt,
};
use marlin_agent_kernel::{
    LoopProgramExecutionReceipt, LoopProgramExecutionReplayBundleReceipt,
    LoopProgramRuntimeHandoffExecution, LoopProgramRuntimeReplayBundleReceipt,
};
use marlin_gerbil_scheme::{
    GerbilLoopCaseDriverVerticalTraceReceipt,
    project_gerbil_loop_case_driver_intent_case_artifact_manifest,
};

/// Request to write one Scheme-projected scripted intent-case bundle.
#[derive(Clone, Debug)]
pub struct GerbilScriptedIntentCaseArtifactBundleRequest {
    pub output_root: PathBuf,
    pub run_id: IntentCaseRunId,
    pub vertical_trace: GerbilLoopCaseDriverVerticalTraceReceipt,
    pub execution_receipt: LoopProgramExecutionReceipt,
    pub side_effect_replay_bundle: Option<LoopProgramExecutionReplayBundleReceipt>,
    pub runtime_repair_receipt: Option<RuntimeRepairCaseReceipt>,
    pub observed_span_source: Option<IntentCaseObservedSpanSource>,
}

/// Receipt for a written intent-case artifact bundle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentCaseArtifactBundleMaterializationReceipt {
    pub bundle_root: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: IntentCaseArtifactManifest,
    pub completeness_receipt: IntentCaseArtifactCompletenessReceipt,
    pub artifacts: Box<[IntentCaseMaterializedArtifactReceipt]>,
}

impl IntentCaseArtifactBundleMaterializationReceipt {
    #[must_use]
    pub fn has_artifact_kind(&self, kind: IntentCaseArtifactKind) -> bool {
        self.artifacts.iter().any(|artifact| artifact.kind == kind)
    }
}

/// Receipt for one written artifact file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentCaseMaterializedArtifactReceipt {
    pub artifact_id: IntentCaseArtifactId,
    pub kind: IntentCaseArtifactKind,
    pub path: PathBuf,
    pub bytes_written: u64,
}

/// Materialize one scripted Gerbil loop case into the Intent Case Lab artifact layout.
pub fn materialize_gerbil_scripted_intent_case_artifact_bundle(
    request: GerbilScriptedIntentCaseArtifactBundleRequest,
) -> Result<
    IntentCaseArtifactBundleMaterializationReceipt,
    IntentCaseArtifactBundleMaterializationError,
> {
    let manifest = project_gerbil_loop_case_driver_intent_case_artifact_manifest(
        &request.vertical_trace,
        request.run_id,
    );
    let manifest = IntentCaseObservedSpanSource::enrich_manifest(
        manifest,
        request.observed_span_source.as_ref(),
    );

    materialize_intent_case_artifact_bundle(
        &request.output_root,
        manifest,
        &request.vertical_trace,
        &request.execution_receipt,
        request.side_effect_replay_bundle.as_ref(),
        request.runtime_repair_receipt.as_ref(),
    )
}

fn materialize_intent_case_artifact_bundle(
    output_root: &Path,
    manifest: IntentCaseArtifactManifest,
    vertical_trace: &GerbilLoopCaseDriverVerticalTraceReceipt,
    execution_receipt: &LoopProgramExecutionReceipt,
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
    runtime_repair_receipt: Option<&RuntimeRepairCaseReceipt>,
) -> Result<
    IntentCaseArtifactBundleMaterializationReceipt,
    IntentCaseArtifactBundleMaterializationError,
> {
    let manifest = enrich_manifest_with_side_effect_artifacts(manifest, side_effect_replay_bundle);
    let manifest = IntentCaseObservedSpanSource::enrich_manifest_with_side_effect_span_expectations(
        manifest,
        side_effect_replay_bundle,
    );
    let manifest =
        IntentCaseObservedSpanSource::enrich_manifest_with_runtime_repair_span_expectations(
            manifest,
            runtime_repair_receipt,
        );
    ensure_execution_trace_matches(&manifest, execution_receipt)?;
    ensure_trace_correlation_integrity(&manifest)?;
    let bundle_root = bundle_root(output_root, &manifest)?;
    create_directory(&bundle_root)?;

    let manifest_path = bundle_root.join("manifest.receipt");
    let mut artifacts = Vec::new();
    for artifact in manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.present)
    {
        let path = materialized_artifact_path(output_root, artifact)?;
        let content = render_artifact_content(
            artifact,
            &manifest,
            vertical_trace,
            execution_receipt,
            side_effect_replay_bundle,
            runtime_repair_receipt,
        );
        let bytes_written = write_file(&path, content)?;
        artifacts.push(IntentCaseMaterializedArtifactReceipt {
            artifact_id: artifact.artifact_id.clone(),
            kind: artifact.kind,
            path,
            bytes_written,
        });
    }

    let completeness_receipt =
        IntentCaseArtifactCompletenessReceipt::from_manifest_and_materialized_artifacts(
            &manifest,
            artifacts.iter().map(|artifact| artifact.kind),
        );
    if !completeness_receipt.is_complete() {
        return Err(
            IntentCaseArtifactBundleMaterializationError::IncompleteArtifactBundle {
                missing_artifacts: completeness_receipt.missing_artifacts.clone(),
            },
        );
    }

    write_file(
        &manifest_path,
        render_manifest_receipt(&manifest, &completeness_receipt),
    )?;

    Ok(IntentCaseArtifactBundleMaterializationReceipt {
        bundle_root,
        manifest_path,
        manifest,
        completeness_receipt,
        artifacts: artifacts.into_boxed_slice(),
    })
}

fn enrich_manifest_with_side_effect_artifacts(
    mut manifest: IntentCaseArtifactManifest,
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
) -> IntentCaseArtifactManifest {
    let Some(replay_bundle) = side_effect_replay_bundle else {
        return manifest;
    };
    let Some(artifact_prefix) = intent_case_artifact_prefix_from_manifest(&manifest) else {
        return manifest;
    };

    let has_tool_processes = replay_bundle
        .step_replay_bundles
        .iter()
        .any(|bundle| !bundle.side_effects.tool_processes.is_empty());
    if has_tool_processes {
        append_runtime_artifact_if_missing(
            &mut manifest,
            IntentCaseArtifactKind::ToolCalls,
            "tool-calls",
            &artifact_prefix,
            "51-tool-calls.receipt",
        );
    }

    let has_file_writes = replay_bundle
        .step_replay_bundles
        .iter()
        .any(|bundle| !bundle.side_effects.file_writes.is_empty());
    if has_file_writes {
        append_runtime_artifact_if_missing(
            &mut manifest,
            IntentCaseArtifactKind::SandboxReceipts,
            "sandbox-receipts",
            &artifact_prefix,
            "53-sandbox-receipts.receipt",
        );
        append_runtime_artifact_if_missing(
            &mut manifest,
            IntentCaseArtifactKind::DiffPatch,
            "diff-patch",
            &artifact_prefix,
            "60-diff.patch",
        );
    }

    manifest
}

fn append_runtime_artifact_if_missing(
    manifest: &mut IntentCaseArtifactManifest,
    kind: IntentCaseArtifactKind,
    lane: &str,
    artifact_prefix: &str,
    filename: &str,
) {
    if manifest.has_artifact_kind(kind) {
        return;
    }
    manifest.artifacts.push(IntentCaseArtifactRef::present(
        IntentCaseArtifactId::new(format!("{}:{lane}", manifest.case_id.as_str())),
        kind,
        format!("{artifact_prefix}/{filename}"),
    ));
}

fn intent_case_artifact_prefix_from_manifest(
    manifest: &IntentCaseArtifactManifest,
) -> Option<String> {
    manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.present)
        .filter_map(|artifact| artifact.path.as_deref())
        .find_map(|path| {
            Path::new(path)
                .parent()
                .map(|parent| parent.to_string_lossy().replace('\\', "/"))
        })
}

fn ensure_execution_trace_matches(
    manifest: &IntentCaseArtifactManifest,
    execution_receipt: &LoopProgramExecutionReceipt,
) -> Result<(), IntentCaseArtifactBundleMaterializationError> {
    let trace_entries = manifest.trace_index.entries.len();
    let execution_steps = execution_receipt.steps.len();
    if trace_entries == execution_steps {
        Ok(())
    } else {
        Err(
            IntentCaseArtifactBundleMaterializationError::ExecutionTraceMismatch {
                trace_entries,
                execution_steps,
            },
        )
    }
}

fn bundle_root(
    output_root: &Path,
    manifest: &IntentCaseArtifactManifest,
) -> Result<PathBuf, IntentCaseArtifactBundleMaterializationError> {
    let first_artifact = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.present)
        .ok_or(IntentCaseArtifactBundleMaterializationError::EmptyManifest)?;
    let artifact_path = materialized_artifact_path(output_root, first_artifact)?;
    Ok(artifact_path
        .parent()
        .expect("artifact path should have parent")
        .to_owned())
}

fn materialized_artifact_path(
    output_root: &Path,
    artifact: &IntentCaseArtifactRef,
) -> Result<PathBuf, IntentCaseArtifactBundleMaterializationError> {
    let relative_path = artifact.path.as_ref().ok_or_else(|| {
        IntentCaseArtifactBundleMaterializationError::MissingArtifactPath {
            artifact_id: artifact.artifact_id.clone(),
        }
    })?;
    safe_relative_path(relative_path)
        .map(|path| output_root.join(path))
        .map_err(
            |()| IntentCaseArtifactBundleMaterializationError::UnsafeArtifactPath {
                artifact_id: artifact.artifact_id.clone(),
                path: relative_path.clone(),
            },
        )
}

fn safe_relative_path(path: &str) -> Result<PathBuf, ()> {
    let path = Path::new(path);
    if path.is_absolute() {
        return Err(());
    }

    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => relative.push(segment),
            Component::CurDir => {}
            _ => return Err(()),
        }
    }
    Ok(relative)
}

fn create_directory(path: &Path) -> Result<(), IntentCaseArtifactBundleMaterializationError> {
    fs::create_dir_all(path).map_err(|error| IntentCaseArtifactBundleMaterializationError::Io {
        path: path.to_owned(),
        message: error.to_string(),
    })
}

fn write_file(
    path: &Path,
    content: String,
) -> Result<u64, IntentCaseArtifactBundleMaterializationError> {
    if let Some(parent) = path.parent() {
        create_directory(parent)?;
    }
    fs::write(path, content.as_bytes()).map_err(|error| {
        IntentCaseArtifactBundleMaterializationError::Io {
            path: path.to_owned(),
            message: error.to_string(),
        }
    })?;
    Ok(content.len() as u64)
}

fn render_artifact_content(
    artifact: &IntentCaseArtifactRef,
    manifest: &IntentCaseArtifactManifest,
    vertical_trace: &GerbilLoopCaseDriverVerticalTraceReceipt,
    execution_receipt: &LoopProgramExecutionReceipt,
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
    runtime_repair_receipt: Option<&RuntimeRepairCaseReceipt>,
) -> String {
    match artifact.kind {
        IntentCaseArtifactKind::Intent => render_intent_artifact(manifest, vertical_trace),
        IntentCaseArtifactKind::PolicyPack => render_policy_pack_artifact(vertical_trace),
        IntentCaseArtifactKind::LoopProgram => render_loop_program_artifact(manifest),
        IntentCaseArtifactKind::VerticalTrace => render_vertical_trace_artifact(vertical_trace),
        IntentCaseArtifactKind::ExecutionTrace => {
            render_execution_trace_artifact(execution_receipt)
        }
        IntentCaseArtifactKind::ModelEvents => {
            render_model_events_artifact(manifest, execution_receipt)
        }
        IntentCaseArtifactKind::ToolCalls => {
            render_tool_calls_artifact(manifest, execution_receipt, side_effect_replay_bundle)
        }
        IntentCaseArtifactKind::SandboxReceipts => {
            render_sandbox_artifact(manifest, execution_receipt, side_effect_replay_bundle)
        }
        IntentCaseArtifactKind::MemoryReceipts => render_memory_artifact(execution_receipt),
        IntentCaseArtifactKind::DiffPatch => {
            render_diff_artifact(manifest, side_effect_replay_bundle)
        }
        IntentCaseArtifactKind::TestBefore => {
            render_test_artifact(manifest, side_effect_replay_bundle, "before")
        }
        IntentCaseArtifactKind::TestAfter => {
            render_test_artifact(manifest, side_effect_replay_bundle, "after")
        }
        IntentCaseArtifactKind::VerifierReceipt => {
            render_verifier_artifact(execution_receipt, runtime_repair_receipt)
        }
        IntentCaseArtifactKind::PolicyExplanation => {
            render_policy_explanation_artifact(manifest, vertical_trace)
        }
        IntentCaseArtifactKind::ReplayScript => render_replay_script_artifact(manifest),
    }
}

fn render_intent_artifact(
    manifest: &IntentCaseArtifactManifest,
    vertical_trace: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> String {
    format!(
        "* Intent Case {}\n:PROPERTIES:\n:RUN_ID: {}\n:LOOP_PROGRAM_ID: {}\n:POLICY_DIGEST: {}\n:PROFILE_REF: {}\n:END:\n\nScripted intent case projected from Gerbil config-interface vertical trace.\n",
        manifest.case_id,
        manifest.run_id,
        manifest.loop_program_id,
        manifest.policy_digest,
        vertical_trace.profile_ref()
    )
}

fn render_policy_pack_artifact(
    vertical_trace: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> String {
    let mechanism_policies = vertical_trace
        .mechanism_policy_ids()
        .collect::<Vec<_>>()
        .join(",");
    let capabilities = vertical_trace
        .capability_tags()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "policy_epoch={}\nmechanism_policy_count={}\nmechanism_policy_ids={}\ncapability_mask={}\ncapability_tags={}\nlive_llm_required={}\nlive_llm_allowed={}\n",
        vertical_trace.policy_epoch(),
        vertical_trace.mechanism_policy_count(),
        mechanism_policies,
        vertical_trace.capability_mask(),
        capabilities,
        vertical_trace.live_llm_required(),
        vertical_trace.live_llm_allowed()
    )
}

fn render_loop_program_artifact(manifest: &IntentCaseArtifactManifest) -> String {
    let mut lines = vec![format!("loop_program_id={}", manifest.loop_program_id)];
    lines.extend(manifest.trace_index.entries.iter().map(|entry| {
        format!(
            "step={} transition={} action={} event={} model_invocation_id={} tool_call_id={} resource_key={} sandbox_profile={}",
            entry.step_index,
            entry.transition_id,
            entry.action,
            entry.event,
            entry
                .model_invocation_id
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or("none"),
            entry
                .tool_call_id
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or("none"),
            entry
                .resource_key
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or("none"),
            entry
                .sandbox_profile
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or("none")
        )
    }));
    lines.join("\n") + "\n"
}

fn render_vertical_trace_artifact(
    vertical_trace: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> String {
    let mut lines = vec![
        format!("case_id={}", vertical_trace.case_id()),
        format!("profile_ref={}", vertical_trace.profile_ref()),
        format!("loop_program_id={}", vertical_trace.loop_program_id()),
        format!("policy_epoch={}", vertical_trace.policy_epoch()),
        format!("transition_count={}", vertical_trace.transition_count()),
    ];
    lines.extend(
        vertical_trace
            .transition_actions()
            .zip(vertical_trace.transition_events())
            .enumerate()
            .map(|(index, (action, event))| {
                format!("transition.{} action={} event={}", index + 1, action, event)
            }),
    );
    lines.join("\n") + "\n"
}

fn render_execution_trace_artifact(execution_receipt: &LoopProgramExecutionReceipt) -> String {
    let mut lines = vec![
        format!("program_id={:?}", execution_receipt.program_id),
        format!("status={:?}", execution_receipt.status),
        format!("step_count={}", execution_receipt.steps.len()),
    ];
    for step in &execution_receipt.steps {
        lines.push(format!(
            "step={} transition={:?} action={:?} event={:?} generated_event={:?} runtime_status={:?}",
            step.machine_receipt.step_index.get(),
            step.machine_receipt.transition_id,
            step.machine_receipt.action,
            step.machine_receipt.event,
            step.generated_event,
            step.runtime_handoff_execution.status
        ));
        lines.extend(
            step.runtime_handoff_execution
                .executions
                .iter()
                .map(render_runtime_execution),
        );
    }
    lines.join("\n") + "\n"
}

fn render_runtime_execution(execution: &LoopProgramRuntimeHandoffExecution) -> String {
    format!(
        "  runtime owner={} kind={:?} status={:?} next_event={:?}",
        execution.owner.as_str(),
        execution.kind,
        execution.status,
        execution.next_event
    )
}

fn render_model_events_artifact(
    manifest: &IntentCaseArtifactManifest,
    execution_receipt: &LoopProgramExecutionReceipt,
) -> String {
    let mut lines = Vec::new();
    for step in &execution_receipt.steps {
        if format!("{:?}", step.machine_receipt.action) == "InvokeModel" {
            let step_index = step.machine_receipt.step_index.get();
            lines.push(format!(
                "model step={} model_invocation_id={} status={:?}",
                step_index,
                model_invocation_id_for_step(manifest, step_index).unwrap_or("none"),
                step.runtime_handoff_execution.status
            ));
        }
    }
    if lines.is_empty() {
        lines.push("model=none".to_owned());
    }
    lines.join("\n") + "\n"
}

fn render_tool_calls_artifact(
    manifest: &IntentCaseArtifactManifest,
    execution_receipt: &LoopProgramExecutionReceipt,
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
) -> String {
    let mut lines = Vec::new();
    for step in &execution_receipt.steps {
        for projection in &step.runtime_handoff_execution.tool_process_projections {
            let step_index = step.machine_receipt.step_index.get();
            lines.push(format!(
                "step={} tool_call_id={} resource_key={} sandbox_profile={} owner={} tool_process_command={:?}",
                step_index,
                tool_call_id_for_step(manifest, step_index).unwrap_or("none"),
                resource_key_for_step(manifest, step_index).unwrap_or("none"),
                sandbox_profile_for_step(manifest, step_index).unwrap_or("none"),
                projection.owner.as_str(),
                projection.command
            ));
        }
    }
    if let Some(replay_bundle) = side_effect_replay_bundle {
        lines.push(format!(
            "side_effect_replay policy_status={:?} execution_status={:?} step_bundle_count={}",
            replay_bundle.policy_status,
            replay_bundle.execution_status,
            replay_bundle.step_replay_bundles.len()
        ));
        for step_bundle in &replay_bundle.step_replay_bundles {
            lines.extend(render_tool_process_side_effects(manifest, step_bundle));
        }
    }
    if lines.is_empty() {
        lines.push("tool_calls=none".to_owned());
    }
    lines.join("\n") + "\n"
}

fn render_sandbox_artifact(
    manifest: &IntentCaseArtifactManifest,
    execution_receipt: &LoopProgramExecutionReceipt,
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
) -> String {
    let mut lines = Vec::new();
    for step in &execution_receipt.steps {
        for execution in &step.runtime_handoff_execution.executions {
            let step_index = step.machine_receipt.step_index.get();
            lines.push(format!(
                "step={} resource_key={} sandbox_profile={} owner={} status={:?}",
                step_index,
                resource_key_for_step(manifest, step_index).unwrap_or("none"),
                sandbox_profile_for_step(manifest, step_index).unwrap_or("none"),
                execution.owner.as_str(),
                execution.status
            ));
        }
    }
    if let Some(replay_bundle) = side_effect_replay_bundle {
        lines.push(format!(
            "side_effect_policy_status={:?}",
            replay_bundle.policy_status
        ));
        for step_bundle in &replay_bundle.step_replay_bundles {
            lines.extend(render_file_write_side_effects(manifest, step_bundle));
        }
    }
    if lines.is_empty() {
        lines.push("sandbox_receipts=none".to_owned());
    }
    lines.join("\n") + "\n"
}

fn render_memory_artifact(execution_receipt: &LoopProgramExecutionReceipt) -> String {
    let mut lines = Vec::new();
    for step in &execution_receipt.steps {
        for projection in &step.runtime_handoff_execution.memory_projections {
            lines.push(format!(
                "step={} owner={} memory_intent={:?}",
                step.machine_receipt.step_index.get(),
                projection.owner.as_str(),
                projection.intent
            ));
        }
    }
    if lines.is_empty() {
        lines.push("memory_receipts=none".to_owned());
    }
    lines.join("\n") + "\n"
}

fn render_diff_artifact(
    manifest: &IntentCaseArtifactManifest,
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
) -> String {
    let Some(replay_bundle) = side_effect_replay_bundle else {
        return format!(
            "diff --git a/scripted-intent-case b/scripted-intent-case\n# case_id={}\n# scripted run did not apply a live model patch\n",
            manifest.case_id
        );
    };
    let mut lines = vec![format!(
        "diff --git a/intent-case/{} b/intent-case/{}",
        manifest.case_id, manifest.case_id
    )];
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

fn render_test_artifact(
    manifest: &IntentCaseArtifactManifest,
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
    phase: &str,
) -> String {
    let Some(replay_bundle) = side_effect_replay_bundle else {
        return format!(
            "case_id={}\nphase={phase}\nmode=scripted\nstatus=not-run-in-scripted-bundle\n",
            manifest.case_id
        );
    };

    let tool_process_count = replay_bundle
        .step_replay_bundles
        .iter()
        .map(|bundle| bundle.side_effects.tool_processes.len())
        .sum::<usize>();
    format!(
        "case_id={}\nphase={phase}\nmode=side-effect-replay\npolicy_status={:?}\ntool_process_count={tool_process_count}\n",
        manifest.case_id, replay_bundle.policy_status
    )
}

fn render_tool_process_side_effects(
    manifest: &IntentCaseArtifactManifest,
    step_bundle: &LoopProgramRuntimeReplayBundleReceipt,
) -> Vec<String> {
    step_bundle
        .side_effects
        .tool_processes
        .iter()
        .map(|tool_process| {
            let spawn = tool_process.spawn_receipt.as_ref();
            let step_index = tool_process.projection.step_index.get();
            format!(
                "side_effect step={} tool_call_id={} resource_key={} sandbox_profile={} owner={} status={:?} pid={} exit_status={:?} stdout_digest={} stderr_digest={} stdout_bytes={} stderr_bytes={} diagnostic={}",
                step_index,
                tool_call_id_for_step(manifest, step_index).unwrap_or("none"),
                resource_key_for_step(manifest, step_index).unwrap_or("none"),
                sandbox_profile_for_step(manifest, step_index).unwrap_or("none"),
                tool_process.projection.owner.as_str(),
                tool_process.status,
                spawn.map(|receipt| receipt.pid).unwrap_or_default(),
                spawn.map(|receipt| receipt.output.status.code()),
                spawn
                    .map(|receipt| stable_bytes_digest(&receipt.output.stdout))
                    .unwrap_or_else(|| "none".to_owned()),
                spawn
                    .map(|receipt| stable_bytes_digest(&receipt.output.stderr))
                    .unwrap_or_else(|| "none".to_owned()),
                spawn.map(|receipt| receipt.output.stdout.len()).unwrap_or_default(),
                spawn.map(|receipt| receipt.output.stderr.len()).unwrap_or_default(),
                tool_process.diagnostic.as_deref().unwrap_or("none")
            )
        })
        .collect()
}

fn model_invocation_id_for_step(
    manifest: &IntentCaseArtifactManifest,
    step_index: u64,
) -> Option<&str> {
    manifest
        .trace_index
        .entries
        .iter()
        .find(|entry| entry.step_index == step_index && entry.action == "invoke_model")
        .and_then(|entry| entry.model_invocation_id.as_ref())
        .map(|id| id.as_str())
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

fn render_file_write_side_effects(
    manifest: &IntentCaseArtifactManifest,
    step_bundle: &LoopProgramRuntimeReplayBundleReceipt,
) -> Vec<String> {
    step_bundle
        .side_effects
        .file_writes
        .iter()
        .map(|file_write| {
            let write = file_write.write_receipt.as_ref();
            let step_index = file_write.projection.step_index.get();
            format!(
                "file_write step={} resource_key={} sandbox_profile={} relative_path={} status={:?} before_hash={} after_hash={} bytes_written={} diagnostic={}",
                step_index,
                resource_key_for_step(manifest, step_index).unwrap_or("none"),
                sandbox_profile_for_step(manifest, step_index).unwrap_or("none"),
                file_write.relative_path.display(),
                file_write.status,
                write
                    .and_then(|receipt| receipt.before_hash.as_deref())
                    .unwrap_or("none"),
                write
                    .map(|receipt| receipt.after_hash.as_str())
                    .unwrap_or("none"),
                write.map(|receipt| receipt.bytes_written).unwrap_or_default(),
                file_write.diagnostic.as_deref().unwrap_or("none")
            )
        })
        .collect()
}

fn render_verifier_artifact(
    execution_receipt: &LoopProgramExecutionReceipt,
    runtime_repair_receipt: Option<&RuntimeRepairCaseReceipt>,
) -> String {
    let mut content = render_action_projection_artifact(execution_receipt, "verifier", "Verify");
    if let Some(receipt) = runtime_repair_receipt {
        content.push_str(&render_runtime_repair_case_receipt(receipt));
    }
    content
}

fn render_action_projection_artifact(
    execution_receipt: &LoopProgramExecutionReceipt,
    label: &str,
    action_name: &str,
) -> String {
    let mut lines = Vec::new();
    for step in &execution_receipt.steps {
        if format!("{:?}", step.machine_receipt.action) == action_name {
            lines.push(format!(
                "{} step={} status={:?}",
                label,
                step.machine_receipt.step_index.get(),
                step.runtime_handoff_execution.status
            ));
        }
    }
    if lines.is_empty() {
        lines.push(format!("{label}=none"));
    }
    lines.join("\n") + "\n"
}

fn render_policy_explanation_artifact(
    manifest: &IntentCaseArtifactManifest,
    vertical_trace: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> String {
    let policies = vertical_trace
        .mechanism_policy_ids()
        .map(|policy| format!("- {policy}"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "* Policy Explanation {}\n:PROPERTIES:\n:POLICY_EPOCH: {}\n:POLICY_DIGEST: {}\n:END:\n\nMechanism policies:\n{}\n",
        manifest.case_id, manifest.policy_epoch, manifest.policy_digest, policies
    )
}

fn render_replay_script_artifact(manifest: &IntentCaseArtifactManifest) -> String {
    format!(
        "#!/usr/bin/env gxi\n;; replay-intent-case\n;; case_id={}\n;; run_id={}\n;; loop_program_id={}\n",
        manifest.case_id, manifest.run_id, manifest.loop_program_id
    )
}

fn stable_bytes_digest(bytes: &[u8]) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;

    let mut value = FNV_OFFSET;
    for byte in bytes {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(FNV_PRIME);
    }
    format!("fnv1a64:{value:016x}")
}
