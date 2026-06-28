//! Intent-case artifact bundle projection for Scheme-owned loop cases.

use std::fmt::Write as _;

use marlin_agent_harness_types::{
    IntentCaseArtifactId, IntentCaseArtifactKind, IntentCaseArtifactManifest,
    IntentCaseArtifactManifestRequest, IntentCaseArtifactRef, IntentCaseId,
    IntentCaseLoopProgramId, IntentCasePolicyDigest, IntentCaseRunId, IntentCaseRunReceipt,
    IntentCaseRuntimeOwner, IntentCaseTraceEntry, IntentCaseTraceEntryId,
    IntentCaseTraceEntryRequest, IntentCaseTraceIndex, IntentCaseTransitionId,
};

use super::GerbilLoopCaseDriverVerticalTraceReceipt;

/// Runtime owner recorded for Rust-driven execution of Scheme-projected cases.
pub const GERBIL_LOOP_CASE_DRIVER_INTENT_CASE_RUNTIME_OWNER: &str = "marlin-agent-core";

const INTENT_CASE_ARTIFACT_ROOT: &str = "artifacts/intent-cases";

/// Project a verified Scheme vertical trace into the typed artifact manifest
/// expected by intent-case lab runs.
#[must_use]
pub fn project_gerbil_loop_case_driver_intent_case_artifact_manifest(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
    run_id: impl Into<IntentCaseRunId>,
) -> IntentCaseArtifactManifest {
    let context = intent_case_manifest_context(receipt, run_id.into());
    let artifact_ids = intent_case_artifact_ids(&context.case_id);

    build_intent_case_artifact_manifest(receipt, context, artifact_ids)
}

/// Project a verified Scheme vertical trace into an intent-case run receipt.
#[must_use]
pub fn project_gerbil_loop_case_driver_intent_case_run_receipt(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
    run_id: impl Into<IntentCaseRunId>,
) -> IntentCaseRunReceipt {
    let manifest = project_gerbil_loop_case_driver_intent_case_artifact_manifest(receipt, run_id);
    let diagnostics = intent_case_manifest_diagnostics(receipt, &manifest);

    if diagnostics.is_empty() {
        IntentCaseRunReceipt::passed(manifest)
    } else {
        IntentCaseRunReceipt::incomplete(manifest, diagnostics)
    }
}

fn intent_case_manifest_diagnostics(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
    manifest: &IntentCaseArtifactManifest,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if !manifest.has_core_artifact_bundle() {
        diagnostics.push("intent-case manifest is missing a core artifact lane".to_owned());
    }
    if manifest.trace_index.entries.len() != receipt.transition_count() {
        diagnostics.push(format!(
            "intent-case trace index has {} entries but vertical trace declares {} transitions",
            manifest.trace_index.entries.len(),
            receipt.transition_count()
        ));
    }
    diagnostics
}

#[derive(Clone, Debug)]
struct IntentCaseManifestContext {
    case_id: String,
    run_id: IntentCaseRunId,
    policy_epoch: u64,
    policy_digest: String,
    loop_program_id: String,
    artifact_prefix: String,
}

#[derive(Clone, Debug)]
struct IntentCaseArtifactIds {
    intent: IntentCaseArtifactId,
    policy_pack: IntentCaseArtifactId,
    loop_program: IntentCaseArtifactId,
    vertical_trace: IntentCaseArtifactId,
    execution_trace: IntentCaseArtifactId,
    model_events: IntentCaseArtifactId,
    tool_calls: IntentCaseArtifactId,
    sandbox_receipts: IntentCaseArtifactId,
    memory_receipts: IntentCaseArtifactId,
    diff_patch: IntentCaseArtifactId,
    test_before: IntentCaseArtifactId,
    test_after: IntentCaseArtifactId,
    verifier_receipt: IntentCaseArtifactId,
    replay_script: IntentCaseArtifactId,
    policy_explanation: IntentCaseArtifactId,
}

fn intent_case_manifest_context(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
    run_id: IntentCaseRunId,
) -> IntentCaseManifestContext {
    let case_id = receipt.case_id().as_str().to_owned();
    let loop_program_id = receipt.loop_program_id().as_str().to_owned();
    let artifact_prefix = intent_case_artifact_prefix(&case_id, run_id.as_str());

    IntentCaseManifestContext {
        case_id,
        run_id,
        policy_epoch: receipt.policy_epoch(),
        policy_digest: policy_digest_hex(receipt.policy_digest_octets()),
        loop_program_id,
        artifact_prefix,
    }
}

