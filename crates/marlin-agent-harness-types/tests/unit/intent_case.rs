use marlin_agent_harness_types::{
    INTENT_CASE_ARTIFACT_COMPLETENESS_RECEIPT_SCHEMA_ID, INTENT_CASE_RUN_RECEIPT_SCHEMA_ID,
    IntentCaseArtifactCompletenessReceipt, IntentCaseArtifactCompletenessStatus,
    IntentCaseArtifactId, IntentCaseArtifactKind, IntentCaseArtifactManifest,
    IntentCaseArtifactManifestRequest, IntentCaseArtifactRef, IntentCaseId,
    IntentCaseLoopProgramId, IntentCasePolicyDigest, IntentCaseRunId, IntentCaseRunReceipt,
    IntentCaseRunStatus, IntentCaseRuntimeOwner, IntentCaseSpanName, IntentCaseTraceEntry,
    IntentCaseTraceEntryId, IntentCaseTraceEntryRequest, IntentCaseTraceIndex,
    IntentCaseTransitionId,
};

#[test]
fn manifest_builds_complete_correlation_keys_from_trace_artifact_refs() {
    let artifact_id = IntentCaseArtifactId::new("case-a:vertical-trace");
    let manifest = base_manifest()
        .with_artifact(IntentCaseArtifactRef::present(
            artifact_id.clone(),
            IntentCaseArtifactKind::VerticalTrace,
            "artifacts/intent-cases/case-a/run-a/30-vertical-trace.receipt",
        ))
        .with_trace_index(IntentCaseTraceIndex::new([
            trace_entry().with_artifact_ref(artifact_id.clone())
        ]));

    let correlation_keys = manifest.correlation_keys();

    assert!(manifest.has_complete_trace_correlation());
    assert!(manifest.trace_artifact_ref_missing_ids().is_empty());
    assert!(manifest.trace_entries_without_runtime_owner().is_empty());
    assert_eq!(correlation_keys.len(), 1);
    assert_eq!(correlation_keys[0].case_id.as_str(), "case-a");
    assert_eq!(correlation_keys[0].run_id.as_str(), "run-a");
    assert_eq!(correlation_keys[0].policy_epoch, 7);
    assert_eq!(correlation_keys[0].policy_digest.as_str(), "digest-a");
    assert_eq!(correlation_keys[0].loop_program_id.as_str(), "program-a");
    assert_eq!(correlation_keys[0].step_index, 1);
    assert_eq!(
        correlation_keys[0].transition_id.as_str(),
        "program-a:transition-1"
    );
    assert_eq!(correlation_keys[0].action.as_str(), "dispatch_tools");
    assert_eq!(correlation_keys[0].event.as_str(), "tool_request");
    assert_eq!(correlation_keys[0].runtime_owner.as_str(), "runtime-a");
    assert!(correlation_keys[0].model_invocation_id.is_none());
    assert_eq!(
        correlation_keys[0]
            .tool_call_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("case-a:program-a:tool-call-1")
    );
    assert_eq!(
        correlation_keys[0]
            .resource_key
            .as_ref()
            .map(|id| id.as_str()),
        Some("agent-flow.tool-intent")
    );
    assert_eq!(
        correlation_keys[0]
            .sandbox_profile
            .as_ref()
            .map(|id| id.as_str()),
        Some("scripted-tool")
    );
    assert_eq!(correlation_keys[0].artifact_id, artifact_id);
}

