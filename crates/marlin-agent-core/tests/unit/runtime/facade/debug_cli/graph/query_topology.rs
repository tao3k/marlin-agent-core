use marlin_agent_core::{
    ProjectRuntimeQuerySummary,
    protocol::{
        GraphQueryContext, GraphQueryFamily, GraphQueryRelationshipFact, GraphQueryRequest,
    },
    run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn debug_cli_graph_query_executes_store_backed_topology_query() {
    let dir = tempdir().expect("tempdir");
    let request_path = dir.path().join("topology-request.json");
    let topology_root = dir.path().join("topology.org");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-a")
            .with_root_session("root-a")
            .with_content_anchor("content:import"),
        GraphQueryFamily::Topology,
        "project-overview imports-project topology.org",
    )
    .with_content_anchor("content:import")
    .with_limit(5);
    fs::write(
        &request_path,
        serde_json::to_string(&request).expect("request JSON"),
    )
    .expect("write request");
    fs::write(
        &topology_root,
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
         Source path marker: topology.org\n\
         ** Overview\n\
         Project topology overview.\n\
         ** Navigation\n\
         - project imports marlin-agent-core.\n\
         ** Visibility Boundaries\n\
         - sibling sessions stay transcript-hidden.\n",
    )
    .expect("write topology root");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        request_path.to_str().expect("utf8 path"),
        "--org-topology-store-root",
        dir.path().to_str().expect("utf8 path"),
        "--org-topology-root",
        "topology.org",
        "--receipt-id",
        "receipt:cli-store-topology",
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: ProjectRuntimeQuerySummary =
        serde_json::from_str(&query.stdout).expect("project runtime query summary");
    assert_eq!(summary.receipt_id.as_str(), "receipt:cli-store-topology");
    assert_eq!(summary.family, GraphQueryFamily::Topology);
    assert_eq!(summary.match_count, 1);
    assert_eq!(
        summary
            .source_anchor_ids
            .iter()
            .map(|anchor_id| anchor_id.as_str())
            .collect::<Vec<_>>(),
        vec!["docs/40-rfcs/40.120-project-scoped-agent-graph-runtime.org"]
    );
    assert!(
        summary
            .relationship_facts
            .contains(&GraphQueryRelationshipFact::ContractValidated)
    );
}

#[test]
fn debug_cli_graph_query_requires_topology_root_for_topology_store_root() {
    let dir = tempdir().expect("tempdir");
    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--org-topology-store-root",
        dir.path().to_str().expect("utf8 path"),
    ]);

    assert_ne!(query.status, 0);
    assert!(
        query
            .stderr
            .contains("--org-topology-store-root requires --org-topology-root")
    );
}

#[test]
fn debug_cli_graph_query_rejects_missing_topology_store_root() {
    let dir = tempdir().expect("tempdir");
    let request_path = dir.path().join("topology-request.json");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Topology,
        "project topology",
    );
    fs::write(
        &request_path,
        serde_json::to_string(&request).expect("request JSON"),
    )
    .expect("write request");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        request_path.to_str().expect("utf8 path"),
        "--org-topology-store-root",
        dir.path().to_str().expect("utf8 path"),
        "--org-topology-root",
        "missing-topology.org",
    ]);

    assert_ne!(query.status, 0);
    assert!(
        query
            .stderr
            .contains("--org-topology-root points at missing Org root(s): missing-topology.org")
    );
}

#[test]
fn debug_cli_graph_query_rejects_mixed_topology_and_tool_store_roots() {
    let dir = tempdir().expect("tempdir");
    let request_path = dir.path().join("topology-request.json");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Topology,
        "project topology",
    );
    fs::write(
        &request_path,
        serde_json::to_string(&request).expect("request JSON"),
    )
    .expect("write request");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        request_path.to_str().expect("utf8 path"),
        "--org-topology-root",
        "topology.org",
        "--org-tool-root",
        "tools.org",
    ]);

    assert_ne!(query.status, 0);
    assert!(
        query
            .stderr
            .contains("--org-topology-root cannot be combined with --org-tool-root")
    );
}