fn intent_case_artifact_ids(case_id: &str) -> IntentCaseArtifactIds {
    IntentCaseArtifactIds {
        intent: artifact_id(case_id, "intent"),
        policy_pack: artifact_id(case_id, "policy-pack"),
        loop_program: artifact_id(case_id, "loop-program"),
        vertical_trace: artifact_id(case_id, "vertical-trace"),
        execution_trace: artifact_id(case_id, "execution-trace"),
        model_events: artifact_id(case_id, "model-events"),
        tool_calls: artifact_id(case_id, "tool-calls"),
        sandbox_receipts: artifact_id(case_id, "sandbox-receipts"),
        memory_receipts: artifact_id(case_id, "memory-receipts"),
        diff_patch: artifact_id(case_id, "diff-patch"),
        test_before: artifact_id(case_id, "test-before"),
        test_after: artifact_id(case_id, "test-after"),
        verifier_receipt: artifact_id(case_id, "verifier-receipt"),
        replay_script: artifact_id(case_id, "replay-script"),
        policy_explanation: artifact_id(case_id, "policy-explanation"),
    }
}

fn build_intent_case_artifact_manifest(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
    context: IntentCaseManifestContext,
    artifact_ids: IntentCaseArtifactIds,
) -> IntentCaseArtifactManifest {
    let mut manifest =
        IntentCaseArtifactManifest::from_request(intent_case_manifest_request(&context))
            .with_trace_index(intent_case_trace_index(receipt, &context, &artifact_ids));

    for artifact in intent_case_artifact_refs(receipt, &context, artifact_ids) {
        manifest = manifest.with_artifact(artifact);
    }
    manifest
}

fn intent_case_manifest_request(
    context: &IntentCaseManifestContext,
) -> IntentCaseArtifactManifestRequest {
    IntentCaseArtifactManifestRequest {
        case_id: IntentCaseId::new(context.case_id.clone()),
        run_id: context.run_id.clone(),
        policy_epoch: context.policy_epoch,
        policy_digest: IntentCasePolicyDigest::new(context.policy_digest.clone()),
        loop_program_id: IntentCaseLoopProgramId::new(context.loop_program_id.clone()),
    }
}

fn intent_case_trace_index(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
    context: &IntentCaseManifestContext,
    artifact_ids: &IntentCaseArtifactIds,
) -> IntentCaseTraceIndex {
    IntentCaseTraceIndex::new(
        receipt
            .transition_actions()
            .zip(receipt.transition_events())
            .enumerate()
            .map(|(index, (action, event))| {
                intent_case_trace_entry(index, action, event, receipt, context, artifact_ids)
            })
            .collect::<Vec<_>>(),
    )
}

fn intent_case_trace_entry(
    index: usize,
    action: &str,
    event: &str,
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
    context: &IntentCaseManifestContext,
    artifact_ids: &IntentCaseArtifactIds,
) -> IntentCaseTraceEntry {
    let step_index = (index + 1) as u64;

    let mut entry = IntentCaseTraceEntry::from_request(IntentCaseTraceEntryRequest {
        trace_id: trace_entry_id(&context.case_id, step_index),
        step_index,
        transition_id: transition_id(&context.loop_program_id, step_index),
        action: action.to_owned(),
        event: event.to_owned(),
    })
    .with_runtime_owner(IntentCaseRuntimeOwner::new(
        GERBIL_LOOP_CASE_DRIVER_INTENT_CASE_RUNTIME_OWNER,
    ))
    .with_artifact_ref(artifact_ids.vertical_trace.clone())
    .with_artifact_ref(artifact_ids.execution_trace.clone());

    if action == "invoke_model" {
        entry = entry.with_model_invocation_id(model_invocation_id(
            &context.case_id,
            &context.loop_program_id,
            step_index,
        ));
    }
    if action == "dispatch_tools" {
        entry = entry
            .with_tool_call_id(tool_call_id(
                &context.case_id,
                &context.loop_program_id,
                step_index,
            ))
            .with_resource_key(tool_resource_key(receipt))
            .with_sandbox_profile(tool_sandbox_profile(receipt));
    }

    for artifact_id in trace_entry_action_artifact_refs(action, receipt, artifact_ids) {
        entry = entry.with_artifact_ref(artifact_id);
    }
    entry
}