#[test]
fn manifest_reports_broken_trace_correlation() {
    let missing_artifact_id = IntentCaseArtifactId::new("case-a:missing-artifact");
    let missing_ref_manifest = base_manifest().with_trace_index(IntentCaseTraceIndex::new([
        trace_entry().with_artifact_ref(missing_artifact_id.clone()),
    ]));
    assert!(!missing_ref_manifest.has_complete_trace_correlation());
    assert_eq!(
        missing_ref_manifest.trace_artifact_ref_missing_ids(),
        vec![missing_artifact_id]
    );

    let declared_missing_artifact_id = IntentCaseArtifactId::new("case-a:declared-missing");
    let declared_missing_manifest = base_manifest()
        .with_artifact(IntentCaseArtifactRef::missing(
            declared_missing_artifact_id.clone(),
            IntentCaseArtifactKind::VerifierReceipt,
        ))
        .with_trace_index(IntentCaseTraceIndex::new([
            trace_entry().with_artifact_ref(declared_missing_artifact_id.clone())
        ]));
    assert!(!declared_missing_manifest.has_complete_trace_correlation());
    assert_eq!(
        declared_missing_manifest.trace_artifact_ref_missing_ids(),
        vec![declared_missing_artifact_id]
    );

    let missing_owner_manifest = base_manifest().with_trace_index(IntentCaseTraceIndex::new([
        IntentCaseTraceEntry::from_request(IntentCaseTraceEntryRequest {
            trace_id: IntentCaseTraceEntryId::new("case-a:trace-2"),
            step_index: 2,
            transition_id: IntentCaseTransitionId::new("program-a:transition-2"),
            action: "verify".to_owned(),
            event: "verification_receipt".to_owned(),
        }),
    ]));
    assert!(!missing_owner_manifest.has_complete_trace_correlation());
    assert_eq!(
        missing_owner_manifest.trace_entries_without_runtime_owner()[0].as_str(),
        "case-a:trace-2"
    );

    let missing_action_identity_manifest =
        base_manifest().with_trace_index(IntentCaseTraceIndex::new([
            IntentCaseTraceEntry::from_request(IntentCaseTraceEntryRequest {
                trace_id: IntentCaseTraceEntryId::new("case-a:trace-3"),
                step_index: 3,
                transition_id: IntentCaseTransitionId::new("program-a:transition-3"),
                action: "invoke_model".to_owned(),
                event: "start".to_owned(),
            })
            .with_runtime_owner(IntentCaseRuntimeOwner::new("runtime-a")),
        ]));
    assert!(!missing_action_identity_manifest.has_complete_trace_correlation());
    assert_eq!(
        missing_action_identity_manifest.trace_entries_without_action_identity()[0].as_str(),
        "case-a:trace-3"
    );

    let missing_tool_resource_manifest =
        base_manifest().with_trace_index(IntentCaseTraceIndex::new([
            IntentCaseTraceEntry::from_request(IntentCaseTraceEntryRequest {
                trace_id: IntentCaseTraceEntryId::new("case-a:trace-4"),
                step_index: 4,
                transition_id: IntentCaseTransitionId::new("program-a:transition-4"),
                action: "dispatch_tools".to_owned(),
                event: "tool_request".to_owned(),
            })
            .with_runtime_owner(IntentCaseRuntimeOwner::new("runtime-a"))
            .with_tool_call_id("case-a:program-a:tool-call-4"),
        ]));
    assert!(!missing_tool_resource_manifest.has_complete_trace_correlation());
    assert_eq!(
        missing_tool_resource_manifest.trace_entries_without_action_identity()[0].as_str(),
        "case-a:trace-4"
    );

    let missing_tool_sandbox_manifest =
        base_manifest().with_trace_index(IntentCaseTraceIndex::new([
            IntentCaseTraceEntry::from_request(IntentCaseTraceEntryRequest {
                trace_id: IntentCaseTraceEntryId::new("case-a:trace-5"),
                step_index: 5,
                transition_id: IntentCaseTransitionId::new("program-a:transition-5"),
                action: "dispatch_tools".to_owned(),
                event: "tool_request".to_owned(),
            })
            .with_runtime_owner(IntentCaseRuntimeOwner::new("runtime-a"))
            .with_tool_call_id("case-a:program-a:tool-call-5")
            .with_resource_key("agent-flow.tool-intent"),
        ]));
    assert!(!missing_tool_sandbox_manifest.has_complete_trace_correlation());
    assert_eq!(
        missing_tool_sandbox_manifest.trace_entries_without_action_identity()[0].as_str(),
        "case-a:trace-5"
    );
}

