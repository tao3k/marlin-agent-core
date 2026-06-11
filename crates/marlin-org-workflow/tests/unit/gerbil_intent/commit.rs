use std::fs;

use marlin_org_store::{FileSystemOrgSourceStore, OrgSourceWritePolicy};
use marlin_org_workflow::{GerbilWorkspacePatchIntentCommit, OrgWorkspaceSourceCommitter};

use super::support::{failed_contract_facts, test_root, workspace_patch_source_intent};

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
fn gerbil_patch_intent_commit_rejects_failed_contract_facts_without_write() {
    let root = test_root("gerbil-intent-contract-reject");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(root.join("memory.org"), "* Goal\n").expect("seed document");
    let mut store = FileSystemOrgSourceStore::new(&root);
    let request = GerbilWorkspacePatchIntentCommit::new(
        "memory.org",
        workspace_patch_source_intent(true),
        OrgSourceWritePolicy::write(),
    )
    .with_contract_facts(failed_contract_facts());

    let receipt = OrgWorkspaceSourceCommitter::commit_gerbil_intent(&mut store, &request);

    assert!(receipt.loaded_nodes.is_empty());
    assert!(!receipt.source.accepted());
    assert_eq!(receipt.source.diagnostics.len(), 1);
    assert!(
        receipt.source.diagnostics[0]
            .message
            .contains("org contract assertion failed")
    );
    assert!(!receipt.source.wrote_documents);
    assert_eq!(
        fs::read_to_string(root.join("memory.org")).expect("read unchanged document"),
        "* Goal\n"
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
    let _ = fs::remove_dir_all(root);
}
