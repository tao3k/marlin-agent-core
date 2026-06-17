use std::collections::BTreeMap;

use marlin_agent_protocol::{
    GraphQueryContext, GraphQueryFamily, GraphQueryRelationshipFact, GraphQueryRequest,
};
use marlin_org_memory::{
    MemoryOrgWorkspace, TOOL_CAPABILITY_BACKEND_REQUIREMENTS_PROPERTY,
    TOOL_CAPABILITY_CONTRACT_VALIDATED_PROPERTY, TOOL_CAPABILITY_ID_PROPERTY,
    TOOL_CAPABILITY_ISOLATION_REQUIREMENTS_PROPERTY, TOOL_CAPABILITY_PROJECT_ID_PROPERTY,
    TOOL_CAPABILITY_REQUIRED_CAPABILITIES_PROPERTY, TOOL_CAPABILITY_REQUIRED_RECEIPTS_PROPERTY,
    TOOL_CAPABILITY_ROOT_SESSION_ID_PROPERTY, TOOL_CAPABILITY_WORKSPACE_ID_PROPERTY,
    TOOL_CAPABILITY_WORKTREE_ID_PROPERTY, ToolCapabilityGraphStoreQuery,
};
use marlin_org_model::{LinkKind, OrgLink, OrgNode, OrgNodeId, OrgSourceSpan};
use marlin_org_store::{MemoryOrgSourceStore, OrgProjectRootCandidate};

#[test]
fn tool_capability_graph_matches_project_local_card_with_contract_frontier() {
    let mut node = tool_node(ToolNodeFixture {
        id: "tool-node:rustfmt",
        title: "Rust formatter capability",
        capability_id: "tool:rustfmt",
        project_id: "project-alpha",
        workspace_id: "workspace-a",
        worktree_id: "worktree-a",
        root_session_id: "root-a",
        contract_validated: true,
    });
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
fn tool_capability_graph_queries_store_backed_tool_roots() {
    let store = MemoryOrgSourceStore::new(BTreeMap::from([(
        ".marlin/tools/rustfmt.org".to_string(),
        r#"* Rust formatter capability
:PROPERTIES:
:TOOL_CAPABILITY_ID: tool:rustfmt
:PROJECT_ID: project-alpha
:WORKSPACE_ID: workspace-a
:ROOT_SESSION_ID: root-a
:REQUIRED_RECEIPTS: receipt:format-check
:CONTRACT_VALIDATED: true
:END:
Source path marker: rustfmt.org
"#
        .to_string(),
    )]));
    let workspace = MemoryOrgWorkspace::new();
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-a")
            .with_root_session("root-a"),
        GraphQueryFamily::Tool,
        "rustfmt receipt:format-check rustfmt.org",
    )
    .with_tool_capability("tool:rustfmt")
    .with_limit(5);

    let response = workspace
        .query_tool_capability_graph_from_store(ToolCapabilityGraphStoreQuery {
            receipt_id: "receipt:tool-store-query".to_string(),
            request,
            store: &store,
            candidates: vec![
                OrgProjectRootCandidate::tool_capability(".marlin/tools/rustfmt.org"),
                OrgProjectRootCandidate::tool_capability(".marlin/tools/missing.org"),
            ],
        })
        .expect("store-backed tool query succeeds");

    assert_eq!(response.receipt_id.as_str(), "receipt:tool-store-query");
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
            .has_fact(GraphQueryRelationshipFact::ContractValidated)
    );
}

#[test]
fn tool_capability_cards_project_typed_backend_requirements() {
    let mut node = tool_node(ToolNodeFixture {
        id: "tool-node:rustfmt-card",
        title: "Rust formatter card",
        capability_id: "tool:rustfmt",
        project_id: "project-alpha",
        workspace_id: "workspace-a",
        worktree_id: "worktree-a",
        root_session_id: "root-a",
        contract_validated: true,
    });
    node.properties.insert(
        TOOL_CAPABILITY_REQUIRED_RECEIPTS_PROPERTY.to_string(),
        "receipt:format-check, receipt:workspace-clean".to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_REQUIRED_CAPABILITIES_PROPERTY.to_string(),
        "tool:workspace-status".to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_ISOLATION_REQUIREMENTS_PROPERTY.to_string(),
        "isolation:write-worktree".to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_BACKEND_REQUIREMENTS_PROPERTY.to_string(),
        "backend:process-sandbox backend:macos-compatible".to_string(),
    );
    let workspace = MemoryOrgWorkspace::from_nodes([node]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha").with_workspace("workspace-a"),
        GraphQueryFamily::Tool,
        "rust formatter card",
    )
    .with_tool_capability("tool:rustfmt")
    .with_limit(5);

    let cards = workspace
        .query_tool_capability_cards("receipt:tool-card-query", request)
        .expect("tool card query succeeds");

    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert_eq!(
        card.graph_match
            .tool_capability_id
            .as_ref()
            .expect("tool capability")
            .as_str(),
        "tool:rustfmt"
    );
    assert_eq!(
        card.graph_match
            .source_anchor_id
            .as_ref()
            .expect("source anchor")
            .as_str(),
        "tool-node:rustfmt-card"
    );
    let required_receipts = card
        .required_receipt_ids
        .iter()
        .map(|receipt_id| receipt_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        required_receipts,
        ["receipt:format-check", "receipt:workspace-clean"]
    );
    assert_eq!(
        card.required_capability_ids[0].as_str(),
        "tool:workspace-status"
    );
    assert_eq!(
        card.isolation_requirement_ids[0].as_str(),
        "isolation:write-worktree"
    );
    let backend_requirements = card
        .backend_requirement_ids
        .iter()
        .map(|requirement_id| requirement_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        backend_requirements,
        ["backend:process-sandbox", "backend:macos-compatible"]
    );
}

#[test]
fn tool_capability_graph_requires_external_project_policy() {
    let workspace = MemoryOrgWorkspace::from_nodes(vec![tool_node(ToolNodeFixture {
        id: "tool-node:external",
        title: "External deployment capability",
        capability_id: "tool:deploy",
        project_id: "project-beta",
        workspace_id: "workspace-z",
        worktree_id: "worktree-z",
        root_session_id: "root-z",
        contract_validated: false,
    })]);

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

struct ToolNodeFixture<'a> {
    id: &'a str,
    title: &'a str,
    capability_id: &'a str,
    project_id: &'a str,
    workspace_id: &'a str,
    worktree_id: &'a str,
    root_session_id: &'a str,
    contract_validated: bool,
}

fn tool_node(fixture: ToolNodeFixture<'_>) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(fixture.id), fixture.title);
    node.properties.insert(
        TOOL_CAPABILITY_ID_PROPERTY.to_string(),
        fixture.capability_id.to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_PROJECT_ID_PROPERTY.to_string(),
        fixture.project_id.to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_WORKSPACE_ID_PROPERTY.to_string(),
        fixture.workspace_id.to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_WORKTREE_ID_PROPERTY.to_string(),
        fixture.worktree_id.to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_ROOT_SESSION_ID_PROPERTY.to_string(),
        fixture.root_session_id.to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_CONTRACT_VALIDATED_PROPERTY.to_string(),
        fixture.contract_validated.to_string(),
    );
    node
}
