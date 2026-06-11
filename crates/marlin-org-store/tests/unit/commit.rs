use std::collections::BTreeMap;

use marlin_org_patch::{OrgPatchDiagnostic, OrgPatchPlan, OrgTextEdit};
use marlin_org_store::{
    MemoryOrgSourceStore, OrgSourceCommit, OrgSourceCommitter, OrgSourceDiagnosticKind,
    OrgSourceDocumentHash, OrgSourceWritePolicy,
};

#[test]
fn dry_run_validates_and_reports_without_writing() {
    let text = "* Goal\n";
    let mut store =
        MemoryOrgSourceStore::new(BTreeMap::from([("memory.org".to_owned(), text.to_owned())]));
    let mut commit = OrgSourceCommit::new(
        add_text_plan(7, "- [ ] next\n"),
        OrgSourceWritePolicy::dry_run(),
    );
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("memory.org", text));

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(receipt.accepted());
    assert_eq!(receipt.applied_edits, 1);
    assert_eq!(receipt.planned_edits.len(), 1);
    assert_eq!(receipt.changed_documents.len(), 1);
    assert!(!receipt.wrote_documents);
    assert_eq!(store.document("memory.org"), Some(text));
}

#[test]
fn write_commits_changed_documents() {
    let text = "* Goal\n";
    let mut store =
        MemoryOrgSourceStore::new(BTreeMap::from([("memory.org".to_owned(), text.to_owned())]));
    let mut commit = OrgSourceCommit::new(
        add_text_plan(7, "- [ ] next\n"),
        OrgSourceWritePolicy::write(),
    );
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("memory.org", text));

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(receipt.accepted());
    assert_eq!(receipt.applied_edits, 1);
    assert_eq!(receipt.planned_edits.len(), 1);
    assert!(receipt.wrote_documents);
    assert_eq!(store.document("memory.org"), Some("* Goal\n- [ ] next\n"));
}

#[test]
fn stale_hash_blocks_commit_without_writing() {
    let mut store = MemoryOrgSourceStore::new(BTreeMap::from([(
        "memory.org".to_owned(),
        "* Changed\n".to_owned(),
    )]));
    let mut commit = OrgSourceCommit::new(
        add_text_plan(7, "- [ ] next\n"),
        OrgSourceWritePolicy::write(),
    );
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("memory.org", "* Goal\n"));

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(!receipt.accepted());
    assert_eq!(receipt.conflicts.len(), 1);
    assert!(!receipt.wrote_documents);
    assert_eq!(store.document("memory.org"), Some("* Changed\n"));
}

#[test]
fn missing_expected_hash_blocks_commit() {
    let mut store = MemoryOrgSourceStore::new(BTreeMap::from([(
        "memory.org".to_owned(),
        "* Goal\n".to_owned(),
    )]));
    let commit = OrgSourceCommit::new(
        add_text_plan(7, "- [ ] next\n"),
        OrgSourceWritePolicy::write(),
    );

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(!receipt.accepted());
    assert_eq!(
        receipt.diagnostics[0].kind,
        OrgSourceDiagnosticKind::MissingExpectedHash
    );
    assert_eq!(store.document("memory.org"), Some("* Goal\n"));
}

#[test]
fn patch_plan_diagnostics_block_commit() {
    let mut store = MemoryOrgSourceStore::new(BTreeMap::from([(
        "memory.org".to_owned(),
        "* Goal\n".to_owned(),
    )]));
    let mut plan = add_text_plan(7, "- [ ] next\n");
    plan.diagnostics.push(OrgPatchDiagnostic {
        node: None,
        operation: "set-todo".to_owned(),
        message: "requires token span".to_owned(),
    });
    let mut commit = OrgSourceCommit::new(plan, OrgSourceWritePolicy::write());
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("memory.org", "* Goal\n"));

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(!receipt.accepted());
    assert_eq!(
        receipt.diagnostics[0].kind,
        OrgSourceDiagnosticKind::PatchDiagnostic
    );
    assert_eq!(store.document("memory.org"), Some("* Goal\n"));
}

#[test]
fn require_clean_blocks_dirty_store() {
    let text = "* Goal\n";
    let mut store =
        MemoryOrgSourceStore::new(BTreeMap::from([("memory.org".to_owned(), text.to_owned())]));
    store.set_clean(false);
    let mut commit = OrgSourceCommit::new(
        add_text_plan(7, "- [ ] next\n"),
        OrgSourceWritePolicy::write_require_clean(),
    );
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("memory.org", text));

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(!receipt.accepted());
    assert_eq!(
        receipt.diagnostics[0].kind,
        OrgSourceDiagnosticKind::DirtyStore
    );
    assert_eq!(store.document("memory.org"), Some(text));
}

#[test]
fn multi_document_commit_is_rejected_by_default_before_writing() {
    let first = "* One\n";
    let second = "* Two\n";
    let mut store = MemoryOrgSourceStore::new(BTreeMap::from([
        ("one.org".to_owned(), first.to_owned()),
        ("two.org".to_owned(), second.to_owned()),
    ]));
    let mut commit = OrgSourceCommit::new(two_document_plan(), OrgSourceWritePolicy::write());
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("one.org", first));
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("two.org", second));

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(!receipt.accepted());
    assert_eq!(receipt.planned_edits.len(), 2);
    assert_eq!(
        receipt.diagnostics[0].kind,
        OrgSourceDiagnosticKind::MultiDocumentWriteUnsupported
    );
    assert_eq!(store.document("one.org"), Some(first));
    assert_eq!(store.document("two.org"), Some(second));
}

fn add_text_plan(start_byte: usize, replacement: &str) -> OrgPatchPlan {
    OrgPatchPlan {
        edits: vec![OrgTextEdit {
            document: "memory.org".to_owned(),
            start_byte,
            end_byte: start_byte,
            replacement: replacement.to_owned(),
            reason: "test-append".to_owned(),
        }],
        diagnostics: Vec::new(),
    }
}

fn two_document_plan() -> OrgPatchPlan {
    OrgPatchPlan {
        edits: vec![
            OrgTextEdit {
                document: "one.org".to_owned(),
                start_byte: 6,
                end_byte: 6,
                replacement: "- [ ] one\n".to_owned(),
                reason: "test-append".to_owned(),
            },
            OrgTextEdit {
                document: "two.org".to_owned(),
                start_byte: 6,
                end_byte: 6,
                replacement: "- [ ] two\n".to_owned(),
                reason: "test-append".to_owned(),
            },
        ],
        diagnostics: Vec::new(),
    }
}
