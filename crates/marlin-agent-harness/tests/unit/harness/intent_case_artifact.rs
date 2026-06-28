use std::{fs, sync::Arc};

use super::intent_case_artifact_support::{
    SchemeProjectedLoopProgramEventMapper, artifact_content, cap,
    config_interface_case_driver_stdout, execute_vertical_receipt, gerbil_vertical_receipts,
    handled_by, observed_span_source_for_vertical_receipt,
    observed_span_source_for_vertical_receipt_with_trace, scheme_projected_runtime_executor,
};
use marlin_agent_harness::{
    GerbilScriptedIntentCaseArtifactBundleRequest, IntentCaseArtifactBundleMaterializationReceipt,
    IntentCaseArtifactKind, IntentCaseObservedSpanSource, IntentCaseSpanName,
    StaticProviderRuntime, TraceRecorder, materialize_gerbil_scripted_intent_case_artifact_bundle,
};
use marlin_agent_harness_types::{
    RuntimeRepairCaseId, RuntimeRepairCaseReceipt, RuntimeRepairContentSummary, RuntimeRepairCount,
    RuntimeRepairDenialReason, RuntimeRepairDurationMillis, RuntimeRepairHandoffStatus,
    RuntimeRepairLiveCaseReceipt, RuntimeRepairLiveCaseReceiptRequest, RuntimeRepairLiveGateStatus,
    RuntimeRepairModelCompletionId, RuntimeRepairModelId, RuntimeRepairNoLiveCaseReceipt,
    RuntimeRepairNoLiveCaseReceiptRequest, RuntimeRepairProfileRef,
};
use marlin_agent_kernel::{
    DenylistedLoopProgramToolDispatchHandler, LoopProgramExecutionDriver,
    LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffExecutionStatus,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner,
};
use marlin_agent_protocol::LoopProgramEventKind;
use marlin_agent_runtime::{TokioAgentRuntime, observability};
use marlin_gerbil_scheme::{
    project_gerbil_loop_case_driver_loop_program, verify_gerbil_loop_case_driver_vertical_trace,
};

