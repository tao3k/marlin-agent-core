use std::{
    fs,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use marlin_agent_harness::{
    GerbilScriptedIntentCaseArtifactBundleRequest, IntentCaseArtifactKind,
    IntentCaseObservedSpanSource, IntentCaseSpanName,
    materialize_gerbil_scripted_intent_case_artifact_bundle,
};
use marlin_agent_harness_types::{
    RuntimeRepairCaseId, RuntimeRepairCaseReceipt, RuntimeRepairContentSummary, RuntimeRepairCount,
    RuntimeRepairDenialReason, RuntimeRepairDurationMillis, RuntimeRepairHandoffStatus,
    RuntimeRepairLiveCaseReceipt, RuntimeRepairLiveCaseReceiptRequest, RuntimeRepairLiveGateStatus,
    RuntimeRepairModelCompletionId, RuntimeRepairModelId, RuntimeRepairNoLiveCaseReceipt,
    RuntimeRepairNoLiveCaseReceiptRequest, RuntimeRepairProfileRef,
};
use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, DenylistedLoopProgramToolDispatchHandler,
    GenericLoopMachineReceipt, HybridLoopProgramRuntimeHandoffExecutor, LoopProgramEventMapper,
    LoopProgramExecutionDriver, LoopProgramExecutionReceipt, LoopProgramExecutionRequest,
    LoopProgramExecutionStatus, LoopProgramFileSandbox, LoopProgramFileWriteTemplate,
    LoopProgramRuntimeHandoffExecutionReceipt, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffExecutionStatus, LoopProgramRuntimeHandoffHandler,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramToolProcessCommandTemplate, LoopProgramToolProcessProgram,
    PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor, ReceiptDrivenLoopProgramEventMapper,
    RetryBudgetToolHandler, StaticLoopProgramFileWriteResolver,
    StaticLoopProgramRuntimeHandoffHandler, StaticLoopProgramToolProcessResolver,
};
use marlin_agent_protocol::LoopProgramEventKind;
use marlin_agent_runtime::TokioAgentRuntime;
use marlin_gerbil_scheme::{
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverVerticalTraceReceipt,
    project_gerbil_loop_case_driver_intent_case_artifact_manifest,
    project_gerbil_loop_case_driver_loop_event_kind, project_gerbil_loop_case_driver_loop_program,
    run_gerbil_config_interface_case_driver_smoke, verify_gerbil_loop_case_driver_vertical_trace,
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
    let replay_bundle = LoopProgramRuntimeSideEffectExecutor::new(tool_shell_resolver(
        "printf harness-policy-combination-tool-spawn",
    ))
    .with_started_at_ms(1100)
    .with_observed_at_ms(1125)
    .execute_loop_execution(&runtime.context(), &execution_receipt)
    .await;
    let output_root = tempfile::tempdir().expect("create tool side-effect artifact tempdir");
    let observed_span_source = observed_span_source_for_vertical_receipt(&receipt);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "tool-side-effects".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
        },
    )
    .expect("tool side-effect bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    let tool_calls = artifact_content(&bundle, IntentCaseArtifactKind::ToolCalls);
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
    let replay_bundle = LoopProgramRuntimeSideEffectExecutor::new(tool_shell_resolver(
        "printf retry-budget-admitted-tool-spawn",
    ))
    .with_started_at_ms(3100)
    .with_observed_at_ms(3130)
    .execute_loop_execution(&runtime.context(), &execution_receipt)
    .await;
    let output_root = tempfile::tempdir().expect("create retry-budget artifact tempdir");
    let observed_span_source = observed_span_source_for_vertical_receipt(&receipt);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "real-policy-002-retry-budget-gate".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
        },
    )
    .expect("retry-budget gate bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
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
    let replay_bundle = LoopProgramRuntimeSideEffectExecutor::new(tool_shell_resolver(
        "printf harness-policy-combination-demo-tool",
    ))
    .with_started_at_ms(2100)
    .with_observed_at_ms(2134)
    .execute_loop_execution(&runtime.context(), &execution_receipt)
    .await;
    let output_root = tempfile::tempdir().expect("create policy-combination artifact tempdir");
    let observed_span_source = observed_span_source_for_vertical_receipt(&receipt);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "policy-combination-demo".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
        },
    )
    .expect("policy-combination demo bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    for kind in [
        IntentCaseArtifactKind::Intent,
        IntentCaseArtifactKind::PolicyPack,
        IntentCaseArtifactKind::LoopProgram,
        IntentCaseArtifactKind::VerticalTrace,
        IntentCaseArtifactKind::ExecutionTrace,
        IntentCaseArtifactKind::ModelEvents,
        IntentCaseArtifactKind::ToolCalls,
        IntentCaseArtifactKind::MemoryReceipts,
        IntentCaseArtifactKind::VerifierReceipt,
        IntentCaseArtifactKind::PolicyExplanation,
        IntentCaseArtifactKind::ReplayScript,
    ] {
        assert!(
            bundle.has_artifact_kind(kind),
            "missing artifact kind {kind:?}"
        );
    }

    let manifest_receipt =
        fs::read_to_string(&bundle.manifest_path).expect("read policy-combination manifest");
    assert!(manifest_receipt.contains("completeness_status=complete"));
    assert!(manifest_receipt.contains("correlation case_id=policy-combination"));
    assert!(manifest_receipt.contains("run_id=policy-combination-demo"));
    assert!(manifest_receipt.contains("runtime_owner=marlin-agent-core"));

    let memory = artifact_content(&bundle, IntentCaseArtifactKind::MemoryReceipts);
    let model = artifact_content(&bundle, IntentCaseArtifactKind::ModelEvents);
    let loop_program = artifact_content(&bundle, IntentCaseArtifactKind::LoopProgram);
    let tool_calls = artifact_content(&bundle, IntentCaseArtifactKind::ToolCalls);
    let verifier = artifact_content(&bundle, IntentCaseArtifactKind::VerifierReceipt);
    let policy_explanation = artifact_content(&bundle, IntentCaseArtifactKind::PolicyExplanation);

    assert!(memory.contains("memory_intent="));
    assert!(model.contains("model step="));
    assert!(model.contains("model_invocation_id="));
    assert!(model.contains(":model-invocation-"));
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
    let output_root = tempfile::tempdir().expect("create sandbox artifact tempdir");
    let observed_span_source = observed_span_source_for_vertical_receipt(&receipt);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "sandbox-file-write".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
        },
    )
    .expect("sandbox file-write bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    let sandbox = artifact_content(&bundle, IntentCaseArtifactKind::SandboxReceipts);
    let patch = artifact_content(&bundle, IntentCaseArtifactKind::DiffPatch);
    assert!(sandbox.contains("side_effect_policy_status=Ready"));
    assert!(sandbox.contains("file_write step="));
    assert!(sandbox.contains("resource_key=agent-flow."));
    assert!(sandbox.contains("sandbox_profile=workspace-file-repair"));
    assert!(sandbox.contains("relative_path=src/lib.rs"));
    assert!(sandbox.contains("status=Completed"));
    assert!(sandbox.contains("after_hash=fnv1a64:"));
    assert!(patch.contains("bytes_written=26"));
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
    let output_root = tempfile::tempdir().expect("create sandbox deny artifact tempdir");
    let observed_span_source = observed_span_source_for_vertical_receipt(&receipt);

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "sandbox-denylist".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span_source),
        },
    )
    .expect("sandbox denylist bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::SandboxReceipts));
    assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::DiffPatch));
    let sandbox = artifact_content(&bundle, IntentCaseArtifactKind::SandboxReceipts);
    let patch = artifact_content(&bundle, IntentCaseArtifactKind::DiffPatch);
    assert!(sandbox.contains("side_effect_policy_status=Blocked"));
    assert!(sandbox.contains("resource_key=agent-flow."));
    assert!(sandbox.contains("sandbox_profile=workspace-file-repair"));
    assert!(sandbox.contains("relative_path=secret.rs"));
    assert!(sandbox.contains("status=Denied"));
    assert!(sandbox.contains("diagnostic=loop_program.file_write.sandbox_denied:secret.rs"));
    assert!(sandbox.contains("bytes_written=0"));
    assert!(patch.contains(
        "# file=secret.rs status=Denied diagnostic=loop_program.file_write.sandbox_denied:secret.rs"
    ));
    assert!(!workspace.path().join("secret.rs").exists());
}

