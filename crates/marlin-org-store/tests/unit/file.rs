use std::{
    collections::BTreeMap,
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_org_model::{CheckboxState, OrgNodeId, OrgNodeSourceTokens, OrgSourceSpan};
use marlin_org_patch::{OrgPatchPlan, OrgPatchPlanner, OrgTextEdit};
use marlin_org_store::{
    FileSystemOrgSourceStore, OrgSourceCommit, OrgSourceCommitter, OrgSourceDiagnosticKind,
    OrgSourceDocumentHash, OrgSourceStore, OrgSourceWritePolicy,
};
use marlin_workspace_patch::{AffectedNodeSource, WorkspacePatch, WorkspacePatchOp};

#[test]
fn filesystem_store_commits_through_source_committer() {
    let root = test_root("commit");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(root.join("memory.org"), "* Goal\n").expect("seed document");

    let mut store = FileSystemOrgSourceStore::new(&root);
    let mut commit = OrgSourceCommit::new(
        add_text_plan("memory.org", 7, "- [ ] next\n"),
        OrgSourceWritePolicy::write(),
    );
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("memory.org", "* Goal\n"));

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(receipt.accepted());
    assert!(receipt.wrote_documents);
    assert_eq!(
        fs::read_to_string(root.join("memory.org")).expect("read committed document"),
        "* Goal\n- [ ] next\n"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn filesystem_store_rejects_root_escape_on_write() {
    let root = test_root("escape");
    fs::create_dir_all(&root).expect("create temp root");
    let mut store = FileSystemOrgSourceStore::new(&root);

    let result = store.write_documents(BTreeMap::from([(
        "../outside.org".to_owned(),
        "* Escape\n".to_owned(),
    )]));

    assert!(result.is_err());
    assert!(!root.join("../outside.org").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn filesystem_commit_missing_document_reports_diagnostic() {
    let root = test_root("missing");
    fs::create_dir_all(&root).expect("create temp root");
    let mut store = FileSystemOrgSourceStore::new(&root);
    let mut commit = OrgSourceCommit::new(
        add_text_plan("missing.org", 0, "* Added\n"),
        OrgSourceWritePolicy::write(),
    );
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("missing.org", ""));

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(!receipt.accepted());
    assert_eq!(
        receipt.diagnostics[0].kind,
        OrgSourceDiagnosticKind::MissingDocument
    );
    assert!(!root.join("missing.org").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn filesystem_store_commits_workspace_patch_through_org_planner() {
    let root = test_root("workspace-patch");
    fs::create_dir_all(&root).expect("create temp root");
    let original = "* TODO Goal\nbody\n";
    fs::write(root.join("memory.org"), original).expect("seed document");

    let node = OrgNodeId::new("goal:1");
    let mut patch = WorkspacePatch::new("persist next action");
    patch.ops.push(WorkspacePatchOp::AddCheckbox {
        node: node.clone(),
        text: "verify persisted".to_owned(),
        state: CheckboxState::Open,
    });
    let plan = OrgPatchPlanner::plan(
        &patch,
        &[AffectedNodeSource {
            node,
            source: OrgSourceSpan {
                document: "memory.org".to_owned(),
                start_byte: 0,
                end_byte: original.len(),
                start_line: 1,
                end_line: 2,
            },
            tokens: OrgNodeSourceTokens::default(),
        }],
    );
    assert!(plan.is_applicable());

    let mut commit = OrgSourceCommit::new(plan, OrgSourceWritePolicy::write());
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("memory.org", original));
    let mut store = FileSystemOrgSourceStore::new(&root);

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(receipt.accepted());
    assert_eq!(receipt.applied_edits, 1);
    assert!(receipt.wrote_documents);
    assert_eq!(
        fs::read_to_string(root.join("memory.org")).expect("read committed document"),
        format!("{original}\n- [ ] verify persisted\n")
    );
    let _ = fs::remove_dir_all(root);
}

fn add_text_plan(document: &str, start_byte: usize, replacement: &str) -> OrgPatchPlan {
    OrgPatchPlan {
        edits: vec![OrgTextEdit {
            document: document.to_owned(),
            start_byte,
            end_byte: start_byte,
            replacement: replacement.to_owned(),
            reason: "test-append".to_owned(),
        }],
        diagnostics: Vec::new(),
    }
}

fn test_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-org-store-{name}-{}-{suffix}",
        std::process::id()
    ))
}
