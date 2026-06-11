use marlin_org_model::{LinkKind, OrgCheckbox, OrgLink, OrgNode, OrgNodeId};
use marlin_workspace_patch::{
    AffectedNodeSource, DecisionRecord, EvidenceRef, EvidenceTrust, MemoryDispatchReceipt,
    MetricPoint, PatchId, WorkspacePatch, WorkspacePatchExecutionReceipt, WorkspacePatchOp,
    WorkspacePatchReceipt, WorkspaceValidationReport,
};
use marlin_workspace_protocol::{WorkspaceError, WorkspaceResult};
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

pub(super) fn apply_workspace_patch(
    nodes: &mut BTreeMap<OrgNodeId, OrgNode>,
    patch: WorkspacePatch,
) -> WorkspaceResult<WorkspacePatchReceipt> {
    let before_hash = workspace_hash(nodes);
    let mut affected_nodes = Vec::new();
    let mut memory_dispatch = Vec::new();

    for op in patch.ops {
        apply_op(nodes, op, &mut affected_nodes, &mut memory_dispatch)?;
    }

    affected_nodes.sort();
    affected_nodes.dedup();
    let affected_sources = affected_sources(nodes, &affected_nodes);
    let after_hash = workspace_hash(nodes);

    Ok(WorkspacePatchReceipt {
        patch_id: PatchId::new(format!("patch:{after_hash}")),
        affected_nodes,
        affected_sources,
        before_hash,
        after_hash,
        validation: WorkspaceValidationReport::accepted(),
        execution: WorkspacePatchExecutionReceipt::commit_accepted(
            "in-memory workspace patch applied",
        ),
        memory_dispatch,
    })
}

fn affected_sources(
    nodes: &BTreeMap<OrgNodeId, OrgNode>,
    affected_nodes: &[OrgNodeId],
) -> Vec<AffectedNodeSource> {
    affected_nodes
        .iter()
        .filter_map(|node_id| {
            nodes.get(node_id).and_then(|node| {
                node.source.clone().map(|source| AffectedNodeSource {
                    node: node_id.clone(),
                    source,
                    tokens: node.tokens.clone(),
                })
            })
        })
        .collect()
}

fn apply_op(
    nodes: &mut BTreeMap<OrgNodeId, OrgNode>,
    op: WorkspacePatchOp,
    affected_nodes: &mut Vec<OrgNodeId>,
    memory_dispatch: &mut Vec<MemoryDispatchReceipt>,
) -> WorkspaceResult<()> {
    match op {
        WorkspacePatchOp::SetTodo { node, state } => {
            let target = node_mut(nodes, &node)?;
            target.todo = Some(state);
            affected_nodes.push(node);
        }
        WorkspacePatchOp::SetProperty { node, key, value } => {
            let target = node_mut(nodes, &node)?;
            target.properties.insert(key, value);
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddCheckbox { node, text, state } => {
            let target = node_mut(nodes, &node)?;
            target.checkboxes.push(OrgCheckbox::new(text, state));
            affected_nodes.push(node);
        }
        WorkspacePatchOp::MarkCheckbox { node, index, state } => {
            let target = node_mut(nodes, &node)?;
            let checkbox = target.checkboxes.get_mut(index).ok_or_else(|| {
                WorkspaceError::PatchRejected(format!(
                    "checkbox index {index} not found on {}",
                    node.as_str()
                ))
            })?;
            checkbox.state = state;
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AppendSection {
            node,
            heading,
            body,
        } => {
            let child_id =
                OrgNodeId::new(format!("{}/section/{}", node.as_str(), stable_id(&heading)));
            {
                let parent = node_mut(nodes, &node)?;
                parent.children.push(child_id.clone());
            }
            let mut child = OrgNode::heading(child_id.clone(), heading);
            child.body = Some(body);
            nodes.insert(child_id, child);
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddLink { node, link } => {
            let target = node_mut(nodes, &node)?;
            target.links.push(link);
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddEvidenceRef { node, evidence } => {
            let target = node_mut(nodes, &node)?;
            target.links.push(evidence_link(&evidence));
            target.properties.insert(
                "LAST_EVIDENCE_TRUST".to_string(),
                evidence_trust(&evidence.trust),
            );
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddMetricPoint { node, metric } => {
            let target = node_mut(nodes, &node)?;
            target
                .properties
                .insert(metric_key(&metric), metric.value.to_string());
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddDecision { node, decision } => {
            let target = node_mut(nodes, &node)?;
            target.properties.insert(
                format!("DECISION_{}", target.properties.len()),
                decision_text(&decision),
            );
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddTraceEvent { node, body } => {
            let target = node_mut(nodes, &node)?;
            target
                .properties
                .insert(format!("TRACE_{}", target.properties.len()), body);
            affected_nodes.push(node);
        }
        WorkspacePatchOp::MarkMemoryCandidate { node, dispatch } => {
            let target = node_mut(nodes, &node)?;
            target
                .properties
                .insert("MEMORY_DISPATCH".to_string(), dispatch.clone());
            memory_dispatch.push(MemoryDispatchReceipt {
                target: dispatch,
                accepted: true,
                reason: Some("memory candidate marked".to_string()),
            });
            affected_nodes.push(node);
        }
    }

    Ok(())
}

fn node_mut<'a>(
    nodes: &'a mut BTreeMap<OrgNodeId, OrgNode>,
    id: &OrgNodeId,
) -> WorkspaceResult<&'a mut OrgNode> {
    nodes
        .get_mut(id)
        .ok_or_else(|| WorkspaceError::NodeNotFound(id.as_str().to_string()))
}

fn workspace_hash(nodes: &BTreeMap<OrgNodeId, OrgNode>) -> String {
    let mut hasher = DefaultHasher::new();
    format!("{nodes:?}").hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn stable_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn evidence_link(evidence: &EvidenceRef) -> OrgLink {
    OrgLink {
        kind: LinkKind::Custom("evidence".to_string()),
        target: evidence.target.clone(),
        description: Some(evidence.summary.clone()),
    }
}

fn evidence_trust(trust: &EvidenceTrust) -> String {
    match trust {
        EvidenceTrust::Internal => "internal",
        EvidenceTrust::External => "external",
        EvidenceTrust::Quarantined => "quarantined",
        EvidenceTrust::Verified => "verified",
    }
    .to_string()
}

fn metric_key(metric: &MetricPoint) -> String {
    format!("METRIC_{}", metric.name)
}

fn decision_text(decision: &DecisionRecord) -> String {
    format!("{}: {}", decision.decision, decision.rationale)
}