#[test]
fn harness_materializes_scripted_intent_case_bundles_for_all_gerbil_vertical_cases() {
    let stdout = config_interface_case_driver_stdout();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");
    let output_root = tempfile::tempdir().expect("create intent-case artifact tempdir");

    for receipt in &vertical_receipts {
        let loop_program = project_gerbil_loop_case_driver_loop_program(receipt)
            .expect("vertical trace projects into LoopProgram");
        let driver = LoopProgramExecutionDriver::new(scheme_projected_runtime_executor())
            .with_event_mapper(SchemeProjectedLoopProgramEventMapper::from_vertical_trace(
                receipt,
            ))
            .with_max_steps(receipt.transition_count() + 2);
        let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
            loop_program,
            vec![LoopProgramEventKind::Start],
        ));
        let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
            GerbilScriptedIntentCaseArtifactBundleRequest {
                output_root: output_root.path().to_owned(),
                run_id: format!("scripted-bundle-{}", receipt.case_id().as_str()).into(),
                vertical_trace: receipt.clone(),
                execution_receipt,
                side_effect_replay_bundle: None,
                runtime_repair_receipt: None,
                observed_span_source: Some(observed_span_source_for_vertical_receipt(receipt)),
            },
        )
        .expect("scripted intent-case bundle materializes");

        assert!(bundle.bundle_root.is_dir());
        assert!(bundle.manifest_path.is_file());
        assert!(bundle.manifest.has_core_artifact_bundle());
        assert!(bundle.manifest.has_complete_trace_correlation());
        assert!(bundle.completeness_receipt.is_supported_schema());
        assert!(bundle.completeness_receipt.is_complete());
        assert_eq!(bundle.completeness_receipt.case_id, bundle.manifest.case_id);
        assert_eq!(bundle.completeness_receipt.run_id, bundle.manifest.run_id);
        assert_eq!(
            bundle.completeness_receipt.policy_digest,
            bundle.manifest.policy_digest
        );
        assert_eq!(
            bundle.completeness_receipt.loop_program_id,
            bundle.manifest.loop_program_id
        );
        assert_eq!(
            bundle.completeness_receipt.expected_artifacts,
            bundle.manifest.present_artifact_kinds()
        );
        assert_eq!(
            bundle.completeness_receipt.materialized_artifacts,
            bundle.manifest.present_artifact_kinds()
        );
        assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
        assert_eq!(
            bundle.completeness_receipt.trace_entry_count,
            receipt.transition_count()
        );
        assert_eq!(
            bundle.completeness_receipt.correlation_key_count,
            bundle.manifest.correlation_keys().len()
        );
        assert_eq!(bundle.manifest.trace_artifact_ref_missing_ids(), Vec::new());
        assert_eq!(
            bundle.manifest.trace_entries_without_runtime_owner(),
            Vec::new()
        );
        assert!(!bundle.manifest.correlation_keys().is_empty());
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::Intent));
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::PolicyPack));
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::LoopProgram));
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::VerticalTrace));
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::ExecutionTrace));
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::ReplayScript));
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::RunReceipt));
        assert_eq!(
            bundle.artifacts.len(),
            bundle
                .manifest
                .artifacts
                .iter()
                .filter(|artifact| artifact.present)
                .count()
        );
        assert_eq!(
            bundle.manifest.correlation_keys().len(),
            bundle
                .manifest
                .trace_index
                .entries
                .iter()
                .map(|entry| entry.artifact_refs.len())
                .sum::<usize>()
        );

        let manifest_receipt =
            fs::read_to_string(&bundle.manifest_path).expect("read manifest receipt");
        let replay_script = artifact_content(&bundle, IntentCaseArtifactKind::ReplayScript);
        let run_receipt = artifact_content(&bundle, IntentCaseArtifactKind::RunReceipt);
        assert!(manifest_receipt.contains("completeness_schema="));
        assert!(manifest_receipt.contains("expected_artifact_count="));
        assert!(manifest_receipt.contains("materialized_artifact_count="));
        assert!(manifest_receipt.contains("missing_artifact_count=0"));
        assert!(manifest_receipt.contains("expected_span_count=2"));
        assert!(manifest_receipt.contains("observed_span_count=2"));
        assert!(manifest_receipt.contains("missing_span_count=0"));
        assert!(manifest_receipt.contains("expected_span name=harness.execution"));
        assert!(manifest_receipt.contains("expected_span name=harness.result"));
        assert!(manifest_receipt.contains("observed_span name=harness.execution"));
        assert!(manifest_receipt.contains("observed_span name=harness.result"));
        assert!(manifest_receipt.contains("completeness_status=complete"));
        assert!(manifest_receipt.contains("correlation_key_count="));
        assert!(manifest_receipt.contains("correlation case_id="));
        assert!(manifest_receipt.contains("run_id=scripted-bundle-"));
        assert!(manifest_receipt.contains("policy_digest="));
        assert!(manifest_receipt.contains("loop_program_id="));
        assert!(
            replay_script.contains("replay_receipt_schema=marlin.intent-case.replay-receipt.v1")
        );
        assert!(replay_script.contains("replay_case_id="));
        assert!(replay_script.contains("replay_run_id=scripted-bundle-"));
        assert!(replay_script.contains("replay_policy_digest="));
        assert!(replay_script.contains("replay_loop_program_id="));
        assert!(replay_script.contains("replay_expected_artifact_count="));
        assert!(replay_script.contains("replay_expected_artifact_lanes="));
        assert!(
            replay_script.contains("replay_expected_span_names=harness.execution,harness.result")
        );
        assert!(replay_script.contains("replay_trace_entry_count="));
        assert!(replay_script.contains("replay_correlation_key_count="));
        assert!(replay_script.contains("replay_internal_json_boundary=false"));
        assert!(run_receipt.contains("run_receipt_schema=marlin.intent-case.run-receipt.v1"));
        assert!(
            run_receipt
                .contains("run_receipt_manifest_schema=marlin.intent-case.artifact-manifest.v4")
        );
        assert!(run_receipt.contains("run_receipt_status=passed"));
        assert!(run_receipt.contains("run_receipt_case_id="));
        assert!(run_receipt.contains("run_receipt_run_id=scripted-bundle-"));
        assert!(run_receipt.contains("run_receipt_policy_digest="));
        assert!(run_receipt.contains("run_receipt_loop_program_id="));
        assert!(run_receipt.contains("run_receipt_expected_artifact_count="));
        assert!(run_receipt.contains("run_receipt_materialized_artifact_count="));
        assert!(run_receipt.contains("run_receipt_expected_artifact_lanes="));
        assert!(run_receipt.contains("run-receipt"));
        assert!(run_receipt.contains("run_receipt_trace_entry_count="));
        assert!(run_receipt.contains("run_receipt_correlation_key_count="));
        assert!(run_receipt.contains("run_receipt_diagnostic_count=0"));
        assert!(run_receipt.contains("run_receipt_internal_json_boundary=false"));
        assert!(
            replay_script.contains(
                "replay_command='direnv exec . rtk --ultra-compact cargo test -p marlin-agent-harness intent_case'"
            )
        );
        assert!(!replay_script.contains(".json"));
        assert!(!run_receipt.contains(".json"));
        assert!(manifest_receipt.contains("step_index=1"));
        assert!(manifest_receipt.contains("runtime_owner=marlin-agent-core"));
        assert!(manifest_receipt.contains("model_invocation_id="));
        assert!(manifest_receipt.contains("tool_call_id="));
        assert!(manifest_receipt.contains("resource_key="));
        assert!(manifest_receipt.contains("sandbox_profile="));
        if receipt
            .transition_actions()
            .any(|action| action == "invoke_model")
        {
            assert!(
                manifest_receipt.contains(":model-invocation-"),
                "manifest receipt missing model invocation id for {}",
                receipt.case_id()
            );
        }
        if receipt
            .transition_actions()
            .any(|action| action == "dispatch_tools")
        {
            assert!(
                manifest_receipt.contains(":tool-call-"),
                "manifest receipt missing tool call id for {}",
                receipt.case_id()
            );
            assert!(
                manifest_receipt.contains("resource_key=agent-flow."),
                "manifest receipt missing resource key for {}",
                receipt.case_id()
            );
        }

        for artifact in &bundle.artifacts {
            assert!(artifact.path.is_file(), "missing artifact {artifact:?}");
            assert_ne!(artifact.bytes_written, 0, "empty artifact {artifact:?}");
            let content = fs::read_to_string(&artifact.path).expect("read materialized artifact");
            assert_artifact_correlation_header(&content, &bundle, artifact.kind);
            let path = artifact.path.to_string_lossy();
            assert!(
                !path.ends_with(".json") && !path.ends_with(".jsonl"),
                "internal scripted bundle should not use JSON artifact paths: {path}"
            );
        }

        if receipt.tool_intent_count() > 0 {
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::ToolCalls));
        }
        if receipt.memory_intent_count() > 0 {
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::MemoryReceipts));
        }
        if receipt.live_llm_required() {
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::ModelEvents));
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::DiffPatch));
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::TestBefore));
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::TestAfter));
        }
    }
}

