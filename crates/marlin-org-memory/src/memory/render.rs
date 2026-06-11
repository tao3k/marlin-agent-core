//! View rendering helpers for `MemoryOrgWorkspace`.

use crate::memory::format::todo_state;
use marlin_org_model::{CheckboxState, LinkKind, OrgNode};
use marlin_workspace_view::{WorkspaceField, WorkspaceViewSpec};
use std::hash::{DefaultHasher, Hash, Hasher};

pub(super) fn render_node_lines(node: &OrgNode, view: &WorkspaceViewSpec) -> Vec<String> {
    let mut lines = Vec::new();
    if includes(view, WorkspaceField::Title)
        && let Some(title) = &node.title
    {
        lines.push(format!("title: {title}"));
    }
    if includes(view, WorkspaceField::Todo)
        && let Some(todo) = &node.todo
    {
        lines.push(format!("todo: {}", todo_state(todo)));
    }
    if includes(view, WorkspaceField::SourceSpan)
        && let Some(source) = &node.source
    {
        lines.push(format!(
            "source: {}:{}-{} bytes={}..{}",
            source.document, source.start_line, source.end_line, source.start_byte, source.end_byte
        ));
    }
    if includes(view, WorkspaceField::SelectedProperties) {
        for (key, value) in &node.properties {
            lines.push(format!("property.{key}: {value}"));
        }
    }
    if includes(view, WorkspaceField::OpenCheckboxes) {
        for checkbox in &node.checkboxes {
            if checkbox.state == CheckboxState::Open {
                lines.push(format!("open: {}", checkbox.text));
            }
        }
    }
    if includes(view, WorkspaceField::EvidenceLinks) {
        for link in &node.links {
            if matches!(&link.kind, LinkKind::Custom(kind) if kind == "evidence") {
                let description = link
                    .description
                    .as_ref()
                    .map(|value| format!(" - {value}"))
                    .unwrap_or_default();
                lines.push(format!("evidence: {}{}", link.target, description));
            }
        }
    }
    if includes(view, WorkspaceField::Decisions) {
        for (key, value) in &node.properties {
            if key.starts_with("DECISION_") {
                lines.push(format!("decision: {value}"));
            }
        }
    }

    lines
}

pub(super) fn includes(view: &WorkspaceViewSpec, field: WorkspaceField) -> bool {
    view.include.contains(&field) && !view.exclude.contains(&field)
}

pub(super) fn view_hash(view: &WorkspaceViewSpec) -> String {
    let mut hasher = DefaultHasher::new();
    format!("{view:?}").hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
