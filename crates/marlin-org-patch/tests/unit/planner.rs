use marlin_org_model::{
    CheckboxState, LinkKind, OrgLink, OrgNodeId, OrgNodeSourceTokens, OrgSourceSpan, TodoState,
};
use marlin_org_patch::OrgPatchPlanner;
use marlin_workspace_patch::{
    AffectedNodeSource, EvidenceRef, EvidenceTrust, WorkspacePatch, WorkspacePatchOp,
};

#[test]
fn plans_checkbox_insertion_from_source_span() {
    let node = OrgNodeId::new("n1");
    let mut patch = WorkspacePatch::new("add next action");
    patch.ops.push(WorkspacePatchOp::AddCheckbox {
        node: node.clone(),
        text: "ship planner".to_owned(),
        state: CheckboxState::Open,
    });

    let plan = OrgPatchPlanner::plan(&patch, &[source_for(node)]);

    assert!(plan.is_applicable());
    assert_eq!(plan.edits.len(), 1);
    assert_eq!(plan.edits[0].document, "memory.org");
    assert_eq!(plan.edits[0].start_byte, 48);
    assert_eq!(plan.edits[0].end_byte, 48);
    assert_eq!(plan.edits[0].replacement, "\n- [ ] ship planner\n");
}

#[test]
fn plans_link_and_evidence_insertions() {
    let node = OrgNodeId::new("n1");
    let mut patch = WorkspacePatch::new("attach evidence");
    patch.ops.push(WorkspacePatchOp::AddLink {
        node: node.clone(),
        link: OrgLink {
            kind: LinkKind::File,
            target: "docs/20-workspace/20.50-org-patch-planning.org".to_owned(),
            description: Some("patch plan".to_owned()),
        },
    });
    patch.ops.push(WorkspacePatchOp::AddEvidenceRef {
        node: node.clone(),
        evidence: EvidenceRef {
            target: "id:decision-1".to_owned(),
            summary: "planner keeps source writes explicit".to_owned(),
            trust: EvidenceTrust::Verified,
        },
    });

    let plan = OrgPatchPlanner::plan(&patch, &[source_for(node)]);

    assert_eq!(plan.edits.len(), 2);
    assert_eq!(
        plan.edits[0].replacement,
        "\n[[file:docs/20-workspace/20.50-org-patch-planning.org][patch plan]]\n"
    );
    assert_eq!(
        plan.edits[1].replacement,
        "\n- evidence: planner keeps source writes explicit :: id:decision-1 [verified]\n"
    );
}

#[test]
fn reports_missing_source_span() {
    let mut patch = WorkspacePatch::new("add next action");
    patch.ops.push(WorkspacePatchOp::AddCheckbox {
        node: OrgNodeId::new("missing"),
        text: "blocked without source".to_owned(),
        state: CheckboxState::Checked,
    });

    let plan = OrgPatchPlanner::plan(&patch, &[]);

    assert!(plan.edits.is_empty());
    assert_eq!(plan.diagnostics.len(), 1);
    assert_eq!(plan.diagnostics[0].operation, "add-checkbox");
    assert_eq!(
        plan.diagnostics[0].message,
        "missing parser-owned source span for node"
    );
}

#[test]
fn diagnoses_operations_that_need_token_spans() {
    let node = OrgNodeId::new("n1");
    let mut patch = WorkspacePatch::new("close task");
    patch.ops.push(WorkspacePatchOp::SetTodo {
        node: node.clone(),
        state: TodoState::Done,
    });

    let plan = OrgPatchPlanner::plan(&patch, &[source_for(node)]);

    assert!(plan.edits.is_empty());
    assert_eq!(plan.diagnostics.len(), 1);
    assert_eq!(plan.diagnostics[0].operation, "set-todo");
    assert_eq!(
        plan.diagnostics[0].message,
        "requires a headline TODO keyword token span"
    );
}

#[test]
fn plans_token_span_replacements() {
    let node = OrgNodeId::new("n1");
    let mut patch = WorkspacePatch::new("update structured tokens");
    patch.ops.push(WorkspacePatchOp::SetTodo {
        node: node.clone(),
        state: TodoState::Done,
    });
    patch.ops.push(WorkspacePatchOp::SetProperty {
        node: node.clone(),
        key: "OWNER".to_owned(),
        value: "marlin-org-patch".to_owned(),
    });
    patch.ops.push(WorkspacePatchOp::MarkCheckbox {
        node: node.clone(),
        index: 0,
        state: CheckboxState::Checked,
    });

    let plan = OrgPatchPlanner::plan(&patch, &[source_with_tokens(node)]);

    assert!(plan.is_applicable());
    assert_eq!(plan.edits.len(), 3);
    assert_eq!(plan.edits[0].start_byte, 2);
    assert_eq!(plan.edits[0].end_byte, 6);
    assert_eq!(plan.edits[0].replacement, "DONE");
    assert_eq!(plan.edits[1].start_byte, 18);
    assert_eq!(plan.edits[1].end_byte, 29);
    assert_eq!(plan.edits[1].replacement, "marlin-org-patch");
    assert_eq!(plan.edits[2].start_byte, 42);
    assert_eq!(plan.edits[2].end_byte, 43);
    assert_eq!(plan.edits[2].replacement, "X");
}

fn source_for(node: OrgNodeId) -> AffectedNodeSource {
    AffectedNodeSource {
        node,
        source: OrgSourceSpan {
            document: "memory.org".to_owned(),
            start_byte: 12,
            end_byte: 48,
            start_line: 2,
            end_line: 5,
        },
        tokens: OrgNodeSourceTokens::default(),
    }
}

fn source_with_tokens(node: OrgNodeId) -> AffectedNodeSource {
    let mut tokens = OrgNodeSourceTokens {
        todo_keyword: Some(span(2, 6)),
        ..OrgNodeSourceTokens::default()
    };
    tokens
        .property_values
        .insert("OWNER".to_owned(), span(18, 29));
    tokens.checkbox_markers.push(span(42, 43));

    AffectedNodeSource {
        node,
        source: OrgSourceSpan {
            document: "memory.org".to_owned(),
            start_byte: 0,
            end_byte: 64,
            start_line: 1,
            end_line: 5,
        },
        tokens,
    }
}

fn span(start_byte: usize, end_byte: usize) -> OrgSourceSpan {
    OrgSourceSpan {
        document: "memory.org".to_owned(),
        start_byte,
        end_byte,
        start_line: 1,
        end_line: 1,
    }
}