#[test]
fn harness_materializes_observed_span_source_into_intent_case_manifest_receipt() {
    let receipt = gerbil_vertical_receipts()
        .into_iter()
        .next()
        .expect("at least one Gerbil vertical case should exist");
    let execution_receipt = execute_vertical_receipt(&receipt);
    let output_root = tempfile::tempdir().expect("create observed span artifact tempdir");
    let observed_span_source = IntentCaseObservedSpanSource::new([
        "harness.result",
        "harness.execution",
        "harness.result",
    ]);
    let expected_spans: Vec<IntentCaseSpanName> =
        vec!["harness.execution".into(), "harness.result".into()];

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "observed-spans".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: None,
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
        },
    )
    .expect("observed span bundle materializes");

    assert_eq!(bundle.manifest.expected_span_names(), expected_spans);
    assert_eq!(bundle.manifest.observed_span_names(), expected_spans);
    assert_eq!(bundle.completeness_receipt.expected_spans, expected_spans);
    assert_eq!(bundle.completeness_receipt.observed_spans, expected_spans);
    assert_eq!(
        bundle.completeness_receipt.missing_spans,
        Vec::<IntentCaseSpanName>::new()
    );
    let manifest_receipt =
        fs::read_to_string(&bundle.manifest_path).expect("read observed span manifest");
    assert!(manifest_receipt.contains("observed_span_count=2"));
    assert!(manifest_receipt.contains("observed_span name=harness.execution"));
    assert!(manifest_receipt.contains("observed_span name=harness.result"));
    assert!(manifest_receipt.contains("missing_span_count=0"));
}

