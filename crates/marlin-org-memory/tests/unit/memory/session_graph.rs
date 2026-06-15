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
    let mut node = session_node(
        "session-node:child-a",
        "Child session keeps UI audit context pack",
        "project-alpha",
        "workspace-a",
        "worktree-a",
        "root-a",
        "session-a",
        "agent:reviewer",
        true,
    );
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
    let workspace = MemoryOrgWorkspace::from_nodes(vec![session_node(
        "session-node:external",
        "External child session",
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
        GraphQueryFamily::Session,
        "external child session",
    );

    let response = workspace
        .query_session_graph("receipt:session-external", request)
        .expect("session query succeeds");

    assert!(response.matches.is_empty());
}

fn session_node(
    id: &str,
    title: &str,
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
        SESSION_FACT_PROJECT_ID_PROPERTY.to_string(),
        project_id.to_string(),
    );
    node.properties.insert(
        SESSION_FACT_WORKSPACE_ID_PROPERTY.to_string(),
        workspace_id.to_string(),
    );
    node.properties.insert(
        SESSION_FACT_WORKTREE_ID_PROPERTY.to_string(),
        worktree_id.to_string(),
    );
    node.properties.insert(
        SESSION_FACT_ROOT_SESSION_ID_PROPERTY.to_string(),
        root_session_id.to_string(),
    );
    node.properties.insert(
        SESSION_FACT_SESSION_ID_PROPERTY.to_string(),
        session_id.to_string(),
    );
    node.properties.insert(
        SESSION_FACT_AGENT_ID_PROPERTY.to_string(),
        agent_id.to_string(),
    );
    node.properties
        .insert(SESSION_FACT_KIND_PROPERTY.to_string(), "child".to_string());
    node.properties.insert(
        SESSION_FACT_CONTRACT_VALIDATED_PROPERTY.to_string(),
        contract_validated.to_string(),
    );
    node
}
