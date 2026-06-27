//! Materialization of typed intent-case artifact bundles.

use std::{
    error::Error,
    fmt, fs,
    path::{Component, Path, PathBuf},
};

use marlin_agent_harness_types::{
    IntentCaseArtifactId, IntentCaseArtifactKind, IntentCaseArtifactManifest,
    IntentCaseArtifactRef, IntentCaseRunId,
};
use marlin_agent_kernel::{LoopProgramExecutionReceipt, LoopProgramRuntimeHandoffExecution};
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
}

/// Receipt for a written intent-case artifact bundle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentCaseArtifactBundleMaterializationReceipt {
    pub bundle_root: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: IntentCaseArtifactManifest,
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

/// Error returned when an intent-case artifact bundle cannot be materialized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntentCaseArtifactBundleMaterializationError {
    EmptyManifest,
    MissingArtifactPath {
        artifact_id: IntentCaseArtifactId,
    },
    UnsafeArtifactPath {
        artifact_id: IntentCaseArtifactId,
        path: String,
    },
    ExecutionTraceMismatch {
        trace_entries: usize,
        execution_steps: usize,
    },
    Io {
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for IntentCaseArtifactBundleMaterializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyManifest => formatter.write_str("intent-case manifest has no artifacts"),
            Self::MissingArtifactPath { artifact_id } => {
                write!(
                    formatter,
                    "artifact {artifact_id} is present but has no path"
                )
            }
            Self::UnsafeArtifactPath { artifact_id, path } => {
                write!(formatter, "artifact {artifact_id} has unsafe path {path:?}")
            }
            Self::ExecutionTraceMismatch {
                trace_entries,
                execution_steps,
            } => write!(
                formatter,
                "trace index has {trace_entries} entries but execution receipt has {execution_steps} steps"
            ),
            Self::Io { path, message } => write!(formatter, "write {}: {message}", path.display()),
        }
    }
}

impl Error for IntentCaseArtifactBundleMaterializationError {}

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

    materialize_intent_case_artifact_bundle(
        &request.output_root,
        manifest,
        &request.vertical_trace,
        &request.execution_receipt,
    )
}

fn materialize_intent_case_artifact_bundle(
    output_root: &Path,
    manifest: IntentCaseArtifactManifest,
    vertical_trace: &GerbilLoopCaseDriverVerticalTraceReceipt,
    execution_receipt: &LoopProgramExecutionReceipt,
) -> Result<
    IntentCaseArtifactBundleMaterializationReceipt,
    IntentCaseArtifactBundleMaterializationError,