#[test]
fn harness_materializes_real_policy_001_sandbox_denylist_gate() {
    let receipt = gerbil_vertical_receipts()
        .into_iter()
        .find(|receipt| receipt.case_id().as_str() == "real-policy-001/sandbox-denylist")
        .expect("real-policy-001 sandbox-denylist vertical case should exist");
    let loop_program = project_gerbil_loop_case_driver_loop_program(&receipt)
        .expect("sandbox-denylist vertical trace projects into LoopProgram");
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        tool_handler: Arc::new(DenylistedLoopProgramToolDispatchHandler::new(
            LoopProgramRuntimeOwner::new("runtime.sandbox.denylist"),
            ["loop-program.dispatch-tools"],
        )),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let execution_receipt =
        LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
            .with_event_mapper(SchemeProjectedLoopProgramEventMapper::from_vertical_trace(
                &receipt,
            ))
            .with_max_steps(receipt.transition_count() + 2)
            .run(LoopProgramExecutionRequest::new(
                loop_program,
                vec![LoopProgramEventKind::Start],
            ));
    let output_root = tempfile::tempdir().expect("create sandbox-denylist artifact tempdir");
    let observed_span_source = observed_span_source_for_vertical_receipt(&receipt);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "real-policy-001-sandbox-denylist-gate".into(),
            vertical_trace: receipt,
            execution_receipt: execution_receipt.clone(),
            side_effect_replay_bundle: None,
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
        },
    )
    .expect("sandbox-denylist gate bundle materializes");

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert_eq!(execution_receipt.steps.len(), 2);
    assert_eq!(
        execution_receipt.steps[0].runtime_handoff_execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Denied
    );
    assert_eq!(
        execution_receipt.steps[0].generated_event,
        Some(LoopProgramEventKind::Error)
    );
    assert_eq!(
        execution_receipt.steps[0]
            .runtime_handoff_execution
            .executions[0]
            .status,
        LoopProgramRuntimeHandoffExecutionStatus::Denied
    );
    assert_eq!(
        execution_receipt.steps[0]
            .runtime_handoff_execution
            .executions[0]
            .owner
            .as_str(),
        "runtime.sandbox.denylist"
    );
    assert!(
        execution_receipt.steps[0]
            .runtime_handoff_execution
            .tool_process_projections
            .is_empty(),
        "denylisted tool handoff must not project process side effects"
    );
    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::SandboxReceipts));
    let sandbox = artifact_content(&bundle, IntentCaseArtifactKind::SandboxReceipts);
    assert!(sandbox.contains("owner=runtime.sandbox.denylist"));
    assert!(sandbox.contains("status=Denied"));
    assert!(sandbox.contains("resource_key=agent-flow.sandboxed-tool"));
    assert!(sandbox.contains("sandbox_profile=sandbox-denylist"));
    let tool_calls = artifact_content(&bundle, IntentCaseArtifactKind::ToolCalls);
    assert!(tool_calls.contains("tool_calls=none"));
}

