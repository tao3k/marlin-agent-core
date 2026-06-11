use marlin_org_model::{OrgNodeId, OrgNodeSourceTokens, OrgSourceSpan};
use marlin_workspace_patch::{
    AffectedNodeSource, MemoryDispatchReceipt, PatchId, WorkspacePatchExecutionMode,
    WorkspacePatchExecutionReceipt, WorkspacePatchReceipt, WorkspaceValidationReport,
};

#[test]
fn patch_id_exposes_stable_str_view() {
    let patch_id = PatchId::new("patch:gerbil");

    assert_eq!(patch_id.as_str(), "patch:gerbil");
}

#[test]
fn patch_receipt_records_hashes_validation_and_dispatch() {
    let receipt = WorkspacePatchReceipt {
        patch_id: PatchId::new("patch:1"),
        affected_nodes: vec![OrgNodeId::from("goal:workspace")],
        affected_sources: vec![AffectedNodeSource {
            node: OrgNodeId::from("goal:workspace"),
            source: OrgSourceSpan {
                document: "doc:workspace".to_string(),
                start_byte: 0,
                end_byte: 12,
                start_line: 1,
                end_line: 1,
            },
            tokens: OrgNodeSourceTokens::default(),
        }],
        before_hash: "before".to_string(),
        after_hash: "after".to_string(),
        validation: WorkspaceValidationReport::accepted(),
        execution: WorkspacePatchExecutionReceipt::commit_accepted(
            "write policy allowed durable workspace update",
        ),
        memory_dispatch: vec![MemoryDispatchReceipt {
            target: "semantic-long-term".to_string(),
            accepted: true,
            reason: None,
        }],
    };

    assert_ne!(receipt.before_hash, receipt.after_hash);
    assert!(receipt.validation.accepted);
    assert_eq!(receipt.affected_nodes[0].as_str(), "goal:workspace");
    assert_eq!(receipt.affected_sources[0].source.document, "doc:workspace");
    assert_eq!(receipt.execution.mode, WorkspacePatchExecutionMode::Commit);
    assert!(receipt.execution.policy.accepted);
    assert_eq!(
        receipt.execution.policy.reason.as_deref(),
        Some("write policy allowed durable workspace update")
    );
    assert_eq!(receipt.memory_dispatch[0].target, "semantic-long-term");
}

#[test]
fn patch_execution_receipt_defaults_to_rejected_dry_run_boundary() {
    let execution = WorkspacePatchExecutionReceipt::default();

    assert_eq!(execution.mode, WorkspacePatchExecutionMode::DryRun);
    assert!(!execution.policy.accepted);
    assert_eq!(
        execution.policy.reason.as_deref(),
        Some("execution metadata absent")
    );
}
