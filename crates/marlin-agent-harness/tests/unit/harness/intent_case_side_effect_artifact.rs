use std::{fs, path::PathBuf, sync::Arc};

use super::intent_case_artifact_support::{
    artifact_content, cap, execute_vertical_receipt, gerbil_vertical_receipts, handled_by,
    observed_span_source_for_vertical_receipt_with_trace, tool_shell_resolver,
};
use marlin_agent_harness::{
    GerbilScriptedIntentCaseArtifactBundleRequest, IntentCaseArtifactBundleMaterializationReceipt,
    IntentCaseArtifactKind, TraceRecorder, materialize_gerbil_scripted_intent_case_artifact_bundle,
};
use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, LoopProgramExecutionDriver,
    LoopProgramExecutionRequest, LoopProgramExecutionStatus, LoopProgramFileSandbox,
    LoopProgramFileWriteTemplate, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, LoopProgramRuntimeSideEffectExecutor,
    PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor, ReceiptDrivenLoopProgramEventMapper,
    RetryBudgetToolHandler, StaticLoopProgramFileWriteResolver,
    StaticLoopProgramToolProcessResolver,
};
use marlin_agent_protocol::LoopProgramEventKind;
use marlin_agent_runtime::{TokioAgentRuntime, observability};
use marlin_gerbil_scheme::project_gerbil_loop_case_driver_loop_program;

#[cfg(unix)]
#[tokio::test]
async fn harness_materializes_real_tool_side_effect_receipts_into_tool_call_artifacts() {
    let receipt = gerbil_vertical_receipts()
        .into_iter()
        .find(|receipt| {
            receipt.has_capability(&cap("+policy-combination")) && receipt.tool_intent_count() > 0
        })
        .expect("policy-combination case should project tool side effects");
    let execution_receipt = execute_vertical_receipt(&receipt);
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let trace_recorder = TraceRecorder::new();
    let trace_guard = trace_recorder.install();
    let replay_bundle = LoopProgramRuntimeSideEffectExecutor::new(tool_shell_resolver(
        "printf harness-policy-combination-tool-spawn",
    ))
    .with_started_at_ms(1100)
    .with_observed_at_ms(1125)
    .execute_loop_execution(&runtime.context(), &execution_receipt)
    .await;
    drop(trace_guard);
    assert!(trace_recorder.contains_span(&observability::runtime_tool_span_name()));
    let output_root = tempfile::tempdir().expect("create tool side-effect artifact tempdir");
    let observed_span_source =
        observed_span_source_for_vertical_receipt_with_trace(&receipt, &trace_recorder);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "tool-side-effects".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
            real_llm_case_receipt: None,
        },
    )
    .expect("tool side-effect bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    assert_eq!(bundle.completeness_receipt.missing_spans, Vec::new());
    assert_run_receipt_quality_gate(&bundle);
    assert!(
        bundle
            .completeness_receipt
            .expected_spans
            .iter()
            .any(|span_name| span_name.as_str() == observability::SPAN_RUNTIME_TOOL)
    );
    let manifest_receipt =
        fs::read_to_string(&bundle.manifest_path).expect("read tool side-effect manifest");
    assert!(manifest_receipt.contains("expected_span name=runtime.tool"));
    assert!(manifest_receipt.contains("observed_span name=runtime.tool"));
    let tool_calls = artifact_content(&bundle, IntentCaseArtifactKind::ToolCalls);
    let sandbox = artifact_content(&bundle, IntentCaseArtifactKind::SandboxReceipts);
    assert!(tool_calls.contains("side_effect_replay policy_status=Ready"));
    assert!(tool_calls.contains("tool_call_id="));
    assert!(tool_calls.contains(":tool-call-"));
    assert!(tool_calls.contains("resource_key="));
    assert!(tool_calls.contains("resource_key=agent-flow.policy-combination-tool"));
    assert!(tool_calls.contains("sandbox_profile="));
    assert!(tool_calls.contains("sandbox_profile=policy-combination-tool"));
    assert!(tool_calls.contains("status=Completed"));
    assert!(tool_calls.contains("stdout_digest=fnv1a64:"));
    assert!(tool_calls.contains("stdout_bytes=37"));
    assert!(!tool_calls.contains("harness-policy-combination-tool-spawn"));
    assert!(sandbox.contains("side_effect_tool_process_count=1"));
    assert!(sandbox.contains("side_effect_completed_tool_process_count=1"));
    assert!(sandbox.contains("side_effect_file_write_count=0"));
    assert!(sandbox.contains("side_effect_sandbox_enforcement=no-file-write-projection"));
    assert!(sandbox.contains("tool_process_sandbox_projection step="));
    assert!(sandbox.contains("sandbox_profile=policy-combination-tool"));
    assert!(sandbox.contains("sandbox_scope=profile-projection"));
}