#[tokio::test]
async fn harness_materializes_runtime_repair_receipt_into_verifier_artifact() {
    let receipt = gerbil_vertical_receipts()
        .into_iter()
        .find(|receipt| {
            receipt.live_llm_required()
                && receipt.has_capability(&cap("+tool-repair"))
                && receipt.has_capability(&cap("+verification"))
        })
        .expect("repair case should project verifier receipt lane");
    let execution_receipt = execute_vertical_receipt(&receipt);
    let graph_rewrite_projected = execution_receipt
        .steps
        .iter()
        .any(|step| format!("{:?}", step.machine_receipt.action) == "RewriteGraph");
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let trace_recorder = TraceRecorder::new();
    let trace_guard = trace_recorder.install();
    let model_completion_id = runtime
        .spawn_provider(
            Arc::new(StaticProviderRuntime::<String, String>::new(
                "fixture-live-repair-completion".to_owned(),
            )),
            "repair model request".to_owned(),
        )
        .join()
        .await
        .expect("provider runtime task should finish");
    drop(trace_guard);
    assert!(trace_recorder.contains_span(&observability::runtime_provider_span_name()));
    let runtime_repair_receipt =
        RuntimeRepairLiveCaseReceipt::new(RuntimeRepairLiveCaseReceiptRequest {
            case_id: RuntimeRepairCaseId::new(receipt.case_id().as_str()),
            profile_ref: RuntimeRepairProfileRef::new(receipt.profile_ref().as_str()),
            program_id: execution_receipt.program_id.clone(),
            model_completion_id: RuntimeRepairModelCompletionId::new(model_completion_id),
            model: RuntimeRepairModelId::new("gpt-repair-policy-fixture"),
            elapsed_ms: RuntimeRepairDurationMillis::new(7),
            action_count: RuntimeRepairCount::new(execution_receipt.steps.len()),
            tool_projection_count: RuntimeRepairCount::new(1),
            patch_tool_success: true,
            graph_rewrite_projected,
            verification_success: true,
            repaired_content: RuntimeRepairContentSummary::from_text("fn answer() -> i32 { 41 }\n"),
        });
    let output_root = tempfile::tempdir().expect("create runtime repair artifact tempdir");
    let observed_span_source =
        observed_span_source_for_vertical_receipt_with_trace(&receipt, &trace_recorder);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "runtime-repair-receipt".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: None,
            runtime_repair_receipt: Some(RuntimeRepairCaseReceipt::from(runtime_repair_receipt)),
            observed_span_source: Some(observed_span_source),
        },
    )
    .expect("runtime repair receipt bundle materializes");

    let verifier = artifact_content(&bundle, IntentCaseArtifactKind::VerifierReceipt);
    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_spans, Vec::new());
    assert!(
        bundle
            .completeness_receipt
            .expected_spans
            .iter()
            .any(|span_name| span_name.as_str() == observability::SPAN_RUNTIME_PROVIDER)
    );
    let manifest_receipt =
        fs::read_to_string(&bundle.manifest_path).expect("read runtime repair manifest");
    assert!(manifest_receipt.contains("expected_span name=runtime.provider"));
    assert!(manifest_receipt.contains("observed_span name=runtime.provider"));
    let model_events = artifact_content(&bundle, IntentCaseArtifactKind::ModelEvents);
    assert!(model_events.contains("model step="));
    assert!(model_events.contains("runtime_repair_model_event=present"));
    assert!(model_events.contains("runtime_repair_model_event_kind=live"));
    assert!(
        model_events.contains(
            "runtime_repair_model_event_schema=marlin.runtime-repair.live-case-receipt.v1"
        )
    );
    assert!(
        model_events
            .contains("runtime_repair_model_event_completion_id=fixture-live-repair-completion")
    );
    assert!(model_events.contains("runtime_repair_model_event_model=gpt-repair-policy-fixture"));
    assert!(model_events.contains("runtime_repair_model_event_elapsed_ms=7"));
    assert!(!model_events.contains("repair model request"));
    assert!(!model_events.contains("fn answer() -> i32 { 41 }"));
    let test_after = artifact_content(&bundle, IntentCaseArtifactKind::TestAfter);
    assert!(test_after.contains("test_receipt_schema=marlin.intent-case.test-artifact-receipt.v1"));
    assert!(test_after.contains("test_receipt_phase=after"));
    assert!(test_after.contains("test_receipt_mode=runtime-repair"));
    assert!(test_after.contains("test_receipt_status=verified"));
    assert!(test_after.contains("test_receipt_runtime_repair_kind=live"));
    assert!(test_after.contains("test_receipt_runtime_repair_verification_success=true"));
    assert!(!test_after.contains("fn answer() -> i32 { 41 }"));
    assert!(verifier.contains("runtime_repair_receipt=present"));
    assert!(verifier.contains("runtime_repair_kind=live"));
    assert!(verifier.contains("runtime_repair_schema=marlin.runtime-repair.live-case-receipt.v1"));
    assert!(verifier.contains("runtime_repair_model=gpt-repair-policy-fixture"));
    assert!(verifier.contains("runtime_repair_repaired_content_digest=fnv1a64:"));
    assert!(verifier.contains("runtime_repair_repaired_content_bytes=26"));
    assert!(
        !verifier.contains("fn answer() -> i32 { 41 }"),
        "verifier artifact should keep repaired content as digest metadata, not raw source"
    );
}

