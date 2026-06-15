use marlin_agent_protocol::{
    GraphQueryContext, GraphQueryFamily, GraphQueryRelationshipFact, GraphQueryRequest,
};
use marlin_org_memory::{
    MemoryOrgWorkspace, SESSION_FACT_AGENT_ID_PROPERTY, SESSION_FACT_CONTEXT_PACK_ID_PROPERTY,
    SESSION_FACT_CONTRACT_VALIDATED_PROPERTY, SESSION_FACT_FORKED_FROM_CONTENT_ID_PROPERTY,
    SESSION_FACT_KIND_PROPERTY, SESSION_FACT_PROJECT_ID_PROPERTY,
    SESSION_FACT_ROOT_SESSION_ID_PROPERTY, SESSION_FACT_SESSION_ID_PROPERTY,
    SESSION_FACT_WORKSPACE_ID_PROPERTY, SESSION_FACT_WORKTREE_ID_PROPERTY,
};
use marlin_org_model::{OrgNode, OrgNodeId, OrgSourceSpan};

#[test]
fn session_graph_matches_session_local_boundary_card() {
    let mut node = session_node(SessionNodeFixture {
        id: "session-node:child-a",
        title: "Child session keeps UI audit context pack",
        project_id: "project-alpha",
        workspace_id: "workspace-a",
        worktree_id: "worktree-a",
        root_session_id: "root-a",
        session_id: "session-a",
        agent_id: "agent:reviewer",
        contract_validated: true,
    });
    node.properties.insert(
        SESSION_FACT_FORKED_FROM_CONTENT_ID_PROPERTY.to_string(),
        "content:turn-7".to_string(),
    );
    node.properties.insert(
        SESSION_FACT_CONTEXT_PACK_ID_PROPERTY.to_string(),
        "context-pack:ui-audit".to_string(),
    );
    node.source = Some(OrgSourceSpan {
        document: ".marlin/sessions/session-a.org".to_string(),
        start_byte: 10,
        end_byte: 80,
        start_line: 4,
        end_line: 12,
    });
    let workspace = MemoryOrgWorkspace::from_nodes(vec![node]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-a")
            .with_worktree("worktree-a")
            .with_root_session("root-a")
            .with_session("session-a")
            .with_content_anchor("content:turn-7"),
        GraphQueryFamily::Session,
        "context-pack:ui-audit session-a.org",
    )
    .with_content_anchor("content:turn-7")
    .with_limit(5);

    let response = workspace
        .query_session_graph("receipt:session-query", request)
        .expect("session query succeeds");

    assert_eq!(response.matches.len(), 1);
    let query_match = &response.matches[0];
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
    assert_eq!(
        query_match
            .content_id
            .as_ref()
            .map(|content_id| content_id.as_str()),
        Some("content:turn-7")
    );
    assert!(
        query_match
            .relationship
            .has_fact(GraphQueryRelationshipFact::SameSessionLineage)
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
            .is_some_and(|anchor_id| anchor_id.as_str() == "session-node:child-a")
    );
}

#[test]
fn session_graph_requires_external_project_policy() {
    let workspace = MemoryOrgWorkspace::from_nodes(vec![session_node(SessionNodeFixture {
        id: "session-node:external",
        title: "External child session",
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
        GraphQueryFamily::Session,
        "external child session",
    );

    let response = workspace
        .query_session_graph("receipt:session-external", request)
        .expect("session query succeeds");

    assert!(response.matches.is_empty());
}

struct SessionNodeFixture<'a> {
    id: &'a str,
    title: &'a str,
    project_id: &'a str,
    workspace_id: &'a str,
    worktree_id: &'a str,
    root_session_id: &'a str,
    session_id: &'a str,
    agent_id: &'a str,
    contract_validated: bool,
}

fn session_node(fixture: SessionNodeFixture<'_>) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(fixture.id), fixture.title);
    node.properties.insert(
        SESSION_FACT_PROJECT_ID_PROPERTY.to_string(),
        fixture.project_id.to_string(),
    );
    node.properties.insert(
        SESSION_FACT_WORKSPACE_ID_PROPERTY.to_string(),
        fixture.workspace_id.to_string(),
    );
    node.properties.insert(
        SESSION_FACT_WORKTREE_ID_PROPERTY.to_string(),
        fixture.worktree_id.to_string(),
    );
    node.properties.insert(
        SESSION_FACT_ROOT_SESSION_ID_PROPERTY.to_string(),
        fixture.root_session_id.to_string(),
    );
    node.properties.insert(
        SESSION_FACT_SESSION_ID_PROPERTY.to_string(),
        fixture.session_id.to_string(),
    );
    node.properties.insert(
        SESSION_FACT_AGENT_ID_PROPERTY.to_string(),
        fixture.agent_id.to_string(),
    );
    node.properties
        .insert(SESSION_FACT_KIND_PROPERTY.to_string(), "child".to_string());
    node.properties.insert(
        SESSION_FACT_CONTRACT_VALIDATED_PROPERTY.to_string(),
        fixture.contract_validated.to_string(),
    );
    node
}
