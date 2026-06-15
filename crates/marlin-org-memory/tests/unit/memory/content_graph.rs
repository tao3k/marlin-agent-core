use marlin_agent_protocol::{
    GraphQueryContext, GraphQueryFamily, GraphQueryRelationshipFact, GraphQueryRequest,
};
use marlin_org_memory::{
    CONTENT_NODE_AGENT_ID_PROPERTY, CONTENT_NODE_BODY_REF_PROPERTY,
    CONTENT_NODE_COMPRESSION_STATE_PROPERTY, CONTENT_NODE_CONTENT_ID_PROPERTY,
    CONTENT_NODE_CONTRACT_VALIDATED_PROPERTY, CONTENT_NODE_PARENT_CONTENT_ID_PROPERTY,
    CONTENT_NODE_PROJECT_ID_PROPERTY, CONTENT_NODE_ROLE_PROPERTY,
    CONTENT_NODE_ROOT_SESSION_ID_PROPERTY, CONTENT_NODE_SESSION_ID_PROPERTY,
    CONTENT_NODE_TOKEN_COUNT_PROPERTY, CONTENT_NODE_WORKSPACE_ID_PROPERTY,
    CONTENT_NODE_WORKTREE_ID_PROPERTY, MemoryOrgWorkspace,
};
use marlin_org_model::{LinkKind, OrgLink, OrgNode, OrgNodeId, OrgSourceSpan};

#[test]
fn content_graph_matches_exact_anchor_and_parent_ancestry() {
    let mut node = content_node(
        "content-node:summary-a",
        "Packed summary for UI audit",
        "content:summary-a",
        "project-alpha",
        "workspace-a",
        "worktree-a",
        "root-a",
        "session-a",
        "agent:reviewer",
        true,
    );
    node.properties.insert(
        CONTENT_NODE_PARENT_CONTENT_ID_PROPERTY.to_string(),
        "content:turn-7".to_string(),
    );
    node.links.push(OrgLink {
        kind: LinkKind::Id,
        target: "evidence:ui-audit".to_string(),
        description: Some("bounded evidence".to_string()),
    });
    node.source = Some(OrgSourceSpan {
        document: ".marlin/content/session-a.org".to_string(),
        start_byte: 10,
        end_byte: 80,
        start_line: 8,
        end_line: 15,
    });
    let workspace = MemoryOrgWorkspace::from_nodes(vec![node]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-a")
            .with_worktree("worktree-a")
            .with_root_session("root-a")
            .with_session("session-a")
            .with_content_anchor("content:turn-7"),
        GraphQueryFamily::Content,
        "summary evidence:ui-audit session-a.org",
    )
    .with_content_anchor("content:summary-a")
    .with_limit(5);

    let response = workspace
        .query_content_graph("receipt:content-query", request)
        .expect("content query succeeds");

    assert_eq!(response.matches.len(), 1);
    let query_match = &response.matches[0];
    assert_eq!(
        query_match
            .content_id
            .as_ref()
            .map(|content_id| content_id.as_str()),
        Some("content:summary-a")
    );
    assert_eq!(
        query_match
            .source_session_id
            .as_ref()
            .map(|session_id| session_id.as_str()),
        Some("session-a")
    );
    assert_eq!(
        query_match
            .source_agent_id
            .as_ref()
            .map(|agent_id| agent_id.as_str()),
        Some("agent:reviewer")
    );
    assert!(
        query_match
            .relationship
            .has_fact(GraphQueryRelationshipFact::SameContentAncestry)
    );
    assert!(
        query_match
            .relationship
            .has_fact(GraphQueryRelationshipFact::ContractValidated)
    );
    assert!(
        query_match
            .source_anchor_id
            .as_ref()
            .is_some_and(|anchor_id| anchor_id.as_str() == "content-node:summary-a")
    );
}

#[test]
fn content_graph_requires_external_project_policy() {
    let workspace = MemoryOrgWorkspace::from_nodes(vec![content_node(
        "content-node:external",
        "External packed summary",
        "content:external",
        "project-beta",
        "workspace-z",
        "worktree-z",
        "root-z",
        "session-z",
        "agent:external",
        false,
    )]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Content,
        "external packed summary",
    );

    let response = workspace
        .query_content_graph("receipt:content-external", request)
        .expect("content query succeeds");

    assert!(response.matches.is_empty());
}

fn content_node(
    id: &str,
    title: &str,
    content_id: &str,
    project_id: &str,
    workspace_id: &str,
    worktree_id: &str,
    root_session_id: &str,
    session_id: &str,
    agent_id: &str,
    contract_validated: bool,
) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(id), title);
    node.properties.insert(
        CONTENT_NODE_CONTENT_ID_PROPERTY.to_string(),
        content_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_PROJECT_ID_PROPERTY.to_string(),
        project_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_WORKSPACE_ID_PROPERTY.to_string(),
        workspace_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_WORKTREE_ID_PROPERTY.to_string(),
        worktree_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_ROOT_SESSION_ID_PROPERTY.to_string(),
        root_session_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_SESSION_ID_PROPERTY.to_string(),
        session_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_AGENT_ID_PROPERTY.to_string(),
        agent_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_ROLE_PROPERTY.to_string(),
        "summary".to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_BODY_REF_PROPERTY.to_string(),
        "body-ref:summary-a".to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_TOKEN_COUNT_PROPERTY.to_string(),
        "512".to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_COMPRESSION_STATE_PROPERTY.to_string(),
        "packed".to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_CONTRACT_VALIDATED_PROPERTY.to_string(),
        contract_validated.to_string(),
    );
    node
}