#[test]
fn harness_materializes_no_live_llm_gate_receipt_into_verifier_artifact() {
    let receipt = gerbil_vertical_receipts()
        .into_iter()
        .find(|receipt| {
            receipt.live_llm_required()
                && receipt.has_capability(&cap("+tool-repair"))
                && receipt.has_capability(&cap("+verification"))
        })
        .expect("repair case should project no-live verifier receipt lane");
    let execution_receipt = execute_vertical_receipt(&receipt);
    let no_live_receipt =
        RuntimeRepairNoLiveCaseReceipt::new(RuntimeRepairNoLiveCaseReceiptRequest {
            case_id: RuntimeRepairCaseId::new(receipt.case_id().as_str()),
            profile_ref: RuntimeRepairProfileRef::new(receipt.profile_ref().as_str()),
            program_id: execution_receipt.program_id.clone(),
            gate_status: RuntimeRepairLiveGateStatus::Disabled,
            denial_reason: RuntimeRepairDenialReason::new("live-llm-disabled"),
            live_llm_allowed: false,
            action_count: RuntimeRepairCount::new(execution_receipt.steps.len()),
            model_handoff_status: RuntimeRepairHandoffStatus::Denied,
        });
    let output_root = tempfile::tempdir().expect("create no-live repair artifact tempdir");
    let observed_span_source = observed_span_source_for_vertical_receipt(&receipt);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "no-live-runtime-repair-receipt".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: None,
            runtime_repair_receipt: Some(RuntimeRepairCaseReceipt::from(no_live_receipt)),
            observed_span_source: Some(observed_span_source),
        },
    )
    .expect("no-live runtime repair receipt bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    assert!(
        !bundle
            .completeness_receipt
            .expected_spans
            .iter()
            .any(|span_name| span_name.as_str() == observability::SPAN_RUNTIME_PROVIDER),
        "no-live repair denial must not claim a provider runtime span"
    );
    assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::VerifierReceipt));
    let model_events = artifact_content(&bundle, IntentCaseArtifactKind::ModelEvents);
    assert!(model_events.contains("model step="));
    assert!(model_events.contains("runtime_repair_model_event=present"));
    assert!(model_events.contains("runtime_repair_model_event_kind=no-live"));
    assert!(model_events.contains(
        "runtime_repair_model_event_schema=marlin.runtime-repair.no-live-case-receipt.v1"
    ));
    assert!(model_events.contains("runtime_repair_model_event_gate_status=Disabled"));
    assert!(model_events.contains("runtime_repair_model_event_denial_reason=live-llm-disabled"));
    assert!(model_events.contains("runtime_repair_model_event_live_llm_allowed=false"));
    assert!(model_events.contains("runtime_repair_model_event_handoff_status=Denied"));
    assert!(!model_events.contains("runtime_repair_model_event_completion_id="));
    let test_after = artifact_content(&bundle, IntentCaseArtifactKind::TestAfter);
    assert!(test_after.contains("test_receipt_schema=marlin.intent-case.test-artifact-receipt.v1"));
    assert!(test_after.contains("test_receipt_phase=after"));
    assert!(test_after.contains("test_receipt_mode=runtime-repair"));
    assert!(test_after.contains("test_receipt_status=blocked"));
    assert!(test_after.contains("test_receipt_runtime_repair_kind=no-live"));
    assert!(test_after.contains("test_receipt_runtime_repair_verification_success=none"));
    let verifier = artifact_content(&bundle, IntentCaseArtifactKind::VerifierReceipt);
    assert!(verifier.contains("runtime_repair_receipt=present"));
    assert!(verifier.contains("runtime_repair_kind=no-live"));
    assert!(
        verifier.contains("runtime_repair_schema=marlin.runtime-repair.no-live-case-receipt.v1")
    );
    assert!(verifier.contains("runtime_repair_gate_status=Disabled"));
    assert!(verifier.contains("runtime_repair_denial_reason=live-llm-disabled"));
    assert!(verifier.contains("runtime_repair_live_llm_allowed=false"));
    assert!(verifier.contains("runtime_repair_model_handoff_status=Denied"));
    assert!(!verifier.contains("runtime_repair_model_completion_id="));
}

