use marlin_agent_protocol::{
    GraphQueryContext, GraphQueryExternalProjectPolicy, GraphQueryFallbackPolicy, GraphQueryFamily,
    GraphQueryRelationshipFact, GraphQueryRequest, GraphQueryScoreBasisPoints,
};
use marlin_org_memory::{
    MemoryOrgWorkspace, PROJECT_MEMORY_CONTRACT_VALIDATED_PROPERTY, PROJECT_MEMORY_ID_PROPERTY,
    PROJECT_MEMORY_PROJECT_ID_PROPERTY, PROJECT_MEMORY_RECALL_QUERY_PROPERTY,
    PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY, PROJECT_MEMORY_WORKTREE_ID_PROPERTY,
};
use marlin_org_model::{OrgNode, OrgNodeId};

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
