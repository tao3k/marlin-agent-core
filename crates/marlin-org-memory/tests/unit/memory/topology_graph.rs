use marlin_agent_protocol::{
    GraphQueryContext, GraphQueryFamily, GraphQueryRelationshipFact, GraphQueryRequest,
};
use marlin_org_memory::{
    MemoryOrgWorkspace, TOPOLOGY_AGENT_ID_PROPERTY, TOPOLOGY_CONTENT_ID_PROPERTY,
    TOPOLOGY_CONTRACT_VALIDATED_PROPERTY, TOPOLOGY_EDGE_KIND_PROPERTY, TOPOLOGY_ID_PROPERTY,
    TOPOLOGY_NODE_KIND_PROPERTY, TOPOLOGY_PROJECT_ID_PROPERTY, TOPOLOGY_ROOT_SESSION_ID_PROPERTY,
    TOPOLOGY_SCOPE_PROPERTY, TOPOLOGY_SESSION_ID_PROPERTY, TOPOLOGY_SOURCE_ANCHOR_PROPERTY,
    TOPOLOGY_WORKSPACE_ID_PROPERTY, TOPOLOGY_WORKTREE_ID_PROPERTY, TopologyGraphStoreQuery,
};
use marlin_org_model::{OrgNode, OrgNodeId, OrgSourceSpan};
use marlin_org_store::{MemoryOrgSourceStore, OrgProjectRootCandidate};
use std::collections::BTreeMap;

#[test]
fn topology_graph_matches_project_import_overview() {
    let mut node = topology_node(TopologyNodeFixture {
        id: "topology-node:project-import",
        title: "Project topology: marlin-agent-core import",
        project_id: "project-alpha",
        workspace_id: "workspace-a",
        worktree_id: "worktree-a",
        root_session_id: "root-a",
        session_id: "session-a",
        agent_id: "agent:planner",
        content_id: "content:import",
        topology_id: "topology.project-alpha.import",
        node_kind: "project",
        edge_kind: "imports-project",
        source_anchor: "docs/40-rfcs/40.120-project-scoped-agent-graph-runtime.org",
        contract_validated: true,
    });
    node.source = Some(OrgSourceSpan {
        document: ".marlin/topology/project-alpha.org".to_string(),
        start_byte: 10,
        end_byte: 90,
        start_line: 3,
        end_line: 14,
    });
    let workspace = MemoryOrgWorkspace::from_nodes(vec![node]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-a")
            .with_worktree("worktree-a")
            .with_root_session("root-a")
            .with_session("session-a")
            .with_agent("agent:planner")
            .with_content_anchor("content:import"),
        GraphQueryFamily::Topology,
        "imports-project project-overview",
    )
    .with_content_anchor("content:import")
    .with_limit(5);

    let response = workspace
        .query_topology_graph("receipt:topology-query", request)
        .expect("topology query succeeds");

    assert_eq!(response.matches.len(), 1);
    let query_match = &response.matches[0];
    assert_eq!(
        query_match
            .source_anchor_id
            .as_ref()
            .map(|anchor_id| anchor_id.as_str()),
        Some("docs/40-rfcs/40.120-project-scoped-agent-graph-runtime.org")
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
            .content_id
            .as_ref()
            .map(|content_id| content_id.as_str()),
        Some("content:import")
    );
    assert!(
        query_match
            .relationship
            .has_fact(GraphQueryRelationshipFact::SameProject)
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
}

#[test]
fn topology_graph_requires_external_project_policy() {
    let workspace = MemoryOrgWorkspace::from_nodes(vec![topology_node(TopologyNodeFixture {
        id: "topology-node:external",
        title: "External topology root",
        project_id: "project-beta",
        workspace_id: "workspace-z",
        worktree_id: "worktree-z",
        root_session_id: "root-z",
        session_id: "session-z",
        agent_id: "agent:external",
        content_id: "content:external",
        topology_id: "topology.project-beta.import",
        node_kind: "project",
        edge_kind: "imports-project",
        source_anchor: "external/project.org",
        contract_validated: false,
    })]);

    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Topology,
        "external topology root",
    );

    let response = workspace
        .query_topology_graph("receipt:topology-external", request)
        .expect("topology query succeeds");

    assert!(response.matches.is_empty());
}