#[test]
fn artifact_completeness_receipt_marks_complete_materialized_bundle() {
    let artifact_id = IntentCaseArtifactId::new("case-a:vertical-trace");
    let manifest = base_manifest()
        .with_artifact(IntentCaseArtifactRef::present(
            artifact_id.clone(),
            IntentCaseArtifactKind::VerticalTrace,
            "artifacts/intent-cases/case-a/run-a/30-vertical-trace.receipt",
        ))
        .with_trace_index(IntentCaseTraceIndex::new([
            trace_entry().with_artifact_ref(artifact_id)
        ]));

    let receipt = IntentCaseArtifactCompletenessReceipt::from_manifest_and_materialized_artifacts(
        &manifest,
        [IntentCaseArtifactKind::VerticalTrace],
    );

    assert!(receipt.is_supported_schema());
    assert_eq!(
        receipt.schema_id,
        INTENT_CASE_ARTIFACT_COMPLETENESS_RECEIPT_SCHEMA_ID
    );
    assert!(receipt.is_complete());
    assert_eq!(
        receipt.status,
        IntentCaseArtifactCompletenessStatus::Complete
    );
    assert_eq!(
        receipt.expected_artifacts,
        vec![IntentCaseArtifactKind::VerticalTrace]
    );
    assert_eq!(
        manifest.expected_artifact_kinds(),
        vec![IntentCaseArtifactKind::VerticalTrace]
    );
    assert_eq!(
        receipt.materialized_artifacts,
        vec![IntentCaseArtifactKind::VerticalTrace]
    );
    assert_eq!(receipt.missing_artifacts, Vec::new());
    assert_eq!(receipt.expected_spans, Vec::<IntentCaseSpanName>::new());
    assert_eq!(receipt.observed_spans, Vec::<IntentCaseSpanName>::new());
    assert_eq!(receipt.missing_spans, Vec::<IntentCaseSpanName>::new());
    assert_eq!(receipt.trace_entry_count, 1);
    assert_eq!(receipt.correlation_key_count, 1);
}

#[test]
fn artifact_completeness_receipt_reports_missing_expected_manifest_lanes() {
    let artifact_id = IntentCaseArtifactId::new("case-a:vertical-trace");
    let manifest = base_manifest()
        .with_artifact(IntentCaseArtifactRef::present(
            artifact_id.clone(),
            IntentCaseArtifactKind::VerticalTrace,
            "artifacts/intent-cases/case-a/run-a/30-vertical-trace.receipt",
        ))
        .with_expected_artifact_kind(IntentCaseArtifactKind::VerifierReceipt)
        .with_trace_index(IntentCaseTraceIndex::new([
            trace_entry().with_artifact_ref(artifact_id)
        ]));

    let receipt = IntentCaseArtifactCompletenessReceipt::from_manifest_and_materialized_artifacts(
        &manifest,
        [IntentCaseArtifactKind::VerticalTrace],
    );

    assert!(!receipt.is_complete());
    assert_eq!(
        receipt.status,
        IntentCaseArtifactCompletenessStatus::Incomplete
    );
    assert_eq!(
        receipt.expected_artifacts,
        vec![
            IntentCaseArtifactKind::VerticalTrace,
            IntentCaseArtifactKind::VerifierReceipt,
        ]
    );
    assert_eq!(
        receipt.materialized_artifacts,
        vec![IntentCaseArtifactKind::VerticalTrace]
    );
    assert_eq!(
        receipt.missing_artifacts,
        vec![IntentCaseArtifactKind::VerifierReceipt]
    );
    assert_eq!(receipt.trace_entry_count, 1);
    assert_eq!(receipt.correlation_key_count, 1);
}

#[test]
fn artifact_completeness_receipt_reports_missing_expected_spans() {
    let artifact_id = IntentCaseArtifactId::new("case-a:vertical-trace");
    let manifest = base_manifest()
        .with_artifact(IntentCaseArtifactRef::present(
            artifact_id.clone(),
            IntentCaseArtifactKind::VerticalTrace,
            "artifacts/intent-cases/case-a/run-a/30-vertical-trace.receipt",
        ))
        .with_expected_span_names(["harness.execution", "runtime.tool"])
        .with_observed_span_name("harness.execution")
        .with_trace_index(IntentCaseTraceIndex::new([
            trace_entry().with_artifact_ref(artifact_id)
        ]));

    let receipt = IntentCaseArtifactCompletenessReceipt::from_manifest_and_materialized_artifacts(
        &manifest,
        [IntentCaseArtifactKind::VerticalTrace],
    );

    assert!(!receipt.is_complete());
    assert_eq!(
        receipt.status,
        IntentCaseArtifactCompletenessStatus::Incomplete
    );
    assert_eq!(receipt.missing_artifacts, Vec::new());
    assert_eq!(
        receipt.expected_spans,
        vec![
            IntentCaseSpanName::new("harness.execution"),
            IntentCaseSpanName::new("runtime.tool"),
        ]
    );
    assert_eq!(
        receipt.observed_spans,
        vec![IntentCaseSpanName::new("harness.execution")]
    );
    assert_eq!(
        receipt.missing_spans,
        vec![IntentCaseSpanName::new("runtime.tool")]
    );
    assert_eq!(receipt.trace_entry_count, 1);
    assert_eq!(receipt.correlation_key_count, 1);
}

