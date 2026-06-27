use marlin_agent_harness_types::{
    IntentCaseArtifactId, IntentCaseArtifactKind, IntentCaseArtifactManifest,
    IntentCaseArtifactManifestRequest, IntentCaseArtifactRef, IntentCaseId,
    IntentCaseLoopProgramId, IntentCasePolicyDigest, IntentCaseRunId, IntentCaseRuntimeOwner,
    IntentCaseTraceEntry, IntentCaseTraceEntryId, IntentCaseTraceEntryRequest,
    IntentCaseTraceIndex, IntentCaseTransitionId,
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

fn trace_entry() -> IntentCaseTraceEntry {
    IntentCaseTraceEntry::from_request(IntentCaseTraceEntryRequest {
        trace_id: IntentCaseTraceEntryId::new("case-a:trace-1"),
        step_index: 1,
        transition_id: IntentCaseTransitionId::new("program-a:transition-1"),
        action: "dispatch_tools".to_owned(),
        event: "tool_request".to_owned(),
    })
    .with_runtime_owner(IntentCaseRuntimeOwner::new("runtime-a"))
}
