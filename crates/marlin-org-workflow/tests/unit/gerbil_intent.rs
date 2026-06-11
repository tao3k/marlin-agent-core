use marlin_gerbil_ir::WorkspacePatchIntentSpec;
use marlin_org_model::OrgNodeId;
use marlin_org_workflow::GerbilWorkspacePatchIntentDryRunner;
use marlin_workspace_patch::{PatchId, ValidationSeverity, WorkspacePatch, WorkspacePatchOp};

#[test]
fn gerbil_patch_intent_dry_run_returns_receipt_without_workspace_write() {
    let intent = workspace_patch_intent(true);

    let receipt = GerbilWorkspacePatchIntentDryRunner::dry_run(&intent);

    assert_eq!(receipt.patch_id, PatchId::new("intent:memory"));
    assert_eq!(
        receipt.affected_nodes,
        [OrgNodeId::new("memory.org:1:goal")]
    );
    assert!(receipt.affected_sources.is_empty());
    assert_eq!(receipt.before_hash, "dry-run:no-workspace-read");
    assert_eq!(receipt.after_hash, "dry-run:no-workspace-write");
    assert!(receipt.validation.accepted);
    assert!(receipt.validation.diagnostics.is_empty());
    assert_eq!(receipt.memory_dispatch.len(), 1);
    assert_eq!(receipt.memory_dispatch[0].target, "long-term");
    assert!(!receipt.memory_dispatch[0].accepted);
    assert_eq!(
        receipt.memory_dispatch[0].reason.as_deref(),
        Some("dry-run only: memory dispatch not executed")
    );
}

#[test]
fn gerbil_patch_intent_dry_run_rejects_missing_dry_run_first() {
    let intent = workspace_patch_intent(false);

    let receipt = GerbilWorkspacePatchIntentDryRunner::dry_run(&intent);

    assert!(!receipt.validation.accepted);
    assert_eq!(receipt.validation.diagnostics.len(), 1);
    assert_eq!(
        receipt.validation.diagnostics[0].severity,
        ValidationSeverity::Error
    );
    assert!(
        receipt.validation.diagnostics[0]
            .message
            .contains("requires dry_run_first")
    );
    assert!(receipt.memory_dispatch.is_empty());
}

fn workspace_patch_intent(dry_run_first: bool) -> WorkspacePatchIntentSpec {
    let mut patch = WorkspacePatch::new("gerbil intent");
    patch.source_agent = Some("gerbil".to_owned());
    patch.ops.push(WorkspacePatchOp::MarkMemoryCandidate {
        node: OrgNodeId::new("memory.org:1:goal"),
        dispatch: "long-term".to_owned(),
    });

    WorkspacePatchIntentSpec {
        intent_id: "intent:memory".to_owned(),
        patch,
        dry_run_first,
    }
}
