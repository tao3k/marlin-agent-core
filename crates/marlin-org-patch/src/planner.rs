//! Planner that lowers typed workspace patch operations into `Org` text edits.

use std::collections::BTreeMap;

use marlin_org_model::{CheckboxState, LinkKind, OrgLink, OrgNodeId, TodoState};
use marlin_workspace_patch::{
    AffectedNodeSource, EvidenceRef, EvidenceTrust, WorkspacePatch, WorkspacePatchOp,
    WorkspacePatchReceipt,
};

use crate::{OrgPatchDiagnostic, OrgPatchPlan, OrgTextEdit};

/// Builds source-aware `Org` text edit plans from typed workspace patches.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OrgPatchPlanner;

impl OrgPatchPlanner {
    /// Plan source edits for a patch using parser-owned source spans.
    pub fn plan(patch: &WorkspacePatch, sources: &[AffectedNodeSource]) -> OrgPatchPlan {
        let lookup = source_lookup(sources);
        let mut plan = OrgPatchPlan::default();

        for op in &patch.ops {
            plan_op(op, &lookup, &mut plan);
        }

        plan
    }

    /// Plan source edits from a patch receipt's affected source spans.
    pub fn plan_from_receipt(
        patch: &WorkspacePatch,
        receipt: &WorkspacePatchReceipt,
    ) -> OrgPatchPlan {
        Self::plan(patch, &receipt.affected_sources)
    }
}

fn source_lookup(sources: &[AffectedNodeSource]) -> BTreeMap<&OrgNodeId, &AffectedNodeSource> {
    sources
        .iter()
        .map(|affected| (&affected.node, affected))
        .collect()
}

fn plan_op(
    op: &WorkspacePatchOp,
    sources: &BTreeMap<&OrgNodeId, &AffectedNodeSource>,
    plan: &mut OrgPatchPlan,
) {
    match op {
        WorkspacePatchOp::AddCheckbox { node, text, state } => append_to_node(
            plan,
            sources,
            node,
            "add-checkbox",
            format!("\n- [{}] {}\n", checkbox_marker(state), text),
        ),
        WorkspacePatchOp::AddLink { node, link } => append_to_node(
            plan,
            sources,
            node,
            "add-link",
            format!("\n{}\n", format_link(link)),
        ),
        WorkspacePatchOp::AddEvidenceRef { node, evidence } => append_to_node(
            plan,
            sources,
            node,
            "add-evidence-ref",
            format!("\n- evidence: {}\n", format_evidence(evidence)),
        ),
        WorkspacePatchOp::SetTodo { node, state } => {
            replace_todo_keyword(plan, sources, node, state);
        }
        WorkspacePatchOp::SetProperty { node, key, value } => {
            replace_property_value(plan, sources, node, key, value);
        }
        WorkspacePatchOp::MarkCheckbox { node, index, state } => {
            replace_checkbox_marker(plan, sources, node, *index, state);
        }
        WorkspacePatchOp::AppendSection { node, .. } => needs_token_span(
            plan,
            node,
            "append-section",
            "requires the parent heading level before emitting a child heading",
        ),
        WorkspacePatchOp::AddMetricPoint { node, .. } => needs_token_span(
            plan,
            node,
            "add-metric-point",
            "requires a metric storage convention before emitting source text",
        ),
        WorkspacePatchOp::AddDecision { node, .. } => needs_token_span(
            plan,
            node,
            "add-decision",
            "requires a decision storage convention before emitting source text",
        ),
        WorkspacePatchOp::AddTraceEvent { node, .. } => needs_token_span(
            plan,
            node,
            "add-trace-event",
            "requires a trace storage convention before emitting source text",
        ),
        WorkspacePatchOp::MarkMemoryCandidate { node, .. } => needs_token_span(
            plan,
            node,
            "mark-memory-candidate",
            "requires a memory dispatch property token span",
        ),
    }
}

fn append_to_node(
    plan: &mut OrgPatchPlan,
    sources: &BTreeMap<&OrgNodeId, &AffectedNodeSource>,
    node: &OrgNodeId,
    operation: &str,
    replacement: String,
) {
    let Some(affected) = sources.get(node) else {
        plan.diagnostics.push(OrgPatchDiagnostic {
            node: Some(node.clone()),
            operation: operation.to_owned(),
            message: "missing parser-owned source span for node".to_owned(),
        });
        return;
    };
    let source = &affected.source;

    plan.edits.push(OrgTextEdit {
        document: source.document.clone(),
        start_byte: source.end_byte,
        end_byte: source.end_byte,
        replacement,
        reason: operation.to_owned(),
    });
}

