// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_context_org_model::{CheckboxState, LinkKind, OrgNode, OrgNodeId, TodoState};
use marlin_workspace_status::{
    ChecklistStatus, ContractStatus, DecisionTrace, EvidenceStatus, GoalState, GoalStatus,
    MetricTrace, ReleaseStatus, SddStatus, WorkspaceStatusReport, WorkspaceTarget,
};
use marlin_workspace_view::RenderedContractFacts;
use std::collections::BTreeMap;

pub(super) fn target_node<'a>(
    nodes: &'a BTreeMap<OrgNodeId, OrgNode>,
    target: &WorkspaceTarget,
) -> Option<&'a OrgNode> {
    match target {
        WorkspaceTarget::Goal(id) | WorkspaceTarget::Sdd(id) | WorkspaceTarget::Checklist(id) => {
            nodes.get(id)
        }
        WorkspaceTarget::Workspace => nodes.values().next(),
    }
}

pub(super) fn status_for_node(
    node: Option<&OrgNode>,
    contract_facts: &RenderedContractFacts,
    release: Option<ReleaseStatus>,
) -> WorkspaceStatusReport {
    let contracts = Some(contract_status(contract_facts));
    let Some(node) = node else {
        return WorkspaceStatusReport {
            goal: None,
            sdd: None,
            checklist: None,
            evidence: None,
            contracts,
            release,
            metrics: Vec::new(),
            decisions: DecisionTrace { recent: Vec::new() },
            next_actions: Vec::new(),
        };
    };

    let checklist = checklist_status(node);
    let evidence = evidence_status(node);
    WorkspaceStatusReport {
        goal: Some(GoalStatus {
            title: node
                .title
                .clone()
                .unwrap_or_else(|| node.id.as_str().to_string()),
            state: node
                .todo
                .as_ref()
                .map(goal_state)
                .unwrap_or(GoalState::Active),
            open_blockers: node
                .checkboxes
                .iter()
                .filter(|checkbox| checkbox.state == CheckboxState::Open)
                .map(|checkbox| checkbox.text.clone())
                .collect(),
        }),
        sdd: Some(SddStatus {
            title: node
                .title
                .clone()
                .unwrap_or_else(|| node.id.as_str().to_string()),
            accepted: checklist.open == 0,
            missing_evidence: usize::from(evidence.linked == 0),
        }),
        checklist: Some(checklist),
        evidence: Some(evidence),
        contracts,
        release,
        metrics: metric_traces(node),
        decisions: decision_trace(node),
        next_actions: node
            .checkboxes
            .iter()
            .filter(|checkbox| checkbox.state == CheckboxState::Open)
            .map(|checkbox| checkbox.text.clone())
            .collect(),
    }
}

fn contract_status(contract_facts: &RenderedContractFacts) -> ContractStatus {
    let summary = &contract_facts.summary;

    ContractStatus {
        resolved_references: summary.resolved_references,
        unresolved_references: summary.unresolved_references,
        diagnostics: summary.diagnostics,
        templates: summary.templates,
        contract_assertions: summary.contract_assertions,
        validation_receipts: summary.validation_receipts,
        validation_passed: summary.validation_passed,
        validation_failed: summary.validation_failed,
        validation_skipped: summary.validation_skipped,
        validation_matched_nodes: summary.validation_matched_nodes,
        validation_matched_node_ids: summary.validation_matched_node_ids.clone(),
        validation_skip_reasons: summary.validation_skip_reasons.clone(),
        reference_resolutions: contract_facts.resolutions.clone(),
        diagnostic_records: contract_facts.diagnostics.clone(),
        template_records: contract_facts.templates.clone(),
        registry: contract_facts.registry.clone(),
        validation_report: contract_facts.validations.clone(),
        contract_expectation_summaries: summary.contract_expectation_summaries.clone(),
        rendered_summary: contract_facts.rendered_lines.clone(),
    }
}

fn checklist_status(node: &OrgNode) -> ChecklistStatus {
    let done = node
        .checkboxes
        .iter()
        .filter(|checkbox| checkbox.state == CheckboxState::Checked)
        .count();
    let open = node
        .checkboxes
        .iter()
        .filter(|checkbox| checkbox.state == CheckboxState::Open)
        .count();
    let blocked = node
        .checkboxes
        .iter()
        .filter(|checkbox| checkbox.text.to_lowercase().contains("block"))
        .count();
    ChecklistStatus {
        done,
        open,
        blocked,
    }
}

fn evidence_status(node: &OrgNode) -> EvidenceStatus {
    let linked = node
        .links
        .iter()
        .filter(|link| matches!(&link.kind, LinkKind::Custom(kind) if kind == "evidence"))
        .count();
    let quarantined = usize::from(
        node.properties
            .get("LAST_EVIDENCE_TRUST")
            .is_some_and(|trust| trust == "quarantined"),
    );
    EvidenceStatus {
        linked,
        missing: usize::from(linked == 0),
        quarantined,
    }
}

fn metric_traces(node: &OrgNode) -> Vec<MetricTrace> {
    node.properties
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix("METRIC_").map(|name| MetricTrace {
                name: name.to_string(),
                latest: value.parse().ok(),
                target: None,
            })
        })
        .collect()
}

fn decision_trace(node: &OrgNode) -> DecisionTrace {
    DecisionTrace {
        recent: node
            .properties
            .iter()
            .filter(|(key, _value)| key.starts_with("DECISION_"))
            .map(|(_key, value)| value.clone())
            .collect(),
    }
}

fn goal_state(todo: &TodoState) -> GoalState {
    match todo {
        TodoState::Todo => GoalState::Todo,
        TodoState::Next => GoalState::Next,
        TodoState::Wait => GoalState::Waiting,
        TodoState::Blocked => GoalState::Blocked,
        TodoState::Done => GoalState::Done,
        TodoState::Custom(value) => GoalState::Custom(value.clone()),
    }
}
