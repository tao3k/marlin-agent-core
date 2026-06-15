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
    let mut node = content_node(ContentNodeFixture {
        id: "content-node:summary-a",
        title: "Packed summary for UI audit",
        content_id: "content:summary-a",
        project_id: "project-alpha",
        workspace_id: "workspace-a",
        worktree_id: "worktree-a",
        root_session_id: "root-a",
        session_id: "session-a",
        agent_id: "agent:reviewer",
        contract_validated: true,
    });
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
    let workspace = MemoryOrgWorkspace::from_nodes(vec![content_node(ContentNodeFixture {
        id: "content-node:external",
        title: "External packed summary",
        content_id: "content:external",
        project_id: "project-beta",
        workspace_id: "workspace-z",
        worktree_id: "worktree-z",
        root_session_id: "root-z",
        session_id: "session-z",
        agent_id: "agent:external",
        contract_validated: false,
    })]);

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

struct ContentNodeFixture<'a> {
    id: &'a str,
    title: &'a str,
    content_id: &'a str,
    project_id: &'a str,
    workspace_id: &'a str,
    worktree_id: &'a str,
    root_session_id: &'a str,
    session_id: &'a str,
    agent_id: &'a str,
    contract_validated: bool,
}

fn content_node(fixture: ContentNodeFixture<'_>) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(fixture.id), fixture.title);
    node.properties.insert(
        CONTENT_NODE_CONTENT_ID_PROPERTY.to_string(),
        fixture.content_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_PROJECT_ID_PROPERTY.to_string(),
        fixture.project_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_WORKSPACE_ID_PROPERTY.to_string(),
        fixture.workspace_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_WORKTREE_ID_PROPERTY.to_string(),
        fixture.worktree_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_ROOT_SESSION_ID_PROPERTY.to_string(),
        fixture.root_session_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_SESSION_ID_PROPERTY.to_string(),
        fixture.session_id.to_string(),
    );
    node.properties.insert(
        CONTENT_NODE_AGENT_ID_PROPERTY.to_string(),
        fixture.agent_id.to_string(),
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
        fixture.contract_validated.to_string(),
    );
    node
}