#[cfg(unix)]
#[tokio::test]
async fn harness_materializes_real_policy_002_retry_budget_gate() {
    let receipt = gerbil_vertical_receipts()
        .into_iter()
        .find(|receipt| receipt.case_id().as_str() == "real-policy-002/retry-budget")
        .expect("real-policy-002 retry-budget vertical case should exist");
    let loop_program = project_gerbil_loop_case_driver_loop_program(&receipt)
        .expect("retry-budget vertical trace projects into LoopProgram");
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        tool_handler: Arc::new(RetryBudgetToolHandler::new(
            LoopProgramRuntimeOwner::new("runtime.retry-budget.tool"),
            1,
        )),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let execution_receipt = LoopProgramExecutionDriver::new(
        PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor::new(
            LoopProgramRuntimeHandoffRouter::new(handlers),
            AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
                "runtime.agent-flow.retry-budget-tool",
            )),
        ),
    )
    .with_event_mapper(ReceiptDrivenLoopProgramEventMapper)
    .with_max_steps(receipt.transition_count() + 2)
    .run(LoopProgramExecutionRequest::new(
        loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert_eq!(execution_receipt.steps.len(), 3);
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.runtime_handoff_execution.status.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramRuntimeHandoffExecutionReportStatus::Denied,
            LoopProgramRuntimeHandoffExecutionReportStatus::Completed,
            LoopProgramRuntimeHandoffExecutionReportStatus::Completed,
        ]
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.generated_event.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(LoopProgramEventKind::Error),
            Some(LoopProgramEventKind::ToolReceipt),
            None,
        ]
    );
    assert!(
        execution_receipt.steps[0]
            .runtime_handoff_execution
            .tool_process_projections
            .is_empty()
    );
    assert_eq!(
        execution_receipt.steps[1]
            .runtime_handoff_execution
            .tool_process_projections
            .len(),
        1
    );

    let (runtime, _events) = TokioAgentRuntime::new(4);
    let trace_recorder = TraceRecorder::new();
    let trace_guard = trace_recorder.install();
    let replay_bundle = LoopProgramRuntimeSideEffectExecutor::new(tool_shell_resolver(
        "printf retry-budget-admitted-tool-spawn",
    ))
    .with_started_at_ms(3100)
    .with_observed_at_ms(3130)
    .execute_loop_execution(&runtime.context(), &execution_receipt)
    .await;
    drop(trace_guard);
    assert!(trace_recorder.contains_span(&observability::runtime_tool_span_name()));
    let output_root = tempfile::tempdir().expect("create retry-budget artifact tempdir");
    let observed_span_source =
        observed_span_source_for_vertical_receipt_with_trace(&receipt, &trace_recorder);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "real-policy-002-retry-budget-gate".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
            real_llm_case_receipt: None,
        },
    )
    .expect("retry-budget gate bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    assert_eq!(bundle.completeness_receipt.missing_spans, Vec::new());
    assert_run_receipt_quality_gate(&bundle);
    assert!(
        bundle
            .completeness_receipt
            .expected_spans
            .iter()
            .any(|span_name| span_name.as_str() == observability::SPAN_RUNTIME_TOOL)
    );
    let tool_calls = artifact_content(&bundle, IntentCaseArtifactKind::ToolCalls);
    assert!(tool_calls.contains("side_effect_replay policy_status=Ready"));
    assert!(tool_calls.contains("tool_call_id="));
    assert!(tool_calls.contains("resource_key=agent-flow.retry-budget-tool"));
    assert!(tool_calls.contains("sandbox_profile=retry-budget-tool"));
    assert!(tool_calls.contains("status=Completed"));
    assert!(tool_calls.contains("stdout_digest=fnv1a64:"));
    assert!(tool_calls.contains("stdout_bytes=32"));
    assert!(
        !tool_calls.contains("retry-budget-admitted-tool-spawn"),
        "tool artifact should retain stdout digest metadata, not raw stdout"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn harness_materializes_policy_combination_demo_artifact_bundle() {
    let receipt = gerbil_vertical_receipts()
        .into_iter()
        .find(|receipt| {
            receipt.has_capability(&cap("+policy-combination"))
                && receipt.memory_intent_count() > 0
                && receipt.tool_intent_count() > 0
        })
        .expect("policy-combination case should project memory, tool, rewrite, and checker lanes");
    let mechanism_policy_ids = receipt
        .mechanism_policy_ids()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let execution_receipt = execute_vertical_receipt(&receipt);
    let actions = execution_receipt
        .steps
        .iter()
        .map(|step| format!("{:?}", step.machine_receipt.action))
        .collect::<Vec<_>>();
    assert_eq!(
        actions,
        vec![
            "ReadMemory",
            "InvokeModel",
            "RewriteGraph",
            "DispatchTools",
            "Verify",
            "Stop"
        ]
    );

    let (runtime, _events) = TokioAgentRuntime::new(4);
    let trace_recorder = TraceRecorder::new();
    let trace_guard = trace_recorder.install();
    let replay_bundle = LoopProgramRuntimeSideEffectExecutor::new(tool_shell_resolver(
        "printf harness-policy-combination-demo-tool",
    ))
    .with_started_at_ms(2100)
    .with_observed_at_ms(2134)
    .execute_loop_execution(&runtime.context(), &execution_receipt)
    .await;
    drop(trace_guard);
    assert!(trace_recorder.contains_span(&observability::runtime_tool_span_name()));
    let output_root = tempfile::tempdir().expect("create policy-combination artifact tempdir");
    let observed_span_source =
        observed_span_source_for_vertical_receipt_with_trace(&receipt, &trace_recorder);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "policy-combination-demo".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
            real_llm_case_receipt: None,
        },
    )
    .expect("policy-combination demo bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    assert_eq!(bundle.completeness_receipt.missing_spans, Vec::new());
    assert_run_receipt_quality_gate(&bundle);
    assert!(
        bundle
            .completeness_receipt
            .expected_spans
            .iter()
            .any(|span_name| span_name.as_str() == observability::SPAN_RUNTIME_TOOL)
    );
    for kind in [
        IntentCaseArtifactKind::Intent,
        IntentCaseArtifactKind::PolicyPack,
        IntentCaseArtifactKind::PolicyMergeReceipts,
        IntentCaseArtifactKind::LoopProgram,
        IntentCaseArtifactKind::VerticalTrace,
        IntentCaseArtifactKind::ExecutionTrace,
        IntentCaseArtifactKind::ModelEvents,
        IntentCaseArtifactKind::ToolCalls,
        IntentCaseArtifactKind::MemoryReceipts,
        IntentCaseArtifactKind::VerifierReceipt,
        IntentCaseArtifactKind::PolicyExplanation,
        IntentCaseArtifactKind::ReplayScript,
        IntentCaseArtifactKind::RunReceipt,
    ] {
        assert!(
            bundle.has_artifact_kind(kind),
            "missing artifact kind {kind:?}"
        );
    }

    let manifest_receipt =
        fs::read_to_string(&bundle.manifest_path).expect("read policy-combination manifest");
    assert!(manifest_receipt.contains("completeness_status=complete"));
    assert!(manifest_receipt.contains("expected_span name=runtime.tool"));
    assert!(manifest_receipt.contains("observed_span name=runtime.tool"));
    assert!(manifest_receipt.contains("correlation case_id=policy-combination"));
    assert!(manifest_receipt.contains("run_id=policy-combination-demo"));
    assert!(manifest_receipt.contains("runtime_owner=marlin-agent-core"));

    let memory = artifact_content(&bundle, IntentCaseArtifactKind::MemoryReceipts);
    let model = artifact_content(&bundle, IntentCaseArtifactKind::ModelEvents);
    let policy_merge_receipts =
        artifact_content(&bundle, IntentCaseArtifactKind::PolicyMergeReceipts);
    let loop_program = artifact_content(&bundle, IntentCaseArtifactKind::LoopProgram);
    let tool_calls = artifact_content(&bundle, IntentCaseArtifactKind::ToolCalls);
    let verifier = artifact_content(&bundle, IntentCaseArtifactKind::VerifierReceipt);
    let policy_explanation = artifact_content(&bundle, IntentCaseArtifactKind::PolicyExplanation);

    assert!(memory.contains("memory_intent="));
    assert!(model.contains("model step="));
    assert!(model.contains("model_invocation_id="));
    assert!(model.contains(":model-invocation-"));
    assert!(policy_merge_receipts.contains("policy_merge_source=gerbil-poo-flow"));
    assert!(policy_merge_receipts.contains("policy_merge_kinds="));
    assert!(policy_merge_receipts.contains("conflict_error"));
    assert!(policy_merge_receipts.contains("policy_feedback_status=stable-resource-consensus"));
    assert!(
        policy_merge_receipts
            .contains("policy_feedback_recommendation.1=keep-exclusive-resource-mixins-aligned")
    );
    assert!(policy_merge_receipts.contains("policy_combination_experiment=memory-rewrite-checker"));
    assert!(policy_merge_receipts.contains(
        "policy_combination_expected_lanes=memory,model,rewrite,tool,tool-replay,checker"
    ));
    assert!(policy_merge_receipts.contains("policy_combination_memory_intent_count=1"));
    assert!(policy_merge_receipts.contains("policy_combination_tool_intent_count=1"));
    assert!(policy_merge_receipts.contains("policy_combination_memory_projection_count=1"));
    assert!(policy_merge_receipts.contains("policy_combination_model_step_count=1"));
    assert!(policy_merge_receipts.contains("policy_combination_rewrite_step_count=1"));
    assert!(policy_merge_receipts.contains("policy_combination_tool_projection_count=1"));
    assert!(policy_merge_receipts.contains("policy_combination_tool_replay_process_count=1"));
    assert!(policy_merge_receipts.contains("policy_combination_checker_step_count=1"));
    assert!(policy_merge_receipts.contains("policy_combination_side_effect_replay=true"));
    assert!(policy_merge_receipts.contains("policy_combination_experiment_status=complete"));
    assert!(policy_merge_receipts.contains("policy_combination_recommendation_count=0"));
    assert!(policy_merge_receipts.contains("policy_merge_internal_json_boundary=false"));
    assert!(loop_program.contains("action=rewrite_graph"));
    assert!(tool_calls.contains("side_effect_replay policy_status=Ready"));
    assert!(tool_calls.contains("tool_call_id="));
    assert!(tool_calls.contains(":tool-call-"));
    assert!(tool_calls.contains("resource_key="));
    assert!(tool_calls.contains("sandbox_profile="));
    assert!(tool_calls.contains("status=Completed"));
    assert!(tool_calls.contains("stdout_digest=fnv1a64:"));
    assert!(!tool_calls.contains("harness-policy-combination-demo-tool"));
    assert!(verifier.contains("verifier step="));
    for policy_id in mechanism_policy_ids {
        assert!(
            policy_explanation.contains(&format!("- {policy_id}")),
            "policy explanation missing mechanism policy {policy_id}"
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn harness_materializes_sandbox_file_write_receipts_into_sandbox_and_patch_artifacts() {
    let receipt = gerbil_vertical_receipts()
        .into_iter()
        .find(|receipt| {
            receipt.live_llm_required()
                && receipt.has_capability(&cap("+tool-repair"))
                && receipt.has_capability(&cap("+verification"))
        })
        .expect("repair case should project file-write side effects");
    let execution_receipt = execute_vertical_receipt(&receipt);
    let workspace = tempfile::tempdir().expect("create sandbox workspace");
    let bug_relative_path = PathBuf::from("src/lib.rs");
    let bug_file = workspace.path().join(&bug_relative_path);
    fs::create_dir_all(bug_file.parent().expect("bug fixture parent"))
        .expect("create sandbox fixture dir");
    fs::write(&bug_file, "fn answer() -> i32 { 40 }\n").expect("write sandbox fixture");
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let trace_recorder = TraceRecorder::new();
    let trace_guard = trace_recorder.install();
    let replay_bundle =
        LoopProgramRuntimeSideEffectExecutor::new(StaticLoopProgramToolProcessResolver::default())
            .with_file_write_resolver(StaticLoopProgramFileWriteResolver::new(
                vec![LoopProgramFileWriteTemplate::new(
                    "agent-flow.tool-intent",
                    ["loop-program.dispatch-tools"],
                    bug_relative_path.clone(),
                    b"fn answer() -> i32 { 41 }\n".to_vec(),
                )]
                .into_boxed_slice(),
            ))
            .with_file_sandbox(
                LoopProgramFileSandbox::new(workspace.path().to_owned())
                    .with_allowed_relative_paths([bug_relative_path]),
            )
            .execute_loop_execution(&runtime.context(), &execution_receipt)
            .await;
    drop(trace_guard);
    assert!(trace_recorder.contains_span(&observability::runtime_tool_span_name()));
    let output_root = tempfile::tempdir().expect("create sandbox artifact tempdir");
    let observed_span_source =
        observed_span_source_for_vertical_receipt_with_trace(&receipt, &trace_recorder);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "sandbox-file-write".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
            real_llm_case_receipt: None,
        },
    )
    .expect("sandbox file-write bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    assert_eq!(bundle.completeness_receipt.missing_spans, Vec::new());
    assert_run_receipt_quality_gate(&bundle);
    assert!(
        bundle
            .completeness_receipt
            .expected_spans
            .iter()
            .any(|span_name| span_name.as_str() == observability::SPAN_RUNTIME_TOOL)
    );
    let sandbox = artifact_content(&bundle, IntentCaseArtifactKind::SandboxReceipts);
    let patch = artifact_content(&bundle, IntentCaseArtifactKind::DiffPatch);
    let test_before = artifact_content(&bundle, IntentCaseArtifactKind::TestBefore);
    let test_after = artifact_content(&bundle, IntentCaseArtifactKind::TestAfter);
    assert!(sandbox.contains("side_effect_policy_status=Ready"));
    assert!(sandbox.contains("side_effect_tool_process_count=0"));
    assert!(sandbox.contains("side_effect_file_write_count=1"));
    assert!(sandbox.contains("side_effect_completed_file_write_count=1"));
    assert!(sandbox.contains("side_effect_denied_file_write_count=0"));
    assert!(sandbox.contains("side_effect_sandbox_enforcement=file-write"));
    assert!(sandbox.contains("file_write step="));
    assert!(sandbox.contains("resource_key=agent-flow."));
    assert!(sandbox.contains("sandbox_profile=workspace-file-repair"));
    assert!(sandbox.contains("relative_path=src/lib.rs"));
    assert!(sandbox.contains("status=Completed"));
    assert!(sandbox.contains("after_hash=fnv1a64:"));
    assert!(patch.contains("bytes_written=26"));
    assert!(
        test_before.contains("test_receipt_schema=marlin.intent-case.test-artifact-receipt.v1")
    );
    assert!(test_before.contains("test_receipt_phase=before"));
    assert!(test_before.contains("test_receipt_mode=side-effect-replay"));
    assert!(test_before.contains("test_receipt_status=observed"));
    assert!(test_before.contains("test_receipt_completed_file_write_count=1"));
    assert!(test_before.contains("test_receipt_patch_bytes_written=26"));
    assert!(test_after.contains("test_receipt_phase=after"));
    assert!(test_after.contains("test_receipt_mode=side-effect-replay"));
    assert!(test_after.contains("test_receipt_status=observed"));
    assert!(test_after.contains("test_receipt_file_write_count=1"));
    assert!(!test_after.contains("fn answer() -> i32 { 41 }"));
    assert_eq!(
        fs::read_to_string(&bug_file).expect("read sandbox repaired fixture"),
        "fn answer() -> i32 { 41 }\n"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn harness_materializes_sandbox_denylist_receipts_without_writing_files() {
    let receipt = gerbil_vertical_receipts()
        .into_iter()
        .find(|receipt| {
            receipt.live_llm_required()
                && receipt.has_capability(&cap("+tool-repair"))
                && receipt.has_capability(&cap("+verification"))
        })
        .expect("repair case should project denied file-write side effects");
    let execution_receipt = execute_vertical_receipt(&receipt);
    let workspace = tempfile::tempdir().expect("create sandbox deny workspace");
    let denied_relative_path = PathBuf::from("secret.rs");
    let allowed_relative_path = PathBuf::from("src/lib.rs");
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let trace_recorder = TraceRecorder::new();
    let trace_guard = trace_recorder.install();
    let replay_bundle =
        LoopProgramRuntimeSideEffectExecutor::new(StaticLoopProgramToolProcessResolver::default())
            .with_file_write_resolver(StaticLoopProgramFileWriteResolver::new(
                vec![LoopProgramFileWriteTemplate::new(
                    "agent-flow.tool-intent",
                    ["loop-program.dispatch-tools"],
                    denied_relative_path.clone(),
                    b"sealed\n".to_vec(),
                )]
                .into_boxed_slice(),
            ))
            .with_file_sandbox(
                LoopProgramFileSandbox::new(workspace.path().to_owned())
                    .with_allowed_relative_paths([allowed_relative_path]),
            )
            .execute_loop_execution(&runtime.context(), &execution_receipt)
            .await;
    drop(trace_guard);
    assert!(trace_recorder.contains_span(&observability::runtime_tool_span_name()));
    let output_root = tempfile::tempdir().expect("create sandbox deny artifact tempdir");
    let observed_span_source =
        observed_span_source_for_vertical_receipt_with_trace(&receipt, &trace_recorder);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "sandbox-denylist".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
            real_llm_case_receipt: None,
        },
    )
    .expect("sandbox denylist bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    assert_eq!(bundle.completeness_receipt.missing_spans, Vec::new());
    assert_run_receipt_quality_gate(&bundle);
    assert!(
        bundle
            .completeness_receipt
            .expected_spans
            .iter()
            .any(|span_name| span_name.as_str() == observability::SPAN_RUNTIME_TOOL)
    );
    assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::SandboxReceipts));
    assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::DiffPatch));
    let sandbox = artifact_content(&bundle, IntentCaseArtifactKind::SandboxReceipts);
    let patch = artifact_content(&bundle, IntentCaseArtifactKind::DiffPatch);
    let test_after = artifact_content(&bundle, IntentCaseArtifactKind::TestAfter);
    assert!(sandbox.contains("side_effect_policy_status=Blocked"));
    assert!(sandbox.contains("side_effect_tool_process_count=0"));
    assert!(sandbox.contains("side_effect_file_write_count=1"));
    assert!(sandbox.contains("side_effect_completed_file_write_count=0"));
    assert!(sandbox.contains("side_effect_denied_file_write_count=1"));
    assert!(sandbox.contains("side_effect_sandbox_enforcement=file-write"));
    assert!(sandbox.contains("resource_key=agent-flow."));
    assert!(sandbox.contains("sandbox_profile=workspace-file-repair"));
    assert!(sandbox.contains("relative_path=secret.rs"));
    assert!(sandbox.contains("status=Denied"));
    assert!(sandbox.contains("diagnostic=loop_program.file_write.sandbox_denied:secret.rs"));
    assert!(sandbox.contains("bytes_written=0"));
    assert!(patch.contains(
        "# file=secret.rs status=Denied diagnostic=loop_program.file_write.sandbox_denied:secret.rs"
    ));
    assert!(test_after.contains("test_receipt_schema=marlin.intent-case.test-artifact-receipt.v1"));
    assert!(test_after.contains("test_receipt_phase=after"));
    assert!(test_after.contains("test_receipt_mode=side-effect-replay"));
    assert!(test_after.contains("test_receipt_status=blocked"));
    assert!(test_after.contains("test_receipt_denied_file_write_count=1"));
    assert!(test_after.contains("test_receipt_patch_bytes_written=0"));
    assert!(!workspace.path().join("secret.rs").exists());
}