#[test]
fn artifact_completeness_receipt_reports_missing_materialized_lanes() {
    let artifact_id = IntentCaseArtifactId::new("case-a:vertical-trace");
    let manifest = base_manifest()
        .with_artifact(IntentCaseArtifactRef::present(
            artifact_id.clone(),
            IntentCaseArtifactKind::VerticalTrace,
            "artifacts/intent-cases/case-a/run-a/30-vertical-trace.receipt",
        ))
        .with_trace_index(IntentCaseTraceIndex::new([
            trace_entry().with_artifact_ref(artifact_id)
        ]));

    let receipt = IntentCaseArtifactCompletenessReceipt::from_manifest_and_materialized_artifacts(
        &manifest,
        [],
    );

    assert!(!receipt.is_complete());
    assert_eq!(
        receipt.status,
        IntentCaseArtifactCompletenessStatus::Incomplete
    );
    assert_eq!(
        receipt.missing_artifacts,
        vec![IntentCaseArtifactKind::VerticalTrace]
    );
    assert_eq!(receipt.trace_entry_count, 1);
    assert_eq!(receipt.correlation_key_count, 1);
}

#[test]
fn core_artifact_bundle_requires_run_receipt_lane() {
    let manifest_without_run_receipt = base_manifest()
        .with_artifact(core_artifact_ref(
            IntentCaseArtifactKind::PolicyPack,
            "10-policy-pack.receipt",
        ))
        .with_artifact(core_artifact_ref(
            IntentCaseArtifactKind::LoopProgram,
            "20-loop-program.receipt",
        ))
        .with_artifact(core_artifact_ref(
            IntentCaseArtifactKind::VerticalTrace,
            "30-vertical-trace.receipt",
        ))
        .with_artifact(core_artifact_ref(
            IntentCaseArtifactKind::ExecutionTrace,
            "40-execution-trace.receipt",
        ))
        .with_artifact(core_artifact_ref(
            IntentCaseArtifactKind::ReplayScript,
            "90-replay-script.ss",
        ));

    assert!(!manifest_without_run_receipt.has_core_artifact_bundle());

    let manifest_with_run_receipt = manifest_without_run_receipt.with_artifact(core_artifact_ref(
        IntentCaseArtifactKind::RunReceipt,
        "95-run-receipt.receipt",
    ));

    assert!(manifest_with_run_receipt.has_core_artifact_bundle());
}

#[test]
fn run_receipt_preserves_typed_manifest_boundary() {
    let receipt = IntentCaseRunReceipt::passed(base_manifest());

    assert!(receipt.is_supported_schema());
    assert_eq!(receipt.schema_id, INTENT_CASE_RUN_RECEIPT_SCHEMA_ID);
    assert_eq!(receipt.status, IntentCaseRunStatus::Passed);
    assert_eq!(receipt.manifest.case_id.as_str(), "case-a");
    assert!(receipt.diagnostics.is_empty());
}

fn base_manifest() -> IntentCaseArtifactManifest {
    IntentCaseArtifactManifest::from_request(IntentCaseArtifactManifestRequest {
        case_id: IntentCaseId::new("case-a"),
        run_id: IntentCaseRunId::new("run-a"),
        policy_epoch: 7,
        policy_digest: IntentCasePolicyDigest::new("digest-a"),
        loop_program_id: IntentCaseLoopProgramId::new("program-a"),
    })
}

fn core_artifact_ref(kind: IntentCaseArtifactKind, filename: &str) -> IntentCaseArtifactRef {
    IntentCaseArtifactRef::present(
        IntentCaseArtifactId::new(format!("case-a:{kind:?}")),
        kind,
        format!("artifacts/intent-cases/case-a/run-a/{filename}"),
    )
}

fn trace_entry() -> IntentCaseTraceEntry {
    IntentCaseTraceEntry::from_request(IntentCaseTraceEntryRequest {
        trace_id: IntentCaseTraceEntryId::new("case-a:trace-1"),
        step_index: 1,
        transition_id: IntentCaseTransitionId::new("program-a:transition-1"),
        action: "dispatch_tools".to_owned(),
        event: "tool_request".to_owned(),
    })
    .with_runtime_owner(IntentCaseRuntimeOwner::new("runtime-a"))
    .with_tool_call_id("case-a:program-a:tool-call-1")
    .with_resource_key("agent-flow.tool-intent")
    .with_sandbox_profile("scripted-tool")
}