fn trace_entry_action_artifact_refs(
    action: &str,
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
    artifact_ids: &IntentCaseArtifactIds,
) -> Vec<IntentCaseArtifactId> {
    let mut artifact_refs = Vec::new();

    match action {
        "invoke_model" => {
            artifact_refs.push(artifact_ids.model_events.clone());
        }
        "dispatch_tools" => {
            if receipt.tool_intent_count() > 0 {
                artifact_refs.push(artifact_ids.tool_calls.clone());
            }
            if has_capability(receipt, "+sandbox") || has_capability(receipt, "+denylist") {
                artifact_refs.push(artifact_ids.sandbox_receipts.clone());
            }
            if receipt.live_llm_required() || has_capability(receipt, "+repair") {
                artifact_refs.push(artifact_ids.diff_patch.clone());
                artifact_refs.push(artifact_ids.test_before.clone());
            }
        }
        "rewrite_graph" if receipt.live_llm_required() || has_capability(receipt, "+repair") => {
            artifact_refs.push(artifact_ids.diff_patch.clone());
        }
        "verify" => {
            artifact_refs.push(artifact_ids.verifier_receipt.clone());
            if receipt.live_llm_required() || has_capability(receipt, "+repair") {
                artifact_refs.push(artifact_ids.test_after.clone());
            }
        }
        action if action.contains("memory") && receipt.memory_intent_count() > 0 => {
            artifact_refs.push(artifact_ids.memory_receipts.clone());
        }
        _ => {}
    }

    artifact_refs
}

fn intent_case_artifact_refs(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
    context: &IntentCaseManifestContext,
    artifact_ids: IntentCaseArtifactIds,
) -> Vec<IntentCaseArtifactRef> {
    let artifact_prefix = &context.artifact_prefix;
    let policy_digest = &context.policy_digest;

    let mut artifacts = vec![
        IntentCaseArtifactRef::present(
            artifact_ids.intent.clone(),
            IntentCaseArtifactKind::Intent,
            format!("{artifact_prefix}/00-intent.org"),
        ),
        IntentCaseArtifactRef::present(
            artifact_ids.policy_pack.clone(),
            IntentCaseArtifactKind::PolicyPack,
            format!("{artifact_prefix}/10-policy-pack.receipt"),
        )
        .with_content_digest(policy_digest.clone()),
        IntentCaseArtifactRef::present(
            artifact_ids.loop_program.clone(),
            IntentCaseArtifactKind::LoopProgram,
            format!("{artifact_prefix}/20-loop-program.receipt"),
        )
        .with_content_digest(policy_digest.clone()),
        IntentCaseArtifactRef::present(
            artifact_ids.vertical_trace.clone(),
            IntentCaseArtifactKind::VerticalTrace,
            format!("{artifact_prefix}/30-vertical-trace.receipt"),
        ),
        IntentCaseArtifactRef::present(
            artifact_ids.execution_trace.clone(),
            IntentCaseArtifactKind::ExecutionTrace,
            format!("{artifact_prefix}/40-execution-trace.receipt"),
        ),
        IntentCaseArtifactRef::present(
            artifact_ids.policy_explanation.clone(),
            IntentCaseArtifactKind::PolicyExplanation,
            format!("{artifact_prefix}/80-policy-explanation.org"),
        ),
        IntentCaseArtifactRef::present(
            artifact_ids.replay_script.clone(),
            IntentCaseArtifactKind::ReplayScript,
            format!("{artifact_prefix}/90-replay-script.ss"),
        ),
    ];
    artifacts.extend(optional_intent_case_artifact_refs(
        receipt,
        context,
        &artifact_ids,
    ));
    artifacts
}

fn optional_intent_case_artifact_refs(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
    context: &IntentCaseManifestContext,
    artifact_ids: &IntentCaseArtifactIds,
) -> Vec<IntentCaseArtifactRef> {
    let mut artifacts = Vec::new();

    if has_action(receipt, "invoke_model") {
        artifacts.push(optional_artifact_ref(
            artifact_ids.model_events.clone(),
            IntentCaseArtifactKind::ModelEvents,
            context,
            "50-model-events.receipt",
        ));
    }
    if receipt.tool_intent_count() > 0 {
        artifacts.push(optional_artifact_ref(
            artifact_ids.tool_calls.clone(),
            IntentCaseArtifactKind::ToolCalls,
            context,
            "51-tool-calls.receipt",
        ));
    }
    if receipt.memory_intent_count() > 0 {
        artifacts.push(optional_artifact_ref(
            artifact_ids.memory_receipts.clone(),
            IntentCaseArtifactKind::MemoryReceipts,
            context,
            "52-memory-receipts.receipt",
        ));
    }
    if has_capability(receipt, "+sandbox") || has_capability(receipt, "+denylist") {
        artifacts.push(optional_artifact_ref(
            artifact_ids.sandbox_receipts.clone(),
            IntentCaseArtifactKind::SandboxReceipts,
            context,
            "53-sandbox-receipts.receipt",
        ));
    }
    if receipt.live_llm_required() || has_capability(receipt, "+repair") {
        artifacts.extend(repair_artifact_refs(context, artifact_ids));
    }
    if has_action(receipt, "verify") {
        artifacts.push(optional_artifact_ref(
            artifact_ids.verifier_receipt.clone(),
            IntentCaseArtifactKind::VerifierReceipt,
            context,
            "62-verifier-receipt.receipt",
        ));
    }

    artifacts
}