fn assert_run_receipt_quality_gate(bundle: &IntentCaseArtifactBundleMaterializationReceipt) {
    assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::RunReceipt));
    let run_receipt = artifact_content(bundle, IntentCaseArtifactKind::RunReceipt);
    assert!(run_receipt.contains("artifact_kind=run-receipt"));
    assert!(run_receipt.contains("run_receipt_schema=marlin.intent-case.run-receipt.v1"));
    assert!(
        run_receipt.contains("run_receipt_manifest_schema=marlin.intent-case.artifact-manifest.v5")
    );
    assert!(run_receipt.contains("run_receipt_status=passed"));
    assert!(run_receipt.contains(&format!(
        "run_receipt_expected_artifact_count={}",
        bundle.manifest.expected_artifact_kinds().len()
    )));
    assert!(run_receipt.contains(&format!(
        "run_receipt_materialized_artifact_count={}",
        bundle.manifest.present_artifact_kinds().len()
    )));
    assert!(run_receipt.contains(&format!(
        "run_receipt_expected_span_count={}",
        bundle.manifest.expected_span_names().len()
    )));
    assert!(run_receipt.contains(&format!(
        "run_receipt_observed_span_count={}",
        bundle.manifest.observed_span_names().len()
    )));
    assert!(run_receipt.contains("run_receipt_missing_span_count=0"));
    assert!(run_receipt.contains(&format!(
        "run_receipt_trace_entry_count={}",
        bundle.manifest.trace_index.entries.len()
    )));
    assert!(run_receipt.contains(&format!(
        "run_receipt_correlation_key_count={}",
        bundle.manifest.correlation_keys().len()
    )));
    assert!(run_receipt.contains("run_receipt_missing_trace_artifact_ref_count=0"));
    assert!(run_receipt.contains("run_receipt_missing_runtime_owner_count=0"));
    assert!(run_receipt.contains("run_receipt_missing_action_identity_count=0"));
    assert!(run_receipt.contains("run_receipt_complete_trace_correlation=true"));
    assert!(run_receipt.contains("run_receipt_internal_json_boundary=false"));
    assert!(!run_receipt.contains(".json"));
}