#[test]
fn harness_materializes_runtime_repair_receipt_into_verifier_artifact() {
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
    let runtime_repair_receipt =
        RuntimeRepairLiveCaseReceipt::new(RuntimeRepairLiveCaseReceiptRequest {
            case_id: RuntimeRepairCaseId::new(receipt.case_id().as_str()),
            profile_ref: RuntimeRepairProfileRef::new(receipt.profile_ref().as_str()),
            program_id: execution_receipt.program_id.clone(),
            model_completion_id: RuntimeRepairModelCompletionId::new(
                "fixture-live-repair-completion",
            ),
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
    let observed_span_source = observed_span_source_for_vertical_receipt(&receipt);

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
    assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::VerifierReceipt));
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

fn config_interface_case_driver_stdout() -> String {
    static STDOUT: OnceLock<String> = OnceLock::new();

    STDOUT
        .get_or_init(|| {
            run_gerbil_config_interface_case_driver_smoke()
                .expect("gxi case-driver smoke should produce verified stdout")
        })
        .clone()
}

fn gerbil_vertical_receipts() -> Vec<GerbilLoopCaseDriverVerticalTraceReceipt> {
    verify_gerbil_loop_case_driver_vertical_trace(&config_interface_case_driver_stdout(), 7)
        .expect("vertical trace verifies")
}

fn observed_span_source_for_vertical_receipt(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> IntentCaseObservedSpanSource {
    let manifest = project_gerbil_loop_case_driver_intent_case_artifact_manifest(
        receipt,
        "expected-span-probe",
    );
    IntentCaseObservedSpanSource::new(manifest.expected_span_names())
}

fn execute_vertical_receipt(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> LoopProgramExecutionReceipt {
    let loop_program = project_gerbil_loop_case_driver_loop_program(receipt)
        .expect("vertical trace projects into LoopProgram");
    LoopProgramExecutionDriver::new(scheme_projected_runtime_executor())
        .with_event_mapper(SchemeProjectedLoopProgramEventMapper::from_vertical_trace(
            receipt,
        ))
        .with_max_steps(receipt.transition_count() + 2)
        .run(LoopProgramExecutionRequest::new(
            loop_program,
            vec![LoopProgramEventKind::Start],
        ))
}

fn artifact_content(
    bundle: &marlin_agent_harness::IntentCaseArtifactBundleMaterializationReceipt,
    kind: IntentCaseArtifactKind,
) -> String {
    let artifact = bundle
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == kind)
        .expect("artifact kind materialized");
    fs::read_to_string(&artifact.path).expect("read materialized artifact")
}

fn cap(tag: impl Into<String>) -> GerbilLoopCaseDriverCapability {
    GerbilLoopCaseDriverCapability::new(tag)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SchemeProjectedLoopProgramEventMapper {
    events: Box<[LoopProgramEventKind]>,
}

impl SchemeProjectedLoopProgramEventMapper {
    fn from_vertical_trace(receipt: &GerbilLoopCaseDriverVerticalTraceReceipt) -> Self {
        Self {
            events: receipt
                .transition_events()
                .map(|event| {
                    project_gerbil_loop_case_driver_loop_event_kind(event)
                        .expect("Scheme vertical event should project")
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }
}

impl LoopProgramEventMapper for SchemeProjectedLoopProgramEventMapper {
    fn next_event(
        &self,
        machine_receipt: &GenericLoopMachineReceipt,
        runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> Option<LoopProgramEventKind> {
        if runtime_handoff_execution.status
            == LoopProgramRuntimeHandoffExecutionReportStatus::Denied
        {
            return Some(LoopProgramEventKind::Error);
        }
        if runtime_handoff_execution.status
            != LoopProgramRuntimeHandoffExecutionReportStatus::Completed
        {
            return None;
        }

        self.events
            .get(machine_receipt.step_index.get() as usize)
            .cloned()
    }
}

fn scheme_projected_runtime_executor() -> HybridLoopProgramRuntimeHandoffExecutor {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        model_handler: handled_by("scheme.projected.model"),
        graph_handler: handled_by("scheme.projected.graph"),
        verification_handler: handled_by("scheme.projected.verification"),
        control_handler: handled_by("scheme.projected.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };

    HybridLoopProgramRuntimeHandoffExecutor::new(
        LoopProgramRuntimeHandoffRouter::new(handlers),
        AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
            "scheme.projected.agent-flow",
        )),
    )
}

fn handled_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
        LoopProgramRuntimeOwner::new(owner),
    ))
}

fn tool_shell_resolver(script: &'static str) -> StaticLoopProgramToolProcessResolver {
    StaticLoopProgramToolProcessResolver::new(
        vec![
            LoopProgramToolProcessCommandTemplate::new(
                "agent-flow.tool-intent",
                ["loop-program.dispatch-tools"],
                LoopProgramToolProcessProgram::new("sh"),
            )
            .with_args(["-c", script]),
        ]
        .into_boxed_slice(),
    )
}
