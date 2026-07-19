//! Exercises the live single-file repair scenario against the parent test harness.

use super::{
    Arc, BUGGY_FIXTURE, FIXED_FIXTURE, GerbilScriptedIntentCaseArtifactBundleRequest, Instant,
    IntentCaseArtifactKind, LiteLlmStreamGateway, LiveRepairDecisionMapper, LoopProgramActionKind,
    LoopProgramEventKind, LoopProgramExecutionDriver, LoopProgramExecutionRequest,
    LoopProgramExecutionStatus, LoopProgramRuntimeHandoffExecutionReportStatus, ModelEndpoint,
    ModelGatewayMessageRole, PATCH_INTENT, PATCH_REPLACE, RuntimeEdgeModelGateway,
    RuntimeEdgePolicy, RuntimeLiveRepairGateStatus, RuntimeRepairCaseId, RuntimeRepairCaseReceipt,
    RuntimeRepairContentSummary, RuntimeRepairCount, RuntimeRepairDurationMillis,
    RuntimeRepairHybridHandoffExecutor, RuntimeRepairLiveCaseReceipt,
    RuntimeRepairLiveCaseReceiptRequest, RuntimeRepairModelCompletionId, RuntimeRepairModelId,
    RuntimeRepairProfileRef, RuntimeRepairToolPhase, SCHEME_REAL_REPAIR_CASE_ID,
    SCHEME_REAL_REPAIR_PROFILE_REF, SCHEME_REAL_REPAIR_PROGRAM_ID, fs,
    intent_case_artifact_content, live_llm_timeout,
    materialize_gerbil_scripted_intent_case_artifact_bundle, runtime_live_repair_gate_receipt,
    scheme_projected_real_repair_loop_case, unique_temp_repair_workspace,
};

