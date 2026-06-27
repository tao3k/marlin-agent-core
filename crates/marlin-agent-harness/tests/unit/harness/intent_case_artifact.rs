use std::{
    fs,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use marlin_agent_harness::{
    GerbilScriptedIntentCaseArtifactBundleRequest, IntentCaseArtifactKind,
    materialize_gerbil_scripted_intent_case_artifact_bundle,
};
use marlin_agent_harness_types::{
    RuntimeRepairCaseId, RuntimeRepairCaseReceipt, RuntimeRepairContentSummary, RuntimeRepairCount,
    RuntimeRepairDurationMillis, RuntimeRepairLiveCaseReceipt, RuntimeRepairLiveCaseReceiptRequest,
    RuntimeRepairModelCompletionId, RuntimeRepairModelId, RuntimeRepairProfileRef,
};
use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, GenericLoopMachineReceipt,
    HybridLoopProgramRuntimeHandoffExecutor, LoopProgramEventMapper, LoopProgramExecutionDriver,
    LoopProgramExecutionReceipt, LoopProgramExecutionRequest, LoopProgramFileSandbox,
    LoopProgramFileWriteTemplate, LoopProgramRuntimeHandoffExecutionReceipt,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffHandler,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramToolProcessCommandTemplate, LoopProgramToolProcessProgram,
    StaticLoopProgramFileWriteResolver, StaticLoopProgramRuntimeHandoffHandler,
    StaticLoopProgramToolProcessResolver,
};
use marlin_agent_protocol::LoopProgramEventKind;
use marlin_agent_runtime::TokioAgentRuntime;
use marlin_gerbil_scheme::{
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverVerticalTraceReceipt,
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
        assert!(manifest_receipt.contains("completeness_status=complete"));
        assert!(manifest_receipt.contains("correlation_key_count="));
        assert!(manifest_receipt.contains("correlation case_id="));
        assert!(manifest_receipt.contains("run_id=scripted-bundle-"));
        assert!(manifest_receipt.contains("policy_digest="));
        assert!(manifest_receipt.contains("loop_program_id="));
        assert!(manifest_receipt.contains("step_index=1"));
        assert!(manifest_receipt.contains("runtime_owner=marlin-agent-core"));

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

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "tool-side-effects".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
        },
    )
    .expect("tool side-effect bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    let tool_calls = artifact_content(&bundle, IntentCaseArtifactKind::ToolCalls);
    assert!(tool_calls.contains("side_effect_replay policy_status=Ready"));
    assert!(tool_calls.contains("status=Completed"));
    assert!(tool_calls.contains("stdout_digest=fnv1a64:"));
    assert!(tool_calls.contains("stdout_bytes=37"));
    assert!(!tool_calls.contains("harness-policy-combination-tool-spawn"));
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

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "sandbox-file-write".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: Some(replay_bundle),
            runtime_repair_receipt: None,
        },
    )
    .expect("sandbox file-write bundle materializes");

    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    let sandbox = artifact_content(&bundle, IntentCaseArtifactKind::SandboxReceipts);
    let patch = artifact_content(&bundle, IntentCaseArtifactKind::DiffPatch);
    assert!(sandbox.contains("side_effect_policy_status=Ready"));
    assert!(sandbox.contains("file_write step="));
    assert!(sandbox.contains("relative_path=src/lib.rs"));
    assert!(sandbox.contains("status=Completed"));
    assert!(sandbox.contains("after_hash=fnv1a64:"));
    assert!(patch.contains("bytes_written=26"));
    assert_eq!(
        fs::read_to_string(&bug_file).expect("read sandbox repaired fixture"),
        "fn answer() -> i32 { 41 }\n"
    );
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

    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: output_root.path().to_owned(),
            run_id: "runtime-repair-receipt".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: None,
            runtime_repair_receipt: Some(RuntimeRepairCaseReceipt::from(runtime_repair_receipt)),
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
