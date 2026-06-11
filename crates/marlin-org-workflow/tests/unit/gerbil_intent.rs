use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_gerbil_ir::WorkspacePatchIntentSpec;
use marlin_org_model::{CheckboxState, OrgNodeId};
use marlin_org_store::{FileSystemOrgSourceStore, OrgSourceWritePolicy};
use marlin_org_workflow::{
    GerbilWorkspacePatchIntentCommit, GerbilWorkspacePatchIntentDryRunner,
    OrgWorkspaceSourceCommitter, gerbil_workspace_patch_receipt_evidence,
};
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
    assert_eq!(
        receipt.execution.mode,
        marlin_workspace_patch::WorkspacePatchExecutionMode::DryRun
    );
    assert!(receipt.execution.policy.accepted);
    assert!(receipt.validation.diagnostics.is_empty());
    assert_eq!(receipt.memory_dispatch.len(), 1);
    assert_eq!(receipt.memory_dispatch[0].target, "long-term");
    assert!(!receipt.memory_dispatch[0].accepted);
    assert_eq!(
        receipt.memory_dispatch[0].reason.as_deref(),
        Some("dry-run only: memory dispatch not executed")
    );
    let evidence = gerbil_workspace_patch_receipt_evidence(&receipt);
    assert_eq!(
        evidence.kind,
        marlin_agent_protocol::LoopEvidenceKind::Workflow
    );
    assert_eq!(evidence.subject, "workspace-patch:intent:memory");
    assert!(evidence.present);
    assert_eq!(
        evidence.detail.as_deref(),
        Some(
            "accepted=true mode=DryRun policy_accepted=true affected_nodes=1 affected_sources=0 memory_dispatch=1 diagnostics=0"
        )
    );
}

#[test]
fn gerbil_patch_intent_dry_run_rejects_missing_dry_run_first() {
    let intent = workspace_patch_intent(false);

    let receipt = GerbilWorkspacePatchIntentDryRunner::dry_run(&intent);

    assert!(!receipt.validation.accepted);
    assert_eq!(
        receipt.execution.mode,
        marlin_workspace_patch::WorkspacePatchExecutionMode::DryRun
    );
    assert!(!receipt.execution.policy.accepted);
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
    let evidence = gerbil_workspace_patch_receipt_evidence(&receipt);
    assert_eq!(
        evidence.kind,
        marlin_agent_protocol::LoopEvidenceKind::Workflow
    );
    assert_eq!(evidence.subject, "workspace-patch:intent:memory");
    assert!(!evidence.present);
    assert_eq!(
        evidence.detail.as_deref(),
        Some(
            "accepted=false mode=DryRun policy_accepted=false affected_nodes=1 affected_sources=0 memory_dispatch=0 diagnostics=1"
        )
    );
}

#[test]
fn gerbil_patch_intent_commit_writes_when_policy_allows() {
    let root = test_root("gerbil-intent-commit");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(root.join("memory.org"), "* Goal\n").expect("seed document");
    let mut store = FileSystemOrgSourceStore::new(&root);
    let request = GerbilWorkspacePatchIntentCommit::new(
        "memory.org",
        workspace_patch_source_intent(true),
        OrgSourceWritePolicy::write(),
    );

    let receipt = OrgWorkspaceSourceCommitter::commit_gerbil_intent(&mut store, &request);

    assert!(receipt.source.accepted());
    assert_eq!(receipt.plan.edits.len(), 1);
    assert_eq!(receipt.source.applied_edits, 1);
    assert!(receipt.source.wrote_documents);
    assert_eq!(
        fs::read_to_string(root.join("memory.org")).expect("read committed document"),
        "* Goal\n\n- [ ] verify via gerbil\n"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn gerbil_patch_intent_commit_rejects_missing_dry_run_first_without_write() {
    let root = test_root("gerbil-intent-reject");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(root.join("memory.org"), "* Goal\n").expect("seed document");
    let mut store = FileSystemOrgSourceStore::new(&root);
    let request = GerbilWorkspacePatchIntentCommit::new(
        "memory.org",
        workspace_patch_source_intent(false),
        OrgSourceWritePolicy::write(),
    );

    let receipt = OrgWorkspaceSourceCommitter::commit_gerbil_intent(&mut store, &request);

    assert!(receipt.loaded_nodes.is_empty());
    assert!(!receipt.source.accepted());
    assert_eq!(receipt.source.diagnostics.len(), 1);
    assert!(
        receipt.source.diagnostics[0]
            .message
            .contains("requires dry_run_first")
    );
    assert!(!receipt.source.wrote_documents);
    assert_eq!(
        fs::read_to_string(root.join("memory.org")).expect("read unchanged document"),
        "* Goal\n"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn gerbil_patch_intent_commit_dry_run_policy_plans_without_write() {
    let root = test_root("gerbil-intent-dry-run-policy");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(root.join("memory.org"), "* Goal\n").expect("seed document");
    let mut store = FileSystemOrgSourceStore::new(&root);
    let request = GerbilWorkspacePatchIntentCommit::new(
        "memory.org",
        workspace_patch_source_intent(true),
        OrgSourceWritePolicy::dry_run(),
    );

    let receipt = OrgWorkspaceSourceCommitter::commit_gerbil_intent(&mut store, &request);

    assert!(receipt.source.accepted());
    assert_eq!(receipt.plan.edits.len(), 1);
    assert_eq!(receipt.source.applied_edits, 1);
    assert!(!receipt.source.wrote_documents);
    assert_eq!(
        fs::read_to_string(root.join("memory.org")).expect("read unchanged document"),
        "* Goal\n"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn gerbil_patch_intent_commit_rejects_missing_document_without_write() {
    let root = test_root("gerbil-intent-missing-document");
    fs::create_dir_all(&root).expect("create temp root");
    let mut store = FileSystemOrgSourceStore::new(&root);
    let request = GerbilWorkspacePatchIntentCommit::new(
        "missing.org",
        workspace_patch_source_intent(true),
        OrgSourceWritePolicy::write(),
    );

    let receipt = OrgWorkspaceSourceCommitter::commit_gerbil_intent(&mut store, &request);

    assert!(receipt.loaded_nodes.is_empty());
    assert!(!receipt.source.accepted());
    assert_eq!(receipt.source.diagnostics.len(), 1);
    assert!(
        receipt.source.diagnostics[0]
            .message
            .contains("missing document")
    );
    assert!(receipt.plan.edits.is_empty());
    assert!(!receipt.source.wrote_documents);
    assert!(!root.join("missing.org").exists());
    let _ = fs::remove_dir_all(root);
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

fn workspace_patch_source_intent(dry_run_first: bool) -> WorkspacePatchIntentSpec {
    let mut patch = WorkspacePatch::new("gerbil source edit");
    patch.source_agent = Some("gerbil".to_owned());
    patch.ops.push(WorkspacePatchOp::AddCheckbox {
        node: OrgNodeId::new("memory.org:1:goal"),
        text: "verify via gerbil".to_owned(),
        state: CheckboxState::Open,
    });

    WorkspacePatchIntentSpec {
        intent_id: "intent:source-edit".to_owned(),
        patch,
        dry_run_first,
    }
}

fn test_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-org-workflow-{name}-{}-{suffix}",
        std::process::id()
    ))
}