#[test]
#[ignore = "requires MARLIN_LIVE_LLM_GATE=1 and live LiteLLM provider credentials"]
fn live_runtime_repair_single_file_bug_fix_runs_llm_tool_and_verifier_loop() {
    let gate_receipt = runtime_live_repair_gate_receipt();
    if gate_receipt.status == RuntimeLiveRepairGateStatus::Disabled {
        eprintln!("skipping live repair loop: {gate_receipt:?}");
        return;
    }
    assert_eq!(
        gate_receipt.status,
        RuntimeLiveRepairGateStatus::Enabled,
        "live repair gate configured but not ready: {gate_receipt:?}"
    );

    let provider = gate_receipt
        .provider
        .clone()
        .expect("enabled gate receipt must include provider");
    let model = gate_receipt
        .model
        .clone()
        .expect("enabled gate receipt must include model");
    let repair_workspace = unique_temp_repair_workspace();
    fs::create_dir_all(&repair_workspace).expect("create repair workspace");
    let bug_file = repair_workspace.join("lib.rs");
    let test_binary = repair_workspace.join("lib-tests");
    fs::write(&bug_file, BUGGY_FIXTURE).expect("write bug fixture");
    assert_eq!(
        fs::read_to_string(&bug_file).expect("read bug fixture"),
        BUGGY_FIXTURE
    );

    let edge_gateway = RuntimeEdgeModelGateway::new(
        LiteLlmStreamGateway::new(),
        RuntimeEdgePolicy::new()
            .with_concurrency_limit(1)
            .with_timeout_ms(live_llm_timeout().as_millis() as u64),
    )
    .expect("runtime edge model gateway");
    let mapper = LiveRepairDecisionMapper::new(
        Arc::new(edge_gateway),
        ModelEndpoint::new(provider, model),
        bug_file.clone(),
        test_binary.clone(),
    )
    .expect("live repair decision mapper");
    let completion_receipts = mapper.completion_receipts();
    let tool_receipts = mapper.tool_receipts();
    let verification_receipts = mapper.verification_receipts();
    let driver = LoopProgramExecutionDriver::new(RuntimeRepairHybridHandoffExecutor::new())
        .with_event_mapper(mapper);

    let scheme_case = scheme_projected_real_repair_loop_case();
    let scheme_receipt = scheme_case.receipt();
    let loop_program = scheme_case.loop_program().clone();
    let started_at = Instant::now();
    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Continue,
            LoopProgramActionKind::RewriteGraph,
            LoopProgramActionKind::Verify,
            LoopProgramActionKind::Stop,
        ]
    );
    let model_completion = completion_receipts
        .lock()
        .expect("completion receipts")
        .first()
        .cloned()
        .expect("live model completion receipt");
    assert_eq!(
        model_completion.choices[0].message.role,
        ModelGatewayMessageRole::Assistant
    );
    assert!(
        model_completion.choices[0]
            .message
            .content
            .to_ascii_lowercase()
            .contains(&PATCH_INTENT.to_ascii_lowercase()),
        "live repair model did not return the typed patch intent: {:?}",
        model_completion.choices[0].message.content
    );
    assert!(
        model_completion.choices[0]
            .message
            .content
            .contains(PATCH_REPLACE),
        "live repair model did not identify the concrete replacement: {:?}",
        model_completion.choices[0].message.content
    );

    let tool_steps = execution_receipt
        .steps
        .iter()
        .filter(|step| step.machine_receipt.action == LoopProgramActionKind::DispatchTools)
        .collect::<Vec<_>>();
    assert_eq!(tool_steps.len(), 1);
    assert!(tool_steps.iter().all(|step| {
        step.runtime_handoff_execution.status
            == LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    }));
    assert!(tool_steps.iter().all(|step| {
        !step
            .runtime_handoff_execution
            .tool_process_projections
            .is_empty()
    }));

    let repaired_content = fs::read_to_string(&bug_file).expect("read repaired fixture");
    let tool_receipts = tool_receipts.lock().expect("tool receipts").clone();
    assert_eq!(tool_receipts.len(), 1);
    assert_eq!(tool_receipts[0].phase, RuntimeRepairToolPhase::ApplyPatch);
    assert!(tool_receipts[0].success);
    let verification_receipts = verification_receipts
        .lock()
        .expect("verification receipts")
        .clone();
    assert_eq!(verification_receipts.len(), 1);
    assert!(verification_receipts[0].success);
    assert_eq!(verification_receipts[0].repaired_source, FIXED_FIXTURE);

    let live_receipt = RuntimeRepairLiveCaseReceipt::new(RuntimeRepairLiveCaseReceiptRequest {
        case_id: RuntimeRepairCaseId::new(scheme_receipt.case_id().as_str()),
        profile_ref: RuntimeRepairProfileRef::new(scheme_receipt.profile_ref().as_str()),
        program_id: execution_receipt.program_id.clone(),
        model_completion_id: RuntimeRepairModelCompletionId::new(model_completion.id),
        model: RuntimeRepairModelId::new(model_completion.model),
        elapsed_ms: RuntimeRepairDurationMillis::new(started_at.elapsed().as_millis() as u64),
        action_count: RuntimeRepairCount::new(execution_receipt.steps.len()),
        tool_projection_count: RuntimeRepairCount::new(
            tool_steps
                .iter()
                .map(|step| {
                    step.runtime_handoff_execution
                        .tool_process_projections
                        .len()
                })
                .sum(),
        ),
        patch_tool_success: tool_receipts[0].success,
        graph_rewrite_projected: execution_receipt
            .steps
            .iter()
            .any(|step| step.machine_receipt.action == LoopProgramActionKind::RewriteGraph),
        verification_success: verification_receipts[0].success,
        repaired_content: RuntimeRepairContentSummary::from_text(&repaired_content),
    });

    assert_eq!(live_receipt.case_id.as_str(), SCHEME_REAL_REPAIR_CASE_ID);
    assert_eq!(
        live_receipt.profile_ref.as_str(),
        SCHEME_REAL_REPAIR_PROFILE_REF
    );
    assert_eq!(
        live_receipt.program_id.as_str(),
        SCHEME_REAL_REPAIR_PROGRAM_ID
    );
    assert!(!live_receipt.model_completion_id.as_str().trim().is_empty());
    assert!(!live_receipt.model.as_str().trim().is_empty());
    assert_eq!(live_receipt.action_count.get(), 6);
    assert_eq!(live_receipt.tool_projection_count.get(), 1);
    assert!(live_receipt.patch_tool_success);
    assert!(live_receipt.graph_rewrite_projected);
    assert!(live_receipt.verification_success);
    assert_eq!(
        live_receipt.repaired_content,
        RuntimeRepairContentSummary::from_text(FIXED_FIXTURE)
    );
    eprintln!(
        "live runtime repair receipt: case={} profile={} model={} elapsed_ms={} actions={} tool_projections={} patch_success={} rewrite_projected={} verify_success={}",
        live_receipt.case_id,
        live_receipt.profile_ref,
        live_receipt.model,
        live_receipt.elapsed_ms.get(),
        live_receipt.action_count.get(),
        live_receipt.tool_projection_count.get(),
        live_receipt.patch_tool_success,
        live_receipt.graph_rewrite_projected,
        live_receipt.verification_success
    );

    let artifact_root =
        tempfile::tempdir().expect("create live runtime repair artifact bundle tempdir");
    let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: artifact_root.path().to_owned(),
            run_id: "live-runtime-repair-receipt".into(),
            vertical_trace: scheme_case.receipt().clone(),
            execution_receipt,
            side_effect_replay_bundle: None,
            runtime_repair_receipt: Some(RuntimeRepairCaseReceipt::from(live_receipt)),
            real_llm_case_receipt: None,
            observed_span_source: None,
        },
    )
    .expect("live runtime repair receipt materializes into intent-case artifact bundle");

    assert!(bundle.manifest_path.is_file());
    assert!(bundle.completeness_receipt.is_complete());
    assert_eq!(bundle.completeness_receipt.missing_artifacts, Vec::new());
    assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::VerifierReceipt));
    assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::ModelEvents));
    assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::ToolCalls));
    let verifier = intent_case_artifact_content(&bundle, IntentCaseArtifactKind::VerifierReceipt);
    assert!(verifier.contains("runtime_repair_receipt=present"));
    assert!(verifier.contains("runtime_repair_kind=live"));
    assert!(verifier.contains("runtime_repair_schema=marlin.runtime-repair.live-case-receipt.v1"));
    assert!(verifier.contains("runtime_repair_repaired_content_digest=fnv1a64:"));
    assert!(verifier.contains(&format!(
        "runtime_repair_repaired_content_bytes={}",
        FIXED_FIXTURE.len()
    )));
    assert!(
        !verifier.contains(FIXED_FIXTURE.trim()),
        "live verifier artifact should keep repaired source as digest metadata, not raw source"
    );
    fs::remove_dir_all(&repair_workspace).expect("remove repair workspace");
}