fn repair_artifact_refs(
    context: &IntentCaseManifestContext,
    artifact_ids: &IntentCaseArtifactIds,
) -> [IntentCaseArtifactRef; 3] {
    [
        optional_artifact_ref(
            artifact_ids.diff_patch.clone(),
            IntentCaseArtifactKind::DiffPatch,
            context,
            "60-diff.patch",
        ),
        optional_artifact_ref(
            artifact_ids.test_before.clone(),
            IntentCaseArtifactKind::TestBefore,
            context,
            "61-test-before.receipt",
        ),
        optional_artifact_ref(
            artifact_ids.test_after.clone(),
            IntentCaseArtifactKind::TestAfter,
            context,
            "63-test-after.receipt",
        ),
    ]
}

fn optional_artifact_ref(
    artifact_id: IntentCaseArtifactId,
    kind: IntentCaseArtifactKind,
    context: &IntentCaseManifestContext,
    filename: &str,
) -> IntentCaseArtifactRef {
    IntentCaseArtifactRef::present(
        artifact_id,
        kind,
        format!("{}/{filename}", context.artifact_prefix),
    )
}

fn has_capability(receipt: &GerbilLoopCaseDriverVerticalTraceReceipt, tag: &str) -> bool {
    receipt
        .capability_tags()
        .any(|capability| capability.as_str() == tag)
}

fn has_action(receipt: &GerbilLoopCaseDriverVerticalTraceReceipt, expected: &str) -> bool {
    receipt
        .transition_actions()
        .any(|action| action == expected)
}

fn artifact_id(case_id: &str, lane: &str) -> IntentCaseArtifactId {
    IntentCaseArtifactId::new(format!("{case_id}:{lane}"))
}

fn trace_entry_id(case_id: &str, step_index: u64) -> IntentCaseTraceEntryId {
    IntentCaseTraceEntryId::new(format!("{case_id}:trace-step-{step_index}"))
}

fn transition_id(loop_program_id: &str, step_index: u64) -> IntentCaseTransitionId {
    IntentCaseTransitionId::new(format!("{loop_program_id}:transition-{step_index}"))
}

fn model_invocation_id(case_id: &str, loop_program_id: &str, step_index: u64) -> String {
    format!("{case_id}:{loop_program_id}:model-invocation-{step_index}")
}

fn tool_call_id(case_id: &str, loop_program_id: &str, step_index: u64) -> String {
    format!("{case_id}:{loop_program_id}:tool-call-{step_index}")
}

fn tool_resource_key(receipt: &GerbilLoopCaseDriverVerticalTraceReceipt) -> &'static str {
    if has_capability(receipt, "+policy-combination") {
        "agent-flow.policy-combination-tool"
    } else if has_capability(receipt, "+memory-recall")
        || has_capability(receipt, "+tool-selection")
    {
        "agent-flow.memory-selected-tool"
    } else if has_capability(receipt, "+tool-repair") || has_capability(receipt, "+repair") {
        "agent-flow.repair-tool"
    } else if has_capability(receipt, "+sandbox") || has_capability(receipt, "+denylist") {
        "agent-flow.sandboxed-tool"
    } else {
        "agent-flow.tool-intent"
    }
}

fn tool_sandbox_profile(receipt: &GerbilLoopCaseDriverVerticalTraceReceipt) -> &'static str {
    if has_capability(receipt, "+policy-combination") {
        "policy-combination-tool"
    } else if has_capability(receipt, "+denylist") {
        "sandbox-denylist"
    } else if has_capability(receipt, "+sandbox") {
        "tool-sandbox"
    } else if has_capability(receipt, "+tool-repair") || has_capability(receipt, "+repair") {
        "workspace-file-repair"
    } else {
        "scripted-tool"
    }
}

fn intent_case_artifact_prefix(case_id: &str, run_id: &str) -> String {
    format!(
        "{}/{}/{}",
        INTENT_CASE_ARTIFACT_ROOT,
        safe_artifact_path_segment(case_id),
        safe_artifact_path_segment(run_id)
    )
}

fn safe_artifact_path_segment(value: &str) -> String {
    let mut segment = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
            segment.push(character);
        } else {
            segment.push('_');
        }
    }
    if segment.is_empty() {
        "unnamed".to_owned()
    } else {
        segment
    }
}

fn policy_digest_hex(octets: &[u8]) -> String {
    let mut digest = String::with_capacity(octets.len() * 2);
    for octet in octets {
        write!(&mut digest, "{octet:02x}").expect("write hex digest into string");
    }
    digest
}
