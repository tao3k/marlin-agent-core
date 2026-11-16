// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_context::org::{
    MemoryOrgWorkspace, PROJECT_MEMORY_CONTRACT_VALIDATED_PROPERTY, PROJECT_MEMORY_ID_PROPERTY,
    PROJECT_MEMORY_PROJECT_ID_PROPERTY, PROJECT_MEMORY_RECALL_QUERY_PROPERTY,
    PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY, PROJECT_MEMORY_WORKTREE_ID_PROPERTY,
};
use marlin_agent_protocol::{
    GraphQueryContext, ProjectMemoryRecallIntent, ProjectMemoryRecallRequest,
};
use marlin_context_org_model::{OrgNode, OrgNodeId};

#[test]
fn recall_source_spans_follow_matched_org_nodes_not_duplicate_memory_ids() {
    let nodes = ["a", "b"].map(|suffix| {
        let mut node = loaded_memory_node_without_source(
            &format!("node:{suffix}"),
            "memory:duplicated",
            "Duplicate frontier",
            "duplicate frontier",
        );
        node.source = Some(marlin_context_org_model::OrgSourceSpan {
            document: format!(".marlin/memory/{suffix}.org"),
            start_byte: 0,
            end_byte: 100,
            start_line: 1,
            end_line: 10,
        });
        node
    });
    let workspace = MemoryOrgWorkspace::from_nodes(nodes);
    let request = ProjectMemoryRecallRequest::new(
        GraphQueryContext::new("project-alpha"),
        ProjectMemoryRecallIntent::RecoverDecision,
    )
    .with_query_term("duplicate")
    .with_query_term("frontier")
    .with_limit(5);

    let pack = workspace
        .recall_project_memory_from_loaded_nodes(
            "context-pack-duplicate",
            "receipt-recall-duplicate",
            request,
        )
        .expect("recall succeeds");

    assert_eq!(pack.facts.len(), 2);
    assert!(
        pack.facts
            .iter()
            .all(|fact| fact.evidence_ids[0].as_str() == "memory:duplicated")
    );
    let mut source_spans = pack
        .facts
        .iter()
        .map(|fact| {
            fact.source_span
                .as_ref()
                .expect("source span")
                .as_str()
                .to_owned()
        })
        .collect::<Vec<_>>();
    source_spans.sort();
    assert_eq!(
        source_spans,
        [
            ".marlin/memory/a.org:L1-L10".to_owned(),
            ".marlin/memory/b.org:L1-L10".to_owned()
        ]
    );
}

#[test]
fn recall_from_loaded_nodes_keeps_fact_when_source_span_is_missing() {
    let workspace = MemoryOrgWorkspace::from_nodes([loaded_memory_node_without_source(
        "memory-node:missing-source",
        "memory:missing-source",
        "Missing source span recall",
        "missing-source frontier",
    )]);
    let request = ProjectMemoryRecallRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_worktree("worktree-a")
            .with_root_session("root-session-1"),
        ProjectMemoryRecallIntent::ExplainEvidence,
    )
    .with_query_term("missing-source")
    .with_query_term("frontier")
    .with_limit(5);

    let pack = workspace
        .recall_project_memory_from_loaded_nodes(
            "context-pack-missing-source",
            "receipt-recall-missing-source",
            request,
        )
        .expect("recall succeeds");

    assert_eq!(pack.context_pack_id.as_str(), "context-pack-missing-source");
    assert_eq!(
        pack.source_receipts[0].as_str(),
        "receipt-recall-missing-source"
    );
    assert_eq!(pack.facts.len(), 1);
    let fact = &pack.facts[0];
    assert_eq!(fact.claim, "Missing source span recall");
    assert!(fact.source_span.is_none());
    assert_eq!(
        fact.graph_match
            .source_anchor_id
            .as_ref()
            .expect("source anchor")
            .as_str(),
        "memory-node:missing-source"
    );
    assert_eq!(
        fact.graph_match
            .memory_id
            .as_ref()
            .expect("memory id")
            .as_str(),
        "memory:missing-source"
    );
    assert_eq!(fact.evidence_ids.len(), 1);
    assert_eq!(fact.evidence_ids[0].as_str(), "memory:missing-source");
}

fn loaded_memory_node_without_source(
    node_id: &str,
    memory_id: &str,
    title: &str,
    recall_query: &str,
) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(node_id), title);
    node.properties.insert(
        PROJECT_MEMORY_ID_PROPERTY.to_string(),
        memory_id.to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_PROJECT_ID_PROPERTY.to_string(),
        "project-alpha".to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_WORKTREE_ID_PROPERTY.to_string(),
        "worktree-a".to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY.to_string(),
        "root-session-1".to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_RECALL_QUERY_PROPERTY.to_string(),
        recall_query.to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_CONTRACT_VALIDATED_PROPERTY.to_string(),
        "true".to_string(),
    );
    node
}
