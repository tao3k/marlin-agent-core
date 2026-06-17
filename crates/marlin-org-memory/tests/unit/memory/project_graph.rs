use marlin_agent_protocol::{
    GraphQueryContext, GraphQueryExternalProjectPolicy, GraphQueryFallbackPolicy, GraphQueryFamily,
    GraphQueryRelationshipFact, GraphQueryRequest, GraphQueryScoreBasisPoints,
};
use marlin_org_memory::{
    MemoryOrgWorkspace, PROJECT_MEMORY_CONTRACT_VALIDATED_PROPERTY, PROJECT_MEMORY_ID_PROPERTY,
    PROJECT_MEMORY_PROJECT_ID_PROPERTY, PROJECT_MEMORY_RECALL_QUERY_PROPERTY,
    PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY, PROJECT_MEMORY_WORKTREE_ID_PROPERTY,
};
use marlin_org_model::{LinkKind, OrgLink, OrgNode, OrgNodeId, OrgSourceSpan};

#[test]
fn project_memory_graph_matches_same_project_across_worktrees_without_sibling_transcript() {
    let workspace = MemoryOrgWorkspace::from_nodes(vec![
        memory_node(
            "memory:ui-continuation",
            "Button audit keeps icon labels stable",
            "project-alpha",
            "worktree-a",
            "root-a",
            true,
        ),
        raw_session_node(
            "session:root-a:raw-turn",
            "Button audit raw sibling transcript should stay hidden",
            "project-alpha",
            "worktree-a",
            "root-a",
        ),
        memory_node(
            "memory:external-ui",
            "Button audit belongs to another project",
            "project-beta",
            "worktree-z",
            "root-z",
            false,
        ),
    ]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_worktree("worktree-b")
            .with_root_session("root-b"),
        GraphQueryFamily::Memory,
        "button audit",
    )
    .with_limit(5);

    let response = workspace
        .query_project_memory_graph("receipt:project-memory", request)
        .expect("project memory query succeeds");

    assert_eq!(response.matches.len(), 1);
    let query_match = &response.matches[0];
    assert_eq!(
        query_match.memory_id.as_ref().map(|id| id.as_str()),
        Some("memory:ui-continuation")
    );
    assert!(
        query_match
            .relationship
            .has_fact(GraphQueryRelationshipFact::SameProject)
    );
    assert!(
        query_match
            .relationship
            .has_fact(GraphQueryRelationshipFact::ContractValidated)
    );
    assert!(
        !query_match
            .relationship
            .has_fact(GraphQueryRelationshipFact::SameWorktreeProvenance)
    );
    assert!(query_match.score_basis_points.as_u16() >= 7_000);
}

#[test]
fn project_memory_graph_requires_external_project_policy() {
    let workspace = MemoryOrgWorkspace::from_nodes(vec![memory_node(
        "memory:external-ui",
        "Button audit external memory",
        "project-beta",
        "worktree-z",
        "root-z",
        false,
    )]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Memory,
        "button audit",
    );

    let denied = workspace
        .query_project_memory_graph("receipt:external-denied", request.clone())
        .expect("project memory query succeeds");
    assert!(denied.matches.is_empty());

    let allowed_request = GraphQueryRequest {
        context: request
            .context
            .with_fallback_policy(GraphQueryFallbackPolicy {
                external_projects: GraphQueryExternalProjectPolicy::Enabled {
                    min_score_basis_points: GraphQueryScoreBasisPoints::new(1_000),
                },
                ..GraphQueryFallbackPolicy::default()
            }),
        ..request
    };

    let allowed = workspace
        .query_project_memory_graph("receipt:external-allowed", allowed_request)
        .expect("project memory query succeeds");
    assert_eq!(allowed.matches.len(), 1);
    assert!(
        allowed.matches[0]
            .relationship
            .has_fact(GraphQueryRelationshipFact::ExternalProject)
    );
}

#[test]
fn project_memory_graph_matches_contract_recall_query_frontier_terms() {
    let mut node = memory_node(
        "memory:hybrid-recall-boundary",
        "Org memory candidate boundary",
        "project-alpha",
        "worktree-a",
        "root-a",
        true,
    );
    node.properties.insert(
        PROJECT_MEMORY_RECALL_QUERY_PROPERTY.to_string(),
        "hybrid frontier evidence graph".to_string(),
    );
    let workspace = MemoryOrgWorkspace::from_nodes(vec![node]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Memory,
        "hybrid frontier",
    );

    let response = workspace
        .query_project_memory_graph("receipt:recall-frontier", request)
        .expect("project memory query succeeds");

    assert_eq!(response.matches.len(), 1);
    let query_match = &response.matches[0];
    assert_eq!(
        query_match.memory_id.as_ref().map(|id| id.as_str()),
        Some("memory:hybrid-recall-boundary")
    );
    assert_eq!(query_match.summary, "Org memory candidate boundary");
    assert!(
        query_match
            .relationship
            .has_fact(GraphQueryRelationshipFact::ContractValidated)
    );
}

