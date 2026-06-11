use marlin_org_model::{OrgNodeId, OrgNodeSourceTokens, OrgSourceSpan};
use marlin_workspace_patch::{
    AffectedNodeSource, MemoryDispatchReceipt, PatchId, WorkspacePatchReceipt,
    WorkspaceValidationReport,
};

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
    assert_eq!(receipt.memory_dispatch[0].target, "semantic-long-term");
}
