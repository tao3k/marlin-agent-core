use marlin_agent_protocol::{
    GraphQueryContext, GraphQueryFamily, GraphQueryRelationshipFact, GraphQueryRequest,
};
use marlin_org_memory::{
    MemoryOrgWorkspace, TOOL_CAPABILITY_CONTRACT_VALIDATED_PROPERTY, TOOL_CAPABILITY_ID_PROPERTY,
    TOOL_CAPABILITY_PROJECT_ID_PROPERTY, TOOL_CAPABILITY_REQUIRED_RECEIPTS_PROPERTY,
    TOOL_CAPABILITY_ROOT_SESSION_ID_PROPERTY, TOOL_CAPABILITY_WORKSPACE_ID_PROPERTY,
    TOOL_CAPABILITY_WORKTREE_ID_PROPERTY,
};
use marlin_org_model::{LinkKind, OrgLink, OrgNode, OrgNodeId, OrgSourceSpan};

#[test]
fn tool_capability_graph_matches_project_local_card_with_contract_frontier() {
    let mut node = tool_node(
        "tool-node:rustfmt",
        "Rust formatter capability",
        "tool:rustfmt",
        "project-alpha",
        "workspace-a",
        "worktree-a",
        "root-a",
        true,
    );
    node.tags.push("rustfmt".to_string());
    node.properties.insert(
        TOOL_CAPABILITY_REQUIRED_RECEIPTS_PROPERTY.to_string(),
        "receipt:format-check receipt:workspace-clean".to_string(),
    );
    node.links.push(OrgLink {
        kind: LinkKind::Id,
        target: "tool-policy-alpha".to_string(),
        description: Some("capability policy".to_string()),
    });
    node.source = Some(OrgSourceSpan {
        document: ".marlin/tools/rustfmt.org".to_string(),
        start_byte: 10,
        end_byte: 80,
        start_line: 3,
        end_line: 11,
    });
    let workspace = MemoryOrgWorkspace::from_nodes(vec![node]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-a")
            .with_worktree("worktree-b"),
        GraphQueryFamily::Tool,
        "rustfmt receipt:format-check tool-policy-alpha rustfmt.org",
    )
    .with_tool_capability("tool:rustfmt")
    .with_limit(5);

    let response = workspace
        .query_tool_capability_graph("receipt:tool-query", request)
        .expect("tool query succeeds");

    assert_eq!(response.matches.len(), 1);
    let query_match = &response.matches[0];
    assert_eq!(
        query_match
            .tool_capability_id
            .as_ref()
            .map(|capability_id| capability_id.as_str()),
        Some("tool:rustfmt")
    );
    assert_eq!(query_match.summary, "Rust formatter capability");
    assert!(
        query_match
            .relationship
            .has_fact(GraphQueryRelationshipFact::SameProject)
    );
    assert!(
        query_match
            .relationship
            .has_fact(GraphQueryRelationshipFact::SameWorkspace)
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
            .is_some_and(|anchor_id| anchor_id.as_str() == "tool-node:rustfmt")
    );
}

#[test]
fn tool_capability_graph_requires_external_project_policy() {
    let workspace = MemoryOrgWorkspace::from_nodes(vec![tool_node(
        "tool-node:external",
        "External deployment capability",
        "tool:deploy",
        "project-beta",
        "workspace-z",
        "worktree-z",
        "root-z",
        false,
    )]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Tool,
        "deployment",
    )
    .with_tool_capability("tool:deploy");

    let response = workspace
        .query_tool_capability_graph("receipt:tool-external", request)
        .expect("tool query succeeds");

    assert!(response.matches.is_empty());
}

fn tool_node(
    id: &str,
    title: &str,
    capability_id: &str,
    project_id: &str,
    workspace_id: &str,
    worktree_id: &str,
    root_session_id: &str,
    contract_validated: bool,
) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(id), title);
    node.properties.insert(
        TOOL_CAPABILITY_ID_PROPERTY.to_string(),
        capability_id.to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_PROJECT_ID_PROPERTY.to_string(),
        project_id.to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_WORKSPACE_ID_PROPERTY.to_string(),
        workspace_id.to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_WORKTREE_ID_PROPERTY.to_string(),
        worktree_id.to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_ROOT_SESSION_ID_PROPERTY.to_string(),
        root_session_id.to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_CONTRACT_VALIDATED_PROPERTY.to_string(),
        contract_validated.to_string(),
    );
    node
}