#[test]
fn project_memory_graph_matches_contract_indexed_frontier_terms() {
    let mut node = memory_node(
        "memory:contract-indexed-frontier",
        "Quiet memory candidate",
        "project-alpha",
        "worktree-a",
        "root-a",
        true,
    );
    node.tags.push("graph_frontier".to_string());
    node.properties.insert(
        "EVIDENCE_FACT".to_string(),
        "contract-evidence-alpha".to_string(),
    );
    node.links.push(OrgLink {
        kind: LinkKind::Id,
        target: "policy-shard-beta".to_string(),
        description: Some("evidence backlink".to_string()),
    });
    node.source = Some(OrgSourceSpan {
        document: ".data/memory/session-alpha.org".to_string(),
        start_byte: 10,
        end_byte: 80,
        start_line: 7,
        end_line: 12,
    });
    let workspace = MemoryOrgWorkspace::from_nodes(vec![node]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Memory,
        "graph_frontier contract-evidence-alpha policy-shard-beta session-alpha.org",
    );

    let response = workspace
        .query_project_memory_graph("receipt:contract-indexed-frontier", request)
        .expect("project memory query succeeds");

    assert_eq!(response.matches.len(), 1);
    assert_eq!(
        response.matches[0].memory_id.as_ref().map(|id| id.as_str()),
        Some("memory:contract-indexed-frontier")
    );
    assert!(
        response.matches[0]
            .relationship
            .has_fact(GraphQueryRelationshipFact::ContractValidated)
    );
}

#[test]
fn project_memory_graph_source_anchor_follows_matched_node_not_memory_id() {
    let mut first = memory_node(
        "memory-node:duplicate-a",
        "Duplicate A",
        "project-alpha",
        "worktree-a",
        "root-a",
        true,
    );
    first.properties.insert(
        PROJECT_MEMORY_ID_PROPERTY.to_string(),
        "memory:duplicated".to_string(),
    );
    first.properties.insert(
        PROJECT_MEMORY_RECALL_QUERY_PROPERTY.to_string(),
        "duplicate frontier".to_string(),
    );
    let mut second = memory_node(
        "memory-node:duplicate-b",
        "Duplicate B",
        "project-alpha",
        "worktree-b",
        "root-b",
        true,
    );
    second.properties.insert(
        PROJECT_MEMORY_ID_PROPERTY.to_string(),
        "memory:duplicated".to_string(),
    );
    second.properties.insert(
        PROJECT_MEMORY_RECALL_QUERY_PROPERTY.to_string(),
        "duplicate frontier".to_string(),
    );
    let workspace = MemoryOrgWorkspace::from_nodes(vec![first, second]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Memory,
        "duplicate frontier",
    )
    .with_limit(5);

    let response = workspace
        .query_project_memory_graph("receipt:duplicate-anchor", request)
        .expect("project memory query succeeds");

    assert_eq!(response.matches.len(), 2);
    assert!(response.matches.iter().all(|query_match| {
        query_match
            .memory_id
            .as_ref()
            .is_some_and(|memory_id| memory_id.as_str() == "memory:duplicated")
    }));
    let mut source_anchor_ids = response
        .matches
        .iter()
        .map(|query_match| {
            query_match
                .source_anchor_id
                .as_ref()
                .expect("source anchor")
                .as_str()
                .to_owned()
        })
        .collect::<Vec<_>>();
    source_anchor_ids.sort();
    assert_eq!(
        source_anchor_ids,
        ["memory-node:duplicate-a", "memory-node:duplicate-b"]
    );
}

fn memory_node(
    id: &str,
    title: &str,
    project_id: &str,
    worktree_id: &str,
    root_session_id: &str,
    contract_validated: bool,
) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(id), title);
    node.properties
        .insert(PROJECT_MEMORY_ID_PROPERTY.to_string(), id.to_string());
    node.properties.insert(
        PROJECT_MEMORY_PROJECT_ID_PROPERTY.to_string(),
        project_id.to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_WORKTREE_ID_PROPERTY.to_string(),
        worktree_id.to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY.to_string(),
        root_session_id.to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_CONTRACT_VALIDATED_PROPERTY.to_string(),
        contract_validated.to_string(),
    );
    node
}

fn raw_session_node(
    id: &str,
    title: &str,
    project_id: &str,
    worktree_id: &str,
    root_session_id: &str,
) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(id), title);
    node.properties.insert(
        PROJECT_MEMORY_PROJECT_ID_PROPERTY.to_string(),
        project_id.to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_WORKTREE_ID_PROPERTY.to_string(),
        worktree_id.to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY.to_string(),
        root_session_id.to_string(),
    );
    node
}