> {
    ensure_execution_trace_matches(&manifest, execution_receipt)?;
    let bundle_root = bundle_root(output_root, &manifest)?;
    create_directory(&bundle_root)?;

    let manifest_path = bundle_root.join("manifest.receipt");
    write_file(&manifest_path, render_manifest_receipt(&manifest))?;

    let mut artifacts = Vec::new();
    for artifact in manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.present)
    {
        let path = materialized_artifact_path(output_root, artifact)?;
        let content =
            render_artifact_content(artifact, &manifest, vertical_trace, execution_receipt);
        let bytes_written = write_file(&path, content)?;
        artifacts.push(IntentCaseMaterializedArtifactReceipt {
            artifact_id: artifact.artifact_id.clone(),
            kind: artifact.kind,
            path,
            bytes_written,
        });
    }

    Ok(IntentCaseArtifactBundleMaterializationReceipt {
        bundle_root,
        manifest_path,
        manifest,
        artifacts: artifacts.into_boxed_slice(),
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

fn render_manifest_receipt(manifest: &IntentCaseArtifactManifest) -> String {
    let mut lines = vec![
        "schema=marlin.intent-case.artifact-materialization.v1".to_owned(),
        format!("manifest_schema={}", manifest.schema_id),
        format!("case_id={}", manifest.case_id),
        format!("run_id={}", manifest.run_id),
        format!("policy_epoch={}", manifest.policy_epoch),
        format!("policy_digest={}", manifest.policy_digest),
        format!("loop_program_id={}", manifest.loop_program_id),
        format!("artifact_count={}", manifest.artifacts.len()),
        format!("trace_entry_count={}", manifest.trace_index.entries.len()),
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
    lines.join("\n") + "\n"
}

fn render_artifact_content(
    artifact: &IntentCaseArtifactRef,
    manifest: &IntentCaseArtifactManifest,
    vertical_trace: &GerbilLoopCaseDriverVerticalTraceReceipt,
    execution_receipt: &LoopProgramExecutionReceipt,
) -> String {
    match artifact.kind {
        IntentCaseArtifactKind::Intent => render_intent_artifact(manifest, vertical_trace),
        IntentCaseArtifactKind::PolicyPack => render_policy_pack_artifact(vertical_trace),
        IntentCaseArtifactKind::LoopProgram => render_loop_program_artifact(manifest),
        IntentCaseArtifactKind::VerticalTrace => render_vertical_trace_artifact(vertical_trace),
        IntentCaseArtifactKind::ExecutionTrace => {
            render_execution_trace_artifact(execution_receipt)
        }
        IntentCaseArtifactKind::ModelEvents => render_model_events_artifact(execution_receipt),
        IntentCaseArtifactKind::ToolCalls => render_tool_calls_artifact(execution_receipt),
        IntentCaseArtifactKind::SandboxReceipts => render_sandbox_artifact(execution_receipt),
        IntentCaseArtifactKind::MemoryReceipts => render_memory_artifact(execution_receipt),
        IntentCaseArtifactKind::DiffPatch => render_scripted_diff_artifact(manifest),
        IntentCaseArtifactKind::TestBefore => render_scripted_test_artifact(manifest, "before"),
        IntentCaseArtifactKind::TestAfter => render_scripted_test_artifact(manifest, "after"),
        IntentCaseArtifactKind::VerifierReceipt => render_verifier_artifact(execution_receipt),
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
            "step={} transition={} action={} event={}",
            entry.step_index, entry.transition_id, entry.action, entry.event
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

fn render_model_events_artifact(execution_receipt: &LoopProgramExecutionReceipt) -> String {
    render_action_projection_artifact(execution_receipt, "model", "InvokeModel")
}

fn render_tool_calls_artifact(execution_receipt: &LoopProgramExecutionReceipt) -> String {
    let mut lines = Vec::new();
    for step in &execution_receipt.steps {
        for projection in &step.runtime_handoff_execution.tool_process_projections {
            lines.push(format!(
                "step={} owner={} tool_process_command={:?}",
                step.machine_receipt.step_index.get(),
                projection.owner.as_str(),
                projection.command
            ));
        }
    }
    if lines.is_empty() {
        lines.push("tool_calls=none".to_owned());
    }
    lines.join("\n") + "\n"
}

fn render_sandbox_artifact(execution_receipt: &LoopProgramExecutionReceipt) -> String {
    let mut lines = Vec::new();
    for step in &execution_receipt.steps {
        for execution in &step.runtime_handoff_execution.executions {
            lines.push(format!(
                "step={} owner={} status={:?}",
                step.machine_receipt.step_index.get(),
                execution.owner.as_str(),
                execution.status
            ));
        }
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

fn render_scripted_diff_artifact(manifest: &IntentCaseArtifactManifest) -> String {
    format!(
        "diff --git a/scripted-intent-case b/scripted-intent-case\n# case_id={}\n# scripted run did not apply a live model patch\n",
        manifest.case_id
    )
}

fn render_scripted_test_artifact(manifest: &IntentCaseArtifactManifest, phase: &str) -> String {
    format!(
        "case_id={}\nphase={phase}\nmode=scripted\nstatus=not-run-in-scripted-bundle\n",
        manifest.case_id
    )
}

fn render_verifier_artifact(execution_receipt: &LoopProgramExecutionReceipt) -> String {
    render_action_projection_artifact(execution_receipt, "verifier", "Verify")
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
