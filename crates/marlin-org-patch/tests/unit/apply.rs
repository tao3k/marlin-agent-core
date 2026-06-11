use std::collections::BTreeMap;

use marlin_org_patch::{OrgPatchApplier, OrgPatchDiagnostic, OrgPatchPlan, OrgTextEdit};

#[test]
fn applies_plan_to_in_memory_documents() {
    let mut documents = BTreeMap::from([("memory.org".to_owned(), "* Goal\nbody\n".to_owned())]);
    let plan = OrgPatchPlan {
        edits: vec![OrgTextEdit {
            document: "memory.org".to_owned(),
            start_byte: 12,
            end_byte: 12,
            replacement: "- [ ] next\n".to_owned(),
            reason: "add-checkbox".to_owned(),
        }],
        diagnostics: Vec::new(),
    };

    let report = OrgPatchApplier::apply_to_documents(&plan, &mut documents);

    assert_eq!(report.applied_edits, 1);
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.changed_documents.len(), 1);
    assert_eq!(report.changed_documents[0].document, "memory.org");
    assert_ne!(
        report.changed_documents[0].before_hash,
        report.changed_documents[0].after_hash
    );
    assert_eq!(documents["memory.org"], "* Goal\nbody\n- [ ] next\n");
}

#[test]
fn refuses_to_apply_plan_with_existing_diagnostics() {
    let mut documents = BTreeMap::from([("memory.org".to_owned(), "* Goal\n".to_owned())]);
    let plan = OrgPatchPlan {
        edits: vec![OrgTextEdit {
            document: "memory.org".to_owned(),
            start_byte: 7,
            end_byte: 7,
            replacement: "- [ ] next\n".to_owned(),
            reason: "add-checkbox".to_owned(),
        }],
        diagnostics: vec![OrgPatchDiagnostic {
            node: None,
            operation: "set-todo".to_owned(),
            message: "requires token span".to_owned(),
        }],
    };

    let report = OrgPatchApplier::apply_to_documents(&plan, &mut documents);

    assert_eq!(report.applied_edits, 0);
    assert!(report.changed_documents.is_empty());
    assert_eq!(report.diagnostics[0].operation, "set-todo");
    assert_eq!(documents["memory.org"], "* Goal\n");
}

#[test]
fn rejects_invalid_or_overlapping_ranges_before_applying() {
    let mut documents = BTreeMap::from([("memory.org".to_owned(), "* Goal\n".to_owned())]);
    let plan = OrgPatchPlan {
        edits: vec![
            OrgTextEdit {
                document: "memory.org".to_owned(),
                start_byte: 2,
                end_byte: 5,
                replacement: "One".to_owned(),
                reason: "first".to_owned(),
            },
            OrgTextEdit {
                document: "memory.org".to_owned(),
                start_byte: 4,
                end_byte: 6,
                replacement: "Two".to_owned(),
                reason: "second".to_owned(),
            },
        ],
        diagnostics: Vec::new(),
    };

    let report = OrgPatchApplier::apply_to_documents(&plan, &mut documents);

    assert_eq!(report.applied_edits, 0);
    assert!(report.changed_documents.is_empty());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].operation, "apply-plan");
    assert_eq!(documents["memory.org"], "* Goal\n");
}

#[test]
fn rejects_missing_documents() {
    let mut documents = BTreeMap::new();
    let plan = OrgPatchPlan {
        edits: vec![OrgTextEdit {
            document: "missing.org".to_owned(),
            start_byte: 0,
            end_byte: 0,
            replacement: "* Added\n".to_owned(),
            reason: "add-heading".to_owned(),
        }],
        diagnostics: Vec::new(),
    };

    let report = OrgPatchApplier::apply_to_documents(&plan, &mut documents);

    assert_eq!(report.applied_edits, 0);
    assert_eq!(
        report.diagnostics[0].message,
        "missing document: missing.org"
    );
}