fn replace_todo_keyword(
    plan: &mut OrgPatchPlan,
    sources: &BTreeMap<&OrgNodeId, &AffectedNodeSource>,
    node: &OrgNodeId,
    state: &TodoState,
) {
    let Some(affected) = sources.get(node) else {
        missing_source_span(plan, node, "set-todo");
        return;
    };
    let Some(span) = affected.tokens.todo_keyword.as_ref() else {
        needs_token_span(
            plan,
            node,
            "set-todo",
            "requires a headline TODO keyword token span",
        );
        return;
    };
    replace_span(plan, span, todo_keyword(state), "set-todo");
}

fn replace_property_value(
    plan: &mut OrgPatchPlan,
    sources: &BTreeMap<&OrgNodeId, &AffectedNodeSource>,
    node: &OrgNodeId,
    key: &str,
    value: &str,
) {
    let Some(affected) = sources.get(node) else {
        missing_source_span(plan, node, "set-property");
        return;
    };
    let Some(span) = affected.tokens.property_values.get(key) else {
        needs_token_span(
            plan,
            node,
            "set-property",
            "requires a property drawer entry token span",
        );
        return;
    };
    replace_span(plan, span, value.to_owned(), "set-property");
}

fn replace_checkbox_marker(
    plan: &mut OrgPatchPlan,
    sources: &BTreeMap<&OrgNodeId, &AffectedNodeSource>,
    node: &OrgNodeId,
    index: usize,
    state: &CheckboxState,
) {
    let Some(affected) = sources.get(node) else {
        missing_source_span(plan, node, "mark-checkbox");
        return;
    };
    let Some(span) = affected.tokens.checkbox_markers.get(index) else {
        needs_token_span(
            plan,
            node,
            "mark-checkbox",
            "requires a checklist item token span",
        );
        return;
    };
    replace_span(
        plan,
        span,
        checkbox_marker(state).to_owned(),
        "mark-checkbox",
    );
}

fn replace_span(
    plan: &mut OrgPatchPlan,
    span: &marlin_org_model::OrgSourceSpan,
    replacement: String,
    reason: &str,
) {
    plan.edits.push(OrgTextEdit {
        document: span.document.clone(),
        start_byte: span.start_byte,
        end_byte: span.end_byte,
        replacement,
        reason: reason.to_owned(),
    });
}

fn missing_source_span(plan: &mut OrgPatchPlan, node: &OrgNodeId, operation: &str) {
    plan.diagnostics.push(OrgPatchDiagnostic {
        node: Some(node.clone()),
        operation: operation.to_owned(),
        message: "missing parser-owned source span for node".to_owned(),
    });
}

fn needs_token_span(plan: &mut OrgPatchPlan, node: &OrgNodeId, operation: &str, message: &str) {
    plan.diagnostics.push(OrgPatchDiagnostic {
        node: Some(node.clone()),
        operation: operation.to_owned(),
        message: message.to_owned(),
    });
}

fn checkbox_marker(state: &CheckboxState) -> &'static str {
    match state {
        CheckboxState::Open => " ",
        CheckboxState::Checked => "X",
        CheckboxState::Partial => "-",
    }
}

fn todo_keyword(state: &TodoState) -> String {
    match state {
        TodoState::Todo => "TODO".to_string(),
        TodoState::Next => "NEXT".to_string(),
        TodoState::Wait => "WAIT".to_string(),
        TodoState::Blocked => "BLOCKED".to_string(),
        TodoState::Done => "DONE".to_string(),
        TodoState::Custom(value) => value.clone(),
    }
}

fn format_link(link: &OrgLink) -> String {
    let target = match &link.kind {
        LinkKind::Id => format!("id:{}", link.target),
        LinkKind::File => format!("file:{}", link.target),
        LinkKind::Url => link.target.clone(),
        LinkKind::Custom(kind) => format!("{}:{}", kind, link.target),
    };

    match &link.description {
        Some(description) => format!("[[{}][{}]]", target, description),
        None => format!("[[{}]]", target),
    }
}

fn format_evidence(evidence: &EvidenceRef) -> String {
    format!(
        "{} :: {} [{}]",
        evidence.summary,
        evidence.target,
        evidence_trust(&evidence.trust)
    )
}

fn evidence_trust(trust: &EvidenceTrust) -> &'static str {
    match trust {
        EvidenceTrust::Internal => "internal",
        EvidenceTrust::External => "external",
        EvidenceTrust::Quarantined => "quarantined",
        EvidenceTrust::Verified => "verified",
    }
}