fn assert_artifact_correlation_header(
    content: &str,
    bundle: &IntentCaseArtifactBundleMaterializationReceipt,
    kind: IntentCaseArtifactKind,
) {
    let artifact_id = bundle
        .manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == kind && artifact.present)
        .expect("artifact kind should be present in manifest")
        .artifact_id
        .as_str();
    assert_header_value(
        content,
        "artifact_receipt_schema",
        "marlin.intent-case.artifact-receipt.v1",
    );
    assert_header_value(content, "artifact_kind", artifact_kind_name(kind));
    assert_header_value(content, "artifact_id", artifact_id);
    assert_header_value(content, "case_id", bundle.manifest.case_id.as_str());
    assert_header_value(content, "run_id", bundle.manifest.run_id.as_str());
    assert_header_value(
        content,
        "policy_epoch",
        &bundle.manifest.policy_epoch.to_string(),
    );
    assert_header_value(
        content,
        "policy_digest",
        bundle.manifest.policy_digest.as_str(),
    );
    assert_header_value(
        content,
        "loop_program_id",
        bundle.manifest.loop_program_id.as_str(),
    );
    assert_header_value(
        content,
        "trace_entry_count",
        &bundle.manifest.trace_index.entries.len().to_string(),
    );
    assert_header_value(
        content,
        "correlation_key_count",
        &bundle.manifest.correlation_keys().len().to_string(),
    );
    assert_header_value(content, "internal_json_boundary", "false");
}

fn assert_header_value(content: &str, key: &str, value: &str) {
    let key_value = format!("{key}={value}");
    let org_property = format!(":{}: {value}", key.to_ascii_uppercase());
    assert!(
        content.contains(&key_value) || content.contains(&org_property),
        "artifact missing header value {key_value}"
    );
}

fn artifact_kind_name(kind: IntentCaseArtifactKind) -> &'static str {
    match kind {
        IntentCaseArtifactKind::Intent => "intent",
        IntentCaseArtifactKind::PolicyPack => "policy-pack",
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