#[test]
fn topology_graph_executes_store_backed_root_query() {
    let store = MemoryOrgSourceStore::new(BTreeMap::from([(
        ".marlin/topology/project.org".to_string(),
        "* Project topology: marlin-agent-core import\n\
         :PROPERTIES:\n\
         :TOPOLOGY_ID: topology.project-alpha.import\n\
         :PROJECT_ID: project-alpha\n\
         :WORKSPACE_ID: workspace-a\n\
         :ROOT_SESSION_ID: root-a\n\
         :CONTENT_ID: content:import\n\
         :TOPOLOGY_SCOPE: project-overview\n\
         :TOPOLOGY_NODE_KIND: project\n\
         :TOPOLOGY_EDGE_KIND: imports-project\n\
         :SOURCE_ANCHOR: docs/40-rfcs/40.120-project-scoped-agent-graph-runtime.org\n\
         :CONTRACT_VALIDATED: true\n\
         :END:\n\
         ** Overview\n\
         Project import topology overview.\n\
         ** Navigation\n\
         - project imports marlin-agent-core.\n\
         ** Visibility Boundaries\n\
         - sibling sessions stay transcript-hidden.\n"
            .to_string(),
    )]));
    let workspace = MemoryOrgWorkspace::new();
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-a")
            .with_root_session("root-a")
            .with_content_anchor("content:import"),
        GraphQueryFamily::Topology,
        "project-overview imports-project",
    )
    .with_content_anchor("content:import");

    let response = workspace
        .query_topology_graph_from_store(TopologyGraphStoreQuery {
            receipt_id: "receipt:topology-store".to_string(),
            request,
            store: &store,
            candidates: vec![OrgProjectRootCandidate::topology(
                ".marlin/topology/project.org",
            )],
        })
        .expect("store backed topology query succeeds");

    assert_eq!(response.matches.len(), 1);
    assert_eq!(response.receipt_id.as_str(), "receipt:topology-store");
    assert!(
        response.matches[0]
            .relationship
            .has_fact(GraphQueryRelationshipFact::ContractValidated)
    );
    assert_eq!(
        response.matches[0]
            .source_anchor_id
            .as_ref()
            .map(|anchor_id| anchor_id.as_str()),
        Some("docs/40-rfcs/40.120-project-scoped-agent-graph-runtime.org")
    );
}

struct TopologyNodeFixture<'a> {
    id: &'a str,
    title: &'a str,
    project_id: &'a str,
    workspace_id: &'a str,
    worktree_id: &'a str,
    root_session_id: &'a str,
    session_id: &'a str,
    agent_id: &'a str,
    content_id: &'a str,
    topology_id: &'a str,
    node_kind: &'a str,
    edge_kind: &'a str,
    source_anchor: &'a str,
    contract_validated: bool,
}

fn topology_node(fixture: TopologyNodeFixture<'_>) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(fixture.id), fixture.title);
    node.properties.insert(
        TOPOLOGY_ID_PROPERTY.to_string(),
        fixture.topology_id.to_string(),
    );
    node.properties.insert(
        TOPOLOGY_PROJECT_ID_PROPERTY.to_string(),
        fixture.project_id.to_string(),
    );
    node.properties.insert(
        TOPOLOGY_WORKSPACE_ID_PROPERTY.to_string(),
        fixture.workspace_id.to_string(),
    );
    node.properties.insert(
        TOPOLOGY_WORKTREE_ID_PROPERTY.to_string(),
        fixture.worktree_id.to_string(),
    );
    node.properties.insert(
        TOPOLOGY_ROOT_SESSION_ID_PROPERTY.to_string(),
        fixture.root_session_id.to_string(),
    );
    node.properties.insert(
        TOPOLOGY_SESSION_ID_PROPERTY.to_string(),
        fixture.session_id.to_string(),
    );
    node.properties.insert(
        TOPOLOGY_AGENT_ID_PROPERTY.to_string(),
        fixture.agent_id.to_string(),
    );
    node.properties.insert(
        TOPOLOGY_CONTENT_ID_PROPERTY.to_string(),
        fixture.content_id.to_string(),
    );
    node.properties.insert(
        TOPOLOGY_SCOPE_PROPERTY.to_string(),
        "project-overview".to_string(),
    );
    node.properties.insert(
        TOPOLOGY_NODE_KIND_PROPERTY.to_string(),
        fixture.node_kind.to_string(),
    );
    node.properties.insert(
        TOPOLOGY_EDGE_KIND_PROPERTY.to_string(),
        fixture.edge_kind.to_string(),
    );
    node.properties.insert(
        TOPOLOGY_SOURCE_ANCHOR_PROPERTY.to_string(),
        fixture.source_anchor.to_string(),
    );
    node.properties.insert(
        TOPOLOGY_CONTRACT_VALIDATED_PROPERTY.to_string(),
        fixture.contract_validated.to_string(),
    );
    node
}
